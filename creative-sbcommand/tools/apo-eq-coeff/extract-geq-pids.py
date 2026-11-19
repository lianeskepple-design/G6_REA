#!/usr/bin/env python3
"""Extract the GraphicEQ / Scout GEQ ten-band PID order from the app.

Reads the static constructor of
Creative.Platform.Devices.Features.PropKeys.CTTHXCommonPropKeys in
Creative.Platform.Devices.dll and pairs every CTPKEY_*_GainN store with
the integer index passed to the indexed-key constructor.

Those PIDs are APO property ids, not the device-side `5a 11` notification
ids. The USB id mapping still requires the online single-band capture.

Requires dnfile (tested with 0.18.0).
"""
import argparse
import json
import struct
import sys
from pathlib import Path

DEFAULT_ASSEMBLY = Path(
    r"C:\Program Files (x86)\Creative\Sound Blaster Command"
    r"\Platform\Creative.Platform.Devices.dll"
)

GRAPHIC_BANDS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
SCOUT_BANDS = [31.25, 62.5, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]

# Operand sizes for the small IL subset used by the .cctor.
_FOUR = {
    0x20, 0x22, 0x27, 0x28, 0x29, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E,
    0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74,
    0x75, 0x79, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0x81, 0x8C, 0x8D, 0x8F,
    0xA2, 0xA3, 0xA4, 0xC2, 0xC6, 0xD0, 0xDD,
}
_ONE = {
    0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x1F, 0xDE,
    0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37,
}
_EIGHT = {0x21, 0x23}


def _method_body(pe, rva):
    head = pe.get_data(rva, 16)
    if not head:
        raise ValueError(f"no data at RVA {rva:#x}")
    if head[0] & 3 == 2:
        size, start = head[0] >> 2, rva + 1
    elif head[0] & 3 == 3:
        size = struct.unpack("<I", head[4:8])[0]
        start = rva + 12
    else:
        raise ValueError(f"unsupported method header {head[0]:#x}")
    return pe.get_data(start, size)


def _find_cctor(pe):
    for ti, row in enumerate(pe.net.mdtables.TypeDef, 1):
        if str(row.TypeName) != "CTTHXCommonPropKeys":
            continue
        for idx in row.MethodList:
            method = idx.row
            if str(method.Name) == ".cctor":
                return ti, idx.row_index, method
    raise ValueError("CTTHXCommonPropKeys..cctor not found")


def extract(assembly):
    import dnfile

    pe = dnfile.dnPE(str(assembly))
    type_row, method_row, method = _find_cctor(pe)
    code = _method_body(pe, method.Rva)
    fields = {i: str(f.Name) for i, f in enumerate(pe.net.mdtables.Field, 1)}

    pairs = {}
    scout_family = []
    i = 0
    last_i4 = None
    last_base = None
    while i < len(code):
        op = code[i]
        i += 1
        if op == 0xFE and i < len(code):
            op2 = code[i]
            i += 1
            if op2 in (0x06, 0x07, 0x15, 0x16, 0x1C):
                i += 4
            elif op2 in (0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E):
                i += 2
            elif op2 in (0x12, 0x19):
                i += 1
            continue
        if op in _EIGHT:
            i += 8
            continue
        if op in _FOUR:
            value = struct.unpack("<I", code[i:i + 4])[0]
            i += 4
            if op == 0x7E:
                last_base = fields.get(value & 0xFFFFFF, "")
            elif op == 0x80:
                name = fields.get(value & 0xFFFFFF, "")
                tail = name.rsplit("Gain", 1)[-1]
                if ("GraphicEQ_Gain" in name or "Scout_GEQ_Gain" in name) \
                        and tail.isdigit():
                    pairs[name] = last_i4
                if name.startswith("CTPKEY_Scout_") \
                        and not name.endswith("_Guid") and last_i4 is not None:
                    scout_family.append({
                        "pid": last_i4,
                        "name": name,
                        "base": last_base,
                    })
            continue
        if op in _ONE:
            value = code[i]
            i += 1
            if op == 0x1F:
                last_i4 = value
            continue
        if op == 0x45:
            count = struct.unpack("<I", code[i:i + 4])[0]
            i += 4 + 4 * count
            continue
        if 0x15 <= op <= 0x1E:
            last_i4 = -1 if op == 0x15 else op - 0x16

    graphic = []
    scout = []
    for index in range(10):
        gname = f"CTPKEY_GraphicEQ_Gain{index}"
        sname = f"CTPKEY_Scout_GEQ_Gain{index}"
        if gname not in pairs or pairs[gname] is None:
            raise ValueError(f"missing {gname}")
        if sname not in pairs or pairs[sname] is None:
            raise ValueError(f"missing {sname}")
        graphic.append({"index": index, "pid": pairs[gname],
                        "hz": GRAPHIC_BANDS[index]})
        scout.append({"index": index, "pid": pairs[sname],
                      "hz": SCOUT_BANDS[index]})

    return {
        "source": {
            "assembly": str(assembly),
            "type": "Creative.Platform.Devices.Features.PropKeys"
                    ".CTTHXCommonPropKeys",
            "type_row": type_row,
            "method": ".cctor",
            "method_row": method_row,
            "method_rva": hex(method.Rva),
        },
        "graphic_eq": {
            "pid_range": [graphic[0]["pid"], graphic[-1]["pid"]],
            "bands": graphic,
        },
        "scout_geq": {
            "pid_range": [scout[0]["pid"], scout[-1]["pid"]],
            "bands": scout,
        },
        "scout_family": sorted(scout_family, key=lambda row: row["pid"]),
        "usb_parameter_ids": {
            "status": "unverified",
            "note": "`5a 11` notifications use a device-side id namespace. "
                    "These APO PIDs are not the USB ids; run the online "
                    "single-band capture before writing any USB mapping.",
        },
        "notes": [
            "PIDs are scoped by the property-set GUID; both families use "
            "the same numeric values only inside their own GUID.",
            "Scout family base is CTPKEY_Scout_Guid: enable=0, SVM=1/2/3, "
            "GEQ enable=4, preamp=5, gains=10..19, reverb enable=20.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembly", type=Path, default=DEFAULT_ASSEMBLY)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    if not args.assembly.is_file():
        print(f"assembly not found: {args.assembly}", file=sys.stderr)
        return 2
    result = extract(args.assembly)
    for name in ("graphic_eq", "scout_geq"):
        print(name)
        for band in result[name]["bands"]:
            print(f"  index {band['index']:2d}  pid {band['pid']:2d}  "
                  f"{band['hz']:g} Hz")
    print("scout_family")
    for row in result["scout_family"]:
        print(f"  pid {row['pid']:2d}  {row['name']}")
    if args.json:
        args.json.write_text(
            json.dumps(result, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8")
        print(f"written: {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
