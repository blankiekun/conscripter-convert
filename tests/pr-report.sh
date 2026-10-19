#!/usr/bin/env bash
# Runs the checks a PR needs and prints a Markdown block for its body.
#   tests/pr-report.sh [--base REF]    (default origin/main; run from the branch)
# cargo test, clippy -D warnings, tests/snapshot.sh --diff REF (txtdiff counts
# per changed game, missing assets, gaps), findings_lint, dropped_lint,
# docpaths, doclint; metrics totals before (REF) and after per changed game,
# and the source places whose motion time went down (motion_drop.py);
# flags engine code changed without a docs/games/*.md. Exit 1 on any failure.
# Fill in each "why it changed" line, and the "Found by" block, before
# opening the PR.
set -uo pipefail
cd "$(dirname "$0")/.."
base=origin/main; [ "${1:-}" = --base ] && base=$2
fail=0 log=$(mktemp)
run() { # title, command...: one line, output in a fold when it fails or is long
    local t=$1; shift
    if "$@" > "$log" 2>&1; then echo "- $t: ok"; else
        fail=1; echo "- $t: **FAILED**"; echo '  ```'; tail -n 25 "$log" | sed 's/^/  /'; echo '  ```'; fi
}
echo "## Checks"
run "cargo test" cargo test -q
run "cargo clippy -D warnings" cargo clippy -q --all-targets -- -D warnings
run "findings_lint" python3 -I research/tools/findings_lint.py
run "dropped_lint" python3 -I research/tools/dropped_lint.py
run "docpaths" python3 -I research/tools/docpaths.py
run "doclint (template headings, Status length)" python3 -I research/tools/doclint.py
eng=$(git diff --name-only "$base"...HEAD -- 'crates/conv-engines/src/*' | cut -d/ -f4 | sort -u | tr '\n' ' ')
if [ -n "$eng" ] && [ -z "$(git diff --name-only "$base"...HEAD -- 'docs/games/*.md')" ]; then
    fail=1; echo "- **engine code changed (${eng% }) but no docs/games/*.md**: update the game's Status"; fi
echo; echo "## tests/snapshot.sh --diff $base"
tests/snapshot.sh --diff "$base" > "$log" 2>&1; rc=$?
echo '```'; grep -v ': same$\|metrics same$' "$log"; echo '```'
[ $rc = 0 ] || fail=1
tot() { sed -n 's/^"totals": \(.*\),$/\1/p'; }
for id in $(sed -n 's/^\([a-z-]*\): \(0.txt differs\|CHANGED\|metrics CHANGED\).*/\1/p' "$log" | sort -u); do
    echo "- $id metrics before: $(git show "$base:tests/snapshots/$id.metrics.json" 2>/dev/null | tot)"
    echo "  after: $(tot < out/snapshot/$id/metrics.json 2>/dev/null)"
    echo "  why it changed: <fill in>"
    r=out/snapshot-ref/out/$id n=out/snapshot/$id  # motion time that went down, per source place (motion_drop.py)
    [ -f "$r.map" ] && [ -f "$n.map" ] && python3 -I research/tools/motion_drop.py "$r/0.txt" "$r.map" "$n/0.txt" "$n.map" |
        sed '/ ms of motion /{s/^/  - motion down: /;s/$/ (why: <fill in>)/;b};s/^/  /'
done
echo; echo "## Found by"
echo "Found by: <user|check|agent>"
echo "<user only> The check that now catches this kind: <name it, or \"none possible: why\">"
echo "<user only> Left of the report: <none, or what is still open>"
rm -f "$log"; exit $fail
