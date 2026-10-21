#!/usr/bin/env bash
# Lead tool: lands PR <n> in one call. Checks the PR head out in a scratch
# worktree (out/land-<n>), rebases it on origin/main (conflicts in
# tests/snapshots/ take main's side), reruns tests/snapshot.sh --update when
# those conflicted or the PR touches shared code (crates/, engine), pushes
# with --force-with-lease, waits for the required checks (rerunning a
# stale engine-on-master once) and squash-merges. Then it removes out/land-<n>
# and, when clean, another worktree on the PR's branch with its local branch.
#
#   tests/land.sh [--dry-run] <pr>     --dry-run stops before the push
#
# Takes out/land.lock (one landing at a time) and refuses a draft or non-OPEN PR.
# Any other conflict stops with a message to pass to the PR's agent.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD
dry=0; [ "${1:-}" = --dry-run ] && { dry=1; shift; }
pr=${1:?usage: tests/land.sh [--dry-run] <pr>}
mkdir -p out
exec 9>out/land.lock
flock -n 9 || { echo "REFUSED: another landing is running (out/land.lock); wait for it."; exit 1; }
read -r state draft br title < <(gh pr view "$pr" --json state,isDraft,headRefName,title --jq '[.state,.isDraft,.headRefName,.title]|@tsv' | tr '\t' ' ')
echo "#$pr  $title  [$br]  $state"
[ "$state" = OPEN ] && [ "$draft" = false ] || { echo "REFUSED: #$pr is not an open, non-draft PR"; exit 1; }
git fetch -q origin main "$br"
wt=out/land-$pr
[ -e "$wt/.git" ] || git worktree add -q --detach "$wt" "origin/$br"
cd "$wt"
git checkout -q --detach "origin/$br"
git submodule update --init -q engine
if ! git rebase -q origin/main 2>/dev/null; then
    other=$(git diff --name-only --diff-filter=U | grep -v '^tests/snapshots/' || true)
    if [ -n "$other" ]; then
        git rebase --abort
        echo "REFUSED: real conflict in: $other"
        echo "Tell the agent on $br: rebase on origin/main, resolve those files, push, report again."
        exit 1
    fi
    while ! GIT_EDITOR=true git rebase --continue >/dev/null 2>&1; do
        git checkout --ours -- tests/snapshots 2>/dev/null || true # in a rebase, ours = main
        git add tests/snapshots
        if git diff --name-only --diff-filter=U | grep -q .; then
            git rebase --abort; echo "REFUSED: unresolved conflict"; exit 1
        fi
    done
    redo=1
fi
git submodule update --init -q engine
if [ "${redo:-0}" = 1 ] || git diff --name-only origin/main...HEAD | grep -qE '^(crates/|engine$)'; then
    tests/snapshot.sh --update
    if ! git diff --quiet -- tests/snapshots; then
        git add tests/snapshots
        git commit -qm "Snapshots: rerun after rebase"
    fi
fi
if [ $dry = 1 ]; then echo "dry run: rebased at $(git rev-parse --short HEAD), not pushed"; git log --oneline origin/main..HEAD; exit 0; fi
git -c credential.helper= -c credential.helper='!gh auth git-credential' push -q --force-with-lease="$br:origin/$br" origin "HEAD:refs/heads/$br"
# the checks take a moment to register after a push (minutes when CI is busy);
# wait for the pushed commit's own: right after the push the PR still shows
# the old head's, and watching those ends at once ("no checks reported")
sha=$(git rev-parse HEAD)
for _ in $(seq 80); do [ "$(gh api "repos/{owner}/{repo}/commits/$sha/check-runs" --jq .total_count 2>/dev/null)" -gt 0 ] 2>/dev/null && break; sleep 15; done
# only the required checks: the rest still report on the PR. engine-on-master
# fails when it ran before the engine PR merged; once the pointer is on (or
# behind) master, a rerun passes, so rerun it once instead of refusing.
if ! gh pr checks "$pr" --required --watch --interval 30; then
    failed=$(gh pr checks "$pr" --required --json name,bucket,link --jq '.[] | select(.bucket == "fail" or .bucket == "cancel") | .name + " " + .link')
    # (one line per workflow run: a push run can report beside the pull_request one)
    [ -n "$failed" ] && ! grep -qv '^engine-on-master ' <<< "$failed" || { echo "REFUSED: checks failed on #$pr"; exit 1; }
    sha=$(git ls-tree HEAD engine | awk '{print $3}')
    status=$(gh api "repos/blankiekun/CONscripter/compare/master...$sha" --jq .status)
    [ "$status" = identical ] || [ "$status" = behind ] || { echo "REFUSED: engine $sha is $status master: merge its engine PR first"; exit 1; }
    for run in $(sed -E 's|.*/runs/([0-9]+)/.*|\1|' <<< "$failed" | sort -u); do
        echo "engine-on-master failed but engine $sha is now $status master: rerunning run $run"
        gh run rerun "$run" --failed
    done
    sleep 20 # until the rerun shows as pending
    gh pr checks "$pr" --required --watch --interval 30 || { echo "REFUSED: checks failed on #$pr"; exit 1; }
fi
gh pr merge "$pr" --squash
cd "$root"
landed=$(git -C "$wt" rev-parse HEAD)
git worktree remove --force "$wt" && echo "removed $wt"
# a clean worktree still on the merged branch (not this checkout, never saves)
git worktree list --porcelain | awk -v b="refs/heads/$br" '/^worktree /{p=substr($0,10)} $0=="branch "b{print p}' |
while read -r other; do
    [ "$other" = "$root" ] || [[ "$other" == *saiima-flow* ]] && continue
    [ -z "$(git -C "$other" status --porcelain)" ] || { echo "kept $other: uncommitted changes"; continue; }
    ! git -C "$other" cherry "$landed" HEAD | grep -q "^+" || { echo "kept $other: commits not landed"; continue; }
    git worktree remove --force "$other" && git branch -D "$br" && echo "removed $other and branch $br"
done || true
