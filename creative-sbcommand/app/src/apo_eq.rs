//! Instruction-level port of the G6 APO 10-band EQ coefficient functions.
//!
//! Evidence: `KSUSBAPO64.dll` 1.2.109.0, SHA-256
//! `f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156`.
//!
//! Recovered functions (static VAs, image base `0x180000000`):
//!
//! | function | VA | role |
//! |---|---|---|
//! | peak coefficients | `0x180087120` | `CEqFilterPeak` vtable `+0x18` |
//! | low shelf coefficients | `0x180087970` | `CEqFilterLowFreq` vtable `+0x18` |
//! | high shelf coefficients | `0x1800887e0` | `CEqFilterHighFreq` vtable `+0x18` |
//! | per-sample kernel | `0x1801469e0` | direct-form II, 4-sample unrolled |
//!
//! The band object stores five `f32` slots consumed by the kernel:
//!
//! ```text
//! +0x08 = c8  = -a1
//! +0x0c = c12 = -a2
//! +0x10 = c16 = b0
//! +0x14 = c20 = b1
//! +0x18 = c24 = b2
//! +0x1c = w[n-1]
//! +0x20 = w[n-2]
//! ```
//!
//! and the kernel is:
//!
//! ```text
//! w[n] = x[n] + c8*w[n-1] + c12*w[n-2]
//! y[n] = c16*w[n] + c20*w[n-1] + c24*w[n-2]
//! ```
//!
//! This is a normal direct-form II biquad with
//! `b = [c16, c20, c24]` and `a = [1, -c8, -c12]`.
//!
//! ## Gain convention
//!
//! The DLL stores a linear value in `[B+0x28]`; the conversion from the
//! property value (the UI sends dB) happens before the coefficient call and
//! has not been located. The two recovered formulas fix the convention per
//! filter type: the peaking response reaches `sqrt(gain)` at its centre, and
//! the shelf plateau reaches `gain`. To make the document-level dB label
//! correct for both types this module converts:
//!
//! * peaking: `gain = 10^(dB/10)` (power), so `sqrt(gain) = 10^(dB/20)`;
//! * shelves: `gain = 10^(dB/20)` (amplitude), so the plateau is `10^(dB/20)`.
//!
//! `*_from_gain` entry points accept the raw linear value so callers that
//! recover the real DLL conversion can bypass the inference.

use crate::eq_core::Biquad;

/// `2π` rounded to `f32`, matching the constant at `0x1801916C4`.
const TAU_F32: f32 = 6.283_185_5;
/// `0.707` (`0x18018BB18`) used by both shelf formulas.
const SHELF_MID: f32 = 0.707;
/// `1.4` (`0x1801916B8`) used by both shelf formulas.
const SHELF_BASE: f32 = 1.4;
/// `0.4` (`0x1801916A4`), low shelf bandwidth term.
const LOW_SHELF_WIDTH: f32 = 0.4;
/// `2.0` (`0x180187B54`), low shelf `2/3` and high shelf `1/r` gain terms.
const TWO: f32 = 2.0;
/// `3.0` (`0x1801881B8`), low shelf `2/3` frequency term.
const THREE: f32 = 3.0;
/// `0.45` (`0x1801916A8`), high shelf frequency term.
const HIGH_SHELF_WIDTH: f32 = 0.45;

/// The five coefficient slots written by the three coefficient functions.
///
/// `c8/c12` are the negated denominator coefficients; `c16/c20/c24` are the
/// numerator coefficients. The field order matches the DLL store order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApoCoefficients {
    /// `[B+0x08]`, `-a1`.
    pub c8: f32,
    /// `[B+0x0c]`, `-a2`.
    pub c12: f32,
    /// `[B+0x10]`, `b0`.
    pub c16: f32,
    /// `[B+0x14]`, `b1`.
    pub c20: f32,
    /// `[B+0x18]`, `b2`.
    pub c24: f32,
}

impl ApoCoefficients {
    /// Coefficients written by the DLL's `gain == 1.0` early return.
    pub const BYPASS: Self = Self {
        c8: 0.0,
        c12: 0.0,
        c16: 1.0,
        c20: 0.0,
        c24: 0.0,
    };

    /// Coefficients written by the DLL's `gain == 0.0` early return.
    pub const SILENCE: Self = Self {
        c8: 0.0,
        c12: 0.0,
        c16: 0.0,
        c20: 0.0,
        c24: 0.0,
    };

    /// True when the five slots represent the DLL's bypass path.
    pub fn is_bypass(self) -> bool {
        self == Self::BYPASS
    }

    /// True when every slot is finite.
    pub fn is_finite(self) -> bool {
        self.c8.is_finite()
            && self.c12.is_finite()
            && self.c16.is_finite()
            && self.c20.is_finite()
            && self.c24.is_finite()
    }

    /// Convert the DLL slot layout into the shared `Biquad` representation.
    ///
    /// The conversion is algebraic, not a change of design: the `Biquad`
    /// fields are `f64` because the response plotting code works in `f64`.
    pub fn to_biquad(self) -> Biquad {
        Biquad {
            b0: self.c16 as f64,
            b1: self.c20 as f64,
            b2: self.c24 as f64,
            a1: -(self.c8 as f64),
            a2: -(self.c12 as f64),
        }
    }

    /// Magnitude response in dB at angular frequency `w`.
    pub fn magnitude_db(self, w: f64) -> f64 {
        self.to_biquad().magnitude_db(w)
    }
}

/// Direct-form II state used by `0x1801469E0`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ApoState {
    /// `[B+0x1c]`, `w[n-1]`.
    pub w1: f32,
    /// `[B+0x20]`, `w[n-2]`.
    pub w2: f32,
}

impl ApoState {
    /// Clear both delay slots, as the DLL's reset path does.
    pub fn reset(&mut self) {
        self.w1 = 0.0;
        self.w2 = 0.0;
    }
}

/// Process one sample with the recovered direct-form II kernel.
///
/// The operation order matches `0x1801469E0`: `w` is formed from the input
/// and the two delayed states, then `y` from the three numerator terms.
#[inline]
pub fn process_sample(coeff: ApoCoefficients, state: &mut ApoState, x: f32) -> f32 {
    let w = x + coeff.c8 * state.w1 + coeff.c12 * state.w2;
    let y = coeff.c16 * w + coeff.c20 * state.w1 + coeff.c24 * state.w2;
    state.w2 = state.w1;
    state.w1 = w;
    y
}

fn validate(sample_rate: u32, frequency: f32, ratio: f32) -> Result<(), String> {
    if sample_rate == 0 {
        return Err("sample rate must be positive".into());
    }
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err("band frequency is not usable".into());
    }
    if frequency >= sample_rate as f32 / 2.0 {
        return Err(format!(
            "sample rate {sample_rate} Hz cannot activate {frequency} Hz band (Nyquist)"
        ));
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err("band width ratio is not usable".into());
    }
    Ok(())
}

fn linear_from_db(db: f32, divisor: f32) -> Result<f32, String> {
    if !db.is_finite() {
        return Err("band gain is not finite".into());
    }
    let linear = 10f32.powf(db / divisor);
    if !linear.is_finite() || linear <= 0.0 {
        return Err("band gain is outside the finite filter range".into());
    }
    Ok(linear)
}

/// Peaking section, port of `0x180087120`.
///
/// `gain` is the raw linear value stored in `[B+0x28]`.
pub fn peak_from_gain(
    gain: f32,
    sample_rate: u32,
    frequency: f32,
    ratio: f32,
) -> Result<ApoCoefficients, String> {
    validate(sample_rate, frequency, ratio)?;
    if !gain.is_finite() {
        return Err("band gain is not finite".into());
    }
    if gain == 0.0 {
        return Ok(ApoCoefficients::SILENCE);
    }
    if gain < 0.0 {
        return Err("band gain must not be negative".into());
    }
    if gain == 1.0 {
        return Ok(ApoCoefficients::BYPASS);
    }

    let a = gain.sqrt();
    let w = frequency / sample_rate as f32;
    let w0 = w * TAU_F32;
    let k = (w0 / ratio * 0.5).tan() / a;
    let t = (k - 1.0) / (k + 1.0);
    let c = w0.cos();
    let half_low = 0.5 * (1.0 - a);
    let half_high = 0.5 * (1.0 + a);
    let edge = (1.0 - t) * c;
    let coeff = ApoCoefficients {
        c8: edge,
        c12: t,
        c16: half_high - half_low * t,
        c20: -edge,
        c24: half_low - half_high * t,
    };
    if !coeff.is_finite() {
        return Err("band coefficients are not finite".into());
    }
    Ok(coeff)
}

/// Low shelf section, port of `0x180087970`.
///
/// `gain` is the raw linear value stored in `[B+0x28]`. The DLL constructs
/// shelf bands with `[B+0x40] == 1.0`, so the usual call uses `ratio = 1.0`.
pub fn low_shelf_from_gain(
    gain: f32,
    sample_rate: u32,
    frequency: f32,
    ratio: f32,
) -> Result<ApoCoefficients, String> {
    validate(sample_rate, frequency, ratio)?;
    if !gain.is_finite() {
        return Err("band gain is not finite".into());
    }
    if gain == 0.0 {
        return Ok(ApoCoefficients::SILENCE);
    }
    if gain < 0.0 {
        return Err("band gain must not be negative".into());
    }
    if gain == 1.0 {
        return Ok(ApoCoefficients::BYPASS);
    }

    let a = gain.sqrt();
    let w = frequency / sample_rate as f32;
    let w0 = w * TAU_F32;
    // 0x180087A15..0x180087A32: (w0/r)*2/3 + w0.
    let mut warped = w0 / ratio;
    warped *= TWO;
    warped /= THREE;
    warped += w0;
    let s = warped.sin();
    let c = warped.cos();

    // 0x180087A4B..0x180087A89: 1.4 - 0.4*(1/r - 0.707).
    let mut denominator = 1.0 / ratio;
    denominator -= SHELF_MID;
    denominator *= LOW_SHELF_WIDTH;
    let denominator = SHELF_BASE - denominator;

    // 0x180087A8D..0x180087A9C: sqrt((gain+1)/den - gain + 2A - 1).
    let mut root = (gain + 1.0) / denominator;
    root -= gain;
    root += a * TWO;
    root -= 1.0;
    if !(root.is_finite() && root >= 0.0) {
        return Err("low shelf produced a non-finite design term".into());
    }
    let sroot = root.sqrt();

    let a_minus_1 = a - 1.0;
    let a_plus_1 = a + 1.0;
    let scaled = sroot * s;
    let d = a_minus_1 * c + a_plus_1 + scaled;
    if !d.is_finite() || d == 0.0 {
        return Err("low shelf produced a degenerate denominator".into());
    }
    let coeff = ApoCoefficients {
        c8: 2.0 * (a_plus_1 * c + a_minus_1) / d,
        c12: -((a_minus_1 * c + a_plus_1) - scaled) / d,
        c16: a * (scaled + a_plus_1 - a_minus_1 * c) / d,
        c20: 2.0 * a * (a_minus_1 - a_plus_1 * c) / d,
        c24: a * (a_plus_1 - a_minus_1 * c - scaled) / d,
    };
    if !coeff.is_finite() {
        return Err("low shelf coefficients are not finite".into());
    }
    Ok(coeff)
}

/// High shelf section, port of `0x1800887E0`.
///
/// `gain` is the raw linear value stored in `[B+0x28]`. The DLL constructs
/// shelf bands with `[B+0x40] == 1.0`, so the usual call uses `ratio = 1.0`.
pub fn high_shelf_from_gain(
    gain: f32,
    sample_rate: u32,
    frequency: f32,
    ratio: f32,
) -> Result<ApoCoefficients, String> {
    validate(sample_rate, frequency, ratio)?;
    if !gain.is_finite() {
        return Err("band gain is not finite".into());
    }
    if gain == 0.0 {
        return Ok(ApoCoefficients::SILENCE);
    }
    if gain < 0.0 {
        return Err("band gain must not be negative".into());
    }
    if gain == 1.0 {
        return Ok(ApoCoefficients::BYPASS);
    }

    let a = gain.sqrt();
    let w = frequency / sample_rate as f32;
    let w0 = w * TAU_F32;
    // 0x18008887B..0x18008889F: w0 - 0.45*(w0/r).
    let warped = w0 - (w0 / ratio) * HIGH_SHELF_WIDTH;
    let s = warped.sin();
    let c = warped.cos();

    // 0x1800888B7..0x1800888E6: 1.4 - 2*(1/r - 0.707).
    let mut denominator = 1.0 / ratio;
    denominator -= SHELF_MID;
    denominator *= TWO;
    let denominator = SHELF_BASE - denominator;

    // 0x1800888EA..0x180088905: sqrt((gain+1)/den - gain + 2A - 1).
    let mut root = (gain + 1.0) / denominator;
    root -= gain;
    root += a * TWO;
    root -= 1.0;
    if !(root.is_finite() && root >= 0.0) {
        return Err("high shelf produced a non-finite design term".into());
    }
    let sroot = root.sqrt();

    let a_minus_1 = a - 1.0;
    let a_plus_1 = a + 1.0;
    let scaled = sroot * s;
    let d = a_plus_1 - a_minus_1 * c + scaled;
    if !d.is_finite() || d == 0.0 {
        return Err("high shelf produced a degenerate denominator".into());
    }
    let coeff = ApoCoefficients {
        c8: -2.0 * (a_minus_1 - a_plus_1 * c) / d,
        c12: -((a_plus_1 - a_minus_1 * c) - scaled) / d,
        c16: a * (scaled + a_plus_1 + a_minus_1 * c) / d,
        c20: -2.0 * a * (a_plus_1 * c + a_minus_1) / d,
        c24: a * (a_plus_1 + a_minus_1 * c - scaled) / d,
    };
    if !coeff.is_finite() {
        return Err("high shelf coefficients are not finite".into());
    }
    Ok(coeff)
}

/// Peaking section from the UI-visible dB value (power convention).
pub fn peak_from_db(
    gain_db: f32,
    sample_rate: u32,
    frequency: f32,
    ratio: f32,
) -> Result<ApoCoefficients, String> {
    let gain = linear_from_db(gain_db, 10.0)?;
    peak_from_gain(gain, sample_rate, frequency, ratio)
}

/// Low shelf from the UI-visible dB value (amplitude convention).
pub fn low_shelf_from_db(
    gain_db: f32,
    sample_rate: u32,
    frequency: f32,
) -> Result<ApoCoefficients, String> {
    let gain = linear_from_db(gain_db, 20.0)?;
    low_shelf_from_gain(gain, sample_rate, frequency, 1.0)
}

/// High shelf from the UI-visible dB value (amplitude convention).
pub fn high_shelf_from_db(
    gain_db: f32,
    sample_rate: u32,
    frequency: f32,
) -> Result<ApoCoefficients, String> {
    let gain = linear_from_db(gain_db, 20.0)?;
    high_shelf_from_gain(gain, sample_rate, frequency, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: u32 = 48_000;
    const BANDS_HZ: [f32; 10] = [
        31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];

    fn w_of(f: f32) -> f64 {
        std::f64::consts::TAU * f as f64 / FS as f64
    }

    #[test]
    fn flat_returns_bypass() {
        for db in [0.0f32] {
            assert_eq!(peak_from_db(db, FS, 1000.0, 2.0).unwrap(), ApoCoefficients::BYPASS);
            assert_eq!(low_shelf_from_db(db, FS, 31.0).unwrap(), ApoCoefficients::BYPASS);
            assert_eq!(high_shelf_from_db(db, FS, 16000.0).unwrap(), ApoCoefficients::BYPASS);
        }
    }

    #[test]
    fn peak_hits_labeled_db_at_centre() {
        for db in [-12.0f32, -6.0, -3.0, 3.0, 6.0, 12.0] {
            let c = peak_from_db(db, FS, 1000.0, 2.0).unwrap();
            let measured = c.magnitude_db(w_of(1000.0));
            assert!(
                (measured - db as f64).abs() < 0.02,
                "peak {db} dB measured {measured} dB"
            );
        }
    }

    #[test]
    fn peak_stays_unity_at_band_edges() {
        for db in [-12.0f32, -6.0, 6.0, 12.0] {
            let c = peak_from_db(db, FS, 1000.0, 2.0).unwrap();
            for probe in [20.0f64, 20_000.0] {
                let measured = c.magnitude_db(w_of(probe as f32));
                assert!(
                    measured.abs() < 0.3,
                    "edge {probe} Hz: {measured} dB at {db} dB"
                );
            }
        }
    }

    #[test]
    fn low_shelf_plateau_matches_labeled_db() {
        for db in [-12.0f32, -6.0, 6.0, 12.0] {
            let c = low_shelf_from_db(db, FS, 31.0).unwrap();
            let dc = c.magnitude_db(w_of(20.0));
            let ny = c.magnitude_db(w_of(20_000.0));
            assert!(
                (dc - db as f64).abs() < 0.3,
                "low shelf {db} dB: 20 Hz {dc} dB"
            );
            assert!(ny.abs() < 0.3, "low shelf {db} dB: 20 kHz {ny} dB");
        }
    }

    #[test]
    fn high_shelf_plateau_matches_labeled_db() {
        for db in [-12.0f32, -6.0, 6.0, 12.0] {
            let c = high_shelf_from_db(db, FS, 16000.0).unwrap();
            let dc = c.magnitude_db(w_of(20.0));
            let ny = c.magnitude_db(w_of(20_000.0));
            assert!(dc.abs() < 0.3, "high shelf {db} dB: 20 Hz {dc} dB");
            assert!(
                (ny - db as f64).abs() < 0.3,
                "high shelf {db} dB: 20 kHz {ny} dB"
            );
        }
    }

    #[test]
    fn factory_band_curves_are_finite() {
        for (index, f) in BANDS_HZ.iter().enumerate() {
            let db = if index % 2 == 0 { 3.0 } else { -2.0 };
            let c = if index == 0 {
                low_shelf_from_db(db, FS, *f).unwrap()
            } else if index == 9 {
                high_shelf_from_db(db, FS, *f).unwrap()
            } else {
                peak_from_db(db, FS, *f, 2.0).unwrap()
            };
            for probe in [1.0f64, 10.0, 100.0, 1000.0, 10_000.0, 20_000.0] {
                let w = std::f64::consts::TAU * probe / FS as f64;
                assert!(c.magnitude_db(w).is_finite());
            }
        }
    }

    #[test]
    fn df2_kernel_matches_biquad_df1() {
        let c = peak_from_db(6.0, FS, 1000.0, 2.0).unwrap();
        let biquad = c.to_biquad();
        let mut state = ApoState::default();
        let mut z = [0.0f64; 4];
        for n in 0..512 {
            let x = if n == 0 { 1.0f32 } else { 0.0 };
            let y_apo = process_sample(c, &mut state, x);
            let y_df1 = biquad.run(x as f64, &mut z);
            assert!(
                (y_apo as f64 - y_df1).abs() < 1e-6,
                "sample {n}: DF2 {y_apo} vs DF1 {y_df1}"
            );
        }
    }

    #[test]
    fn slots_match_independent_reference() {
        // Reference values produced by an independent float32 emulation of the
        // three disassembled instruction sequences (tools/apo-eq-coeff).
        let peak = peak_from_db(6.0, FS, 1000.0, 2.0).unwrap();
        assert!((peak.c8 - 1.9508814).abs() < 1e-5, "peak c8 {}", peak.c8);
        assert!((peak.c12 + 0.9677154).abs() < 1e-5, "peak c12 {}", peak.c12);
        assert!((peak.c16 - 1.0160658).abs() < 1e-5, "peak c16 {}", peak.c16);
        assert!((peak.c20 + 1.9508814).abs() < 1e-5, "peak c20 {}", peak.c20);
        assert!((peak.c24 - 0.9516495).abs() < 1e-5, "peak c24 {}", peak.c24);

        let low = low_shelf_from_db(6.0, FS, 31.0).unwrap();
        assert!((low.c8 - 1.9929479).abs() < 1e-4, "low c8 {}", low.c8);
        assert!((low.c12 + 0.9929802).abs() < 1e-4, "low c12 {}", low.c12);
        assert!((low.c16 - 1.0014560).abs() < 1e-4, "low c16 {}", low.c16);
        assert!((low.c20 + 1.9929317).abs() < 1e-4, "low c20 {}", low.c20);
        assert!((low.c24 - 0.9915404).abs() < 1e-4, "low c24 {}", low.c24);

        let high = high_shelf_from_db(6.0, FS, 16000.0).unwrap();
        assert!((high.c8 - 0.2875).abs() < 1e-4, "high c8 {}", high.c8);
        assert!((high.c12 + 0.1348).abs() < 1e-4, "high c12 {}", high.c12);
        assert!((high.c16 - 1.5324).abs() < 1e-4, "high c16 {}", high.c16);
        assert!((high.c20 + 0.9953).abs() < 1e-4, "high c20 {}", high.c20);
        assert!((high.c24 - 0.3102).abs() < 1e-4, "high c24 {}", high.c24);
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert!(peak_from_db(f32::NAN, FS, 1000.0, 2.0).is_err());
        assert!(peak_from_db(6.0, 0, 1000.0, 2.0).is_err());
        assert!(peak_from_db(6.0, FS, 24_000.0, 2.0).is_err());
        assert!(peak_from_db(6.0, FS, 1000.0, 0.0).is_err());
        assert!(low_shelf_from_db(6.0, FS, 0.0).is_err());
        assert!(high_shelf_from_db(f32::INFINITY, FS, 16000.0).is_err());
    }
}
