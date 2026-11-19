"""Scan KSUSBAPO64.dll .text for RIP-relative references to a target VA.

Read-only. Useful for locating initializers of runtime objects whose vtable
pointer lives in uninitialized .data.
"""
import sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP

DLL = r"C:\Windows\System32\KSUSBAPO64.dll"


def main():
    target = int(sys.argv[1], 0)
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
    hit = 0
    for insn in md.disasm(code, va):
        for op in insn.operands:
            if op.type == X86_OP_MEM and op.mem.base == X86_REG_RIP:
                dest = insn.address + insn.size + op.mem.disp
                if abs(dest - target) <= 0x20:
                    print(f"{insn.address:#x}: {insn.mnemonic} {insn.op_str}  -> {dest:#x}")
                    hit += 1
    print(f"hits: {hit}")


if __name__ == "__main__":
    main()
