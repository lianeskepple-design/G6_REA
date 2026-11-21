"""Static reader for KSUSBAPO64.dll: VA -> file offset, qword/dword/float at a VA.

Read-only: parses the PE headers and maps virtual addresses to file offsets. It never
loads or executes the DLL, so it is safe to run on an untrusted image.
"""

from __future__ import annotations

import struct
import sys

IMAGE_BASE = 0x180000000
DEFAULT_PATH = r"C:\Windows\System32\KSUSBAPO64.dll"


class Image:
    def __init__(self, path: str = DEFAULT_PATH) -> None:
        with open(path, "rb") as handle:
            self.data = handle.read()
        if self.data[:2] != b"MZ":
            raise ValueError("not a PE image")
        pe_offset = struct.unpack_from("<I", self.data, 0x3C)[0]
        if self.data[pe_offset:pe_offset + 4] != b"PE\0\0":
            raise ValueError("missing PE signature")
        coff = pe_offset + 4
        nsections = struct.unpack_from("<H", self.data, coff + 2)[0]
        opt_size = struct.unpack_from("<H", self.data, coff + 16)[0]
        opt = coff + 20
        self.image_base = struct.unpack_from("<Q", self.data, opt + 24)[0]
        section_table = opt + opt_size
        self.sections = []
        for index in range(nsections):
            off = section_table + index * 40
            name = self.data[off:off + 8].rstrip(b"\0").decode("ascii", "replace")
            vsize, vaddr = struct.unpack_from("<II", self.data, off + 8)
            raw_size, raw_ptr = struct.unpack_from("<II", self.data, off + 16)
            self.sections.append((name, vaddr, max(vsize, raw_size), raw_ptr, raw_size))

    def offset_of(self, va: int) -> int:
        rva = va - self.image_base
        for name, vaddr, vsize, raw_ptr, raw_size in self.sections:
            if vaddr <= rva < vaddr + vsize:
                delta = rva - vaddr
                if delta >= raw_size:
                    raise ValueError(
                        f"VA 0x{va:x} is in the uninitialised part of section {name}")
                return raw_ptr + delta
        raise ValueError(f"VA 0x{va:x} is not mapped by any section")

    def qword(self, va: int) -> int:
        return struct.unpack_from("<Q", self.data, self.offset_of(va))[0]

    def dword(self, va: int) -> int:
        return struct.unpack_from("<I", self.data, self.offset_of(va))[0]

    def float32(self, va: int) -> float:
        return struct.unpack_from("<f", self.data, self.offset_of(va))[0]

    def double(self, va: int) -> float:
        return struct.unpack_from("<d", self.data, self.offset_of(va))[0]


def main(argv: list[str]) -> int:
    image = Image()
    for spec in argv[1:]:
        va_text, _, kind = spec.partition(":")
        va = int(va_text, 16)
        kind = kind or "float"
        if kind == "qword":
            print(f"0x{va:x} qword = 0x{image.qword(va):016x}")
        elif kind == "dword":
            value = image.dword(va)
            print(f"0x{va:x} dword = 0x{value:08x} ({value})")
        elif kind == "double":
            print(f"0x{va:x} double = {image.double(va)!r}")
        elif kind == "ptr":
            target = image.qword(va)
            print(f"0x{va:x} ptr -> 0x{target:x}")
        elif kind == "vtable":
            # Resolve a global that points at an object whose first qword is the vtable.
            obj = image.qword(va)
            vtable = image.qword(obj)
            print(f"0x{va:x} global -> obj 0x{obj:x} -> vtable 0x{vtable:x}")
            if vtable and vtable < 0x1000:
                vtable = obj
                print(f"  (reading the object itself as the vtable: 0x{vtable:x})")
            for slot in (0x178, 0x180, 0x1F8, 0x2A8, 0x2B8):
                target = image.qword(vtable + slot)
                print(f"  +0x{slot:x} = 0x{target:x}")
        else:
            print(f"0x{va:x} float = {image.float32(va)!r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
