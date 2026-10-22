# CONscripter-convert: design

Shared design of every game. Per game: `docs/games/<game>.md` (list in
`CLAUDE.md`); new game: `docs/new-game.md`; lessons: `docs/lessons.md`;
engine commands: the engine's `COMMANDS.md`.

Input: an installed game directory. Output: `0.txt` + `arc.nsa` archives +
converted media + `ons_args`, plus a conversion report, for CONscripter
(our ONScripter-Yuri fork, the `engine/` submodule).

## Scope

Text, sprites, backgrounds, music, voices and choices convert well.
Custom systems (minigames, battle systems, scripted menus) are reported,
never silently broken. The deterministic pipeline is the source of truth
and works offline.

## Architecture

Pipeline: detect engine, unpack archives, decompile scripts, IR, NScripter
output + packing; assets are decoded or transcoded beside it.

Each source engine is compiled in: readers and compiler in
`crates/conv-engines/src/<engine>/`, one CLI subcommand per engine
(`crates/conv-cli/src/main.rs`). Crates: `conv-core` (the IR),
`conv-engines` (readers), `conv-cons` (IR to `0.txt`, NSA writer),
`conv-cli`. Only `conv-cons` knows NScripter and CONscripter.

## The IR

Engine-neutral instructions in labelled blocks, serde-serializable (JSON)
so tools can read and diff it. The full list is the `Op` enum in
`crates/conv-core/src/ir/mod.rs`; the families:

- **Text:** `Say { speaker, text, voice }`, `ClearText`, `TextboxShow/Hide`, `WaitClick`, `Wait(ms)`
- **Graphics:** `Bg { image, transition }`, `SpriteShow { slot, image, x, y, transition }`,
  `SpriteHide { slot }`, `Cg { image }`, `ScreenEffect { kind, params }`
- **Audio:** `BgmPlay/Stop { fade }`, `SePlay { channel, file }`, `Voice { file }`
- **Control flow:** `Label`, `Jump`, `Call`/`Return`, `Choice(Vec<Option{ text, target, cond }>)`, `If { cond, then, else }`
- **Variables:** `Set { var, expr }`, named and marked `Local`/`Global`
- **Escape hatches:** `Unsupported { engine, raw, loc }`, `Comment`

Every instruction carries a `SourceLoc` (file and line or byte offset).
Sprite layers (body + face + accessories) compose *before* the IR, which
refers to the merged image. The output stage maps named variables to
NScripter numbers (`%n`, `$n`), globals in the `globalon` range.

## Output stage (IR to NScripter)

| IR | NScripter output |
|---|---|
| `Label` / `Jump` / `Call` | `*label`, `goto`, `gosub` / `return` |
| `Say` | `dwave 0,"voice"` + text line with name prefix and `\` / `@` click waits |
| `Bg` | `bg "file",effect#` |
| `SpriteShow` | `lsp n,":a;file",x,y` + `print effect#` (`ld` has only 3 fixed positions) |
| `Choice` | `select` / `csel` |
| `BgmPlay` / `SePlay` | `bgm`, `bgmstop`, `dwave n` |
| transitions | `effect` definitions; mask wipes become mask images |
| `If` / `Set` | `if %n == ...`, `mov` / `add` |
| `Unsupported` | `;` comment + entry in the conversion report |

It also writes the `*define` section (`effect` defs, window and font
setup, menu, `game`). The script is UTF-8.

## Target engine: CONscripter

[CONscripter](https://github.com/blankiekun/CONscripter) forks
[ONScripter-Yuri](https://github.com/YuriSizuku/OnscripterYuri) (itself
ONScripter-Jh): one tree builds for web (WebAssembly), Windows, Linux,
macOS and Android, with UTF-8 scripts, Lua and arbitrary resolutions on
top of NScripter. Existing NScripter commands first; an engine feature
when they can't reproduce a game feature (`CLAUDE.md`, "Rules"). Build
commands: `CONTRIBUTING.md`, `docs/tools.md`.

**Startup options.** Every converted game, web build included, passes
options in `ons_args` (whitespace-separated, up to 16). The converter
writes `--enc:utf8` (ONScripter-Yuri reads GBK by default; CONscripter
lays UTF-8 text out as a Shift-JIS one's, `engine/COMMANDS.md`).

### Done with stock commands

Per-game details live in the game's document.
- **Window caption:** `caption` converts the title from the script's
  encoding to UTF-8, so Japanese titles show in the browser tab and the
  window manager.
- **Movies:** ffmpeg to H.264 + AAC MP4 (loose files in `movie/`), played
  with `bgmstop` + `mpegplay "movie\name.mp4",skippable`. Web: a `<video>`
  element (muted when the browser blocks autoplay with sound); Windows and
  Android: the system player; Linux and macOS builds skip movies.
- **BGM loop points:** `LOOPSTART`/`LOOPEND` Vorbis comments in each
  `.ogg`; SDL_mixer (2.0.2+) loops sample-exact when `bgm` plays looped
  (`LOOPSTART` without an end is ignored). The loop survives saves and
  works in stock ONScripter. `bgm "(seconds)file"` loop-back is out: it is
  not saved and never fires in ONScripter-Yuri (`playSound` registers
  `Mix_HookMusicFinished` only when `Mix_PlayMusic` fails).
- **Backlog speaker names and voice replay:** every text line starts with
  a `[name|voice]` tag. With a `pretextgosub` (ours only runs `saveon`)
  ONScripter keeps a page's tag in the log and in saves; the backlog reads
  it with `gettaglog`, shows the name above the page and replays the voice
  on a name click.
- **Ruby:** `(base/reading)`, added after escaping (which turns the text's
  own parentheses full-width), drawn by `rubyon`. `rubyon` puts the ruby in
  the line gap and draws text a line gap lower, so `setwindow` and the
  backlog's `strsp` start a line gap higher. ONScripter reads the markup
  bytes singly: single-byte padding treats them like colour tags. Readings
  wider than their base push the rest of the line right.
- **Web text overlay:** `ons_args` omits `--no-text-overlay`, so the web
  player gets the text, choice and backlog hooks and lays selectable text
  over the picture (scanning, Yomitan). The player toggles it while the
  game runs (`Module.consNoTextOverlay`, `COMMANDS.md`, "Web hooks").
- **Right-click menu (not emitted yet):** `rgosub *label` gosubs at a
  right click during a text wait and returns to the same wait; each game
  picks the menu, where its menus are written.

## Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, ...) via the `image` crate's types to PNG, or `--webp` near-lossless
  WebP (`docs/lessons.md`); then sprite composition and resize to the
  target resolution, per image in parallel (`rayon`).
- **Audio:** OGG stays; `ffmpeg` transcodes the rest. Loop points
  (KiriKiri `.sli`, RealLive metadata) don't carry over: note them in the
  report or bake intro + loop into the output.
- **Video:** `ffmpeg` to H.264 + AAC MP4.
- **Packing:** `arc.nsa` / `.sar` written natively; `nscript.dat` optional.

## Testing and validation

1. **Unit tests** (`cargo test`) on small hand-written synthetic archives
   and scripts; commercial assets stay out of the repo.
2. **Snapshots:** `tests/snapshot.sh` dry-runs every game on the machine
   and names the games whose `0.txt` changed.
3. **Linter** (`conscripter-convert lint`): jump targets exist, variable
   numbers in range, referenced files in the archives, text valid.
4. **Smoke test:** CONscripter's debug trace (`engine/DEBUG.md`) plays a
   build headless, recording every page, choice, missing file and script
   error; `engine/tools/cons_trace_check.py` checks it.
5. **Conversion report** per run: fraction converted, unsupported
   instructions with source locations, assets dropped or approximated.

## Legal guardrails

The tool works on the user's installed copy: it downloads no games, ships
no assets and keeps no keys (the user supplies them). Output is for
personal use, since publishing would distribute copyrighted assets (the
README says so).
