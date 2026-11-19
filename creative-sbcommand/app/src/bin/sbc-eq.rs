//! Offline WAV EQ CLI.
//!
//! The DSP kernel itself lives in `aurora_audio_command::eq_core`, shared with
//! the desktop equalizer page. Band topology and its limits are documented
//! there and in `reference/apo-effects-structure.md`.
use aurora_audio_command::eq_core::{self, OutputMode};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn usage() -> &'static str {
    "Usage:\n  sbc-eq --list-eq-presets\n  sbc-eq --process-wav input.wav output.wav --eq-preset NAME --output-mode headphones|speakers [--headroom-db N]\n\n--headroom-db N is signed extra gain in dB added to the preset preamp:\nnegative attenuates (e.g. -6), positive boosts. It is NOT automatic\nanti-clipping and enables no limiter; positive values can leave float32\nsamples above 1.0. Non-finite values, or gains whose float32 conversion\noverflows, exit non-zero before any output file is created."
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--list-eq-presets" {
        for preset in eq_core::presets()? {
            println!("{}", preset.name);
        }
        return Ok(());
    }
    if args.len() < 6 || args[0] != "--process-wav" {
        return Err(usage().into());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let (mut preset, mut mode, mut headroom) = (None, None, 0.0);
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--eq-preset" if i + 1 < args.len() => {
                preset = Some(args[i + 1].as_str());
                i += 2
            }
            "--output-mode" if i + 1 < args.len() => {
                mode = Some(args[i + 1].as_str());
                i += 2
            }
            "--headroom-db" if i + 1 < args.len() => {
                headroom = args[i + 1].parse()?;
                i += 2
            }
            _ => return Err(usage().into()),
        }
    }
    let mode_text = mode.ok_or("missing --output-mode")?;
    let mode = OutputMode::parse(mode_text)
        .ok_or("output mode must be headphones or speakers")?;
    let curve = eq_core::preset_curve(preset.ok_or("missing --eq-preset")?, mode)?;
    eq_core::render_wav(&input, &output, &curve, headroom)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("sbc-eq: {e}");
        std::process::exit(1)
    }
}
