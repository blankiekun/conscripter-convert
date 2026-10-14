# CONscripter-convert — Plan

Converts Japanese visual novels (that you own) into games playable on
**[CONscripter](https://github.com/blankiekun/CONscripter)**, our fork of
ONScripter-Yuri, and helps reverse-engineer engines it doesn't support yet —
optionally with AI agents. The converter and the engine are developed
together: when a game feature can't be reproduced with the engine's existing
commands, the feature is added to CONscripter (§4.1).

- **Games:** each converted game has its own document in `docs/games/`
  (formats, opcodes, progression variables, status, leftovers); the list
  is in §6. How to start a new game: `docs/new-game.md`. This file holds
  what is shared by every game: architecture, IR, output stage, engine
  additions and lessons.

- **Language:** Rust (cargo workspace) for the converter; C++ for engine
  features, which go into CONscripter (the `engine/` submodule).
- **Input:** an installed game directory.
- **Output:** `0.txt` / `nscript.dat` + `arc.nsa`/`.sar` archives + converted media,
  + `ons_args`, runnable by CONscripter, plus a conversion report.

## 1. Scope and expectations

- Games that are mostly text, sprites, backgrounds, music, voices and choices
  convert well.
- Custom systems (minigames, battle systems, heavily scripted menus written in
  the engine's own scripting language) do **not** convert automatically. They
  are reported clearly, never silently broken.
- The deterministic pipeline is the source of truth. AI agents only **propose**
  artifacts (profiles, opcode tables, mappings) that the pipeline then
  validates. Agents **only run when explicitly invoked** (`conscripter-convert ai …`);
  `convert`, `lint` etc. never call the API. The tool is fully usable offline.

## 2. Architecture

```
            ┌──────────┐   ┌──────────┐   ┌────────────┐   ┌──────────┐   ┌────────────┐
 game dir → │ Detect   │ → │ Unpack    │ → │ Decompile  │ → │ IR       │ → │ ONS output │ → 0.txt + arc.nsa
            │ engine   │   │ archives  │   │ scripts    │   │ (common) │   │ + packing  │
            └──────────┘   └──────────┘   └────────────┘   └──────────┘   └────────────┘
                                 │                               ▲
                                 └─→ Asset decode/transcode ─────┘
                 ▲                        ▲                ▲                ▲
                 └──────── AI agent layer (optional): proposes profiles, opcode
                           tables, macro mappings, fixes; validated by the pipeline
```

Each source engine is a Rust trait implementation, compiled in (no dynamic
plugins):

```rust
pub trait Engine {
    fn id(&self) -> &'static str;
    fn detect(&self, dir: &Path) -> Confidence;
    fn open_vfs(&self, dir: &Path, profile: &Profile) -> Result<Box<dyn Vfs>>;
    fn decode_asset(&self, entry: &VfsEntry, data: &[u8]) -> Result<Asset>;   // Image/Audio/Video/Raw
    fn decompile(&self, vfs: &dyn Vfs, profile: &Profile) -> Result<ir::Program>;
}
```

Only the output stage (`conv-cons`) knows about NScripter and CONscripter,
so another output engine could be added later.

## 3. The IR (core design decision)

A small, engine-neutral list of instructions, grouped into labelled blocks,
and serializable (serde → JSON) so tools and agents can read and diff it.

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

## 4. Output stage (IR → NScripter)

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

### 4.1 Target engine: CONscripter

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

1. Implement it in `engine/src/onsyuri/` as a new command or option, in the
   style of the surrounding ONScripter code, so it works on every platform
   the fork builds for (the web build especially).
2. Keep it opt-in: a new command, or an option in `ons_args`. Scripts that
   don't use it must behave exactly as in stock ONScripter-Yuri.
3. Document it in the engine's `ADDITIONS.md`
   and here in §4.2, with the game feature that needed it.
4. Commit and push in the engine repo, then commit the updated submodule
   pointer here together with the converter change that uses it.

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

### 4.2 CONscripter additions

Engine features added for converted games (details in the engine's
`ADDITIONS.md`):

| Command | Game feature that needed it |
|---|---|
| `spfilter` (colour filters on sprites, kept in saves) | 最果てのイマ's effect presets (0x88/0x90): lighting tints on bust-ups, plus negative, greyscale and brightness filters. Replaced ~930 pre-tinted PNGs (about 130 MB of `arc.nsa`) |
| `pushscreen` / `popscreen` (save and restore background, sprites, filters and BGM; kept in saves) | Link scenes, which can change the whole screen before returning to their page. Replaced the converter's file-order guess of what was on screen |
| `delsavegame` (delete a save and its thumbnail) | The save menu's delete button. Replaced a global flag per slot that only hid the save; a slot an older build flagged is deleted when the menu lists it |
| `cons_slide.dll`, `cons_zoom.dll` (effect 99 plugins: slides in 8 directions, four zooms) | 0x76 transitions: Slide in/out (subs 10-25, 7 uses) and Zoom (37-40, 116 uses), decoded from the exe (`transition_style` in the Xuse converter). Scroll (41-44) maps to ONScripter's effects 11-14 and Mosaic (27) to 16 then 17, without engine work |
| `cons_flip.dll` (effect 99 plugin: a picture turning edge-on, either axis, growing or shrinking) | 0x76 RollFlip (subs 32-35, 24 uses): seen in the original under wine (the new picture widens from a centre line, or the old one narrows into it), sizes sin/cos of progress × 90° from the exe (0x446460); also TvPowerSwitch (sub 48, 11 uses) as a 700 ms vertical collapse. CONscripter 0.15.0 |
| `cons_melt.dll` (effect 99 plugin: the old picture melts down off the screen behind a smooth random edge, falling columns stretched) | 0x76 MeltDown (subs 45-47, 3 uses): filmed under wine (sub 45), the edge spline and stretch from the exe (0x445310, 0x4458e0). CONscripter 0.17.0 |
| `windowtrans opacity` | The settings' ウィンドウ透過 row (the original's ウインドウの不透明度 slider): the text window picture drawn at 0-255 over its own alpha. CONscripter 0.16.0 |
| WebP in the web build (libwebp linked in; emscripten's SDL_image has no WebP) | Archive size on the web: `--webp` stores images as near-lossless WebP, 218 MB instead of 457 MB of PNG (`arc.nsa` 765 MB instead of 992 MB). Opt-in, because builds whose SDL_image lacks libwebp can't read it |
| `screenfilter` (the `spfilter` filters on the picture behind the text window, 4 slots, kept in saves and by `pushscreen`) and `spfilter`/`screenfilter` kind 5 (monotone: greyscale plus r, g, b offsets) | 0x70 screen filters, 395 uses: darkening (brightness down to -250), greyscale and sepia (monotone palette `pal0001.pal`, grey + (28, -9, -38)) flashbacks, negatives, colour overlays, each faded in by `print` |
| `spfilter`/`screenfilter` kinds 6 (blur: box mean over x, y ranges), 7 (hue and saturation) and 8 (lightness inverted) | 0x70's Smooth (40 uses, mostly a 3 px blur for flashbacks and wind), HueSatura (4, desaturating) and BrightnessInvert (1) |
| `screenshake` and `screenripple` (effects that move by themselves on the picture behind the text window, 8 shared slots, stepped like sprite animations, kept in saves) | 0x8c's Animation-Vibrate (12 uses: shaking while a line is shown) and Animation-Replace (25: a ripple through `mask1001.png` and others, 「視界が歪んだ」). The parameters are the original modules'. 0x70's Filter-Replace (16 uses, wind and heat haze) is a still `screenripple` in slots 5-8 |
| `screenpulse` (a tinted copy of the picture that swells over it and fades, sin(πt); same slots as `screenshake`) | 0x6f "surprise" pulses (87 uses): 10-20 % zoom, opacity 100-200, untinted, light red or white |
| `!zNNN` text size tag (NNN percent of the window's font size, on the baseline, scaled width) | 0x49 text styles: shouting at 125-175 % and mumbling at 75-95 % in about 380 messages (NScripter text has colour tags but no size). Colours from the same records use `#RRGGBB` |
| Text hooks in the web build (`onsyuriText`, `onsyuriChoice`, `onsyuriOverlay`) | Not for a converted game: chimimouryou lays each page's text over the game for Yomitan and reading statistics. Was a patch kept in chimimouryou (`scripts/onsyuri/text-hook.patch`) |

Planned, from limits hit by converted games (each one is worked around
in the script today), roughly in order of payoff:

| Game feature | Workaround today | Engine feature that would fix it |
|---|---|---|
| Right-click menu / hide window | Everything behind the on-screen button bar | Send right-click to a script label (`rgosub`-style); not planned for now |

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

## 5. Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, …) into the `image` crate's types and save as PNG, or with
  `--webp` as near-lossless WebP (§13.1). Then sprite
  composition, resize to the target resolution, and alpha conversion according
  to `target_caps`. Run per-image work in parallel with `rayon`.
- **Audio:** keep OGG where possible; transcode the rest with an `ffmpeg`
  subprocess. Loop points (KiriKiri `.sli`, RealLive loop metadata) can't be
  carried over directly: either note them in the report or bake intro + loop
  into the output.
- **Video:** `ffmpeg` to H.264 + AAC MP4, which browsers and the system players play.
- **Packing:** write `arc.nsa` / `.sar` natively (simple formats) and
  optionally encode `nscript.dat`.

## 6. Engine support order

| Order | Engine | Why |
|---|---|---|
| 1 | **NScripter** | Round-trip test (NScripter → IR → NScripter) of the IR and output stage, no reverse engineering involved |
| 2 | **Xuse** (最果てのイマ COMPLETE) | First real target, an undocumented in-house engine reversed from the exe. **Finished 2026-10-14** |
| 3 | **Nitroplus NSS** (装甲悪鬼村正) | In progress (branch `nitroplus`, record in `docs/games/muramasa.md`): NSS compiles to NScripter, Lua as the fallback |
| 4+ | KiriKiri2/KAG, RealLive/Siglus, CatSystem2, BGI/Ethornell, Yu-ris, Artemis, Majiro, Softpal… | Chosen by which games you want to convert next |

### Per-game documents

| Game | Engine | Branch | Document | Status |
|---|---|---|---|---|
| 最果てのイマ COMPLETE | Xuse | `xuse` | [`docs/games/saihate-no-ima.md`](docs/games/saihate-no-ima.md) | Finished 2026-10-14 |
| 装甲悪鬼村正 | Nitroplus NSS | `nitroplus` | [`docs/games/muramasa.md`](docs/games/muramasa.md) | In progress |

A game's document holds what is specific to it: the install and build
command, file formats, the opcode or command table with handler
addresses, how its system screens map to CONscripter, the progression
variables (`docs/new-game.md` step 11.2), the status and the optional
leftovers. Start one from step 0.4 of `docs/new-game.md`.

**For every new engine:**
- **Identify it** (M0): file extensions, archive magic bytes, strings in
  the `.exe`, bundled DLLs; compare against GARbro's format list.
- **Existing work to learn from:** GARbro (archive and image formats),
  arc_unpacker, VNTextPatch, rlvm/RLdev, KrkrExtract. Use them as
  references, port the logic into Rust, or (for C++ code) link through
  `cxx`. Check each license before reusing code.
- **Per-game profiles** (`profiles/<game>.toml`, planned) can map custom
  macros to IR, define sprite layer rules, set per-game decryption
  parameters (supplied by the user) and override cases where automatic
  conversion gets it wrong. Decryption keys found go there, not in the
  tool's code. Profiles are also the main thing the AI agents write
  (§8).

## 7. Project layout

Cargo workspace:

```
crates/
  conv-core/         ir.rs, vfs.rs, profile.rs, report.rs, source_loc.rs
  conv-engines/      nscripter/, kirikiri/ (xp3, kag, tlg), reallive/, ...
  conv-assets/       image codecs, compositing, ffmpeg wrapper
  conv-cons/         emit.rs, varalloc.rs, effects.rs, nsa.rs, caps.rs, lint.rs
  conv-walker/       IR interpreter: branch exploration, reachability, missing files
  conv-ai/           Claude API client, agent loops, tools, response cache
  conv-mcp/          MCP server exposing the tools to Claude Code (§8.5)
  conv-cli/          `conscripter-convert` binary
engine/              CONscripter (git submodule): the target engine, with our additions
profiles/
tests/               synthetic fixtures + insta snapshot tests
target_caps.toml
```

**Main crates:** `clap`, `serde` / `serde_json` / `toml`, `binrw` (binary
parsing), `memmap2`, `encoding_rs`, `image` / `png`, `flate2`, `rayon`,
`thiserror` / `anyhow`, `tracing`, `insta` + `proptest` (tests), and
`reqwest` + `tokio` (AI layer only, behind the `ai` cargo feature).

**CLI:**

```
conscripter-convert inspect <gamedir>                 # engine, archives, script count, unsupported features
conscripter-convert extract <gamedir> -o out/         # unpack and decode only
conscripter-convert convert <gamedir> -o ons/ [--profile p.toml] [--target ons]
conscripter-convert lint ons/                         # check the generated script
conscripter-convert walk ons/|<gamedir>               # IR walker report
conscripter-convert run ons/                          # launch CONscripter on the result
conscripter-convert ai <task> <gamedir> [...]         # run an agent (see §8)
conscripter-convert mcp                               # serve tools over MCP (stdio)
```

## 8. AI agent layer

### 8.1 Principles

1. **Proposals, not edits.** Agents write candidate artifacts (profile
   fragments, opcode tables, mapping files) to `.conscripter-convert/proposals/`. Nothing
   reaches the output until it passes validation (round-trip, lint, IR
   walker, smoke run) and you approve it with `conscripter-convert ai accept <id>`.
   Agents never start on their own: `convert` failing only *suggests* the
   matching `conscripter-convert ai …` command in its report.
2. **Validated by tools.** Every agent gets the validators as tools, so it can
   check its own hypotheses before proposing (for example: "decompile all
   scenes with this opcode table — do the instruction lengths line up?").
3. **Sandboxed.** Tools only get read access to the game's VFS and write access
   to a scratch workspace. No arbitrary shell, no network beyond the API.
4. **Opt-in data sharing.** Sending script text or assets to the API is a
   choice set in config (`ai.send_script_text`, `ai.send_images`), defaulting
   to off and asked about on first use.
5. **Reproducible and cheap to re-run.** Every request/response is cached on
   disk, keyed by a hash of the request. Re-running a conversion replays cached
   answers instead of calling the API again. Full transcripts are saved next to
   each proposal.
6. **Bounded.** Each agent has a maximum number of turns, a token budget, and
   a dollar cap from config. `inspect` estimates the cost before running.

### 8.2 Agents

| Agent | Input | Tools | Output (proposal) | Validated by |
|---|---|---|---|---|
| **Format sleuth** | Unknown archive or image file | `hexdump(range)`, `entropy(range)`, `find_magic`, `try_decompress(algo, range)`, `list_known_formats` | A format description (header layout, index structure, compression) + test vectors | Extracting the archive with the generic parser driven by that description; sizes and checksums add up |
| **Opcode inferrer** (bytecode engines) | Script files, partial opcode table | `hexdump`, `disasm(table, file)`, `opcode_stats`, `find_strings(sjis)`, `xref(opcode)` | `opcodes.toml`: opcode → length, argument types, IR meaning | All scripts decompile with no stray bytes; text appears in sensible order; IR walker finds no dangling jumps |
| **Macro mapper** (KAG/TJS etc.) | Macro definitions + example uses | `read_script`, `grep_vfs`, `expand_macro`, `ir_preview` | Profile `[macros]` entries mapping custom macros to IR sequences | Conversion of the scenes that use them, IR diffs, lint |
| **Profile drafter** | `inspect` report | All read-only tools + the agents above as sub-tasks | `profiles/<game>.toml` | Full convert + lint + walker |
| **Repair agent** | Lint / walker / smoke-run failures | `read_report`, `ir_slice(loc)`, `read_source(loc)`, `diff_ir` | Profile overrides that fix the cause upstream. Never patches the generated `0.txt` directly. | Re-run; failure count goes down and none are added |
| **Visual checker** | Screenshots from the headless smoke run + matching original frames, if available | Image input | List of suspected problems (sprite offset, missing alpha, wrong layer order) with source locations | Human review (advisory only) |
| **Translator** (optional) | Extracted `Say` text + glossary | Glossary lookup | Translated text table, kept separate from the IR so the original stays intact | Line-length and encoding checks, name consistency against the glossary |

### 8.3 Provider and API usage

- **Claude API** over HTTPS (`POST /v1/messages`). There is no official Rust
  SDK, so `conv-ai` is a thin typed client (`reqwest` + `serde`) and a
  hand-written tool-use loop (`while stop_reason == "tool_use"`).
  Credentials come from `ANTHROPIC_API_KEY`, never from config files that
  could get committed.
- **Models:** default `claude-opus-5-5` for the reasoning-heavy agents
  (opcode inference, format sleuth, repair). Per-agent override in config, for
  example a cheaper model for bulk translation drafts. Model IDs live only in
  config so they're easy to update.
- **Thinking and effort:** adaptive thinking (default on this model). Depth is
  set per agent with `output_config.effort` — `high` for reverse-engineering
  agents, `low`/`medium` for bulk work.
- **Tool calls:** tools are declared with `strict: true` JSON schemas and
  `tool_choice: auto` (forced tool choice is rejected by current models).
  Every tool input is re-validated in Rust before running.
- **Structured outputs:** final proposals are requested with
  `output_config.format` (a JSON schema generated from the Rust types with
  `schemars`), so they deserialize straight into the profile/opcode structs.
- **Streaming:** used for long agent turns to avoid HTTP timeouts.
- **Refusals:** always check `stop_reason` (including `refusal`) before
  reading content. Server-side refusal fallback is enabled by default
  (`fallbacks: "default"` with the `server-side-fallback-2026-07-01` beta
  header); it can be turned off in config.
- **Prompt caching:** stable prefix first — tool definitions, then a frozen
  system prompt with the IR spec, NScripter reference and the engine's
  current notes — with a `cache_control` breakpoint after it. Per-request data
  (hex dumps, script excerpts) goes after the breakpoint. Track
  `usage.cache_read_input_tokens` in the cost report.
- **Batch API:** translation and per-scene macro mapping are sent as message
  batches (about half the cost, asynchronous), with results matched by
  `custom_id`.
- **Errors:** retry on 429/5xx/connection errors with backoff; fail fast on
  400/404 and show the API's message.

### 8.4 Agent context

Agents get engine knowledge from a `knowledge/` directory (format notes, opcode
tables found so far, NScripter command reference) that grows as games are
converted. Accepted proposals feed back into it, so the next game on the same
engine starts with more knowledge.

### 8.5 Interactive mode: MCP server

`conscripter-convert mcp` exposes the same tools (VFS browsing, hexdump, disasm, IR
preview, convert, lint, walk) over MCP. That lets you do interactive reverse
engineering with Claude Code or another MCP client. The built-in agents cover
the batch, non-interactive cases.

## 9. Testing and validation

1. **Unit and snapshot tests per engine** (`insta`), using small synthetic
   archives and scripts written by hand. Never commit commercial assets.
2. **Property tests** (`proptest`): NSA pack → unpack round-trip,
   Shift-JIS encoding, variable allocation.
3. **Round-trip test** with NScripter: script → IR → script should come out
   equivalent.
4. **Linter:** every jump target exists, variable numbers are in range,
   referenced files exist in the archives, Shift-JIS is valid.
5. **IR walker:** follows every choice branch; reports unreachable code,
   endless loops and missing files.
6. **Smoke test:** CONscripter headless (Xvfb or the SDL dummy driver)
   with auto-advance; checks that it reaches the ending labels without errors
   and captures screenshots for the visual checker.
7. **AI layer tests:** a mock API server replays recorded transcripts, so agent
   loops are tested deterministically without network access or cost. A small
   eval set (known formats and opcode tables with the answers removed) measures
   agent accuracy when prompts or models change.
8. **Conversion report** per run: fraction of instructions converted,
   unsupported instructions with source locations, assets dropped or
   approximated, AI proposals and their validation status, API cost.

## 10. Milestones

- **M0:** identify the first game's engine (最果てのイマ) and archive formats (from the
  installed files); build the target engine, write `target_caps.toml`; set
  up the cargo workspace and IR types. (Done with ogapee's ONScripter; the
  target is now CONscripter, §4.1.)
- **M1:** NScripter output stage, NSA packer, NScripter input engine,
  round-trip tests, linter.
- **M2:** the first game's engine: archive reader, image/audio decoders,
  script decompiler, plus a game profile. First full conversion. (If the
  engine is undocumented, pull the AI layer from M4 forward, since the format
  sleuth and opcode inferrer are most useful exactly here.)
- **M3:** asset pipeline polish (composition, resizing, alpha, audio loops),
  IR walker, conversion report.
- **M4:** AI layer foundation: API client, tool loop, response cache, cost
  limits, mock-server tests. First agents: **format sleuth**, **opcode
  inferrer** and **repair agent**.
- **M5:** **macro mapper** and **profile drafter**; MCP server.
- **M6:** second engine (whichever game comes next), headless smoke tests
  and **visual checker**.
- **M7 (optional):** translator agent via the Batch API.

## 11. Legal and ethical guardrails

- The tool only works on a copy the user has installed; it never downloads
  games or ships assets.
- Decryption parameters live in user-supplied profiles, not in the tool.
- Converted output is for personal use; publishing it would distribute
  copyrighted assets. The README says so.
- AI features send game content to a third-party API only with explicit
  opt-in, and only the parts each agent needs.

## 12. Decisions

- First game: 最果てのイマ (Saihate no Ima); its engine is identified in M0.
- Target: CONscripter (our ONScripter-Yuri fork, `engine/` submodule).
  Until 2026-10-12 the target was ogapee's ONScripter with no engine
  patches; now features the original games need and ONScripter lacks are
  added to the engine (§4.1).
- AI agents run only when explicitly invoked with `conscripter-convert ai …`.

## 13. Lessons learned and ways to speed up conversion

General lessons, collected while converting games. Each one is stated
for any engine, with the case that taught it as an example (most come
from 最果てのイマ, Xuse; its details are in `docs/games/saihate-no-ima.md`).
The steps that apply them are in `docs/new-game.md`.

### 13.1 Lessons learned

**Know the game before the bytecode.**
- **Learn the game's mechanics first** (wiki, playthrough video): the
  route choice mechanic, loops, endings. Example: 最果てのイマ's route
  choices are hyperlinks in the text. A no-click trace hit a dead end
  that restarts the chapter, which looked like a control-flow bug until
  the link targets were followed.
- **A loop is not necessarily a bug.** Before "fixing" one, check whether
  the original loops there too (dead ends, retries, back to the title).
- **Ask what to drop.** Debug menus, right-click menus and side rooms
  can each cost a session. The user decides.
- **A 100 % save unlocks the extras at once.** Download one instead of
  playing or reverse-engineering the unlock bits.

**Reading the exe.**
- **Rank opcodes by use count** and convert from the top: a handful of
  opcodes make most of the visible game.
- **Strings name handlers.** Error messages, logged source-file names
  (`Story.cpp` is bookkeeping, not display) and RTTI class names settle
  an opcode's meaning faster than tracing its logic. Dump them first.
- **MSVC RTTI maps factory ids to classes:** `.?AV<class>@@` string →
  type descriptor → complete object locator → vtable. The constructor
  that stores the vtable, with a static `{type id, factory}` table in
  `.rdata`, gives the class for each id.
- **Jump tables can be two-level:** a byte table indexed by the
  sub-number, then an address table. Reading the address table alone
  gives wrong handlers.
- **Look for divide-by-constant magic numbers.** A division compiles to
  a multiply and shift (`imul 0x14f8b589; sar 13` is ÷ 100000). Searching
  for the literal divisor finds nothing and wrongly suggests a lookup
  table.
- **Follow a field into the code before naming it**, and never name it
  after the values it takes. Examples: a 0/128/255 byte that looked like
  black/grey/white was an acceleration; a field whose values (1000, 500,
  0) looked like fade milliseconds was a sort depth, which the handler
  passed to the object creator (fixed in `6b22f6d`).
- **An operand can refer to state defined elsewhere.** An opcode may
  only name a preset that another opcode defines in a setup script; the
  context around the definitions (evening and night backgrounds next to
  colour presets) can fix their meaning.
- **Object lifetimes depend on mode bits.** Check across the corpus which
  modes keep an object (and what is attached to it) and which recreate
  it, before tracking state.

**Checking decoded data.**
- **Check the whole corpus, not one sample:** every message parses,
  every jump target exists, every checksum verifies. Corpus-wide checks
  catch edge cases that one scene hides.
- **Find out how many index spaces a jump uses** (per-script labels,
  global tables, implicit entries, labels on marker instructions).
  Getting it wrong still produces a script that runs, so check it with
  a trace, not by eye.
- **Cross-check references against asset names.** Name prefixes in the
  archives are a free oracle: checking each sprite reference's character
  against its image name found 623 mismatches in one pass.
- **A prefix check can't catch off-by-one inside a block.** The next
  image is usually the same character, so a 0-based misread still passed
  97 %. Count references to degenerate assets (empty files, 1×1
  placeholders) instead: 1,639 references dropped to 0 after the fix.
- **Count pages as they are displayed, not in file order.** Setup
  scripts and debug menus come first in the file but are never shown.
- **Check the routes against a walkthrough before calling a game done.**
  Simulate each route on both sides (the source with the original's
  run-time rules, the converted `0.txt`) and diff page by page,
  including the wrong-order clicks the walkthrough warns about. Static
  reading and spot runs missed a link that stayed clickable after a
  jump skipped its record (`docs/new-game.md` step 11).

**Matching the original.**
- **Film the original instead of guessing.** Run it under wine (silent,
  its own Xvfb display, a script copy patched to slow an effect down,
  shots at fixed fractions), then read the effect's kernel in the exe for
  the exact curve and fit the frames to it. Static reading had got
  transitions wrong.
- **Use reference screenshots of the original** for layout: measuring
  pixels settles the text box and name plate in one pass.
- **A wine prefix shares the user's home by default.** Its Documents,
  Desktop etc. are symlinks to the real folders, and games save there.
  Replace them with real folders before the first run.
- **Effects ONScripter can't draw: approximate in the converter first,
  then add them to the engine.** Pre-tinted sprite copies (~930 images,
  130 MB) were replaced by the engine's `spfilter`, which also follows
  the game's control flow instead of a file-order guess.
- **Small effect plugins beat general machinery.** Each transition family
  became a short `cons_*.dll` effect-99 plugin, checkable on Xvfb
  mid-effect. Release them in batches: a CONscripter release is a
  version bump and a tag, and CI builds every platform.

**Target engine facts (ONScripter / CONscripter).**
- **ONScripter ignores SIGTERM while stuck in a script loop.** Always run
  it under `timeout -k 5 …`.
- **Sprite numbers set the stacking order, and saves record them.**
  Lower `lsp` numbers draw on top, so dialogs need the lowest slots.
  Renumber menus and bars freely, but keep the game's own sprites at
  fixed numbers so old saves still load.
- **The font must be fixed-width.** ONScripter draws every character in
  a full-width cell, so a proportional font leaves gaps after narrow
  glyphs. Use IPA Gothic (`ipag.ttf`).
- **Read tracking is by script byte offset.** `kidoku.dat` marks
  positions in `0.txt`; after a rebuild it marks the wrong text. Delete
  it.
- **The target's image decoder limits the asset formats.** Check a new
  format with one test file in every target build (the web build
  especially) before changing the converter. WebP needed CONscripter's
  own libwebp on the web.
- **Judge lossy image quality on the worst region,** not the average:
  crop the worst block of the difference image and compare it enlarged.
  Lossy WebP's 4:2:0 subsampling blurred small coloured detail at every
  quality level; near-lossless was fine at half the PNG size.

**Process.**
- **Never delete an output folder without listing it first.** Users put
  their own files there (an `onscripter` binary was lost once).
- **Keep a handoff file** (`<game>.md`) with a "Resume here" section.
  Context resets happen mid-task, and re-deriving state is expensive.
- **Handoff notes go stale.** When an item is done, grep `PLAN.md`, the
  game's document and the handoff file for it.
- **Parallel agents need separate branches and worktrees** (`xuse`,
  `nitroplus`), branches based on `origin/main`, and staging by path
  name: the other agent's uncommitted files share the tree.

### 13.1b Lessons from 装甲悪鬼村正 (Nitroplus NSS, 2026-10-13)

- **With source scripts, compile by running them at convert time.** Story
  scenes are straight runs of helper calls. Inlining the helpers, tracking
  objects and variables, and emitting what the player sees at each wait
  gave a readable build in one session. Only what really depends on the
  player has to become runtime code.
- **Find the runtime state statically, then keep it small.** Story flags
  = variables the story files both set and test (about 190 of thousands).
  Everything else (helper scratch, positions, names) stays convert-time.
  A choice or a flag test forks the run; branches that lead to different
  files dispatch at the end of the file, the rest merge.
- **Pure-logic library code that loops over flags** (the route ranking)
  can't be traced. Port it as a small intrinsic and test it with an IR
  interpreter against a hand-worked example.
- **Whole-corpus runs find what single scenes don't.** Two engine-killing
  bugs only showed on the full game: more than 254 transition effects
  (`effect` numbers stop at 255) and a half-width character in vertical
  text (CONscripter exits silently). Run the whole game, at least as a
  dry run, after every feature.
- **Engine facts worth knowing early:** `:a` on a JPEG uses its right
  half as the mask; loose files with Japanese names aren't found (pack
  them); `;$Sw,h` sets any screen size, `,valueN` raises the global
  border so more locals are saved per slot; `print 10,ms` works without
  an `effect` definition.
- **Check duplicate definitions for comment blocks** before deciding which
  one is live (Muramasa keeps old helper versions inside `/* */`).
- **Merging fork states:** a variable set again after the fork must keep
  that later value. Small synthetic file sets in unit tests caught this.
- **Long waits:** the shell tool kills a foreground command after 10
  minutes (exit 137, looks like OOM). Run long jobs in the background and
  wait for the notification. Max RSS includes the mapped archives, so it
  overstates real memory use.
- **Let the user watch:** a progress page (artifact) with screenshots,
  republished at milestones, lets the user follow from their phone.

### 13.2 Ways to speed up conversion

Tooling worth building, in rough order of payoff. For 最果てのイマ the
Python scripts in `research/xuse/` did the job; build each one into the
converter when a second game needs the same thing:

1. **A static tracer in the converter** (`conscripter-convert trace`, a Rust port of
   `research/xuse/nstrace.py`). It runs the emitted `0.txt` without
   ONScripter, counts displayed pages, prints the script-to-script path,
   and flags hot loops. A 100,000-page trace of the Python version takes 0.5 s,
   compared with minutes per real ONScripter run. Add a "click link N"
   option so each route can be walked.
2. **A smoke-test command** that automates the manual recipe: copy `0.txt`,
   symlink the archives, insert `autoclick`, add a displayed-page counter
   and screenshots at chosen pages, and run headless under `timeout -k 5`.
   Today this is done by hand on every check.
3. **Faster rebuilds** matter less than expected. A full `convert-game`
   takes a few minutes, mostly writing the 1.2 GB `arc.nsa`. A `--script-only`
   mode would still help when iterating on lowering in a tight loop.
4. **Tests that check the whole corpus** for decoding (every message parses,
   every jump target exists, every referenced asset is in the archive), so
   regressions show up in `cargo test` rather than in a run of the game.
5. **A route map**: a graph of scripts, links and conditional jumps from
   the trace. It shows the game's structure at a glance and lets us
   compare scene order with the real game.
6. **Asking for game knowledge up front:** route structure, the choice
   mechanic, known endings. The AI agents (§8) should get this as context
   before they start inferring opcodes.
7. **Single-scene builds** (`convert-script`) for visual work, so one
   scene is checked without the whole game.

From 装甲悪鬼村正:

8. **Dry runs for logic, real builds for looks.** `convert-nss --dry-run`
   compiles the whole game in about 16 s; a full build takes about 8
   minutes, nearly all of it extracting and packing 2.4 GB of assets.
   Make builds incremental: keep extracted assets between runs and
   repack only what changed, or stream archive entries straight into the
   .nsa without loose files. Extract in parallel threads.
9. **Start a build at a label** (`--start f_ma02_003`, or the jump patch
   used by hand today) so a scene deep in the game is checked without
   clicking through everything before it.
10. **A lint pass over the emitted `0.txt`**: missing labels, effect
    count, half-width characters in vertical text, files missing from the
    archives. Both whole-game crashes would have shown up here first.
    Done: `conscripter-convert lint <dir>` (`conv_cons::lint`), also run
    at the end of `convert-nss`. Its first run found a third crash case:
    a half-width `･` in vertical ruby.
11. **Frontier report in the CLI**: compare the files a run reaches with
    the route map's successors and print the condition that blocks each
    (done by a scratch Python script today). It points straight at the
    next thing to support.
12. **Faster headless runs**: an engine option to auto-advance (skip mode
    or `autoclick`) and to save a screenshot every N pages, instead of
    xdotool clicks and ImageMagick `import` (about 2.7 s a click).
