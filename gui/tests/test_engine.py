"""CONscripter beside a build (engine.py): python -m unittest discover gui/tests"""

import io
import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import engine  # noqa: E402


def tiny_pe(resources: dict) -> bytes:
    """A PE32+ file with one .rsrc section holding `resources`."""
    rva, raw, align = 0x1000, 0x200, 0x200
    rsrc = engine._build_resources(resources, rva)
    rsize = -(-len(rsrc) // align) * align
    opt_size = 112 + 16 * 8
    head = bytearray(raw)
    head[:2] = b"MZ"
    struct.pack_into("<I", head, 0x3C, 0x40)
    head[0x40:0x44] = b"PE\0\0"
    coff = 0x44
    struct.pack_into("<HHIIIHH", head, coff, 0x8664, 1, 0, 0, 0, opt_size, 0x22)
    opt = coff + 20
    struct.pack_into("<H", head, opt, 0x20B)
    struct.pack_into("<II", head, opt + 32, 0x1000, align)  # section, file alignment
    struct.pack_into("<II", head, opt + 56, rva + 0x1000, raw)  # SizeOfImage, SizeOfHeaders
    struct.pack_into("<I", head, opt + 108, 16)  # NumberOfRvaAndSizes
    struct.pack_into("<II", head, opt + 112 + 16, rva, len(rsrc))
    struct.pack_into("<8sIIII", head, opt + opt_size, b".rsrc", len(rsrc), rva, rsize, raw)
    return bytes(head) + rsrc + b"\0" * (rsize - len(rsrc))


def ico(*sizes: int) -> bytes:
    """An .ico with a fake picture per size (the bytes are only carried)."""
    entries = [(s % 256, s % 256, 0, 0, 1, 32) for s in sizes]
    return engine._ico(entries, [bytes([s % 256]) * (10 + s) for s in sizes])


class ExeIcon(unittest.TestCase):
    def test_replace_and_read_back(self):
        old = ico(16)
        images = engine._parse_ico(old)
        group = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHIH", *images[0][0], len(images[0][1]), 1)
        exe = tiny_pe({engine.RT_ICON: {1: {1033: images[0][1]}},
                       engine.RT_GROUP_ICON: {"IDI_ICON1": {1033: group}},
                       16: {1: {1033: b"version"}}})
        self.assertEqual(engine.PE(exe).resources()[engine.RT_GROUP_ICON].keys(), {"IDI_ICON1"})
        new = ico(32, 48, 256)
        out = engine.set_exe_icon(exe, new)
        pe = engine.PE(out)
        self.assertEqual(pe.nsections, 2)
        tree = pe.resources()
        self.assertEqual(tree[16][1][1033], b"version")  # the other resources stay
        self.assertEqual(list(tree[engine.RT_GROUP_ICON]), ["IDI_ICON1"])  # the name the program loads
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "x.exe"
            p.write_bytes(out)
            self.assertEqual(engine.exe_icon(p), new)

    def test_not_a_program(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "x.exe"
            p.write_bytes(b"hello")
            self.assertIsNone(engine.exe_icon(p))
        with self.assertRaises(ValueError):
            engine.set_exe_icon(b"hello", ico(16))


class Install(unittest.TestCase):
    def test_names(self):
        self.assertEqual(engine.file_name("装甲悪鬼村正"), "装甲悪鬼村正")
        self.assertEqual(engine.file_name('A: "B"/C?.'), "A BC")
        self.assertEqual(engine.file_name("..."), "game")

    def test_version_from_the_build(self):
        with tempfile.TemporaryDirectory() as d:
            Path(d, "version.txt").write_text("T\r\nbuild 2026-10-15, converter abc, CONscripter 0.21.0\r\n")
            self.assertEqual(engine.build_version(Path(d)), "0.21.0")
            self.assertIsNone(engine.build_version(Path(d) / "none"))

    def test_before_universal(self):
        for v, old in (("0.28.1", True), ("v0.28.1", True), ("0.21.0", True), ("0.28.2", False),
                       ("0.29.0-dev", False), ("v1.0.0rc1", False), ("dev", False), ("", False)):
            self.assertEqual(engine._before_universal(v), old, v)

    def test_mac_suffix(self):
        def suffix(version, machine, translated=""):
            with mock.patch.object(engine.sys, "platform", "darwin"), mock.patch.object(engine.os, "name", "posix"), \
                    mock.patch.object(engine.platform, "machine", return_value=machine), \
                    mock.patch.object(engine, "_sysctl", return_value=translated):
                return engine.platform_suffix(version)
        self.assertEqual(suffix(None, "x86_64"), "universal_darwin")
        self.assertEqual(suffix("0.29.0", "x86_64"), "universal_darwin")
        self.assertEqual(suffix("0.28.1", "arm64"), "aarch64_darwin")
        self.assertEqual(suffix("0.28.1", "x86_64", "1"), "aarch64_darwin")  # Rosetta
        self.assertIsNone(suffix("0.28.1", "x86_64"))  # an Intel Mac, before the universal build

    @unittest.skipIf(os.name == "nt" or sys.platform == "darwin", "the Linux layout")
    def test_linux_layout(self):
        try:
            from PIL import Image
        except ImportError:
            self.skipTest("no Pillow")
        png = io.BytesIO()
        Image.new("RGBA", (128, 128), (200, 0, 0, 255)).save(png, "PNG")
        with tempfile.TemporaryDirectory() as d, mock.patch.dict(os.environ, {"XDG_CACHE_HOME": d}):
            out = Path(d) / "game"
            out.mkdir()
            (out / "version.txt").write_text("T\r\nbuild x, converter y, CONscripter 9.9.9\r\n")
            cached = Path(d) / "conscripter-convert" / f"conscripter_v9.9.9_{engine.platform_suffix()}"
            cached.parent.mkdir()
            cached.write_bytes(b"\x7fELF fake")
            lines = []
            target = engine.install(out, "俺たちに翼はない", png.getvalue(), lines.append)
            self.assertEqual(target, out / "俺たちに翼はない")
            self.assertEqual(target.read_bytes(), b"\x7fELF fake")
            self.assertTrue(os.access(target, os.X_OK))
            with Image.open(out / "icon.png") as im:
                self.assertEqual(im.size, (128, 128))
            desktop = (out / "俺たちに翼はない.desktop").read_text(encoding="utf-8")
            self.assertIn(f"Path={out}", desktop)
            self.assertIn(f"Icon={out / 'icon.png'}", desktop)

    def test_png_and_ico(self):
        try:
            from PIL import Image
        except ImportError:
            self.skipTest("no Pillow")
        png = io.BytesIO()
        Image.new("RGBA", (128, 128), (0, 0, 200, 255)).save(png, "PNG")
        made = engine.png_to_ico(png.getvalue())
        self.assertEqual(max(e[0] for e, _ in engine._parse_ico(made)), 128)
        self.assertEqual(Image.open(io.BytesIO(engine.ico_to_png(made))).size, (128, 128))
        self.assertEqual(engine.png_to_icns(png.getvalue())[:4], b"icns")


if __name__ == "__main__":
    unittest.main()
