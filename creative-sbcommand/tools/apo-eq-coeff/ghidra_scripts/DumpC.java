// Dump the decompiled C for the addresses passed as script arguments.
// @category REA
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.util.task.ConsoleTaskMonitor;

public class DumpC extends GhidraScript {
    @Override
    public void run() throws Exception {
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        String[] args = getScriptArgs();
        for (String argument : args) {
            Address address = currentProgram.getAddressFactory()
                    .getDefaultAddressSpace().getAddress(argument);
            Function function = getFunctionAt(address);
            if (function == null) {
                function = getFunctionContaining(address);
            }
            println("===== " + argument + " =====");
            if (function == null) {
                println("no function at or containing the address");
                continue;
            }
            println("FUNCTION " + function.getName() + " @ " + function.getEntryPoint());
            DecompileResults results = decompiler.decompileFunction(
                    function, 90, new ConsoleTaskMonitor());
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
