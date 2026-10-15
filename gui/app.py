"""CONscripter-convert: drop a game folder, get a CONscripter game.

The window finds which supported game the folder holds (games.py), shows
the output folder and font it will use, and runs the conscripter-convert
binary with that game's recommended settings, showing its output live.

    python app.py                      the window
    python app.py --detect <folder>    print what would be built, no window
    python app.py --self-test <file>   write the bundle's state as JSON (CI)
"""

from __future__ import annotations

import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk
from tkinter.scrolledtext import ScrolledText
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
import games  # noqa: E402
import tools  # noqa: E402

try:
    from tkinterdnd2 import DND_FILES, TkinterDnD
except ImportError:  # no drag and drop; the folder button still works
    TkinterDnD = None

APP_TITLE = "CONscripter-convert"
ENGINE_URL = "https://github.com/blankiekun/CONscripter/releases"


class App:
    def __init__(self) -> None:
        self.root = TkinterDnD.Tk() if TkinterDnD else tk.Tk()
        self.root.title(APP_TITLE)
        self.root.minsize(640, 560)
        self.converter = tools.find_converter()
        self.detections: list[games.Detection] = []
        self.detection: Optional[games.Detection] = None
        self.folder: Optional[Path] = None
        self.detecting = False
        # The advanced options' values, per game (kept when switching games).
        self.option_vars: dict[str, dict[str, tk.Variable]] = {}
        self.option_widgets: list[tk.Widget] = []
        self.font: Optional[Path] = None
        self.font_chosen = False  # by the user, not the game's default
        self.ffmpeg_dir: Optional[Path] = None
        self.build: Optional[tools.Build] = None
        self.events: queue.Queue = queue.Queue()
        self.started = 0.0
        self.written = 0  # bytes this build has written so far (the most seen)
        self.expected = 0.0  # about how many it writes, 0 = unknown
        self.log_lines: list[str] = []
        self._layout()
        self._refresh()
        self.root.protocol("WM_DELETE_WINDOW", self._close)
        self.root.after(100, self._poll)

    # --- layout ---------------------------------------------------------

    def _layout(self) -> None:
        pad = {"padx": 12, "pady": 4}
        r = self.root
        r.columnconfigure(0, weight=1)

        self.drop = tk.Label(r, text="", relief="groove", bd=2, height=4, cursor="hand2",
                             font=("TkDefaultFont", 12))
        self.drop.grid(row=0, column=0, sticky="ew", padx=12, pady=(12, 4))
        self.drop.bind("<Button-1>", lambda e: self._choose_game())
        if TkinterDnD:
            self.drop.drop_target_register(DND_FILES)
            self.drop.dnd_bind("<<Drop>>", self._dropped)
            r.drop_target_register(DND_FILES)
            r.dnd_bind("<<Drop>>", self._dropped)

        info = ttk.Frame(r)
        info.grid(row=1, column=0, sticky="ew", **pad)
        info.columnconfigure(1, weight=1)
        ttk.Label(info, text="Game:").grid(row=0, column=0, sticky="w")
        self.game_var = tk.StringVar()
        self.game_box = ttk.Combobox(info, textvariable=self.game_var, state="readonly")
        self.game_box.bind("<<ComboboxSelected>>", lambda e: self._pick(self.game_box.current()))
        self.game_box.grid(row=0, column=1, columnspan=2, sticky="ew", pady=2)
        self.notes = ttk.Label(info, text="", foreground="#b05000", wraplength=580, justify="left")
        self.notes.grid(row=1, column=1, columnspan=2, sticky="w")

        ttk.Label(info, text="Output:").grid(row=2, column=0, sticky="w")
        self.out_var = tk.StringVar()
        self.out_var.trace_add("write", lambda *a: self._refresh())
        ttk.Entry(info, textvariable=self.out_var, width=40).grid(row=2, column=1, sticky="ew", pady=2)
        ttk.Button(info, text="Change…", command=self._choose_output).grid(row=2, column=2, padx=(6, 0))

        ttk.Label(info, text="Font:").grid(row=3, column=0, sticky="w")
        self.font_label = ttk.Label(info, text="", wraplength=480)
        self.font_label.grid(row=3, column=1, sticky="w", pady=2)
        ttk.Button(info, text="Change…", command=self._choose_font).grid(row=3, column=2, padx=(6, 0))

        ttk.Label(info, text="ffmpeg:").grid(row=4, column=0, sticky="w")
        self.ffmpeg_label = ttk.Label(info, text="", wraplength=480, justify="left")
        self.ffmpeg_label.grid(row=4, column=1, sticky="w", pady=2)
        self.ffmpeg_button = ttk.Button(info, text="Locate…", command=self._choose_ffmpeg)
        self.ffmpeg_button.grid(row=4, column=2, padx=(6, 0))

        self.adv_button = ttk.Button(info, text="▸ Advanced options", command=self._toggle_advanced)
        self.adv_button.grid(row=5, column=0, columnspan=2, sticky="w", pady=(6, 0))
        self.adv = ttk.Frame(info, padding=(16, 4, 0, 0))
        self.adv.columnconfigure(1, weight=1)
        self.adv.grid(row=6, column=0, columnspan=3, sticky="ew")
        self.adv.grid_remove()

        bar = ttk.Frame(r)
        bar.grid(row=2, column=0, sticky="ew", **pad)
        bar.columnconfigure(3, weight=1)
        self.start_button = ttk.Button(bar, text="Build", command=self._start)
        self.start_button.grid(row=0, column=0)
        self.cancel_button = ttk.Button(bar, text="Cancel", command=self._cancel)
        self.cancel_button.grid(row=0, column=1, padx=6)
        self.open_button = ttk.Button(bar, text="Open output folder", command=self._open_output)
        self.open_button.grid(row=0, column=2)
        self.progress = ttk.Progressbar(bar, mode="determinate", value=0)
        self.progress.grid(row=0, column=3, sticky="ew", padx=(12, 0))

        self.status = tk.Label(r, text="", anchor="w", justify="left", font=("TkDefaultFont", 11, "bold"))
        self.status.grid(row=3, column=0, sticky="ew", **pad)
        # Long paths wrap instead of widening the window.
        r.bind("<Configure>", lambda e: e.widget is r and self._wrap(e.width))
        self.game_box.config(width=40)

        self.log = ScrolledText(r, height=16, state="disabled", wrap="word", font=("TkFixedFont", 9))
        self.log.grid(row=4, column=0, sticky="nsew", padx=12, pady=(4, 12))
        r.rowconfigure(4, weight=1)

    def _wrap(self, width: int) -> None:
        self.status.config(wraplength=max(200, width - 30))
        self.notes.config(wraplength=max(200, width - 110))
        self.ffmpeg_label.config(wraplength=max(200, width - 200))
        for w in self.adv.winfo_children():
            if getattr(w, "wraps", False):
                w.config(wraplength=max(200, width - 80))

    # --- choosing -------------------------------------------------------

    def _dropped(self, event) -> None:
        paths = self.root.tk.splitlist(event.data)
        if paths and not self._busy() and not self.detecting:
            self._load(Path(paths[0]))

    def _choose_game(self) -> None:
        if self._busy() or self.detecting:
            return
        d = filedialog.askdirectory(title="The game folder")
        if d:
            self._load(Path(d))

    def _load(self, folder: Path) -> None:
        """Look for a game in `folder`, in a thread: on a network share
        reading the folders can take a while."""
        if folder.is_file():
            folder = folder.parent
        self.folder, self.detecting = folder, True
        self.detection, self.detections = None, []
        self.game_box.config(values=[])
        self.game_var.set("")
        self._build_options()
        self.root.config(cursor="watch")
        self._refresh()

        def run() -> None:
            try:
                found = games.detect(folder)
            except Exception as e:  # noqa: BLE001 - shown to the user
                found = e
            self.events.put(("detected", (folder, found)))
        threading.Thread(target=run, daemon=True).start()

    def _detected(self, folder: Path, found) -> None:
        if folder != self.folder:
            return  # an older drop
        self.detecting = False
        self.root.config(cursor="")
        self._clear_log()
        if isinstance(found, Exception):
            self._log(f"Could not read {folder}: {found}")
            found = []
        self.detections = found
        self.status.config(text="")
        if not self.detections:
            self.detection = None
            self.game_box.config(values=[])
            self.game_var.set("")
            self._log(f"No supported game found in {folder}.")
            self._log("Supported: " + ", ".join(g.name for g in games.GAMES) + ".")
            self._log("Choose the folder the game's files are in (see the README for each game's markers).")
        else:
            self.game_box.config(values=[f"{d.game.name}  —  {d.root}" for d in self.detections])
            self._pick(0)
            if len(self.detections) > 1:
                self._log(f"{len(self.detections)} games found; choose one under Game.")
        self._refresh()

    def _pick(self, i: int) -> None:
        self.detection = d = self.detections[i]
        self.game_box.current(i)
        self.out_var.set(str(games.default_output(d)))
        if not self.font_chosen:
            self.font = tools.find_font(d.game.font)
        self._build_options()
        self._refresh()

    # --- advanced options -----------------------------------------------

    def _toggle_advanced(self) -> None:
        if self.adv.winfo_ismapped():
            self.adv.grid_remove()
            self.adv_button.config(text="▸ Advanced options")
        else:
            self.adv.grid()
            self.adv_button.config(text="▾ Advanced options")

    def _values(self) -> dict:
        if not self.detection:
            return {}
        return {k: v.get() for k, v in self.option_vars.get(self.detection.game.key, {}).items()}

    def _build_options(self) -> None:
        """The detected game's options, from its `Game.options`."""
        for w in self.adv.winfo_children():
            w.destroy()
        self.option_widgets = []
        game = self.detection.game if self.detection else None
        if not game:
            ttk.Label(self.adv, text="Choose a game folder first.", foreground="#606060").grid(row=0, column=0, sticky="w")
            return
        if game.key not in self.option_vars:
            made: dict[str, tk.Variable] = {}
            for o in game.options:
                v = tk.BooleanVar(value=bool(o.default)) if o.kind == "bool" else tk.StringVar(value=str(o.default))
                v.trace_add("write", lambda *a: self._refresh())
                made[o.key] = v
            self.option_vars[game.key] = made
        values = self.option_vars[game.key]
        row = 0
        self.option_needs: list[tuple[tk.Widget, str]] = []
        for o in game.options:
            if o.kind == "bool":
                w = ttk.Checkbutton(self.adv, text=o.label, variable=values[o.key])
                w.grid(row=row, column=0, columnspan=2, sticky="w")
            else:
                ttk.Label(self.adv, text=o.label + ":").grid(row=row, column=0, sticky="w", padx=(0, 6))
                w = ttk.Entry(self.adv, textvariable=values[o.key], width=30)
                w.grid(row=row, column=1, sticky="ew", pady=1)
            self.option_widgets.append(w)
            if o.needs:
                self.option_needs.append((w, o.needs))
            hint = ttk.Label(self.adv, text=o.help, foreground="#606060", wraplength=560, justify="left")
            hint.wraps = True
            hint.grid(row=row + 1, column=0, columnspan=2, sticky="w", padx=(20, 0), pady=(0, 4))
            row += 2
        reset = ttk.Button(self.adv, text="Recommended settings", command=self._reset_options)
        reset.grid(row=row, column=0, sticky="w", pady=(2, 4))
        self.option_widgets.append(reset)
        self.command_label = ttk.Label(self.adv, text="", foreground="#404040", wraplength=560, justify="left",
                                       font=("TkFixedFont", 9))
        self.command_label.wraps = True
        self.command_label.grid(row=row + 1, column=0, columnspan=2, sticky="w")
        self._wrap(self.root.winfo_width())

    def _reset_options(self) -> None:
        if self.detection:
            for k, v in self.detection.game.defaults().items():
                self.option_vars[self.detection.game.key][k].set(v)

    def _choose_output(self) -> None:
        d = filedialog.askdirectory(title="Where to put the converted game")
        if d:
            out = Path(d)
            # A non-empty folder gets the game in a new folder inside it.
            if self.detection and any(out.iterdir()) and not games.is_previous_build(out):
                out = out / self.detection.game.output_name
            self.out_var.set(str(out))

    def _choose_font(self) -> None:
        f = filedialog.askopenfilename(title="A Japanese font",
                                       filetypes=[("Fonts", "*.ttf *.otf *.ttc"), ("All files", "*")])
        if f:
            self.font, self.font_chosen = Path(f), True
            self._refresh()

    def _choose_ffmpeg(self) -> None:
        f = filedialog.askopenfilename(title="ffmpeg" + tools.EXE)
        if f:
            self.ffmpeg_dir = Path(f).parent
            self._refresh()

    # --- state ----------------------------------------------------------

    def _busy(self) -> bool:
        return bool(self.build and self.build.running())

    def _ffmpeg_state(self) -> tuple[bool, str]:
        if not (self.detection and self.detection.game.needs_ffmpeg):
            return True, "not needed for this game"
        ff = tools.find_ffmpeg(self.ffmpeg_dir)
        if not ff:
            return False, ("NOT FOUND. This game needs ffmpeg (with libx264): install it and put it on PATH, "
                           "or click Locate….")
        missing = getattr(self, "_ffmpeg_checked", {}).get(ff)
        if missing is None:
            missing = tools.ffmpeg_missing_encoders(ff)
            self._ffmpeg_checked = {ff: missing}
        if missing:
            return False, f"{ff} lacks the encoders {', '.join(missing)}: install a full ffmpeg build."
        return True, str(ff)

    def _refresh(self) -> None:
        if not hasattr(self, "start_button"):
            return
        busy = self._busy()
        d = self.detection
        how = "drop the game folder here, or click to choose it" if TkinterDnD else "click to choose the game folder"
        if self.detecting:
            self.drop.config(text=f"Detecting the game…\n{self.folder}", fg="#404040")
        elif d:
            self.drop.config(text=f"Detected: {d.game.name}  ({d.game.engine})\n{d.root}\n"
                                  f"(another game: {how})", fg="#006000" if not d.problems else "#b05000")
        elif self.folder:
            self.drop.config(text=f"No supported game found in\n{self.folder}\n({how})", fg="#c00000")
        else:
            self.drop.config(text=how[0].upper() + how[1:], fg="black")
        notes = []
        ready = True
        if not self.converter:
            notes.append("conscripter-convert was not found (next to this app or on PATH).")
            ready = False
        if d:
            notes += [f"✗ {p}" for p in d.problems] + [f"! {w}" for w in d.warnings]
            ready &= not d.problems
            option_problems = d.game.option_problems(self._values())
            notes += [f"✗ {p}" for p in option_problems]
            ready &= not option_problems
            out = Path(self.out_var.get().strip()) if self.out_var.get().strip() else None
            if not out:
                ready = False
            else:
                problem = games.output_problem(d.root, out)
                if problem:
                    notes.append(f"✗ {problem}")
                    ready = False
        else:
            ready = False
        self.notes.config(text="\n".join(notes))
        if not busy and not self.status.cget("text").startswith(("✓", "✗")):
            self.status.config(fg="black", text=(
                f"Ready: {d.game.name} with {self._settings_name()}; "
                f"{self._size_text()}." if d and ready else ""))
        self.font_label.config(text=font_name(self.font) if self.font else "none found: choose a Japanese TTF (IPA Gothic)")
        ready &= self.font is not None
        ok, text = self._ffmpeg_state() if d else (True, "")
        self.ffmpeg_label.config(text=text, foreground="black" if ok else "#c00000")
        ready &= ok
        self.start_button.config(state="normal" if ready and not busy else "disabled")
        self.cancel_button.config(state="normal" if busy else "disabled")
        self.open_button.config(state="normal" if d and Path(self.out_var.get()).is_dir() and not busy else "disabled")
        values = self._values()
        for w in self.option_widgets:
            off = busy or any(w is n and not values.get(k) for n, k in getattr(self, "option_needs", []))
            w.config(state="disabled" if off else "normal")
        if d and getattr(self, "command_label", None) and self.command_label.winfo_exists():
            out = Path(self.out_var.get().strip() or d.game.output_name)
            args = d.game.args(d.root, out, self.font or Path(d.game.font), self._values())
            self.command_label.config(text="conscripter-convert " + " ".join(_quote(a) for a in args))

    def _settings_name(self) -> str:
        if self.detection and self._values() != self.detection.game.defaults():
            return "your settings (Advanced options)"
        return f"its recommended settings ({self.detection.game.doc})"

    def _peak_gb(self) -> float:
        """The most disk the build uses: packing writes the archives while
        the loose files are still there."""
        v = self._values()
        if v.get("dry_run"):
            return 0.0
        size = self.detection.game.size_gb * _webp_factor(v)
        return size if v.get("loose") else size * 2

    def _size_text(self) -> str:
        v = self._values()
        if v.get("dry_run"):
            return "writes the script only"
        size = self.detection.game.size_gb * _webp_factor(v)
        if v.get("loose"):
            return f"writes about {size:.1f} GB"
        return f"about {size:.1f} GB when done, {self._peak_gb():.1f} GB free needed while it packs"

    # --- building -------------------------------------------------------

    def _confirm_output(self, out: Path) -> bool:
        if out.exists() and any(out.iterdir()):
            if games.is_previous_build(out):
                if not messagebox.askyesno(APP_TITLE, (
                        f"{out} holds an earlier build.\n\nBuild over it? Its files are replaced; "
                        "saves made with the earlier build may not load in the new one."), default="no"):
                    return False
            elif not messagebox.askyesno(APP_TITLE, (
                    f"{out} is not empty.\n\nThe converter writes into it and deletes the folders it "
                    "packs into archives (such as image/ or sound/), whatever they hold. "
                    "Use it anyway?"), icon="warning", default="no"):
                return False
        need = self._peak_gb()
        if not need:
            return True
        probe = out
        while not probe.exists() and probe.parent != probe:
            probe = probe.parent
        try:
            free = shutil.disk_usage(probe).free / 1e9
        except OSError:
            return True
        if free < need * 1.1:
            return messagebox.askyesno(APP_TITLE, (
                f"The build writes about {need:.1f} GB and the disk has {free:.1f} GB free.\n\n"
                "Start anyway?"), icon="warning", default="no")
        return True

    def _start(self) -> None:
        d = self.detection
        out = Path(self.out_var.get().strip()).expanduser()
        if not d or not self.font or not self.converter or not self._confirm_output(out):
            return
        args = d.game.args(d.root, out, self.font, self._values())
        self._clear_log()
        self._log("$ conscripter-convert " + " ".join(_quote(a) for a in args))
        ff = tools.find_ffmpeg(self.ffmpeg_dir) if d.game.needs_ffmpeg else None
        try:
            self.build = tools.Build(self.converter, args, lambda s: self.events.put(("line", s)),
                                     lambda c: self.events.put(("exit", c)), ff.parent if ff else None)
        except OSError as e:
            self._finish(f"Could not start the converter: {e}", ok=False)
            return
        self.started = time.monotonic()
        self.written, self.expected = 0, self._peak_gb() * 1e9
        if self.expected:
            # The bar follows the bytes written: the converter prints
            # little until the end.
            self.progress.config(mode="determinate", value=0)
            self._watch_output(self.build, out, time.time() - 2)
        else:
            self.progress.config(mode="indeterminate")
            self.progress.start(12)
        self.status.config(text=f"Building {d.game.name}… (this takes several minutes)", fg="black")
        self._refresh()

    def _watch_output(self, build: tools.Build, out: Path, since: float) -> None:
        def run() -> None:
            while build.running():
                self.events.put(("written", tools.written_bytes(out, since)))
                time.sleep(1.5)
        threading.Thread(target=run, daemon=True).start()

    def _cancel(self) -> None:
        if self.build and messagebox.askyesno(APP_TITLE, "Stop the build? The output folder is left half-written."):
            self.build.cancel()

    def _poll(self) -> None:
        try:
            while True:
                kind, value = self.events.get_nowait()
                if kind == "line":
                    self._log(value)
                elif kind == "written":
                    if self._busy():
                        self.written = max(self.written, value)
                        self.progress.config(value=min(99.0, 100 * self.written / self.expected))
                elif kind == "detected":
                    self._detected(*value)
                else:
                    self._exited(value)
        except queue.Empty:
            pass
        if self._busy():
            m, s = divmod(int(time.monotonic() - self.started), 60)
            text = f"Building {self.detection.game.name}… {m}:{s:02d}"
            if self.expected:
                pct = min(99, int(100 * self.written / self.expected))
                text += f", {self.written / 1e9:.1f} GB written (about {pct}%)"
            self.status.config(text=text)
        self.root.after(100, self._poll)

    def _exited(self, code: int) -> None:
        out = self.out_var.get()
        if self.build and self.build.cancelled:
            self._finish("Cancelled. The output folder is incomplete.", ok=False)
        elif code == 0:
            self._finish(f"Done. The game is in {out}. Play it with CONscripter ({ENGINE_URL}).", ok=True)
        else:
            self._finish(f"Failed (exit code {code}). The log below says why.", ok=False)

    def _finish(self, text: str, ok: bool) -> None:
        self.progress.stop()
        self.progress.config(mode="determinate", value=100 if ok else 0)
        self.status.config(text=("✓ " if ok else "✗ ") + text, fg="#007000" if ok else "#c00000")
        self._log("")
        self._log(text)
        self._refresh()

    def _open_output(self) -> None:
        open_folder(Path(self.out_var.get()))

    def _close(self) -> None:
        if self._busy():
            if not messagebox.askyesno(APP_TITLE, "A build is running. Stop it and quit?"):
                return
            self.build.cancel()
        self.root.destroy()

    # --- log ------------------------------------------------------------

    def _log(self, line: str) -> None:
        self.log_lines.append(line)
        self.log.config(state="normal")
        self.log.insert("end", line + "\n")
        self.log.see("end")
        self.log.config(state="disabled")

    def _clear_log(self) -> None:
        self.log_lines.clear()
        self.log.config(state="normal")
        self.log.delete("1.0", "end")
        self.log.config(state="disabled")


def _webp_factor(values: dict) -> float:
    """About how much smaller WebP makes a build (the pictures are most of it)."""
    if not values.get("webp"):
        return 1.0
    return 0.5 if str(values.get("webp_quality", "")).strip() else 0.7


BUNDLED_FONTS = {"ipag.ttf": "IPA Gothic", "ipam.ttf": "IPA Mincho"}


def font_name(font: Path) -> str:
    if font.parent == tools.bundle_dir() / "fonts" and font.name in BUNDLED_FONTS:
        return f"{BUNDLED_FONTS[font.name]} (included)"
    return str(font)


def _quote(s: str) -> str:
    return f'"{s}"' if any(c in s for c in " \t'\"") else s


def open_folder(path: Path) -> None:
    if os.name == "nt":
        os.startfile(path)  # type: ignore[attr-defined]
    elif sys.platform == "darwin":
        subprocess.Popen(["open", str(path)], env=tools.child_env())
    else:
        subprocess.Popen(["xdg-open", str(path)], env=tools.child_env())


def cli_detect(folder: str) -> int:
    found = games.detect(folder)
    if not found:
        print(f"no supported game in {folder}")
        return 1
    for d in found:
        font = tools.find_font(d.game.font)
        print(f"{d.game.name} ({d.game.engine}) at {d.root}")
        for p in d.problems:
            print(f"  problem: {p}")
        for w in d.warnings:
            print(f"  warning: {w}")
        args = d.game.args(d.root, games.default_output(d), font or Path(d.game.font))
        print("  conscripter-convert " + " ".join(_quote(a) for a in args))
        if d.game.needs_ffmpeg:
            print(f"  ffmpeg: {tools.find_ffmpeg() or 'NOT FOUND'}")
    return 0


def self_test(report: str) -> int:
    """For CI: is the converter bundled and does it run, are the fonts there."""
    state: dict = {"converter": str(tools.find_converter() or ""), "dnd": TkinterDnD is not None,
                   "fonts": {g.font: str(tools.find_font(g.font) or "") for g in games.GAMES}}
    if state["converter"]:
        r = subprocess.run([state["converter"], "--help"], capture_output=True, encoding="utf-8", errors="replace", env=tools.child_env())
        state["converter_runs"] = r.returncode == 0 and "convert-game" in r.stdout
    ffmpeg = tools.find_ffmpeg()
    if ffmpeg:  # not on every CI runner; a bundle must not hide its encoders
        state["ffmpeg_missing"] = tools.ffmpeg_missing_encoders(ffmpeg)
    Path(report).write_text(json.dumps(state, indent=2), encoding="utf-8")
    ok = state.get("converter_runs") and all(state["fonts"].values()) and state["dnd"] and not state.get("ffmpeg_missing")
    return 0 if ok else 1


def main() -> int:
    tools.unbundle_dll_directory()
    if len(sys.argv) == 3 and sys.argv[1] == "--detect":
        return cli_detect(sys.argv[2])
    if len(sys.argv) == 3 and sys.argv[1] == "--self-test":
        return self_test(sys.argv[2])
    app = App()
    if len(sys.argv) == 2:
        app._load(Path(sys.argv[1]))
    app.root.mainloop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
