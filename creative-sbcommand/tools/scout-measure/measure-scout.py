#!/usr/bin/env python3
"""Scout measurement orchestration.

No-device paths:
  --self-test          run the sweep generator and analyzer self-tests
  --analyze ...        analyze pre-recorded off/on WAV files

Device path (requires a G6 endpoint and a loopback or physical input):
  --record             play the sweep and record off/on captures

The script never changes device settings; the operator toggles Scout in
Sound Blaster Command between the two recordings.
"""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def run(script: str, *args: str) -> int:
    return subprocess.call([sys.executable, "-I", str(HERE / script), *args])


def self_test() -> int:
    with tempfile.TemporaryDirectory(prefix="scout-self-test-") as tmp:
        if run("generate-sweep.py", "--output-dir", tmp) != 0:
            return 1
        if run("analyze-response.py", "--self-test") != 0:
            return 1
        if run("analyze-response.py", "--make-test-wavs", tmp) != 0:
            return 1
        output = Path(tmp) / "cli-response.json"
        if run("analyze-response.py",
               "--sweep", str(Path(tmp) / "test-sweep.wav"),
               "--off", str(Path(tmp) / "test-off.wav"),
               "--on", str(Path(tmp) / "test-on.wav"),
               "--output", str(output)) != 0:
            return 1
        result = json.loads(output.read_text(encoding="utf-8"))
        gains = [2.0, -1.0, 0.0, 3.0, -2.0, 1.0, 0.0, -4.0, 2.0, 1.0]
        for band, gain in zip(result["bands"], gains):
            if abs(band["delta_db"] - (gain - 1.0)) > 0.8:
                print(f"CLI self-test band {band['hz']} Hz: {band['delta_db']} vs {gain - 1.0}")
                return 1
    print("PASS: sweep generation, synthetic recovery and end-to-end CLI")
    return 0


def analyze(args) -> int:
    command = ["--sweep", str(args.sweep), "--off", str(args.off),
               "--on", str(args.on), "--output", str(args.output)]
    if args.subtract_preamp:
        command.append("--subtract-preamp")
    return run("analyze-response.py", *command)


def record(args) -> int:
    try:
        import numpy as np
        import sounddevice as sd
        import soundfile as sf
    except Exception as error:
        print(f"audio libraries unavailable: {error}")
        return 2

    devices = sd.query_devices()
    output_index = None
    input_index = None
    for index, device in enumerate(devices):
        name = str(device.get("name", ""))
        if output_index is None and device.get("max_output_channels", 0) > 0 \
                and args.device_match.lower() in name.lower():
            output_index = index
        if input_index is None and device.get("max_input_channels", 0) > 0 \
                and args.input_match.lower() in name.lower():
            input_index = index
    if output_index is None:
        print(f"no output device matches {args.device_match!r}; connect the G6 or use --analyze on pre-recorded files")
        return 2
    if input_index is None:
        print(f"no input device matches {args.input_match!r}; use a physical loopback cable "
              "or record with another tool and run --analyze")
        return 2

    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    sweep_path = out_dir / "scout-sweep-float32.wav"
    if not sweep_path.exists():
        if run("generate-sweep.py", "--output-dir", str(out_dir)) != 0:
            return 3
    sweep, rate = sf.read(str(sweep_path), dtype="float32", always_2d=True)
    info = sd.query_devices(output_index)
    input_info = sd.query_devices(input_index)
    print(f"output: {output_index} {info['name']}")
    print(f"input:  {input_index} {input_info['name']}")

    def capture(path: Path, prompt: str) -> Path:
        input(prompt)
        recorded = []

        def callback(indata, frames, timing, status):
            if status:
                print(f"input status: {status}")
            recorded.append(indata.copy())

        with sd.InputStream(device=input_index, samplerate=rate, channels=2,
                            dtype="float32", callback=callback):
            sd.play(sweep, samplerate=rate, device=output_index, blocking=True)
            sd.sleep(int(args.tail_seconds * 1000))
        sf.write(str(path), np.concatenate(recorded), rate, subtype="FLOAT")
        print(f"recorded {path} ({len(recorded)} blocks)")
        return path

    off_path = capture(out_dir / f"{args.label}-off.wav", args.prompt_off)
    on_path = capture(out_dir / f"{args.label}-on.wav", args.prompt_on)
    result = analyze(argparse.Namespace(
        sweep=sweep_path, off=off_path, on=on_path,
        output=out_dir / f"{args.label}-response.json",
        subtract_preamp=args.subtract_preamp))
    (out_dir / f"{args.label}-measure-report.json").write_text(json.dumps({
        "output_device": str(info["name"]),
        "input_device": str(input_info["name"]),
        "sweep": str(sweep_path),
        "off": str(off_path),
        "on": str(on_path),
        "result": str(out_dir / f"{args.label}-response.json"),
    }, indent=2), encoding="utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--analyze", action="store_true")
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--sweep", type=Path)
    parser.add_argument("--off", type=Path)
    parser.add_argument("--on", type=Path)
    parser.add_argument("--output", type=Path, default=Path("scout-response.json"))
    parser.add_argument("--subtract-preamp", action="store_true")
    parser.add_argument("--output-dir", default="scout-measure-out")
    parser.add_argument("--device-match", default="G6")
    parser.add_argument("--input-match", default="")
    parser.add_argument("--tail-seconds", type=float, default=1.0)
    parser.add_argument("--label", default="scout")
    parser.add_argument("--prompt-off",
                        default="Set Scout OFF in Sound Blaster Command, then press Enter...")
    parser.add_argument("--prompt-on",
                        default="Set Scout ON in Sound Blaster Command, then press Enter...")
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    if args.analyze:
        if not (args.sweep and args.off and args.on):
            parser.error("--analyze requires --sweep, --off and --on")
        return analyze(args)
    if args.record:
        return record(args)
    parser.error("choose --self-test, --analyze or --record")


if __name__ == "__main__":
    raise SystemExit(main())
