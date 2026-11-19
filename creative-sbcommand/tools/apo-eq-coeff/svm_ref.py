"""Independent check of the Smart Volume AGC probe."""
import struct
import sys

import numpy as np

F32 = np.float32


def bits(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def from_bits(word):
    return struct.unpack("<f", struct.pack("<I", word))[0]


def time_constant(fs, seconds):
    return F32(F32(1.0) - F32(10.0) ** F32(F32(-1.0) / F32(F32(fs) * F32(seconds))))


def main(path):
    lines = [line.split() for line in open(path, encoding="utf-8") if line.strip()]
    data = {}
    for parts in lines:
        data[parts[0]] = parts[1:]

    failures = 0
    fs = 48000
    expected = {
        "k1": 0.98710001,
        "k2": -0.97420001,
        "alpha": time_constant(fs, 0.04),
        "beta": time_constant(fs, 0.01),
        "attack": time_constant(fs, 0.0018638),
        "release": time_constant(fs, 8.0),
        "ramp": 1.0 / (0.08 * fs),
    }
    actual = data["constants"]
    names = ["k1", "k2", "alpha", "beta", "attack", "release", "ramp"]
    for name, word in zip(names, actual[:7]):
        value = from_bits(int(word, 16))
        if abs(value - float(expected[name])) > 1e-6:
            print(f"{name} mismatch: {value} vs {expected[name]}")
            failures += 1
    if int(actual[7]) != 30:
        print(f"lookahead mismatch: {actual[7]}")
        failures += 1

    quiet_gain = from_bits(int(data["quiet_gain"][0], 16))
    if abs(quiet_gain - 1.0) > 0.02:
        print(f"quiet gain should stay near 1: {quiet_gain}")
        failures += 1

    loud = data["loud"]
    input_rms = from_bits(int(loud[1], 16))
    output_rms = from_bits(int(loud[3], 16))
    loud_gain = from_bits(int(loud[5], 16))
    if not (loud_gain < 0.5):
        print(f"loud gain was not compressed: {loud_gain}")
        failures += 1
    if not (output_rms < input_rms * 0.5):
        print(f"loud RMS not reduced: {input_rms} -> {output_rms}")
        failures += 1

    release_gain = from_bits(int(data["release_gain"][0], 16))
    if not (release_gain > loud_gain + 0.1):
        print(f"release did not recover: loud {loud_gain}, release {release_gain}")
        failures += 1

    if failures:
        print(f"FAIL ({failures} checks)")
        return 1
    print("PASS: SVM filter/time constants, compression and release match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "svm_out.txt"))
