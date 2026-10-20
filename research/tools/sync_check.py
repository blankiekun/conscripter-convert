#!/usr/bin/env python3
"""Flag a diff that changes an engine's sync code but none of its tests.

usage: sync_check.py [REF [HEAD]]   (default origin/main HEAD; run from the branch)
       sync_check.py --self-test

Composition bugs (an unwaited action across a sync, a 0 ms action on one
in flight) live in the code that decides what is still running when the
script waits. A change there needs a test of the composed output
(docs/new-game-reference.md, "Timing and overlap"). Prints one line per engine whose sync code
changed with no test change: "sync changed, no composition test".
Never fails: the reviewer decides (tests/pr-report.sh shows the line).
"""
import re
import subprocess
import sys

E = "crates/conv-engines/src/"
# engine: (its sync code, its tests: a directory, or None for the sync file's own `#[cfg(test)]` module)
SYNC = {
    "nitroplus": (E + "nitroplus/convert/sync.rs", E + "nitroplus/convert/tests/"),
    # the others keep their waits, fades and motions in one convert.rs with its tests inline
    "xuse": (E + "xuse/convert.rs", None),
    "mages": (E + "mages/convert.rs", None),
    "typemoon": (E + "typemoon/convert.rs", None),
}


def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout


def touches_tests(diff, test_line):
    """diff: `git diff -U0` of one file; test_line: the new file's `#[cfg(test)]` line (None: no tests)."""
    if test_line is None:
        return False
    for a, b in re.findall(r"^@@ -\S+ \+(\d+)(?:,(\d+))? @@", diff, re.M):
        if int(a) + max(int(b or 1), 1) - 1 >= test_line:
            return True
    return False


def flags(ref, head="HEAD"):
    out = []
    changed = git("diff", "--name-only", f"{ref}...{head}").split()
    for eng, (sync, tests) in SYNC.items():
        if sync not in changed:
            continue
        if tests:
            ok = any(f.startswith(tests) for f in changed)
        else:
            src = git("show", f"{head}:{sync}").split("\n")
            line = next((i + 1 for i, s in enumerate(src) if s.startswith("#[cfg(test)]")), None)
            ok = touches_tests(git("diff", "-U0", f"{ref}...{head}", "--", sync), line)
        if not ok:
            out.append(f"{eng}: sync changed ({sync.removeprefix(E)}), no composition test")
    return out


def self_test():
    d = "@@ -10,2 +10,3 @@ fn a\n-x\n+y\n@@ -50 +51,0 @@\n-z\n"
    assert not touches_tests(d, 60) and not touches_tests(d, None)
    assert touches_tests(d, 51) and not touches_tests(d, 52)
    assert touches_tests("@@ -10,2 +10,3 @@\n", 12) and not touches_tests("@@ -10,2 +10,3 @@\n", 13)
    assert touches_tests("@@ -70,0 +71,4 @@\n+#[test]\n", 60)


if __name__ == "__main__":
    self_test()  # cheap: every run checks the checker
    if sys.argv[1:] == ["--self-test"]:
        print("self-test ok")
    else:
        for f in flags(*(sys.argv[1:] or ["origin/main"])):
            print(f)
