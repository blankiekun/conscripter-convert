// Applies a names file to the program: one line per address,
//   <address>\t<name>[\t<comment>]
// '#' starts a comment line. A name at a function's entry renames it;
// at code that has no function yet, it creates one; elsewhere (data) it
// sets a label. The comment becomes the function's plate comment (it
// appears above the function in the decompiled C) or the data's
// end-of-line comment. Run by ghidra_export.sh; argument: the file.
//@category Symbol

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
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
}
