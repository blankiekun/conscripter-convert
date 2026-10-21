# Starting a team lead on a new game

How to open a lead session that runs the conversion of a new game with
agents. The conversion steps themselves are `docs/new-game.md`; the
lead's standing rules are `docs/lead.md`.

## 1. Ask the user first

Before dispatching anything, ask (one question each, with options):

1. **Which Claude plan are they on?** It sets how many agents run at
   once and which models do the work (section 2).
2. **The game:** title, platform (PC, Vita, Switch, …), where their copy
   is, whether they own it (conversion is only for games the user owns).
3. **Scope:** story only, or extras too (gallery, replay, music room);
   which languages; which targets matter most (web, desktop, Android).
4. **Running the original:** may the original be run under wine or an
   emulator if the code leaves a doubt (CLAUDE.md, "Wine is the last
   resort")?

Write the answers into the game document's scope section and the
machine-local facts (where the copy is, keys, tools) into memory, not
the repo.

## 2. Fit the work to the plan

Usage is mostly cache reads: every call re-reads the session's context,
so cost grows with calls x context, and Opus costs several times
Sonnet. A fresh agent costs about 45k tokens before it does anything.

| Plan | Lead model | Agents at once | Who does what |
|---|---|---|---|
| Pro | Sonnet | 1 | `worker` (Sonnet) and `checker` (Haiku) for most items; `engineer` only for decompiles and the first-contact findings |
| Max 5x | Opus (Sonnet when the week runs low) | 2 | `builder` for features with a clear spec, `engineer` for reverse engineering |
| Max 20x | Opus | 4 | as `docs/lead.md` |
| Team / Enterprise | ask what share of the seat this project may use, then as the closest row | | |
| API (pay per token) | Sonnet | 1-2 | prefer `worker`/`checker`; keep prompts and reports short; no long wine or emulator runs without asking |

On every plan:
- Ask the user to say when the weekly usage passes ~75 %; then drop to
  one agent and Sonnet, and finish open PRs before starting new items.
- Keep the lead's own context small: run the `lead-handoff` skill and
  `/clear` near ~150k tokens.
- Do small jobs (doc edits under 10 lines, doc-only rebases) yourself.
- Wine and emulator runs cost the most per answer: decompile and dry
  run first.

## 3. Set up the game

1. Pick the game's id and branch prefix (`<game>/`), add its row to the
   table in `CLAUDE.md` and create `docs/games/<game>.md` from
   `docs/games/TEMPLATE.md`, in one PR.
2. Make its worktree: `git worktree add ../conscripter-convert-<game>
   -b <game>/start origin/main`.
3. New engine: `docs/new-game-reference.md`, "A new engine's code"; the reader goes in
   `crates/conv-engines/src/<engine>/`, one CLI subcommand.
4. Start the lead's state file `out/lead-scratch/NEXT-LEAD-PROMPT.md`
   (start here, running agents, queue, rules, the user's decisions).

## 4. Dispatch in phases

Each phase is one or more agents, one item and one PR each, in the
order of `docs/new-game.md`:

| Phase | `docs/new-game.md` | Agent type |
|---|---|---|
| First contact: formats, archives, exe identified | 0-1 | `engineer` |
| Script decode and opcode map | 2 | `engineer` |
| Lowering to the IR, control flow, text and voices | 3-5 | `builder` once the opcodes are known |
| Pictures, sound, system screens, extras | 6-8 | `builder`; `engineer` for screens the exe draws |
| Matching the original, engine additions | 9-10 | `engineer` |
| Route check, web build, finishing | 11-13 | `worker` for route runs and releases |

Prompts name the game, the worktree, the one item, what it waits on,
"Follow CLAUDE.md's Rules", and "report in at most 10 lines". Review
each PR's diff, not only its body; merge per `docs/lead.md`.
