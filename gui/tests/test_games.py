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
    touch(root, "movie/mvOP.ngs")


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
            # the fake exes are not a tested build
            self.assertEqual([w for w in found[0].warnings if "not the build" not in w], [])

    def test_steam_libraries(self):
        steam, other = self.dir / "Steam", self.dir / "Games2"
        (steam / "steamapps/common").mkdir(parents=True)
        make_muramasa(other / "steamapps/common/Muramasa")
        touch(steam, "steamapps/libraryfolders.vdf",
              f'"libraryfolders"\n{{\n\t"0"\n\t{{\n\t\t"path"\t\t"{steam}"\n\t}}\n'
              f'\t"1"\n\t{{\n\t\t"path"\t\t"{other}"\n\t}}\n}}\n'.encode())
        found = games.detect_steam([steam])
        self.assertEqual([(d.game.key, d.root) for d in found], [("muramasa", other / "steamapps/common/Muramasa")])
        self.assertEqual(games.detect_steam([self.dir / "none"]), [])

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
        make_muramasa(self.dir / "a" / "b" / "c" / "d")
        self.assertEqual(self.keys(self.dir), [])

    def test_folder_inside(self):
        make_saiima(self.dir)
        self.assertEqual(self.keys(self.dir / "HD" / "CDFiles"), [("saihate-no-ima", self.dir)])

    def test_any_case(self):
        make_muramasa(self.dir)
        (self.dir / "Muramasa.exe").rename(self.dir / "MURAMASA.EXE")
        (self.dir / "nss.npa").rename(self.dir / "NSS.NPA")
        self.assertEqual(self.keys(self.dir), [("muramasa", self.dir)])

    def test_movies(self):
        game = self.dir / "ゲームデータ"
        make_muramasa(game)
        (game / "movie" / "mvOP.ngs").unlink()
        (game / "movie").rmdir()
        (d,) = games.detect(game)
        self.assertIn("movie", d.warnings[0])
        (self.dir / "ムービー").mkdir()  # the disc's, beside the game
        (d,) = games.detect(game)
        self.assertEqual(d.warnings, [])

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

    def game(self, key):
        return next(g for g in games.GAMES if g.key == key)

    def args(self, key, values=None):
        return self.game(key).args(Path("G"), Path("O"), Path("F.ttf"), values)

    def test_saiima(self):
        self.assertEqual(self.args("saihate-no-ima"),
                         ["convert-game", "G", "-o", "O", "--font", "F.ttf", "--title", "Saihate no Ima COMPLETE"])

    def test_muramasa(self):
        # G has no movie folder: the build is asked to leave them out (detection warns)
        self.assertEqual(self.args("muramasa"), ["convert-nss", "G", "-o", "O", "--font", "F.ttf", "--no-movies",
                                                  "--title", "装甲悪鬼村正", "--file", "ma00_000.nss", "--chain", "2000"])
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "ムービー").mkdir()
            game = Path(d) / "G"
            self.assertNotIn("--no-movies", self.game("muramasa").args(game, Path("O"), Path("F.ttf")))

    def test_oretsuba(self):
        self.assertEqual(self.args("oretsuba"), ["mages", "convert", "G", "-o", "O", "--font", "F.ttf",
                                                  "--title", "俺たちに翼はない"])

    def test_options(self):
        self.assertEqual(self.args("saihate-no-ima", {"webp": True, "loose": True, "title": " T "})[-4:],
                         ["--title", "T", "--webp", "--loose"])
        self.assertEqual(self.args("saihate-no-ima", {"webp": True, "webp_quality": "80"})[-3:],
                         ["--webp", "--webp-quality", "80"])
        self.assertNotIn("--webp-quality", self.args("saihate-no-ima", {"webp_quality": "80"}))  # needs --webp
        self.assertEqual(self.args("muramasa", {"chain": "", "dry_run": True})[-3:],
                         ["--dry-run", "--file", "ma00_000.nss"])
        self.assertEqual(self.args("oretsuba", {"scenes": "S01_01A,S01_02A", "title": ""})[-2:],
                         ["--scenes", "S01_01A,S01_02A"])
        for key in ("muramasa", "oretsuba"):
            args = self.args(key, {"webp": True, "webp_quality": "80"})
            self.assertEqual(args[args.index("--webp"):args.index("--webp") + 3], ["--webp", "--webp-quality", "80"])
        self.assertEqual(self.args("saihate-no-ima", {"dry_run": True})[-1], "--dry-run")

    def test_same_options_everywhere(self):
        """Every game has the shared options first, in the same order."""
        common = ["title", "small", "webp", "webp_quality", "audio_codec", "audio_quality", "loose", "dry_run"]
        extras = {"saihate-no-ima": [], "muramasa": ["file", "chain"], "django": [], "oretsuba": ["scenes"], "tsukihime": [],
                  "muramasa-en": ["japanese"], "tsukihime-en": ["japanese"]}
        for g in games.GAMES:
            self.assertEqual([o.key for o in g.options], common + extras[g.key], g.key)

    def test_audio(self):
        for key in ("saihate-no-ima", "muramasa", "django", "oretsuba"):
            self.assertNotIn("--audio-codec", self.args(key))  # the default
            args = self.args(key, {"audio_codec": "opus", "audio_quality": "24"})
            self.assertEqual(args[args.index("--audio-codec"):args.index("--audio-codec") + 4],
                             ["--audio-codec", "opus", "--audio-quality", "24"])
        g = self.game("muramasa")
        self.assertTrue(g.needs_ffmpeg_with({}))
        self.assertTrue(g.needs_ffmpeg_with({"audio_codec": "opus"}))
        self.assertTrue(g.needs_ffmpeg_with({"audio_quality": "2"}))
        self.assertEqual(g.option_problems({"audio_codec": "opus", "audio_quality": "3"})[0][:8], "Quality:")
        self.assertEqual(g.option_problems({"audio_codec": "vorbis", "audio_quality": "-1"}), [])
        self.assertEqual(len(g.option_problems({"audio_codec": "vorbis", "audio_quality": "12"})), 1)
        self.assertEqual(len(g.option_problems({"audio_quality": "x"})), 1)
        self.assertEqual(g.option_problems({"small": True, "audio_quality": "24"}), [])  # Opus
        self.assertEqual(self.args("saihate-no-ima", {"small": True})[-1], "--small")

    def test_slider_steps(self):
        # Every step is a valid value, and the default is one of them.
        g = self.game("muramasa")
        for values in ({"webp": True}, {"audio_codec": "opus"}, {"audio_codec": "vorbis"}):
            for o in g.options:
                if o.steps:
                    steps = [v for v, _ in o.steps({**g.defaults(), **values})]
                    self.assertIn(o.default, steps)
                    for v in steps:
                        self.assertEqual(g.option_problems({**values, o.key: v}), [], (o.key, v))

    def test_option_problems(self):
        g = self.game("muramasa")
        self.assertEqual(g.option_problems(g.defaults()), [])
        self.assertEqual(len(g.option_problems({**g.defaults(), "file": " "})), 1)
        self.assertEqual(len(g.option_problems({**g.defaults(), "chain": "two"})), 1)
        self.assertEqual(len(g.option_problems({**g.defaults(), "chain": "0"})), 1)
        x = self.game("saihate-no-ima")
        self.assertEqual(x.option_problems({"webp": True, "webp_quality": "0"}), [])
        self.assertEqual(len(x.option_problems({"webp": True, "webp_quality": "101"})), 1)
        self.assertEqual(x.option_problems({"webp": False, "webp_quality": "101"}), [])  # unused

    def test_converter_accepts(self):
        """Each command parses with the converter's own --help (when built)."""
        conv = tools.find_converter()
        if not conv:
            self.skipTest("conscripter-convert not built")
        import subprocess
        for g in games.GAMES:
            on = {o.key: True if o.kind == "bool" else o.choices[-1] if o.kind == "choice"
                  else (o.default or str(max(o.low, 6))) for o in g.options}
            args = g.args(Path("G"), Path("O"), Path("F.ttf"), on)
            def help(args):
                sub = args[:2] if args[0] in ("mages", "typemoon") else args[:1]
                return subprocess.run([str(conv), *sub, "--help"], capture_output=True, encoding="utf-8", errors="replace").stdout
            out = help(args)
            if args[0] == "build":  # an English edition: build's --japanese, then the Japanese game's command
                out += help(self.game(g.key.removesuffix("-en")).args(Path("G"), Path("O"), Path("F.ttf")))
            for flag in (a for a in args if a.startswith("-")):
                self.assertIn(flag, out, f"{g.key}: {flag}")


class Written(unittest.TestCase):
    def test_counts_only_new_files(self):
        import time
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            touch(d, "old/a", b"x" * 100)
            os.utime(d / "old/a", (1, 1))
            since = time.time() - 2
            touch(d, "bg/b", b"x" * 30)
            touch(d, "arc.nsa", b"x" * 12)
            self.assertEqual(tools.written_bytes(d, since), 42)
            self.assertEqual(tools.written_bytes(d / "missing", since), 0)


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



class ChildEnv(unittest.TestCase):
    """A bundle's library path must not reach the programs it starts (a
    system ffmpeg failed to load with the bundle's fontconfig)."""

    def setUp(self):
        self.saved = dict(os.environ)
        sys.frozen = True

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.saved)
        del sys.frozen

    def test_drops_the_bundles_path(self):
        os.environ["LD_LIBRARY_PATH"] = "/tmp/_MEI123"
        os.environ.pop("LD_LIBRARY_PATH_ORIG", None)
        self.assertNotIn("LD_LIBRARY_PATH", tools.child_env())

    def test_restores_the_users_path(self):
        os.environ["LD_LIBRARY_PATH"] = "/tmp/_MEI123:/opt/lib"
        os.environ["LD_LIBRARY_PATH_ORIG"] = "/opt/lib"
        env = tools.child_env()
        self.assertEqual(env["LD_LIBRARY_PATH"], "/opt/lib")
        self.assertNotIn("LD_LIBRARY_PATH_ORIG", env)


if __name__ == "__main__":
    unittest.main()
