"""Independent check of the CMSS3D matrix probe."""
import struct
import sys

import numpy as np

F32 = np.float32


def f32(value):
    return F32(value)


def bits(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def from_bits(word):
    return struct.unpack("<f", struct.pack("<I", word))[0]


def expected_targets(mask, strength=50):
    u = f32(strength) / f32(100.0)
    front = min(f32(1.0) + u, f32(1.0))
    extra = min(f32(1.0) - u, f32(1.0))
    if mask in (0x3B, 0x33, 0x603, 0x60B):
        return [front, extra, f32(0.0), f32(0.0), f32(0.0)]
    if mask in (0x07, 0x0F):
        return [f32(0.9535), f32(0.0), f32(0.0), f32(0.3015), f32(0.0)]
    if mask in (0x37, 0x3F, 0x607, 0x60F):
        return [f32(0.2875), extra, f32(0.0), f32(0.3015), f32(0.3015)]
    if mask in (0x637, 0x63B, 0x63F, 0x633, 0x70F):
        return [f32(0.2875), f32(extra * f32(0.9535)), f32(0.0),
                f32(0.3015), f32(0.3015)]
    return [f32(0.0)] * 5


def main(path):
    lines = [line.split() for line in open(path, encoding="utf-8") if line.strip()]
    failures = 0
    ramp = None
    state = None
    steady = None

    for parts in lines:
        if parts[0] == "mask":
            mask = int(parts[1], 16)
            strength = int(parts[2])
            actual = [int(x, 16) for x in parts[3:]]
            expected = [bits(v) for v in expected_targets(mask, strength)]
            if actual != expected:
                print(f"mask {mask:#x} mismatch\n actual  {actual}\n expected {expected}")
                failures += 1
        elif parts[0] == "steady":
            steady = [int(x, 16) for x in parts[1:]]
        elif parts[0] == "ramp":
            ramp = [int(x, 16) for x in parts[1:]]
        elif parts[0] == "ramp_state":
            state = (int(parts[1]), int(parts[2], 16))

    targets = expected_targets(0x3F)
    expected_steady = [
        bits(f32(f32(1.0) * targets[0] + f32(0.2) * targets[3])),
        bits(f32(f32(1.0) * targets[0] + f32(0.2) * targets[3])),
        bits(f32(f32(0.5) * targets[1] + f32(0.1) * targets[4])),
        bits(f32(f32(0.5) * targets[1] + f32(0.1) * targets[4])),
        bits(f32(0.0)), bits(f32(0.0)),
    ]
    if steady != expected_steady:
        print(f"steady mismatch\n actual  {steady}\n expected {expected_steady}")
        failures += 1

    step = f32(targets[0] / f32(8.0))
    expected_ramp = [float(f32(step * f32(i))) for i in range(8)]
    actual_ramp = [from_bits(word) for word in ramp]
    if any(abs(a - b) > 1e-6 for a, b in zip(actual_ramp, expected_ramp)):
        print(f"ramp mismatch\n actual  {actual_ramp}\n expected {expected_ramp}")
        failures += 1
    if state != (0, bits(targets[0])):
        print(f"ramp_state mismatch: {state}")
        failures += 1

    if failures:
        print(f"FAIL ({failures} checks)")
        return 1
    print("PASS: CMSS3D mask table, steady matrix and transition ramp match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "surround_out.txt"))
