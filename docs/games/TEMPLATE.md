# <Title> (<engine>)

Copy this file to `docs/games/<id>.md` (the id of the game's entry in
`crates/conv-engines/src/registry.rs` and of `tests/snapshots/<id>.txt`)
and keep its headings, so that every session finds the same section in
every game (`python3 -I research/tools/mdsec.py docs/games/<id>.md
Status`). The document is public: no local paths (`<game>` for the game
folder), only commands anyone with the game can run.

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

## Status

Only the open state: a table of the areas (✅ done, 🟡 partly, ❌ not
started), then "Next", in order. About 60 lines at most. When an item
is done, its write-up moves to "History".

### Next

1. …

## History

Finished work, newest first, each with its date.
