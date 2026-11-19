#!/usr/bin/env python3
"""Analyze Scout on/off sweep recordings and recover the ten band gains.

The deconvolution uses FFT division with regularization; no audio device is
touched. `--self-test` synthesizes known band gains and verifies recovery.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.fft import next_fast_len
from scipy.signal import fftconvolve, lfilter

BANDS_HZ = [31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0]


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


def read_mono(path: Path) -> tuple[np.ndarray, int]:
    data, rate = sf.read(str(path), always_2d=True)
    return data.mean(axis=1).astype(np.float64), int(rate)


def align(recording: np.ndarray, length: int) -> np.ndarray:
    if recording.size == 0:
        raise ValueError("empty recording")
    threshold = 0.02 * float(np.max(np.abs(recording)))
    starts = np.flatnonzero(np.abs(recording) > threshold)
    start = int(starts[0]) if starts.size else 0
    return recording[start:]


def transfer(recording: np.ndarray, sweep: np.ndarray, sample_rate: int,
             nfft: int) -> tuple[np.ndarray, np.ndarray]:
    length = max(recording.size, sweep.size)
    rec = np.pad(recording, (0, length - recording.size))
    src = np.pad(sweep, (0, length - sweep.size))
    spectrum = np.fft.rfft(src, nfft)
    reg = 1e-6 * float(np.max(np.abs(spectrum)) or 1.0)
    response = np.fft.rfft(rec, nfft) / (spectrum + reg)
    freqs = np.fft.rfftfreq(nfft, 1.0 / sample_rate)
    return response, freqs


def band_delta(freqs: np.ndarray, delta_db: np.ndarray, band: float,
               width_octaves: float = 1.0 / 12.0) -> float:
    lo = band * 2.0 ** (-width_octaves)
    hi = band * 2.0 ** width_octaves
    mask = (freqs >= lo) & (freqs <= hi)
    if not np.any(mask):
        return float("nan")
    return float(np.median(delta_db[mask]))


def preamp_estimate(freqs: np.ndarray, delta_db: np.ndarray) -> float:
    values = []
    for left, right in zip(BANDS_HZ, BANDS_HZ[1:]):
        middle = math.sqrt(left * right)
        value = band_delta(freqs, delta_db, middle, 1.0 / 12.0)
        if np.isfinite(value):
            values.append(value)
    return float(np.median(values)) if values else 0.0


def analyze(off: np.ndarray, on: np.ndarray, sweep: np.ndarray, sample_rate: int,
            subtract_preamp: bool = False) -> dict:
    nfft = next_fast_len(max(off.size, on.size, sweep.size))
    off_ir, freqs = transfer(align(off, sweep.size), sweep, sample_rate, nfft)
    on_ir, _ = transfer(align(on, sweep.size), sweep, sample_rate, nfft)
    floor = 1e-12
    ratio = np.maximum(np.abs(on_ir), floor) / np.maximum(np.abs(off_ir), floor)
    delta = 20.0 * np.log10(ratio)
    preamp = preamp_estimate(freqs, delta)
    bands = []
    for band in BANDS_HZ:
        raw = band_delta(freqs, delta, band)
        bands.append({
            "hz": band,
            "delta_db": raw,
            "net_db": raw - preamp if subtract_preamp else raw,
        })
    return {"preamp_db": preamp, "bands": bands, "sample_rate": sample_rate}


def synth_ir(sample_rate: int, gains: list[float], preamp_db: float,
             nfft: int = 16384, delay: int = 256) -> np.ndarray:
    freqs = np.fft.rfftfreq(nfft, 1.0 / sample_rate)
    log_bands = np.log2(np.array(BANDS_HZ))
    log_freqs = np.log2(np.clip(freqs, BANDS_HZ[0], BANDS_HZ[-1]))
    gain_db = np.interp(log_freqs, log_bands, np.array(gains), left=gains[0], right=gains[-1])
    magnitude = 10.0 ** ((gain_db + preamp_db) / 20.0)
    phase = np.exp(-2j * math.pi * freqs * delay / sample_rate)
    ir = np.fft.irfft(magnitude * phase, nfft)
    return ir


def _apo_peak(gain_db: float, fs: int, f: float, ratio: float = 2.0):
    a = math.sqrt(10.0 ** (gain_db / 10.0))
    w0 = 2.0 * math.pi * f / fs
    k = math.tan(w0 / (2.0 * ratio)) / a
    t = (k - 1.0) / (k + 1.0)
    c = math.cos(w0)
    b = [0.5 * (1.0 + a) - 0.5 * (1.0 - a) * t,
         -c * (1.0 - t),
         0.5 * (1.0 - a) - 0.5 * (1.0 + a) * t]
    return b, [1.0, b[1], -t]


def _apo_low(gain_db: float, fs: int, f: float, ratio: float = 1.0):
    gain = 10.0 ** (gain_db / 20.0)
    a = math.sqrt(gain)
    w0 = 2.0 * math.pi * f / fs
    x = w0 + (w0 / ratio) * 2.0 / 3.0
    s = math.sin(x)
    c = math.cos(x)
    den = 1.4 - 0.4 * (1.0 / ratio - 0.707)
    root = (gain + 1.0) / den - gain + 2.0 * a - 1.0
    sroot = math.sqrt(max(root, 0.0))
    am = a - 1.0
    ap = a + 1.0
    sc = sroot * s
    d = am * c + ap + sc
    c8 = 2.0 * (ap * c + am) / d
    c12 = -((am * c + ap) - sc) / d
    c16 = a * (sc + ap - am * c) / d
    c20 = 2.0 * a * (am - ap * c) / d
    c24 = a * (ap - am * c - sc) / d
    return [c16, c20, c24], [1.0, -c8, -c12]


def _apo_high(gain_db: float, fs: int, f: float, ratio: float = 1.0):
    gain = 10.0 ** (gain_db / 20.0)
    a = math.sqrt(gain)
    w0 = 2.0 * math.pi * f / fs
    x = w0 - (w0 / ratio) * 0.45
    s = math.sin(x)
    c = math.cos(x)
    den = 1.4 - 2.0 * (1.0 / ratio - 0.707)
    root = (gain + 1.0) / den - gain + 2.0 * a - 1.0
    sroot = math.sqrt(max(root, 0.0))
    am = a - 1.0
    ap = a + 1.0
    sc = sroot * s
    d = ap - am * c + sc
    c8 = -2.0 * (am - ap * c) / d
    c12 = -((ap - am * c) - sc) / d
    c16 = a * (sc + ap + am * c) / d
    c20 = -2.0 * a * (ap * c + am) / d
    c24 = a * (ap + am * c - sc) / d
    return [c16, c20, c24], [1.0, -c8, -c12]


def synth_eq(sweep: np.ndarray, gains: list[float], preamp_db: float,
             sample_rate: int) -> np.ndarray:
    bands = [31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0]
    y = sweep.astype(np.float64) * 10.0 ** (preamp_db / 20.0)
    for index, gain in enumerate(gains):
        if gain == 0.0:
            continue
        if index == 0:
            b, a = _apo_low(gain, sample_rate, bands[index])
        elif index == len(gains) - 1:
            b, a = _apo_high(gain, sample_rate, bands[index])
        else:
            b, a = _apo_peak(gain, sample_rate, bands[index])
        y = lfilter(b, a, y)
    return y


def self_test() -> int:
    sample_rate = 48000
    sweep = farina(sample_rate, 3.0, 20.0, 20000.0, 0.25)
    gains = [2.0, -1.0, 0.0, 3.0, -2.0, 1.0, 0.0, -4.0, 2.0, 1.0]
    preamp = -1.0
    ir_on = synth_ir(sample_rate, gains, preamp)
    on = synth_eq(sweep, gains, preamp, sample_rate)
    off = sweep.astype(np.float64).copy()
    rng = np.random.default_rng(1234)
    on += rng.normal(0.0, 1e-6, on.size)
    off += rng.normal(0.0, 1e-6, off.size)

    result = analyze(off, on, sweep, sample_rate, subtract_preamp=False)
    failures = 0
    if abs(result["preamp_db"] - preamp) > 0.5:
        print(f"preamp estimate {result['preamp_db']:.3f} vs {preamp}")
        failures += 1
    for band, expected in zip(result["bands"], gains):
        measured = band["delta_db"]
        if not np.isfinite(measured) or abs(measured - (expected + preamp)) > 0.6:
            print(f"{band['hz']} Hz: {measured} vs {expected + preamp}")
            failures += 1
    if failures:
        print(f"FAIL ({failures} bands)")
        return 1
    print("PASS: synthetic 10-band response recovered within 0.6 dB")
    return 0


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--sweep", type=Path)
    parser.add_argument("--off", type=Path)
    parser.add_argument("--on", type=Path)
    parser.add_argument("--output", type=Path, default=Path("scout-response.json"))
    parser.add_argument("--subtract-preamp", action="store_true")
    parser.add_argument("--make-test-wavs", type=Path,
                        help="write synthetic sweep/off/on WAVs for an end-to-end CLI test")
    args = parser.parse_args()

    if args.make_test_wavs:
        out = args.make_test_wavs
        out.mkdir(parents=True, exist_ok=True)
        sweep = farina(48000, 3.0, 20.0, 20000.0, 0.25)
        gains = [2.0, -1.0, 0.0, 3.0, -2.0, 1.0, 0.0, -4.0, 2.0, 1.0]
        on = synth_eq(sweep, gains, -1.0, 48000)
        sf.write(str(out / "test-sweep.wav"), sweep, 48000, subtype="FLOAT")
        sf.write(str(out / "test-off.wav"), sweep, 48000, subtype="FLOAT")
        sf.write(str(out / "test-on.wav"), on.astype(np.float32), 48000, subtype="FLOAT")
        print(out)
        return 0
    if args.self_test:
        return self_test()
    if not (args.sweep and args.off and args.on):
        parser.error("provide --sweep, --off and --on, or use --self-test")

    sweep, sweep_rate = read_mono(args.sweep)
    off, off_rate = read_mono(args.off)
    on, on_rate = read_mono(args.on)
    if not (sweep_rate == off_rate == on_rate):
        parser.error("sweep and recordings must share a sample rate")
    result = analyze(off, on, sweep, sweep_rate, args.subtract_preamp)
    result["files"] = {
        "sweep": str(args.sweep),
        "off": str(args.off),
        "on": str(args.on),
        "sha256": {
            "sweep": digest(args.sweep),
            "off": digest(args.off),
            "on": digest(args.on),
        },
    }
    args.output.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"{'Hz':>8} {'delta dB':>10} {'net dB':>10}")
    for band in result["bands"]:
        print(f"{band['hz']:8.2f} {band['delta_db']:10.3f} {band['net_db']:10.3f}")
    print(f"preamp estimate: {result['preamp_db']:.3f} dB")
    print(f"written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
