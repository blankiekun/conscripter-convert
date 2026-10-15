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


FFMPEG_ENCODERS = ("libx264", "aac", "libvorbis")


def ffmpeg_missing_encoders(ffmpeg: Path) -> list[str]:
    """The encoders the converter uses that this ffmpeg lacks."""
    try:
        out = subprocess.run([str(ffmpeg), "-hide_banner", "-encoders"], capture_output=True, encoding="utf-8", errors="replace",
                             timeout=20, **_no_window()).stdout
    except (OSError, subprocess.SubprocessError):
        return list(FFMPEG_ENCODERS)
    names = {line.split()[1] for line in out.splitlines() if len(line.split()) > 1}
    return [e for e in FFMPEG_ENCODERS if e not in names]


def _no_window() -> dict:
    return {"creationflags": subprocess.CREATE_NO_WINDOW} if os.name == "nt" else {}


class Build:
    """One converter run. `on_line` gets each line of output and `on_exit`
    the exit code, both from a reader thread."""

    def __init__(self, converter: Path, args: list[str], on_line: Callable[[str], None],
                 on_exit: Callable[[int], None], ffmpeg_dir: Optional[Path] = None):
        env = dict(os.environ)
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
