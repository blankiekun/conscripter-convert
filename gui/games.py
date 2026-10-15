"""The games the converter supports, how to recognise each one from its
folder, and the converter command with the recommended settings.

One `Game` entry per game. To add a game: write its `markers` (files and
folders that must all exist under the game folder), an optional `check`
for problems the markers can't see, and `command` from the game's
document in docs/games/. `detect()` tries every entry on the folder the
user dropped and on the folders up to two levels inside it, so dropping
the folder above the game data also works.
"""

from __future__ import annotations

import os
import struct
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Optional


@dataclass(frozen=True)
class Game:
    key: str
    name: str  # shown to the user
    engine: str
    markers: tuple[str, ...]  # relative paths that must exist (a trailing / = a folder)
    command: Callable[[Path, Path, Path], list[str]]  # (game, output, font) -> converter arguments
    font: str  # recommended bundled font file
    output_name: str  # default output folder name
    needs_ffmpeg: bool
    size_gb: float  # about how much the build writes
    doc: str  # the game's document in the repository
    check: Optional[Callable[[Path], tuple[list[str], list[str]]]] = None  # -> (problems, warnings)


@dataclass
class Detection:
    game: Game
    root: Path  # the folder the converter is given
    problems: list[str] = field(default_factory=list)  # the build can't start
    warnings: list[str] = field(default_factory=list)  # it can, but tell the user


def _has(root: Path, marker: str) -> bool:
    p = root / marker.rstrip("/")
    return p.is_dir() if marker.endswith("/") else p.is_file()


# --- 最果てのイマ COMPLETE (Xuse) ---------------------------------------

def _saiima_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/saihate-no-ima.md, "Build"
    return ["convert-game", str(game), "-o", str(output), "--font", str(font),
            "--title", "Saihate no Ima COMPLETE"]


def _saiima_check(root: Path) -> tuple[list[str], list[str]]:
    cds = list((root / "HD" / "CDFiles").glob("*.cd"))
    if len(cds) < 100:
        return [f"HD/CDFiles has {len(cds)} scripts (.cd), the full game has 172: is the install complete?"], []
    return [], []


# --- 装甲悪鬼村正 (Nitroplus NSS) -----------------------------------------

def _muramasa_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/muramasa.md: the new game starts at ma00_000.nss, and
    # --chain 2000 follows the story through every file.
    return ["convert-nss", str(game), "--file", "ma00_000.nss", "--chain", "2000",
            "-o", str(output), "--font", str(font), "--title", "装甲悪鬼村正"]


# --- 俺たちに翼はない (PS Vita, MAGES TaskList) -----------------------------

ORETSUBA_TITLE_ID = "PCSG00299"


def read_sfo(path: Path) -> dict[str, object]:
    """The keys of a PlayStation param.sfo (strings and integers)."""
    data = path.read_bytes()
    magic, _version, keys, values, count = struct.unpack_from("<4sIIII", data, 0)
    if magic != b"\0PSF":
        raise ValueError("not a param.sfo")
    out: dict[str, object] = {}
    for i in range(count):
        key_off, fmt, length, _max, value_off = struct.unpack_from("<HHIII", data, 20 + 16 * i)
        start = keys + key_off
        key = data[start:data.index(b"\0", start)].decode("utf-8", "replace")
        raw = data[values + value_off:values + value_off + length]
        if fmt == 0x0404:
            out[key] = struct.unpack("<I", raw[:4])[0]
        else:
            out[key] = raw.rstrip(b"\0").decode("utf-8", "replace")
    return out


def _oretsuba_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/oretsuba.md, "Status": all scenes, the default title.
    return ["mages", "convert", str(game), "-o", str(output), "--font", str(font)]


def _oretsuba_check(root: Path) -> tuple[list[str], list[str]]:
    problems: list[str] = []
    warnings: list[str] = []
    with open(root / "media" / "SCRIPT.cpk", "rb") as f:
        if f.read(4) != b"CPK ":
            problems.append("media/SCRIPT.cpk is still encrypted: give the decrypted game folder "
                            "(docs/games/oretsuba.md, 'Decrypting the package').")
    try:
        title_id = read_sfo(root / "sce_sys" / "param.sfo").get("TITLE_ID")
    except (OSError, ValueError, struct.error):
        title_id = None
    if title_id != ORETSUBA_TITLE_ID:
        warnings.append(f"Title ID {title_id or 'unknown'}: only {ORETSUBA_TITLE_ID} has been tested.")
    return problems, warnings


GAMES: tuple[Game, ...] = (
    Game(
        key="saihate-no-ima",
        name="最果てのイマ COMPLETE",
        engine="Xuse",
        markers=("Farthest2015.exe", "HD/CDFiles/", "CD/"),
        command=_saiima_command,
        font="ipag.ttf",
        output_name="saihate-no-ima-cons",
        needs_ffmpeg=True,  # movies and the staff roll become MP4s
        size_gb=2.0,
        doc="docs/games/saihate-no-ima.md",
        check=_saiima_check,
    ),
    Game(
        key="muramasa",
        name="装甲悪鬼村正",
        engine="Nitroplus NSS",
        markers=("Muramasa.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"),
        command=_muramasa_command,
        font="ipam.ttf",  # the game's text is in a Mincho face
        output_name="muramasa-cons",
        needs_ffmpeg=False,
        size_gb=3.5,
        doc="docs/games/muramasa.md",
    ),
    Game(
        key="oretsuba",
        name="俺たちに翼はない (PS Vita)",
        engine="MAGES TaskList",
        markers=("media/SCRIPT.cpk", "media/CG.cpk", "sce_sys/param.sfo"),
        command=_oretsuba_command,
        font="ipag.ttf",
        output_name="oretsuba-cons",
        needs_ffmpeg=True,  # sounds become Ogg, movies MP4
        size_gb=2.0,
        doc="docs/games/oretsuba.md",
        check=_oretsuba_check,
    ),
)


def _candidates(folder: Path, depth: int = 2):
    yield folder
    if depth == 0:
        return
    try:
        children = sorted(p for p in folder.iterdir() if p.is_dir() and not p.name.startswith("."))
    except OSError:
        return
    for child in children:
        yield from _candidates(child, depth - 1)


def detect(folder: Path | str) -> list[Detection]:
    """Every supported game found in `folder` or up to two levels inside it."""
    found: list[Detection] = []
    for root in _candidates(Path(folder)):
        for game in GAMES:
            if all(_has(root, m) for m in game.markers):
                d = Detection(game, root)
                if game.check:
                    try:
                        d.problems, d.warnings = game.check(root)
                    except OSError as e:
                        d.problems = [f"Can't read the game files: {e}"]
                found.append(d)
    found.sort(key=lambda d: bool(d.problems))  # buildable ones first
    return found


def default_output(detection: Detection) -> Path:
    """A new folder beside the game folder, not yet taken."""
    base = detection.root.parent / detection.game.output_name
    out, n = base, 2
    while out.exists() and any(out.iterdir()) and not is_previous_build(out):
        out = base.with_name(f"{base.name}-{n}")
        n += 1
    return out


def is_previous_build(folder: Path) -> bool:
    return (folder / "0.txt").is_file()


def output_problem(game_root: Path, output: Path) -> Optional[str]:
    """Why `output` can't be used at all (None if it can)."""
    g, o = os.path.realpath(game_root), os.path.realpath(output)
    if o == g:
        return "The output folder is the game folder."
    if o.startswith(g.rstrip(os.sep) + os.sep):
        return "The output folder is inside the game folder."
    if g.startswith(o.rstrip(os.sep) + os.sep):
        return "The game folder is inside the output folder."
    if output.exists() and not output.is_dir():
        return "The output path is a file."
    return None
