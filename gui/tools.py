"""Finding the converter, the fonts and ffmpeg, and running the converter
as a child process that can be cancelled with everything it started."""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import sys
import threading
from pathlib import Path
from typing import Callable, Optional

EXE = ".exe" if os.name == "nt" else ""
HERE = Path(__file__).resolve().parent


def bundle_dir() -> Path:
    """Where PyInstaller unpacked the app's own files (the source folder when
    run from source)."""
    return Path(getattr(sys, "_MEIPASS", HERE))


def app_dir() -> Path:
    """The folder the app's executable is in."""
    return Path(sys.executable).resolve().parent if getattr(sys, "frozen", False) else HERE


def find_converter() -> Optional[Path]:
    override = os.environ.get("CONSCRIPTER_CONVERT")  # for testing the app with another binary
    if override:
        return Path(override)
    name = "conscripter-convert" + EXE
    for p in (bundle_dir() / "bin" / name, app_dir() / name,
              HERE.parent / "target" / "release" / name):  # a source checkout
        if p.is_file():
            return p
    found = shutil.which("conscripter-convert")
    return Path(found) if found else None


def find_font(name: str) -> Optional[Path]:
    """A bundled font, else one installed on the system."""
    for d in (bundle_dir() / "fonts", app_dir() / "fonts", HERE / "fonts"):
        if (d / name).is_file():
            return d / name
    system = [Path("/usr/share/fonts"), Path("/usr/local/share/fonts"), Path.home() / ".local/share/fonts",
              Path.home() / "Library/Fonts", Path("/Library/Fonts")]
    if os.name == "nt":
        system = [Path(os.environ.get("WINDIR", r"C:\Windows")) / "Fonts",
                  Path(os.environ.get("LOCALAPPDATA", "")) / "Microsoft/Windows/Fonts"]
    for d in system:
        if d.is_dir():
            hit = next(d.rglob(name), None)
            if hit:
                return hit
    return None


def find_ffmpeg(extra_dir: Optional[Path] = None) -> Optional[Path]:
    for d in (extra_dir, app_dir()):
        if d and (d / ("ffmpeg" + EXE)).is_file():
            return d / ("ffmpeg" + EXE)
    found = shutil.which("ffmpeg")
    return Path(found) if found else None


FFMPEG_ENCODERS = ("libx264", "aac", "libvorbis")  # libopus too for --audio-codec opus


def ffmpeg_missing_encoders(ffmpeg: Path, encoders: tuple[str, ...] = FFMPEG_ENCODERS) -> list[str]:
    """The `encoders` (those the converter uses) that this ffmpeg lacks."""
    try:
        out = subprocess.run([str(ffmpeg), "-hide_banner", "-encoders"], capture_output=True, encoding="utf-8", errors="replace",
                             timeout=20, env=child_env(), **_no_window()).stdout
    except (OSError, subprocess.SubprocessError):
        return list(encoders)
    names = {line.split()[1] for line in out.splitlines() if len(line.split()) > 1}
    return [e for e in encoders if e not in names]


def child_env() -> dict:
    """The environment for the programs the app starts (the converter,
    ffmpeg). A one-file bundle on Linux points the loader at its own
    libraries (LD_LIBRARY_PATH): a system ffmpeg started with them can fail
    to load (an older fontconfig) and seem to have no encoders. The
    bootloader keeps the user's value in LD_LIBRARY_PATH_ORIG."""
    env = dict(os.environ)
    if getattr(sys, "frozen", False):
        for var in ("LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"):
            orig = env.pop(var + "_ORIG", None)
            if orig is not None:
                env[var] = orig
            else:
                env.pop(var, None)
    return env


def unbundle_dll_directory() -> None:
    """On Windows a bundle adds its folder to the DLL search path
    (SetDllDirectory), which the programs it starts inherit: undo it."""
    if os.name == "nt" and getattr(sys, "frozen", False):
        import ctypes
        ctypes.windll.kernel32.SetDllDirectoryW(None)


def _no_window() -> dict:
    return {"creationflags": subprocess.CREATE_NO_WINDOW} if os.name == "nt" else {}


def written_bytes(folder: Path, since: float) -> int:
    """The size of the files under `folder` changed at or after `since`
    (a time.time()): what this build has written so far, not the files an
    earlier build left there."""
    total = 0
    stack = [folder]
    while stack:
        try:
            entries = list(os.scandir(stack.pop()))
        except OSError:
            continue
        for e in entries:
            try:
                if e.is_dir(follow_symlinks=False):
                    stack.append(Path(e.path))
                else:
                    st = e.stat(follow_symlinks=False)
                    if st.st_mtime >= since:
                        total += st.st_size
            except OSError:  # deleted while we looked (the packer removes its folders)
                pass
    return total


class Build:
    """One converter run. `on_line` gets each line of output and `on_exit`
    the exit code, both from a reader thread."""

    def __init__(self, converter: Path, args: list[str], on_line: Callable[[str], None],
                 on_exit: Callable[[int], None], ffmpeg_dir: Optional[Path] = None):
        env = child_env()
        if ffmpeg_dir:  # the converter runs "ffmpeg" from PATH
            env["PATH"] = str(ffmpeg_dir) + os.pathsep + env.get("PATH", "")
        kwargs: dict = {}
        if os.name == "nt":
            kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP
        else:
            kwargs["start_new_session"] = True  # its own process group, ffmpeg children included
        self.cancelled = False
        self.proc = subprocess.Popen([str(converter), *args], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                     stdin=subprocess.DEVNULL, env=env, **kwargs)
        self._on_line, self._on_exit = on_line, on_exit
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        assert self.proc.stdout
        for raw in self.proc.stdout:
            self._on_line(raw.decode("utf-8", "replace").rstrip("\r\n"))
        self.proc.stdout.close()
        self._on_exit(self.proc.wait())

    def running(self) -> bool:
        return self.proc.poll() is None

    def cancel(self) -> None:
        """Kill the converter and the ffmpeg processes it started."""
        if not self.running():
            return
        self.cancelled = True
        if os.name == "nt":
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(self.proc.pid)], capture_output=True, **_no_window())
        else:
            try:
                os.killpg(self.proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
