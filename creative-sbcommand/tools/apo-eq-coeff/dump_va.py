"""Dump qwords / floats at virtual addresses in KSUSBAPO64.dll (read-only)."""
import struct
import sys
import pefile

DLL = r"C:\Windows\System32\KSUSBAPO64.dll"


def load():
    pe = pefile.PE(DLL, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    secs = []
    for s in pe.sections:
        secs.append((base + s.VirtualAddress,
                     base + s.VirtualAddress + max(s.Misc_VirtualSize, s.SizeOfRawData),
                     s.PointerToRawData))
    with open(DLL, "rb") as fh:
        return base, secs, fh.read()


def read_va(secs, data, va, n):
    for start, end, raw in secs:
        if start <= va < end and va + n <= end:
            off = raw + (va - start)
            return data[off:off + n]
    raise ValueError(f"VA {va:#x} not mapped")


def main():
    _, secs, data = load()
    for arg in sys.argv[1:]:
        va = int(arg, 0)
        blob = read_va(secs, data, va, 32)
        q = struct.unpack("<4Q", blob)
        f = struct.unpack("<8f", blob)
        print(f"{va:#x}: qwords " + " ".join(f"{x:#x}" for x in q))
        print(f"{va:#x}: floats " + " ".join(f"{x:.9g}" for x in f))


if __name__ == "__main__":
    main()
