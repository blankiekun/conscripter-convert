#!/usr/bin/env python3
"""PreToolUse hook on Read: refuse a large image once, pointing to view.py.

A second Read of the same file in the same session goes through: that is
the agent saying view.py can't do what it needs.
"""
import json
import os
import subprocess
import sys
import tempfile

MAX_PIXELS = 250_000  # about a 500x500 crop; a full 800x600 screenshot is over

hook = json.load(sys.stdin)
path = hook.get("tool_input", {}).get("file_path", "")
if not path.lower().endswith((".png", ".jpg", ".jpeg", ".bmp", ".webp", ".gif")) or path.endswith(".view.png"):
    sys.exit(0)
try:
    w, h = map(int, subprocess.run(["magick", "identify", "-ping", "-format", "%w %h\n", path],
                                   capture_output=True, text=True, timeout=10).stdout.split()[:2])
except (ValueError, OSError, subprocess.TimeoutExpired):
    sys.exit(0)  # not readable as an image: let Read report it
if w * h <= MAX_PIXELS:
    sys.exit(0)

seen = os.path.join(tempfile.gettempdir(), f"claude-image-read-{hook.get('session_id', 'x')}")
try:
    with open(seen) as f:
        if path + "\n" in f.readlines():
            sys.exit(0)
except OSError:
    pass
with open(seen, "a") as f:
    f.write(path + "\n")
print(f"{path} is {w}x{h}: view it through research/tools/view.py instead "
      "(crop, scale, side by side; CLAUDE.md, \"Numbers before pictures\"). "
      "If view.py can't do what you need, say why in one line, then Read the same file again: "
      "the second Read goes through.", file=sys.stderr)
sys.exit(2)
