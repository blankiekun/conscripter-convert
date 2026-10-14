#!/bin/bash
# Decompiles a binary with Ghidra (headless) into one pseudo-C file per
# function, for reading and grepping:
#
#   ghidra_export.sh <binary> <outdir> [--names <names.tsv>] [--rtti]
#                    [-- <extra analyzeHeadless import options>]
#
#   <outdir>/decomp/<address>_<name>.c   one file per function
#   <outdir>/functions.tsv               address, name, size, callers, callees, strings
#   <outdir>/strings.tsv                 cp932 strings and the functions using them
#   <outdir>/project/                    the Ghidra project (reused by later runs)
#
# The first run imports and analyzes the binary (minutes); later runs reuse
# the project and only re-apply the names and re-export (faster). Delete
# <outdir>/project to analyze again.
#
# --names: a tab-separated file of `<address> <name> [comment]` (see
#   ApplyNames.java), applied before every export so identified functions
#   keep their names across sessions. Keep it in research/<engine>/.
# --rtti: on the first run, also recover C++ classes from MSVC RTTI
#   (RecoverClassesFromRTTIScript; Windows binaries only, slow).
# After --: options for the import, e.g. `-processor ARM:LE:32:v8T`
#   or `-loader ElfLoader`.
#
# Finds analyzeHeadless on PATH, or in $GHIDRA_INSTALL_DIR/support
# (default /opt/ghidra). Heap: GHIDRA_HEADLESS_MAXMEM (default 2G).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
usage() { sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }

[ $# -ge 2 ] || usage
bin=$(realpath "$1")
out=$(realpath -m "$2")
shift 2
names=/dev/null
rtti=
extra=()
while [ $# -gt 0 ]; do
    case $1 in
        --names) names=$(realpath -m "$2"); shift 2 ;;
        --rtti) rtti=1; shift ;;
        --) shift; extra=("$@"); break ;;
        *) usage ;;
    esac
done
[ -f "$bin" ] || { echo "no such file: $bin" >&2; exit 1; }

headless=$(command -v analyzeHeadless || echo "${GHIDRA_INSTALL_DIR:-/opt/ghidra}/support/analyzeHeadless")
[ -x "$headless" ] || { echo "analyzeHeadless not found; set GHIDRA_INSTALL_DIR" >&2; exit 1; }

mkdir -p "$out/project"
log="$out/ghidra.log"
common=(-scriptPath "$here" -log "$log" -scriptlog "$log")
post=(-postScript ApplyNames.java "$names" -postScript ExportDecomp.java "$out")

if [ -e "$out/project/ghidra.gpr" ]; then
    echo "reusing $out/project (delete it to analyze again)"
    "$headless" "$out/project" ghidra -process "$(basename "$bin")" -noanalysis \
        "${common[@]}" "${post[@]}"
else
    pre=()
    [ -n "$rtti" ] && pre=(-postScript RecoverClassesFromRTTIScript.java)
    "$headless" "$out/project" ghidra -import "$bin" "${extra[@]}" \
        "${common[@]}" "${pre[@]}" "${post[@]}"
fi
grep -E 'ExportDecomp.java> exported|ApplyNames.java> (applied|no names)|ERROR' "$log" | tail -5 || true
