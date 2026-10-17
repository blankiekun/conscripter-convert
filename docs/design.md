# CONscripter-convert — Design

Converts Japanese visual novels (that you own) into games playable on
**[CONscripter](https://github.com/blankiekun/CONscripter)**, our fork of
ONScripter-Yuri, and helps reverse-engineer engines it doesn't support yet.
The converter and the engine are developed
together: when a game feature can't be reproduced with the engine's existing
commands, the feature is added to CONscripter ("Target engine: CONscripter" below).

- **Games:** each converted game has its own document in `docs/games/`
  (formats, opcodes, progression variables, status, leftovers); the list
  is in `CLAUDE.md` and `README.md`. How to start a new game:
  `docs/new-game.md`. This file holds the design shared by every game:
  architecture, IR, output stage, assets, testing. General lessons are in
  `docs/lessons.md`; every engine command is in the engine's `COMMANDS.md`.

- **Language:** Rust (cargo workspace) for the converter; C++ for engine
  features, which go into CONscripter (the `engine/` submodule).
- **Input:** an installed game directory.
- **Output:** `0.txt` / `nscript.dat` + `arc.nsa`/`.sar` archives + converted media,
  + `ons_args`, runnable by CONscripter, plus a conversion report.

## Scope and expectations

- Games that are mostly text, sprites, backgrounds, music, voices and choices
  convert well.
- Custom systems (minigames, battle systems, heavily scripted menus written in
  the engine's own scripting language) do **not** convert automatically. They
  are reported clearly, never silently broken.
- The deterministic pipeline is the source of truth, and the tool is fully
  usable offline.

## Architecture

```
            ┌──────────┐   ┌──────────┐   ┌────────────┐   ┌──────────┐   ┌────────────┐
 game dir → │ Detect   │ → │ Unpack    │ → │ Decompile  │ → │ IR       │ → │ ONS output │ → 0.txt + arc.nsa
            │ engine   │   │ archives  │   │ scripts    │   │ (common) │   │ + packing  │
            └──────────┘   └──────────┘   └────────────┘   └──────────┘   └────────────┘
                                 │                               ▲
                                 └─→ Asset decode/transcode ─────┘
```

Each source engine is compiled in (no dynamic plugins): its readers and
compiler live in `crates/conv-engines/src/<engine>/`, and the CLI has one
subcommand per engine (`crates/conv-cli/src/main.rs`).

Only the output stage (`conv-cons`) knows about NScripter and CONscripter,
so another output engine could be added later.

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
font setup, menu, `game`). Text is encoded to Shift-JIS with `encoding_rs`.

### Target engine: CONscripter

The target is [CONscripter](https://github.com/blankiekun/CONscripter), a
fork of [ONScripter-Yuri](https://github.com/YuriSizuku/OnscripterYuri)
(itself based on ONScripter-Jh). It is checked out as the `engine/`
submodule. ONScripter-Yuri was chosen because it builds for the web
(WebAssembly), Windows, Linux, macOS and Android from one tree, and adds
UTF-8 scripts, Lua and arbitrary resolutions to the NScripter command set.

**Rule for engine features.** Convert with existing NScripter/ONScripter
commands first. When a feature of the original game can't be reproduced
that way (or only with a workaround that visibly differs from the
original), add the feature to CONscripter instead of approximating it:

1. Implement it in `engine/src/cons/` as a new command or option, in the
   style of the surrounding ONScripter code, so it works on every platform
   the fork builds for (the web build especially).
2. Don't make it opt-in (the user, 2026-10-14): a new behaviour applies
   to every script, with no command or flag to turn it on, as bug fixes
   do; a new command takes effect where a script uses it. Check that it
   doesn't change the other games' builds for the worse.
3. Document it in the engine's `COMMANDS.md`.
4. Open the engine PR first, then the converter PR that uses it (the
   steps are in `CLAUDE.md`, "Rules").

**Startup options.** CONscripter reads command-line options from an
`ons_args` file in the game directory (whitespace-separated, up to 16).
The converter writes one with `--enc:sjis`, because ONScripter-Yuri reads
scripts as GBK by default. This is how every converted game passes
options to the engine, including in the web build.

**Building** (Arch Linux; `sdl2_mixer` from the package or built locally):

```sh
git submodule update --init engine
cmake -B engine/build -S engine -DCMAKE_BUILD_TYPE=RelWithDebInfo
make -C engine/build -j"$(nproc)"     # → engine/build/conscripter
```

`engine/script/cross_web.sh` builds the WebAssembly version (needs emsdk).

| Right-click menu / hide window | Everything behind the on-screen button bar | CONscripter has `rgosub *label` now (engine PR #20): a right click at a text wait gosubs and returns to the same wait. Not emitted yet: which menu a right click opens is each game's choice (its command menu, or hiding the window), to be set where the game's menus are written |

### Done with stock commands

Done without engine work (the stock commands are enough; the per-game
details, record layouts and handler addresses are in the game's document):
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
- **No web text overlay:** `ons_args` carries `--no-text-overlay`
  (engine #49), so the web player gets no text, choice or backlog hooks
  and draws no selectable text over the picture.
- **Right-click menu (not emitted yet):** CONscripter has `rgosub *label`
  (a right click at a text wait gosubs and returns to the same wait);
  which menu a right click opens is each game's choice, to be set where
  the game's menus are written.

## Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, …) into the `image` crate's types and save as PNG, or with
  `--webp` as near-lossless WebP (`docs/lessons.md`). Then sprite
  composition, resize to the target resolution, and alpha conversion according
  to `target_caps`. Run per-image work in parallel with `rayon`.
- **Audio:** keep OGG where possible; transcode the rest with an `ffmpeg`
  subprocess. Loop points (KiriKiri `.sli`, RealLive loop metadata) can't be
  carried over directly: either note them in the report or bake intro + loop
  into the output.
- **Video:** `ffmpeg` to H.264 + AAC MP4, which browsers and the system players play.
- **Packing:** write `arc.nsa` / `.sar` natively (simple formats) and
  optionally encode `nscript.dat`.

## Testing and validation

1. **Unit tests per engine** (`cargo test`), using small synthetic
   archives and scripts written by hand. Never commit commercial assets.
2. **Snapshots:** `tests/snapshot.sh` dry-runs every game present on the
   machine and says which games' `0.txt` changed.
3. **Linter** (`conscripter-convert lint`): every jump target exists,
   variable numbers are in range, referenced files exist in the archives,
   Shift-JIS is valid.
4. **Smoke test:** CONscripter's debug trace (`engine/DEBUG.md`) plays a
   build headless by itself and records every page, choice, missing file
   and script error; `engine/tools/cons_trace_check.py` checks it.
5. **Conversion report** per run: fraction of instructions converted,
   unsupported instructions with source locations, assets dropped or
   approximated.
## Legal and ethical guardrails

- The tool only works on a copy the user has installed; it never downloads
  games or ships assets.
- Decryption keys are supplied by the user, not kept in the tool.
- Converted output is for personal use; publishing it would distribute
  copyrighted assets. The README says so.

## Decisions

- First game: 最果てのイマ (Saihate no Ima), engine Xuse.
- Target: CONscripter (our ONScripter-Yuri fork, `engine/` submodule).
  Until 2026-10-12 the target was ogapee's ONScripter with no engine
  patches; now features the original games need and ONScripter lacks are
  added to the engine ("Target engine: CONscripter").
