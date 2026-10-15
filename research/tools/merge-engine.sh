#!/usr/bin/env bash
# Resolves a merge conflict on the engine submodule pointer: of the two
# CONscripter commits the branches point to, takes the one that contains
# the other (CONscripter's master is linear, so one always does once
# both are pushed). Fails when neither does: then rebase your engine
# commits onto CONscripter's master first. Run it in the middle of the
# conflicted merge, then go on with `git commit`.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
ours=$(git ls-files -u -- engine | awk '$3 == 2 {print $2}')
theirs=$(git ls-files -u -- engine | awk '$3 == 3 {print $2}')
if [ -z "$ours" ] || [ -z "$theirs" ]; then
    echo "engine/ is not in conflict"; exit 1
fi
git -C engine fetch -q origin
if git -C engine merge-base --is-ancestor "$ours" "$theirs"; then
    pick=$theirs
elif git -C engine merge-base --is-ancestor "$theirs" "$ours"; then
    pick=$ours
else
    echo "neither $ours nor $theirs contains the other: rebase the engine commits onto CONscripter master first"
    exit 1
fi
git -C engine checkout -q "$pick"
git update-index --cacheinfo 160000,"$pick",engine
echo "engine -> $(git -C engine log --oneline -1 "$pick")"
