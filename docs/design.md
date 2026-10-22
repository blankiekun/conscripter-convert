# CONscripter-convert: design

Shared design of every game: architecture, IR, output stage, assets,
testing. Per-game formats, opcodes and status: `docs/games/<game>.md`
(list in `CLAUDE.md`); new game: `docs/new-game.md`; lessons:
`docs/lessons.md`; engine commands: the engine's `COMMANDS.md`.

Rust workspace for the converter; C++ for engine features, which go into
CONscripter, our ONScripter-Yuri fork (`engine/` submodule). Input: an
installed game directory. Output: `0.txt` + `arc.nsa` archives +
converted media + `ons_args`, plus a conversion report.

## Scope

Text, sprites, backgrounds, music, voices and choices convert well.
Custom systems (minigames, battle systems, heavily scripted menus in the
engine's own language) do not convert automatically; they are reported,
never silently broken. The deterministic pipeline is the source of truth
and works offline.

## Architecture

```
            ┌──────────┐   ┌──────────┐   ┌────────────┐   ┌──────────┐   ┌────────────┐
 game dir → │ Detect   │ → │ Unpack    │ → │ Decompile  │ → │ IR       │ → │ ONS output │ → 0.txt + arc.nsa
            │ engine   │   │ archives  │   │ scripts    │   │ (common) │   │ + packing  │
            └──────────┘   └──────────┘   └────────────┘   └──────────┘   └────────────┘
                                 │                               ▲
                                 └─→ Asset decode/transcode ─────┘
```

Each source engine is compiled in (no plugins): readers and compiler in
`crates/conv-engines/src/<engine>/`, one CLI subcommand per engine
(`crates/conv-cli/src/main.rs`). Only the output stage (`conv-cons`) knows
NScripter and CONscripter.

| Path | Contents |
|---|---|
| `crates/conv-core` | The engine-neutral intermediate representation (IR) |
| `crates/conv-engines` | Source engine readers, one module per engine (`xuse/`, ...) |
| `crates/conv-cons` | CONscripter output: IR to `0.txt`, NSA archive writer |
| `crates/conv-cli` | The `conscripter-convert` command |
| `engine/` | CONscripter, the target engine (git submodule) |
| `research/` | Reverse-engineering scripts (`research/README.md`) |
| `docs/games/` | One document per game: formats, opcodes, progression variables, status |

## The IR (core design decision)

A small, engine-neutral list of instructions, grouped into labelled blocks,
and serializable (serde → JSON) so tools can read and diff it. The core
instructions are below; the full list is the `Op` enum in
`crates/conv-core/src/ir/mod.rs`.

- **Text:** `Say { speaker, text, voice }`, `ClearText`, `TextboxShow/Hide`, `WaitClick`, `Wait(ms)`
- **Graphics:** `Bg { image, transition }`, `SpriteShow { slot, image, x, y, transition }`,
  `SpriteHide { slot }`, `Cg { image }`, `ScreenEffect { kind, params }`
- **Audio:** `BgmPlay/Stop { fade }`, `SePlay { channel, file }`, `Voice { file }`
- **Control flow:** `Label`, `Jump`, `Call`/`Return`, `Choice(Vec<Option{ text, target, cond }>)`, `If { cond, then, else }`
- **Variables:** `Set { var, expr }`. Variables are named and marked `Local`/`Global`.
- **Escape hatches:** `Unsupported { engine, raw, loc }`, `Comment`

Rules:
- Every instruction carries a `SourceLoc` (file and line or byte offset).
- Sprite layer composition (body + face + accessories) happens *before* the
  IR; the IR refers to the merged image.
- The output stage maps named variables to NScripter numbers (`%n`, `$n`),
  putting globals in the `globalon` range.

## Output stage (IR → NScripter)

| IR | NScripter output |
|---|---|
| `Label` / `Jump` / `Call` | `*label`, `goto`, `gosub` / `return` |
| `Say` | `dwave 0,"voice"` + text line with name prefix and `\` / `@` click waits |
| `Bg` | `bg "file",effect#` |
| `SpriteShow` | `lsp n,":a;file",x,y` + `print effect#`. Not `ld`, which only has 3 fixed positions. |
| `Choice` | `select` / `csel` |
| `BgmPlay` / `SePlay` | `bgm`, `bgmstop`, `dwave n` |
| transitions | `effect` definitions; mask wipes converted to mask images |
| `If` / `Set` | `if %n == ...`, `mov` / `add` |
| `Unsupported` | `;` comment + entry in the conversion report |

The output stage also writes the `*define` section (`effect` defs, window and
font setup, menu, `game`). The script is written in UTF-8.

### Target engine: CONscripter

[CONscripter](https://github.com/blankiekun/CONscripter) forks
[ONScripter-Yuri](https://github.com/YuriSizuku/OnscripterYuri) (itself
ONScripter-Jh), chosen because one tree builds for web (WebAssembly),
Windows, Linux, macOS and Android and adds UTF-8 scripts, Lua and
arbitrary resolutions to NScripter. Convert with existing NScripter
commands first; add an engine feature when they can't reproduce a game
feature (`CLAUDE.md`, "Rules"). Build commands: `CONTRIBUTING.md`,
`docs/tools.md`.

**Startup options.** CONscripter reads whitespace-separated options (up to
16) from `ons_args` in the game directory; this is how every converted
game passes options, web build included. The converter writes
`--enc:utf8` (the script is UTF-8; CONscripter lays its text out as a
Shift-JIS one's, `engine/COMMANDS.md`) because ONScripter-Yuri reads GBK
by default.

### Done with stock commands

Stock commands suffice (per-game details live in the game's document):
- **Window caption:** CONscripter's `caption` converts the title from the
  script's encoding to UTF-8, so Japanese titles show in the browser tab
  and the window manager.
- **Movies:** ffmpeg to H.264 + AAC MP4 (loose files in `movie/`),
  played with `bgmstop` + `mpegplay "movie\name.mp4",skippable`. The web
  build plays it in a `<video>` element (muted when the browser blocks
  autoplay with sound); Windows and Android hand it to the system
  player; the Linux and macOS builds skip movies.
- **BGM loop points:** written into each `.ogg` as `LOOPSTART`/`LOOPEND`
  Vorbis comments, which SDL_mixer (2.0.2+) loops on by itself,
  sample-exact, when `bgm` plays music looped (it ignores `LOOPSTART`
  without an end). Being in the file, the loop survives saves and works
  in stock ONScripter. Not used: `bgm "(seconds)file"` loop-back, which
  isn't saved and never fires in ONScripter-Yuri (`playSound` registers
  `Mix_HookMusicFinished` only when `Mix_PlayMusic` fails).
- **Backlog speaker names and voice replay:** every text line starts with
  a `[name|voice]` tag. ONScripter keeps a page's tag in the log, and in
  saves, when there is a `pretextgosub` (ours only runs `saveon`). The
  backlog reads it with `gettaglog`, shows the name above the page and
  replays the voice when the name is clicked.
- **Ruby:** NScripter's `(base/reading)`, added after escaping (which
  turns the text's own parentheses full-width), drawn by `rubyon`.
  `rubyon` puts the ruby in the line gap and draws text a line gap
  lower, so `setwindow` and the backlog's `strsp` start a line gap higher
  to keep the text in place. ONScripter reads the markup bytes singly, so
  single-byte padding must treat them like colour tags. Readings wider
  than their base push the rest of the line right.
- **Web text overlay on:** `ons_args` leaves out `--no-text-overlay`,
  so the web player gets the text, choice and backlog hooks and lays
  selectable text over the picture (for scanning and Yomitan). The
  player turns it off and on while the game runs
  (`Module.consNoTextOverlay`, engine `COMMANDS.md`, "Web hooks").
- **Right-click menu (not emitted yet):** `rgosub *label` gosubs at a
  right click during a text wait and returns to the same wait. Which menu
  it opens is each game's choice, set where the game's menus are written.

## Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, …) into the `image` crate's types and save as PNG, or with
  `--webp` as near-lossless WebP (`docs/lessons.md`). Then sprite
  composition and resize to the target resolution. Per-image work runs in
  parallel with `rayon`.
- **Audio:** keep OGG where possible; transcode the rest with an `ffmpeg`
  subprocess. Loop points (KiriKiri `.sli`, RealLive loop metadata) can't be
  carried over directly: either note them in the report or bake intro + loop
  into the output.
- **Video:** `ffmpeg` to H.264 + AAC MP4, which browsers and the system players play.
- **Packing:** write `arc.nsa` / `.sar` natively (simple formats) and
  optionally encode `nscript.dat`.

## Testing and validation

1. **Unit tests per engine** (`cargo test`) on small synthetic archives
   and scripts written by hand; commercial assets stay out of the repo.
2. **Snapshots:** `tests/snapshot.sh` dry-runs every game present on the
   machine and says which games' `0.txt` changed.
3. **Linter** (`conscripter-convert lint`): every jump target exists,
   variable numbers are in range, referenced files exist in the archives,
   the text is valid.
4. **Smoke test:** CONscripter's debug trace (`engine/DEBUG.md`) plays a
   build headless and records every page, choice, missing file
   and script error; `engine/tools/cons_trace_check.py` checks it.
5. **Conversion report** per run: fraction of instructions converted,
   unsupported instructions with source locations, assets dropped or
   approximated.

## Legal guardrails

- The tool works on the user's installed copy; it downloads no games and
  ships no assets.
- The user supplies decryption keys; the tool keeps none.
- Converted output is for personal use (publishing would distribute
  copyrighted assets); the README says so.
