"""Detection on fake game folders: python -m unittest discover gui/tests"""

import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import games  # noqa: E402
import tools  # noqa: E402


def touch(root: Path, rel: str, data: bytes = b"x") -> None:
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(data)


def sfo(entries: dict[str, str]) -> bytes:
    """A minimal param.sfo with string values."""
    keys = b"".join(k.encode() + b"\0" for k in entries)
    keys += b"\0" * (-len(keys) % 4)
    index, values, koff = b"", b"", 0
    for k, v in entries.items():
        raw = v.encode() + b"\0"
        size = len(raw) + (-len(raw) % 4)
        index += struct.pack("<HHIII", koff, 0x0204, len(raw), size, len(values))
        values += raw.ljust(size, b"\0")
        koff += len(k) + 1
    key_start = 20 + len(index)
    return struct.pack("<4sIIII", b"\0PSF", 0x101, key_start, key_start + len(keys), len(entries)) + index + keys + values


def make_saiima(root: Path, scripts: int = 174) -> None:
    touch(root, "Farthest2015.exe")
    touch(root, "CD/bv0000.xsa")
    for i in range(scripts):
        touch(root, f"HD/CDFiles/{i:04}.cd")


def make_muramasa(root: Path) -> None:
    for f in ("Muramasa.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"):
        touch(root, f)


def make_oretsuba(root: Path, title_id: str = "PCSG00299", magic: bytes = b"CPK ") -> None:
    touch(root, "media/SCRIPT.cpk", magic + b"\xff\0\0\0")
    touch(root, "media/CG.cpk", b"CPK ")
    touch(root, "sce_sys/param.sfo", sfo({"CATEGORY": "gd", "TITLE": "俺たちに翼はない", "TITLE_ID": title_id}))


class Detect(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def keys(self, folder):
        return [(d.game.key, d.root) for d in games.detect(folder)]

    def test_each_game(self):
        for make, key in ((make_saiima, "saihate-no-ima"), (make_muramasa, "muramasa"), (make_oretsuba, "oretsuba")):
            root = self.dir / key
            make(root)
            found = games.detect(root)
            self.assertEqual([(d.game.key, d.root) for d in found], [(key, root)])
            self.assertEqual(found[0].problems, [])
            self.assertEqual(found[0].warnings, [])

    def test_empty_and_unrelated(self):
        self.assertEqual(self.keys(self.dir), [])
        touch(self.dir, "HD/CDFiles/0000.cd")  # Xuse layout but not this game
        touch(self.dir, "nss.npa")  # some of Muramasa's files
        self.assertEqual(self.keys(self.dir), [])

    def test_missing_dir(self):
        self.assertEqual(self.keys(self.dir / "nope"), [])

    def test_folder_above(self):
        # The Muramasa release: FMDM_game/ゲームデータ holds the data.
        make_muramasa(self.dir / "FMDM_game" / "ゲームデータ")
        self.assertEqual(self.keys(self.dir / "FMDM_game"), [("muramasa", self.dir / "FMDM_game" / "ゲームデータ")])
        self.assertEqual(self.keys(self.dir), [("muramasa", self.dir / "FMDM_game" / "ゲームデータ")])

    def test_too_deep(self):
        make_muramasa(self.dir / "a" / "b" / "c")
        self.assertEqual(self.keys(self.dir), [])

    def test_several(self):
        make_saiima(self.dir / "saiima")
        make_oretsuba(self.dir / "vita")
        self.assertEqual(sorted(k for k, _ in self.keys(self.dir)), ["oretsuba", "saihate-no-ima"])

    def test_incomplete_saiima(self):
        make_saiima(self.dir, scripts=3)
        (d,) = games.detect(self.dir)
        self.assertTrue(d.problems)

    def test_encrypted_vita(self):
        make_oretsuba(self.dir, magic=b"\x8f\x12\x00\x7a")
        (d,) = games.detect(self.dir)
        self.assertIn("encrypted", d.problems[0])

    def test_other_title_id(self):
        make_oretsuba(self.dir, title_id="PCSG99999")
        (d,) = games.detect(self.dir)
        self.assertEqual(d.problems, [])
        self.assertIn("PCSG99999", d.warnings[0])

    def test_sfo(self):
        touch(self.dir, "p.sfo", sfo({"TITLE_ID": "PCSG00299", "APP_VER": "01.00"}))
        self.assertEqual(games.read_sfo(self.dir / "p.sfo"), {"TITLE_ID": "PCSG00299", "APP_VER": "01.00"})


class Commands(unittest.TestCase):
    """The recommended settings, as the game documents give them."""

    def args(self, key):
        game = next(g for g in games.GAMES if g.key == key)
        return game.command(Path("G"), Path("O"), Path("F.ttf"))

    def test_saiima(self):
        self.assertEqual(self.args("saihate-no-ima"),
                         ["convert-game", "G", "-o", "O", "--font", "F.ttf", "--title", "Saihate no Ima COMPLETE"])

    def test_muramasa(self):
        self.assertEqual(self.args("muramasa"), ["convert-nss", "G", "--file", "ma00_000.nss", "--chain", "2000",
                                                  "-o", "O", "--font", "F.ttf", "--title", "装甲悪鬼村正"])

    def test_oretsuba(self):
        self.assertEqual(self.args("oretsuba"), ["mages", "convert", "G", "-o", "O", "--font", "F.ttf"])

    def test_converter_accepts(self):
        """Each command parses with the converter's own --help (when built)."""
        conv = tools.find_converter()
        if not conv:
            self.skipTest("conscripter-convert not built")
        import subprocess
        for g in games.GAMES:
            args = g.command(Path("G"), Path("O"), Path("F.ttf"))
            sub = args[:2] if args[0] == "mages" else args[:1]
            out = subprocess.run([str(conv), *sub, "--help"], capture_output=True, text=True).stdout
            for flag in (a for a in args if a.startswith("-")):
                self.assertIn(flag, out, f"{g.key}: {flag}")


class Output(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        make_muramasa(self.dir / "game")
        (self.det,) = games.detect(self.dir / "game")

    def tearDown(self):
        self.tmp.cleanup()

    def test_default_beside_game(self):
        self.assertEqual(games.default_output(self.det), self.dir / "muramasa-cons")

    def test_default_skips_used_folder(self):
        touch(self.dir, "muramasa-cons/notes.txt")
        self.assertEqual(games.default_output(self.det), self.dir / "muramasa-cons-2")

    def test_default_reuses_previous_build(self):
        touch(self.dir, "muramasa-cons/0.txt")
        self.assertEqual(games.default_output(self.det), self.dir / "muramasa-cons")

    def test_problems(self):
        g = self.dir / "game"
        self.assertIsNotNone(games.output_problem(g, g))
        self.assertIsNotNone(games.output_problem(g, g / "out"))
        self.assertIsNotNone(games.output_problem(g, self.dir))
        touch(self.dir, "afile")
        self.assertIsNotNone(games.output_problem(g, self.dir / "afile"))
        self.assertIsNone(games.output_problem(g, self.dir / "out"))
        self.assertIsNone(games.output_problem(g, self.dir / "gameout"))  # a name prefix isn't inside


def alive(pid: int) -> bool:
    """Running (a killed child whose new parent hasn't reaped it yet is a zombie)."""
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0] != "Z"
    except FileNotFoundError:
        return True  # no /proc (macOS): kill(0) worked
    except ProcessLookupError:
        return False


@unittest.skipIf(os.name == "nt", "uses sh")
class Cancel(unittest.TestCase):
    def test_kills_children(self):
        import threading
        done = threading.Event()
        codes = []
        script = Path(tempfile.mkdtemp()) / "fake"
        script.write_text("#!/bin/sh\necho started\nsleep 60 &\necho $! > \"$1\"\nwait\n")
        script.chmod(0o755)
        pidfile = script.with_name("pid")
        b = tools.Build(script, [str(pidfile)], lambda s: None, lambda c: (codes.append(c), done.set()))
        for _ in range(100):
            if pidfile.exists() and pidfile.read_text().strip():
                break
            done.wait(0.05)
        child = int(pidfile.read_text())
        b.cancel()
        self.assertTrue(done.wait(5))
        self.assertTrue(b.cancelled)
        for _ in range(50):
            if not alive(child):
                break
            done.wait(0.05)
        else:
            self.fail("the child process survived")


if __name__ == "__main__":
    unittest.main()
