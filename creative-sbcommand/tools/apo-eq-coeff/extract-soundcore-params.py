#!/usr/bin/env python3
"""Dump the SoundCore parameter enums used by Sound Blaster Command.

These are per-feature parameter offsets in Creative.Platform.Devices.dll
(CTSoundCore._etParam*), not the APO property PIDs and not the USB
`5a 11` notification ids. They are kept as evidence for the eventual
feature-scoped mapping of the G6 hardware capture.

Requires dnfile (tested with 0.18.0).
"""
import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_ASSEMBLY = Path(
    r"C:\Program Files (x86)\Creative\Sound Blaster Command"
    r"\Platform\Creative.Platform.Devices.dll"
)

WANTED = [
    "_etParamCMSS3D",
    "_etParamCrystalizer",
    "_etParamCustomEQ",
    "_etParamDialogPlus",
    "_etParamEfxMasterControl",
    "_etParamGraphicEQ",
    "_etParamSimpleSurround",
    "_etParamSmartVolume",
    "_etParamSurroundType",
    "_etParamXBass",
]


def _field_constants(pe):
    table = pe.net.mdtables
    consts = {}
    constant_table = getattr(table, "Constant", None)
    if not constant_table:
        return {}
    for constant in constant_table:
        parent = constant.Parent
        if parent is None or parent.table is None:
            continue
        raw = getattr(constant.Value, "value", None)
        if raw is None:
            continue
        try:
            consts[(parent.table.name, parent.row_index)] = int.from_bytes(
                raw, "little", signed=True)
        except (TypeError, ValueError):
            continue
    return consts


def extract(assembly):
    import dnfile

    pe = dnfile.dnPE(str(assembly))
    table = pe.net.mdtables
    consts = _field_constants(pe)
    result = {}
    for type_def in table.TypeDef:
        name = str(type_def.TypeName)
        if name not in WANTED:
            continue
        ext = type_def.Extends
        if ext is None or ext.table is None \
                or str(getattr(ext.row, "TypeName", "")) != "Enum":
            continue
        members = []
        for idx in type_def.FieldList:
            value = consts.get(("Field", idx.row_index))
            if value is None:
                continue
            members.append({"name": str(idx.row.Name), "value": value})
        result[name] = members
    missing = [name for name in WANTED if name not in result]
    if missing:
        raise ValueError(f"missing enums: {', '.join(missing)}")
    return {
        "source": {
            "assembly": str(assembly),
            "type": "CTSoundCore parameter enums",
        },
        "enums": result,
        "usb_parameter_ids": {
            "status": "unverified",
            "note": "These are per-feature SoundCore offsets. The `5a 11` "
                    "id namespace is device-side and still requires the "
                    "online single-band capture.",
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembly", type=Path, default=DEFAULT_ASSEMBLY)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    if not args.assembly.is_file():
        print(f"assembly not found: {args.assembly}", file=sys.stderr)
        return 2
    data = extract(args.assembly)
    for name, members in data["enums"].items():
        print(name)
        for member in members:
            print(f"  {member['value']:4d} {member['name']}")
    if args.json:
        args.json.write_text(
            json.dumps(data, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8")
        print(f"written: {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
