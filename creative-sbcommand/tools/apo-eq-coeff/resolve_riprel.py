"""Disassemble a range and resolve every RIP-relative operand to its constant.

Prints target VA plus float32 / float64 / int32 interpretations of the first
word(s), so the EQ coefficient constants can be identified without manual
RIP arithmetic. Read-only.
"""
import sys
import struct
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP

DLL = r"C:\Windows\System32\KSUSBAPO64.dll"


def main():
    start = int(sys.argv[1], 0)
    length = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x400
    pe = pefile.PE(DLL, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    sections = []
    for s in pe.sections:
        sections.append((base + s.VirtualAddress,
                         base + s.VirtualAddress + max(s.Misc_VirtualSize, s.SizeOfRawData),
                         s.PointerToRawData,
                         s.SizeOfRawData))
    with open(DLL, "rb") as fh:
        data = fh.read()

    def read_va(va, n):
        for sva, eva, raw, rsize in sections:
            if sva <= va < eva:
                off = raw + (va - sva)
                if off + n <= raw + rsize:
                    return data[off:off + n]
                return None
        return None

    # locate code bytes containing start
    code = None
    for sva, eva, raw, rsize in sections:
        if sva <= start < sva + rsize:
            off = raw + (start - sva)
            code = data[off:off + length]
            break
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    last_target = None
    for insn in md.disasm(code, start):
        line = f"{insn.address:#x}: {insn.mnemonic:<10} {insn.op_str}"
        for op in insn.operands:
            if op.type == X86_OP_MEM and op.mem.base == X86_REG_RIP:
                target = insn.address + insn.size + op.mem.disp
                blob = read_va(target, 16)
                if blob is None:
                    line += f"   ; -> {target:#x} (unmapped)"
                    continue
                f32 = struct.unpack("<4f", blob[:16])
                f64 = struct.unpack("<2d", blob[:16])
                i32 = struct.unpack("<4i", blob[:16])
                line += (f"   ; -> {target:#x}  f32={f32[0]:.9g}  f64={f64[0]:.17g}"
                         f"  i32={i32[0]:#x}")
        print(line)


if __name__ == "__main__":
    main()
