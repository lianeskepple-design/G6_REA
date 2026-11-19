//! Shared 10-band EQ kernel for the G6 factory curves.
//!
//! Band topology follows `reference/apo-effects-structure.md`: 10 bands are
//! 1 low shelf (band 1) + 8 peaking (bands 2..9) + 1 high shelf (band 10).
//! The shared chain now uses the instruction-level coefficient port in
//! [`crate::apo_eq`] (APO VAs `0x180087120`, `0x180087970`, `0x1800887E0`).
//! `Biquad::design` below keeps the earlier RBJ implementation for comparison;
//! the UI curve, WAV renderer and CLI use the APO port.
//!
//! The desktop equalizer page and the `sbc-eq` CLI share this module, so the
//! curve drawn in the UI is the response the renderer actually applies.
//! This is not a claim about the G6 hardware DSP or Scout processing.

use crate::apo_eq;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use serde_json::Value;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::path::Path;

/// Factory preset database embedded at compile time (audited G6 data).
pub const PRESET_JSON: &str = include_str!("../../reference/g6-eq-presets-current.json");

/// Centre frequencies of the ten graphic-EQ bands, in Hz.
pub const BAND_FREQS: [f64; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub const BAND_COUNT: usize = BAND_FREQS.len();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    Speakers,
    Headphones,
}

impl OutputMode {
    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "speakers" => Some(OutputMode::Speakers),
            "headphones" => Some(OutputMode::Headphones),
            _ => None,
        }
    }

    /// Name used by the preset JSON (`settings` object key).
    pub fn key(self) -> &'static str {
        match self {
            OutputMode::Speakers => "Speaker",
            OutputMode::Headphones => "Headphone",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OutputMode::Speakers => "Speakers",
            OutputMode::Headphones => "Headphones",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub f: f64,
    pub gain_db: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub preamp_db: f64,
    pub bands: Vec<Band>,
}

impl Curve {
    /// Builds a curve from ten band gains on the standard band centres.
    pub fn from_bands(bands: &[f64; 10], preamp_db: f64) -> Self {
        Self {
            preamp_db,
            bands: BAND_FREQS
                .iter()
                .zip(bands.iter())
                .map(|(f, g)| Band { f: *f, gain_db: *g })
                .collect(),
        }
    }

    pub fn gains(&self) -> [f64; 10] {
        let mut out = [0.0; 10];
        for (slot, band) in out.iter_mut().zip(self.bands.iter()) {
            *slot = band.gain_db;
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct Preset {
    pub name: String,
    pub order: i64,
    pub speaker: Curve,
    pub headphone: Curve,
}

/// Every factory EQ preset, ordered like the original list.
pub fn presets() -> Result<Vec<Preset>, Box<dyn Error>> {
    let root: Value = serde_json::from_str(PRESET_JSON)?;
    let map = root["eqPresets"]
        .as_object()
        .ok_or("invalid EQ preset database")?;
    let mut out = Vec::with_capacity(map.len());
    for (name, item) in map {
        let order = item["order"].as_i64().unwrap_or(0);
        out.push(Preset {
            name: name.clone(),
            order,
            speaker: parse_curve(&item["settings"]["Speaker"], name)?,
            headphone: parse_curve(&item["settings"]["Headphone"], name)?,
        });
    }
    out.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// One preset's curve for the requested output mode.
pub fn preset_curve(name: &str, mode: OutputMode) -> Result<Curve, Box<dyn Error>> {
    let root: Value = serde_json::from_str(PRESET_JSON)?;
    let item = root["eqPresets"]
        .get(name)
        .ok_or_else(|| format!("unknown EQ preset: {name}"))?;
    parse_curve(&item["settings"][mode.key()], name)
}

fn parse_curve(settings: &Value, name: &str) -> Result<Curve, Box<dyn Error>> {
    let preamp = settings["preamp"]
        .as_f64()
        .ok_or_else(|| format!("preset {name} has an invalid preamp"))?;
    let freqs = settings["freqs"]
        .as_array()
        .ok_or_else(|| format!("preset {name} has invalid frequencies"))?;
    let values = settings["values"]
        .as_array()
        .ok_or_else(|| format!("preset {name} has invalid gains"))?;
    if freqs.len() != BAND_COUNT || values.len() != BAND_COUNT {
        return Err(format!("preset {name} must have {BAND_COUNT} bands").into());
    }
    if !preamp.is_finite() {
        return Err(format!("preset {name} contains NaN/Inf").into());
    }
    let mut bands = Vec::with_capacity(BAND_COUNT);
    for (f, g) in freqs.iter().zip(values) {
        let f = f.as_f64().ok_or("invalid frequency")?;
        let gain_db = g.as_f64().ok_or("invalid gain")?;
        if !f.is_finite() || !gain_db.is_finite() {
            return Err(format!("preset {name} contains NaN/Inf").into());
        }
        bands.push(Band { f, gain_db });
    }
    Ok(Curve { preamp_db: preamp, bands })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind {
    LowShelf,
    Peaking,
    HighShelf,
}

/// Band kind by position: first is a low shelf, last a high shelf, rest peaking.
pub fn band_kind(index: usize, count: usize) -> BandKind {
    if index == 0 {
        BandKind::LowShelf
    } else if index + 1 == count {
        BandKind::HighShelf
    } else {
        BandKind::Peaking
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Biquad {
    /// `ratio` is `f[i+1]/f[i]` for peaking sections; it must be finite and > 0.
    pub fn design(
        kind: BandKind,
        sample_rate: u32,
        f: f64,
        gain_db: f64,
        ratio: f64,
    ) -> Result<Self, Box<dyn Error>> {
        if sample_rate == 0 {
            return Err("sample rate must be positive".into());
        }
        if f >= sample_rate as f64 / 2.0 {
            return Err(format!(
                "sample rate {sample_rate} Hz cannot activate {f} Hz band (Nyquist)"
            )
            .into());
        }
        if !f.is_finite() || f <= 0.0 {
            return Err(format!("band frequency {f} Hz is not usable").into());
        }
        let a = 10f64.powf(gain_db / 40.0);
        if !a.is_finite() || a <= 0.0 {
            return Err(format!("band gain {gain_db} dB is outside the finite filter range").into());
        }
        let w = std::f64::consts::TAU * f / sample_rate as f64;
        let c = w.cos();
        match kind {
            BandKind::Peaking => {
                if !ratio.is_finite() || ratio <= 0.0 {
                    return Err(format!("band width ratio {ratio} is not usable").into());
                }
                let alpha = (w / (2.0 * ratio)).tan();
                if !alpha.is_finite() || alpha < 0.0 {
                    return Err(format!(
                        "band {f} Hz produced a non-finite bandwidth term"
                    )
                    .into());
                }
                let a0 = 1.0 + alpha / a;
                Ok(Self {
                    b0: (1.0 + alpha * a) / a0,
                    b1: (-2.0 * c) / a0,
                    b2: (1.0 - alpha * a) / a0,
                    a1: (-2.0 * c) / a0,
                    a2: (1.0 - alpha / a) / a0,
                })
            }
            BandKind::LowShelf | BandKind::HighShelf => {
                // RBJ shelf, slope S = 1.
                let alpha = w.sin() / 2.0 * 2f64.sqrt();
                let t = 2.0 * a.sqrt() * alpha;
                let (p1, m1) = (a - 1.0, a + 1.0);
                let (n, d) = match kind {
                    BandKind::LowShelf => (
                        (
                            a * (m1 - p1 * c + t),
                            2.0 * a * (p1 - m1 * c),
                            a * (m1 - p1 * c - t),
                        ),
                        (
                            m1 + p1 * c + t,
                            -2.0 * (p1 + m1 * c),
                            m1 + p1 * c - t,
                        ),
                    ),
                    _ => (
                        (
                            a * (m1 + p1 * c + t),
                            -2.0 * a * (p1 + m1 * c),
                            a * (m1 + p1 * c - t),
                        ),
                        (
                            m1 - p1 * c + t,
                            2.0 * (p1 - m1 * c),
                            m1 - p1 * c - t,
                        ),
                    ),
                };
                let a0 = d.0;
                if !a0.is_finite() || a0 == 0.0 {
                    return Err(format!("band {f} Hz produced a degenerate shelf").into());
                }
                Ok(Self {
                    b0: n.0 / a0,
                    b1: n.1 / a0,
                    b2: n.2 / a0,
                    a1: d.1 / a0,
                    a2: d.2 / a0,
                })
            }
        }
    }

    /// Direct-form I, one channel state.
    pub fn run(self, x: f64, z: &mut [f64; 4]) -> f64 {
        let y = self.b0 * x + self.b1 * z[0] + self.b2 * z[1] - self.a1 * z[2] - self.a2 * z[3];
        z[1] = z[0];
        z[0] = x;
        z[3] = z[2];
        z[2] = y;
        y
    }

    /// |H(e^{jw})| at angular frequency `w`.
    pub fn magnitude(&self, w: f64) -> f64 {
        let (c, s) = (w.cos(), w.sin());
        let (z1r, z1i) = (c, -s);
        let (z2r, z2i) = (c * c - s * s, -2.0 * c * s);
        let nr = self.b0 + self.b1 * z1r + self.b2 * z2r;
        let ni = self.b1 * z1i + self.b2 * z2i;
        let dr = 1.0 + self.a1 * z1r + self.a2 * z2r;
        let di = self.a1 * z1i + self.a2 * z2i;
        ((nr * nr + ni * ni) / (dr * dr + di * di)).sqrt()
    }

    pub fn magnitude_db(&self, w: f64) -> f64 {
        20.0 * self.magnitude(w).log10()
    }
}

/// A designed filter chain plus its preamp/headroom gain.
#[derive(Clone, Debug)]
pub struct Chain {
    filters: Vec<Biquad>,
    gain: f64,
}

impl Chain {
    /// Designs the chain for `curve` at `sample_rate`, applying
    /// `curve.preamp_db + headroom_db` as a linear input gain. Bands whose
    /// gain is exactly 0 dB are skipped.
    pub fn design(
        curve: &Curve,
        sample_rate: u32,
        headroom_db: f64,
    ) -> Result<Self, Box<dyn Error>> {
        if curve.bands.len() != BAND_COUNT {
            return Err(format!("curve must have {BAND_COUNT} bands").into());
        }
        let gain = linear_gain(curve.preamp_db, headroom_db)?;
        let mut filters = Vec::with_capacity(BAND_COUNT);
        for (i, band) in curve.bands.iter().enumerate() {
            if !band.f.is_finite() || !band.gain_db.is_finite() {
                return Err("curve contains NaN/Inf".into());
            }
            if band.gain_db == 0.0 {
                continue;
            }
            let kind = band_kind(i, curve.bands.len());
            let ratio = if kind == BandKind::Peaking {
                curve.bands[i + 1].f / band.f
            } else {
                1.0
            };
            let designed = match kind {
                BandKind::Peaking => apo_eq::peak_from_db(
                    band.gain_db as f32,
                    sample_rate,
                    band.f as f32,
                    ratio as f32,
                ),
                BandKind::LowShelf => {
                    apo_eq::low_shelf_from_db(band.gain_db as f32, sample_rate, band.f as f32)
                }
                BandKind::HighShelf => {
                    apo_eq::high_shelf_from_db(band.gain_db as f32, sample_rate, band.f as f32)
                }
            }
            .map_err(|message| -> Box<dyn Error> { message.into() })?;
            filters.push(designed.to_biquad());
        }
        Ok(Self { filters, gain })
    }

    pub fn gain(&self) -> f64 {
        self.gain
    }

    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    /// Fresh per-channel state, one `[f64; 4]` per filter.
    pub fn new_states(&self) -> Vec<[f64; 4]> {
        vec![[0.0; 4]; self.filters.len()]
    }

    /// Runs one channel's sample through every section.
    pub fn process_channel(&self, x: f64, states: &mut [[f64; 4]]) -> f64 {
        let mut y = x * self.gain;
        for (filter, state) in self.filters.iter().zip(states.iter_mut()) {
            y = filter.run(y, state);
        }
        y
    }

    /// Magnitude response in dB at `freq` Hz, including the input gain.
    pub fn response_db(&self, freq: f64, sample_rate: f64) -> f64 {
        let nyquist = sample_rate / 2.0;
        let f = freq.clamp(0.0, nyquist);
        let w = std::f64::consts::TAU * f / sample_rate;
        let mut db = 20.0 * self.gain.log10();
        for filter in &self.filters {
            db += filter.magnitude_db(w);
        }
        db
    }
}

/// Linear gain for the signed dB value added to the preset preamp. Rejects
/// non-finite/overflowed powers before any output path is created.
pub fn linear_gain(preamp_db: f64, headroom_db: f64) -> Result<f64, Box<dyn Error>> {
    let total_db = preamp_db + headroom_db;
    if !total_db.is_finite() {
        return Err(format!(
            "combined preamp+headroom gain ({preamp_db} + {headroom_db} dB) is not finite"
        )
        .into());
    }
    let gain = 10f64.powf(total_db / 20.0);
    if !gain.is_finite() {
        return Err(format!(
            "combined preamp+headroom gain ({preamp_db} + {headroom_db} dB) overflows the finite gain range"
        )
        .into());
    }
    if gain > f32::MAX as f64 {
        return Err(format!(
            "combined preamp+headroom gain ({preamp_db} + {headroom_db} dB = {gain}) exceeds the float32 sample range; reduce the headroom"
        )
        .into());
    }
    Ok(gain)
}

/// The f64 pipeline is checked first; the float32 cast is a second, independent
/// overflow point (+1000 dB is finite in f64 but becomes +inf in f32).
pub fn finite_f32(value: f64, index: usize) -> Result<f32, Box<dyn Error>> {
    let sample = value as f32;
    if !sample.is_finite() {
        return Err(format!(
            "sample {index} is {sample} after float32 conversion; reduce the headroom"
        )
        .into());
    }
    Ok(sample)
}

pub fn same_file(input: &Path, output: &Path) -> bool {
    if input == output
        || input
            .canonicalize()
            .ok()
            .zip(output.canonicalize().ok())
            .map(|(a, b)| a == b)
            .unwrap_or(false)
    {
        return true;
    }
    false
}

fn decode_samples(
    reader: &mut WavReader<std::io::BufReader<std::fs::File>>,
    bits: u16,
    fmt: SampleFormat,
) -> Result<Vec<f32>, Box<dyn Error>> {
    let mut out = Vec::new();
    match (fmt, bits) {
        (SampleFormat::Float, 32) => {
            for s in reader.samples::<f32>() {
                let v = s?;
                if !v.is_finite() {
                    return Err("input WAV contains NaN/Inf".into());
                }
                out.push(v);
            }
        }
        (SampleFormat::Int, 16) => {
            for s in reader.samples::<i16>() {
                out.push(s? as f32 / 32768.0);
            }
        }
        (SampleFormat::Int, 24) => {
            for s in reader.samples::<i32>() {
                out.push(s? as f32 / 8388608.0);
            }
        }
        (SampleFormat::Int, 32) => {
            for s in reader.samples::<i32>() {
                out.push(s? as f32 / 2147483648.0);
            }
        }
        _ => {
            return Err(format!(
                "unsupported WAV encoding: {fmt:?} {bits}-bit"
            )
            .into())
        }
    }
    Ok(out)
}

/// Decodes a WAV into float32 samples and returns its spec.
///
/// Shared by the DSP CLI; the EQ-only renderer keeps its own path so its
/// output-file safety rules stay unchanged.
pub fn load_wav_f32(input: &Path) -> Result<(WavSpec, Vec<f32>), Box<dyn Error>> {
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("invalid WAV sample rate or channel count".into());
    }
    let samples = decode_samples(&mut reader, spec.bits_per_sample, spec.sample_format)?;
    if samples.len() % spec.channels as usize != 0 {
        return Err("sample count is not channel aligned".into());
    }
    Ok((spec, samples))
}

/// Writes float32 samples with `create_new`, removing a partial file on error.
pub fn write_wav_f32(
    output: &Path,
    spec: WavSpec,
    samples: &[f32],
) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new().write(true).create_new(true).open(output)?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        let writer = BufWriter::new(file);
        let out_spec = WavSpec {
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut w = WavWriter::new(writer, out_spec)?;
        for &s in samples {
            w.write_sample(s)?;
        }
        w.finalize()?;
        Ok(())
    })();
    if let Err(err) = result {
        if let Err(remove_err) = std::fs::remove_file(output) {
            return Err(format!(
                "{err}; additionally failed to remove partial output '{}': {remove_err}",
                output.display()
            )
            .into());
        }
        return Err(err);
    }
    Ok(())
}

/// Renders `input` through `curve` into a float32 `output` WAV.
///
/// The output file is only created after the whole input has been decoded,
/// validated and processed, using `create_new` so an existing file is never
/// overwritten. A failed write removes the partial file.
pub fn render_wav(
    input: &Path,
    output: &Path,
    curve: &Curve,
    headroom_db: f64,
) -> Result<(), Box<dyn Error>> {
    if !headroom_db.is_finite() {
        return Err("headroom must be finite".into());
    }
    if same_file(input, output) {
        return Err("input and output refer to the same file".into());
    }
    if output.exists() {
        return Err(format!("output already exists: {}", output.display()).into());
    }
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("invalid WAV sample rate or channel count".into());
    }
    let mut samples = decode_samples(&mut reader, spec.bits_per_sample, spec.sample_format)?;
    if samples.len() % spec.channels as usize != 0 {
        return Err("sample count is not channel aligned".into());
    }
    let chain = Chain::design(curve, spec.sample_rate, headroom_db)?;
    let channels = spec.channels as usize;
    let per_channel = chain.filter_count();
    let mut states = vec![[0.0f64; 4]; channels * per_channel];
    for (i, x) in samples.iter_mut().enumerate() {
        let ch = i % channels;
        let slice = &mut states[ch * per_channel..(ch + 1) * per_channel];
        let y = chain.process_channel(*x as f64, slice);
        if !y.is_finite() {
            return Err("processing produced NaN/Inf".into());
        }
        *x = finite_f32(y, i)?;
    }
    // Create only after full input decoding, preset validation, and processing.
    let file = OpenOptions::new().write(true).create_new(true).open(output)?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        let writer = BufWriter::new(file);
        let out_spec = WavSpec {
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut w = WavWriter::new(writer, out_spec)?;
        for s in samples {
            w.write_sample(s)?;
        }
        w.finalize()?;
        Ok(())
    })();
    if let Err(err) = result {
        if let Err(remove_err) = std::fs::remove_file(output) {
            return Err(format!(
                "{err}; additionally failed to remove partial output '{}': {remove_err}",
                output.display()
            )
            .into());
        }
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(x: f64) -> f64 {
        20.0 * x.log10()
    }

    #[test]
    fn loads_all_43_presets() {
        let all = presets().unwrap();
        assert_eq!(all.len(), 43);
        for preset in &all {
            assert_eq!(preset.speaker.bands.len(), BAND_COUNT);
            assert_eq!(preset.headphone.bands.len(), BAND_COUNT);
        }
    }

    #[test]
    fn presets_are_ordered_and_unique() {
        let all = presets().unwrap();
        let mut names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len(), "preset names must be unique");
        assert!(
            all.windows(2).all(|w| w[0].order <= w[1].order),
            "presets must be sorted by their original order value"
        );
        assert_eq!(all[0].name, "Acoustic");
    }

    #[test]
    fn flat_curve_is_identity() {
        let b = Biquad::design(BandKind::Peaking, 48000, 1000.0, 0.0, 2.0).unwrap();
        let mut z = [0.0; 4];
        for x in [0.25, -0.4, 0.8] {
            assert!((b.run(x, &mut z) - x).abs() < 1e-12);
        }
    }

    #[test]
    fn peaking_hits_target_gain_and_keeps_unity_ends() {
        let f = 1000.0;
        let w = std::f64::consts::TAU * f / 48000.0;
        for g in [6.0, -6.0, 12.0] {
            let b = Biquad::design(BandKind::Peaking, 48000, f, g, 2.0).unwrap();
            assert!((b.magnitude_db(w) - g).abs() < 1e-9, "peak {g} dB centre");
            assert!(b.magnitude_db(1e-9).abs() < 1e-9, "peak {g} dB DC");
            assert!(
                b.magnitude_db(std::f64::consts::PI).abs() < 1e-9,
                "peak {g} dB Nyquist"
            );
        }
    }

    #[test]
    fn shelves_reach_target_at_their_end() {
        let lo = Biquad::design(BandKind::LowShelf, 48000, 31.0, 6.0, 1.0).unwrap();
        assert!((lo.magnitude_db(1e-9) - 6.0).abs() < 1e-9, "low shelf DC");
        assert!(
            lo.magnitude_db(std::f64::consts::PI).abs() < 1e-9,
            "low shelf Nyquist"
        );
        let hi = Biquad::design(BandKind::HighShelf, 48000, 16000.0, 6.0, 1.0).unwrap();
        assert!(
            (hi.magnitude_db(std::f64::consts::PI) - 6.0).abs() < 1e-9,
            "high shelf Nyquist"
        );
        assert!(hi.magnitude_db(1e-9).abs() < 1e-9, "high shelf DC");
    }

    #[test]
    fn bandwidth_ratio_is_validated() {
        assert!(Biquad::design(BandKind::Peaking, 48000, 1000.0, 3.0, 0.0).is_err());
        assert!(Biquad::design(BandKind::Peaking, 48000, 1000.0, 3.0, f64::NAN).is_err());
        assert!(Biquad::design(BandKind::Peaking, 48000, 1000.0, 3.0, -2.0).is_err());
    }

    #[test]
    fn headphone_and_speaker_curves_are_distinct() {
        let s = preset_curve("AdventureAndAction", OutputMode::Speakers).unwrap();
        let h = preset_curve("AdventureAndAction", OutputMode::Headphones).unwrap();
        assert_ne!(s.gains(), h.gains());
    }

    #[test]
    fn nyquist_band_rejected() {
        assert!(Biquad::design(BandKind::Peaking, 32000, 16000.0, 2.0, 2.0).is_err());
    }

    #[test]
    fn nonfinite_and_overflowing_gain_rejected() {
        assert!(linear_gain(0.0, f64::NAN).is_err());
        assert!(linear_gain(0.0, f64::NEG_INFINITY).is_err());
        assert!(linear_gain(0.0, f64::INFINITY).is_err());
        assert!(linear_gain(0.0, 1e10).is_err());
        assert!(linear_gain(0.0, 1000.0).is_err());
        assert!(linear_gain(0.0, 8000.0).is_err());
        assert!(linear_gain(f64::MAX, f64::MAX).is_err());
        let minus_six = linear_gain(0.0, -6.0).unwrap();
        assert!((minus_six - 0.501_187_233_627_276_7).abs() < 1e-12);
        assert!(linear_gain(0.0, 6.0).unwrap() > 1.0);
    }

    #[test]
    fn float32_conversion_rejects_overflow() {
        assert!(finite_f32(1.0e300, 0).is_err());
        assert!(finite_f32(f64::from(f32::MAX) * 2.0, 1).is_err());
        assert_eq!(finite_f32(0.25, 2).unwrap(), 0.25);
    }

    #[test]
    fn chain_response_matches_band_gains() {
        // Only the 1 kHz band moves: the chain must hit +6 dB there and stay
        // near unity far away from it.
        let mut gains = [0.0; 10];
        gains[5] = 6.0;
        let curve = Curve::from_bands(&gains, 0.0);
        let chain = Chain::design(&curve, 48000, 0.0).unwrap();
        let peak = chain.response_db(1000.0, 48000.0);
        assert!((peak - 6.0).abs() < 0.02, "expected 6 dB, got {peak}");
        assert!(chain.response_db(5.0, 48000.0).abs() < 0.5);
        assert!(chain.response_db(20000.0, 48000.0).abs() < 0.5);
    }

    #[test]
    fn chain_response_includes_preamp_and_headroom() {
        let curve = Curve::from_bands(&[0.0; 10], 0.0);
        let chain = Chain::design(&curve, 48000, -6.0).unwrap();
        assert_eq!(chain.filter_count(), 0);
        assert!((chain.response_db(1000.0, 48000.0) + 6.0).abs() < 1e-9);
        assert!((db(chain.gain()) + 6.0).abs() < 1e-12);
    }

    #[test]
    fn factory_curve_response_is_finite() {
        for preset in presets().unwrap() {
            for curve in [preset.speaker, preset.headphone] {
                let chain = Chain::design(&curve, 48000, 0.0).unwrap();
                for i in 0..64 {
                    let f = 31.0 * (16000.0f64 / 31.0).powf(i as f64 / 63.0);
                    assert!(chain.response_db(f, 48000.0).is_finite());
                }
            }
        }
    }
}
