# 最果てのイマ COMPLETE (Xuse)

The first game converted, and the reference for the Xuse engine.
Finished: only bugs are worked on. `<install>` is the folder holding
`Farthest2015.exe`.

## Status

Plays from start to end in CONscripter ("Scope", Converted). All 82
opcodes the corpus uses convert and agree with the exe (84,710 of
84,713 instructions; 3 are the dropped debug menu).

Open bugs:
- The launch warning's visitor counter ("あなたは N 人目のお客様です")
  isn't drawn (the user: only worth a one-line fix; it isn't one). The
  exe (`0x44f150`, `0x44f400`, `ImageDigit` `0x488dd0`/`0x488f30`): N is
  the system data's `0x51eb50`, set at first run to the low dword of
  the local time's FILETIME (`0x45fe80`), plus `rand() % 10000` at every
  launch, saved; drawn in base 10 from `Entrance.004` #1 (13×16 cells,
  table `0x4fe320`, rect `0x4fe398`), right-aligned to x 492, top 173
  (+0xcc 1, +0xd0 0). Ours would need a global variable, a random add
  and a digit-sprite loop in `boot.rs` (about 40 lines).
- Value still not from the exe: the chapter history greys entries
  from a runtime index (`+0x19c`, `0x4682a0`; ours all white).
- System screens darken the blurred scene by 20 a channel too
  (Brightness `0x436510` -20 after Filter-Smooth, `0x46c030`); the save
  menu's `filter` takes one filter, so ours only blurs.
- Top bar values still from shots (`screens.rs` `top_bars`), read so
  far: rows, buttons and level box from `0x496610`, `0x497160` and the
  table `0x4fa720` (box (610, 30), prev 708, next 734, exit 764, row
  533, pitch 26: all as ours); depth digits right- and bottom-aligned
  (`0x496397`: +0x24, +0x28 = 1) ending at (669, 47) and (690, 47),
  digit widths `0x4fa780` (9 5 7 6 9 6 7 7 8 8), cell `0x4fa7f8` 9×13:
  ours draws full cells at (647, 34), (685, 34). Not read: scene title
  field (531, 6) (`0x499040` in `0x496fe0`) against ours (524, 6),
  chapter title (73, 6) (`0x49b1a0`) against ours (72, 7), the cursor
  (`0x140c`: box - 70, row + 5) against ours (542, row + 5).
- Unchecked: that `arc.nsa` (about 1.2 GB) loads in the web build
  (else split the voices into `arc1.nsa`), and that saves, thumbnails
  and `kidoku.dat` persist there.

Static checks (`docs/new-game.md` steps 6-7), gaps ranked:
- Normal blend rounds differently: the exe's MMX blend (`0x42c4e0`,
  C twin `0x42bbe0`, table `0x518228`) is k = a·opacity >> 8, d +=
  (s − d)·k >> 8, floored; ours (`blendWeight`, engine
  `AnimationInfo.cpp`) is exact. Over every (s, d, a): ours −1 to +2
  levels, +1 or more in 49.7 %, max +2 at (s 192, d 0, a 129); opaque
  pixels lose 1 level in the exe where s > d (255 over 0 gives 254).
- Fx animation module (`Aom_Animation_Parts` `0x417580`, via `0x8b`)
  writes the object's position (+0x18c/+0x190), scale (+0x1a8/+0x1ac),
  rotation (+0x1b0) and opacity (`0x40f160`) when its flags (+0x20 bits
  0x20-0x100) are set; the flags' writer wasn't traced. Ours plays the
  frames only (the `.ap` frame flags are all 0).
- No gap: object opacity, blend, anchor, scale and size (+0x1b2, +0x1b4,
  +0x180, +0x1a8/+0x1ac, +0x198/+0x19c) are written by their opcodes,
  `object_create`/`0x40f310` defaults, save restore `0x40f7d0`, env
  creators `0x5013c0` (blend 1 only for unused kind 1), bust-ups
  (`0x434130`: centre anchor, as ours) and system screens; the CG set
  `0x51ea18` also by `0x54`/`0x55` (`0x4222c0`, handled); surfaces take
  the desktop format (32-bit: identity), present `0x4ad120` is a 1:1
  Blt; no gamma ramp, premultiplied alpha or D3D drawing.
- No gap: links are dead when their variable is 0 or their target's
  link scene is open (`0x492bb0` case `0xb`, `0x464870`; ours
  `conv_link_open`); no page mixes link variables. PNG gAMA is ignored
  by the exe (`0x4b6930`) and by SDL_image, native and web. Variable
  tables: as ours, every operand ("Progression variables").

Next: take a bug when the user or a play-through reports one.

## Build

`conscripter-convert build <install> -o <output>` recognises the game
and runs `convert-game` with its title (README, "Converting a game").
Full build (2-3 min):

```sh
target/release/conscripter-convert convert-game <install> \
  -o out/saiima-cons --font <a Japanese TTF, e.g. ipag.ttf> --title "Saihate no Ima COMPLETE"
```

`--dry-run` writes `0.txt` only (about 10 s; the same `0.txt` as the
full build): use it for anything about logic. `--original-archives`
(`docs/original-archives.md`) reads pictures, voices and effects in
place from `HD/` and `CD/`.

The scene replay's lists, the CG gallery's slots and the two staff
rolls are read from `Farthest2015.exe` at build time (`xuse/exe.rs`;
addresses in `replay.rs`, `gallery.rs`, `staffroll.rs`): a missing or
different exe stops the build with an error.

## Testing

- **Scripted runs:** `python3 -I research/xuse/smoke.py <build>
  <workdir> "1,0,998,1,201,999"` plays a click queue on a patched copy
  (format in its docstring; 999 = screenshot). `--goto conv_link2`
  jumps to a label after FIRST START. `systemcall` menus stay
  unanswered.
- **Debug trace** (`docs/new-game-reference.md`, "Checking a build
  with the debug trace"): `--trace t.jsonl --trace-ignore
  22-48,lsp2:12-23 --auto-play 0`, then
  `engine/tools/cons_trace_check.py t.jsonl --script 0.txt --config
  research/xuse/cons_trace_check.json` (the config lists the findings
  that are meant). Cover the routes with `cons_explore.py`; one
  `--auto-choice first` run leaves most labels unreached. Save slots
  900+ hold the blog bar's levels, so point `--checkpoint-slot` elsewhere
  (e.g. 500) when using `--checkpoint-every`.
- **Control flow without the engine:** `python3 -I
  research/xuse/nstrace.py 0.txt 50000000 --pages N [--click K]`.
- **Sprite numbers:** 0-5 yes/no dialog, 6-33 menus (more at 980-998),
  34-48 bars, 49 name text, 50 plate, 51-58 link words (`humanz 58`),
  59 full-screen animation, 60 static, 61 overlay, 62+ characters;
  `lsp2` 0-11 save menu, 12-23 top bars, 24 flash, 25 a turning
  bust-up, 26 a fade to a colour.

### Running the original under wine

The recipe of CLAUDE.md and `docs/tools.md` (scratch prefix with real
user folders, own Xvfb, `ja_JP.UTF-8`, the null ALSA sink of
`docs/games/muramasa.md`). Game specifics:

- **Install:** `HKLM\Software\XUSE_CORP\Farthest2015` (32-bit view,
  `/reg:32`): `Install` = `C:\Xuse\ima\`, `HD`, `CD` its subfolders;
  make `C:\Xuse\ima` a farm of symlinks to the install. Saves go to
  `Documents\Xuse\最果てのイマ COMPLETE\`.
- **Input:** the game draws at screen offset (3, 29); clicks need
  `xdotool mousedown`, a short sleep, `mouseup`. To the story: ENTER
  (405, 463), click past the intro, wait ~14 s, FIRST START (675, 240).
  Ctrl skips (stops at links).
- **Patching a `.cd`:** overwrite an instruction in place and fix the
  MD5 footer (a label repointed in the file's own table has no effect).
  The opening's `s2_l17` `0x43` (#73) is a safe slot for experiments.
- **Saves:** write one with `research/tools/xuse_save.py` ("Save
  format") and load it from the title's load (670, 262): slot rows at
  y 90, 165, 255, then load (652, 544) and はい (370, 320). Extras need
  a cleared system save; title → option (660, 315).
- **Reference trace** (`docs/tools.md`, "Matching the original"):
  `GAME=<install>
  research/xuse/tracedll/xtrace.sh -o t.jsonl [--save slot.bin]
  [--choices 12,34|none] [--pages N] [--frames] [--no-cover]` (hooks and
  records: `xtrace.c`'s head comment). `SPEED=4` plays 1 s a page
  within 2 frames of a 1x run. Each run also marks branch coverage:
  `python3 -I research/xuse/cover.py mark t1.jsonl ... --write` fills
  the findings table's last column.
- **Breakpoint trace:** `winedbg --gdb --no-start` plus
  `research/tools/wine_bptrace.py research/xuse/names.tsv <names>`;
  break on handlers and their per-frame callbacks (not `frame_loop`). Detaching crashes the game; end it with `wineserver -k`.
- **Trace diff** (`docs/tools.md`, "Matching the original") against a dry run with
  the same choices:

  ```sh
  target/release/conscripter-convert convert-game <install> -o out/saiima-dry \
    --dry-run --source-map out/saiima.map --title x
  research/tools/scratch.sh out/saiima-cons <scratch> out/saiima-dry/0.txt
  cp -r out/saiima-dry/screens out/saiima-dry/ons_args <scratch>/
  <scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time \
    --trace-ignore 22-48,lsp2:12-23
  research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/saiima.map --pages 50
  ```

### Save format

Folder `Documents\Xuse\最果てのイマ COMPLETE\`: `Farthest2015_000.bin`
(system data), `_sys.bin` (settings), slots 1-50 in
`Farthest2015_NNN.bin`, and `Farthest2015.log` (the game's error log,
strings XORed with `0x53`: read it when a load fails). No compression
or encryption; integers little-endian; every chunk ends in a big-endian
CRC-16/XMODEM (`0x4a7be0`) and the loader refuses a wrong one.

`python3 -I research/tools/xuse_save.py dump <save>` prints a slot or
system save as JSON; `write <template> <json> -o <out> [--cd
<install>/HD/CDFiles]` applies an edited dump to a save the game wrote
and recomputes the CRCs. A slot's position is `"scene": {"label":
"s9_l3"}`, its variables `"vars"`; the script redraws the screen at a
scene's entry label. Chunks (slot: `LOCL`, `sRIO`, `SVAR`, `LSNL`/
`LSNP`, `HyLk`, `CPHS`; system: `GLBL`, CG, RMSN, SWTC, `SVAR`, `OTHR`
with the progress word `0x51eb3c`): `xuse_save.py`'s code.

## Scope

- **Release:** the retail Windows game, installed (`<install>`, with
  `HD/` and `CD/` in it); no manual.
- **Engine:** Xuse's in-house engine (`Farthest2015.exe`, 32-bit, VC9;
  debug strings mention `AdvObject`, `Gaf004Loader`,
  `KeyFrameSequencer`; bundles libpng, zlib, libogg/libvorbis). No
  public documentation of its formats: everything was reversed from the
  exe.
- **Reader:** `crates/conv-engines/src/xuse/`; research scripts in
  `research/xuse/`; the engine overview in `docs/engines/xuse.md`.
- **Converted:** everything the story uses (text, voices, sprites with
  filters, backgrounds, BGM with loop points, sound effects, movies,
  every transition kind, screen effects, text modes, ruby, links,
  chapter titles), the system screens (title with the exe's intro, save/load,
  settings, backlog with names and voice replay, button bars, yes/no,
  the blog bar) and the extras (CG gallery, scene replay, BGM room).
  Movies play in the web, Windows and Android builds.
- **Dropped by the user:** the system-voice room (WSS), right-click
  menus (and the autosave in them), native Linux/macOS movie playback,
  the unreachable debug menu (`0x30`, `0x62` and the message of
  `0001.cd` #1; `LEFT_OUT` in `xuse/build.rs`).

## Script language

### Script bytecode

The dispatch table is at `0x4fb270` (157 handlers, opcode 0-`0x9d`).
For most opcodes `arg` is the pool offset of the operand record. Each
opcode's handler address, record layout and conversion are in the
comments of `crates/conv-engines/src/xuse/cd.rs` (the records) and
`convert.rs` (the lowering; `0x76`'s `sub` → transition kind in
`transition_style`, drawn as the engine's `COMMANDS.md` says for the
`cons_*.dll` effects). What is easy to get wrong:

- **Labels:** `0x10`/`0x11` take a label number in the current script;
  a `0x10` that targets its own instruction is a label marker (jumps
  land after it). `0x15`, `0x16`, `0x22` and links take a label-table
  index, and the engine's table has an implicit entry 0: index `n` =
  stored entry `n − 1`.
- **Variables:** bit 15 set = kept in each slot save, clear = in the
  system data ("Progression variables"). `0x01`-`0x0e` are the
  arithmetic (`0x03` is `-= var`, an engine quirk); `0x16`-`0x1b` are
  `if v[a] <cmp> b goto`; `0x22`/`0x23` gosub/return (stack depth 20).
- **Messages** (`0x40`, `0x41` voiced): pre-wrapped display lines, no
  inline markup; `sub` bit 0 = the first line is the speaker's name,
  which the exe drops in text modes that can't show names. Voice clip =
  (id − 1) % 100000; `id / 100000` is the character.
- **Hyperlinks** (`0x46`): one record per linked word before its
  message (row, column, length, style, label-table index); targets are
  link scenes (`blog?<title>`), `0x47` returns from one and `0x48` to
  the root. Converted as buttons that `gosub` the scene inside
  `pushscreen`/`popscreen`, nested at most 7 deep (`MAX_LINK_DEPTH` in
  conv-cons, the user's choice; the game's own scenes nest five).
- **Chapters:** titles are `0.cd` label names `chapter<route>]?<title>`;
  `0x36` sets the title (`$150`), `0x37`/`0x38` close and clear it.
  `0x93 N` unlocks replay scene N (`%3000+N`), `0x99` sets progress bit
  `sub` (`%3600 + bit`).
- **Drawing order:** objects sort by a key: band from the layer byte,
  depth in the low 24 bits (`0x4126c0`); `op_76` maps the layer to a
  band through `0x503e54`, `0x7a`/`0x70`/`0x8c` through `0x503e68`.
- **Operand coverage:** every operand is read through `CdScript::sub`,
  `CdScript::arg` and the pool readers in `cd.rs`, which note what was
  read; the dry run reports each operand given and never read as
  "`<opcode>` operand `<which>` ignored" (a gap in `metrics.json`).
  The harmless ones, each with its values and why, are
  `UNNEEDED_OPERANDS` in `convert.rs` (with `POOL_RECORDS`, `NO_SUB`,
  `NO_ARG`).

## Progression variables

Two tables in the exe (`load_svar` `0x46de00` sizes and names them):
bit 15 set, the slot table `0x519554` (150 entries, kept in each slot
save, `LocalSave.cpp`); clear, the system table `0x519544` (20, in the
system data, `GlobalSave.cpp`). Every reader (`0x473cb0` for the
arithmetic and ifs, `0x46e360` for links) indexes the table by the low
15 bits, ignores bits 16 up, and logs an index past the table. The
conversion keeps the same border (`script_var` in `xuse/convert.rs`):
slot ones are locals (n 0-10 → `%10+n`, 11-20 → `%18+n`), system ones
globals (`%230+n`). `research/xuse/svar_census.py` lists every operand:
358 (354 with bit 15), 22 codes (slot 0, 3-20; system 1-3), all inside
the tables and 1:1 into ours.

Slot variables (names from a save's `SVAR`): %10 `rv`, %13 `debug`,
%14-%17 `us_enable`, `us_disable`, `us_hide`, `href_enable`, %18
`sentaku_now`, %19 `sentaku_all`, %20/%29/%30/%31 `sentaku_azusa`,
`_fueko`, `_yoko`, `_sayaka`, %32 `sentaku_zumi`, %33 `sentaku_bad`,
%34 `loopsu`, %35-%37 `loop1`-`loop3`, %38 `clear_count`. System: %231
`war_clear` (end of the epilogue), %232 set in あずさ's route
(`azusa_tsuikabun`, read in `0015` `s15_l1`), %233 compared with 4603
(`s6_l1`).

| Var | Meaning | Set | Reset | Read |
|---|---|---|---|---|
| %18 | heroine this loop: 1 あずさ, 2 ふえこ, 3 ようこ, 4 さやか | each link scene | each お茶会－０ (`s4_l2`), `s0_l4` | end of 最初の日 |
| %19 | all four heroines done | `s0_l6` | | `s0_l5`, `s0_l8` |
| %20, %29-%31 | あずさ, ふえこ, ようこ, さやか done this cycle | their link scenes | `s0_l4` | 最初の日 hides the heroine's link |
| %32 | a heroine link was clicked this loop | link scenes | each お茶会－０ | 最初の日 |
| %33 | loops with no link | end of 最初の日 (+1) | `s0_l2` | `<= 2` → リピート (0021), else 死 (0022) |
| %35 | 1 in cycle 1 | `s0_l2` | `s0_l5` | route text variants |
| %36 | 1 in cycle 2 | `s0_l5` | `s0_l8` | route text variants |
| %37 | 1 in the war arc | `s0_l8` | end of the epilogue | route ends → `s5_l1` |
| %38 | routes finished | `s0_l5`, `s0_l8` (+1) | | not read |

A route's end calls `s0_l5` (cycle 1) or `s0_l8` (cycle 2), then goes
back to お茶会－０ (`s4_l1`). FIRST START runs `s0_l2`, which zeroes all
of this. `research/xuse/routesim.py` simulates routes on the bytecode
and on `0.txt` (its `0.txt` side still reads cp932 scripts and no named
variables: update it before its next use).

## Story structure and walkthrough

### Walkthrough

Routes are chosen by clicking a heroine's underlined link in 「最初の日」
(お茶会－０ opens each loop); the game loops by itself on the same
system data (sources: public guides, BIGLOBE, ifs.nog.cc, ugukan.net).

| Loop | In 最初の日 | Chapter / route |
|---|---|---|
| 1 | no link | リピート |
| 2-5 | one heroine per loop | cycle 1: the four routes |
| 6 | no link | リピート again |
| 7 | no link | 死 |
| 8-11 | one heroine per loop | cycle 2 (truth phase, text changes) |
| 12 | none / automatic | 戦争編 → epilogue |

Heroine links: あずさ 「最初の頃」 (2nd link), さやか 「たまに早く出る」
(3rd), ふえこ 「違う制服」 (4th), ようこ 「静かな朝」 (5th). Every other
link is lore. The guides put 死 right after cycle 1; the bytecode gives
it on the third no-link loop (`%33 <= 2` → リピート), and the
conversion follows the bytecode.

Scripts: 0000 `!boot`, 0004 `00_プロローグ` (お茶会－０ and 最初の日),
0009 あずさ, 0010 さやか, 0011/0012 ようこ, 0013 ふえこ, 0021 リピート,
0022 死, 0019 `09_回想初期化` (scene replay setup); 60 `15_Blog…`
scripts (ids 96-155) are the link scenes.

## Executables

The build the decompile addresses in this document and the code belong
to.

| File | Version | sha256 |
|---|---|---|
| `Farthest2015.exe` | unknown | `efb7ec8e00338376d37822880e7fb73ae86eca03895bf55a6b6826b1dda6a37f` |

Decompile: `research/tools/ghidra/ghidra_export.sh
<install>/Farthest2015.exe research/xuse/local/ghidra --names
research/xuse/names.tsv` (pseudo-C per function, git-ignored).
`names.tsv` names the dispatch table, every handler (`op_XX`) and the
addresses the code cites; the exe's findings are in
`docs/games/saihate-no-ima.findings.tsv`.

Letter advance: 25 px at font 24 (`0x4a2001` `imul ebx,ebx,0x19`, the
letters after the name; TextField `0x48bd00` adds field+0xc4, set at
`0x486850`, to each glyph width; the main window's value not traced).
Ours: 24 + 1 = 25, unchanged.

## Files and formats

| Files | Format | Contents |
|---|---|---|
| `HD/*.arc` (env, fx, gd, mask, pal, wnd) | XARC (`xarc.rs`) | UI, effects, masks, palettes: 130 entries (64 GAF4, 32 PNG, 31 `.ap` + 2 `.def` text, 1 RIFF palette) |
| `HD/bg*.004`, `bu*.004`, `LensFlare.004` (+ 64 `.004` in XARC) | GAF4 (`gaf.rs`) | 2,971 PNGs: backgrounds 800×600, sprites RGBA in `b`/`m`/`s` sizes, UI parts |
| `CD/BV*.xsa` | XSA (`xsa.rs`) | voices: 15,507 Ogg Vorbis clips |
| `HD/bm0000.bin` | XSA | BGM: 23 Ogg tracks; slot param 0 = loop start in samples |
| `HD/k0000.bin`, `k0001.bin` | XSA | sound effects: 311 clips |
| `HD/cgv.bin`, `wss.bin`, `silent.bin` | XSA | 113 / 182 / 1 clips (`wss`: the dropped system-voice room) |
| `HD/CDFiles/*.cd` | CD (`cd.rs`) | the compiled scripts, one per `.rio` source: 172 files, 84,713 instructions |
| `HD/CDFiles/0.cd` | ciphered (`symbols.rs`, cipher `0x4bcb20`) | label symbol table: 1,820 names with code offsets |
| `HD/CDFiles/1.cd` | unknown cipher | probably the variable definitions (`sVAR`, loader `0x46de00`); not needed |
| `HD/mv000*.bin` | MPEG-1 program stream | movies; ffmpeg reads them |
| `HD/zpkc.bin`, `res/m*.bin` | PNG, BMP | images |

Tools: `conscripter-convert xarc|gaf|xsa|cd`, each with `--help`.

- **XARC:** offset table, then per entry `"DATA"`, 4 unknown bytes, a
  `SYSTEMTIME`, name length, data size, the name with nibbles swapped,
  raw data. Header, entry headers and data each end in a big-endian
  CRC-16/XMODEM.
- **XSA:** `u32` version 1, 256 slots of (size, offset, 0, 0), the Ogg
  clips back to back. A clip index is archive `index >> 8`, slot
  `index & 0xff` (`0x47f620`).
- **GAF4:** `"GAF4"`, u16 version 3, a 64-byte key name, u32 image
  count at `0x46`; everything from `0x4a` is XORed with a key built
  from the name (`0x40b2e0`). Inside, `DSET` groups each hold a `PICT`
  (4 zero bytes + a PNG) and an `FTAG` with the original file name.
  The ~600-byte block before the first `DSET` isn't decoded.
- **CD:** 32 unknown bytes; the 260-byte source name XORed with `0x16`;
  six u32s (table count + 1, code size, pool size, script id, 0, 0);
  the label table (script id, label number, code offset); 12-byte
  instructions (u16 opcode, u16 sub, i32 arg, u32 source position); the
  operand pool (strings: u32 length + Shift-JIS XORed with `0x53`); an
  MD5 footer of everything before it.
