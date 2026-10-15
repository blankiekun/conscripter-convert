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

import engine


@dataclass(frozen=True)
class Option:
    """An advanced setting, shown under "Advanced options": a converter
    flag. A "bool" adds `flag` when on; "text", "int" and "number" add
    `flag` and the value, and are left out when empty (unless `required`);
    a "choice" (one of `choices`) adds them unless it is the default. An
    option with `needs` is used only when that "bool" option is on, and
    `check(value, values)` returns a problem with the value, or None."""
    key: str
    label: str
    flag: str
    kind: str = "bool"  # "bool", "text", "int", "number" or "choice"
    default: object = False
    help: str = ""
    required: bool = False
    needs: str = ""
    low: int = 1  # an "int" option's range
    high: int = 1_000_000
    choices: tuple[str, ...] = ()
    check: Optional[Callable[[str, dict], Optional[str]]] = None


def _title(default: str) -> Option:
    return Option("title", "Window title", "--title", "text", default, "The game window's caption.")


def _is_number(v: str) -> bool:
    try:
        float(v)
        return True
    except ValueError:
        return False


def _audio_quality_check(v: str, values: dict) -> Optional[str]:
    low, high = (6, 256) if values.get("audio_codec") == "opus" else (-1, 10)
    if _is_number(v) and not low <= float(v) <= high:
        return f"from {low} to {high} for {values.get('audio_codec', 'vorbis').capitalize()}."
    return None


# Every convert command takes these (crates/conv-cli/src/build.rs, AudioArgs).
AUDIO = (
    Option("audio_codec", "Voices and sound effects", "--audio-codec", "choice", "vorbis",
           "vorbis: the game's own sounds, as they are. opus: re-encoded with ffmpeg, about half the size "
           "(the voices are most of a game's sound); needs CONscripter 0.21.0 or later. Music stays Vorbis.",
           choices=("vorbis", "opus")),
    Option("audio_quality", "Their quality", "--audio-quality", "number", "",
           "Opus: kbit/s per channel, 6-256 (empty: 32, about Vorbis's quality at half the size). "
           "Vorbis: -1 to 10 (ffmpeg's -q:a), re-encoding the game's sounds. "
           "Empty with Vorbis: the game's own sounds.",
           check=_audio_quality_check),
)

LOOSE = Option("loose", "Loose files (no .nsa archives)", "--loose",
               help="Keep the pictures and sounds as files in folders instead of packing them into arc.nsa.")
DRY_RUN = Option("dry_run", "Script only (dry run)", "--dry-run",
                 help="Write 0.txt only, no pictures or sounds: a quick check that the scripts convert.")


@dataclass(frozen=True)
class Game:
    key: str
    name: str  # shown to the user
    engine: str
    markers: tuple[str, ...]  # relative paths that must exist (a trailing / = a folder)
    command: Callable[[Path, Path, Path], list[str]]  # (game, output, font) -> the arguments before the options
    font: str  # recommended bundled font file
    output_name: str  # default output folder name
    needs_ffmpeg: bool
    size_gb: float  # about how big the finished build is
    doc: str  # the game's document in the repository
    check: Optional[Callable[[Path], tuple[list[str], list[str]]]] = None  # -> (problems, warnings)
    options: tuple[Option, ...] = ()  # the advanced settings; their defaults are the recommended ones
    icon: Optional[Callable[[Path], Optional[bytes]]] = None  # (game) -> its icon, an .ico or PNG file's bytes

    def defaults(self) -> dict[str, object]:
        return {o.key: o.default for o in self.options}

    def args(self, game: Path, output: Path, font: Path, values: Optional[dict] = None) -> list[str]:
        """The whole converter command line, with `values` (the defaults
        when None) for the options."""
        values = {**self.defaults(), **(values or {})}
        args = self.command(game, output, font)
        for o in self.options:
            v = values[o.key]
            if o.needs and not values[o.needs]:
                continue
            if o.kind == "bool":
                if v:
                    args.append(o.flag)
            elif o.kind == "choice":
                if v != o.default:
                    args += [o.flag, str(v)]
            elif str(v).strip():
                args += [o.flag, str(v).strip()]
        return args

    def needs_ffmpeg_with(self, values: dict) -> bool:
        """Whether the build runs ffmpeg with these options (re-encoding
        the voices needs it even where the game's own build doesn't)."""
        values = {**self.defaults(), **values}
        return self.needs_ffmpeg or values.get("audio_codec") == "opus" or bool(str(values.get("audio_quality", "")).strip())

    def option_problems(self, values: dict) -> list[str]:
        values = {**self.defaults(), **values}
        out = []
        for o in self.options:
            v = str(values[o.key]).strip()
            if o.kind == "bool" or (o.needs and not values[o.needs]):
                continue
            if o.required and not v:
                out.append(f"{o.label}: needs a value.")
            elif o.kind == "int" and v and not (v.isdigit() and o.low <= int(v) <= o.high):
                out.append(f"{o.label}: must be a whole number from {o.low} to {o.high}.")
            elif o.kind == "number" and v and not _is_number(v):
                out.append(f"{o.label}: must be a number.")
            elif o.check and v and (problem := o.check(v, values)):
                out.append(f"{o.label}: {problem}")
        return out


@dataclass
class Detection:
    game: Game
    root: Path  # the folder the converter is given
    problems: list[str] = field(default_factory=list)  # the build can't start
    warnings: list[str] = field(default_factory=list)  # it can, but tell the user


def _read(path: Path) -> Optional[bytes]:
    try:
        return path.read_bytes()
    except OSError:
        return None


def _has(root: Path, marker: str) -> bool:
    p = root / marker.rstrip("/")
    return p.is_dir() if marker.endswith("/") else p.is_file()


# --- 最果てのイマ COMPLETE (Xuse) ---------------------------------------

def _saiima_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/saihate-no-ima.md, "Build"
    return ["convert-game", str(game), "-o", str(output), "--font", str(font)]


def _saiima_check(root: Path) -> tuple[list[str], list[str]]:
    cds = list((root / "HD" / "CDFiles").glob("*.cd"))
    if len(cds) < 100:
        return [f"HD/CDFiles has {len(cds)} scripts (.cd), the full game has 172: is the install complete?"], []
    return [], []


# --- 装甲悪鬼村正 (Nitroplus NSS) -----------------------------------------

def _muramasa_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/muramasa.md: the new game starts at ma00_000.nss, and
    # --chain 2000 follows the story through every file (the options).
    return ["convert-nss", str(game), "-o", str(output), "--font", str(font)]


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
    # docs/games/oretsuba.md, "Status": all scenes.
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
        size_gb=1.5,
        doc="docs/games/saihate-no-ima.md",
        check=_saiima_check,
        icon=lambda root: engine.exe_icon(root / "Farthest2015.exe"),
        options=(
            _title("Saihate no Ima COMPLETE"),
            Option("webp", "WebP pictures (half the size, or less with a quality)", "--webp",
                   help="Store pictures as WebP instead of PNG. Plays in the web build of CONscripter, "
                        "and in a desktop build only if its SDL_image reads WebP."),
            Option("webp_quality", "WebP quality", "--webp-quality", "int", "",
                   "0-100, as in image editors: lossy WebP at that quality, far smaller (80: about a quarter "
                   "of near-lossless, and hard to tell apart). Empty: near-lossless.",
                   needs="webp", low=0, high=100),
            *AUDIO,
            LOOSE,
        ),
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
        size_gb=3.7,
        doc="docs/games/muramasa.md",
        icon=lambda root: engine.exe_icon(root / "Muramasa.exe"),
        options=(
            _title("装甲悪鬼村正"),
            Option("file", "Start at the script", "--file", "text", "ma00_000.nss",
                   "The story file the build starts at. A new game starts at ma00_000.nss.", required=True),
            Option("chain", "Story files to follow", "--chain", "int", "2000",
                   "How many story files to follow from the start file; 2000 is the whole story. "
                   "Empty: the start file's scene only."),
            *AUDIO,
            LOOSE,
            DRY_RUN,
        ),
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
        size_gb=2.3,
        doc="docs/games/oretsuba.md",
        check=_oretsuba_check,
        icon=lambda root: _read(root / "sce_sys" / "icon0.png"),
        options=(
            _title("俺たちに翼はない"),
            Option("scenes", "Only these scenes", "--scenes", "text", "",
                   "Scene names such as S01_01A, separated by commas, played in that order. "
                   "Empty: every scene, with the menus and extras."),
            *AUDIO,
            LOOSE,
            DRY_RUN,
        ),
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
