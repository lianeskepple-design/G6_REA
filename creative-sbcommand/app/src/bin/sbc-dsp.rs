//! Offline WAV DSP-chain CLI.
//!
//! Runs the Rust APO EQ kernel followed by the C reference stages:
//! Crystalizer -> Surround (6-channel only) -> Smart Volume. The C side is
//! reached through `reference/g6_chain_capi.h`; no C struct layout is mirrored
//! in Rust.
use aurora_audio_command::eq_core::{self, Chain, OutputMode};
use aurora_audio_command::sbx_modes::{EffectLevels, Mode, ProfileSet};
use std::env;
use std::error::Error;
use std::path::PathBuf;

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
    fn g6_chain_set_bass_mode(chain: *mut G6Chain, mode: i32) -> i32;
    fn g6_chain_set_dialog(chain: *mut G6Chain, enabled: i32, strength: i32) -> i32;
    fn g6_chain_set_surround(
        chain: *mut G6Chain,
        enabled: i32,
        mask: u32,
        strength: i32,
    ) -> i32;
    fn g6_chain_set_svm(chain: *mut G6Chain, enabled: i32, level_db: f32, mix: f32) -> i32;
    fn g6_chain_set_svm_delay(chain: *mut G6Chain, seconds: f32) -> i32;
    fn g6_chain_process_interleaved(
        chain: *mut G6Chain,
        samples: *mut f32,
        frames: usize,
    ) -> i32;
}

struct ChainHandle(*mut G6Chain);

impl ChainHandle {
    fn new(sample_rate: u32, channels: u32) -> Result<Self, Box<dyn Error>> {
        let ptr = unsafe { g6_chain_create(sample_rate, channels) };
        if ptr.is_null() {
            return Err("failed to create the C DSP chain".into());
        }
        Ok(Self(ptr))
    }

    fn set_crystalizer(&self, strength: u32) -> Result<(), Box<dyn Error>> {
        if strength > 100 {
            return Err("Crystalizer strength must be 0..100".into());
        }
        if unsafe { g6_chain_set_crystalizer(self.0, 1, strength) } == 0 {
            return Err("failed to configure Crystalizer".into());
        }
        Ok(())
    }

    fn set_bass(&self, strength: i32, cutoff_hz: f32) -> Result<(), Box<dyn Error>> {
        if !(0..=100).contains(&strength) {
            return Err("Bass strength must be 0..100".into());
        }
        if !cutoff_hz.is_finite() || !(40.0..=400.0).contains(&cutoff_hz) {
            return Err("Bass cutoff must be 40..400 Hz".into());
        }
        if unsafe { g6_chain_set_bass(self.0, 1, strength, cutoff_hz) } == 0 {
            return Err("failed to configure Bass".into());
        }
        Ok(())
    }

    fn set_bass_mode(&self, mode: i32) -> Result<(), Box<dyn Error>> {
        if !(0..=1).contains(&mode) {
            return Err("Bass mode must be reference or apo-legacy".into());
        }
        if unsafe { g6_chain_set_bass_mode(self.0, mode) } == 0 {
            return Err("failed to configure Bass mode".into());
        }
        Ok(())
    }

    fn set_dialog(&self, strength: i32) -> Result<(), Box<dyn Error>> {
        if !(0..=100).contains(&strength) {
            return Err("Dialog+ strength must be 0..100".into());
        }
        if unsafe { g6_chain_set_dialog(self.0, 1, strength) } == 0 {
            return Err("failed to configure Dialog+".into());
        }
        Ok(())
    }

    fn set_surround(&self, mask: u32, strength: i32) -> Result<(), Box<dyn Error>> {
        if !(0..=100).contains(&strength) {
            return Err("Surround strength must be 0..100".into());
        }
        if unsafe { g6_chain_set_surround(self.0, 1, mask, strength) } == 0 {
            return Err("Surround requires 6-channel input".into());
        }
        Ok(())
    }

    fn set_svm(&self, level_db: f32, mix: f32) -> Result<(), Box<dyn Error>> {
        if !level_db.is_finite() {
            return Err("SVM level must be finite".into());
        }
        if !(0.0..=1.0).contains(&mix) {
            return Err("SVM mix must be 0..1".into());
        }
        if unsafe { g6_chain_set_svm(self.0, 1, level_db, mix) } == 0 {
            return Err("failed to configure Smart Volume".into());
        }
        Ok(())
    }

    fn set_svm_delay(&self, seconds: f32) -> Result<(), Box<dyn Error>> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("SVM delay must be a non-negative number of seconds".into());
        }
        if unsafe { g6_chain_set_svm_delay(self.0, seconds) } == 0 {
            return Err("failed to set the Smart Volume delay".into());
        }
        Ok(())
    }

    fn process(&self, samples: &mut [f32], channels: usize) -> Result<(), Box<dyn Error>> {
        if channels == 0 || samples.len() % channels != 0 {
            return Err("sample count is not channel aligned".into());
        }
        let frames = samples.len() / channels;
        if unsafe { g6_chain_process_interleaved(self.0, samples.as_mut_ptr(), frames) } == 0 {
            return Err("C DSP chain processing failed".into());
        }
        Ok(())
    }
}

impl Drop for ChainHandle {
    fn drop(&mut self) {
        unsafe { g6_chain_destroy(self.0) };
    }
}

fn usage() -> &'static str {
    "Usage:\n  sbc-dsp --process-wav input.wav output.wav [options]\n\n\
Stages run in this order: EQ -> Crystalizer -> Bass -> Dialog+ -> Surround -> SVM.\n\n\
Options:\n\
  --eq-preset NAME           apply a factory EQ preset\n\
  --output-mode MODE         headphones|speakers (required with --eq-preset)\n\
  --headroom-db N            signed dB added to the preset preamp (default 0)\n\
  --crystalizer N            Crystalizer strength 0..100\n\
  --bass N                   Missing-fundamental Bass strength 0..100\n\
  --bass-cutoff HZ           Bass cutoff 40..400 Hz (default 120)\n\
  --bass-mode MODE           reference|apo-legacy (default reference)\n\
  --dialog N                 Dialog+ strength 0..100\n\
  --surround-mask MASK       CMSS3D mask, decimal or 0x.. (requires 6 channels)\n\
  --surround-strength N      Surround strength 0..100 (required with mask)\n\
  --svm-level DB             Smart Volume threshold, legacy (level_db-12)/10\n\
  --svm-mix MIX              Smart Volume mix 0..1 (default 1.0)\n\
  --svm-delay-ms MS          Smart Volume look-ahead in ms (DLL default 20, max 50)\n\
  --mode MODE                direct|sbx|scout (Direct and Scout bypass everything)\n\
  --sbx-profile NAME         apply a factory SBX profile (implies --mode sbx)\n\n\
Input/output must differ and the output must not exist. Output is float32 WAV."
}

fn parse_mask(text: &str) -> Result<u32, Box<dyn Error>> {
    let value = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)?
    } else {
        text.parse()?
    };
    Ok(value)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 3 || args[0] != "--process-wav" {
        return Err(usage().into());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    if eq_core::same_file(&input, &output) {
        return Err("input and output refer to the same file".into());
    }
    if output.exists() {
        return Err(format!("output already exists: {}", output.display()).into());
    }

    let mut eq_preset: Option<String> = None;
    let mut output_mode: Option<String> = None;
    let mut headroom = 0.0f64;
    let mut crystalizer: Option<u32> = None;
    let mut bass = Option::<i32>::None;
    let mut bass_cutoff = 120.0f32;
    let mut bass_mode = 0i32;
    let mut dialog: Option<i32> = None;
    let mut surround_mask: Option<u32> = None;
    let mut surround_strength: Option<i32> = None;
    let mut svm_level: Option<f32> = None;
    let mut svm_mix = 1.0f32;
    let mut svm_delay_ms: Option<f32> = None;
    let mut mode_text: Option<String> = None;
    let mut sbx_profile: Option<String> = None;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--eq-preset" if i + 1 < args.len() => {
                eq_preset = Some(args[i + 1].clone());
                i += 2;
            }
            "--output-mode" if i + 1 < args.len() => {
                output_mode = Some(args[i + 1].clone());
                i += 2;
            }
            "--headroom-db" if i + 1 < args.len() => {
                headroom = args[i + 1].parse()?;
                i += 2;
            }
            "--crystalizer" if i + 1 < args.len() => {
                crystalizer = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--bass" if i + 1 < args.len() => {
                bass = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--bass-cutoff" if i + 1 < args.len() => {
                bass_cutoff = args[i + 1].parse()?;
                i += 2;
            }
            "--bass-mode" if i + 1 < args.len() => {
                bass_mode = match args[i + 1].as_str() {
                    "reference" => 0,
                    "apo" | "apo-legacy" => {
                        eprintln!(
                            "warning: --bass-mode apo-legacy is a legacy \
                             misattribution (FIR/cubic run on the APO limiter \
                             envelope, not audio); use reference for parity"
                        );
                        1
                    }
                    _ => {
                        return Err(
                            "--bass-mode must be reference or apo-legacy".into()
                        )
                    }
                };
                i += 2;
            }
            "--dialog" if i + 1 < args.len() => {
                dialog = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--surround-mask" if i + 1 < args.len() => {
                surround_mask = Some(parse_mask(&args[i + 1])?);
                i += 2;
            }
            "--surround-strength" if i + 1 < args.len() => {
                surround_strength = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--svm-level" if i + 1 < args.len() => {
                svm_level = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--mode" if i + 1 < args.len() => {
                mode_text = Some(args[i + 1].clone());
                i += 2;
            }
            "--sbx-profile" if i + 1 < args.len() => {
                sbx_profile = Some(args[i + 1].clone());
                i += 2;
            }
            "--svm-delay-ms" if i + 1 < args.len() => {
                svm_delay_ms = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--svm-mix" if i + 1 < args.len() => {
                svm_mix = args[i + 1].parse()?;
                i += 2;
            }
            _ => return Err(format!("unknown argument: {}", args[i]).into()),
        }
    }

    if eq_preset.is_some() != output_mode.is_some() {
        return Err("--eq-preset and --output-mode must be used together".into());
    }
    if surround_mask.is_some() != surround_strength.is_some() {
        return Err("--surround-mask and --surround-strength must be used together".into());
    }
    if bass.is_none() && bass_cutoff != 120.0 {
        return Err("--bass-cutoff requires --bass".into());
    }
    if bass.is_none() && bass_mode != 0 {
        return Err("--bass-mode requires --bass".into());
    }

    /* Resolve SBX / Scout / Direct before anything is applied. */
    let (levels, mode_note) = match (&mode_text, &sbx_profile) {
        (None, None) => (None, String::new()),
        (Some(text), profile) => {
            let mode = match text.to_ascii_lowercase().as_str() {
                "direct" => Mode::Direct,
                "scout" => Mode::Scout,
                "sbx" => Mode::Sbx,
                other => {
                    return Err(format!("unknown mode {other}; use direct, sbx or scout").into())
                }
            };
            let levels = match (mode, profile) {
                (Mode::Sbx, None) => return Err("--mode sbx needs --sbx-profile NAME".into()),
                (Mode::Direct, Some(_)) | (Mode::Scout, Some(_)) => {
                    return Err("Direct and Scout bypass SBX, so --sbx-profile does not apply"
                        .into())
                }
                (Mode::Direct, None) => EffectLevels::direct(),
                (Mode::Scout, None) => EffectLevels::scout(),
                (Mode::Sbx, Some(name)) => {
                    let profiles = ProfileSet::load().map_err(|err| err.to_string())?;
                    let profile = profiles
                        .find(name)
                        .ok_or_else(|| format!("unknown SBX profile {name}"))?;
                    EffectLevels::from_profile(profile)
                }
            };
            let note = match mode {
                Mode::Direct => "mode direct (every effect bypassed)".to_string(),
                Mode::Scout => "mode scout (SBX off; the Scout DSP is device-side and not implemented)"
                    .to_string(),
                Mode::Sbx => format!("mode sbx profile {}", profile.as_deref().unwrap_or("")),
            };
            let explicit_effect = crystalizer.is_some()
                || bass.is_some()
                || dialog.is_some()
                || surround_mask.is_some()
                || svm_level.is_some();
            if levels.bypass_all() && (explicit_effect || eq_preset.is_some()) {
                return Err(
                    "Direct and Scout bypass every effect, including the equalizer; drop the effect flags"
                        .into(),
                );
            }
            (Some(levels), note)
        }
        (None, Some(name)) => {
            let profiles = ProfileSet::load().map_err(|err| err.to_string())?;
            let profile = profiles
                .find(name)
                .ok_or_else(|| format!("unknown SBX profile {name}"))?;
            (
                Some(EffectLevels::from_profile(profile)),
                format!("mode sbx profile {}", profile.name),
            )
        }
    };

    let (spec, mut samples) = eq_core::load_wav_f32(&input)?;
    let channels = spec.channels as usize;
    if channels == 0 || channels > 8 {
        return Err("only 1..8 channel WAV files are supported".into());
    }

    if let Some(name) = eq_preset {
        let mode_text = output_mode.as_deref().unwrap_or("");
        let mode = OutputMode::parse(mode_text)
            .ok_or("output mode must be headphones or speakers")?;
        let curve = eq_core::preset_curve(&name, mode)?;
        let chain = Chain::design(&curve, spec.sample_rate, headroom)?;
        let per_channel = chain.filter_count();
        let mut states = vec![[0.0f64; 4]; channels * per_channel];
        for (index, sample) in samples.iter_mut().enumerate() {
            let channel = index % channels;
            let state = &mut states[channel * per_channel..(channel + 1) * per_channel];
            let value = chain.process_channel(*sample as f64, state);
            *sample = eq_core::finite_f32(value, index)?;
        }
    }

    let handle = ChainHandle::new(spec.sample_rate, spec.channels as u32)?;
    if let Some(levels) = levels.as_ref() {
        if !levels.bypass_all() {
            if levels.crystalizer_on {
                handle.set_crystalizer(levels.crystalizer as u32)?;
            }
            if levels.bass_on {
                handle.set_bass(levels.bass as i32, levels.bass_crossover as f32)?;
                handle.set_bass_mode(bass_mode)?;
            }
            if levels.dialog_on {
                handle.set_dialog(levels.dialog as i32)?;
            }
            if levels.surround_on {
                if channels != 6 {
                    /* The factory profiles do not carry a mask; 0x3f is 5.1. */
                    eprintln!("sbc-dsp: profile surround skipped, input is not 6-channel");
                } else {
                    handle.set_surround(0x3f, levels.surround as i32)?;
                }
            }
            if levels.smart_volume_on {
                handle.set_svm(levels.smart_volume_db(), svm_mix)?;
            }
        }
    }
    if let Some(strength) = crystalizer {
        handle.set_crystalizer(strength)?;
    }
    if let Some(strength) = bass {
        handle.set_bass(strength, bass_cutoff)?;
        handle.set_bass_mode(bass_mode)?;
    }
    if let Some(strength) = dialog {
        handle.set_dialog(strength)?;
    }
    if let (Some(mask), Some(strength)) = (surround_mask, surround_strength) {
        if channels != 6 {
            return Err("Surround requires 6-channel input".into());
        }
        handle.set_surround(mask, strength)?;
    }
    if let Some(ms) = svm_delay_ms {
        handle.set_svm_delay(ms / 1000.0).map_err(|err| err.to_string())?;
    }
    if let Some(level) = svm_level {
        handle.set_svm(level, svm_mix)?;
    }
    handle.process(&mut samples, channels)?;
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("processing produced NaN/Inf".into());
    }
    eq_core::write_wav_f32(&output, spec, &samples)?;

    if !mode_note.is_empty() {
        println!("{mode_note}");
    }
    println!(
        "wrote {} ({} Hz, {} ch, {} frames)",
        output.display(),
        spec.sample_rate,
        channels,
        samples.len() / channels
    );
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("sbc-dsp: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_chain_has_lookahead_then_outputs_signal() {
        let handle = ChainHandle::new(48000, 2).unwrap();
        handle.set_crystalizer(50).unwrap();
        handle.set_svm(-20.0, 1.0).unwrap();
        /*
         * Smart Volume runs a 20 ms look-ahead (960 frames at 48 kHz) and the
         * Crystalizer block adds 128 more, so the first blocks are silent by
         * construction; the tail has to carry the signal.
         */
        let mut head = vec![0.1f32; 512 * 2];
        handle.process(&mut head, 2).unwrap();
        assert!(head.iter().all(|sample| *sample == 0.0));

        let mut samples = vec![0.1f32; 4096 * 2];
        handle.process(&mut samples, 2).unwrap();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn surround_needs_six_channels() {
        let handle = ChainHandle::new(48000, 2).unwrap();
        assert!(handle.set_surround(0x3f, 50).is_err());
    }

    #[test]
    fn six_channel_surround_is_finite() {
        let handle = ChainHandle::new(48000, 6).unwrap();
        handle.set_surround(0x3f, 50).unwrap();
        let mut samples = vec![0.05f32; 256 * 6];
        handle.process(&mut samples, 6).unwrap();
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }
}
