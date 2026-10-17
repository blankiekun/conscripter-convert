"""Putting CONscripter beside a finished build: the release's binary for
this platform, named after the game and with the game's own icon.

    Windows  <title>.exe, its icon resources replaced by the game's
    Linux    <title>, plus <title>.desktop and icon.png (an ELF has no icon;
             the .desktop file also starts it in the game folder)
    macOS    <title>.app, a bundle whose launcher starts the binary in the
             folder the .app is in, with the icon as icon.icns

The binary comes from CONscripter's GitHub release for the version the
build was made for (its version.txt), kept in a cache folder.
"""

from __future__ import annotations

import io
import os
import platform
import re
import shutil
import ssl
import struct
import subprocess
import sys
import urllib.request
from pathlib import Path
from typing import Callable, Optional

RELEASES = "https://github.com/blankiekun/CONscripter/releases"
API = "https://api.github.com/repos/blankiekun/CONscripter/releases/latest"


# --- which binary ---------------------------------------------------------

def platform_suffix(version: Optional[str] = None) -> Optional[str]:
    """The release asset's suffix for this computer and CONscripter
    `version` (None: the newest), None when there is no build for it."""
    machine = platform.machine().lower()
    if os.name == "nt":
        return "x64_win.exe" if machine in ("amd64", "x86_64", "arm64") else "x86_win.exe"
    if sys.platform == "darwin":
        if not version or not _before_universal(version):
            return "universal_darwin"  # Apple silicon and Intel
        # Up to 0.28.1 only Apple silicon; an x64 app under Rosetta still runs it.
        if machine == "arm64" or _sysctl("sysctl.proc_translated") == "1":
            return "aarch64_darwin"
        return None
    if sys.platform.startswith("linux"):
        return {"x86_64": "x64_linux", "amd64": "x64_linux", "aarch64": "arm64_linux", "arm64": "arm64_linux",
                "i686": "x86_linux", "i386": "x86_linux"}.get(machine) or (
            "arm32_linux" if machine.startswith("arm") else None)
    return None


def _before_universal(version: str) -> bool:
    """True for CONscripter 0.28.1 and older (no universal macOS build);
    a version that doesn't parse ("v0.29.0-dev" does) counts as new."""
    parts = [re.match(r"\d+", p) for p in version.lstrip("v").split(".")[:3]]
    if len(parts) < 3 or not all(parts):
        return False
    return tuple(int(m.group()) for m in parts) <= (0, 28, 1)


def _sysctl(name: str) -> str:
    try:
        return subprocess.run(["sysctl", "-n", name], capture_output=True, text=True, timeout=5).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return ""


def build_version(output: Path) -> Optional[str]:
    """The CONscripter version the build was made for, from its version.txt
    ("build …, converter …, CONscripter 0.21.0")."""
    try:
        text = (output / "version.txt").read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    m = re.search(r"CONscripter (\d+\.\d+\.\d+)", text)
    return m.group(1) if m else None


def tls_context() -> ssl.SSLContext:
    """The system's certificates plus certifi's: a bundled Python's
    OpenSSL looks for them where the machine that built it keeps them
    (Ubuntu's /usr/lib/ssl), which other Linux systems don't have, and
    python.org's Python on macOS has none of its own."""
    context = ssl.create_default_context()
    try:
        import certifi
        context.load_verify_locations(certifi.where())
    except (ImportError, OSError):
        pass
    return context


def _open(request, timeout: int):
    return urllib.request.urlopen(request, timeout=timeout, context=tls_context())


def latest_version() -> str:
    import json
    with _open(urllib.request.Request(API, headers={"Accept": "application/vnd.github+json"}), 30) as r:
        return json.load(r)["tag_name"].lstrip("v")


def cache_dir() -> Path:
    if os.name == "nt":
        base = Path(os.environ.get("LOCALAPPDATA") or Path.home() / "AppData/Local")
    elif sys.platform == "darwin":
        base = Path.home() / "Library/Caches"
    else:
        base = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    return base / "conscripter-convert"


def download(version: str, suffix: str, log: Callable[[str], None]) -> Path:
    """The release binary, from the cache or downloaded into it."""
    name = f"conscripter_v{version}_{suffix}"
    path = cache_dir() / name
    if path.is_file():
        log(f"CONscripter {version}: {path} (downloaded before)")
        return path
    url = f"{RELEASES}/download/v{version}/{name}"
    log(f"Downloading {url}")
    path.parent.mkdir(parents=True, exist_ok=True)
    part = path.with_name(name + ".part")
    with _open(url, 60) as r, open(part, "wb") as f:
        shutil.copyfileobj(r, f)
    part.replace(path)
    return path


# --- the game's icon --------------------------------------------------------

def file_name(title: str) -> str:
    """`title` as a file name every system accepts."""
    name = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "", title).strip().strip(".")
    return name or "game"


def exe_icon(exe: Path) -> Optional[bytes]:
    """The first icon group of a Windows executable, as an .ico file."""
    try:
        pe = PE(exe.read_bytes())
        tree = pe.resources()
    except (OSError, ValueError, struct.error):
        return None
    groups = tree.get(RT_GROUP_ICON)
    if not groups:
        return None
    group = next(iter(_first(groups).values()))
    icons = tree.get(RT_ICON, {})
    _, kind, count = struct.unpack_from("<HHH", group)
    entries, images = [], []
    for i in range(count):
        w, h, colors, res, planes, bits, size, ident = struct.unpack_from("<BBBBHHIH", group, 6 + 14 * i)
        data = next(iter(icons.get(ident, {}).values()), None)
        if data is None:
            continue
        entries.append((w, h, colors, res, planes, bits, len(data)))
        images.append(data)
    if not images:
        return None
    return _ico(entries, images)


def _first(entries: dict) -> dict:
    """Windows takes named entries before numbered ones, each in order."""
    key = sorted(entries, key=lambda k: (isinstance(k, int), k))[0]
    return entries[key]


def _ico(entries: list[tuple], images: list[bytes]) -> bytes:
    out = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    for e, data in zip(entries, images):
        out += struct.pack("<BBBBHHII", *e[:6], len(data), offset)
        offset += len(data)
    return out + b"".join(images)


def _parse_ico(ico: bytes) -> list[tuple[tuple, bytes]]:
    _, kind, count = struct.unpack_from("<HHH", ico)
    if kind != 1:
        raise ValueError("not an icon")
    out = []
    for i in range(count):
        w, h, colors, res, planes, bits, size, offset = struct.unpack_from("<BBBBHHII", ico, 6 + 16 * i)
        out.append(((w, h, colors, res, planes, bits), ico[offset:offset + size]))
    return out


def png_to_ico(png: bytes) -> bytes:
    """An .ico of the picture at its own size and the usual smaller ones."""
    from PIL import Image
    im = Image.open(io.BytesIO(png)).convert("RGBA")
    side = min(256, max(im.size))
    sizes = [(s, s) for s in (16, 24, 32, 48, 64, 128, 256) if s <= side]
    buf = io.BytesIO()
    im.save(buf, "ICO", sizes=sizes)
    return buf.getvalue()


def ico_to_png(ico: bytes) -> bytes:
    """The largest picture in an .ico, as PNG."""
    from PIL import Image
    im = Image.open(io.BytesIO(ico))
    im.size = max(im.info.get("sizes", [im.size]))  # Pillow's IcoImageFile picks by size
    buf = io.BytesIO()
    im.convert("RGBA").save(buf, "PNG")
    return buf.getvalue()


def png_to_icns(png: bytes) -> bytes:
    from PIL import Image
    im = Image.open(io.BytesIO(png)).convert("RGBA")
    if max(im.size) < 512:  # Finder shows icons at up to 512 points
        im = im.resize((512, 512), Image.LANCZOS)
    buf = io.BytesIO()
    im.save(buf, "ICNS")
    return buf.getvalue()


# --- putting it in the output -----------------------------------------------

def install(output: Path, title: str, icon: Optional[bytes], log: Callable[[str], None]) -> Path:
    """Puts CONscripter into `output` as `title`, with `icon` (an .ico or a
    PNG file's bytes) when given. Returns what it wrote (the program, or the
    .app on macOS). Raises on failure; the build itself is still fine."""
    no_build = f"CONscripter has no build for this computer ({sys.platform} {platform.machine()})"
    if not platform_suffix():
        raise RuntimeError(no_build)
    version = build_version(output) or latest_version()
    suffix = platform_suffix(version)
    if not suffix:
        raise RuntimeError(f"{no_build}, version {version}")
    binary = download(version, suffix, log)
    name = file_name(title)
    ico = png = None
    if icon:
        try:
            ico = icon if icon[:4] == b"\0\0\1\0" else png_to_ico(icon)
            png = icon if icon[:8] == b"\x89PNG\r\n\x1a\n" else ico_to_png(icon)
        except ImportError:
            log("The game's icon is left out: Pillow is not installed.")
        except Exception as e:  # noqa: BLE001 - a broken icon must not stop the rest
            log(f"The game's icon is left out: {e}")
    if os.name == "nt":
        target = output / f"{name}.exe"
        data = binary.read_bytes()
        if ico:
            try:
                data = set_exe_icon(data, ico)
            except (ValueError, struct.error) as e:
                log(f"Kept CONscripter's own icon: {e}")
        target.write_bytes(data)
    elif sys.platform == "darwin":
        target = output / f"{name}.app"
        _make_app(target, binary, name, png, version)
    else:
        target = output / name
        shutil.copyfile(binary, target)
        target.chmod(0o755)
        if png:
            (output / "icon.png").write_bytes(png)
        (output / f"{name}.desktop").write_text(_desktop_entry(output, target, title, png is not None),
                                                encoding="utf-8")
        (output / f"{name}.desktop").chmod(0o755)
    log(f"CONscripter {version} is in the game folder as {target.name}.")
    return target


def _desktop_entry(output: Path, program: Path, title: str, icon: bool) -> str:
    """A launcher for file managers: it starts the game in its folder (where
    CONscripter looks for 0.txt). Its paths are absolute, so it needs
    writing again if the folder moves (build again, or edit the file)."""
    lines = ["[Desktop Entry]", "Type=Application", f"Name={title}",
             f'Exec="{program}"', f"Path={output}", "Terminal=false", "Categories=Game;"]
    if icon:
        lines.append(f"Icon={output / 'icon.png'}")
    return "\n".join(lines) + "\n"


def _make_app(app: Path, binary: Path, name: str, png: Optional[bytes], version: str) -> None:
    if app.exists():
        shutil.rmtree(app)
    macos = app / "Contents" / "MacOS"
    macos.mkdir(parents=True)
    shutil.copyfile(binary, macos / "conscripter")
    (macos / "conscripter").chmod(0o755)
    # A .app starts in /: the launcher moves to the folder the .app is in.
    launcher = macos / "launch"
    launcher.write_text('#!/bin/sh\ncd "$(dirname "$0")/../../.." && exec "$(dirname "$0")/conscripter" "$@"\n')
    launcher.chmod(0o755)
    icon_key = ""
    if png:
        try:
            (app / "Contents" / "Resources").mkdir()
            (app / "Contents" / "Resources" / "icon.icns").write_bytes(png_to_icns(png))
            icon_key = "  <key>CFBundleIconFile</key><string>icon</string>\n"
        except Exception:  # noqa: BLE001 - no icon then
            pass
    ident = re.sub(r"[^A-Za-z0-9.-]", "-", name.encode("ascii", "ignore").decode()) or "game"
    (app / "Contents" / "Info.plist").write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
        '<plist version="1.0">\n<dict>\n'
        f"  <key>CFBundleName</key><string>{_xml(name)}</string>\n"
        f"  <key>CFBundleDisplayName</key><string>{_xml(name)}</string>\n"
        f"  <key>CFBundleIdentifier</key><string>io.github.blankiekun.conscripter.{ident}</string>\n"
        "  <key>CFBundleExecutable</key><string>launch</string>\n"
        "  <key>CFBundlePackageType</key><string>APPL</string>\n"
        f"  <key>CFBundleShortVersionString</key><string>{version}</string>\n"
        f"{icon_key}"
        "  <key>NSHighResolutionCapable</key><true/>\n"
        "</dict>\n</plist>\n", encoding="utf-8")


def _xml(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


# --- Windows executables' icons ---------------------------------------------

RT_ICON, RT_GROUP_ICON = 3, 14


class PE:
    """Just enough of a PE file to read its resources and add a section."""

    def __init__(self, data: bytes):
        if data[:2] != b"MZ":
            raise ValueError("not a Windows program")
        self.data = bytearray(data)
        self.pe = struct.unpack_from("<I", data, 0x3C)[0]
        if data[self.pe:self.pe + 4] != b"PE\0\0":
            raise ValueError("not a Windows program")
        self.coff = self.pe + 4
        self.nsections, = struct.unpack_from("<H", data, self.coff + 2)
        opt_size, = struct.unpack_from("<H", data, self.coff + 16)
        self.opt = self.coff + 20
        magic, = struct.unpack_from("<H", data, self.opt)
        self.dirs = self.opt + (112 if magic == 0x20B else 96)
        self.table = self.opt + opt_size
        self.sections = [struct.unpack_from("<8sIIII", data, self.table + 40 * i) for i in range(self.nsections)]

    def offset(self, rva: int) -> int:
        for _, vsize, va, rsize, raw in self.sections:
            if va <= rva < va + max(vsize, rsize):
                return raw + rva - va
        raise ValueError(f"address {rva:#x} is in no section")

    def directory(self, i: int) -> tuple[int, int]:
        return struct.unpack_from("<II", self.data, self.dirs + 8 * i)

    def resources(self) -> dict:
        """{type: {name: {language: bytes}}}, names as int or str."""
        rva, size = self.directory(2)
        if not rva:
            return {}
        base = self.offset(rva)

        def level(off: int, depth: int):
            named, ids = struct.unpack_from("<HH", self.data, off + 12)
            out = {}
            for i in range(named + ids):
                name, target = struct.unpack_from("<II", self.data, off + 16 + 8 * i)
                if name & 0x80000000:
                    p = base + (name & 0x7FFFFFFF)
                    n, = struct.unpack_from("<H", self.data, p)
                    key = bytes(self.data[p + 2:p + 2 + 2 * n]).decode("utf-16-le")
                else:
                    key = name
                if target & 0x80000000:
                    out[key] = level(base + (target & 0x7FFFFFFF), depth + 1)
                else:
                    drva, dsize, _, _ = struct.unpack_from("<IIII", self.data, base + target)
                    p = self.offset(drva)
                    out[key] = bytes(self.data[p:p + dsize])
            return out
        return level(base, 0)


def _build_resources(tree: dict, rva: int) -> bytes:
    """The .rsrc section for `tree`, to be loaded at `rva`."""
    def order(d: dict):
        return sorted(d, key=lambda k: (isinstance(k, int), k.upper() if isinstance(k, str) else k))

    # Layout: every directory, then the names, then the data entries, then the data.
    dirs, strings, entries, blobs = [], [], [], []

    def walk(d: dict) -> int:
        index = len(dirs)
        dirs.append(None)
        children = []
        for k in order(d):
            v = d[k]
            if isinstance(v, dict):
                children.append((k, ("dir", walk(v))))
            else:
                entries.append(len(blobs))
                blobs.append(v)
                children.append((k, ("data", len(entries) - 1)))
        dirs[index] = children
        return index
    walk(tree)
    dir_offsets, pos = [], 0
    for children in dirs:
        dir_offsets.append(pos)
        pos += 16 + 8 * len(children)
    name_offsets: dict[str, int] = {}
    for children in dirs:
        for k, _ in children:
            if isinstance(k, str) and k not in name_offsets:
                name_offsets[k] = pos
                strings.append(k)
                pos += 2 + 2 * len(k)
    pos += -pos % 4
    entry_start = pos
    pos += 16 * len(entries)
    data_offsets = []
    for b in blobs:
        pos += -pos % 8
        data_offsets.append(pos)
        pos += len(b)
    out = bytearray(pos)
    for children, at in zip(dirs, dir_offsets):
        named = sum(isinstance(k, str) for k, _ in children)
        struct.pack_into("<IIHHHH", out, at, 0, 0, 0, 0, named, len(children) - named)
        for i, (k, (kind, ref)) in enumerate(children):
            name = (0x80000000 | name_offsets[k]) if isinstance(k, str) else k
            target = (0x80000000 | dir_offsets[ref]) if kind == "dir" else entry_start + 16 * ref
            struct.pack_into("<II", out, at + 16 + 8 * i, name, target)
    for k, at in name_offsets.items():
        raw = k.encode("utf-16-le")
        struct.pack_into("<H", out, at, len(k))
        out[at + 2:at + 2 + len(raw)] = raw
    for i, blob_index in enumerate(entries):
        struct.pack_into("<IIII", out, entry_start + 16 * i, rva + data_offsets[blob_index], len(blobs[blob_index]), 0, 0)
    for at, b in zip(data_offsets, blobs):
        out[at:at + len(b)] = b
    return bytes(out)


def set_exe_icon(exe: bytes, ico: bytes) -> bytes:
    """`exe` with its icons replaced by `ico`'s. The resources are written
    again in a new last section (the old one stays, unused): the rest of
    the program doesn't move."""
    pe = PE(exe)
    tree = pe.resources()
    groups = tree.get(RT_GROUP_ICON) or {1: {1033: b""}}
    group_name = sorted(groups, key=lambda k: (isinstance(k, int), k))[0]
    lang = next(iter(groups[group_name]))
    images = _parse_ico(ico)
    if not images:
        raise ValueError("the icon has no pictures")
    tree[RT_ICON] = {i + 1: {lang: data} for i, (_, data) in enumerate(images)}
    group = struct.pack("<HHH", 0, 1, len(images)) + b"".join(
        struct.pack("<BBBBHHIH", *e, len(data), i + 1) for i, (e, data) in enumerate(images))
    tree[RT_GROUP_ICON] = {group_name: {lang: group}}

    file_align, = struct.unpack_from("<I", pe.data, pe.opt + 36)
    sect_align, = struct.unpack_from("<I", pe.data, pe.opt + 32)
    headers_size, = struct.unpack_from("<I", pe.data, pe.opt + 60)
    new_header = pe.table + 40 * pe.nsections
    first_raw = min((raw for *_, raw in pe.sections if raw), default=headers_size)
    if new_header + 40 > min(headers_size, first_raw) or any(pe.data[new_header:new_header + 40]):
        raise ValueError("no room for another section")
    end_va = max(va + vsize for _, vsize, va, _, _ in pe.sections)
    rva = -(-end_va // sect_align) * sect_align
    rsrc = _build_resources(tree, rva)
    if len(pe.data) > max(raw + size for _, _, _, size, raw in pe.sections):
        raise ValueError("the program has data after its sections")
    raw = -(-len(pe.data) // file_align) * file_align
    rsize = -(-len(rsrc) // file_align) * file_align
    struct.pack_into("<8sIIIIIIHHI", pe.data, new_header, b".rsrc", len(rsrc), rva, rsize, raw, 0, 0, 0, 0,
                     0x40000040)  # initialized data, readable
    struct.pack_into("<H", pe.data, pe.coff + 2, pe.nsections + 1)
    struct.pack_into("<I", pe.data, pe.opt + 56, rva + -(-len(rsrc) // sect_align) * sect_align)  # SizeOfImage
    struct.pack_into("<II", pe.data, pe.dirs + 16, rva, len(rsrc))  # the resource directory
    struct.pack_into("<II", pe.data, pe.dirs + 32, 0, 0)  # a signature no longer matches
    out = pe.data + b"\0" * (raw - len(pe.data)) + rsrc + b"\0" * (rsize - len(rsrc))
    struct.pack_into("<I", out, pe.opt + 64, _checksum(out, pe.opt + 64))
    return bytes(out)


def _checksum(data: bytearray, at: int) -> int:
    padded = bytes(data) + b"\0" * (-len(data) % 4)
    total = 0
    for i, (word,) in enumerate(struct.iter_unpack("<I", padded)):
        if i * 4 != at:
            total += word
    total = (total & 0xFFFFFFFF) + (total >> 32)
    total = (total & 0xFFFF) + (total >> 16)
    total = (total & 0xFFFF) + (total >> 16)
    return (total & 0xFFFF) + len(data)
