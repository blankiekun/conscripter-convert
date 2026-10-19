#!/bin/bash
# setup-venv.sh: make research/tools/.venv with the Python packages the
# research tools import (unicorn for exe_call.py), then run exe_call's
# self-test. Once per machine; rerunning it only checks. pip refuses
# `--user` on a system Python, hence the venv.
set -eu
d=$(cd "$(dirname "$0")" && pwd)
[ -x "$d/.venv/bin/python" ] || python3 -m venv "$d/.venv"
"$d/.venv/bin/python" -c 'import unicorn, sys; sys.exit(unicorn.__version__ != "2.1.4")' 2>/dev/null ||
  "$d/.venv/bin/pip" install -q unicorn==2.1.4
"$d/.venv/bin/python" -I "$d/test_exe_call.py"
