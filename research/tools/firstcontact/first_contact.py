#!/usr/bin/env python3
"""Static first look at a Windows game install, before any wine run.

  first_contact.py <install dir> [-o OUT] [--exe NAME.exe]
                   [--ghidra EXPORT_DIR | --ghidra-run]
  first_contact.py --self-test

Writes OUT/report.md (default out/<dir name>-first-contact): for a family we
already convert (KNOWN), the nearest game's document and the command to start
from; the engine guesses (known file names, archive magic numbers, exe strings), the exe's
bitness and imports sorted into graphics / sound / timing / input / files,
the files by extension with their first bytes, and RTTI class names. With
a Ghidra export of the exe (research/tools/ghidra/ghidra_export.sh: give
its folder with --ghidra, or --ghidra-run makes one in OUT/ghidra, minutes)
it lists candidate functions for the script interpreter's dispatch, the
wait/sleep loop and the frame present, and writes them to OUT/hooks.tsv
as `count` breakpoints for fc.sh --hooks: the run's per-second "hits" then
tell which candidate is which (a dispatch runs per statement, a present
per frame, a wait loop per frame or per wait).
"""
import collections
import os
import re
import subprocess
import sys

try:
    import pefile
except ImportError:  # shortcut: no PE details without pefile (pip install pefile)
    pefile = None

# (engine, kind, pattern): kind file = a file name (regex, whole name, any
# depth), magic = an archive's first bytes, str = text in the exe
SIGNS = [
    ("Xuse (最果てのイマ)", "file", r"Farthest\d*\.exe"), ("Xuse", "str", rb"XUSE"),
    ("Nitroplus NSS", "file", r"nss\.npa"), ("Nitroplus NSS", "file", r".*\.npa"),
    ("Nitroplus Diesel (NPK3)", "magic", b"NPK3"), ("Nitroplus Diesel (NPK2)", "magic", b"NPK2"),
    ("Nitroplus Diesel", "str", rb"[Ss]quirrel"),
    ("KiriKiri / KAG", "magic", b"XP3\r\n \n\x1a\x8b\x67\x01"), ("KiriKiri / KAG", "str", rb"TVP\(KIRIKIRI\)"),
    ("KiriKiri Z", "str", rb"krkr"), ("NScripter", "file", r"(nscript\.dat|0\.txt|00\.txt|arc\d*\.nsa|.*\.sar)"),
    ("SiglusEngine", "file", r"(Scene\.pck|Gameexe\.dat|SiglusEngine.*\.exe)"),
    ("RealLive", "file", r"(SEEN\.TXT|Gameexe\.ini|RealLive.*\.exe)"),
    ("CatSystem2", "file", r".*\.int"), ("CatSystem2", "magic", b"KIF\0"), ("CatSystem2", "str", rb"CatScene"),
    ("BGI / Ethornell", "magic", b"BURIKO ARC20"), ("BGI / Ethornell", "magic", b"PackFile    "),
    ("Artemis", "magic", b"pf8"), ("Artemis", "magic", b"pf6"), ("YU-RIS", "magic", b"YPF\0"),
    ("Malie", "magic", b"LIBP"), ("QLiE", "str", rb"FilePackVer"), ("AliceSoft System4", "magic", b"AFAH"),
    ("AliceSoft System4", "file", r".*\.ald"), ("Softpal", "file", r".*\.pac"), ("Will / AdvHD", "file", r"AdvHD\.exe"),
    ("Unity", "file", r"UnityPlayer\.dll"), ("Ren'Py", "file", r".*\.rpa"), ("TyranoScript / NW.js", "file", r"(nw\.dll|package\.nw)"),
    ("Majiro", "magic", b"MajiroArcV"), ("Livemaker", "file", r"live\.dll"), ("Musica", "file", r".*\.paz"),
    ("TYPE-MOON (mrgd00)", "magic", b"mrgd00"), ("ONScripter", "str", rb"ONScripter"),
    ("MAGES / 5pb. (CRI)", "magic", b"CPK "), ("MAGES / 5pb. (CRI)", "magic", b"AFS2"),
    ("MAGES / 5pb. (CRI)", "magic", b"@UTF"), ("MAGES / 5pb. (CRI)", "file", r".*\.(cpk|awb|usm)"),
    ("Nintendo Switch RomFS", "magic", b"BNTX"), ("Nintendo Switch RomFS", "magic", b"FSAR"),
    ("Nintendo Switch RomFS", "file", r".*\.(bntx|bfsar)"), ("PS Vita dump", "file", r"(param\.sfo|eboot\.bin)"),
    ("Lua inside", "str", rb"lua_pcall|LuaQ|\x1bLua"), ("Ogg Vorbis", "str", rb"ov_open_callbacks|vorbis"),
    ("libpng", "str", rb"libpng"), ("zlib", "str", rb"inflate \d"),
]

# families we already convert, by the SIGNS name they start with: the reader to try first, the game
# document to read (Status, then History), the modules that carry over, the build command
KNOWN = {
    "Xuse": ("`xarc list <x.arc>`, `cd`, `gaf`, `xsa`", "docs/games/saihate-no-ima.md",
             "crates/conv-engines/src/xuse/", "convert-game <install> -o out/<game>-dry --dry-run"),
    "Nitroplus NSS": ("`npa list <x.npa>`, `nss check <dir>`, `nss refs <dir>`, `nss route <dir>`",
                      "docs/games/django.md (an older NSS game, reused in 15 min), docs/games/muramasa.md",
                      "crates/conv-engines/src/nitroplus/ (convert/django.rs, convert/muramasa.rs: a game's rules)",
                      "convert-nss <install> -o out/<game>-dry --dry-run"),
    "Nitroplus Diesel": ("`python3 -I research/nitroplus/npk.py <x.npk>`, then the Squirrel reader (nitroplus/squirrel.rs)",
                         "docs/games/muramasa-en.md (English Diesel port paired with the NSS original)",
                         "crates/conv-engines/src/nitroplus/ (npk.rs, squirrel.rs, english.rs)",
                         "convert-nss <NSS install> --english <install> -o out/<game>-dry --dry-run"),
    "MAGES / 5pb.": ("`mages list <x.cpk>`, `mages check <game>`", "docs/games/oretsuba.md (PS Vita TaskList)",
                     "crates/conv-engines/src/mages/", "mages convert <game> -o out/<game>-dry --dry-run"),
    "TYPE-MOON": ("`typemoon list <x.mrg>`, `typemoon check <romfs>`, `typemoon script <romfs>`",
                  "docs/games/tsukihime.md (Switch, as in the 魔法使いの夜 remake)",
                  "crates/conv-engines/src/typemoon/", "typemoon convert <romfs> -o out/<game>-dry --dry-run"),
}


def known(hits):
    """The 'Known family' lines: for each family we convert with evidence, best first."""
    ev = collections.defaultdict(set)
    for eng, e in hits.items():
        fam = next((k for k in KNOWN if eng.startswith(k)), None)
        if fam:
            ev[fam] |= e
    L = []
    for fam, e in sorted(ev.items(), key=lambda kv: -len(kv[1])):
        reader, doc, mods, cmd = KNOWN[fam]
        L += [f"- **{fam}** (a guess from {len(e)} sign(s): " + "; ".join(sorted(e)[:3]) + ")",
              f"  - read first: {doc}", f"  - readers: {reader}", f"  - carries over: {mods}",
              f"  - start from: `target/release/conscripter-convert {cmd}`"]
    return L


# import name -> category (prefix match)
CATS = {
    "graphics": ["Direct3DCreate", "DirectDrawCreate", "D3D11CreateDevice", "D3D12", "CreateDXGIFactory", "wgl",
                 "SwapBuffers", "BitBlt", "StretchBlt", "StretchDIBits", "SetDIBitsToDevice", "CreateDIBSection",
                 "ChangeDisplaySettings", "glBegin", "glDrawArrays"],
    "sound": ["DirectSoundCreate", "waveOut", "XAudio2Create", "mciSendString", "PlaySound", "acmStream", "midiOut",
              "ov_", "mixer"],
    "movie": ["AVIFile", "MCIWnd", "ICDecompress"],
    "timing": ["timeGetTime", "timeBeginPeriod", "timeSetEvent", "QueryPerformance", "GetTickCount", "Sleep",
               "SetTimer", "MsgWaitForMultipleObjects", "CreateWaitableTimer", "WaitForSingleObject"],
    "input": ["DirectInput", "GetAsyncKeyState", "GetKeyState", "GetKeyboardState", "GetCursorPos", "joyGet",
              "XInputGetState", "PeekMessage", "GetMessage", "DispatchMessage"],
    "files": ["CreateFile", "ReadFile", "SetFilePointer", "CreateFileMapping", "MapViewOfFile", "FindFirstFile",
              "GetFileAttributes", "fopen", "fread", "_open", "_read"],
    "com": ["CoCreateInstance", "CoInitialize"],
    "registry": ["RegOpenKey", "RegQueryValue", "RegCreateKey", "RegSetValue"],
}

WAIT_APIS = ["Sleep", "timeGetTime", "QueryPerformanceCounter", "GetTickCount", "PeekMessage", "MsgWaitForMultipleObjects",
             "WaitForSingleObject"]
PRESENT_APIS = ["SwapBuffers", "BitBlt", "StretchBlt", "StretchDIBits", "SetDIBitsToDevice", "UpdateWindow"]
# vtable offsets of present calls (32-bit): IDirect3DDevice9::Present 17, IDirect3DDevice8::Present 15,
# IDirectDrawSurface::Flip 11, IDXGISwapChain::Present 8
PRESENT_VT = {"d3d9": 0x44, "d3d8": 0x3c, "ddraw": 0x2c, "dxgi": 0x20}


def walk(root, depth=3):
    root = os.path.abspath(root)
    for d, dirs, files in os.walk(root):
        if d[len(root):].count(os.sep) >= depth:
            dirs[:] = []
        for f in files:
            yield os.path.join(d, f)


def head(path, n=16):
    try:
        with open(path, "rb") as f:
            return f.read(n)
    except OSError:
        return b""


def show_bytes(b):
    return " ".join(f"{c:02x}" for c in b[:8]) + "  " + "".join(chr(c) if 32 <= c < 127 else "." for c in b[:16])


def pick_exe(root, names):
    exes = [p for p in names if p.lower().endswith(".exe") and os.path.dirname(p) == root
            and not re.match(r"(unins|setup|config|install|launcher)", os.path.basename(p), re.I)]
    return max(exes, key=os.path.getsize) if exes else None


def imports(path):
    if not pefile:
        return None, {}
    pe = pefile.PE(path, fast_load=True)
    pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"]])
    bits = 64 if pe.FILE_HEADER.Machine == 0x8664 else 32
    out = {}
    for d in getattr(pe, "DIRECTORY_ENTRY_IMPORT", []):
        out[d.dll.decode(errors="replace")] = [i.name.decode(errors="replace") if i.name else f"#{i.ordinal}"
                                              for i in d.imports]
    pe.close()
    return bits, out


def categorize(imps):
    cats = collections.defaultdict(list)
    for dll, fns in imps.items():
        for fn in fns:
            for cat, pre in CATS.items():
                if any(fn.startswith(p) for p in pre):
                    cats[cat].append(fn)
        low = dll.lower()
        if low.startswith("dsound") and all(f.startswith("#") for f in fns):
            cats["sound"].append("DirectSoundCreate (by ordinal)")
        for lib, cat in [("d3d", "graphics"), ("ddraw", "graphics"), ("opengl", "graphics"), ("dinput", "input"),
                         ("xaudio", "sound"), ("xinput", "input"), ("quartz", "movie"), ("strmiids", "movie")]:
            if low.startswith(lib) and dll not in cats[cat]:
                cats[cat].append(dll)
    return cats


def engines(root, names, exe_bytes):
    hits = collections.defaultdict(set)
    magics = {}
    for p in names:
        base = os.path.basename(p)
        for eng, kind, pat in SIGNS:
            if kind == "file" and re.fullmatch(pat, base, re.I):
                hits[eng].add(f"file {os.path.relpath(p, root)}")
        if os.path.getsize(p) > 4096:
            magics[p] = head(p)
    for p, b in magics.items():
        for eng, kind, pat in SIGNS:
            if kind == "magic" and b.startswith(pat):
                hits[eng].add(f"magic {pat!r} in {os.path.relpath(p, root)}")
    for eng, kind, pat in SIGNS:
        if kind == "str" and exe_bytes and re.search(pat, exe_bytes):
            hits[eng].add(f"exe string {pat.decode(errors='replace')}")
    return hits


def rtti(exe_bytes, n=40):
    names = sorted(set(m.decode(errors="replace") for m in re.findall(rb"\.\?AV([A-Za-z_][\w@]{2,60})@@", exe_bytes)))
    return names[:n], len(names)


# ---- Ghidra candidates ----
def ghidra_candidates(gdir, exe_imps):
    """Ranks the export's functions: (dispatch, wait, present) lists of (score, addr, name, why)."""
    ddir = os.path.join(gdir, "decomp")
    dispatch, wait, present = [], [], []
    vt = [(lib, off) for lib, off in PRESENT_VT.items() if any(d.lower().startswith(lib) for d in exe_imps)]
    for fn in sorted(os.listdir(ddir)):
        m = re.match(r"([0-9a-fA-F]{8,16})_(.*)\.c$", fn)
        if not m:
            continue
        addr, name = m.groups()
        text = open(os.path.join(ddir, fn), encoding="utf-8", errors="replace").read()
        calls = next((ln for ln in text.splitlines()[:6] if ln.startswith("// calls:")), "")
        ncallees = calls.count(",") + 1 if len(calls) > 10 else 0
        cases = len(re.findall(r"^\s*case ", text, re.M))
        loop = bool(re.search(r"\bdo \{|\bwhile \(", text))
        table = len(re.findall(r"\(\*\*?\(code \*\*?\)\s*\([^;]*\* ?[48]\)\)", text))
        if (cases >= 20 and ncallees >= 5) or ncallees >= 40 or table:  # a CRT memcpy's switch calls nothing
            dispatch.append((max(cases, ncallees) + 20 * table, addr, name,
                             f"{cases} cases, {ncallees} callees" + (f", {table} table calls" if table else "")))
        used = [a for a in WAIT_APIS if re.search(r"::" + a, calls)]
        if loop and len(used) >= 2:
            wait.append((len(used), addr, name, "loop calling " + ", ".join(used)))
        pused = [a for a in PRESENT_APIS if re.search(r"::" + a, calls)]
        vhits = [f"{lib} vtable +{off:#x}" for lib, off in vt if re.search(rf"\+ {off:#x}\)\)\s*\(", text)]
        if pused or vhits:
            present.append((len(pused) * 2 + len(vhits), addr, name, ", ".join(pused + vhits)))
    return [sorted(x, key=lambda r: -r[0])[:k] for x, k in ((dispatch, 8), (wait, 8), (present, 10))]


def report(root, out, exe=None, gdir=None):
    os.makedirs(out, exist_ok=True)
    names = [p for p in walk(root) if os.path.isfile(p)]
    exe = os.path.join(root, exe) if exe else pick_exe(root, names)
    exe_bytes = open(exe, "rb").read() if exe else b""
    bits, imps = imports(exe) if exe else (None, {})
    L = [f"# First contact: {os.path.basename(os.path.abspath(root))}", "",
         f"Made by `research/tools/firstcontact/first_contact.py`. Exe: `{os.path.relpath(exe, root) if exe else 'none'}`"
         + (f", {bits}-bit, {len(exe_bytes)} bytes" if exe else "") + ".", ""]
    hits = engines(root, names, exe_bytes)
    fam = known(hits)
    if fam:
        L += ["## Known family: we already convert this", "",
              "Check the evidence before trusting it; `conscripter-convert games <install>` names an exact game.", ""] + fam + [""]
    L += ["## Engine guesses", ""]
    for eng, ev in sorted(hits.items(), key=lambda kv: -len(kv[1])):
        L.append(f"- **{eng}**: " + "; ".join(sorted(ev)[:4]) + (f" (+{len(ev) - 4})" if len(ev) > 4 else ""))
    if not hits:
        L.append("- none of the known signatures: read the strings and the archive magic below")
    L += ["", "## Imports by use", ""]
    for cat, fns in sorted(categorize(imps).items()):
        L.append(f"- {cat}: " + ", ".join(sorted(set(fns))))
    L += ["", "DLLs: " + ", ".join(imps) if imps else "(no import table read)", "", "## Files", "",
          "| ext | files | MB | first bytes (largest) |", "|---|---|---|---|"]
    by = collections.defaultdict(list)
    for p in names:
        by[os.path.splitext(p)[1].lower() or "(none)"].append(p)
    for ext, ps in sorted(by.items(), key=lambda kv: -sum(os.path.getsize(p) for p in kv[1])):
        big = max(ps, key=os.path.getsize)
        L.append(f"| {ext} | {len(ps)} | {sum(map(os.path.getsize, ps)) / 1e6:.1f} | `{show_bytes(head(big))}` |")
    cls, n = rtti(exe_bytes)
    if n:
        L += ["", f"## RTTI classes ({n}, first {len(cls)})", "", ", ".join(cls)]
    if gdir:
        d, w, p = ghidra_candidates(gdir, imps)
        rows = []
        L += ["", "## Candidates from the Ghidra export", "",
              f"Export: `{gdir}`. Hooks: `hooks.tsv` (count breakpoints; fc.sh --hooks). A dispatch runs per",
              "statement, a present per frame, a wait loop per frame or per wait: the run's \"sec\" records tell.", ""]
        for title, lst, kind in (("dispatch", d, "dispatch"), ("wait loop", w, "wait"), ("present", p, "present")):
            L.append(f"- {title}: " + ("; ".join(f"`{a}` {nm} ({why})" for _, a, nm, why in lst) or "none found"))
            rows += [f"{a}\t{kind}_{a}_{nm}\tcount" for _, a, nm, _ in lst if a not in "".join(rows)]
        with open(os.path.join(out, "hooks.tsv"), "w") as f:
            f.write("# first_contact.py candidates: address, name, mode (fctrace.c FC_HOOKS)\n" + "\n".join(rows) + "\n")
    L += ["", "## Next", "", "One logged run: `research/tools/firstcontact/fc.sh <install dir>"
          + (" --hooks <out>/hooks.tsv" if gdir else "") + "` (docs/new-game.md step 0)."]
    with open(os.path.join(out, "report.md"), "w") as f:
        f.write("\n".join(L) + "\n")
    return L


def self_test():
    import tempfile
    t = tempfile.mkdtemp()
    open(os.path.join(t, "nss.npa"), "wb").write(b"\0" * 5000)
    open(os.path.join(t, "data.xp3"), "wb").write(b"XP3\r\n \n\x1a\x8b\x67\x01" + b"\0" * 5000)
    h = engines(t, [os.path.join(t, f) for f in os.listdir(t)], b"..TVP(KIRIKIRI)..")
    assert "Nitroplus NSS" in h and len(h["KiriKiri / KAG"]) == 2, h
    assert "docs/games/django.md" in known(h)[1] and "KiriKiri" not in "".join(known(h)), known(h)
    open(os.path.join(t, "SCRIPT.cpk"), "wb").write(b"CPK " + b"\0" * 5000)
    h = engines(t, [os.path.join(t, "SCRIPT.cpk")], b"")
    assert len(h["MAGES / 5pb. (CRI)"]) == 2 and "mages convert" in known(h)[-1], h
    assert categorize({"WINMM.dll": ["timeGetTime"], "DSOUND.dll": ["#1"]})["sound"] == ["DirectSoundCreate (by ordinal)"]
    g = os.path.join(t, "g", "decomp")
    os.makedirs(g)
    open(os.path.join(g, "00401000_FUN_00401000.c"), "w").write(
        "// 00401000\n// calls: EXTERNAL:1 KERNEL32.DLL::Sleep, EXTERNAL:2 WINMM.DLL::timeGetTime\n do { x(); } while (1);\n")
    open(os.path.join(g, "00402000_FUN_00402000.c"), "w").write(
        "// 00402000\n// calls: a, b, c, d, e\n" + "case 1:\n" * 25 + "(**(code **)(*p + 0x44))(p);\n")
    d, w, p = ghidra_candidates(os.path.join(t, "g"), {"d3d9.dll": []})
    assert [r[1] for r in d] == ["00402000"] and [r[1] for r in w] == ["00401000"] and [r[1] for r in p] == ["00402000"], (d, w, p)
    print("ok")


if __name__ == "__main__":
    a = sys.argv[1:]
    if a == ["--self-test"]:
        self_test()
        sys.exit()

    def opt(k, flag=False):
        if k not in a:
            return None
        i = a.index(k)
        v = True if flag else a[i + 1]
        del a[i:i + (1 if flag else 2)]
        return v
    out, exe, gdir, grun = opt("-o"), opt("--exe"), opt("--ghidra"), opt("--ghidra-run", True)
    if len(a) != 1 or not os.path.isdir(a[0]):
        sys.exit(__doc__)
    repo = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
    out = out or os.path.join(repo, "out", os.path.basename(os.path.abspath(a[0])) + "-first-contact")
    if grun:
        gdir = os.path.join(out, "ghidra")
        e = os.path.join(a[0], exe) if exe else pick_exe(os.path.abspath(a[0]), [p for p in walk(a[0])])
        subprocess.run([os.path.join(repo, "research/tools/ghidra/ghidra_export.sh"), e, gdir], check=True)
    for ln in report(os.path.abspath(a[0]), out, exe, gdir)[:60]:
        print(ln)
    print(f"-> {out}/report.md")
