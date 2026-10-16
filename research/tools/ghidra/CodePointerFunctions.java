// Creates functions that auto-analysis leaves out of a stripped AArch64
// binary (a Switch NSO through nso2elf.py): targets of BL calls and tail
// calls (B out of the function) it never made functions, even inside
// another function's body, and code pointers stored as 64-bit data (vtables,
// handler tables). Without it, calls show as `func_0x...` and the
// decompile of the callee is missing.
//
// Each pass: every call's and tail call's target, and every 8-aligned
// 64-bit word in a non-executable initialized block that points at a
// 4-aligned address in an executable block, outside every function, is
// disassembled and made a function. Repeats until a pass finds none.
// Run as a post-script on the first import, before the export:
//   ghidra_export.sh <bin> <out> -- -postScript CodePointerFunctions.java
//@category Analysis.AARCH64

import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.mem.MemoryBlock;

public class CodePointerFunctions extends GhidraScript {

	private boolean isCode(Memory mem, Address a) {
		MemoryBlock b = mem.getBlock(a);
		return b != null && b.isExecute() && a.getOffset() % 4 == 0;
	}

	@Override
	public void run() throws Exception {
		Memory mem = currentProgram.getMemory();
		Listing listing = currentProgram.getListing();
		int total = 0;
		for (int pass = 1; pass <= 10; pass++) {
			Set<Address> targets = new LinkedHashSet<>();
			Set<Address> called = new LinkedHashSet<>();
			for (Instruction ins : listing.getInstructions(true)) {
				monitor.checkCancelled();
				// Calls, and tail calls: unconditional branches out of their function.
				boolean tail = ins.getFlowType().isJump() && !ins.getFlowType().isConditional();
				if (!ins.getFlowType().isCall() && !tail) {
					continue;
				}
				var here = listing.getFunctionContaining(ins.getAddress());
				for (Address t : ins.getFlows()) {
					if (tail && (here == null || here.getBody().contains(t))) {
						continue;
					}
					if (isCode(mem, t) && listing.getFunctionAt(t) == null) {
						targets.add(t);
						called.add(t);
					}
				}
			}
			for (MemoryBlock block : mem.getBlocks()) {
				if (!block.isInitialized() || block.isExecute()) {
					continue;
				}
				Address start = block.getStart();
				long offset = (8 - start.getOffset() % 8) % 8;
				for (; offset + 8 <= block.getSize(); offset += 8) {
					monitor.checkCancelled();
					Address at = start.add(offset);
					long v;
					try {
						v = mem.getLong(at);
					}
					catch (MemoryAccessException e) {
						continue;
					}
					Address t;
					try {
						t = start.getNewAddress(v);
					}
					catch (Exception e) {
						continue;
					}
					if (isCode(mem, t) && listing.getFunctionContaining(t) == null) {
						targets.add(t);
					}
				}
			}
			int created = 0;
			for (Address t : targets) {
				// A data pointer inside a known function is a jump-table label, not an
				// entry; a call target there means that function's body ran too far.
				if (!called.contains(t) && listing.getFunctionContaining(t) != null) {
					continue;
				}
				if (listing.getInstructionAt(t) == null) {
					new DisassembleCommand(t, null, true).applyTo(currentProgram, monitor);
				}
				if (listing.getInstructionAt(t) != null && listing.getFunctionAt(t) == null
						&& new CreateFunctionCmd(t).applyTo(currentProgram, monitor)) {
					created++;
				}
			}
			println("pass " + pass + ": " + created + " functions created");
			total += created;
			if (created == 0) {
				break;
			}
		}
		println("created " + total + " functions at call targets and code pointers");
	}
}
