# vn2ons — Plan

Converts Japanese visual novels (that you own) into games playable on the
open-source **ONScripter** engine (a current fork with native Japanese text
support), and helps reverse-engineer engines it doesn't support yet — optionally
with AI agents.

- **First target game:** 最果てのイマ (Saihate no Ima). Its engine is not yet
  identified; that's the first task (see §6).

- **Language:** Rust (cargo workspace). C++ only where we link existing C/C++ code
  (via the `cxx` crate) or build test harnesses around ONScripter itself.
- **Input:** an installed game directory.
- **Output:** `0.txt` / `nscript.dat` + `arc.nsa`/`.sar` archives + converted media,
  runnable by the target ONScripter fork, plus a conversion report.

## 1. Scope and expectations

- Games that are mostly text, sprites, backgrounds, music, voices and choices
  convert well.
- Custom systems (minigames, battle systems, heavily scripted menus written in
  the engine's own scripting language) do **not** convert automatically. They
  are reported clearly, never silently broken.
- The deterministic pipeline is the source of truth. AI agents only **propose**
  artifacts (profiles, opcode tables, mappings) that the pipeline then
  validates. Agents **only run when explicitly invoked** (`vn2ons ai …`);
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

Only the ONScripter output stage knows about NScripter, so another output
engine could be added later.

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

### Phase 0: pick the ONScripter fork and check what it supports

Requirement: Japanese text must work natively (Shift-JIS scripts, Japanese
fonts, correct kinsoku line breaking). Candidates:

- **ogapee's ONScripter** (the original, actively maintained, Japanese-first).
  Default choice.
- **ONScripter-Jh** (adds UTF-8 scripts and more modern features) as an
  alternative if ogapee's lacks something we need.

onscripter-insani is no longer the target. Build the chosen fork on Arch and
test these before relying on them:

- **Resolution:** which `;mode` / `;$S` headers it accepts (800×600?
  widescreen?). If 1280×720 games can't run natively, the tool has to scale
  them down.
- **Images:** whether PNG alpha works, or whether `:a` images with the mask
  stored beside the picture are required.
- **Audio:** OGG, MP3, MIDI support.
- **Video:** what it can play (probably MPEG-1, maybe nothing). Fallback: skip
  videos or replace them with a still image.
- **Text:** Shift-JIS handling, line breaking, English mode.

The results go into `target_caps.toml`, which the output stage reads. If you
switch to ONScripter-EN or ogapee's current ONScripter later, only this file
changes.

## 5. Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, …) into the `image` crate's types and save as PNG. Then sprite
  composition, resize to the target resolution, and alpha conversion according
  to `target_caps`. Run per-image work in parallel with `rayon`.
- **Audio:** keep OGG where possible; transcode the rest with an `ffmpeg`
  subprocess. Loop points (KiriKiri `.sli`, RealLive loop metadata) can't be
  carried over directly: either note them in the report or bake intro + loop
  into the output.
- **Video:** `ffmpeg` to whatever the target can play.
- **Packing:** write `arc.nsa` / `.sar` natively (simple formats) and
  optionally encode `nscript.dat`.

## 6. Engine support order

| Order | Engine | Why |
|---|---|---|
| 1 | **NScripter** | Round-trip test (NScripter → IR → NScripter) of the IR and output stage, no reverse engineering involved |
| 2 | **Saihate no Ima's engine** (to be identified in M0) | First real target. If it turns out to be a known engine (e.g. KiriKiri), use the existing documentation; if not, this is the first job for the format sleuth and opcode inferrer |
| 3+ | KiriKiri2/KAG, RealLive/Siglus, CatSystem2, BGI/Ethornell, Yu-ris, Artemis, Majiro, Softpal… | Chosen by which games you want to convert next |

### Target 1: 最果てのイマ COMPLETE (Xuse, 2015)

Installed at `/home/saya/Z:/Media/InstalledVNs/最果てのイマ COMPLETE/`.
First look (2026-10-12, read-only):

- **Engine:** Xuse's in-house engine (`Farthest2015.exe`, 32-bit, VC9). The
  exe's debug strings mention `AdvObject`, `Gaf004Loader`, `KeyFrameSequencer`.
  It bundles libpng, zlib and libogg/libvorbis, so images are probably PNG/JPEG
  inside containers and audio is probably Ogg Vorbis. I found no public tool
  that documents these formats, so expect to reverse-engineer them.

| Files | Count | Magic / observation | Probably |
|---|---|---|---|
| `HD/*.arc` (env, fx, gd, mask, pal, wnd) | 6 | **Solved** — see below | Archives of UI, effects, masks, palettes: 130 entries total (64 `GAF4`, 32 plain PNG, 31 `.ap` + 2 `.def` text files, 1 RIFF palette) |
| `HD/bg*.004`, `HD/bu*.004`, `HD/LensFlare.004` (+ 64 `.004` inside XARC) | 86 | **Solved** — `GAF4`, see below | 2,971 PNGs: backgrounds 800×600 RGB, sprites RGBA, UI parts. Tool: `vn2ons gaf list\|extract` |
| `CD/BV*.xsa` | 61 | **Solved** — see below | Voice archives: 15,507 clips, plain Ogg Vorbis (44.1 kHz mono) |
| `HD/CDFiles/*.cd` | 172 | **Container solved** — see below | **Compiled scenario scripts**, one per `.rio` source file: 84,713 instructions, 82 distinct opcodes |
| `HD/CDFiles/0.cd` | 1 | **Solved**: cipher from the exe (`0x4bcb20`) | **Label symbol table** for all 172 scripts: 1,820 label names with code offsets, all CRCs verified. Tool: `vn2ons cd symbols`. |
| `HD/CDFiles/1.cd` | 1 | Not the 0.cd cipher (no variant of its two constants works either) | Probably the variable definitions. The loader at `0x46de00` reads an `sVAR` file of 0x44-byte entries (64-byte names) into two tables of 20 and 150 variables; how `1.cd` gets decrypted for it is still unknown. |
| `HD/mv000*.bin` | 3 | MPEG program stream (`00 00 01 ba`) | Movies; ffmpeg reads them directly |
| `HD/bm0000.bin` | 1 | `.xsa` format, **solved** | BGM: 23 Ogg tracks; param 0 = loop start in samples (20 tracks loop) |
| `HD/k0000.bin`, `k0001.bin` | 2 | `.xsa` format, **solved** | Probably sound effects: 311 Ogg clips |
| `HD/cgv.bin`, `wss.bin`, `silent.bin` | 3 | `.xsa` format (182 / 500 slots), **solved** | 113 / 182 / 1 Ogg clips (role unknown; `wss` param 1 = length in samples) |
| `HD/zpkc.bin` | 1 | Plain PNG, 800×600 | Image |
| `res/m*.bin` | 5 | Plain `BM` | BMP images (installer/menu) |

The exe also references `.arc` files that aren't installed (`bu.arc`, `gb.arc`,
`lq.arc`), plus `%s-%s.wav` and `mask%04lu.png` name patterns that hint at
names inside the archives.

**XARC format (solved 2026-10-12):** documented in
`crates/vn2ons-engines/src/xuse/xarc.rs`. Offset table, then per entry
`"DATA"`, 4 unknown bytes (`20 00 00 00` or `20 20 00 00`), a `SYSTEMTIME`,
name length, data size, the name with nibbles swapped, and raw (unencrypted)
data. The archive header, each entry header and each entry's data are followed
by a big-endian CRC-16/XMODEM; all CRCs in all six archives verify. Tool:
`vn2ons xarc list|verify|extract`.

**XSA format (solved 2026-10-12):** documented in
`crates/vn2ons-engines/src/xuse/xsa.rs`. `u32` version (1), then 256 slots of
(size, offset, 0, 0), then the clips back to back. No names and no encryption;
unused slots are zero and only occur at the end of the last archive. The
global voice number is presumably `archive * 256 + slot` (to confirm against
the scripts). Tool: `vn2ons xsa list|extract`. Since ONScripter plays Ogg
directly, the clips can be copied over as-is.

**CD script container (solved 2026-10-12):** documented in
`crates/vn2ons-engines/src/xuse/cd.rs`.
- **Head:** 32 unknown bytes (unique per file).
- **Name:** the 260-byte source file name (`*.rio`), XORed with `0x16`.
- **Header:** six u32s (table count + 1, code size, pool size, script id, 0, 0).
- **Label table:** entries of (script id, label number, code offset), including
  cross-script targets.
- **Code:** 12-byte instructions (u16 opcode, u16 sub, i32 arg, u32 source
  position).
- **Operand pool:** strings are a u32 length plus Shift-JIS XORed with `0x53`.
- **Footer:** the MD5 of everything before it. All 172 files verify.

Tool: `vn2ons cd info|dump|messages|strings|opstats`.

**Opcode dispatch table:** `Farthest2015.exe` at `0x4fb270`, indexed by
the u16 opcode (0–`0x9d`, 157 handlers). Some handlers report their C++
source file in error messages: `ScriptStd` (`0x23`, `0x30`–`0x39`),
`TextRuby` (`0x44`, ruby/furigana), `ScriptText` (`0x46`–`0x49`), `TextIndent`
(`0x4e`), `ScriptSound` (`0x5a`, `0x5e`, `0x5f`), `ScenarioVariable` (`0x64`,
`0x65`, `0x9b`), `Story` (`0x93`), `AdvObjectScript_Filter` (`0x88`). `0x40`
and `0x41` share one handler. Message records have a second block after the
text (`u32 4, u32 0` in every message). Bit 0 of a message's `sub` means
"has voice" (`0x13` vs `0x12`). The display code (`0x475f40`) checks the
voice through `0x4679d0` and drops the flag if the check fails.

**Opcodes so far:** for most opcodes the instruction's `arg` is the pool
offset of its operand record.
- `0x40`: message.
- `0x41`: voiced message.
- **Record layout** (both): `id, size, line number, segment count`, followed
  by that many XOR-`0x53` strings. All 46,578 messages (15,620 voiced) decode.
- **Segments:** they look like pre-wrapped display lines, often with the
  speaker name first. There is no inline markup.
- **Voice ids:** they have the form `group * 10000 + n` (14 groups).
  - **Layout:** each group occupies one contiguous run of clips, roughly in
    ascending id order. The groups are ordered 30, 40, 50, 60, 90, 100, 101,
    121, 131, 71, 81, 111, 11, 1, which suggests the clips are sorted by a
    file name (the exe has `%s%s%s-%s.wav` and `%sBV%04d.xsa`).
  - **Alignment:** a statistical alignment (`research/xuse/voice_align.py`)
    matches 13,512 of 15,478 ids. Over those matches, text length and clip
    length correlate at r = 0.95.
  - **Gap:** that alignment is not exact. About 1,970 ids are unmatched,
    spread over every group, so wrong voices are possible.
  - **Solved:** clip = (id − 1) % 100000 (see saiima.md); `id / 100000`
    is the character. The "groups" above were characters × 10.
  - **Exact rule (history):** it had to come from the exe. The loader
    (`0x47f620`) takes a **clip index**, not an id: archive `index >> 8`,
    slot `index & 0xff`. So the id → index conversion happens before that
    call, and there's no divide-by-10000 anywhere, which suggests a lookup
    table. `0.cd` doesn't contain voice ids.

**Other opcodes (working hypotheses, from operand statistics):**

| Opcode | Uses | Evidence | Probably |
|---|---|---|---|
| `0x10` | 895 | `arg` is always a valid label-table index; 121 point into blog scripts | jump / call to a label |
| `0x4d`, `0x58`, `0x43` | 276 / 579 / 578 | `arg` is always a valid label-table index | call / conditional-jump variants |
| `0x8e` | 10,271 | **solved**: `sub` × 44-byte entries of `mode, character, sprite number, fade, x, y` (rest mostly uninitialised memory). The sprite number indexes `bu*.004` (128 per file, 0-based), and the file names' character codes match the `character` field | show `sub` character sprites |
| `0x54` | 2,195 | **solved**: `arg` = background number, 1-based (0 = none, skipped by the handler); `bg*.004` holds 64 per file (`n >> 6` in the exe's loader at `0x40bff0`). The most common is 1 = `Black` | set background (sub 0/1: transition variant?) |
| `0x46` | 214 | **solved** (from the handler at `0x477460` plus a user-supplied example) | **hyperlink** on the next message: row, column, length, style, target (see 3c) |
| `0x49` | 565 | flags, two colours, a voice-range id, then text + a short name | special voiced text display (chat/popup?); **not** hyperlinks |
| `0x57` | 542 | **solved** (handler `0x473530` → sound request `0x47aab0`, channel 0) | play BGM: `arg` = track (1-based, `bm0000.bin` slot `arg - 1`), `sub` = fade-in ms (capped at 30 s) |
| `0x58` | 579 | **solved** (channel 0, stop flag) | stop BGM: `arg` 0 = all, `sub` = fade-out ms |
| `0x5a` | 2,633 | **solved** (channel 1) | play SE: packed 9-byte record `u32 ?, u8 loop, u32 sound` (1-based across `k0000.bin` + `k0001.bin`) |
| `0x5b` | 890 | **solved** (channel 1, stop flag) | stop SE: `arg` = sound or 0 for all, `sub` = fade ms |
| `0x15`, `0x22` | 442 / 3,094 | handlers look up script variable number `sub` (`0x470830`) | variable operations (core VM) |
| `0x36`–`0x38` | 251 each | handlers resolve labels (`0x4706a0`) | probably choice menus |
| `0x76` | 3,670 | sub 9 in 92% of uses; record holds durations and constants, no sound number | screen effect/transition (not decoded) |

**Blog scripts:** 60 scripts are named `15_Blog…` (ids 96–155) and 44 are
named `11_…`. 33 story scripts have 153 label-table entries pointing into the
blog scripts. Together with the UI files (`BlogStackBar`, `ChapterHistory`),
this suggests the clickable words open in-game blog articles, which act as
the encyclopedia.

Still unknown: most opcode semantics, the message `sub` values, the voice
mapping, the 32-byte head, and `1.cd`.

**Plan for this game:**
1. ~~`XARC` reader~~ (done).
2. ~~`.xsa` reader~~ (done).
3. `.cd` scripts: ~~container and string encoding~~, ~~instruction → pool
   link and messages~~ (done). Next: the remaining opcode semantics.
3b. Exact voice id → clip rule: find where the voice-play opcode turns an
   id into a clip index (follow the `0x41` handler in the exe). Also find
   `1.cd`'s cipher. (`0.cd`: done.)
3c. **Encyclopedia hyperlinks: found.** Opcode `0x46` sits right before
   the message it applies to, one per linked word (214 in the game). Its
   18-byte record is:
   - **position:** `row, column, length` in characters (1-based, relative to
     where the next message starts; row 1 continues the cursor's current
     line);
   - **style:** `0x8000 | 7` (205 links) or `| 18` (9 links);
   - **target:** for 175 links it is an index into the script's label table.
     Your example resolves to a label in the same script, so at least some
     explanations are local subroutines.
   - **Unresolved targets:** the other 39 all equal the script's label-table
     size, i.e. one past the last entry stored in the file. The engine checks
     link targets against a count in the in-memory script object (`+0x30`),
     so it must add one entry at load time. Which entry that is hasn't been
     found yet (the static trace through `0x4ba6b0`/`0x4ba9b0` was
     inconclusive). Next step: take one of these links in the game, see
     where it goes, and match that back to the data.
   - **Entries can be full scenes:** per the user, some encyclopedia entries
     play flashbacks with sprites and other normal VN features, so a link
     target is a full script subroutine, not just text.

   This matches the observation that the same word is linked in some places
   and not in others: each link is placed by hand. Tool: `vn2ons cd messages`
   prints links with their resolved destination.

   **Conversion:** ONScripter can't make words inside text clickable.
   Options:
   - after the message, show the linked word(s) as `btn`/`csel` choices that
     `gosub` the target label and then return;
   - or collect the targets into an encyclopedia menu.

   Either way the target runs as a `gosub` with normal scene commands
   (sprites, backgrounds, sound) and must restore the story's screen state
   when it returns. So the converter has to save/restore the visible
   sprites/background around the call, or the target scenes must set and
   clear their own state.

   Rows and columns can be dropped, since ONScripter re-wraps the text
   anyway.

4. ~~`GAF4` images~~ (done). Format:
   - **Header:** `"GAF4"`, u16 version 3, a 64-byte key name (`"Farthest"` or
     `"generic"`), and a u32 image count at `0x46`.
   - **Encryption:** everything from `0x4a` on is XORed with a key built
     from the name (`0x40b2e0`), indexed by file offset modulo
     `keylen - 1`.
   - **Contents:** inside is a tagged structure (`DSET`, `PICT`, …) holding
     one complete PNG per image. All 86 files yield exactly their header
     count (2,971 PNGs).
   - **Chunks:** each image is a `DSET` group holding a `PICT` (4 zero
     bytes + PNG) and an `FTAG` with the original file name
     (`bg20d1.png`, `04.バストアップ\\bu021203s.png`, …). Sprites come in
     `b`/`m`/`s` variants of each pose (probably sizes). Tool:
     `vn2ons gaf names`.
   - **Still to decode:** the ~600-byte block before the first `DSET`. It
     may hold sprite placement data. Sprites appear to be whole images
     (one PNG per pose and size), so no composition is needed. Some sprite
     archives also contain 1×1 placeholder images.
5. Movies and the remaining `.bin` files last.
6. **Target: a WebAssembly build of ONScripter** (per the user). What that
   means for the conversion:
   - **Menus:** every system screen must be a real graphical menu built in
     the script (game graphics + `spbtn`/`btnwait`/`getmousepos`), since
     there are no native dialogs and the built-in text menus look out of
     place.
   - **Right-click:** ogapee's ONScripter has no `rgosub` (send right-click
     to a script label). Either patch the engine (we build it ourselves; a
     small patch set kept in this repo would apply to the WASM build too)
     or put all access behind the on-screen button bar.
   - **Packaging:** pack assets into `.nsa` archives rather than thousands
     of loose files (fewer browser fetches), bundle the font, and keep
     memory use in mind (a full background archive decoded is tens of MB).
7. **System UI** (from user screenshots of the original, in
   `out/my-screenshots/`, not committed). Each item, with the ONScripter
   tools for it:
   - **Text box:** ADV box (done) at the measured positions.
   - **Backlog:** a full-screen dark window (like `TextWnd5`, 714×472) with
     the history as continuous text. ONScripter's history (`lookback*`)
     draws in the current text window, so a full-screen look needs a
     workaround (switching the window around the history call, to
     investigate).
   - **Right-click menu:** save/load/auto-save, BGM/SE/voice/system-sound
     toggles, settings, title, hide window, history, chapter history,
     quit. Use `rmenu`; there's no equivalent for the toggles or for
     chapter history.
   - **Title screen:** FIRST START / load / option / EXIT over the title art
     (probably `gd.arc`: `Entrance.004`). Build with `spbtn`/`btnwait`.
   - **Save/load screen:** 5 slots per page with thumbnails, 10 pages,
     delete/save/close buttons, and a slot label of date plus chapter name.
     Build with `savegame`/`loadgame`/`getsavestr`/`savetime`; ONScripter's
     built-in text menu is the fallback.
   - **Settings:** a native Windows dialog (volumes, text, mouse,
     confirmations). Only partly possible (volume defaults, `menu_*`
     options).
   - **In-game bars:** the bottom button bar (auto/skip/log/save/load/
     setting) and the top bars (chapter name, blog/scene, level). These
     are HUD sprites and buttons from `wnd.arc`/`gd.arc`.

Any decryption keys found go in the game's profile (§11), not in the tool's
code.

**Identifying the engine (M0):** run `vn2ons inspect` (or by hand at first:
file extensions, archive magic bytes, strings in the `.exe`, any bundled
DLLs) on the installed game, then compare against GARbro's format list.

**Existing work to learn from:** GARbro (archive and image formats),
arc_unpacker, VNTextPatch, rlvm/RLdev, KrkrExtract. Use them as references,
port the logic into Rust, or (for C++ code) link through `cxx`. Check each
license before reusing code.

**Per-game profiles** (`profiles/<game>.toml`) can:
- map custom macros to IR,
- define sprite layer rules,
- set per-game decryption parameters (supplied by the user),
- override cases where automatic conversion gets it wrong.

Profiles are also the main thing the AI agents write (see §8).

## 7. Project layout

Cargo workspace:

```
crates/
  vn2ons-core/       ir.rs, vfs.rs, profile.rs, report.rs, source_loc.rs
  vn2ons-engines/    nscripter/, kirikiri/ (xp3, kag, tlg), reallive/, ...
  vn2ons-assets/     image codecs, compositing, ffmpeg wrapper
  vn2ons-ons/        emit.rs, varalloc.rs, effects.rs, nsa.rs, caps.rs, lint.rs
  vn2ons-walker/     IR interpreter: branch exploration, reachability, missing files
  vn2ons-ai/         Claude API client, agent loops, tools, response cache
  vn2ons-mcp/        MCP server exposing the tools to Claude Code (§8.5)
  vn2ons-cli/        `vn2ons` binary
cpp/                 optional: C++ code linked through cxx, ONScripter test harness
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
vn2ons inspect <gamedir>                 # engine, archives, script count, unsupported features
vn2ons extract <gamedir> -o out/         # unpack and decode only
vn2ons convert <gamedir> -o ons/ [--profile p.toml] [--target ons]
vn2ons lint ons/                         # check the generated script
vn2ons walk ons/|<gamedir>               # IR walker report
vn2ons run ons/                          # launch the target ONScripter on the result
vn2ons ai <task> <gamedir> [...]         # run an agent (see §8)
vn2ons mcp                               # serve tools over MCP (stdio)
```

## 8. AI agent layer

### 8.1 Principles

1. **Proposals, not edits.** Agents write candidate artifacts (profile
   fragments, opcode tables, mapping files) to `.vn2ons/proposals/`. Nothing
   reaches the output until it passes validation (round-trip, lint, IR
   walker, smoke run) and you approve it with `vn2ons ai accept <id>`.
   Agents never start on their own: `convert` failing only *suggests* the
   matching `vn2ons ai …` command in its report.
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
  SDK, so `vn2ons-ai` is a thin typed client (`reqwest` + `serde`) and a
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

`vn2ons mcp` exposes the same tools (VFS browsing, hexdump, disasm, IR
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
6. **Smoke test:** the target ONScripter headless (Xvfb or the SDL dummy driver)
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

- **M0:** identify Saihate no Ima's engine and archive formats (from the
  installed files); build ogapee's ONScripter, write `target_caps.toml`; set
  up the cargo workspace and IR types.
- **M1:** NScripter output stage, NSA packer, NScripter input engine,
  round-trip tests, linter.
- **M2:** Saihate no Ima's engine: archive reader, image/audio decoders,
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
- Target: a current ONScripter fork with native Japanese text (ogapee's by
  default); onscripter-insani is dropped.
- AI agents run only when explicitly invoked with `vn2ons ai …`.

## 13. Lessons learned and ways to speed up conversion

Collected while converting 最果てのイマ (Xuse). Each lesson is general and
should carry over to the next engine.

### 13.1 Lessons learned

- **Learn the game's mechanics before reading its bytecode.** In 最果てのイマ
  the in-text hyperlinks are the route choices. A link pushes a scene,
  `0x47` returns from it, and route scenes set the flags that later steer
  the story. A playthrough with no clicks hits a dead end that restarts
  the chapter. The first full trace showed only that loop, and it looked
  like a control-flow bug until the link targets were followed. Knowing
  the hypertext mechanic up front (game wiki, playthrough video) would
  have pointed straight at it.
- **Error strings in the exe name opcodes.** `0x47`'s handler errors with
  「ルートシーンにいるので、これ以上戻ることはできません」("in the root
  scene, can't go back any further"), which settled its meaning in one
  look. Dump the strings each handler references before tracing its logic.
- **Find out how many index spaces a jump uses.** Xuse has two: per-script
  label numbers (`0x10`/`0x11`) and global label-table indices (`0x15`,
  `0x16`, `0x22`, links), and the in-memory table has an implicit entry 0
  (off by one from the stored table). Labels sit *on* marker
  instructions, and jumps land just after them. Getting any of this wrong
  still produces a script that runs, so check it with a trace (13.2), not
  by eye.
- **A loop is not necessarily a bug.** Before "fixing" one, check whether
  the original game loops there too (dead ends, retries, "go back to the
  title").
- **Count pages as they are displayed, not in file order.** Setup scripts
  and debug menus come first in the file but are never shown, so a
  harness that numbers pages in file order screenshots nothing.
- **ONScripter ignores SIGTERM while it is stuck in a script loop.**
  Always run it under `timeout -k 5 …`.
- **Never delete an output folder without listing it first.** Users put
  their own files there (an `onscripter` binary was lost once).
- **Keep Shift-JIS out of the window caption.** It shows as mojibake on
  Linux, so the caption is ASCII only.
- **Rank opcodes by use count** (`vn2ons cd opstats`) and convert from the
  top. Most of the visible game comes from a handful of opcodes.
- **Source-file names and handler tables in the exe identify opcodes
  quickly** (e.g. a handler in `Story.cpp` is story bookkeeping, not
  display).
- **Check decoded data against the whole corpus, not one sample:** all
  46,578 messages parse, all 810 jump targets exist, and script MD5s
  verify. Corpus-wide checks catch format edge cases that one scene hides.
- **A prefix check can't catch off-by-one inside a block.** Sprite numbers
  were read 0-based; the character-prefix check still passed 97%, because
  the next image is usually the same character. What exposed it: 1×1
  placeholder images being referenced 1,639 times (0 after the fix), and
  the handler's id constant `19,999,999 + n`. Also count references to
  degenerate assets (empty, 1 px).
- **Cross-check decoded references against asset names.** Checking every
  sprite entry's character against its image's `bu<character>` prefix found
  623 mismatches in one pass. They pointed to the "remove character"
  entries, which were being drawn as character 01, and to the
  per-character semantics of `0x8e`. Name prefixes in the asset archives
  are a free oracle; use them for every asset reference.
- **Look for divide-by-constant magic numbers.** The voice rule hid behind
  `imul 0x14f8b589; sar 13` (÷ 100000). An earlier search for a literal
  divide by 10000 found nothing, which wrongly suggested a lookup table.
- **Use reference screenshots of the original game** for layout. Measuring
  pixels from them settled the text box and name plate in one pass.
- **Keep a handoff file** (`saiima.md`) with a "Resume here" section.
  Context resets happen mid-task, and re-deriving state is expensive.

### 13.2 Ways to speed up conversion

Tooling worth building, in rough order of payoff:

1. **A static tracer in the converter** (`vn2ons trace`, a Rust port of
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
   takes 23 s, mostly writing the 450 MB `arc.nsa`. A `--script-only`
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
