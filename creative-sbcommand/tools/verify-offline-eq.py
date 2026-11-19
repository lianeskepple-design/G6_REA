#!/usr/bin/env python3
"""Reproducible offline E2E checks for the sbc-eq binary; needs numpy/scipy/soundfile.

Each run uses a unique directory under the project-local tools/.offline-eq-test
root and removes it on exit, so the script is safe to run repeatedly.
"""
import argparse
import hashlib
import os
import pathlib
import subprocess
import tempfile
import numpy as np
import soundfile as sf

p = argparse.ArgumentParser()
p.add_argument("--exe", required=True, type=pathlib.Path)
p.add_argument("--presets", required=True, type=pathlib.Path)
a = p.parse_args()
exe = a.exe.resolve()
assert a.presets.resolve().is_file(), f"preset database not found: {a.presets}"


def run(*args):
    return subprocess.run([str(exe), *map(str, args)], capture_output=True,
                          text=True, encoding="utf-8", errors="replace", check=False)


def process(src, dst, preset, mode, headroom=None):
    args = ["--process-wav", src, dst, "--eq-preset", preset, "--output-mode", mode]
    if headroom is not None:
        args += ["--headroom-db", headroom]
    return run(*args)


def must_fail(proc, out, label, needle=None, output_must_not_exist=True):
    assert proc.returncode != 0, f"{label}: expected non-zero exit, got 0"
    if output_must_not_exist:
        assert not pathlib.Path(out).exists(), f"{label}: created output despite failure"
    if needle:
        assert needle in proc.stderr, f"{label}: stderr {proc.stderr!r} lacks {needle!r}"


test_root = pathlib.Path(__file__).resolve().parent / ".offline-eq-test"
test_root.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="run-", dir=test_root) as td_name:
    td = pathlib.Path(td_name)
    sr, frames = 48000, 48000
    t = np.arange(frames) / sr
    # Excite all bands with separated coherent tones. The right channel is
    # exactly zero, so any non-zero right output proves channel-state leakage.
    freqs = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 12000]
    x = sum(np.sin(2 * np.pi * f * t) for f in freqs) / len(freqs) * 0.2
    stereo = np.column_stack((x, np.zeros_like(x)))
    src, out_s, out_h = td / "input.wav", td / "speaker.wav", td / "headphone.wav"
    sf.write(src, stereo, sr, subtype="PCM_24")
    for output, mode in ((out_s, "speakers"), (out_h, "headphones")):
        proc = process(src, output, "AdventureAndAction", mode)
        assert proc.returncode == 0, proc.stderr
    y_s, sr_s = sf.read(out_s, always_2d=True)
    y_h, sr_h = sf.read(out_h, always_2d=True)
    assert (sr_s, y_s.shape) == (sr, stereo.shape)
    assert (sr_h, y_h.shape) == (sr, stereo.shape)
    # Real channel isolation: left-only excitation must leave right channel all zero.
    assert np.max(np.abs(y_s[:, 1])) == 0.0, "right channel leaked (speakers)"
    assert np.max(np.abs(y_h[:, 1])) == 0.0, "right channel leaked (headphones)"
    assert not np.allclose(y_s, y_h), "Speaker and Headphone modes unexpectedly identical"
    # A completely flat preset with zero preamp must be identity to float tolerance.
    flat = td / "flat.wav"
    proc = process(src, flat, "Flat", "speakers")
    assert proc.returncode == 0, proc.stderr
    x_dec, _ = sf.read(src, always_2d=True)
    assert np.max(np.abs(sf.read(flat, always_2d=True)[0] - x_dec)) < 2e-7
    # Signed headroom: -6 dB is attenuation, applied on top of the preset preamp.
    att = td / "attenuated.wav"
    proc = process(src, att, "Flat", "speakers", "-6")
    assert proc.returncode == 0, proc.stderr
    y_att = sf.read(att, always_2d=True)[0]
    assert np.max(np.abs(y_att - x_dec * 10 ** (-6 / 20))) < 1e-7, "-6 dB is not a 0.501x gain"
    # Positive headroom is gain, not an automatic anti-clipping stage: a 0.8
    # sample boosted by +6 dB must remain (finite) above 1.0 in float32.
    loud_src, loud_out = td / "loud.wav", td / "loud-out.wav"
    loud = np.column_stack((np.full(4096, 0.8, np.float32), np.zeros(4096, np.float32)))
    sf.write(loud_src, loud, sr, subtype="FLOAT")
    proc = process(loud_src, loud_out, "Flat", "speakers", "6")
    assert proc.returncode == 0, proc.stderr
    y_boost = sf.read(loud_out, always_2d=True, dtype="float32")[0]
    assert np.all(np.isfinite(y_boost)) and np.max(y_boost) > 1.0, "positive headroom was clamped/limited"
    assert abs(float(np.max(y_boost)) - 0.8 * 10 ** (6 / 20)) < 1e-6
    # Verify a Headphone boost around 31 Hz from its +4 dB low band.
    # Estimate tone amplitude using a narrow FFT bin window on a dedicated tone.
    tone_src, tone_out = td / "tone.wav", td / "tone-out.wav"
    tone = (0.1 * np.sin(2 * np.pi * 31 * t)).astype(np.float32)
    sf.write(tone_src, tone, sr, subtype="FLOAT")
    proc = process(tone_src, tone_out, "AdventureAndAction", "headphones")
    assert proc.returncode == 0, proc.stderr
    z, _ = sf.read(tone_out)
    amp_in = np.sqrt(2) * np.sqrt(np.mean(tone[int(sr * .2):] ** 2))
    amp_out = np.sqrt(2) * np.sqrt(np.mean(z[int(sr * .2):] ** 2))
    measured = 20 * np.log10(amp_out / amp_in)
    # Compute full cascade response, including overlap from adjacent bands.
    # Documented topology (reference/apo-effects-structure.md): band 1 low shelf,
    # bands 2..9 peaking with alpha = tan(w0/(2*ratio)), band 10 high shelf.
    gains = [4, 4, -1, -2, 0, 0, 3, 0, 0, 0]
    centers = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
    w = 2 * np.pi * 31 / sr
    expected = 0.0
    last = len(centers) - 1
    for i, (fc, db) in enumerate(zip(centers, gains)):
        if db == 0:
            continue  # zero-gain bands are bypassed by the CLI
        A = 10 ** (db / 40)
        omega = 2 * np.pi * fc / sr
        c = np.cos(omega)
        if i in (0, last):
            alpha = np.sin(omega) / 2 * np.sqrt(2)   # RBJ shelf, slope S=1
            t = 2 * np.sqrt(A) * alpha
            p1, m1 = A - 1.0, A + 1.0
            if i == 0:
                num = [A * (m1 - p1 * c + t), 2 * A * (p1 - m1 * c), A * (m1 - p1 * c - t)]
                den = [m1 + p1 * c + t, -2 * (p1 + m1 * c), m1 + p1 * c - t]
            else:
                num = [A * (m1 + p1 * c + t), -2 * A * (p1 + m1 * c), A * (m1 + p1 * c - t)]
                den = [m1 - p1 * c + t, 2 * (p1 - m1 * c), m1 - p1 * c - t]
            a0 = den[0]
            b0, b1, b2 = num[0] / a0, num[1] / a0, num[2] / a0
            a1, a2 = den[1] / a0, den[2] / a0
        else:
            alpha = np.tan(omega / (2 * (centers[i + 1] / fc)))
            a0 = 1 + alpha / A
            b0 = (1 + alpha * A) / a0
            b1 = -2 * c / a0
            b2 = (1 - alpha * A) / a0
            a1 = -2 * c / a0
            a2 = (1 - alpha / A) / a0
        zz = np.exp(-1j * w)
        h = (b0 + b1 * zz + b2 * zz * zz) / (1 + a1 * zz + a2 * zz * zz)
        expected += 20 * np.log10(abs(h))
    assert abs(measured - expected) < 0.15, f"31 Hz cascade expected {expected:.3f} dB, got {measured:.3f} dB"
    # All requested input sample encodings decode and produce float32 output.
    for subtype in ("PCM_16", "PCM_32", "FLOAT"):
        enc_in, enc_out = td / f"{subtype}.wav", td / f"{subtype}-out.wav"
        sf.write(enc_in, stereo, sr, subtype=subtype)
        proc = process(enc_in, enc_out, "AdventureAndAction", "speakers")
        assert proc.returncode == 0, proc.stderr
        enc, enc_sr = sf.read(enc_out, always_2d=True, dtype="float32")
        ref, _ = sf.read(enc_in, always_2d=True, dtype="float32")
        assert enc_sr == sr and enc.shape == stereo.shape
        assert sf.info(enc_out).subtype == "FLOAT"
        # Flat speakers is identity: this pins the integer decode scaling too.
        assert np.max(np.abs(enc - ref)) < 1e-6, f"{subtype} decode scaling changed"
    # Output protection and invalid preset: no output should be created.
    refused = td / "refused.wav"
    must_fail(process(src, refused, "does-not-exist", "speakers"), refused, "invalid preset")
    existing = td / "already.wav"
    existing.write_bytes(b"keep")
    must_fail(process(src, existing, "AdventureAndAction", "speakers"), existing, "existing output", output_must_not_exist=False)
    assert existing.read_bytes() == b"keep"
    # Hardlink to the input must not be overwritten; source bytes stay intact.
    src_hash = hashlib.sha256(src.read_bytes()).hexdigest()
    hardlink = td / "hardlink.wav"
    os.link(src, hardlink)
    must_fail(process(src, hardlink, "AdventureAndAction", "speakers"), hardlink, "hardlink output", output_must_not_exist=False)
    assert hashlib.sha256(src.read_bytes()).hexdigest() == src_hash
    assert hashlib.sha256(hardlink.read_bytes()).hexdigest() == src_hash
    must_fail(process(src, src, "AdventureAndAction", "speakers"), src, "same input/output", output_must_not_exist=False)
    assert hashlib.sha256(src.read_bytes()).hexdigest() == src_hash
    # Gain validation: +1000 dB is finite in f64 but overflows float32; larger
    # dB overflows the f64 power itself. -inf/NaN/+inf are rejected up front.
    for label, db, needle in (
        ("+1000 dB float32 overflow", "1000", "float32"),
        ("+1e10 dB pow overflow", "1e10", "finite"),
        ("-inf headroom", "-inf", "finite"),
        ("NaN headroom", "nan", "finite"),
        ("+inf headroom", "inf", "finite"),
    ):
        out = td / f"invalid-{label.split()[0]}.wav"
        must_fail(process(src, out, "Flat", "speakers", db), out, label, needle)
    # The gain itself must be float32-representable, so +1000 dB is rejected even
    # for silence (no per-sample overflow can occur there).
    silent_src, silent_out = td / "silent.wav", td / "silent-out.wav"
    sf.write(silent_src, np.zeros((4096, 2), np.float32), sr, subtype="FLOAT")
    must_fail(process(silent_src, silent_out, "Flat", "speakers", "1000"), silent_out,
              "silent +1000 dB", "float32")
    # Active 16 kHz band cannot be represented below its Nyquist limit at 8 kHz.
    low_src, low_out = td / "low-rate.wav", td / "low-rate-out.wav"
    sf.write(low_src, np.zeros(8000, dtype=np.float32), 8000, subtype="FLOAT")
    must_fail(process(low_src, low_out, "Acoustic", "headphones"), low_out, "Nyquist", "Nyquist")
    # NaN input must fail before creating the destination.
    nan_src, nan_out = td / "nan.wav", td / "nan-out.wav"
    sf.write(nan_src, np.array([0.0, np.nan, 0.0], dtype=np.float32), 48000, subtype="FLOAT")
    must_fail(process(nan_src, nan_out, "Flat", "speakers"), nan_out, "NaN input", "NaN/Inf")
    # Deterministic repeat: two runs from identical input produce identical bytes.
    repeat_out = td / "repeat.wav"
    proc = process(src, repeat_out, "AdventureAndAction", "speakers")
    assert proc.returncode == 0, proc.stderr
    assert repeat_out.read_bytes() == out_s.read_bytes(), "repeat run is not deterministic"
    listed = subprocess.run([str(exe), "--list-eq-presets"], capture_output=True,
                            text=True, check=True)
    assert len(listed.stdout.splitlines()) == 34
assert not td.exists(), f"this run's temp dir was not cleaned: {td}"
print("E2E PASS: 34 presets; float32 WAV; 48 kHz/stereo/frame preservation; "
      "left-only excitation keeps R exactly 0; Speaker flat identity; signed -6 dB attenuation; "
      "+6 dB not limited; 31 Hz response {:.3f} dB; huge/non-finite gains (including silent "
      "+1000 dB), invalid preset, hardlink and existing target rejected without output".format(measured))
