#!/usr/bin/env python3
"""USBPcap single-band capture and `5a 11` parameter-notification parser.

The parser is device-independent and self-tests against the existing
capture/g6-usb-20261112/scout-01.pcap. `--capture` starts USBPcapCMD, waits
while the operator changes one EQ band, then parses the capture.
"""
import argparse
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_PCAP = HERE.parents[1] / "capture" / "g6-usb-20261112" / "scout-01.pcap"
TSHARK = Path("C:/Program Files/Wireshark/tshark.exe")
USBCAP = Path("C:/Program Files/USBPcap/USBPcapCMD.exe")


def extract_payloads(pcap: Path, tshark: Path) -> list[tuple[int, str]]:
    command = [str(tshark), "-r", str(pcap), "-Y", "usb.capdata",
               "-T", "fields", "-e", "frame.number", "-e", "usb.capdata"]
    output = subprocess.run(command, check=True, capture_output=True, text=True).stdout
    frames = []
    for line in output.splitlines():
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        raw = parts[1].replace(":", "").strip()
        if raw:
            frames.append((int(parts[0]), raw))
    return frames


def parse_notification(hexstr: str):
    try:
        raw = bytes.fromhex(hexstr)
    except ValueError:
        return None
    if len(raw) < 5 or raw[0] != 0x5A or raw[1] != 0x11:
        return None
    length = raw[2]
    count = raw[3]
    if length != 2 + 6 * count:
        return None
    entries = []
    offset = 5
    for _ in range(count):
        if offset + 6 > len(raw) or raw[offset] != 0x96:
            return None
        identifier = raw[offset + 1]
        value = struct.unpack("<f", raw[offset + 2:offset + 6])[0]
        entries.append((identifier, value))
        offset += 6
    return {"length": length, "count": count, "entries": entries, "hex": hexstr}


def parse_capture(pcap: Path, tshark: Path) -> list[dict]:
    frames = []
    for frame, hexstr in extract_payloads(pcap, tshark):
        parsed = parse_notification(hexstr)
        if parsed:
            parsed["frame"] = frame
            frames.append(parsed)
    return frames


def single_band_diff(frames: list[dict]) -> dict:
    if not frames:
        return {"changed": [], "added": [], "removed": []}
    first = dict(frames[0]["entries"])
    last = dict(frames[-1]["entries"])
    changed = [{"id": key, "before": first[key], "after": last[key]}
               for key in sorted(first.keys() & last.keys())
               if abs(first[key] - last[key]) > 1e-6]
    added = [{"id": key, "after": last[key]} for key in sorted(last.keys() - first.keys())]
    removed = [{"id": key, "before": first[key]} for key in sorted(first.keys() - last.keys())]
    return {"first_frame": frames[0]["frame"], "last_frame": frames[-1]["frame"],
            "changed": changed, "added": added, "removed": removed}


def self_test() -> int:
    if not DEFAULT_PCAP.is_file():
        print(f"missing self-test capture: {DEFAULT_PCAP}")
        return 2
    tshark = TSHARK if TSHARK.is_file() else Path("tshark")
    frames = parse_capture(DEFAULT_PCAP, tshark)
    failures = 0
    if len(frames) < 4:
        print(f"expected at least 4 notifications, found {len(frames)}")
        failures += 1
    if not any(frame["count"] == 8 for frame in frames):
        print("no 8-entry notification found")
        failures += 1
    if not any(frame["count"] == 2 for frame in frames):
        print("no 2-entry notification found")
        failures += 1
    if not any(frame["count"] == 8 and any(identifier == 0 and abs(value - 1.0) < 1e-6
                                           for identifier, value in frame["entries"])
               for frame in frames):
        print("no 8-entry frame with id 0 = 1.0")
        failures += 1
    if failures:
        print(f"FAIL ({failures} checks)")
        return 1
    print(f"PASS: parsed {len(frames)} `5a 11` notifications; "
          f"max entries {max(frame['count'] for frame in frames)}")
    return 0


def capture(output: Path, seconds: float, interface: str,
            report: Path | None = None) -> int:
    if not USBCAP.is_file():
        print(f"USBPcapCMD not found: {USBCAP}")
        return 2
    print(f"capture interface: {interface}")
    print("Start the capture, then change exactly one EQ band in Sound Blaster Command.")
    process = subprocess.Popen([str(USBCAP), "-d", interface, "-o", str(output)])
    try:
        time.sleep(seconds)
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
    if not output.is_file() or output.stat().st_size == 0:
        print("capture produced no pcap; run from an elevated prompt and verify the interface")
        return 2
    print(f"captured {output} ({output.stat().st_size} bytes)")
    return parse_and_report(output, TSHARK if TSHARK.is_file() else Path("tshark"),
                            report)


def parse_and_report(pcap: Path, tshark: Path, report: Path | None = None) -> int:
    frames = parse_capture(pcap, tshark)
    if not frames:
        print("no `5a 11` notifications found; was SB Command running and a band changed?")
        return 2
    for frame in frames[:20]:
        preview = ", ".join(f"{identifier}={value:.6g}"
                            for identifier, value in frame["entries"])
        print(f"frame {frame['frame']} count {frame['count']}: {preview}")
    diff = single_band_diff(frames)
    print(json.dumps(diff, indent=2))
    if report is not None:
        compact = [{"frame": frame["frame"], "count": frame["count"],
                    "entries": frame["entries"]} for frame in frames]
        report.write_text(json.dumps({"pcap": str(pcap), "frames": compact,
                                      "diff": diff}, indent=2), encoding="utf-8")
        print(f"written: {report}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--parse", action="store_true")
    parser.add_argument("--capture", action="store_true")
    parser.add_argument("--pcap", type=Path)
    parser.add_argument("--output", type=Path, default=Path("scout-band.pcap"))
    parser.add_argument("--report", type=Path)
    parser.add_argument("--seconds", type=float, default=20.0)
    parser.add_argument("--interface", default=r"\\.\USBPcap1")
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    if args.capture:
        return capture(args.output, args.seconds, args.interface, args.report)
    if args.parse:
        if not args.pcap:
            parser.error("--parse requires --pcap")
        tshark = TSHARK if TSHARK.is_file() else Path("tshark")
        return parse_and_report(args.pcap, tshark, args.report)
    parser.error("choose --self-test, --parse or --capture")


if __name__ == "__main__":
    raise SystemExit(main())
