#!/usr/bin/env python3
"""PreToolUse hook on Bash: refuse programs that can reach the user's desktop.

wine, Vita3K, Ryujinx, Xvfb, xdotool, `flatpak run`, AppImages, retroarch,
gui/app.py, xdg-open/`gio open` and the engine are allowed only through
research/tools/headless-run.sh or the scripts that re-run themselves under it;
the engine and gui/app.py also with their dummy/offscreen driver set in the
command. ydotool drives the real input device: always refused. Self-test: python3 .claude/hooks/emu_run.py --test
"""
import json
import re
import sys

OK = re.compile(r"headless-run\.sh|\b(xtrace|ntrace|vita-emu|switch-emu)\.sh")
# a program name in command position: after start, ; & | ( ` and wrappers/env assignments
CMD = re.compile(r"(?:^|[;&|(`]|\$\()\s*(?:(?:\w+=\S*|sudo|env|nohup|exec|time|timeout(?:\s+-\S+(?:\s+\d\S*)?)*\s+\d\S*|setsid)\s+)*"
                 r"(?:\S*/)?(wine|wine64|wineserver|winedbg|Vita3K\S*|Ryujinx\S*|Xvfb|xdotool|ydotool|retroarch|xdg-open"
                 r"|flatpak\s+run|gio\s+open|\S*\.AppImage|AppRun|(?:python3?\s+)?(?:\S*/)?gui/app\.py"
                 r"|conscripter(?!-convert)\S*)(?:\s|$)")
# the engine with SDL's dummy video driver, the GUI with Qt's offscreen platform, draw nothing
DUMMY = {"conscripter": re.compile(r"SDL_VIDEODRIVER=dummy"), "gui": re.compile(r"QT_QPA_PLATFORM=offscreen")}
MSG = ("This program can reach the user's desktop. The engine: SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy "
       "in the same command, or research/tools/scratch.sh; gui/app.py: QT_QPA_PLATFORM=offscreen; ydotool: never. "
       "Run the original, emulators, xdotool, AppImages, flatpak apps only through research/tools/headless-run.sh <cmd> (own Xvfb, no audio, "
       "never the user's desktop) or the wrappers xtrace.sh, ntrace.sh, vita-emu.sh, switch-emu.sh. "
       "First search research/tools/shots.py find <words>. A quick check from a generated save may run "
       "through headless-run.sh now; anything longer goes in "
       "research/<engine>/local/film-list.md (docs/tools.md, \"Matching the original\"): "
       "a session films the whole list at once.")


def refused(cmd):
    # the wrapper's argument is in command position too (ydotool there still reaches the desktop)
    for m in CMD.finditer(cmd.replace("headless-run.sh", ";")):
        prog = m.group(1)
        if prog == "ydotool":
            return True
        if OK.search(cmd):
            continue
        kind = "conscripter" if prog.startswith("conscripter") else "gui" if prog.endswith("app.py") else ""
        if not (kind and DUMMY[kind].search(cmd)):
            return True
    return False


def test():
    bad = ["wine game.exe", "cd x && WINEPREFIX=p wine64 g.exe", "timeout -k 5 60 wine g.exe",
           "Xvfb :9 &", "./Vita3K.AppImage", "x; wineserver -k",
           "xdotool click 1", "ydotool click 0xC0", "research/tools/headless-run.sh ydotool key 28",
           "flatpak run io.github.ryubing.Ryujinx", "flatpak run --help", "./squashfs-root/AppRun",
           "retroarch -L vita3k_libretro.so g.vpk", "python3 gui/app.py --help", ".venv/bin/python gui/app.py",
           "xdg-open https://vita3k.org", "gio open x.html", "cd out/g && engine/build/conscripter -r .",
           "cd o && SDL_AUDIODRIVER=dummy timeout -k 5 60 ../../engine/build/conscripter -r .",
           "/tmp/s/conscripter-mine --root ."]
    good = ["research/tools/headless-run.sh wine g.exe", "research/xuse/tracedll/xtrace.sh a",
            "grep -n wine docs/new-game.md", "pkill -x wine", "ls Xvfb-notes", "git commit -m 'wine docs'",
            "research/tools/headless-run.sh xdotool click 1", "research/tools/headless-run.sh flatpak run x",
            "cd o && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 60 ../../engine/build/conscripter -r .",
            "research/tools/headless-run.sh engine/build/conscripter -r .", "pkill -x conscripter",
            "target/release/conscripter-convert convert-game g -o out/g-cons", "cd ../conscripter-convert-xuse",
            "QT_QPA_PLATFORM=offscreen python3 gui/app.py --detect g", "grep -rn xdg-open research/",
            "flatpak list", "gio info x", "cargo build -p conscripter-cli", "cp engine/build/conscripter /tmp/s/"]
    assert all(refused(c) for c in bad), [c for c in bad if not refused(c)]
    assert not any(refused(c) for c in good), [c for c in good if refused(c)]
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        test()
        sys.exit(0)
    cmd = json.load(sys.stdin).get("tool_input", {}).get("command", "")
    if refused(cmd):
        print(MSG, file=sys.stderr)
        sys.exit(2)
