"""Independent float32 check of the Crystalizer chain probe.

Reads crystalizer_out.txt (produced by crystalizer_chain_test.exe) and
recomputes the analysis coefficients/impulse responses and the
activity-derived one-pole/biquad coefficients with numpy float32.
"""
import struct
import sys
import math

import numpy as np

F32 = np.float32


def bits(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def f32(value):
    return F32(value)


FIRST_DEN = [
    [2.70068121, -2.44379687, 0.739700139],
    [2.85020638, -2.71109629, 0.86043334],
    [2.86235762, -2.73373818, 0.871024668],
    [2.93110919, -2.86448121, 0.933326185],
    [2.9655323, -2.93163133, 0.966093123],
]
FIRST_NUM = [
    [0.0282577593, -0.0265499298, -0.0265499298, 0.0282577593],
    [0.0150161376, -0.014787958, -0.014787958, 0.0150161376],
    [0.0138701461, -0.0136921862, -0.0136921862, 0.0138701461],
    [0.00715662446, -0.00713364128, -0.00713364128, 0.00715662446],
    [0.00363776833, -0.00363484677, -0.00363484677, 0.00363776833],
]
SECOND_DEN = [
    [-2.63391948, -2.53106618, -0.881794333],
    [-1.05723643, -1.08735812, -0.70898968],
    [-0.793858826, -0.893759787, -0.687055707],
    [0.759954393, -0.191214889, -0.502502382],
    [1.53758836, -0.503638148, -0.238011643],
]
SECOND_NUM = [
    [0.00191905815, -0.00575717445, 0.00575717445, -0.00191905815],
    [0.0401415043, -0.120424509, 0.120424509, -0.0401415043],
    [0.051605653, -0.154816955, 0.154816955, -0.051605653],
    [0.181083351, -0.543250084, 0.543250084, -0.181083351],
    [0.350401849, -1.05120564, 1.05120564, -0.350401849],
]

BANKS = [
    [-0.903836012, -0.950764000, -0.954675019, -0.977081001, -0.988475025,
     0.499179989, 0.720260978, 0.740343988, 0.861871004, 0.928560019,
     0.130436003, 0.521744013, 0.110532001, 0.332439989, 0.604892015,
     0.449999988, 1.19998801, 1.99987805],
    [-0.938269973, -0.968658984, -0.971168995, -0.985481024, -0.992713988,
     0.473345011, 0.704512000, 0.725588977, 0.853541017, 0.924100995,
     0.152164996, 0.608721972, 0.0496179983, 0.182322994, 0.463085502,
     0.449999988, 3.33323193, 2.79976797],
    [-0.955911994, -0.977712989, -0.979505002, -0.989700019, -0.994836986,
     0.388936013, 0.652549028, 0.676826000, 0.825672984, 0.909062028,
     0.260872006, 0.869593978, 0.0406179987, 0.0740619972, 0.399318010,
     0.5, 4.13312197, 3.59996295],
    [-0.964829028, -0.982259989, -0.983690023, -0.991811991, -0.995896995,
     0.358323991, 0.633565009, 0.658981979, 0.815349996, 0.903443992,
     0.304360986, 1.17392504, 0.0246210005, 0.0300810002, 0.399318010,
     0.200000003, 5.19974422, 5.46647501],
]


def parameter_bank(strength):
    u = F32(strength) / F32(100.0)
    level = F32(u * F32(0.8)) if u <= F32(0.5) else F32(F32(F32(u - F32(0.5)) * F32(0.6)) * F32(2.0) + F32(0.4))
    if level <= F32(0.38):
        bank, weight = 0, F32(level / F32(0.38))
    elif level <= F32(0.75):
        bank, weight = 1, F32((level - F32(0.38)) / F32(0.37))
    else:
        bank, weight = 2, F32((level - F32(0.75)) * F32(4.0))
    out = []
    for i in range(18):
        out.append(F32(F32(F32(1.0) - weight) * F32(BANKS[bank][i])
                       + weight * F32(BANKS[bank + 1][i])))
    return out


def impulse(den, num, count=16):
    w1 = w2 = w3 = F32(0.0)
    out = []
    for n in range(count):
        x = F32(1.0) if n == 0 else F32(0.0)
        w = F32(x + F32(den[0]) * w1 + F32(den[1]) * w2 + F32(den[2]) * w3)
        y = F32(F32(num[0]) * w + F32(num[1]) * w1 + F32(num[2]) * w2
                + F32(num[3]) * w3)
        out.append(y)
        w3, w2, w1 = w2, w1, w
    return out


def main(path):
    lines = [line.strip() for line in open(path, encoding="utf-8") if line.strip()]
    data = {}
    for line in lines:
        parts = line.split()
        data[parts[0]] = parts[1:]
    assert data["class"] == ["2"], data["class"]
    failures = 0

    for band, (den, num) in enumerate(
            ((FIRST_DEN[2], FIRST_NUM[2]), (SECOND_DEN[2], SECOND_NUM[2]))):
        actual = [int(x, 16) for x in data[f"analysis{band}"]]
        expected = [bits(f32(v)) for v in den + num]
        if actual != expected:
            print(f"analysis{band} mismatch\n actual  {actual}\n expected {expected}")
            failures += 1
        impulse_actual = [int(x, 16) for x in data[f"impulse{band}"]]
        impulse_expected = [bits(v) for v in impulse(den, num)]
        if impulse_actual != impulse_expected:
            print(f"impulse{band} mismatch\n actual  {impulse_actual}\n expected {impulse_expected}")
            failures += 1

    params = parameter_bank(50)
    s = params[2]
    t = params[7]
    a = F32(F32(1.0) + F32(4.0) * params[11] * F32(0.2))
    b = F32(F32(1.0) + F32(4.0) * params[10] * F32(0.1))
    c1 = t
    c2 = F32(F32(0.5) * F32(F32(a + F32(1.0)) - F32(t * F32(F32(1.0) - a))))
    c3 = F32(F32(0.5) * F32(F32(F32(1.0) - a) - F32(t * F32(a + F32(1.0)))))
    expected_one = [bits(v) for v in (c1, c2, c3)]
    actual_one = [int(x, 16) for x in data["onepole"]]
    if actual_one != expected_one:
        print(f"onepole mismatch\n actual  {actual_one}\n expected {expected_one}")
        failures += 1

    c8 = F32(F32(-2.0) * s)
    c12 = F32(-(s * s))
    x = F32(F32(0.5) * F32(F32(b + F32(1.0)) - F32(s * F32(F32(1.0) - b))))
    y = F32(F32(0.5) * F32(F32(s * F32(b + F32(1.0))) - F32(F32(1.0) - b)))
    expected_biquad = [bits(v) for v in (c8, c12, F32(x * x), F32(F32(2.0) * x * y), F32(y * y))]
    actual_biquad = [int(x, 16) for x in data["biquad"]]
    if actual_biquad != expected_biquad:
        print(f"biquad mismatch\n actual  {actual_biquad}\n expected {expected_biquad}")
        failures += 1

    def from_bits(word):
        return struct.unpack("<f", struct.pack("<I", int(word, 16)))[0]

    if data.get("finite") != ["1"]:
        print("non-finite block output")
        failures += 1

    steady_rms = from_bits(data.get("steady_rms", ["0"])[0])
    if not (steady_rms > 0.0 and np.isfinite(steady_rms)):
        print(f"steady rms invalid: {steady_rms}")
        failures += 1

    block = [int(x, 16) for x in data.get("block", [])]
    if len(block) < 16 or any(v != 0 for v in block[:16]):
        print(f"expected 128-sample latency (leading zeros), got {block[:16]}")
        failures += 1

    if data.get("delay_ok") != ["1"]:
        print("delay_ok failed")
        failures += 1

    if data.get("drymix_ok") != ["1"]:
        print("drymix_ok failed")
        failures += 1

    drymix_delta = from_bits(data.get("drymix_delta", ["0"])[0])
    if not (0.0 <= drymix_delta < 1.0e-6):
        print(f"dry mix delta too large: {drymix_delta}")
        failures += 1

    activity = [from_bits(x) for x in data.get("activity", [])]
    if (len(activity) != 2 or not all(0.0 <= v <= 0.5 for v in activity)
            or max(activity) == 0.0):
        print(f"activity invalid: {activity}")
        failures += 1

    delta = from_bits(data.get("strength_delta", ["0"])[0])
    if not (delta > 0.0 and np.isfinite(delta)):
        print(f"strength delta invalid: {delta}")
        failures += 1

    window = [from_bits(x) for x in data.get("window", [])]
    expected_window = []
    for n in range(8):
        hann = 0.5 * (1.0 - math.cos(2.0 * math.pi * (n % 256) / 256.0))
        ramp = 1.0 / (1.0 - 0.5 * math.sin(math.pi * (n % 128) / 128.0) ** 2)
        expected_window.append(hann * ramp)
    if any(abs(a - b) > 1e-6 for a, b in zip(window, expected_window)):
        print(f"window mismatch\n actual  {window}\n expected {expected_window}")
        failures += 1

    if data.get("output_filter") != [f"{bits(np.float32(0.5)):08x}"]:
        print(f"output filter mismatch: {data.get('output_filter')}")
        failures += 1

    if failures:
        print(f"FAIL ({failures} checks)")
        return 1
    print("PASS: analysis filters, impulses, dynamic coefficients, activity, "
          "strength delta and finite block match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "crystalizer_out.txt"))
