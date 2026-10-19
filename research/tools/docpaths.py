#!/usr/bin/env python3
"""Check that the repo paths the docs name in backticks exist.

usage: docpaths.py [FILE.md ...]   default: every tracked .md outside
                                   engine/

A path counts when it starts with a top-level folder of the repo
(`crates/…`, `docs/…`, `research/…`); placeholders (`<game>`), globs
and git-ignored paths (`local/`, build outputs) are skipped, and so is `engine/…`
when the submodule isn't checked out. Prints file:line path for each
missing one and exits 1 if there is any. With the submodule checked out it\nalso checks the relative links of engine/*.md.
"""
import os
import re
import subprocess
import sys

TOPS = ("crates/", "docs/", "research/", "tests/", "engine/", "gui/", ".github/", ".claude/")
root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip()
os.chdir(root)
files = sys.argv[1:] or [
    f for f in subprocess.run(["git", "ls-files", "*.md"], capture_output=True, text=True, check=True).stdout.split()
    if not f.startswith("engine/")
]
engine = os.path.exists("engine/Makefile") or os.path.exists("engine/src")
bad = 0
for f in files:
    for n, line in enumerate(open(f, encoding="utf-8"), 1):
        for tok in re.findall(r"`([^`\s]+)`", line):
            p = tok.split(":")[0].rstrip(".,;")  # file.rs:12, trailing punctuation
            if not p.startswith(TOPS) or re.search(r"[<>*{}$|]|\.\.\.|…", p) or "/local/" in p:
                continue
            if p.startswith("engine/") and not engine:
                continue
            # build outputs such as gui/dist/ are git-ignored
            # engine/ is a submodule: ask its own .gitignore (engine/build/ and the like)
            repo, rel = ("engine", p[len("engine/"):]) if p.startswith("engine/") else (".", p)
            if not os.path.exists(p) and subprocess.run(["git", "-C", repo, "check-ignore", "-q", rel], stderr=subprocess.DEVNULL).returncode:
                print(f"{f}:{n} {p}")
                bad += 1
# The engine's own Markdown (a separate repo): relative [text](file.md#anchor) links must resolve inside engine/.
if engine:
    for f in subprocess.run(["git", "-C", "engine", "ls-files", "*.md"], capture_output=True, text=True, check=True).stdout.split():
        if f.startswith(("thirdparty/", "src/")):
            continue
        for n, line in enumerate(open(os.path.join("engine", f), encoding="utf-8"), 1):
            for link in re.findall(r"\]\(([^)\s]+)\)", line):
                target = link.split("#")[0]
                if not target or re.match(r"[a-z]+:", target):
                    continue
                if not os.path.exists(os.path.normpath(os.path.join("engine", os.path.dirname(f), target))):
                    print(f"engine/{f}:{n} {link}")
                    bad += 1
sys.exit(1 if bad else 0)
