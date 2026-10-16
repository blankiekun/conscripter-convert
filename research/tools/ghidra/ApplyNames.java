// Applies a names file to the program: one line per address,
//   <address>\t<name>[\t<comment>]
// '#' starts a comment line. A name at a function's entry renames it;
// at code that has no function yet, it creates one; elsewhere (data) it
// sets a label. A name in an executable block that doesn't decode as
// code (a table in the Vita's code segment, where its read-only data
// lives) stays a label. The comment becomes the function's plate comment (it
// appears above the function in the decompiled C) or the data's
// end-of-line comment. Run by ghidra_export.sh; argument: the file.
//@category Symbol

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSetView;
import ghidra.program.model.listing.Bookmark;
import ghidra.program.model.listing.BookmarkType;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;

public class ApplyNames extends GhidraScript {

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1 || !Files.exists(Path.of(args[0]))) {
			println("no names file, nothing to apply");
			return;
		}
		int applied = 0;
		int lineNo = 0;
		for (String line : Files.readAllLines(Path.of(args[0]), StandardCharsets.UTF_8)) {
			lineNo++;
			if (line.isBlank() || line.startsWith("#")) {
				continue;
			}
			String[] cols = line.split("\t", 3);
			Address a = cols.length >= 2 ? toAddr(cols[0].trim().replaceFirst("^0x", "")) : null;
			if (a == null) {
				printerr(args[0] + ":" + lineNo + ": expected <address>\\t<name>[\\t<comment>]");
				continue;
			}
			String name = cols[1].trim();
			String comment = cols.length == 3 ? cols[2].trim() : "";
			Function f = getFunctionAt(a);
			MemoryBlock block = getMemoryBlock(a);
			if (f == null && getInstructionAt(a) == null && block != null && block.isExecute()) {
				// Code reached only through a table: not disassembled yet.
				disassembleIfCode(a);
			}
			if (f == null && getInstructionAt(a) != null) {
				f = createFunction(a, null);
			}
			if (f != null) {
				if (!name.isEmpty() && !name.equals(f.getName())) {
					f.setName(name, SourceType.USER_DEFINED);
				}
				if (!comment.isEmpty()) {
					f.setComment(comment);
				}
			}
			else {
				if (!name.isEmpty()) {
					Symbol s = getSymbolAt(a);
					if (s != null && !s.getName().equals(name)) {
						s.setName(name, SourceType.USER_DEFINED);
					}
					else if (s == null) {
						createLabel(a, name, true);
					}
				}
				if (!comment.isEmpty()) {
					setEOLComment(a, comment);
				}
			}
			applied++;
		}
		println("applied " + applied + " names from " + args[0]);
	}

	// Disassembles from a, and undoes it when the flow runs into bytes
	// that aren't instructions (Ghidra marks them with an error bookmark).
	private void disassembleIfCode(Address a) throws Exception {
		DisassembleCommand cmd = new DisassembleCommand(a, null, true);
		cmd.applyTo(currentProgram, monitor);
		AddressSetView done = cmd.getDisassembledAddressSet();
		if (done == null || done.isEmpty()) {
			return;
		}
		Address end = done.getMaxAddress().add(16);
		List<Bookmark> bad = new ArrayList<>();
		Iterator<Bookmark> it = currentProgram.getBookmarkManager().getBookmarksIterator(done.getMinAddress(), true);
		while (it.hasNext()) {
			Bookmark b = it.next();
			if (b.getAddress().compareTo(end) > 0) {
				break;
			}
			if (b.getTypeString().equals(BookmarkType.ERROR)) {
				bad.add(b);
			}
		}
		if (!bad.isEmpty()) {
			bad.forEach(currentProgram.getBookmarkManager()::removeBookmark);
			clearListing(done);
			printerr(a + ": not code, kept as a label");
		}
	}
}
