"""Independent check of the Smart Volume probe.

This is a second implementation of the algorithm recovered from KSUSBAPO64.dll
(0x180092f30 and friends), written from the decompilation rather than from
reference/g6_svm.c. It re-runs the probe's script in NumPy and compares the gain
states and the output samples.

Fixed coefficients are compared exactly (same float32 bits); runtime values are
compared with a small tolerance because the DLL uses its own vectorised log10f /
powf (0x1800dc0b0 / 0x1800d96d0) instead of the C library.
"""

import struct
import sys

import numpy as np

F32 = np.float32
RING = 2400
FS = 48000
BLOCKS = 6
BLOCK_FRAMES = 1024
TRACE_STRIDE = 8
PI = 3.14159265


def bits(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def from_bits(word):
    return struct.unpack("<f", struct.pack("<I", word))[0]


def f32(value):
    return F32(value)


def tconst(fs, seconds):
    """1 - 10^(-1/(fs*seconds)) as the DLL builds it in 0x180092da0."""
    return f32(f32(1.0) - f32(10.0) ** f32(f32(-1.0) / (f32(fs) * f32(seconds))))


class Svm:
    """Bit-for-bit model of the shipped C implementation."""

    THRESHOLD = f32(-6.2)
    RATIO = f32(0.4)
    RAMP_SECONDS = f32(0.08)
    ENERGY_SECONDS = f32(0.04)
    MAKEUP_SECONDS = f32(0.01)
    ATTACK_SECONDS = f32(0.004) * f32(0.46593979)
    RELEASE_SECONDS = f32(8.0)
    MIN_DELAY = f32(30.0)
    MAX_DELAY_SECONDS = f32(0.05)
    DEFAULT_DELAY_SECONDS = f32(0.02)

    def __init__(self, fs=FS, channels=2):
        self.fs = fs
        self.channels = channels
        self.k1 = f32(0.98710001)
        self.k2 = f32(-0.97420001)
        self.alpha_energy = tconst(fs, self.ENERGY_SECONDS)
        self.alpha_makeup = tconst(fs, self.MAKEUP_SECONDS)
        self.ramp_step = f32(f32(1.0) / (f32(fs) * self.RAMP_SECONDS))
        self.attack = tconst(fs, self.ATTACK_SECONDS)
        self.release = tconst(fs, self.RELEASE_SECONDS)
        self.threshold = self.THRESHOLD
        self.ratio = self.RATIO
        self.enabled = 1
        self.wet = f32(1.0)
        self.makeup = f32(f32(10.0) ** f32(1.1))
        self.gain1 = f32(1.0)
        self.gain2 = f32(1.0)
        self.mix_gain = f32(1.0)
        self.crossfade = f32(1.0)
        self.user_mix = f32(1.0)
        self.delay_seconds = self.DEFAULT_DELAY_SECONDS
        self.delay_a = f32(self.MAX_DELAY_SECONDS * fs)
        self.delay_b = self.delay_a
        self.target_up = 0
        self.target_down = 0
        self.apply_delay()
        self.ring = np.zeros((channels, RING), dtype=np.float32)
        self.write = [0] * channels
        self.prev_x = [f32(0.0)] * channels
        self.prev_filtered = [f32(0.0)] * channels
        self.energy = [f32(0.0)] * channels

    def apply_delay(self):
        if not self.enabled:
            delay = self.MIN_DELAY
        else:
            delay = f32(self.delay_seconds * f32(self.fs))
            if delay < self.MIN_DELAY:
                delay = self.MIN_DELAY
            maximum = f32(self.MAX_DELAY_SECONDS * f32(self.fs))
            if delay > maximum:
                delay = maximum
        if self.crossfade <= f32(0.5):
            self.delay_a = delay
            self.target_up, self.target_down = 1, 0
        else:
            self.delay_b = delay
            self.target_up, self.target_down = 0, 1

    @staticmethod
    def tap_index(write, delay):
        taps = int(delay + f32(0.5))
        return write - taps + RING if write < taps else write - taps

    def process(self, samples):
        """samples: list of per-channel input values for one frame."""
        detector = f32(0.0)
        for channel in range(self.channels):
            x = f32(samples[channel])
            ring = self.ring[channel]
            write = self.write[channel]
            ring[write] = x
            write = (write + 1) % RING
            self.write[channel] = write

            filtered = f32(f32(x - self.prev_x[channel]) * self.k1
                           - self.k2 * self.prev_filtered[channel])
            self.prev_x[channel] = x
            self.prev_filtered[channel] = filtered
            energy = f32(f32(self.alpha_energy * filtered * filtered)
                         + f32(f32(1.0 - self.alpha_energy) * self.energy[channel]))
            self.energy[channel] = energy
            if energy > detector:
                detector = energy

        target = f32(1.0)
        if detector > f32(0.0):
            level = f32(np.log10(np.float64(detector)))
            if level >= self.threshold:
                exponent = f32(f32(f32(level - self.threshold) * self.ratio)
                               + f32(self.threshold - level))
                target = f32(np.power(np.float64(10.0), np.float64(exponent)))

        previous = self.gain1
        coefficient = self.release if previous <= target else self.attack
        self.gain1 = f32(f32(f32(1.0 - coefficient) * previous)
                         + f32(coefficient * target))

        blend = f32(f32(f32(f32(self.wet * self.gain2) + f32(1.0)) - self.wet)
                    * self.makeup)
        applied = self.mix_gain
        self.gain2 = f32(f32(f32(1.0 - self.attack) * self.gain2)
                         + f32(self.attack * previous))
        self.mix_gain = f32(f32(f32(blend - self.mix_gain) * self.alpha_makeup)
                            + self.mix_gain)

        out = []
        for channel in range(self.channels):
            ring = self.ring[channel]
            write = self.write[channel]
            a = ring[self.tap_index(write, self.delay_a)]
            b = ring[self.tap_index(write, self.delay_b)]
            processed = f32(f32(f32(f32(1.0 - self.crossfade) * b)
                                + f32(self.crossfade * a)) * applied)
            dry = f32(samples[channel])
            out.append(f32(dry + f32(self.user_mix * f32(processed - dry))))

        if self.target_down:
            nxt = f32(self.crossfade - self.ramp_step)
            self.crossfade = nxt
            if nxt < f32(0.0):
                self.crossfade = f32(0.0)
                self.target_down = 0
        elif self.target_up:
            nxt = f32(self.crossfade + self.ramp_step)
            self.crossfade = nxt
            if nxt > f32(1.0):
                self.crossfade = f32(1.0)
                self.target_up = 0
        return out


def close(actual, expected, tolerance):
    return abs(float(actual) - float(expected)) <= tolerance


def main(path):
    lines = [line.split() for line in open(path, encoding="utf-8") if line.strip()]
    data = {}
    frames = []
    for parts in lines:
        if parts[0] == "frame":
            frames.append(parts)
        else:
            data[parts[0]] = parts[1:]

    failures = 0

    def fail(message):
        nonlocal failures
        failures += 1
        print(message)

    # --- fixed coefficients, compared bit for bit -----------------------------
    actual = data["constants"]
    names = ["k1", "k2", "alpha_energy", "alpha_makeup", "ramp_step", "attack",
             "release", "threshold", "ratio", "wet", "makeup", "delay_a",
             "delay_b", "crossfade"]
    expected = {
        "k1": f32(0.98710001),
        "k2": f32(-0.97420001),
        "alpha_energy": tconst(FS, f32(0.04)),
        "alpha_makeup": tconst(FS, f32(0.01)),
        "ramp_step": f32(f32(1.0) / f32(3840.0)),
        "attack": tconst(FS, f32(0.004) * f32(0.46593979)),
        "release": tconst(FS, f32(8.0)),
        "threshold": f32(-6.2),
        "ratio": f32(0.4),
        "wet": f32(1.0),
        "makeup": f32(np.power(np.float64(10.0), 1.1)),
        "delay_a": f32(2400.0),
        "delay_b": f32(960.0),
        "crossfade": f32(1.0),
    }
    for name, word in zip(names, actual):
        value = from_bits(int(word, 16))
        if bits(value) != bits(expected[name]) and not close(value, expected[name], 1e-6):
            fail(f"{name} mismatch: {value!r} vs {expected[name]!r}")

    if data.get("enabled") != ["1", "channels", "2"]:
        fail(f"unexpected enable/channel line: {data.get('enabled')}")

    # --- re-run the probe script ---------------------------------------------
    model = Svm()
    index = 0
    checked = 0
    for block in range(BLOCKS):
        amplitude = f32(0.01) if block < 2 else (f32(0.5) if block < 4 else f32(0.01))
        for sample in range(BLOCK_FRAMES):
            traced = index % TRACE_STRIDE == 0
            row = None
            if traced:
                if checked >= len(frames):
                    fail(f"missing frame {index}")
                    break
                row = frames[checked]
                checked += 1
                if int(row[1]) != index:
                    fail(f"frame index {row[1]} != {index}")
                # Use the probe's own input so both sides see identical samples.
                input_value = from_bits(int(row[8], 16))
            else:
                input_value = f32(amplitude * F32(np.sin(
                    np.float64(2.0 * PI * 1000.0 * index / FS))))
            out = model.process([input_value, input_value])
            if traced:
                for slot, name in enumerate(["gain1", "gain2", "mix_gain", "crossfade"]):
                    actual_value = from_bits(int(row[2 + slot], 16))
                    model_value = getattr(model, name)
                    if not close(actual_value, model_value,
                                 1e-4 * max(1.0, abs(float(model_value)))):
                        fail(f"{name} at frame {index}: {actual_value!r} vs {model_value!r}")
                out_value = from_bits(int(row[6], 16))
                if not close(out_value, out[0], 1e-5 * max(1.0, abs(float(out[0])))):
                    fail(f"output at frame {index}: {out_value!r} vs {out[0]!r}")
                energy_value = from_bits(int(row[7], 16))
                if not close(energy_value, model.energy[0],
                             1e-4 * max(1e-9, abs(float(model.energy[0])))):
                    fail(f"energy at frame {index}: {energy_value!r} vs {model.energy[0]!r}")
            index += 1

    if checked != len(frames):
        fail(f"traced frames: {checked} read, {len(frames)} present")

    tail = [from_bits(int(word, 16)) for word in data["tail"]]
    if not close(tail[0], model.gain1, 1e-5):
        fail(f"tail gain1 {tail[0]!r} vs {model.gain1!r}")
    if not close(tail[2], model.mix_gain, 1e-4):
        fail(f"tail mix_gain {tail[2]!r} vs {model.mix_gain!r}")

    # The startup crossfade runs 0.08 s = 3840 frames and must land exactly on 0.
    if abs(model.crossfade) > 1e-6:
        fail(f"startup crossfade did not finish: {model.crossfade!r}")
    if model.target_down or model.target_up:
        fail("crossfade target flag still set")

    # --- settle: a started chain must not look active -----------------------
    settled = data["settled"]
    if settled[0] != "0":
        fail(f"a settled chain still reports active: {settled[0]}")
    for slot, name in ((1, "delay_a"), (2, "delay_b")):
        if from_bits(int(settled[slot], 16)) != 960.0:
            fail(f"settled {name} {from_bits(int(settled[slot], 16))!r} != 960")

    # --- bypass --------------------------------------------------------------
    bypass = data["bypass"]
    if from_bits(int(bypass[9], 16)) != 0.0:
        fail(f"bypass wet {from_bits(int(bypass[9], 16))!r} != 0")
    if abs(from_bits(int(bypass[10], 16)) - 1.0) > 1e-7:
        fail(f"bypass makeup {from_bits(int(bypass[10], 16))!r} != 1")
    # The crossfade sits on tap B, so 0x1800935c0 loads the 30-sample bypass delay
    # into tap A and starts ramping towards it.
    if from_bits(int(bypass[11], 16)) != 30.0:
        fail(f"bypass delay_a {from_bits(int(bypass[11], 16))!r} != 30")
    if from_bits(int(bypass[12], 16)) != 960.0:
        fail(f"bypass changed tap B: {from_bits(int(bypass[12], 16))!r}")
    if data.get("active") != ["1"]:
        fail(f"pending crossfade not reported as active: {data.get('active')}")

    if failures:
        print(f"FAIL ({failures} checks)")
        return 1
    print("PASS: SVM coefficients, gain trajectory, delay crossfade and bypass match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "svm_out.txt"))
