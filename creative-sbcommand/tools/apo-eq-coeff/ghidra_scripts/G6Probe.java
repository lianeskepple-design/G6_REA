// Probe helper for the G6 APO work.
//   G6Probe scan 0xf18 0xf1c 0xf20 0xf24   -> list instructions touching those struct displacements
//   G6Probe dump 0x18008daf0 ...           -> decompile the listed functions
//   G6Probe xref 0x180096fe0 ...           -> list every reference to the listed addresses
//   G6Probe data 0x180198f28:0x20 ...      -> hex + dword dump of a data address
//   G6Probe disasm 0x180096fe0:32 ...      -> raw disassembly of an address range
//   G6Probe sym CPlugAGCCompressor ...     -> list symbol/namespace matches
//   G6Probe vtable 0x1801f1e80:0x178 ...   -> resolve a global's vtable slots
// Mixing both forms in one argument list is supported; "scan"/"dump" switch mode.
// @category REA
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressFormatException;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.util.task.ConsoleTaskMonitor;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.regex.Pattern;

public class G6Probe extends GhidraScript {

    private static final int MAX_HITS_PER_DISP = 250;

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        List<String> disps = new ArrayList<>();
        List<String> funcs = new ArrayList<>();
        List<String> vtables = new ArrayList<>();
        List<String> symbols = new ArrayList<>();
        List<String> disasmSpecs = new ArrayList<>();
        List<String> xrefs = new ArrayList<>();
        List<String> dataSpecs = new ArrayList<>();
        String mode = "scan";
        for (String a : args) {
            if (a.equalsIgnoreCase("scan") || a.equalsIgnoreCase("dump")
                    || a.equalsIgnoreCase("vtable") || a.equalsIgnoreCase("sym")
                    || a.equalsIgnoreCase("disasm") || a.equalsIgnoreCase("xref")
                    || a.equalsIgnoreCase("data")) {
                mode = a.toLowerCase();
            } else if (mode.equals("scan")) {
                disps.add(a);
            } else if (mode.equals("vtable")) {
                vtables.add(a);
            } else if (mode.equals("sym")) {
                symbols.add(a);
            } else if (mode.equals("disasm")) {
                disasmSpecs.add(a);
            } else if (mode.equals("xref")) {
                xrefs.add(a);
            } else if (mode.equals("data")) {
                dataSpecs.add(a);
            } else {
                funcs.add(a);
            }
        }
        for (String d : disps) {
            scanDisplacement(d);
        }
        for (String x : xrefs) {
            dumpXrefs(x);
        }
        for (String d : dataSpecs) {
            dumpData(d);
        }
        for (String v : vtables) {
            resolveVtable(v);
        }
        for (String s : symbols) {
            dumpSymbolSlots(s);
        }
        for (String d : disasmSpecs) {
            dumpDisassembly(d);
        }
        if (!funcs.isEmpty()) {
            dumpFunctions(funcs);
        }
    }

    /**
     * Print every reference to an address: source instruction, owning function and the
     * reference type. Data references are included so vtable stores are visible.
     */
    private void dumpXrefs(String spec) throws Exception {
        Address target = toAddr(spec);
        println("##### XREF " + spec + " #####");
        int hits = 0;
        for (ghidra.program.model.symbol.Reference ref
                : currentProgram.getReferenceManager().getReferencesTo(target)) {
            Address from = ref.getFromAddress();
            Function f = getFunctionContaining(from);
            String fn = (f == null) ? "<no function>"
                    : f.getName() + "@" + f.getEntryPoint();
            Instruction ins = currentProgram.getListing().getInstructionAt(from);
            String text = (ins == null) ? "<data>" : ins.toString();
            println(from + "  " + text + "   | " + ref.getReferenceType()
                    + "  |  " + fn);
            hits++;
        }
        println("##### " + hits + " reference(s) to " + spec + " #####");
    }

    /**
     * Print a named symbol's address and every qword slot that resolves to code.
     */
    private void dumpSymbolSlots(String name) throws Exception {
        ghidra.program.model.symbol.SymbolIterator it =
                currentProgram.getSymbolTable().getSymbols(name);
        boolean exact = false;
        while (it.hasNext()) {
            ghidra.program.model.symbol.Symbol candidate = it.next();
            exact = true;
            if (candidate.getAddress() == null
                    || candidate.getAddress().equals(ghidra.program.model.address.Address.NO_ADDRESS)) {
                println("skip (no address): " + candidate.getName());
                continue;
            }
            describeSymbolSlots(candidate);
        }
        // Always add the substring listing: namespaces have no address but their
        // children (methods, vtables) are what the probe is usually after.
        ghidra.program.model.symbol.SymbolTable table = currentProgram.getSymbolTable();
        java.util.List<ghidra.program.model.symbol.Symbol> matches = new ArrayList<>();
        for (ghidra.program.model.symbol.Symbol candidate : table.getAllSymbols(true)) {
            String qualified = candidate.getName(true);
            if (qualified.contains(name)
                    && candidate.getAddress() != null
                    && !candidate.getAddress().equals(
                            ghidra.program.model.address.Address.NO_ADDRESS)) {
                matches.add(candidate);
            }
        }
        if (matches.isEmpty()) {
            if (!exact) {
                println("symbol not found: " + name);
            }
            return;
        }
        int shown = 0;
        for (ghidra.program.model.symbol.Symbol match : matches) {
            if (shown++ >= 80) {
                println("... truncated at 80 matches");
                break;
            }
            println("match: " + match.getName(true) + " @ " + match.getAddress());
        }
    }

    private void describeSymbolSlots(ghidra.program.model.symbol.Symbol symbol) throws Exception {
        Address base = symbol.getAddress();
        println("##### SYMBOL " + symbol.getName() + " @ " + base + " #####");
        for (long slot = 0; slot < 0x400; slot += 8) {
            long target;
            try {
                target = currentProgram.getMemory().getLong(base.add(slot));
            } catch (Exception e) {
                continue;
            }
            if (target < 0x180001000L || target > 0x180800000L) {
                continue;
            }
            Address addr = toAddr(target);
            Function fn = getFunctionAt(addr);
            if (fn == null) {
                fn = getFunctionContaining(addr);
            }
            if (fn == null) {
                continue;
            }
            println("  +0x" + Long.toHexString(slot) + " = " + addr
                    + "   |  " + fn.getName());
        }
    }

    /**
     * Resolve a global that holds a vtable pointer: prints the vtable address and the
     * target of each requested slot (given as "global:0x178,0x180,...").
     */
    private void resolveVtable(String spec) throws Exception {
        String[] parts = spec.split(":");
        if (parts.length != 2) {
            println("vtable spec must look like 0x1801f1e80:0x178,0x1f8");
            return;
        }
        Address global = toAddr(parts[0]);
        long obj = currentProgram.getMemory().getLong(global);
        long vt = currentProgram.getMemory().getLong(toAddr(obj));
        if (vt == 0) {
            vt = obj;
        }
        println("##### VTABLE " + parts[0] + " -> obj 0x" + Long.toHexString(obj)
                + " -> vtable 0x" + Long.toHexString(vt) + " #####");
        for (String slotText : parts[1].split(",")) {
            long slot = Long.decode(slotText);
            Address entry = toAddr(vt + slot);
            long target = currentProgram.getMemory().getLong(entry);
            Address fnAddr = toAddr(target);
            Function fn = getFunctionAt(fnAddr);
            if (fn == null) {
                fn = getFunctionContaining(fnAddr);
            }
            println("  +0x" + Long.toHexString(slot) + " = 0x" + Long.toHexString(target)
                    + "   |  " + (fn == null ? "<no function>" : fn.getName()));
        }
    }

    /**
     * Print a data address as hex bytes plus a dword view. Spec: "address:length",
     * for example "0x180198f28:0x20". Length defaults to 64 bytes.
     */
    private void dumpData(String spec) throws Exception {
        String[] parts = spec.split(":");
        Address address = toAddr(parts[0]);
        long length = parts.length > 1 ? Long.decode(parts[1]) : 64;
        println("##### DATA " + parts[0] + " (" + length + " bytes) #####");
        for (long off = 0; off < length; off += 16) {
            StringBuilder line = new StringBuilder(
                    String.format("%08x  ", address.getOffset() + off));
            for (int i = 0; i < 16 && off + i < length; i++) {
                int b = currentProgram.getMemory().getByte(address.add(off + i)) & 0xff;
                line.append(String.format("%02x ", b));
            }
            println(line.toString().trim());
        }
        StringBuilder dwords = new StringBuilder("dwords:");
        for (long off = 0; off + 4 <= length; off += 4) {
            dwords.append(String.format(" [%x]=%08x",
                    off, currentProgram.getMemory().getInt(address.add(off))));
        }
        println(dwords.toString());
    }

    /**
     * Print instructions for "address:count".
     */
    private void dumpDisassembly(String spec) throws Exception {
        String[] parts = spec.split(":");
        Address address = toAddr(parts[0]);
        int count = parts.length > 1 ? Integer.parseInt(parts[1]) : 64;
        println("##### DISASM " + parts[0] + " (" + count + ") #####");
        ghidra.program.model.listing.InstructionIterator it =
                currentProgram.getListing().getInstructions(address, true);
        for (int i = 0; i < count && it.hasNext(); ++i) {
            ghidra.program.model.listing.Instruction ins = it.next();
            println(ins.getAddress() + "  " + ins);
        }
    }

    private void scanDisplacement(String spec) {
        long target;
        String hex = spec;
        String mnemonic = null;
        int colon = spec.indexOf(':');
        if (colon > 0) {
            hex = spec.substring(0, colon);
            mnemonic = spec.substring(colon + 1).toLowerCase();
        }

        try {
            target = Long.parseLong(hex.toLowerCase().replace("0x", ""), 16);
        } catch (NumberFormatException e) {
            println("bad displacement: " + hex);
            return;
        }
        String needle = Long.toHexString(target);
        // Require the 0x prefix so registers such as R14 are not treated as 0x14.
        Pattern pattern = Pattern.compile("(?i)0x0*" + needle + "(?![0-9a-f])");

        println("##### SCAN displacement 0x" + needle
                + (mnemonic == null ? "" : " (mnemonic " + mnemonic + ")") + " #####");
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        int hits = 0;
        Set<String> seen = new LinkedHashSet<>();
        while (it.hasNext() && hits < MAX_HITS_PER_DISP) {
            if (monitor.isCancelled()) {
                break;
            }
            Instruction ins = it.next();
            if (mnemonic != null
                    && !ins.getMnemonicString().toLowerCase().startsWith(mnemonic)) {
                continue;
            }
            boolean match = false;
            for (int i = 0; i < ins.getNumOperands() && !match; i++) {
                String rep = ins.getDefaultOperandRepresentation(i);
                if (rep != null && pattern.matcher(rep).find()) {
                    match = true;
                }
            }
            if (!match) {
                continue;
            }
            Function f = getFunctionContaining(ins.getAddress());
            String fn = (f == null)
                    ? "<no function>"
                    : f.getName() + "@" + f.getEntryPoint();
            String line = ins.getAddress() + "  " + ins + "   |  " + fn;
            if (seen.add(line)) {
                println(line);
                hits++;
            }
        }
        if (hits >= MAX_HITS_PER_DISP) {
            println("... truncated at " + MAX_HITS_PER_DISP + " hits");
        }
        println("##### " + hits + " hit(s) for 0x" + needle + " #####");
    }

    private void dumpFunctions(List<String> addrs) throws AddressFormatException {
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        for (String a : addrs) {
            Address address = currentProgram.getAddressFactory()
                    .getDefaultAddressSpace().getAddress(a);
            Function function = getFunctionAt(address);
            if (function == null) {
                function = getFunctionContaining(address);
            }
            println("===== " + a + " =====");
            if (function == null) {
                println("no function at or containing the address");
                continue;
            }
            println("FUNCTION " + function.getName() + " @ " + function.getEntryPoint());
            DecompileResults results = decompiler.decompileFunction(
                    function, 120, new ConsoleTaskMonitor());
            if (results == null || !results.decompileCompleted()) {
                println("decompile failed: "
                        + (results == null ? "null" : results.getErrorMessage()));
                continue;
            }
            println(results.getDecompiledFunction().getC());
        }
        decompiler.dispose();
    }
}
