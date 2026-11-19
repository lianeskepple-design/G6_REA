#!/usr/bin/env python3
"""Generate a Farina logarithmic sine sweep for Scout frequency-response tests.

No audio device is touched. Produces 48 kHz float32 and 24-bit PCM WAV files
plus a time-reversed inverse reference. The analyzer uses FFT division, so the
inverse file is only a convenience for external tools.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import soundfile as sf


def farina(fs: int, seconds: float, f1: float, f2: float, amplitude: float) -> np.ndarray:
    count = int(round(fs * seconds))
    t = np.arange(count, dtype=np.float64) / fs
    ratio = math.log(f2 / f1)
    phase = 2.0 * math.pi * f1 * seconds / ratio * (np.exp(t / seconds * ratio) - 1.0)
    sweep = amplitude * np.sin(phase)
    fade = int(0.01 * fs)
    if fade > 1:
        ramp = np.linspace(0.0, 1.0, fade)
        sweep[:fade] *= ramp
        sweep[-fade:] *= ramp[::-1]
    return sweep.astype(np.float32)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", default="scout-measure-out")
    parser.add_argument("--sample-rate", type=int, default=48000)
    parser.add_argument("--seconds", type=float, default=8.0)
    parser.add_argument("--f-start", type=float, default=20.0)
    parser.add_argument("--f-end", type=float, default=20000.0)
    parser.add_argument("--amplitude", type=float, default=0.25)
    parser.add_argument("--tail-seconds", type=float, default=1.0)
    args = parser.parse_args()

    if args.sample_rate < 8000:
        parser.error("sample rate must be at least 8000")
    if not (0 < args.f_start < args.f_end < args.sample_rate / 2):
        parser.error("require 0 < f_start < f_end < Nyquist")
    if not (0 < args.amplitude <= 1):
        parser.error("amplitude must be in (0, 1]")

    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    sweep = farina(args.sample_rate, args.seconds, args.f_start, args.f_end, args.amplitude)
    tail = np.zeros(int(round(args.sample_rate * args.tail_seconds)), dtype=np.float32)
    playback = np.concatenate([sweep, tail])

    float_path = out / "scout-sweep-float32.wav"
    pcm_path = out / "scout-sweep-pcm24.wav"
    inverse_path = out / "scout-sweep-inverse.wav"
    sf.write(float_path, playback, args.sample_rate, subtype="FLOAT")
    sf.write(pcm_path, playback, args.sample_rate, subtype="PCM_24")

    # Time-reversed reference; normalized to the same peak for external tools.
    inverse = sweep[::-1].copy()
    peak = float(np.max(np.abs(inverse))) or 1.0
    inverse /= peak
    sf.write(inverse_path, inverse, args.sample_rate, subtype="FLOAT")

    report = {
        "sample_rate": args.sample_rate,
        "sweep_seconds": args.seconds,
        "tail_seconds": args.tail_seconds,
        "f_start": args.f_start,
        "f_end": args.f_end,
        "amplitude": args.amplitude,
        "samples": int(playback.size),
        "peak": float(np.max(np.abs(playback))),
        "sha256": {
            float_path.name: digest(float_path),
            pcm_path.name: digest(pcm_path),
            inverse_path.name: digest(inverse_path),
        },
    }
    (out / "sweep-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
