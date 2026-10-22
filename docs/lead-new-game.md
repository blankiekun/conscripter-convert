# Starting a team lead on a new game

A lead session that converts a new game with agents. Steps: `docs/new-game.md`; standing rules:
`docs/lead.md`.

## 1. Ask the user first

One question each, with options, before dispatching:

1. **Plan**: which Claude plan (sets agents at once and models, section 2).
2. **Game**: title, platform, where their copy is, that they own it (conversion is for owned games only).
3. **Scope**: story only or extras (gallery, replay, music room); languages; targets (web, desktop, Android).
4. **Original**: may it run under wine or an emulator if the code leaves a doubt (CLAUDE.md, "Wine is the last
   resort")?

Done: answers are in the game document's scope section; machine-local facts (copy location, keys, tools) are in
memory, not the repo.

## 2. Fit the work to the plan

Cost grows with calls x context; Opus costs several times Sonnet.

| Plan | Lead model | Agents at once | Who does what |
|---|---|---|---|
| Pro | Sonnet | 1 | `worker`, `checker`; `engineer` only for decompiles and first-contact findings |
| Max 5x | Opus (Sonnet when the week runs low) | 2 | `builder` for features with a clear spec, `engineer` for reverse engineering |
| Max 20x | Opus | 4 | as `docs/lead.md` |
| Team / Enterprise | ask the seat share this project may use, then the closest row | | |
| API | Sonnet | 1-2 | `worker`, `checker`; short prompts and reports; ask before long wine or emulator runs |

On every plan:
- At ~75 % weekly usage (the user says when): one agent, Sonnet, finish open PRs before new items.
- Run the `lead-handoff` skill and `/clear` near ~150k lead context.
- Decompile and dry-run before any wine or emulator run (the costliest per answer).

## 3. Set up the game

1. Choose the id and branch prefix (`<game>/`); one PR adds its row to `CLAUDE.md`'s table and
   `docs/games/<game>.md` from `docs/games/TEMPLATE.md`.
2. `git worktree add ../conscripter-convert-<game> -b <game>/start origin/main`.
3. New engine: `docs/new-game-reference.md`, "A new engine's code" (reader in
   `crates/conv-engines/src/<engine>/`, one CLI subcommand).
4. Write the state file `out/lead-scratch/NEXT-LEAD-PROMPT.md`.

Done: PR open, worktree exists, state file written.

## 4. Dispatch in phases

One item and one PR per agent, in the order of `docs/new-game.md`:

| Phase | Steps | Agent type |
|---|---|---|
| Scope, engine identified, game document | 1-4 | `engineer` |
| Archives, script decode, opcode map | 5-7 | `engineer` |
| IR lowering, registry, checks, text, voices | 8-11 | `builder` once opcodes are known |
| Pictures, sound, engine additions | 12-13 | `builder` |
| System screens, extras | 14-15 | `builder`; `engineer` for screens the exe draws |
| Route check, saves, web build, play-through | 16-19 | `worker` for route runs and releases; `engineer` for mismatches with the original |

Each prompt names the game, worktree, item, what it waits on, "Follow CLAUDE.md's Rules" and "report in at most
10 lines". Review each PR's diff; merge per `docs/lead.md`.
