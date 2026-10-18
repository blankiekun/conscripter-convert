#!/usr/bin/env bash
# Lead tool: lands PR <n> in one call. Checks the PR head out in a scratch
# worktree (out/land-<n>), rebases it on origin/main (conflicts in
# tests/snapshots/ take main's side), reruns tests/snapshot.sh --update when
# those conflicted or the PR touches shared code (crates/, engine), pushes
# with --force-with-lease, waits for the checks and squash-merges.
#
#   tests/land.sh [--dry-run] <pr>     --dry-run stops before the push
#
# Any other conflict stops with a message to pass to the PR's agent.
set -euo pipefail
cd "$(dirname "$0")/.."
dry=0; [ "${1:-}" = --dry-run ] && { dry=1; shift; }
pr=${1:?usage: tests/land.sh [--dry-run] <pr>}
br=$(gh pr view "$pr" --json headRefName --jq .headRefName)
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
sleep 20
gh pr checks "$pr" --watch --interval 30 || { echo "REFUSED: checks failed on #$pr"; exit 1; }
gh pr merge "$pr" --squash
