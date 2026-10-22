# Starting a team lead on a new game

How a lead session runs the conversion of a new game with agents. The conversion steps are
`docs/new-game.md`; the lead's standing rules are `docs/lead.md`.

## 1. Ask the user first

Before dispatching anything, ask (one question each, with options):

1. **Which Claude plan?** It sets the agents at once and the models (section 2).
2. **The game:** title, platform (PC, Vita, Switch, ...), where their copy is, whether they own it (conversion
   is only for owned games).
3. **Scope:** story only, or extras too (gallery, replay, music room); which languages; which targets matter
   most (web, desktop, Android).
4. **Running the original:** may it run under wine or an emulator if the code leaves a doubt (CLAUDE.md, "Wine is
   the last resort")?

Done: the answers are in the game document's scope section; machine-local facts (where the copy is, keys, tools)
are in memory, not the repo.

## 2. Fit the work to the plan

Usage is mostly cache reads: cost grows with calls x context, Opus costs several times Sonnet, and a fresh agent
costs about 45k tokens before it does anything.

| Plan | Lead model | Agents at once | Who does what |
|---|---|---|---|
| Pro | Sonnet | 1 | `worker` and `checker` for most items; `engineer` only for decompiles and the first-contact findings |
| Max 5x | Opus (Sonnet when the week runs low) | 2 | `builder` for features with a clear spec, `engineer` for reverse engineering |
| Max 20x | Opus | 4 | as `docs/lead.md` |
| Team / Enterprise | ask what share of the seat this project may use, then the closest row | | |
| API (pay per token) | Sonnet | 1-2 | `worker`/`checker`; short prompts and reports; ask before long wine or emulator runs |

On every plan:
- Ask the user to say when weekly usage passes ~75 %; then one agent, Sonnet, and finish open PRs before new items.
- Run the `lead-handoff` skill and `/clear` near ~150k lead context.
- Do small jobs (doc edits under 10 lines, doc-only rebases) yourself.
- Decompile and dry-run before any wine or emulator run, which costs the most per answer.

## 3. Set up the game

1. Pick the game's id and branch prefix (`<game>/`); in one PR add its row to the table in `CLAUDE.md` and
   `docs/games/<game>.md` from `docs/games/TEMPLATE.md`.
2. Make its worktree: `git worktree add ../conscripter-convert-<game> -b <game>/start origin/main`.
3. New engine: `docs/new-game-reference.md`, "A new engine's code"; the reader goes in
   `crates/conv-engines/src/<engine>/`, one CLI subcommand.
4. Start the state file `out/lead-scratch/NEXT-LEAD-PROMPT.md` (start here, running agents, queue, rules, the
   user's decisions).

Done: PR open, worktree exists, state file written.

## 4. Dispatch in phases

Each phase is one or more agents, one item and one PR each, in the order of `docs/new-game.md`:

| Phase | `docs/new-game.md` steps | Agent type |
|---|---|---|
| First contact: scope, engine identified, game document | 1-4 | `engineer` |
| Archives, script decode and opcode map | 5-7 | `engineer` |
| Lowering to the IR, registry, checks, text and voices | 8-11 | `builder` once the opcodes are known |
| Pictures, sound, engine additions | 12-13 | `builder` |
| System screens, extras | 14-15 | `builder`; `engineer` for screens the exe draws |
| Route check, saves, web build, play-through | 16-19 | `worker` for route runs and releases; `engineer` for mismatches with the original |

Prompts name the game, the worktree, the one item, what it waits on, "Follow CLAUDE.md's Rules", and "report in at
most 10 lines". Review each PR's diff, not only its body; merge per `docs/lead.md`.
