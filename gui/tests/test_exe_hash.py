"""The executable's sha256 check, with a tiny file and a patched table."""

import dataclasses
import hashlib
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import games  # noqa: E402

DATA = b"tiny exe"


def run(files: dict[str, bytes], hashes: dict[str, str]) -> games.Detection:
    game = dataclasses.replace(games.GAMES[0], exe="Game.EXE", hashes=hashes)
    with tempfile.TemporaryDirectory() as t:
        for name, data in files.items():
            (Path(t) / name).write_bytes(data)
        d = games.Detection(game, Path(t))
        games.check_exe(game, Path(t), d)
    return d


def test_matched():
    d = run({"game.exe": DATA}, {hashlib.sha256(DATA).hexdigest(): "Game.exe 1.01"})
    assert d.version == "Game.exe 1.01" and not d.warnings


def test_differs():
    d = run({"game.exe": b"other"}, {hashlib.sha256(DATA).hexdigest(): "Game.exe 1.01"})
    assert not d.version and len(d.warnings) == 1
    assert "not the build we tested" in d.warnings[0] and "game.exe" in d.warnings[0]


def test_missing():
    d = run({}, {hashlib.sha256(DATA).hexdigest(): "x"})
    assert not d.version and not d.warnings
