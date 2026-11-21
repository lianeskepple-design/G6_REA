//! Real-time WASAPI monitor: microphone -> EQ -> Crystalizer -> SVM -> output.
//!
//! The DSP chain is the same C reference library used by `sbc-dsp`; the audio
//! callback only calls its in-place process/set functions. Parameters reach
//! the callback through atomics, and a single-producer/single-consumer ring
//! buffer carries processed stereo frames from the capture callback to the
//! render callback.

use aurora_audio_command::eq_core::{self, Chain, OutputMode};
use aurora_audio_command::sbx_modes::{EffectLevels, Mode, ProfileSet};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use eframe::egui;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MAX_CHUNK_FRAMES: usize = 1024;

#[repr(C)]
struct G6Chain {
    _private: [u8; 0],
}

extern "C" {
    fn g6_chain_create(sample_rate: u32, channels: u32) -> *mut G6Chain;
    fn g6_chain_destroy(chain: *mut G6Chain);
    fn g6_chain_set_crystalizer(chain: *mut G6Chain, enabled: i32, strength: u32) -> i32;
    fn g6_chain_set_bass(
        chain: *mut G6Chain,
        enabled: i32,
        strength: i32,
        cutoff_hz: f32,
    ) -> i32;
    fn g6_chain_set_dialog(chain: *mut G6Chain, enabled: i32, strength: i32) -> i32;
    fn g6_chain_set_svm(chain: *mut G6Chain, enabled: i32, level_db: f32, mix: f32) -> i32;
    fn g6_chain_process_interleaved(
        chain: *mut G6Chain,
        samples: *mut f32,
        frames: usize,
    ) -> i32;
}

/// The chain is created in the GUI thread and then moved into exactly one
/// capture callback; no other thread touches it.
struct ChainHandle(*mut G6Chain);

unsafe impl Send for ChainHandle {}

impl ChainHandle {
    fn new(sample_rate: u32, channels: u32) -> Result<Self, String> {
        let ptr = unsafe { g6_chain_create(sample_rate, channels) };
        if ptr.is_null() {
            Err("failed to create the C DSP chain".to_string())
        } else {
            Ok(Self(ptr))
        }
    }

    fn set_crystalizer(&mut self, enabled: bool, strength: u32) -> Result<(), String> {
        if unsafe { g6_chain_set_crystalizer(self.0, enabled as i32, strength) } == 0 {
            Err("failed to configure Crystalizer".to_string())
        } else {
            Ok(())
        }
    }

    fn set_bass(&mut self, enabled: bool, strength: u32, cutoff_hz: f32) -> Result<(), String> {
        if strength > 100 {
            return Err("Bass strength must be 0..100".to_string());
        }
        if !cutoff_hz.is_finite() || !(40.0..=400.0).contains(&cutoff_hz) {
            return Err("Bass cutoff must be 40..400 Hz".to_string());
        }
        if unsafe { g6_chain_set_bass(self.0, enabled as i32, strength as i32, cutoff_hz) } == 0 {
            Err("failed to configure Bass".to_string())
        } else {
            Ok(())
        }
    }

    fn set_dialog(&mut self, enabled: bool, strength: u32) -> Result<(), String> {
        if strength > 100 {
            return Err("Dialog+ strength must be 0..100".to_string());
        }
        if unsafe { g6_chain_set_dialog(self.0, enabled as i32, strength as i32) } == 0 {
            Err("failed to configure Dialog+".to_string())
        } else {
            Ok(())
        }
    }

    fn set_svm(&mut self, enabled: bool, level_db: f32, mix: f32) -> Result<(), String> {
        if unsafe { g6_chain_set_svm(self.0, enabled as i32, level_db, mix) } == 0 {
            Err("failed to configure Smart Volume".to_string())
        } else {
            Ok(())
        }
    }

    fn process(&mut self, samples: &mut [f32], channels: usize) -> Result<(), String> {
        if channels == 0 || samples.len() % channels != 0 {
            return Err("sample count is not channel aligned".to_string());
        }
        let frames = samples.len() / channels;
        if unsafe { g6_chain_process_interleaved(self.0, samples.as_mut_ptr(), frames) } == 0 {
            Err("C DSP chain processing failed".to_string())
        } else {
            Ok(())
        }
    }
}

impl Drop for ChainHandle {
    fn drop(&mut self) {
        unsafe { g6_chain_destroy(self.0) };
    }
}

struct LiveParams {
    input_gain_db: AtomicU32,
    crystalizer_on: AtomicBool,
    crystalizer_strength: AtomicU32,
    bass_on: AtomicBool,
    bass_strength: AtomicU32,
    bass_cutoff_hz: AtomicU32,
    dialog_on: AtomicBool,
    dialog_strength: AtomicU32,
    svm_on: AtomicBool,
    svm_level_db: AtomicU32,
    svm_mix: AtomicU32,
    input_rms: AtomicU32,
    input_peak: AtomicU32,
    output_rms: AtomicU32,
    output_peak: AtomicU32,
    ring_fill_ms: AtomicUsize,
    underruns: AtomicUsize,
    overruns: AtomicUsize,
}

impl Default for LiveParams {
    fn default() -> Self {
        let params = Self {
            input_gain_db: AtomicU32::new(0),
            crystalizer_on: AtomicBool::new(false),
            crystalizer_strength: AtomicU32::new(50),
            bass_on: AtomicBool::new(false),
            bass_strength: AtomicU32::new(50),
            bass_cutoff_hz: AtomicU32::new(0),
            dialog_on: AtomicBool::new(false),
            dialog_strength: AtomicU32::new(50),
            svm_on: AtomicBool::new(false),
            svm_level_db: AtomicU32::new(0),
            svm_mix: AtomicU32::new(0),
            input_rms: AtomicU32::new(0),
            input_peak: AtomicU32::new(0),
            output_rms: AtomicU32::new(0),
            output_peak: AtomicU32::new(0),
            ring_fill_ms: AtomicUsize::new(0),
            underruns: AtomicUsize::new(0),
            overruns: AtomicUsize::new(0),
        };
        store_f32(&params.input_gain_db, 0.0);
        store_f32(&params.bass_cutoff_hz, 120.0);
        store_f32(&params.svm_level_db, -20.0);
        store_f32(&params.svm_mix, 1.0);
        params
    }
}

fn store_f32(atom: &AtomicU32, value: f32) {
    atom.store(value.to_bits(), Ordering::Relaxed);
}

fn load_f32(atom: &AtomicU32) -> f32 {
    f32::from_bits(atom.load(Ordering::Relaxed))
}

struct EqProcessor {
    chain: Chain,
    states: Vec<[f64; 4]>,
}

impl EqProcessor {
    fn new(
        preset: &str,
        mode: OutputMode,
        sample_rate: u32,
        channels: usize,
    ) -> Result<Self, String> {
        let curve = eq_core::preset_curve(preset, mode).map_err(|err| err.to_string())?;
        let chain = Chain::design(&curve, sample_rate, 0.0).map_err(|err| err.to_string())?;
        let per_channel = chain.filter_count();
        Ok(Self {
            chain,
            states: vec![[0.0f64; 4]; channels * per_channel],
        })
    }

    fn process(&mut self, buffer: &mut [f32], channels: usize) {
        let per_channel = self.chain.filter_count();
        if per_channel == 0 {
            for sample in buffer.iter_mut() {
                *sample = (*sample as f64 * self.chain.gain()) as f32;
            }
            return;
        }
        for (index, sample) in buffer.iter_mut().enumerate() {
            let channel = index % channels;
            let state = &mut self.states[channel * per_channel..(channel + 1) * per_channel];
            *sample = self.chain.process_channel(*sample as f64, state) as f32;
        }
    }
}

struct LiveEngine {
    _input: Stream,
    _output: Stream,
}

impl LiveEngine {
    fn start(
        input_name: &str,
        output_name: &str,
        eq_preset: Option<(&str, OutputMode)>,
        params: Arc<LiveParams>,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let input = find_device(&host, input_name, true)?;
        let output = find_device(&host, output_name, false)?;
        if pick_rate(&input, &output).is_none() {
            return Err(
                "no common 48/44.1/96/32 kHz f32 configuration for the selected devices"
                    .to_string(),
            );
        }
        let rate = pick_rate(&input, &output).unwrap();
        let (input_config, input_channels) = build_input_config(&input, rate)?;
        let (output_config, output_channels) = build_output_config(&output, rate)?;

        let eq = eq_preset
            .map(|(preset, mode)| EqProcessor::new(preset, mode, rate, 2))
            .transpose()?;
        let chain = ChainHandle::new(rate, 2)?;
        let (producer, consumer) = RingBuffer::<f32>::new((rate as usize) * 2);

        let input_params = params.clone();
        let mut chain = chain;
        let mut eq = eq;
        let mut producer = producer;
        let mut scratch = vec![0.0f32; MAX_CHUNK_FRAMES * 2];
        let input_stream = input
            .build_input_stream::<f32, _, _>(
                &input_config,
                move |data, _| {
                    process_input(
                        data,
                        input_channels,
                        &mut chain,
                        &mut eq,
                        &mut producer,
                        &input_params,
                        &mut scratch,
                    );
                },
                move |err| eprintln!("input stream error: {err}"),
                None,
            )
            .map_err(|err| format!("input stream: {err}"))?;

        let output_params = params.clone();
        let mut consumer: Consumer<f32> = consumer;
        let output_stream = output
            .build_output_stream::<f32, _, _>(
                &output_config,
                move |data, _| {
                    process_output(data, output_channels, &mut consumer, &output_params, rate);
                },
                move |err| eprintln!("output stream error: {err}"),
                None,
            )
            .map_err(|err| format!("output stream: {err}"))?;

        input_stream
            .play()
            .map_err(|err| format!("start input: {err}"))?;
        // Fill the ring before the render stream starts; this removes the
        // startup underruns and gives the capture callback some jitter margin.
        std::thread::sleep(Duration::from_millis(60));
        output_stream
            .play()
            .map_err(|err| format!("start output: {err}"))?;

        Ok(Self {
            _input: input_stream,
            _output: output_stream,
        })
    }
}

impl Drop for LiveEngine {
    fn drop(&mut self) {
        let _ = self._input.pause();
        let _ = self._output.pause();
    }
}

fn process_input(
    data: &[f32],
    input_channels: u16,
    chain: &mut ChainHandle,
    eq: &mut Option<EqProcessor>,
    producer: &mut Producer<f32>,
    params: &LiveParams,
    scratch: &mut [f32],
) {
    let channels = input_channels as usize;
    let frames = data.len() / channels;
    if frames == 0 {
        return;
    }
    let gain_db = load_f32(&params.input_gain_db);
    let gain = if gain_db.is_finite() {
        10f32.powf(gain_db / 20.0)
    } else {
        1.0
    };
    let mut offset = 0;
    while offset < frames {
        let chunk = (frames - offset).min(MAX_CHUNK_FRAMES);
        for i in 0..chunk {
            let base = (offset + i) * channels;
            let (left, right) = if channels == 1 {
                (data[base], data[base])
            } else {
                (data[base], data[base + 1])
            };
            scratch[i * 2] = left * gain;
            scratch[i * 2 + 1] = right * gain;
        }
        let block = &mut scratch[..chunk * 2];
        if let Some(eq) = eq.as_mut() {
            eq.process(block, 2);
        }

        let crystalizer_on = params.crystalizer_on.load(Ordering::Relaxed);
        let crystalizer_strength = params.crystalizer_strength.load(Ordering::Relaxed);
        let bass_on = params.bass_on.load(Ordering::Relaxed);
        let bass_strength = params.bass_strength.load(Ordering::Relaxed);
        let bass_cutoff = load_f32(&params.bass_cutoff_hz);
        let dialog_on = params.dialog_on.load(Ordering::Relaxed);
        let dialog_strength = params.dialog_strength.load(Ordering::Relaxed);
        let svm_on = params.svm_on.load(Ordering::Relaxed);
        let svm_level = load_f32(&params.svm_level_db);
        let svm_mix = load_f32(&params.svm_mix);
        let _ = chain.set_crystalizer(crystalizer_on, crystalizer_strength);
        let _ = chain.set_bass(bass_on, bass_strength, bass_cutoff);
        let _ = chain.set_dialog(dialog_on, dialog_strength);
        let _ = chain.set_svm(svm_on, svm_level, svm_mix);
        if crystalizer_on || bass_on || dialog_on || svm_on {
            let _ = chain.process(block, 2);
        }

        let mut sum = 0.0f64;
        let mut peak = 0.0f32;
        for &sample in block.iter() {
            sum += (sample as f64) * (sample as f64);
            peak = peak.max(sample.abs());
        }
        store_f32(&params.input_rms, (sum / block.len() as f64).sqrt() as f32);
        store_f32(&params.input_peak, peak);

        let mut full = false;
        for &sample in block.iter() {
            if producer.push(sample).is_err() {
                full = true;
                break;
            }
        }
        if full {
            params.overruns.fetch_add(1, Ordering::Relaxed);
            break;
        }
        offset += chunk;
    }
}

fn process_output(
    data: &mut [f32],
    output_channels: u16,
    consumer: &mut Consumer<f32>,
    params: &LiveParams,
    sample_rate: u32,
) {
    let channels = output_channels as usize;
    let frames = data.len() / channels;
    let fill = consumer.slots();
    params.ring_fill_ms.store(
        (fill as u64 * 1000 / (sample_rate as u64 * 2)) as usize,
        Ordering::Relaxed,
    );
    let mut sum = 0.0f64;
    let mut peak = 0.0f32;
    let mut missing = false;
    for i in 0..frames {
        let left = consumer.pop().unwrap_or_else(|_| {
            missing = true;
            0.0
        });
        let right = consumer.pop().unwrap_or_else(|_| {
            missing = true;
            0.0
        });
        for channel in 0..channels {
            let value = match channel {
                0 => left,
                1 => right,
                _ => 0.0,
            };
            data[i * channels + channel] = value;
            sum += (value as f64) * (value as f64);
            peak = peak.max(value.abs());
        }
    }
    if frames > 0 {
        store_f32(&params.output_rms, (sum / (frames * channels) as f64).sqrt() as f32);
        store_f32(&params.output_peak, peak);
    }
    if missing {
        params.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

fn find_device(host: &cpal::Host, name: &str, input: bool) -> Result<Device, String> {
    let devices = if input {
        host.input_devices()
            .map_err(|err| format!("enumerate inputs: {err}"))?
            .collect::<Vec<_>>()
    } else {
        host.output_devices()
            .map_err(|err| format!("enumerate outputs: {err}"))?
            .collect::<Vec<_>>()
    };
    devices
        .into_iter()
        .find(|device| device.name().map(|value| value == name).unwrap_or(false))
        .ok_or_else(|| format!("device not found: {name}"))
}

fn supports(device: &Device, input: bool, rate: u32, channels: u16) -> bool {
    if input {
        let Ok(ranges) = device.supported_input_configs() else {
            return false;
        };
        ranges.into_iter().any(|range| {
            range.sample_format() == SampleFormat::F32
                && range.channels() == channels
                && range.min_sample_rate().0 <= rate
                && rate <= range.max_sample_rate().0
        })
    } else {
        let Ok(ranges) = device.supported_output_configs() else {
            return false;
        };
        ranges.into_iter().any(|range| {
            range.sample_format() == SampleFormat::F32
                && range.channels() == channels
                && range.min_sample_rate().0 <= rate
                && rate <= range.max_sample_rate().0
        })
    }
}

fn pick_rate(input: &Device, output: &Device) -> Option<u32> {
    [48000u32, 44100, 96000, 32000].into_iter().find(|rate| {
        (supports(input, true, *rate, 1) || supports(input, true, *rate, 2))
            && supports(output, false, *rate, 2)
    })
}

fn build_input_config(device: &Device, rate: u32) -> Result<(StreamConfig, u16), String> {
    for channels in [2u16, 1] {
        if supports(device, true, rate, channels) {
            return Ok((
                StreamConfig {
                    channels,
                    sample_rate: cpal::SampleRate(rate),
                    buffer_size: cpal::BufferSize::Default,
                },
                channels,
            ));
        }
    }
    Err("input device has no f32 1/2-channel configuration at the chosen rate".to_string())
}

fn build_output_config(device: &Device, rate: u32) -> Result<(StreamConfig, u16), String> {
    if supports(device, false, rate, 2) {
        Ok((
            StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(rate),
                buffer_size: cpal::BufferSize::Default,
            },
            2,
        ))
    } else {
        Err("output device has no f32 stereo configuration at the chosen rate".to_string())
    }
}

fn device_names() -> Result<(Vec<String>, Vec<String>), String> {
    let host = cpal::default_host();
    let inputs = host
        .input_devices()
        .map_err(|err| format!("enumerate inputs: {err}"))?
        .filter_map(|device| device.name().ok())
        .collect();
    let outputs = host
        .output_devices()
        .map_err(|err| format!("enumerate outputs: {err}"))?
        .filter_map(|device| device.name().ok())
        .collect();
    Ok((inputs, outputs))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EqMode {
    Headphones,
    Speakers,
}

impl EqMode {
    fn output_mode(self) -> OutputMode {
        match self {
            Self::Headphones => OutputMode::Headphones,
            Self::Speakers => OutputMode::Speakers,
        }
    }
}

fn install_cjk_font(ctx: &egui::Context) {
    // egui ships no CJK glyphs. DengXian is a single TTF on Windows; fall back
    // to SimHei or Microsoft YaHei if it is missing.
    const CANDIDATES: [&str; 3] = [
        r"C:\Windows\Fonts\Deng.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
    ];
    for path in CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "cjk".to_owned(),
            egui::FontData::from_owned(bytes).into(),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push("cjk".to_owned());
        }
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            family.push("cjk".to_owned());
        }
        ctx.set_fonts(fonts);
        return;
    }
}

struct LiveApp {
    input_names: Vec<String>,
    output_names: Vec<String>,
    input_index: usize,
    output_index: usize,
    eq_presets: Vec<String>,
    eq_index: usize,
    eq_on: bool,
    eq_mode: EqMode,
    input_gain_db: f32,
    crystalizer_on: bool,
    crystalizer_strength: u32,
    bass_on: bool,
    bass_strength: u32,
    bass_cutoff_hz: f32,
    dialog_on: bool,
    dialog_strength: u32,
    svm_on: bool,
    svm_level_db: f32,
    svm_mix: f32,
    profiles: ProfileSet,
    profile_index: usize,
    mode: Mode,
    mode_note: String,
    params: Arc<LiveParams>,
    engine: Option<LiveEngine>,
    status: String,
}

impl LiveApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        install_cjk_font(&cc.egui_ctx);
        let (input_names, output_names) = device_names().unwrap_or_default();
        let mut eq_presets = vec!["Bypass".to_string()];
        if let Ok(presets) = eq_core::presets() {
            eq_presets.extend(presets.into_iter().map(|preset| preset.name));
        }
        let status = if input_names.is_empty() || output_names.is_empty() {
            "No audio devices found; refresh after connecting one.".to_string()
        } else {
            "Stopped".to_string()
        };
        let profiles = ProfileSet::load().unwrap_or_default();
        let mut app = Self {
            input_names,
            output_names,
            input_index: 0,
            output_index: 0,
            eq_presets,
            eq_index: 0,
            eq_on: false,
            eq_mode: EqMode::Headphones,
            input_gain_db: 0.0,
            crystalizer_on: false,
            crystalizer_strength: 50,
            bass_on: false,
            bass_strength: 50,
            bass_cutoff_hz: 120.0,
            dialog_on: false,
            dialog_strength: 50,
            svm_on: false,
            svm_level_db: -20.0,
            svm_mix: 1.0,
            profiles,
            profile_index: 0,
            mode: Mode::Direct,
            mode_note: String::new(),
            params: Arc::new(LiveParams::default()),
            engine: None,
            status,
        };
        app.apply_mode();
        app
    }

    /// Load the selected mode/profile into the effect switches.
    ///
    /// Direct and Scout bypass every effect, so the switches are cleared rather
    /// than disabled quietly: the audio thread only reads these fields.
    fn apply_mode(&mut self) {
        let levels = match self.mode {
            Mode::Direct => EffectLevels::direct(),
            Mode::Scout => EffectLevels::scout(),
            Mode::Sbx => {
                let Some(profile) = self.profiles.profiles().get(self.profile_index) else {
                    self.mode_note = "SBX profile data is missing".to_string();
                    return;
                };
                EffectLevels::from_profile(profile)
            }
        };
        self.crystalizer_on = levels.crystalizer_on;
        self.crystalizer_strength = levels.crystalizer as u32;
        self.bass_on = levels.bass_on;
        self.bass_strength = levels.bass as u32;
        self.bass_cutoff_hz = levels.bass_crossover as f32;
        self.dialog_on = levels.dialog_on;
        self.dialog_strength = levels.dialog as u32;
        self.svm_on = levels.smart_volume_on;
        self.svm_level_db = levels.smart_volume_db();
        if levels.bypass_equalizer() {
            self.eq_on = false;
        }
        self.mode_note = match self.mode {
            Mode::Direct => "Direct: every effect bypassed, including the equalizer.".to_string(),
            Mode::Scout => {
                "Scout: SBX, EQ and the other effects are off. The Scout DSP runs on the device, so this client only implements the exclusion."
                    .to_string()
            }
            Mode::Sbx => {
                let name = self
                    .profiles
                    .profiles()
                    .get(self.profile_index)
                    .map(|profile| profile.name.as_str())
                    .unwrap_or("<none>");
                if levels.surround_on {
                    format!("SBX profile {name}; Surround is not in the real-time chain.")
                } else {
                    format!("SBX profile {name}.")
                }
            }
        };
    }

    fn refresh_devices(&mut self) {
        match device_names() {
            Ok((inputs, outputs)) => {
                self.input_names = inputs;
                self.output_names = outputs;
                self.input_index = self.input_index.min(self.input_names.len().saturating_sub(1));
                self.output_index = self.output_index.min(self.output_names.len().saturating_sub(1));
                self.status = "Device list refreshed".to_string();
            }
            Err(err) => self.status = err,
        }
    }

    fn sync_params(&self) {
        store_f32(&self.params.input_gain_db, self.input_gain_db);
        self.params
            .crystalizer_on
            .store(self.crystalizer_on, Ordering::Relaxed);
        self.params
            .crystalizer_strength
            .store(self.crystalizer_strength, Ordering::Relaxed);
        self.params.bass_on.store(self.bass_on, Ordering::Relaxed);
        self.params
            .bass_strength
            .store(self.bass_strength, Ordering::Relaxed);
        store_f32(&self.params.bass_cutoff_hz, self.bass_cutoff_hz);
        self.params.dialog_on.store(self.dialog_on, Ordering::Relaxed);
        self.params
            .dialog_strength
            .store(self.dialog_strength, Ordering::Relaxed);
        self.params.svm_on.store(self.svm_on, Ordering::Relaxed);
        store_f32(&self.params.svm_level_db, self.svm_level_db);
        store_f32(&self.params.svm_mix, self.svm_mix);
    }

    fn start(&mut self) {
        let input = self.input_names.get(self.input_index).cloned();
        let output = self.output_names.get(self.output_index).cloned();
        let (Some(input), Some(output)) = (input, output) else {
            self.status = "Select an input and an output device first".to_string();
            return;
        };
        self.sync_params();
        let eq = if self.eq_on && self.eq_index > 0 {
            self.eq_presets
                .get(self.eq_index)
                .map(|preset| (preset.as_str(), self.eq_mode.output_mode()))
        } else {
            None
        };
        match LiveEngine::start(&input, &output, eq, self.params.clone()) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.status = format!("Running: {input} -> {output}");
            }
            Err(err) => self.status = err,
        }
    }

    fn stop(&mut self) {
        self.engine = None;
        self.status = "Stopped".to_string();
    }

    fn meter(ui: &mut egui::Ui, label: &str, rms: f32, peak: f32) {
        let db = if rms > 1.0e-9 {
            20.0 * rms.log10()
        } else {
            -90.0
        };
        let level = ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f32;
        ui.vertical(|ui| {
            ui.label(label);
            ui.add(
                egui::ProgressBar::new(level)
                    .desired_width(180.0)
                    .text(format!("{db:.1} dB  peak {peak:.3}")),
            );
        });
    }
}

impl eframe::App for LiveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_params();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Sound Blaster Live");
            ui.label("Microphone -> EQ -> Crystalizer -> Smart Volume -> output");
            ui.separator();

            let running = self.engine.is_some();
            ui.add_enabled_ui(!running, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Input")
                        .selected_text(
                            self.input_names
                                .get(self.input_index)
                                .map(String::as_str)
                                .unwrap_or("<none>"),
                        )
                        .show_ui(ui, |ui| {
                            for (index, name) in self.input_names.iter().enumerate() {
                                ui.selectable_value(&mut self.input_index, index, name);
                            }
                        });
                    egui::ComboBox::from_label("Output")
                        .selected_text(
                            self.output_names
                                .get(self.output_index)
                                .map(String::as_str)
                                .unwrap_or("<none>"),
                        )
                        .show_ui(ui, |ui| {
                            for (index, name) in self.output_names.iter().enumerate() {
                                ui.selectable_value(&mut self.output_index, index, name);
                            }
                        });
                    if ui.button("Refresh").clicked() {
                        self.refresh_devices();
                    }
                });
            });

            ui.horizontal(|ui| {
                if running {
                    if ui.button("Stop").clicked() {
                        self.stop();
                    }
                } else if ui.button("Start").clicked() {
                    self.start();
                }
                ui.label(&self.status);
            });
            let output_name = self.output_names.get(self.output_index);
            if output_name
                .map(|name| name.contains("UU远程") || name.contains("虚拟"))
                .unwrap_or(false)
            {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 190, 80),
                    "UU 虚拟扬声器不是回环：远程要听到，需在 UU 控制端开启「启用远程声音」，\
                     并让 UU 采集当前输出设备；本地耳机请另选物理输出。",
                );
            }
            ui.separator();

            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.input_gain_db, -12.0..=36.0)
                        .text("Input gain (dB)"),
                );
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Mode:");
                for mode in Mode::ALL {
                    if ui.selectable_label(self.mode == mode, mode.label()).clicked()
                        && self.mode != mode
                    {
                        self.mode = mode;
                        self.apply_mode();
                    }
                }
            });
            if self.mode == Mode::Sbx {
                let current = self
                    .profiles
                    .profiles()
                    .get(self.profile_index)
                    .map(|profile| profile.name.clone())
                    .unwrap_or_else(|| "<none>".to_string());
                let mut picked = None;
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Profile")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (index, profile) in self.profiles.profiles().iter().enumerate() {
                                if ui
                                    .selectable_label(index == self.profile_index, &profile.name)
                                    .clicked()
                                {
                                    picked = Some(index);
                                }
                            }
                        });
                });
                if let Some(index) = picked {
                    self.profile_index = index;
                    self.apply_mode();
                }
            }
            if !self.mode_note.is_empty() {
                ui.label(egui::RichText::new(&self.mode_note).weak());
            }

            ui.add_enabled_ui(self.mode == Mode::Sbx, |ui| {
            ui.checkbox(&mut self.crystalizer_on, "Crystalizer");
            ui.add_enabled(
                self.crystalizer_on,
                egui::Slider::new(&mut self.crystalizer_strength, 0..=100).text("Strength"),
            );
            ui.checkbox(&mut self.bass_on, "Bass (missing fundamental)");
            ui.add_enabled(
                self.bass_on,
                egui::Slider::new(&mut self.bass_strength, 0..=100).text("Bass strength"),
            );
            ui.add_enabled(
                self.bass_on,
                egui::Slider::new(&mut self.bass_cutoff_hz, 40.0..=200.0)
                    .text("Bass cutoff (Hz)"),
            );
            ui.checkbox(&mut self.dialog_on, "Dialog+");
            ui.add_enabled(
                self.dialog_on,
                egui::Slider::new(&mut self.dialog_strength, 0..=100).text("Dialog strength"),
            );
            ui.checkbox(&mut self.svm_on, "Smart Volume");
            ui.add_enabled(
                self.svm_on,
                egui::Slider::new(&mut self.svm_level_db, -50.0..=0.0).text("Level (dB)"),
            );
            ui.add_enabled(
                self.svm_on,
                egui::Slider::new(&mut self.svm_mix, 0.0..=1.0).text("Mix"),
            );
            });

            ui.separator();
            ui.add_enabled_ui(!running && self.mode == Mode::Sbx, |ui| {
                ui.checkbox(&mut self.eq_on, "EQ preset (applies on Start)");
                ui.add_enabled_ui(self.eq_on, |ui| {
                    egui::ComboBox::from_label("Preset")
                        .selected_text(
                            self.eq_presets
                                .get(self.eq_index)
                                .map(String::as_str)
                                .unwrap_or("Bypass"),
                        )
                        .show_ui(ui, |ui| {
                            for (index, name) in self.eq_presets.iter().enumerate() {
                                ui.selectable_value(&mut self.eq_index, index, name);
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.eq_mode, EqMode::Headphones, "Headphones");
                        ui.selectable_value(&mut self.eq_mode, EqMode::Speakers, "Speakers");
                    });
                });
            });

            ui.separator();
            let input_rms = load_f32(&self.params.input_rms);
            let input_peak = load_f32(&self.params.input_peak);
            let output_rms = load_f32(&self.params.output_rms);
            let output_peak = load_f32(&self.params.output_peak);
            ui.horizontal(|ui| {
                Self::meter(ui, "Input", input_rms, input_peak);
                ui.add_space(24.0);
                Self::meter(ui, "Output", output_rms, output_peak);
            });
            ui.label(format!(
                "buffer {} ms   underruns {}   overruns {}",
                self.params.ring_fill_ms.load(Ordering::Relaxed),
                self.params.underruns.load(Ordering::Relaxed),
                self.params.overruns.load(Ordering::Relaxed)
            ));
            ui.label("Tip: use headphones to avoid acoustic feedback.");
        });
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

fn list_devices_cli() -> Result<(), String> {
    let host = cpal::default_host();
    println!("Default host: {:?}", host.id());
    for (index, device) in host
        .input_devices()
        .map_err(|err| err.to_string())?
        .enumerate()
    {
        let name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        let default = device
            .default_input_config()
            .map(|config| {
                format!(
                    "{} Hz, {} ch, {:?}",
                    config.sample_rate().0,
                    config.channels(),
                    config.sample_format()
                )
            })
            .unwrap_or_else(|_| "no default".to_string());
        println!("IN  {index:2}: {name} [{default}]");
    }
    for (index, device) in host
        .output_devices()
        .map_err(|err| err.to_string())?
        .enumerate()
    {
        let name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        let default = device
            .default_output_config()
            .map(|config| {
                format!(
                    "{} Hz, {} ch, {:?}",
                    config.sample_rate().0,
                    config.channels(),
                    config.sample_format()
                )
            })
            .unwrap_or_else(|_| "no default".to_string());
        println!("OUT {index:2}: {name} [{default}]");
    }
    Ok(())
}

fn parse_arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn selftest(args: &[String]) -> Result<(), String> {
    let seconds = args
        .iter()
        .position(|arg| arg == "--selftest")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            parse_arg(args, "--seconds").and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(3);
    let host = cpal::default_host();
    let input_name = parse_arg(args, "--input").or_else(|| {
        host.default_input_device()
            .and_then(|device| device.name().ok())
    });
    let output_name = parse_arg(args, "--output").or_else(|| {
        host.default_output_device()
            .and_then(|device| device.name().ok())
    });
    let (Some(input_name), Some(output_name)) = (input_name, output_name) else {
        return Err("no default input/output device".to_string());
    };
    let params = Arc::new(LiveParams::default());
    if let Some(strength) = parse_arg(args, "--crystalizer")
        .and_then(|value| value.parse::<u32>().ok())
    {
        params.crystalizer_on.store(true, Ordering::Relaxed);
        params
            .crystalizer_strength
            .store(strength, Ordering::Relaxed);
    }
    if let Some(strength) = parse_arg(args, "--bass")
        .and_then(|value| value.parse::<u32>().ok())
    {
        params.bass_on.store(true, Ordering::Relaxed);
        params.bass_strength.store(strength, Ordering::Relaxed);
    }
    if let Some(cutoff) = parse_arg(args, "--bass-cutoff")
        .and_then(|value| value.parse::<f32>().ok())
    {
        store_f32(&params.bass_cutoff_hz, cutoff);
    }
    if let Some(strength) = parse_arg(args, "--dialog")
        .and_then(|value| value.parse::<u32>().ok())
    {
        params.dialog_on.store(true, Ordering::Relaxed);
        params
            .dialog_strength
            .store(strength, Ordering::Relaxed);
    }
    if let Some(level) = parse_arg(args, "--svm-level")
        .and_then(|value| value.parse::<f32>().ok())
    {
        params.svm_on.store(true, Ordering::Relaxed);
        store_f32(&params.svm_level_db, level);
    }
    if let Some(mix) = parse_arg(args, "--svm-mix")
        .and_then(|value| value.parse::<f32>().ok())
    {
        store_f32(&params.svm_mix, mix);
    }
    let eq_name = parse_arg(args, "--eq-preset");
    let eq_mode = match parse_arg(args, "--eq-mode").as_deref() {
        Some("speakers") => OutputMode::Speakers,
        _ => OutputMode::Headphones,
    };
    let eq = eq_name.as_deref().map(|name| (name, eq_mode));
    let engine = LiveEngine::start(&input_name, &output_name, eq, params.clone())?;
    println!(
        "selftest: {input_name} -> {output_name}, {seconds}s, eq {}",
        eq_name.as_deref().unwrap_or("off")
    );
    std::thread::sleep(Duration::from_secs(seconds));
    println!(
        "in rms {:.4} peak {:.4} | out rms {:.4} peak {:.4} | buffer {} ms | underruns {} overruns {}",
        load_f32(&params.input_rms),
        load_f32(&params.input_peak),
        load_f32(&params.output_rms),
        load_f32(&params.output_peak),
        params.ring_fill_ms.load(Ordering::Relaxed),
        params.underruns.load(Ordering::Relaxed),
        params.overruns.load(Ordering::Relaxed)
    );
    drop(engine);
    Ok(())
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--list-devices") {
        if let Err(err) = list_devices_cli() {
            eprintln!("sbc-live: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--selftest") {
        if let Err(err) = selftest(&args) {
            eprintln!("sbc-live: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sound Blaster Live")
            .with_inner_size([820.0, 560.0])
            .with_min_inner_size([720.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sound Blaster Live",
        options,
        Box::new(|cc| Ok(Box::new(LiveApp::new(cc)))),
    )
}
