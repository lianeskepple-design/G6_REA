"""Independent file-output checks for sbc-eq; no audio devices are opened.

The test workspace is the project-local tools/.offline-eq-test directory (a
unique temporary run directory inside it), so repeated runs do not accumulate
files or collide with previous runs. Results are written even if an unexpected
harness error occurs, so a stale JSON report is never left as acceptance
evidence.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

import numpy as np
import soundfile as sf
from scipy import signal

root = Path(__file__).resolve().parent.parent
exe = root / "app/target/debug/sbc-eq.exe"
data = json.loads((root / "reference/g6-eq-presets.json").read_text(encoding="utf-8"))["eqPresets"]
tests = []


def record(name, ok, **details):
    tests.append({"name": name, "passed": bool(ok), **details})


def run(src, out, preset="FootstepEnhancer", mode="headphones", extra=()):
    return subprocess.run(
        [str(exe), "--process-wav", str(src), str(out), "--eq-preset", preset,
         "--output-mode", mode, *extra],
        capture_output=True, text=True, encoding="utf-8", errors="replace", check=False,
    )


testroot = root / "tools/.offline-eq-test"
testroot.mkdir(exist_ok=True)
try:
    with tempfile.TemporaryDirectory(prefix="independent-", dir=testroot) as tmp:
        td = Path(tmp)
        sr = 48000
        x = np.zeros((16384, 2), np.float32)
        x[0, 0] = 0.25
        src = td / "impulse.wav"
        sf.write(src, x, sr, subtype="FLOAT")
        original_hash = hashlib.sha256(src.read_bytes()).hexdigest()
        out = td / "filtered.wav"
        proc = run(src, out)
        if proc.returncode == 0 and out.exists():
            y, rate = sf.read(out, always_2d=True)
            # Documented spec (reference/apo-effects-structure.md): band 1 low shelf,
            # bands 2..9 peaking with alpha = tan(w0/(2*ratio)), band 10 high shelf.
            # Zero-gain bands are bypassed by the CLI, so they are identity here too.
            sos = []
            curve = data["FootstepEnhancer"]["settings"]["Headphone"]
            freqs = curve["freqs"]
            last = len(freqs) - 1
            for index, (fc, db) in enumerate(zip(freqs, curve["values"])):
                if db == 0:
                    sos.append([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
                    continue
                amplitude = 10 ** (db / 40)
                omega = 2 * np.pi * fc / sr
                cosine = np.cos(omega)
                if index in (0, last):
                    alpha = np.sin(omega) / 2 * np.sqrt(2)   # RBJ shelf, slope S=1
                    t = 2 * np.sqrt(amplitude) * alpha
                    p1, m1 = amplitude - 1.0, amplitude + 1.0
                    if index == 0:
                        num = [amplitude * (m1 - p1 * cosine + t),
                               2 * amplitude * (p1 - m1 * cosine),
                               amplitude * (m1 - p1 * cosine - t)]
                        den = [m1 + p1 * cosine + t,
                               -2 * (p1 + m1 * cosine),
                               m1 + p1 * cosine - t]
                    else:
                        num = [amplitude * (m1 + p1 * cosine + t),
                               -2 * amplitude * (p1 + m1 * cosine),
                               amplitude * (m1 + p1 * cosine - t)]
                        den = [m1 - p1 * cosine + t,
                               2 * (p1 - m1 * cosine),
                               m1 - p1 * cosine - t]
                    a0 = den[0]
                    sos.append([num[0] / a0, num[1] / a0, num[2] / a0, 1.0, den[1] / a0, den[2] / a0])
                else:
                    alpha = np.tan(omega / (2 * (freqs[index + 1] / fc)))
                    a0 = 1 + alpha / amplitude
                    sos.append([(1 + alpha * amplitude) / a0, -2 * cosine / a0,
                                (1 - alpha * amplitude) / a0, 1, -2 * cosine / a0,
                                (1 - alpha / amplitude) / a0])
            reference = signal.sosfilt(sos, x, axis=0)
            error = float(np.max(np.abs(y - reference)))
            record("Footstep impulse vs SciPy SOS", error < 1e-7, max_abs_error=error)
            # Left-only excitation: the right channel must decode to exactly zero.
            record("channel isolation (left-only excitation, right == 0)",
                   np.max(np.abs(y[:, 1])) == 0.0)
            record("shape and rate", y.shape == x.shape and rate == sr)
        else:
            record("Footstep processing", False,
                   stderr=proc.stderr or "binary returned 0 without an output file")
        flat = td / "flat.wav"
        proc = run(src, flat, "Flat", "speakers")
        y = sf.read(flat, always_2d=True)[0] if proc.returncode == 0 and flat.exists() else np.empty((0, 2))
        record("Flat identity", proc.returncode == 0 and flat.exists() and np.array_equal(x, y))
        attenuated = td / "attenuated.wav"
        proc = run(src, attenuated, "Flat", "speakers", ("--headroom-db", "-6"))
        y = (sf.read(attenuated, always_2d=True)[0]
             if proc.returncode == 0 and attenuated.exists() else np.zeros_like(x))
        record("signed -6 dB attenuation",
               proc.returncode == 0 and attenuated.exists()
               and np.max(np.abs(y - x * 10 ** (-6 / 20))) < 1e-7)
        # Positive headroom is a signed gain, never an automatic anti-clipping stage.
        loud_src, loud_out = td / "loud.wav", td / "loud-out.wav"
        loud = np.zeros((4096, 2), np.float32)
        loud[:, 0] = 0.8
        sf.write(loud_src, loud, sr, subtype="FLOAT")
        proc = run(loud_src, loud_out, "Flat", "speakers", ("--headroom-db", "6"))
        if proc.returncode == 0 and loud_out.exists():
            boosted, _ = sf.read(loud_out, always_2d=True, dtype="float32")
            peak = float(np.max(boosted))
        else:
            peak = float("nan")
        record("positive +6 dB is gain, not a limiter",
               proc.returncode == 0 and np.isfinite(peak) and peak > 1.0
               and abs(peak - 0.8 * 10 ** (6 / 20)) < 1e-6, peak=peak)
        existing = td / "existing.wav"
        existing.write_bytes(b"preserve me")
        proc = run(src, existing)
        record("existing output preserved",
               proc.returncode != 0 and existing.read_bytes() == b"preserve me")
        alias = td / "hardlink.wav"
        os.link(src, alias)
        proc = run(src, alias)
        record("hardlink output rejected without overwrite",
               proc.returncode != 0
               and hashlib.sha256(src.read_bytes()).hexdigest() == original_hash
               and hashlib.sha256(alias.read_bytes()).hexdigest() == original_hash)
        proc = run(src, src)
        record("same input output preserved",
               proc.returncode != 0 and hashlib.sha256(src.read_bytes()).hexdigest() == original_hash)
        # Gain validation: +1000 dB stays finite in f64 but exceeds float32;
        # +1e10 dB overflows the f64 power; non-finite dB values are rejected.
        for name, db, needle in (
            ("huge +1000 dB rejected without output", "1000", "float32"),
            ("+1e10 dB pow gain overflow rejected without output", "1e10", "finite"),
            ("negative infinity headroom rejected without output", "-inf", "finite"),
            ("NaN headroom rejected without output", "nan", "finite"),
            ("positive infinity headroom rejected without output", "inf", "finite"),
        ):
            out_path = td / ("invalid-" + str(len(tests)) + ".wav")
            proc = run(src, out_path, "Flat", "speakers", ("--headroom-db", db))
            record(name, proc.returncode != 0 and not out_path.exists() and needle in proc.stderr,
                   exit_code=proc.returncode)
        # The scalar gain is rejected before per-sample processing, so silent
        # input cannot hide a float32-overflowing headroom value.
        silent_src, silent_out = td / "silent.wav", td / "silent-out.wav"
        sf.write(silent_src, np.zeros((4096, 2), np.float32), sr, subtype="FLOAT")
        proc = run(silent_src, silent_out, "Flat", "speakers", ("--headroom-db", "1000"))
        record("silent input +1000 dB rejected without output",
               proc.returncode != 0 and not silent_out.exists() and "float32" in proc.stderr,
               exit_code=proc.returncode)
        bad = td / "invalid.wav"
        proc = run(src, bad, "does-not-exist")
        record("invalid preset no output", proc.returncode != 0 and not bad.exists())
        for subtype in ["PCM_16", "PCM_24", "PCM_32", "FLOAT"]:
            source, output = td / (subtype + ".wav"), td / (subtype + "-out.wav")
            sf.write(source, x, sr, subtype=subtype)
            proc = run(source, output, "Flat", "speakers")
            ok = (proc.returncode == 0 and output.exists()
                  and sf.info(output).subtype == "FLOAT" and sf.info(output).frames == len(x))
            if ok:
                got = sf.read(output, always_2d=True, dtype="float32")[0]
                ref = sf.read(source, always_2d=True, dtype="float32")[0]
                ok = np.max(np.abs(got - ref)) < 1e-6
            record("WAV " + subtype, ok)
        fps_s, fps_h = td / "fps-speaker.wav", td / "fps-headphone.wav"
        p_s, p_h = run(src, fps_s, "FPS", "speakers"), run(src, fps_h, "FPS", "headphones")
        fps_ok = p_s.returncode == p_h.returncode == 0 and fps_s.exists() and fps_h.exists()
        if fps_ok:
            fps_ok = np.max(np.abs(sf.read(fps_s)[0] - sf.read(fps_h)[0])) > 1e-4
        record("FPS modes truly differ", fps_ok)
        repeat = td / "filtered-repeat.wav"
        proc = run(src, repeat)
        record("repeat run byte-identical", proc.returncode == 0 and out.exists()
               and repeat.exists() and repeat.read_bytes() == out.read_bytes())
        listed = subprocess.run([str(exe), "--list-eq-presets"], capture_output=True,
                                text=True, check=False)
        lines = listed.stdout.splitlines()
        record("exact 34 factory IDs", listed.returncode == 0 and len(data) == 34
               and len(lines) == 34 and set(lines) == set(data))
except Exception as exc:
    record("harness completed without exception", False,
           error=f"{type(exc).__name__}: {exc}")

result = {"date": "2026-11-12", "binary_sha256": hashlib.sha256(exe.read_bytes()).hexdigest(),
          "tests": tests, "passed": sum(t["passed"] for t in tests), "total": len(tests)}
(root / "reference/offline-independent-results.json").write_text(
    json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(result, ensure_ascii=False, indent=2))
raise SystemExit(0 if all(t["passed"] for t in tests) else 1)
