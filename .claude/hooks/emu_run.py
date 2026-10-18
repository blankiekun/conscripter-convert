#!/usr/bin/env python3
"""PreToolUse hook on Bash: refuse wine/Vita3K/Ryujinx/Xvfb run directly.

Allowed through research/tools/headless-run.sh or the scripts that re-run
themselves under it. Self-test: python3 .claude/hooks/emu_run.py --test
"""
import json
import re
import sys

OK = re.compile(r"headless-run\.sh|\b(xtrace|ntrace|vita-emu|switch-emu)\.sh")
# a program name in command position: after start, ; & | ( ` and wrappers/env assignments
CMD = re.compile(r"(?:^|[;&|(`]|\$\()\s*(?:(?:\w+=\S*|sudo|env|nohup|exec|time|timeout(?:\s+-\S+(?:\s+\d\S*)?)*\s+\d\S*|setsid)\s+)*"
                 r"(?:\S*/)?(wine|wine64|wineserver|winedbg|Vita3K\S*|Ryujinx\S*|Xvfb)(?:\s|$)")
MSG = ("Run the original only through research/tools/headless-run.sh <cmd> (own Xvfb, no audio, "
       "never the user's desktop) or the wrappers xtrace.sh, ntrace.sh, vita-emu.sh, switch-emu.sh. "
       "First search research/tools/shots.py find <words>, then add the doubt to "
       "research/<engine>/local/film-list.md (docs/new-game.md, \"Film list\") and go on: "
       "a session films the whole list at once.")


def refused(cmd):
    return bool(CMD.search(cmd)) and not OK.search(cmd)


def test():
    bad = ["wine game.exe", "cd x && WINEPREFIX=p wine64 g.exe", "timeout -k 5 60 wine g.exe",
           "Xvfb :9 &", "./Vita3K.AppImage", "x; wineserver -k"]
    good = ["research/tools/headless-run.sh wine g.exe", "research/xuse/tracedll/xtrace.sh a",
            "grep -n wine docs/new-game.md", "pkill -x wine", "ls Xvfb-notes", "git commit -m 'wine docs'"]
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
