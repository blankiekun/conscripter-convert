// Writes one pseudo-C file per function, plus functions.tsv and
// strings.tsv indexes, for reading and grepping outside Ghidra.
// Run by ghidra_export.sh; argument: the output directory.
//
// Each file is <outdir>/decomp/<address>_<name>.c and starts with the
// function's callers, callees and the strings it uses. Lines that use a
// global holding a Shift-JIS (cp932) string, or a constant pointing at
// one, get the decoded string as a trailing comment.
//@category Export

import java.io.File;
import java.io.PrintWriter;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.Charset;
import java.nio.charset.CharsetDecoder;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;

public class ExportDecomp extends GhidraScript {

	private static final Pattern ADDR_IN_NAME = Pattern.compile("\\b\\w*?_([0-9a-fA-F]{8,16})\\b");
	private static final Pattern HEX_CONST = Pattern.compile("\\b0x([0-9a-fA-F]{6,16})\\b");

	private final Map<Address, String> strCache = new HashMap<>();
	private final Map<Address, Set<String>> strUsers = new TreeMap<>();
	private CharsetDecoder sjis;
	private Memory mem;

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			printerr("usage: ExportDecomp <outdir>");
			return;
		}
		File out = new File(args[0]);
		File dir = new File(out, "decomp");
		dir.mkdirs();
		for (File f : dir.listFiles()) {
			if (f.getName().endsWith(".c")) {
				f.delete();
			}
		}
		mem = currentProgram.getMemory();
		sjis = Charset.forName("MS932").newDecoder()
				.onMalformedInput(CodingErrorAction.REPORT)
				.onUnmappableCharacter(CodingErrorAction.REPORT);

		DecompInterface ifc = new DecompInterface();
		DecompileOptions opts = new DecompileOptions();
		opts.grabFromProgram(currentProgram);
		ifc.setOptions(opts);
		ifc.openProgram(currentProgram);

		List<Function> funcs = new ArrayList<>();
		for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
			if (!f.isExternal() && !f.isThunk()) {
				funcs.add(f);
			}
		}
		monitor.initialize(funcs.size());
		PrintWriter index = new PrintWriter(new File(out, "functions.tsv"), StandardCharsets.UTF_8);
		index.println("address\tname\tsize\tcallers\tcallees\tstrings");
		int failed = 0;
		for (Function f : funcs) {
			monitor.checkCancelled();
			monitor.incrementProgress(1);
			String addr = f.getEntryPoint().toString();
			Set<Function> callers = f.getCallingFunctions(monitor);
			Set<Function> callees = f.getCalledFunctions(monitor);
			Map<Address, String> strs = stringsUsedBy(f);

			StringBuilder sb = new StringBuilder();
			sb.append("// ").append(addr).append(' ').append(f.getName(true))
					.append("  size ").append(f.getBody().getNumAddresses()).append('\n');
			sb.append("// called by: ").append(names(callers)).append('\n');
			sb.append("// calls: ").append(names(callees)).append('\n');
			for (Map.Entry<Address, String> e : strs.entrySet()) {
				sb.append("// string ").append(e.getKey()).append(": ").append(quote(e.getValue())).append('\n');
			}
			sb.append('\n');
			DecompileResults r = ifc.decompileFunction(f, 120, monitor);
			if (r != null && r.decompileCompleted()) {
				for (String line : r.getDecompiledFunction().getC().split("\n", -1)) {
					sb.append(annotate(line)).append('\n');
				}
			}
			else {
				failed++;
				sb.append("// decompile failed: ").append(r == null ? "?" : r.getErrorMessage()).append('\n');
			}
			String name = f.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
			if (name.length() > 80) {
				name = name.substring(0, 80);
			}
			try (PrintWriter w = new PrintWriter(new File(dir, addr + "_" + name + ".c"), StandardCharsets.UTF_8)) {
				w.print(sb);
			}

			List<String> sl = new ArrayList<>();
			for (String s : strs.values()) {
				sl.add(quote(s));
			}
			index.println(addr + "\t" + f.getName(true) + "\t" + f.getBody().getNumAddresses() + "\t" +
				callers.size() + "\t" + callees.size() + "\t" + String.join(" ", sl).replace('\t', ' '));
		}
		index.close();
		ifc.dispose();

		try (PrintWriter w = new PrintWriter(new File(out, "strings.tsv"), StandardCharsets.UTF_8)) {
			w.println("address\tstring\tused by");
			for (Map.Entry<Address, Set<String>> e : strUsers.entrySet()) {
				w.println(e.getKey() + "\t" + quote(strCache.get(e.getKey())) + "\t" + String.join(" ", e.getValue()));
			}
		}
		println("exported " + funcs.size() + " functions (" + failed + " failed to decompile) to " + dir);
	}

	/** Strings an instruction of f references, directly or through one pointer. */
	private Map<Address, String> stringsUsedBy(Function f) {
		Map<Address, String> found = new TreeMap<>();
		for (Instruction ins : currentProgram.getListing().getInstructions(f.getBody(), true)) {
			List<Address> targets = new ArrayList<>();
			for (Reference ref : ins.getReferencesFrom()) {
				targets.add(ref.getToAddress());
			}
			for (int i = 0; i < ins.getNumOperands(); i++) {
				for (Object o : ins.getOpObjects(i)) {
					if (o instanceof ghidra.program.model.scalar.Scalar sc) {
						Address a = addrOf(sc.getUnsignedValue());
						if (a != null) {
							targets.add(a);
						}
					}
				}
			}
			for (Address a : targets) {
				String s = stringAt(a);
				if (s == null) {
					Address p = pointerAt(a);
					if (p != null) {
						a = p;
						s = stringAt(a);
					}
				}
				if (s != null) {
					found.put(a, s);
					strUsers.computeIfAbsent(a, k -> new TreeSet<>()).add(f.getEntryPoint().toString());
				}
			}
		}
		return found;
	}

	private Address addrOf(long v) {
		try {
			Address a = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(v);
			return mem.contains(a) ? a : null;
		}
		catch (Exception e) {
			return null;
		}
	}

	private Address pointerAt(Address a) {
		MemoryBlock b = mem.getBlock(a);
		if (b == null || !b.isInitialized() || b.isExecute()) {
			return null;
		}
		try {
			int size = currentProgram.getDefaultPointerSize();
			long v = size == 8 ? mem.getLong(a) : mem.getInt(a) & 0xffffffffL;
			return addrOf(v);
		}
		catch (Exception e) {
			return null;
		}
	}

	/** The NUL-terminated cp932 string at a, if it looks like text. */
	private String stringAt(Address a) {
		if (strCache.containsKey(a)) {
			return strCache.get(a);
		}
		String s = null;
		MemoryBlock b = mem.getBlock(a);
		if (b != null && b.isInitialized() && !b.isExecute()) {
			byte[] buf = new byte[512];
			int n;
			try {
				n = mem.getBytes(a, buf);
			}
			catch (Exception e) {
				n = 0;
			}
			int end = 0;
			while (end < n && buf[end] != 0) {
				end++;
			}
			if (end >= 2 && end < n) {
				try {
					String t = sjis.decode(ByteBuffer.wrap(buf, 0, end)).toString();
					if (looksLikeText(t)) {
						s = t;
					}
				}
				catch (CharacterCodingException e) {
					// not text
				}
			}
		}
		strCache.put(a, s);
		return s;
	}

	/**
	 * Random bytes often decode as cp932 too: allow only ASCII and the
	 * usual Japanese ranges (no half-width katakana), and at least 4
	 * characters unless all are Japanese.
	 */
	private static boolean looksLikeText(String t) {
		boolean ascii = false;
		for (int i = 0; i < t.length(); i++) {
			char c = t.charAt(i);
			if (c < 0x80) {
				if (c < 0x20 && c != '\n' && c != '\r' && c != '\t' || c == 0x7f) {
					return false;
				}
				ascii = true;
			}
			else if (!(c >= 0x3000 && c <= 0x30ff || c >= 0x4e00 && c <= 0x9fff ||
				c >= 0xff01 && c <= 0xff5e || c >= 0x2010 && c <= 0x27bf)) {
				return false;
			}
		}
		return t.length() >= (ascii ? 4 : 2);
	}

	private String annotate(String line) {
		List<String> notes = new ArrayList<>();
		for (Pattern p : new Pattern[] { ADDR_IN_NAME, HEX_CONST }) {
			Matcher m = p.matcher(line);
			while (m.find()) {
				Address a = addrOf(Long.parseUnsignedLong(m.group(1), 16));
				if (a == null) {
					continue;
				}
				String s = stringAt(a);
				if (s == null) {
					Address q = pointerAt(a);
					s = q == null ? null : stringAt(q);
				}
				if (s != null) {
					notes.add(quote(s));
				}
			}
		}
		return notes.isEmpty() ? line : line + "  /* " + String.join(", ", notes) + " */";
	}

	private static String names(Set<Function> fs) {
		List<String> l = new ArrayList<>();
		for (Function f : fs) {
			l.add(f.getEntryPoint() + " " + f.getName(true));
		}
		l.sort(null);
		return l.isEmpty() ? "-" : String.join(", ", l);
	}

	private static String quote(String s) {
		return "\"" + s.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
				.replace("\r", "\\r").replace("\t", "\\t") + "\"";
	}
}
