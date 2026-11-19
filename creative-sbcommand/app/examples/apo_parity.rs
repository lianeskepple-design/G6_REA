//! Prints APO-exact coefficient slots and impulse responses as hex float32.
//!
//! Run with `cargo run --example apo_parity` from `app/`, then compare the
//! output with `tools/apo-eq-coeff/apo_parity.c` (see `run_parity.ps1`).

use aurora_audio_command::apo_eq::{
    high_shelf_from_db, low_shelf_from_db, peak_from_db, process_sample, ApoCoefficients,
    ApoState,
};

const FS: u32 = 48_000;
const N: usize = 256;

fn bits(value: f32) -> String {
    format!("{:08x}", value.to_bits())
}

fn report(name: &str, coeff: ApoCoefficients) {
    println!(
        "case {} slots {} {} {} {} {}",
        name,
        bits(coeff.c8),
        bits(coeff.c12),
        bits(coeff.c16),
        bits(coeff.c20),
        bits(coeff.c24)
    );
    let mut state = ApoState::default();
    let mut line = format!("case {} ir", name);
    for n in 0..N {
        let x = if n == 0 { 1.0f32 } else { 0.0 };
        let y = process_sample(coeff, &mut state, x);
        line.push(' ');
        line.push_str(&bits(y));
    }
    println!("{line}");
}

fn main() {
    report("peak_p6", peak_from_db(6.0, FS, 1000.0, 2.0).unwrap());
    report("peak_m12", peak_from_db(-12.0, FS, 1000.0, 2.0).unwrap());
    report("low_p6", low_shelf_from_db(6.0, FS, 31.0).unwrap());
    report("low_m12", low_shelf_from_db(-12.0, FS, 31.0).unwrap());
    report("high_p6", high_shelf_from_db(6.0, FS, 16000.0).unwrap());
    report("high_m12", high_shelf_from_db(-12.0, FS, 16000.0).unwrap());

    // FPS headphone curve from g6-eq-presets-current.json.
    let freqs = [31.0f32, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
    let gains = [2.0f32, 0.0, -2.0, -2.0, 0.0, 2.0, 2.0, -5.0, 2.0, 1.0];
    let coeffs: Vec<ApoCoefficients> = (0..10)
        .map(|i| {
            if i == 0 {
                low_shelf_from_db(gains[i], FS, freqs[i]).unwrap()
            } else if i == 9 {
                high_shelf_from_db(gains[i], FS, freqs[i]).unwrap()
            } else {
                peak_from_db(gains[i], FS, freqs[i], 2.0).unwrap()
            }
        })
        .collect();
    let mut states = [ApoState::default(); 10];
    let mut line = String::from("case fps_chain ir");
    for n in 0..N {
        let mut x = if n == 0 { 1.0f32 } else { 0.0 };
        for band in 0..10 {
            x = process_sample(coeffs[band], &mut states[band], x);
        }
        line.push(' ');
        line.push_str(&bits(x));
    }
    println!("{line}");
}
