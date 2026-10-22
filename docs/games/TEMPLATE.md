# <Title> (<engine>)

Copy this file to `docs/games/<id>.md` (the id of the game's entry in
`crates/conv-engines/src/registry.rs` and of `tests/snapshots/<id>.txt`)
and keep its headings and order, so every session finds the same
section in every game (`python3 -I research/tools/mdsec.py
docs/games/<id>.md Status`). The document is public: `<game>` for the
game folder, only commands anyone with the game can run.

Order is by reach: Status and Build serve every session, so they come
first; formats, script language and variables are reference for the
branch that touches them; Testing and Milestones come last. Keep a
concept (a format, a command, a variable) under one heading. What
`CLAUDE.md`, `docs/lessons.md`, `docs/tools.md` or a tool's `--help`
already say belongs there; point to it.

## Status

Only the open state, 50 lines at most: a table of the areas (✅ done,
🟡 partly, ❌ not started), then "Next". When an item is done, delete it
(git log and the PR body are the record). A paused game says since when,
and "Next" item 1 says how to resume.

### Next

1. …

## Scope

What the user owns (release, version, platform), what is converted and
what the user chose to drop.

## Build

The full build and the dry run, exactly:

```sh
target/release/conscripter-convert <command> <game> -o out/<id>-cons \
  --font <a Japanese TTF, e.g. ipag.ttf> --title "<title>"
```

The web build, when it differs from `docs/new-game.md` step 12.

## Executables

Every decompile address (`FUN_…`, `0x…`) in this document belongs to
one build. List each executable the addresses come from:

| File | Version | sha256 |
|---|---|---|
| `<exe>` | `<version, or unknown>` | `<sha256sum; unknown if not at hand, never guessed>` |

## Files and formats

Archives, scripts, pictures, sounds, movies: how each is read (or a
pointer to the engine's other game that documents it).

## Script language

Opcodes or commands ranked by use, what each becomes in the IR, and
what is left out.

## Story structure and walkthrough

Routes, endings, the choice mechanic; the route map and how it was made.

## Progression variables

The game's flags and where they land in NScripter's variables (locals,
globals; clear of `conv_core::alloc`'s ranges).

## Testing

The trace check (config, `--trace-ignore` ranges), route runs, wine
recipes for filming the original.

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | - | - |
| First dry run | - | - |
| Story plays | - | - |
| Title, save, settings | - | - |
| Extras | - | - |
| Progression verified | - | - |
| Finished | - | - |
