"""The games the converter supports, how to recognise each one from its
folder, and the converter command with the recommended settings.

One `Game` entry per game. To add a game: write its `markers` (files and
folders that must all exist under the game folder), an optional `check`
for problems the markers can't see, and `command` from the game's
document in docs/games/. `detect()` tries every entry on the folder the
user dropped, on the folders up to three levels inside it and on the two
above it, so dropping the folder above the game data, or one inside it,
also works. Files match in any case, as on Windows.
"""

from __future__ import annotations

import hashlib
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
    wide: bool = False  # the app gives it a whole row (long text); the others go two to a row
    # The app shows a slider over these (value, label) pairs, given the
    # other values; "" is the converter's default.
    steps: Optional[Callable[[dict], tuple[tuple[str, str], ...]]] = None


def _title(default: str) -> Option:
    return Option("title", "Window title", "--title", "text", default, "The title on the game's window.", wide=True)


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


def _audio_quality_steps(values: dict) -> tuple[tuple[str, str], ...]:
    if values.get("audio_codec") == "opus":
        return tuple(("", "32 kbit/s (default)") if k == 32 else (str(k), f"{k} kbit/s")
                     for k in (6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256))
    return (("", "original"), *((str(q), str(q)) for q in range(-1, 11)))


# Every convert command takes these (crates/conv-cli/src/build.rs, AudioArgs).
AUDIO = (
    Option("audio_codec", "Voices and sound effects", "--audio-codec", "choice", "vorbis",
           "vorbis keeps the game's sounds as they are. opus re-encodes them with ffmpeg at about half the size "
           "(voices are most of a game's sound) and needs CONscripter 0.21.0 or later. Music stays Vorbis.",
           choices=("vorbis", "opus")),
    Option("audio_quality", "Quality", "--audio-quality", "number", "",
           "Opus: kbit/s per channel, 6-256. The default, 32, sounds about as good as Vorbis at half the size. "
           "Vorbis: -1 to 10 (ffmpeg's -q:a) re-encodes the sounds; \"original\" keeps the game's own.",
           check=_audio_quality_check, steps=_audio_quality_steps),
)

LOOSE = Option("loose", "Loose files (no .nsa archives)", "--loose",
               help="Keep the pictures and sounds as files in folders instead of packing them into arc.nsa.")
DRY_RUN = Option("dry_run", "Script only (dry run)", "--dry-run",
                 help="Write 0.txt only, no pictures or sounds: a quick check that the scripts convert.")

# Every convert command takes these too (build.rs, WebpArgs).
WEBP = (
    Option("webp", "WebP pictures", "--webp",
           help="Store pictures as WebP instead of PNG: about half the size, and smaller still with a WebP quality. "
                "Works in CONscripter's web build, and on the desktop only if its SDL_image reads WebP."),
    Option("webp_quality", "WebP quality", "--webp-quality", "int", "",
           "0-100, as in image editors. Lossy and far smaller: at 80, about a quarter the size of "
           "near-lossless and hard to tell apart. Unset, pictures are near-lossless.",
           needs="webp", low=0, high=100,
           steps=lambda _: (*((str(q), str(q)) for q in range(0, 101, 5)), ("", "near-lossless"))),
)


def _common(title: str) -> tuple[Option, ...]:
    """The options every game has, in the order the app shows them; a
    game's own follow."""
    return (_title(title), *WEBP, *AUDIO, LOOSE, DRY_RUN)


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
    exe: str = ""  # the executable whose sha256 is checked against `hashes` (docs/games/*.md, "Executables")
    hashes: dict[str, str] = field(default_factory=dict)  # sha256 -> the build's label; empty = no check
    outdated: dict[str, str] = field(default_factory=dict)  # a `hashes` label -> the warning for that build
    movies: bool = False  # its movies are in a folder of their own (movie/, ムービー/), in or beside the game

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
    version: str = ""  # the known build the executable matched, if any


def _read(path: Path) -> Optional[bytes]:
    try:
        return path.read_bytes()
    except OSError:
        return None


def _find(root: Path, rel: str) -> Path:
    """`root`/`rel` with each part matched in any case (the converter's
    `find`); a part found in no case is kept as written."""
    at = root
    for part in filter(None, rel.split("/")):
        if not (at / part).exists():
            try:
                part = next((e.name for e in os.scandir(at) if e.name.lower() == part.lower()), part)
            except OSError:
                pass
        at = at / part
    return at


def check_exe(game: Game, root: Path, d: Detection) -> None:
    """Hash the game's executable once: a known build sets `d.version`, any
    other adds a warning. A missing or unreadable file says nothing."""
    exe = _find(root, game.exe) if game.exe and game.hashes else None
    if not exe or not exe.is_file():
        return
    h = hashlib.sha256()
    try:
        with exe.open("rb") as f:
            while chunk := f.read(1 << 20):
                h.update(chunk)
    except OSError:
        return
    digest = h.hexdigest()
    if digest in game.hashes:
        d.version = game.hashes[digest]
        if d.version in game.outdated:
            d.warnings.append(game.outdated[d.version])
    else:
        d.warnings.append(f"This {exe.name} is not the build we tested (sha256 {digest[:12]}…); "
                          "the conversion may still work.")


def _has(root: Path, marker: str) -> bool:
    p = _find(root, marker.rstrip("/"))
    return p.is_dir() if marker.endswith("/") else p.is_file()


MOVIE_FOLDERS = ("movie", "movies", "ムービー")


def find_movies(root: Path) -> Optional[Path]:
    """The movie folder in the game folder or beside it, as the converter
    looks for it (nitroplus/build.rs, `find_movies`)."""
    for d in (root, root.parent):
        try:
            for e in sorted(os.scandir(d), key=lambda e: e.name):
                if e.name.lower() in MOVIE_FOLDERS and e.is_dir():
                    return Path(e.path)
        except OSError:
            pass
    return None


# --- 最果てのイマ COMPLETE (Xuse) ---------------------------------------

def _saiima_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/saihate-no-ima.md, "Build"
    return ["convert-game", str(game), "-o", str(output), "--font", str(font)]


def _saiima_check(root: Path) -> tuple[list[str], list[str]]:
    cds = [p for p in _find(root, "HD/CDFiles").iterdir() if p.suffix.lower() == ".cd"]
    if len(cds) < 100:
        return [f"HD/CDFiles has {len(cds)} scripts (.cd), the full game has 172: is the install complete?"], []
    return [], []


# --- 装甲悪鬼村正 (Nitroplus NSS) -----------------------------------------

def _muramasa_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/muramasa.md: the new game starts at ma00_000.nss, and
    # --chain 2000 follows the story through every file (the options).
    return ["convert-nss", str(game), "-o", str(output), "--font", str(font)]


# --- 続・殺戮のジャンゴ (Nitroplus NSS) ---------------------------------------

def _django_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/django.md: without --file, the whole story.
    return ["convert-nss", str(game), "-o", str(output), "--font", str(font)]


def _django_check(root: Path) -> tuple[list[str], list[str]]:
    if not _find(root, "cg2.npa").exists():
        return [], ["cg2.npa is missing: the build expects the game updated to 1.01 "
                    "(its Django.exe, nss.npa and cg2.npa over the install)"]
    return [], []


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
    with open(_find(root, "media/SCRIPT.cpk"), "rb") as f:
        if f.read(4) != b"CPK ":
            problems.append("media/SCRIPT.cpk is still encrypted: give the decrypted game folder "
                            "(docs/games/oretsuba.md, 'Decrypting the package').")
    try:
        title_id = read_sfo(_find(root, "sce_sys/param.sfo")).get("TITLE_ID")
    except (OSError, ValueError, struct.error):
        title_id = None
    if title_id != ORETSUBA_TITLE_ID:
        warnings.append(f"Title ID {title_id or 'unknown'}: only {ORETSUBA_TITLE_ID} has been tested.")
    return problems, warnings


# --- 月姫 -A piece of blue glass moon- (Switch, TYPE-MOON) ------------------

def _tsukihime_command(game: Path, output: Path, font: Path) -> list[str]:
    # docs/games/tsukihime.md, "Build": the RomFS, its exe in ../exefs/main.
    return ["typemoon", "convert", str(game), "-o", str(output), "--font", str(font)]


def _tsukihime_check(root: Path) -> tuple[list[str], list[str]]:
    if not (root.parent / "exefs" / "main").is_file():
        return ["No exefs/main beside the RomFS folder: extract the ExeFS next to it "
                "(docs/games/tsukihime.md, 'Build')."], []
    return [], []


# --- The English editions, built on the Japanese game (`build --japanese`) --

def _build_command(game: Path, output: Path, font: Path) -> list[str]:
    # `build` adds --english <game> and the Japanese game's options (README).
    return ["build", str(game), "-o", str(output), "--font", str(font)]


def _japanese(name: str) -> Option:
    return Option("japanese", f"{name}'s folder", "--japanese", "text", "",
                  f"The English edition's text and pictures go into the Japanese game: give {name}'s folder.",
                  required=True, wide=True)


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
        exe="Farthest2015.exe",
        hashes={
            "efb7ec8e00338376d37822880e7fb73ae86eca03895bf55a6b6826b1dda6a37f": "Farthest2015.exe",
        },
        doc="docs/games/saihate-no-ima.md",
        check=_saiima_check,
        icon=lambda root: engine.exe_icon(_find(root, "Farthest2015.exe")),
        options=_common("Saihate no Ima COMPLETE"),
    ),
    Game(
        key="muramasa",
        name="装甲悪鬼村正",
        engine="Nitroplus NSS",
        markers=("Muramasa.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"),
        command=_muramasa_command,
        font="ipam.ttf",  # the game's text is in a Mincho face
        output_name="muramasa-cons",
        needs_ffmpeg=True,  # movies become MP4s (without ffmpeg the build logs "asset error" and leaves them out)
        size_gb=3.7,
        doc="docs/games/muramasa.md",
        icon=lambda root: engine.exe_icon(_find(root, "Muramasa.exe")),
        movies=True,
        options=(
            *_common("装甲悪鬼村正"),
            Option("file", "Start file", "--file", "text", "ma00_000.nss",
                   "The story file the build starts at. A new game starts at ma00_000.nss.", required=True),
            Option("chain", "Story files to follow", "--chain", "int", "2000",
                   "How many story files to follow from the start file; 2000 is the whole story. "
                   "Leave it empty for the start file's scene only."),
        ),
    ),
    Game(
        key="django",
        name="続・殺戮のジャンゴ",
        engine="Nitroplus NSS",
        markers=("Django.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"),
        command=_django_command,
        font="ipag.ttf",
        output_name="django-cons",
        needs_ffmpeg=True,  # movies become strips of frames
        size_gb=1.3,
        exe="Django.exe",
        hashes={
            "b40fb35cacc672c5ea377ab86e5358fbe85b2a05c30edf5981fd509c5fc390e7": "Django.exe 1.01",
            "d4bed22dd7062880b96476fe423f495cc1872bfb6a3b17c593c8313976040537": "Django.exe 1.00",
        },
        doc="docs/games/django.md",
        check=_django_check,
        icon=lambda root: engine.exe_icon(_find(root, "Django.exe")),
        movies=True,
        options=_common("続・殺戮のジャンゴ"),
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
        exe="eboot.bin",
        hashes={
            "56ebe9ed1a64d5e9e8d3af5aef0ce4ea21cf35ebffcf4c6cfae49dc6db343bd6": "eboot.bin (PCSG00299)",
        },
        doc="docs/games/oretsuba.md",
        check=_oretsuba_check,
        icon=lambda root: _read(_find(root, "sce_sys/icon0.png")),
        options=(
            *_common("俺たちに翼はない"),
            Option("scenes", "Only these scenes", "--scenes", "text", "",
                   "Scene names such as S01_01A, separated by commas, played in that order. "
                   "Leave it empty for every scene, with the menus and extras.", wide=True),
        ),
    ),
    Game(
        key="tsukihime",
        name="月姫 -A piece of blue glass moon- (Switch)",
        engine="TYPE-MOON",
        markers=("allscr.mrg", "script_text.mrg", "allui.mrg", "knife1.bfsar"),
        command=_tsukihime_command,
        font="ipag.ttf",
        output_name="tsukihime-cons",
        needs_ffmpeg=True,  # movies at 720p, sounds to Ogg
        size_gb=5.3,
        exe="../exefs/main",  # the NSO beside the RomFS
        hashes={
            "6e41c7094303a48b3223147b0fe52a2995c48f59af9d480b7e523ed4a1f103c7": "main 1.0.0 (base game)",
            "c128d79d3abd02b5581348618aa6403fdd651e77f4955f99ae43b34b90395faa": "main 1.0.2 (official update)",
            "d28cb7c2391e74b2e3e2add120c208933aa314724efd2e47bfdf6c5c13aa6174": "main fan patch 2023.09.05",
        },
        outdated={"main 1.0.0 (base game)": "This is the base game, 1.0.0: the converter reads 1.0.2's tables and "
                  "refuses it. Extract the RomFS and ExeFS of the base game with the 1.0.2 update applied "
                  "(docs/games/tsukihime.md, 'Build')."},
        doc="docs/games/tsukihime.md",
        check=_tsukihime_check,
        options=_common("月姫 -A piece of blue glass moon-"),
    ),
    Game(
        key="muramasa-en",
        name="Full Metal Daemon Muramasa (English edition)",
        engine="Nitroplus NSS + Diesel",
        markers=("muramasa_en.exe", "script.npk", "cg.npk", "font.npk"),
        command=_build_command,
        font="ipag.ttf",
        output_name="muramasa-en-cons",
        needs_ffmpeg=True,  # the Japanese game's movies become MP4s
        size_gb=3.7,
        doc="docs/games/muramasa-en.md",
        icon=lambda root: engine.exe_icon(_find(root, "muramasa_en.exe")),
        options=(*_common("Full Metal Daemon Muramasa"), _japanese("装甲悪鬼村正")),
    ),
    Game(
        key="tsukihime-en",
        name="Tsukihime -A piece of blue glass moon- (English fan patch)",
        engine="TYPE-MOON",
        markers=("allscr.mrg", "script_text.mrg", "allui.mrg", "knife1.bfsar", "imgui.ini"),
        command=_build_command,
        font="ipag.ttf",
        output_name="tsukihime-en-cons",
        needs_ffmpeg=True,  # movies at 720p, sounds to Ogg
        size_gb=5.3,
        doc="docs/games/tsukihime.md",
        options=(*_common("Tsukihime -A piece of blue glass moon-"), _japanese("月姫 1.0.2 (its RomFS)")),
    ),
)


def packaged_hint(path: Path) -> Optional[str]:
    """What to tell someone who dropped an undecrypted Switch/Vita package
    (no decryption tool is shipped: the keys stay the player's)."""
    ext = path.suffix.lower()
    if ext in (".nsp", ".xci"):
        return ("This is an encrypted Switch package. Extract its RomFS and ExeFS first with hactool and your "
                "console's prod.keys (docs/games/tsukihime.md, 'Build'), then give the RomFS folder.")
    if ext == ".pkg":
        return ("This is an encrypted PS Vita package. Decrypt and unpack it first with your console's keys "
                "(docs/games/oretsuba.md, 'Decrypting the package'), then give the game folder.")
    return None


def _candidates(folder: Path, depth: int = 3):
    yield folder
    if depth == 0:
        return
    try:
        children = sorted(p for p in folder.iterdir() if p.is_dir() and not p.name.startswith("."))
    except OSError:
        return
    for child in children:
        yield from _candidates(child, depth - 1)


def _detect_in(roots) -> list[Detection]:
    found: list[Detection] = []
    for root in roots:
        for game in GAMES:
            if all(_has(root, m) for m in game.markers):
                d = Detection(game, root)
                if game.check:
                    try:
                        d.problems, d.warnings = game.check(root)
                    except OSError as e:
                        d.problems = [f"Can't read the game files: {e}"]
                check_exe(game, root, d)
                if game.movies and not find_movies(root):
                    d.warnings.append("No movie folder (movie, movies or ムービー) in the game folder or beside it: "
                                      "the movies are left out.")
                found.append(d)
    return found


def detect(folder: Path | str) -> list[Detection]:
    """Every supported game found in `folder` or up to three levels inside
    it; failing that, in the two folders above it (one inside the game
    was dropped, such as HD/ or ムービー/)."""
    folder = Path(folder)
    found = _detect_in(_candidates(folder)) or _detect_in(list(folder.absolute().parents)[:2])
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
