# Team lead notes

- **Delegate only what a few lines of prompt can fully describe**, to
  the cheapest type in `.claude/agents/`: `checker` (haiku) for reruns,
  checks, greps, a clean rebase + push + `gh pr checks --watch`;
  `worker` (sonnet) for mechanical edits, doc updates, build-test-commit
  loops, route runs, releases; `builder` (opus, medium effort) for
  features and engine additions with a clear spec (a screen, a setting,
  wiring existing commands); `engineer` (opus, high) only for reverse
  engineering, decompiles, hard debugging and design calls. The team
  lead picks the type for every agent it starts (the user, 2026-10-17).
- **Do small jobs yourself**: a fresh agent costs about 45k tokens
  before it does anything. A doc-only rebase, a doc edit of 10 lines or
  less, or anything needing what you already know (the session's
  findings, files read, a chain of reasoning) is cheaper done in place.
