#!/usr/bin/env bash
# Sets up a scratch run next to a full build, without writing into it.
# usage: scratch.sh GAME_DIR SCRATCH_DIR [SCRIPT [ENGINE]]
#   GAME_DIR   a full build (out/<game>-cons); only read
#   SCRIPT     the 0.txt to run (default: GAME_DIR/0.txt), copied in
#   ENGINE     the binary (default: the repo's engine/build/conscripter),
#              copied in as cons-<scratch name>, since other agents
#              `pkill -x conscripter`
# SCRATCH_DIR gets symlinks to the read-only inputs only (*.nsa, *.sar,
# *.ns2, default.ttf, movie/), a copy of ons_args, the script and the
# engine, and run.sh: `SCRATCH_DIR/run.sh [engine options]` runs it
# headless there with --save-dir SCRATCH_DIR/save (TIMEOUT, default 600 s).
# Files the engine writes (0.txt, kidoku.dat, envdata, gloval.sav, saves,
# logs) are never links: a link there would write into GAME_DIR.
# Self-check after changing this file: test_scratch.sh.
set -euo pipefail
[ $# -ge 2 ] || { sed -n '3,9p' "$0"; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
game=$(cd "$1" && pwd)
scratch=$(realpath -m "$2")
script=${3:-$game/0.txt}
engine=${4:-$here/../../engine/build/conscripter}
case "$scratch/" in "$game/"*) echo "scratch dir is inside $game"; exit 1;; esac
[ -f "$script" ] && [ -x "$engine" ] || { echo "no script $script or engine $engine"; exit 1; }
mkdir -p "$scratch"

# Old links (from an earlier set-up, or by hand) go first: a copy onto a
# link would write through it.
find "$scratch" -maxdepth 1 -type l -delete
shopt -s nullglob
for f in "$game"/*.nsa "$game"/*.sar "$game"/*.ns2 "$game"/default.ttf "$game"/movie; do
    ln -s "$f" "$scratch/"
done
[ -f "$game/ons_args" ] && cp "$game/ons_args" "$scratch/"
cp "$script" "$scratch/0.txt"
rm -f "$scratch/kidoku.dat"  # byte offsets into the old 0.txt
exe=cons-$(basename "$scratch")
cp "$engine" "$scratch/$exe"
mkdir -p "$scratch/save"
cat > "$scratch/run.sh" <<RUN
#!/usr/bin/env bash
cd "$scratch" && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy exec timeout -k 5 "\${TIMEOUT:-600}" ./$exe --root . --save-dir "$scratch/save" "\$@"
RUN
chmod +x "$scratch/run.sh"
echo "$scratch/run.sh [options]   # engine $exe"
