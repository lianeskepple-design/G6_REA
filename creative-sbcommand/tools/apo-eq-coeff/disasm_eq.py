"""Linear disassembler for the EQ coefficient functions in KSUSBAPO64.dll.

Read-only: parses the PE, maps VA to file offset, disassembles the requested
address ranges, and prints each instruction. No part of the DLL is executed.
"""
import sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64

DLL = r"C:\Windows\System32\KSUSBAPO64.dll"


def load():
    pe = pefile.PE(DLL, fast_load=True)
    image_base = pe.OPTIONAL_HEADER.ImageBase
    sections = []
    for section in pe.sections:
        va = image_base + section.VirtualAddress
        size = max(section.Misc_VirtualSize, section.SizeOfRawData)
        raw = section.PointerToRawData
        sections.append((va, va + size, raw, section.Name.rstrip(b"\x00").decode("ascii", "replace")))
    with open(DLL, "rb") as fh:
        data = fh.read()
    return image_base, sections, data


def va_to_offset(sections, va):
    for start, end, raw, name in sections:
        if start <= va < end:
            return raw + (va - start), name
    raise ValueError(f"VA {va:#x} is not mapped")


def disasm(start, length, stop_at_ret=True):
    image_base, sections, data = load()
    offset, section = va_to_offset(sections, start)
    code = data[offset:offset + length]
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = False
    for insn in md.disasm(code, start):
        print(f"{insn.address:#x}: {insn.bytes.hex():<28} {insn.mnemonic:<10} {insn.op_str}")
        if stop_at_ret and insn.mnemonic.startswith("ret"):
            break


if __name__ == "__main__":
    start = int(sys.argv[1], 0)
    length = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x200
    disasm(start, length)
