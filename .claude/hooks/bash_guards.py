#!/usr/bin/env python3
"""PreToolUse hook on Bash: pkill/pgrep -f, session lines in commits, low memory.

Self-test: python3 .claude/hooks/bash_guards.py --test
"""
import json
import re
import sys

MIN_GIB = 6
# a command word after start, ; & | ( ` or $(
POS = r"(?:^|[;&|(`]|\$\()\s*(?:\w+=\S*\s+)*"
PGREP = re.compile(POS + r"(?:pkill|pgrep)\s+(?:-\w+\s+)*-\w*f")
COMMIT = re.compile(POS + r"git\s+(?:-\S+\s+)*commit\b")
SESSION = re.compile(r"Claude-Session:|claude\.ai/code/session")
HEAVY = re.compile(POS + r"(?:cargo\s+(?:build|test|clippy)\b|\S*tests/snapshot\.sh|\S*tests/pr-report\.sh"
                   r"|\S*build-engine\.sh|\S*cross_web\.sh|\S*conscripter-convert\s+(?:convert-game|build)\b)")


def mem_gib():
    for line in open("/proc/meminfo"):
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) / 1048576
    return 99.0


def check(cmd, gib):
    if PGREP.search(cmd):
        return "pkill -f/pgrep -f match your own shell: use pkill -x/pgrep -x or a PID."
    if COMMIT.search(cmd) and SESSION.search(cmd):
        return "Drop the Claude-Session / claude.ai/code/session line from the commit message (the user forbade it; CI rejects it)."
    if HEAVY.search(cmd) and "--dry-run" not in cmd and gib < MIN_GIB:
        return (f"memory low ({gib:.1f} GiB free; reference traces get OOM-killed): "
                "wait a few minutes and retry")
    return None


def test():
    lo, hi = 2.0, 20.0
    for c in ["pkill -f conscripter", "x && pgrep -fa foo", "pkill -9 -f foo",
              "git commit -m 'a\n\nClaude-Session: https://x'",
              "git commit -m 'https://claude.ai/code/session_1'"]:
        assert check(c, hi), c
    for c in ["cargo build", "cargo test -q", "tests/snapshot.sh", "FOO=1 cargo clippy --all-targets",
              "target/release/conscripter-convert convert-game x"]:
        assert check(c, lo) and not check(c, hi), c
    for c in ["pkill -x conscripter", "pgrep -x foo", "git commit -m 'x'", "echo Claude-Session:",
              "grep -n 'pkill -f' CLAUDE.md", "convert-game x --dry-run", "ls cargo", "echo cargo build"]:
        assert not check(c, lo), c
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        test()
        sys.exit(0)
    msg = check(json.load(sys.stdin).get("tool_input", {}).get("command", ""), mem_gib())
    if msg:
        print(msg, file=sys.stderr)
        sys.exit(2)
