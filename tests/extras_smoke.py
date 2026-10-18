#!/usr/bin/env python3
"""Headless smoke test of a full build's extras: every mode opens.

    python3 -I tests/extras_smoke.py <build dir> [--engine engine/build/conscripter] [--scratch out/extras-smoke]

Runs the build in a scratch folder (research/tools/scratch.sh: the build is
only read) from the extras' menu (`--goto conv_pextras`) with every lock
the menu reads set (`--vars`), and checks with the debug trace that:
  - every button of `screens/extras.lua` is one: a button drawn but given
    no mode (`enabled=false` with no lock: a mode the build left out, such
    as the sound room and 3D viewer of a Muramasa build without its movies)
    fails;
  - choosing each one closes the menu with its value and reaches its mode
    (a screen of its own or a label outside the menu), with no script error
    or crash.
For a build made with the shared extras' menu (`PictureExtras`); a build
without one is skipped. Exit status 1 when a check fails.
"""
import argparse, json, os, re, struct, subprocess, sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def extras_lua(build):
    loose = build / "screens" / "extras.lua"
    if loose.exists():
        return loose.read_text(encoding="utf-8")
    for arc in sorted(build.glob("arc*.nsa")):  # conv-cons/src/nsa.rs: uncompressed entries
        with open(arc, "rb") as f:
            count, base = struct.unpack(">HI", f.read(6))
            head = f.read(base - 6)
            p = 0
            for _ in range(count):
                end = head.index(b"\0", p)
                name = head[p:end].decode("utf-8", "replace").replace("\\", "/").lower()
                offset, length = struct.unpack_from(">II", head, end + 2)
                p = end + 14
                if name == "screens/extras.lua":
                    f.seek(base + offset)
                    return f.read(length).decode("utf-8")
    return None


def run(scratch, args):
    trace = scratch / "smoke.jsonl"
    subprocess.run([str(scratch / "run.sh"), "--trace", str(trace), "--auto-play", "0", "--skip-screens", "none",
                    "--goto", "conv_pextras", "--vars", str(scratch / "unlock.txt"), *args],
                   env={**os.environ, "TIMEOUT": "60"}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return [json.loads(l) for l in trace.read_text(encoding="utf-8", errors="replace").splitlines() if l.strip()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("build", type=Path)
    ap.add_argument("--engine", default=str(REPO / "engine/build/conscripter"))
    ap.add_argument("--scratch", type=Path, default=REPO / "out/extras-smoke")
    o = ap.parse_args()
    lua = extras_lua(o.build)
    if lua is None:
        print(f"{o.build}: no screens/extras.lua, skipped")
        return 0
    script = (o.build / "0.txt").read_text(encoding="utf-8", errors="replace").splitlines()
    # The menu's `screen` command and its place as the trace names it (*label:line).
    line = next(i for i, l in enumerate(script, 1) if l.startswith('screen "extras"'))
    label = next(l for l in reversed(script[:line]) if l.startswith("*")).split()[0]
    at = f"{label}:{line}"
    back = {int(m) for m in re.findall(r"if %9 == (\d+) goto \*conv_pextras_back", "\n".join(script[line:line + 10]))}

    subprocess.run([str(REPO / "research/tools/scratch.sh"), str(o.build), str(o.scratch), str(o.build / "0.txt"), o.engine],
                   check=True, stdout=subprocess.DEVNULL)
    locks = sorted(set(re.findall(r'var\("([^"]+)"\)', lua)))
    (o.scratch / "unlock.txt").write_text("".join(f"{v}=1\n" for v in locks), encoding="utf-8")

    failures = []
    widgets = [l.strip() for l in lua.splitlines() if l.strip().startswith("{\"")]
    buttons = {}  # button number (widget place * 100) -> (its close value, its file)
    for n, w in enumerate(widgets, 1):
        if not w.startswith('{"button"'):
            continue
        file = re.search(r'file="([^"]*)"', w).group(1)
        if "enabled=false" in w:
            failures.append(f"button {n * 100} ({file}) has no mode in this build")
            continue
        buttons[n * 100] = (int(re.search(r'"close", (-?\d+)', w).group(1)), file)

    records = run(o.scratch, [])
    offered = {b["no"] for r in records if r.get("t") == "choice" and r.get("at") == at for b in r["buttons"]}
    for no in sorted(set(buttons) - offered):
        failures.append(f"button {no} ({buttons[no][1]}) not offered with every lock open")
    for no, (value, file) in sorted(buttons.items()):
        records = run(o.scratch, ["--choose", f"{at}={no}"])
        bad = [r for r in records if r.get("t") in ("error", "crash")]
        closed = next((k for k, r in enumerate(records) if r.get("t") == "screen" and r.get("at") == at
                       and r.get("screen") == "extras" and r.get("action") == "close"), None)
        if bad:
            failures.append(f"button {no}: {bad[0]}")
        elif closed is None or records[closed].get("value") != value:
            failures.append(f"button {no} ({file}): the menu didn't close with {value}")
        else:
            want = "conv_pextras_back" if value in back else None
            reached = None
            for r in records[closed + 1:]:
                if r.get("t") == "label" and not r["name"].startswith("conv_pextras_draw"):
                    reached = r["name"]
                elif r.get("t") == "screen" and r.get("action") == "open" and r.get("screen") != "extras":
                    reached = f'screen "{r["screen"]}"'
                if reached:
                    break
            if not reached or (want and reached != want):
                failures.append(f"button {no} ({file}): value {value} reached {reached}, not its mode")
            else:
                print(f"button {no}: value {value} -> {reached}")
    for f in failures:
        print("FAIL", f)
    print(f"{len(buttons)} modes, {len(failures)} failures ({o.build})")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
