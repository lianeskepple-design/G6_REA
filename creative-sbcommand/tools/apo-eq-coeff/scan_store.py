"""Scan .text for float stores to [reg+disp] at a chosen displacement.

Read-only. Prints instruction address and text for stores whose displacement
matches, to locate band-gain / ratio setters.
"""
import sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP, X86_REG_INVALID

DLL = r"C:\Windows\System32\KSUSBAPO64.dll"


def main():
    disp = int(sys.argv[1], 0)
    pe = pefile.PE(DLL, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    text = None
    for s in pe.sections:
        if s.Name.rstrip(b"\x00") == b".text":
            text = s
            break
    va = base + text.VirtualAddress
    with open(DLL, "rb") as fh:
        fh.seek(text.PointerToRawData)
        code = fh.read(text.SizeOfRawData)
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    count = 0
    for insn in md.disasm(code, va):
        if insn.mnemonic not in ("movss", "mov"):
            continue
        for op in insn.operands:
            if op.type == X86_OP_MEM and op.mem.base != X86_REG_RIP and op.mem.base != X86_REG_INVALID and op.mem.disp == disp:
                print(f"{insn.address:#x}: {insn.mnemonic} {insn.op_str}")
                count += 1
                break
    print(f"hits: {count}")


if __name__ == "__main__":
    main()
