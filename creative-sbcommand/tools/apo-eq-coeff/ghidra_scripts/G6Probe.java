// Probe helper for the G6 APO work.
//   G6Probe scan 0xf18 0xf1c 0xf20 0xf24   -> list instructions touching those struct displacements
//   G6Probe dump 0x18008daf0 ...           -> decompile the listed functions
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
        String mode = "scan";
        for (String a : args) {
            if (a.equalsIgnoreCase("scan") || a.equalsIgnoreCase("dump")
                    || a.equalsIgnoreCase("vtable") || a.equalsIgnoreCase("sym")
                    || a.equalsIgnoreCase("disasm")) {
                mode = a.toLowerCase();
            } else if (mode.equals("scan")) {
                disps.add(a);
            } else if (mode.equals("vtable")) {
                vtables.add(a);
            } else if (mode.equals("sym")) {
                symbols.add(a);
            } else if (mode.equals("disasm")) {
                disasmSpecs.add(a);
            } else {
                funcs.add(a);
            }
        }
        for (String d : disps) {
            scanDisplacement(d);
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
     * Print a named symbol's address and every qword slot that resolves to code.
     */
    private void dumpSymbolSlots(String name) throws Exception {
        ghidra.program.model.symbol.SymbolIterator it =
                currentProgram.getSymbolTable().getSymbols(name);
        if (!it.hasNext()) {
            // Fall back to a substring search over the whole symbol table.
            ghidra.program.model.symbol.SymbolTable table = currentProgram.getSymbolTable();
            java.util.List<ghidra.program.model.symbol.Symbol> matches = new ArrayList<>();
            for (ghidra.program.model.symbol.Symbol candidate : table.getAllSymbols(true)) {
                if (candidate.getName().contains(name)) {
                    matches.add(candidate);
                }
            }
            if (matches.isEmpty()) {
                println("symbol not found: " + name);
                return;
            }
            for (ghidra.program.model.symbol.Symbol match : matches) {
                println("match: " + match.getName() + " @ " + match.getAddress());
            }
            return;
        }
        ghidra.program.model.symbol.Symbol symbol = it.next();
        Address base = symbol.getAddress();
        println("##### SYMBOL " + symbol.getName() + " @ " + base + " #####");
        for (long slot = 0; slot < 0x400; slot += 8) {
            long target = currentProgram.getMemory().getLong(base.add(slot));
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

    private void scanDisplacement(String hex) {
        long target;

        try {
            target = Long.parseLong(hex.toLowerCase().replace("0x", ""), 16);
        } catch (NumberFormatException e) {
            println("bad displacement: " + hex);
            return;
        }
        String needle = Long.toHexString(target);
        Pattern pattern = Pattern.compile("(?i)(^|[^0-9a-f])0*" + needle + "(?![0-9a-f])");

        println("##### SCAN displacement 0x" + needle + " #####");
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        int hits = 0;
        Set<String> seen = new LinkedHashSet<>();
        while (it.hasNext() && hits < MAX_HITS_PER_DISP) {
            if (monitor.isCancelled()) {
                break;
            }
            Instruction ins = it.next();
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
