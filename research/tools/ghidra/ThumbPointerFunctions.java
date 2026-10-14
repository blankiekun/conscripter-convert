// Creates functions at the Thumb code pointers a binary stores as data
// (C++ vtables, handler tables), which auto-analysis doesn't follow: a
// virtual method called only through its vtable is otherwise left as
// undefined bytes. For ARM binaries (a PS Vita eboot, for example).
//
// Scans every initialized block for aligned 32-bit words, not inside an
// instruction, whose value is odd and points into an executable block
// outside every function; disassembles there in Thumb mode if needed and
// creates a function. Repeats until a pass finds none (new code
// can hold more pointers). Run as a post-script before the export:
//   ghidra_export.sh <bin> <out> -- -postScript ThumbPointerFunctions.java
//@category Analysis.ARM


import ghidra.app.cmd.disassemble.ArmDisassembleCommand;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.mem.MemoryBlock;

public class ThumbPointerFunctions extends GhidraScript {

	@Override
	public void run() throws Exception {
		Memory mem = currentProgram.getMemory();
		Listing listing = currentProgram.getListing();
		int total = 0;
		for (int pass = 1; pass <= 5; pass++) {
			int created = 0;
			for (MemoryBlock block : mem.getBlocks()) {
				if (!block.isInitialized()) {
					continue;
				}
				Address start = block.getStart();
				long offset = (4 - start.getOffset() % 4) % 4;
				for (; offset + 4 <= block.getSize(); offset += 4) {
					monitor.checkCancelled();
					Address at = start.add(offset);
					if (listing.getInstructionContaining(at) != null) {
						continue;
					}
					long value;
					try {
						value = mem.getInt(at) & 0xffffffffL;
					}
					catch (MemoryAccessException e) {
						continue;
					}
					if ((value & 1) == 0) {
						continue;
					}
					Address target = toAddr(value - 1);
					MemoryBlock to = mem.getBlock(target);
					if (to == null || !to.isExecute() || !to.isInitialized()) {
						continue;
					}
					if (getFunctionContaining(target) != null || listing.getDefinedDataContaining(target) != null) {
						continue;
					}
					// Code that analysis disassembled but put in no function,
					// or bytes it never reached.
					if (listing.getInstructionAt(target) == null) {
						if (listing.getInstructionContaining(target) != null) {
							continue;
						}
						ArmDisassembleCommand cmd = new ArmDisassembleCommand(target, null, true);
						if (!cmd.applyTo(currentProgram, monitor) || listing.getInstructionAt(target) == null) {
							continue;
						}
					}
					if (createFunction(target, null) != null) {
						created++;
					}
				}
			}
			total += created;
			println("pass " + pass + ": " + created + " functions at Thumb pointers");
			if (created == 0) {
				break;
			}
		}
		println("created " + total + " functions at Thumb pointers");
	}
}
