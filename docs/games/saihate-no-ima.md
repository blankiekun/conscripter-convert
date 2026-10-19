# 最果てのイマ COMPLETE (Xuse)

The first game converted, and the reference for the Xuse engine. This
is the game's committed record: file formats, opcodes, how its features
map to CONscripter, the progression variables, and what is left. The
general process is in `docs/new-game.md`, general lessons in `docs/lessons.md`. Paths below are relative to the repository unless they say
otherwise; `<install>` is the folder holding `Farthest2015.exe`.

- **Status:** finished 2026-10-14 (see "Status" below). Branch `xuse`.
- **Engine:** Xuse's in-house engine (`Farthest2015.exe`, 32-bit, VC9).
  The exe's debug strings mention `AdvObject`, `Gaf004Loader`,
  `KeyFrameSequencer`. It bundles libpng, zlib and libogg/libvorbis. No
  public documentation of its formats was found: everything here was
  reversed from the exe.
- **Install:** the retail game, installed (`<install>`, with `HD/` and
  `CD/` in it).
- **Reader:** `crates/conv-engines/src/xuse/`; research scripts in
  `research/xuse/`.

## Build

`conscripter-convert build <install> -o <output>` recognises the game and
runs `convert-game` with `--title "Saihate no Ima COMPLETE"` (README, "Converting a game"); the
convert options (`--dry-run`, `--font`, …) go after `-o`.

Full build (2-3 min):

```sh
target/release/conscripter-convert convert-game <install> \
  -o out/saiima-cons --font <a Japanese TTF, e.g. ipag.ttf> --title "Saihate no Ima COMPLETE"
```

`--dry-run` writes `0.txt` only, no assets (about 10 s; the same `0.txt`
as the full build): use it for anything about logic.

`out/saiima-flow` holds the user's saves: never build into it or touch
it. `research/xuse/smoke.py` drives a build headless with scripted
clicks; `research/xuse/routesim.py` simulates routes on both sides
(`docs/new-game.md` step 11.4).

## What converts

最果てのイマ COMPLETE converts completely: every instruction the story
uses is converted (only the unreachable debug menu is left out).

| Area | Status |
|---|---|
| Archives, images, BGM (with loop points), sound effects, voices, movies | ✅ Decoded |
| Script text, speaker names, ruby, text sizes and colours, hyperlinks (the route choices) | ✅ Converted |
| Backgrounds, sprites, colour filters, music, sound effects, voices in scripts | ✅ Converted |
| Control flow (variables, conditions, goto, gosub/return, link scenes) | ✅ Converted |
| Transitions | ✅ Every kind drawn as in the original (some via CONscripter effect plugins) |
| Screen filters, shakes, ripples, pulses, quakes, flashes, full-screen animations | ✅ Converted |
| ADV text box, name plate and full-screen text modes | ✅ Converted |
| System screens (title, save/load, settings, backlog with names and voice replay, button bars, confirmations) | ✅ Built as in-script menus |
| Extras (CG gallery, scene replay, BGM room) | ✅ Built as in-script menus |
| Movies | ✅ Web, Windows and Android; skipped by the Linux and macOS builds |
| Not converted | The system-voice room (WSS), autosave (in the dropped right-click menu), some settings of the original's settings dialog |

Other Xuse games may convert but haven't been tried.

## Files and formats

| Files | Count | Format | Contents |
|---|---|---|---|
| `HD/*.arc` (env, fx, gd, mask, pal, wnd) | 6 | **Solved** — see below | Archives of UI, effects, masks, palettes: 130 entries total (64 `GAF4`, 32 plain PNG, 31 `.ap` + 2 `.def` text files, 1 RIFF palette) |
| `HD/bg*.004`, `HD/bu*.004`, `HD/LensFlare.004` (+ 64 `.004` inside XARC) | 86 | **Solved** — `GAF4`, see below | 2,971 PNGs: backgrounds 800×600 RGB, sprites RGBA, UI parts. Tool: `conscripter-convert gaf list\|extract` |
| `CD/BV*.xsa` | 61 | **Solved** — see below | Voice archives: 15,507 clips, plain Ogg Vorbis (44.1 kHz mono) |
| `HD/CDFiles/*.cd` | 172 | **Container solved** — see below | **Compiled scenario scripts**, one per `.rio` source file: 84,713 instructions, 82 distinct opcodes |
| `HD/CDFiles/0.cd` | 1 | **Solved**: cipher from the exe (`0x4bcb20`) | **Label symbol table** for all 172 scripts: 1,820 label names with code offsets, all CRCs verified. Tool: `conscripter-convert cd symbols`. |
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

Tools: `conscripter-convert xarc|gaf|xsa|cd`, each with `--help`.

**XARC archives** (`crates/conv-engines/src/xuse/xarc.rs`): offset
table, then per entry `"DATA"`, 4 unknown bytes (`20 00 00 00` or
`20 20 00 00`), a `SYSTEMTIME`, name length, data size, the name with
nibbles swapped, and raw (unencrypted) data. The archive header, each
entry header and each entry's data are followed by a big-endian
CRC-16/XMODEM; all CRCs in all six archives verify.

**XSA sound banks** (`xsa.rs`): `u32` version (1), then 256 slots of
(size, offset, 0, 0), then the clips back to back. No names and no
encryption; unused slots are zero and only occur at the end of the last
archive. The clips are Ogg Vorbis and are copied over as-is. The loader
(`0x47f620`) takes a clip index: archive `index >> 8`, slot
`index & 0xff`.

**GAF4 images** (`gaf.rs`):
- **Header:** `"GAF4"`, u16 version 3, a 64-byte key name (`"Farthest"`
  or `"generic"`), and a u32 image count at `0x46`.
- **Encryption:** everything from `0x4a` on is XORed with a key built
  from the name (`0x40b2e0`), indexed by file offset modulo
  `keylen - 1`.
- **Contents:** a tagged structure (`DSET`, `PICT`, …) holding one
  complete PNG per image. Each image is a `DSET` group holding a `PICT`
  (4 zero bytes + PNG) and an `FTAG` with the original file name
  (`bg20d1.png`, `04.バストアップ\\bu021203s.png`, …). All 86 files yield
  exactly their header count (2,971 PNGs). Sprites come in `b`/`m`/`s`
  sizes of each pose, as whole images (no composition). Some sprite
  archives contain 1×1 placeholder images.
- **Not decoded:** the ~600-byte block before the first `DSET`.

**CD scripts** (`cd.rs`):
- **Head:** 32 unknown bytes (unique per file).
- **Name:** the 260-byte source file name (`*.rio`), XORed with `0x16`.
- **Header:** six u32s (table count + 1, code size, pool size, script
  id, 0, 0).
- **Label table:** entries of (script id, label number, code offset),
  including cross-script targets.
- **Code:** 12-byte instructions (u16 opcode, u16 sub, i32 arg, u32
  source position).
- **Operand pool:** strings are a u32 length plus Shift-JIS XORed with
  `0x53`.
- **Footer:** the MD5 of everything before it. All 172 files verify.

Still unknown: the 32-byte head, and `1.cd`'s cipher.

## Executables

The builds the decompile addresses in this document belong to.

| File | Version | sha256 |
|---|---|---|
| `Farthest2015.exe` | unknown | `efb7ec8e00338376d37822880e7fb73ae86eca03895bf55a6b6826b1dda6a37f` |

## Script bytecode

**Dispatch table:** `Farthest2015.exe` at `0x4fb270`, indexed by the u16
opcode (0–`0x9d`, 157 handlers). Some handlers report their C++ source
file in error messages: `ScriptStd` (`0x23`, `0x30`–`0x39`), `TextRuby`
(`0x44`), `ScriptText` (`0x46`–`0x49`), `TextIndent` (`0x4e`),
`ScriptSound` (`0x5a`, `0x5e`, `0x5f`), `ScenarioVariable` (`0x64`,
`0x65`, `0x9b`), `Story` (`0x93`), `AdvObjectScript_Filter` (`0x88`).
For most opcodes `arg` is the pool offset of the operand record.

**Labels (easy to get wrong):**
- `0x10`/`0x11` take a **label number in the current script** (`arg`).
  A `0x10` that targets the label on its own instruction is a label
  marker; jumps land just after it, so IR labels go after it.
- `0x15`, `0x16`, `0x22` and links take a **label-table index**. The
  engine's in-memory table has an implicit entry 0, so index `n` =
  stored entry `n − 1`.

**Variables:** bit 15 set = kept in each slot save, clear = in the
system data ("Progression variables"): the slot ones are locals
(`%10+n`, then `%18+n` from n 11), the system ones globals (`%230+n`).
The game uses 19 slot and 3 system variables.
`0x01`–`0x0e` are the arithmetic (`=`, `+=`, `*=`, `/=`, `&=`, `|=`
with a constant or a variable; `0x03` is `-= var`, an engine quirk).
`0x16`-`0x1b` are `if v[a] <cmp> b goto` (see the table). `0x22`/`0x23`
are gosub/return (engine stack depth 20).

**Messages** (`0x40`, `0x41` = voiced, one handler): record `id, size,
line number, segment count`, then that many XOR-`0x53` strings:
pre-wrapped display lines, often with the speaker name first, no inline
markup. A second block follows (`u32 4, u32 0` in every message). Bit 0
of `sub` means "the first line is the speaker's name"; the display code
(`0x475f40`) drops it in text modes that can't show names (`0x4679d0`
checks the mode; History, item 27). All 46,578 messages (15,620 voiced) decode.

**Voices:** clip index = (id − 1) % 100000; `id / 100000` is the
character (3 あずさ … 13 ジーン, 0 minor women, 1 minor men). The rule
hid behind `imul 0x14f8b589; sar 13` (÷ 100000). An earlier statistical
alignment (`research/xuse/voice_align.py`) only matched 87 %.

**Hyperlinks** (`0x46`, 214 uses): the route choices and the
encyclopedia. One record right before the message it applies to, per
linked word: `row, column, length` in characters (1-based, from where
the next message starts), style `0x8000 | 7` (205) or `| 18` (9), and a
label-table index. The "style" is the variable that makes the word a
link (History, item 28). Targets are full scenes (link scenes, label names
`blog?<title>`) that can change the whole screen; `0x47` returns from
one and `0x48` returns to the root scene. Converted as buttons over the
linked words that `gosub` the scene inside `pushscreen`/`popscreen`.
The original's scene stack is an unbounded list (`enter_local_scene`
`0x4674e0` → `0x463660`), and encyclopedia entries link to each other,
so a player can nest them without end; ours nests at most 7 deep
(`MAX_LINK_DEPTH`; the user's choice): inside the deepest, link words
aren't buttons. The game's own scenes nest five.
60 scripts are named `15_Blog…` (ids 96–155).

**Chapters:** chapter titles are label names in `0.cd`:
`chapter<route>]?<title>` (251 chapters). `0x36` sits on that label
(pool: own label number, table index of the `|chpe` label) and sets the
title (local `$150`, shown in the chapter bar and save slots). `0x37`
on `|chpe` closes the chapter and jumps to `|chpl` (the next
instruction); `0x38` on `|chpl` clears the name. `0x93 N` unlocks scene
N of the scene replay (`%3000+N`); `0x94` ends a replayed scene.
`rm<NNN><title>` names in the exe are the scene-replay list.

**Opcodes** (handlers, records and how each is converted):

| Opcode | Uses | Handler and record | Meaning and conversion |
|---|---|---|---|
| `0x40`, `0x41` | 46,578 | see "Messages" | message, voiced message |
| `0x10`, `0x11` | 895 (`0x10`) | label number | goto |
| `0x15`, `0x16`, `0x22`, `0x23` | 442, 82, 3,094, 46 | label-table index | goto, if-goto, gosub, return |
| `0x36`–`0x38` | 251 each | see "Chapters" | chapter bookkeeping |
| `0x66` | 205 | **solved** (`op_66` `0x475cd0`, callback `0x475bf0`): low 16 bits of arg = ms; when the high half isn't 0 (133 uses) a click or skip mode ends the wait, otherwise (72) nothing does | timed wait: `delay` (clickable) or, high half 0, `Op::Pause` = `wait` (fixed in the audit; all were `delay` before) |
| `0x72`, `0x74` | 217, 11 | **solved** (`op_72` `0x44a360`, `op_74` `0x44a390`, both → `script_fade` `0x44a2f0` → `fade_start` `0x436000`, `CTaskWork_Fade`): low 16 bits of arg = ms; high half ≠ 0 sets flag bit 0 (the other direction, `0x435f60`; no script uses it); `0x74` adds bit 0x10 = white instead of black (`fade_draw` `0x435f20` fills with `0xffffff` or 0, opacity elapsed × 256 / ms). Only with effects on and not skipping (`0x51ed14` bit 0, `0x51f82e`, `0x51f82c`). All uses fade out (141 × 500 ms, 44 × 3000, …), almost all at a scene's end, before `0x94 0x93` and a link return (`0x47`) or a chapter end; 202 with the text window still up. Filmed under wine (`0004.cd` #23 patched to `0x72 2000`): the colour covers the picture and the bust-ups but not the text window, its name plate and the bars; the text goes on over it; it stays through a `0x76` transition, a later bust-up shows over it, the next `0x54` background replaces it. No script shows a bust-up between a fade and the next background | a fade to black / white (`Op::ScreenFade`): an opaque `lsp2 26` faded in with `print 10,ms`, removed with the next background, `csp -1` and a link return (`popscreen` doesn't keep sprite2) |
| `0x77`, `0x71` | 234, 3,908 | `op_77` `0x444090`: u32 mask (`mask%04u.png` in `HD/mask.arc`), i32 softness (sign = direction), then `0x76`'s u32 ms, flags, layer | `0x77` arms a mask wipe (`Trans_MaskBlend2`), `0x76` (below) any other transition; `0x71` (`op_71` `0x44a1c0`) waits for it, revealing the changes made since |
| `0x8f` | 414 | arg -1 | remove every bust-up |
| `0x43` | 578 | arg 0 waits for the fade, 1 doesn't (handler `0x4772a0`) | hides the text window, before most transitions: `textoff` |
| `0x52`, `0x53` | 1,117 / 501 | | text tags "brn" (continue below after a click) and "p" (new page), only in full-screen modes |
| `0x4c`, `0x4f`, `0x50` | 9 each | mode, style (`TextWnd{style}.004`), position | text window preset parts, only in script 2's preset subroutines |
| `0x47` | 160 | | return from a link scene |
| `0x62`, `0x30` | 1 each | | not converted: only in the unreachable debug menu |
| `0x8e` | 10,271 | **solved** (`op_8e` `0x4347b0`, entries copied by `0x434530`, applied by `0x434130`): `sub` × 44-byte entries of u16 `mode, character, sprite number, depth`, i16 `x, y`, then a 32-byte module name (empty in every entry; `0x434130` would attach a module of type `0x77` made from it). Mode bit 0 shows (clear: removes), bit 2 keeps the object and its filter, bit 1 = not a change: the 500 ms `Trans_CrossFade` (`mov eax,0x1f4` before `0x442c70`) starts only if some entry lacks bit 1, so the 1,301 records whose entries all have it show at once (fixed in the audit). The depth (10-150, and 1100-5000 in 62 entries) is the bust-up's drawing order, not a fade time: bust-ups fade over a fixed 500 ms. The sprite number indexes `bu*.004` (128 per file, 0-based), and the file names' character codes match the `character` field | show `sub` character sprites |
| `0x88` | 45 | **solved** (handler `0x415b50`, `AdvObjectScript_Filter.cpp`) | define one module of an effect preset: u32 preset, u32 slot (0 = clear), u32 module field, u32 kind (module type = kind + 0x63; creator table at `0x501434`: 1 Smooth, 2 Negative, 3 HueSatura, 4 Brightness, 5 GrayScale, 6 Monotone, 8 Color, 9 BrightnessInvert), u16 blend (index into Alpha/Add/Multi/Screen/SignedAdd/Copy/dissolve; the game uses Multi and Copy), u16 opacity (256 = full), then kind data (colour: R, G, B; brightness: i16). Field names from the module's debug output (`Opacity=%u, Blend=%s, Brightness=%d`). Script 2's `s2_l6`–`s2_l16` set preset 1 to one tint per lighting mood |
| `0x90` | 651 | **solved** (handler `0x434960`) | attach a preset to a bust-up: u16 character, u32 module field, u32 preset (0 = remove). Module type 1 = `CModule_Preset`. Converted to CONscripter's `spfilter`, reading the preset from variables (the engine's `COMMANDS.md`) |
| `0x54` | 2,195 | **solved** (`op_54` `0x422380`): `arg` = background number, 1-based (0 = none); `bg*.004` holds 64 per file (`n >> 6` in the exe's loader at `0x40bff0`). The most common is 1 = `Black`. The handler first removes background slots 1-10 (`0x4132d0`: the background and the `0x55` overlays), puts the picture on slot 1 (layer 1, depth 0, marked seen) and, for `sub` 1-3 (41 uses of 1), picture `sub + 5` (`frame01`-`03`) on slot 2 (layer 1, depth 1: under the bust-ups). It starts no fade: 2,190 uses are inside an armed transition, the other 5 show at once. Before the audit the overlay stayed up across backgrounds (86 places), `sub` was ignored and backgrounds outside a transition faded over a guessed 500 ms | set background: `Op::Overlay(None)`, `bg`, then the frame as the overlay |
| `0x46` | 214 | **solved** (from the handler at `0x477460` plus a user-supplied example) | **hyperlink** on the next message: row, column, length, style, target (see 3c) |
| `0x49` | 565 | **solved** (handler `0x477990`): u16 line, u16 column, u16 length (30000 = to the end of the message; the game wraps enlarged messages short enough to fit), u16 size (percent below 0x1000; 0x1000 + n = n × 25 percent from the table at `0x4fa850`, 0x1003 for mumbling, 0x1005-0x1007 for shouting; 0xf000 = keep), u32 colour (COLORREF, 0xff000000 = keep), u32 edge colour; lines and columns count as for ruby; later records override earlier ones | size, colour and edge for characters of the next message: `#RRGGBB`, CONscripter's `!zNNN` size tag and `!o#RRGGBB` outline (the engine's `COMMANDS.md`) |
| `0x4e` | 24 | **solved** (handler `0x494750`): arg = indent << 16 \| 1 (once 2); the indent is in half-width units of the first line (ASCII, half-width kana and Greek count 1), up to the text after the chat log's handle | hanging indent of the next message; the game puts it only before chat messages long enough to wrap. Converted to `indent N` around the message, N measured on the emitted line |
| `0x70` | 457 | **solved** (handler `0x415a30`, objects `filter-<slot>`): u32 slot (0 = clear all), u32 layer \| depth << 8 (layer byte through the table at `0x503e68`: 0/2 → 0x10, 1 → 0, 3 → 0x20, 4 → 0x30; depth = the sort depth within that band, mostly 1000, above the bust-ups; no fade of its own), u32 kind (0 = clear the slot), u16 blend, u16 opacity, then 0x88's kind data; monotone (6) is a u32 palette, `pal%04u.pal` in `HD/pal.arc` (RIFF PAL, 256 entries for the grey levels) | filter on the whole screen, from 0x88's modules. Converted to `screenfilter` + `print 1` (or the surrounding transition); Smooth (1, u8 x, y range), HueSatura (3, i16 hue, saturation) and BrightnessInvert (9) as `screenfilter` kinds 6-8; Filter-Replace (13: u32 map, i32 direction, distance, begin angle in 1/100 turn) as a still `screenripple` in slot 4 + n, starting at the begin angle (`screenripple`'s phase, History, item 38). Kinds from the (type, creator) pairs at `0x501428`. Zigzag (12, one use: i32 centre x, y, direction, distance, frequency, begin angle, reduction; kernel `0x43f0b0`: each pixel from distance × sin(2π·i/1024) along the direction, i = d × frequency/100 for the distance d from the centre) as a still `screenripple` over a generated ring map, its Multi blend as `screenripple`'s blend 1 and its phase as the exe's (from the frequency: the draw `0x41dab0` never reads the begin angle or the reduction, History, item 39) |
| `0x59` | 47 | **solved** (handler `0x473610`) | BGM crossfade: u32 fade ms (capped at 30000), u16 track to stop (0 = all but the new one), u16 track to play; the new track starts fading in, then the old fades out, at once. Converted to CONscripter's `bgm "file",ms`: the old track fades out as the new fades in |
| `0x82` | 38 | **solved** (handler `0x413f10`) | set an object's opacity (field +0x1b2, 0-256): u8 layer, u32 depth, u16 opacity; the object is found by key = depth (layers 0, 1), depth + 10 (layer 2: character `depth`'s bust-up, as 0x90 finds it), + 110, + 120 (3, 4). Converted for bust-ups (34) and the 0x55 overlay (layer 1 depth 2, 3) as `msp n,0,0,-255:msp n,0,0,alpha`; a new picture is opaque again. layer 1 depth 1 (the slot-1 fill, one use after `0x56`) as that filter's strength |
| `0x84`, `0x7d`, `0x7c` (+ `0x7a`/`0x7b` on layer 2) | 10, 5, 6 (6, 8) | **solved** (handlers `0x414180`, `0x413940`, `0x414460`; objects found as in `0x82`) | `0x84` loads a picture into an object: u8 layer, u32 depth, u8 kind, u32 number, u8 ?; image id kind × 10,000,000 + number - 1 (kind 2 = bust-ups, numbered as in `0x8e`). `0x7d`: u8 layer, u32 depth, u8 horizontal, u8 vertical anchor (0-2: left/top, centre, right/bottom; a new object's is 0). `0x7c`: u8 layer, u32 depth, u8 visible. The blog scenes make a hidden layer 2 object with `0x7a` (depth = a character, band 4 above the bust-ups, at (650, 320) or (250, 375)), load a bust-up, centre it and show it; 0135.cd steps a 0-100 % meter (`bu99_*`) a picture a second. `0x7b` on layer 2 removes it, or a bust-up `0x8e` made (0011-0013.cd). Converted as the character layer's sprite (`Op::Sprites`), drawn over the text window (`Op::SpriteOverText`, `spovertext`): `0x7a`'s group 4 is the top sort band (History, item 37) |
| `0x99` | 8 | **solved** (handler `0x478b00`) | sets progress bit `sub` of the global word `0x51eb3c` (saved with the system data, `0x45f801`): bits 0-3 at the start (0000.cd, 0003.cd), 4-7 at the end of each heroine's route (0009-0013.cd). Read by the extras: the title (bit 3, `0x45cce9`), getters for bits 1, 2, and the voice mode (`VoiceMode.cpp`, bit 4 + route via `0x4fbc00`). Converted as global flags `%3600 + bit` for the extras to come |
| `0x9d` | 6 | **solved** (handler `0x478b30`) | marks CG `arg` (1-based, as in `0x54`) as seen without showing it: bit arg - 1 of the global set `0x51ea18`, read by `CGMode.cpp` (`0x44cd56`, `0x44d4af`); `0x55`'s "mark as seen" byte sets the same set (`0x42236b`). Used right after a `0x54` of a related CG (467 → 523). Converted as global flags `%2200 + n` (n up to 576; above the link flags, below the 4096 variables) for a CG gallery |
| `0x56`, `0x83`, `0x67` | 3, 1, 2 | **solved** (handlers `0x4224a0` → `0x412fe0` kind 1, `0x414050`, `0x473d20`) | `0x56` fills a background slot with a colour: u8 slot (0 removes slots 1-10), u8 layer, u32 depth, u32 COLORREF; a screen-sized object with fill module 7 (`0x55` is the same with kind 0, a picture). Slot 1 = the background, 2 = the overlay. `0x83` sets an object's blend (u8 layer, u32 depth, u8 0-6: Alpha, Add, Multi, Screen, SignedAdd, Copy, dissolve). `0x67` yields one frame. Uses: black background at boot; the shared impact routine `s2_l2` (9 calls) puts white on the overlay, sets dissolve and fades it out over 1 s (`0x86` opacity motion); 0006.cd fills the background red at opacity 64 (`0x82`). Converted with generated `system/fill_rrggbb.png`: `bg` / the overlay; the fade-out as a 1 s cross-fade removal that the following 1 s wait no longer adds to; the red (slot 1 on layer 3, the band over the bust-ups and the static) as a black background under `screenfilter 4,1,0,64,255,0,0`, cleared by the next `0x54` (History, item 40) |
| `0x39` | 4 | **solved** (handler `0x475af0`, `ScriptStd.cpp`) | clears the chapter history (an error inside a chapter; nothing in replay modes 1/2): start of 0004.cd, after the ends of 0021/0022.cd. No chapter history yet. The end of 0022.cd (存在消滅) leaves the static at 256 and jumps back to `s4_l1`; the original starts that week clean (filmed under wine), but no code clearing it was found (not `op_15`, `op_36`-`op_39`, `op_68`, `op_93`/`op_94`, nor the noise module) | converted as a clear of the environment effects (`0x8c` slot 0: `Op::Noise(None)`, `screenshake 0`), seen in the original, mechanism not found |
| `0x3b`, `0x3c` | 2, 0 | **solved** (handlers `0x478650`, `0x4786c0`) | `0x3b` copies `sub` bytes of the record into the game state at +4 (an entry name: the boot script sets "debug entry" or "release entry"); `0x3c` jumps when the name matches and is never used. Nothing emitted |
| `0x91` | 2 | **solved** (handler `0x4788d0`, task `0x454a60`, `StaffRoll\Staff.cpp`; descriptors via `0x517f30`) | the staff roll: arg 0 → `StaffRoll1.004` (63 s, BGM 23), 1 → `StaffRoll2.004` (223 s, BGM 22, the CG slideshow). A panel of key frames (clip rectangle, two picture layers with dx, dy, opacity) and a column of credits scrolling from `start` to `end`; details in `xuse/staffroll.rs`. Rendered by the converter into an MP4 with its song, played with `mpegplay` (not skippable, like the original). Checked frame by frame against the original under wine |
| `0x48` | 2 | **solved** (handler `0x477900`, `ScriptText.cpp`) | returns to the root scene: leaves every open link scene (`0x467670`); at the root it logs "既にルートシーンにいるので何も行いません。". Routes 10 and 11 (entered from links) start with it and end with jumps. Converted as `Op::ReturnToRoot`: `return` through each link page, whose cleanup puts its screen back and then goes to `*conv_unwind` instead of showing the page again; at depth 0 it jumps to the label after the 0x48 |
| `0x4a` | 2 | **solved** (handler `0x477af0`; values through `0x491d80`/`0x491e30` as for `0x49`) | the style of the following messages: u16 size (0x1000 + n preset, < 0x1000 percent, 0xf000 + k: 0 none, 1 default, 2 kept), u32 COLORREF (0xff000000 + k likewise), u32 edge colour (stored at +0x60 and +0x64). 0011.cd: four one-character pages at 125 % orange, then the defaults. Converted as a whole-message style under the message's own `0x49` styles; edge not converted |
| `0x17`-`0x1b` | 2 | **solved** (handlers `0x474460` setne, `0x4745e0` setl, `0x474520` setg, `0x474580` setle, `0x4744c0` setge) | `if v[a] <cmp> b goto label[sub]` like `0x16` (==), the jump taken when the comparison holds. Converted as `Op::If` |
| `0x96`, `0x00` | 1, 1 | **solved** (handlers `0x478330`, `0x473d20`) | `0x96` stops the sounds and starts the title task (`0x45b6f0`, mode 0), at the end of the epilogue: converted as `reset`. `0x00` shares 0x67's handler: a one-frame yield, nothing emitted |
| `0x8d` | 1 | **solved** (handler `0x4155c0`, wait task `0x4154b0`) | waits until environment slot n's module (object key n + 0x78) has finished: u32 slot, u8 0xff. 0007.cd waits out a 3 s ripple, then clears it. Converted as a `delay` of the module's time (exact there: it follows the 0x8c that set the slot) |
| `0x80` | 1 | **solved** (handler `0x413cd0`; fields +0x1a8/+0x1ac, 100 for a new object; key-frame motions set them with flag 0x80) | an object's scale: u8 layer, u32 depth, i32 x, y percent. 0013.cd: character 12 at 80 % after 0x82 made it translucent. Converted as a pre-scaled copy of the bust-up centred on the same point, then the opacity again |
| `0x69` | 32 | **solved** (handler `0x473940`, wait callback `0x473810`; checked under wine with a patched `0002.cd`) | no record: a click wait showing the picture alone (text window and bars hidden, the wait icon in the corner), passed by skip mode. Converted like 0x68 with `skippable` |
| `0x86`, `0x87`, `0x7f` | 19, 9, 12 | **solved** (handlers `0x413540`/`0x413350`, `0x413820`, `0x413bb0`; objects found as in `0x82`) | `0x86` adds a module to an object (u8 layer, u32 depth, u32 slot, u32 kind; kind `0x80000001` removes the slot's module) — always a key-frame motion (`CModule_Animation_KeyFrameMotion`, setup `0x416fe0`): the kind is its flags, which fields each key frame has after its u32 ms (0x10 u8; 0x20/0x40/0x80 i32, i32, u8 with 0x40 the position; 0x100 i32 rotation, u8; 0x200 u16 opacity, u8), then a u16 key frame count; the first frame is replaced by the object's state; each u8 is the field's interpolation (`0x4176f0`: 0 kept, 1 a jump halfway, 2 linear, 3 a curve). `0x87` waits for it (u8 layer, u32 depth, u32 slot, u8 `0xff`); `0x7f` puts the object's centre at i32 x, y. The game slides bust-ups (9: start, wait, remove, `0x7f` to the end point; all inside an armed transition, so the original fades and slides at once) and fades the overlay out once (`0x200`, `0002.cd` #14). Converted as `Op::SpriteMove`: `*conv_move` (`getsppos`/`getspsize`, `amsp` steps on the timer); the transition is shown first, then the slide. The rotation (field +0x1b0, tenths of a degree mod 3600, clockwise; `0x81` sets it: u8 layer, u32 depth, i32 angle) of -900 in `0043.cd` #440 is drawn by `Op::SpriteTurn` (lsp2/amsp2, the sprite ends off screen), when its interpolation byte is 2 (linear, as there; History, item 41). Key-frame group 0x80 is the scale (+0x1a8/+0x1ac) |
| `0x8c` | 240 | **solved** (handler `0x4153c0`, `AdvObjectScript_Env.cpp`, objects `environment-<slot>`): u32 slot (0 = clear all), u32 layer \| depth << 8 (the word goes to `0x4109a0` as the object's sort key, see `0x4126c0`; the game uses depth 1000), u32 kind (0 = clear the slot), then the kind's module data (setup table `0x5013bc`; module class from the creator table `0x501434` + RTTI): 1 Animation-Simple on `HD/env.arc` image 50,000,000 + n (light rays, unused), 2 Animation-RandomNoise (88 uses), 3 Filter-LensFlare (unused), 4 Animation-Replace (25), 5 Animation-Vibrate (12). Layouts and maths in `cd.rs` (`Noise`, `Ripple`, `Shake`) | static: an animated sprite (4 generated frames, `system/noise.png`) at the end opacity, sprite 60, kept across `csp -1` like `0x55`; its rise from opacity 0 over `time_ms` (up to 2 s) is CONscripter's `spfade` (`Op::NoiseFadeIn`). Ripple (draw `0x41a2d0`, per pixel `0x43d9d0`: each pixel from amp × sin(2π(grey/256 + phase/1000)) pixels away along the angle, map `mask%04u.png`, blended at the opacity) → `screenripple`; shaking (draw `0x41ac70`: the picture blitted onto itself at a sine or 7-point-path offset) → `screenshake` |
| `0x6d` | 198 | **solved** (handler `0x442000`, object `0x441ee0` at sort key `0x06000000`, the bottom of the text window's band): u32 speed, u32 x and y range in pixels (mostly 80, 20, 20). The picture moves along 0x8c's 7-point path, `speed` points a second; after 4 points both ranges shrink by max(speed / 4, 50) px/s until 0. The script waits; a click ends it. Skipped while skipping or with effects off. Filmed (speed 80, 20 × 20): the picture jumps to the next point every frame (30 a second), at the full range for the first points, then shrinking | a quake, often right after a sound effect. Converted to `screenshake 4,…` (mode 1, at most 30 points a second, the full range for 4 points plus the switching frame (`cb_00441e50`, confirmed by the breakpoint trace; 166 ms), then shrinking at the same rate) + `delay`, twice, then `screenshake 4` |
| `0x6e` | 88 | **solved** (handler `0x441550`, object `0x4413d0` at sort key `0xffffffff`, over everything): `arg` = COLORREF filled over the screen with blend 1 (Add) at opacity 128 for 100 ms; the script waits, a click ends it. White for lightning and shocks, red (`0x0000ff`) for blows | a flash. Converted to CONscripter's `screenflash 128,"#rrggbb"` (`Op::ScreenFlash`; the colour added over everything, the text window included, as the original's sort key puts it) for a 100 ms `delay` |
| `0x6f` | 87 | **solved** (handler `0x441a00`, `Surprise.cpp`, object at sort key `0x06000000`): u32 time (1-32767 ms), u32 preset 0-8 from the table at `0x500b68` (zoom x/y 16.16 = 10/15/20 %, opacity 100/150/200, tint colour/opacity/blend: none for 0-2, light red `0x8080ff` Multi for 3-5, white blend 6 for 6-8, which tints nothing: the colour fill `0x428b50` dispatches through the table at `0x518208`, whose entries for blends 5 and 6 are `0x443090`, a bare `ret`). Takes a snapshot of the picture, tints it, and lays it over the picture scaled by 1 + f × zoom about the centre at f × opacity, f = sin(π t) over the time; the script waits | a "surprise" pulse. Converted to CONscripter's `screenpulse 3,…` (zoom 100/150/200 ‰; opacity 0-256 in both; Multi → `tint` 1, blend 6 → no tint) + `delay` + `screenpulse 3`. Checked against the code (`surprise_create` `0x441730`, step `0x441670`, draw `0x4415b0`), not filmed |
| `0x57` | 542 | **solved** (handler `0x473530` → sound request `0x47aab0`, channel 0) | play BGM: `arg` = track (1-based, `bm0000.bin` slot `arg - 1`), `sub` = fade-in ms (capped at 30 s) |
| `0x58` | 579 | **solved** (channel 0, stop flag) | stop BGM: `arg` 0 = all, `sub` = fade-out ms |
| `0x5a` | 2,633 | **solved** (`op_5a` `0x4736f0` → `sound_request` `0x47aab0`, channel 1) | play SE: packed 9-byte record `u32 fade-in ms, u8 loop, u32 sound` (1-based across `k0000.bin` + `k0001.bin`); a non-looping SE isn't played in skip mode (converted: `Op::SeNotSkipped`, `isskip` before the `dwave`). The fade-in (41 uses, 50-3000 ms, mostly loops) is CONscripter's `dwavefadein` (`Op::SeFadeIn`; added after the audit) |
| `0x5b` | 890 | **solved** (`op_5b` `0x4737a0`, channel 1, stop flag) | stop SE: `arg` = sound or 0 for all, `sub` = fade ms: `dwavefadeout ch,ms` (`Op::SeFadeOut`), which fades and stops while the script goes on |
| `0x76` | 3,670 | **solved** (`op_76` `0x449df0`): `sub` picks the `Trans_*` class through a jump table (0-3 Wave, 4 Dot4x4, 5-8 SoftEdgeWipe, 9 CrossFade, 10-17 Slide in, 18-25 Slide out, 26 Dissolve, 27 Mosaic, 28-31 BoxRolling, 32-35 RollFlip, 36 SmoothCrossFade, 37-40 Zoom, 41-44 Scroll, 45-47 MeltDown, 48 TvPowerSwitch); the record is u32 ms, u32 flags (bit 0 hides the windows, bits 0-1 remove the bust-ups; always 0), u32 layer byte \| depth << 8 (the transition's sort key: what it covers). Skipped when skipping or with effects off | screen transition. The 3 `0x76` and 9 `0x77` records with another key cover only the objects sorting below it (History, item 15): in 11 of them that is everything they change, and in `0011.cd` #193 a filter cleared inside a 0.5 s cross-fade goes at once, before it (History, item 20) |

**Opcode audit (2026-10-14; per operand since 2026-10-16).** Every
opcode the converter handles was read again in the decompiled exe
(`ghidra_export.sh`; every handler of the dispatch table is named
`op_XX` in `names.tsv`) and compared with the rows above and the
converter. Fixed then: `0x76` keyed under a layer-0 filter change (the
change shows at once, `0011.cd` #193); `0x72`/`0x74` fades (were
waits); `0x39` clears the static; `0x8e` records with every entry's
mode bit 1 show at once; `0x54` removes the overlay, puts
`frame01`-`03` up for `sub` 1-3 and starts no fade; `0x66` with a zero
high half can't be clicked through; `0x5a`/`0x5b` sound effects fade in
and out.

**Operand coverage.** The converter reads every operand through
`CdScript::sub`, `CdScript::arg` and the pool readers in `cd.rs`, which
note what was read. The operand records of the opcodes in
`POOL_RECORDS` (`convert.rs`) lie end to end in instruction order and
fill the pool, so each record's length is known. After lowering, the
dry run counts each operand that was given and never read, whatever
its value (0 included), as "`<opcode>` operand `<which>` ignored":
`which` is `sub`, `arg`, `+n` (unread record bytes from byte n) or `-n`
(an unread run ending the record, n bytes before its end: the tail of a
record of varying length). Each such call is a gap in `metrics.json`.
Not noted: `sub` 0xffff on the opcodes in `NO_SUB` and `arg` -1 on
those in `NO_ARG` (`convert.rs`), the compiler's filler: every use of
those opcodes has it, and their handlers (`op_XX`) never read the
instruction's +2 or +4. The harmless operands are in
`UNNEEDED_OPERANDS`, each with the values it covers (a different value
is noted) and why:

| Opcode | Operand | Values | Why |
|---|---|---|---|
| `0x40`, `0x41` | `+8` | any | the source line number (`cd messages` prints it) |
| `0x40`, `0x41` | `-8` | u32 4, u32 0 | after the text of every message |
| `0x36` | `-4` | any | the label index of the chapter's `\|chpe` label |
| `0x37`, `0x38` | `+0` | any | its own label number: the label table places it |
| `0x3b` | `sub`, `+0` | 11, 13; "debug entry", "release entry" | the entry name and its length: only `0x3c` reads it, and no script uses `0x3c` (`op_3b` `0x478650`) |
| `0x43` | `arg` | 0, 1 | neither waits for the window's fade-out: `op_43` (`0x4772a0`) goes on in the same frame for 0, the next for 1; ours goes on at once (History, item 32) |
| `0x52`, `0x53` | `sub` | 0 | text window 0, the only one (`FUN_00476a80`: `sub` picks the window) |
| `0x52`, `0x53` | `arg` | any | the click wait's read-flag number (`FUN_00476a80` keeps it at the window's +0x10, the wait `0x476750` marks it read): CONscripter's kidoku keys the wait by its place in `0.txt` (History, item 34) |
| `0x6d` | `-12` | zeros | `op_6d` (`0x442000`) reads only the record's first 12 bytes (speed, x and y range; y goes to `quake_object` in EAX, which Ghidra drops) |
| `0x76` | `+4` | 0 | the flags (`op_76` `0x449df0`: hide the windows, remove the bust-ups): neither |
| `0x77` | `-8` | flags 0, sort keys 0x271000, 0x3e800, 0x6403, 3, 0 | the 9 records below the top band change only what they cover (History, item 15) |
| `0x7a`, `0x7b`, `0x8a`, `0x8b` | `+1`, `-1`, `-4`, `-9`, `+1` | depth 1000 (slot 1, visible) | the one full-screen animation (sprite 59) |
| `0x83` | `-1` | 6 | the blend, dissolve, which the converter draws |
| `0x84` | `-1` | 0 | `op_84` (`0x414180`) never reads the last byte |
| `0x86` | `+5` | 1 | the module slot: one motion per object |
| `0x86` | `+15` | zeros | the first key frame, which the setup (`0x416fe0`) replaces by the object's state |
| `0x86` | `-2` | 0 | two bytes after the key frames of the 9 slides, which the motion's load (`0x416fe0`) doesn't read (History, item 45) |
| `0x87` | `+0` | layer 2, any depth, slot 1, 0xff | waits for the motion its object's `0x86` started; ours runs it to its end at `0x86` |
| `0x8d` | `-1` | 0xff | as in every `0x87` and `0x8a` wait record |
| `0x88` | `-4` | 0 | a cleared slot's blend and opacity, which `op_88` (`0x415b50`) doesn't use |
| `0x88` | `+8` | u32 0, 2, 1000, 2000 | the module's place in its preset, where the modules set together are in slot order (History, item 36) |
| `0x7a` | `+6` | 2000 | a blog bust-up's depth in the top band, over everything else there (History, item 37) |
| `0x7a` | `+14` | u16 0, 0 | the new object's width and height, which the picture `0x84` loads replaces (History, item 44) |
| `0x46` | `-4` | 0 | the link entry's first field, which only the save and load read (History, item 43) |
| `0x70` | `-4` | 0 | a cleared slot's blend and opacity (kind 0 or slot 0), which `op_70` (`0x415a30`) doesn't read (History, item 38) |
| `0x44` | `+6` | u16 3 or 1, u32 0 | the ruby's alignment: spread (as `rubyon`) or centred over the base, the same here; the u32 isn't drawn (History, item 33) |
| `0x8c` | `+4` | layer 0, depth 1000, 1100, 2000, 500; 0 when clearing | the effect's sort key: every one sorts over all the bust-ups, as ours draws them (History, item 31) |
| `0x4a` | `-4` | `0xff000000`, `0xff000001` | no edge colour of the record's own, or the default one: the default decoration (History, item 29) |
| `0x70` | `-8` | 10, 0 | Zigzag's begin angle and reduction, which its draw (`0x41dab0`) doesn't read (History, item 39) |
| `0x56` | `+2` | u32 0, 2000 | a slot-1 fill's sort depth in its band, where it is alone (History, item 40) |
| `0x90` | `+2` | u32 1, 0, 2 | the preset module's place among the bust-up's modules, where it is alone (History, item 35) |
| `0x8e` | the six module names | byte 0 is 0 | an empty module name (leftover memory after it) |

A parsed
field the converter never used counts as not read: `cd.rs` doesn't
parse those (ruby flags, `0x4a`'s edge colour,
`0x70`'s depth); link styles and
message line numbers are read only for `cd messages`, outside the
tracking.

Per operand or behaviour (counts: instructions; "notes" are the dry
run's):

| Opcode | Operand or behaviour | State |
|---|---|---|
| every opcode the corpus uses, except as below | every operand | converted |
| `0x40`, `0x41` | `sub` bit 0: the first line is the speaker's name (not in full-screen windows: the exe drops it there) | converted (History, item 27) |
| `0x49` | the edge colour: the one of its own (0x808080, a grey outline; History, item 29) | converted (CONscripter's `!o#RRGGBB` … `!o0`) |
| `0x56` | u32 depth at +2 (0 and 2000): 2 notes | dropped |
| `0x70` | the depth in the second u32 among the bust-ups (80, in 2 negative flashes) | partial: drawn over all the bust-ups (History, item 30) |
| `0x76` | MeltDown's stretch for `sub` 46/47 | converted (scaled from the exe), not compared with the film |
| `0x8f` | removes the bust-ups only | partial: `csp -1` also removes the fade cover (no script shows the difference) |
| `0x30`, `0x62`; `0x40` `0001.cd` #1 | everything; the 60 bytes after the message | dropped (the unreachable debug menu; `LEFT_OUT` in `xuse/build.rs`) |

Two doc mistakes corrected on the way: `0x8e`'s last 32 bytes are an
(always empty) module name, not junk; `0x76`'s third u32 is a sort key,
not an easing curve. `0x8f` removes only the bust-ups (keys 11-110), yet
is converted with `csp -1` (the overlay, static and animation are put
back after it; the fade cover goes, which no script shows the
difference of).

`0x76`'s `sub` → transition kind is in `transition_style`
(`crates/conv-engines/src/xuse/convert.rs`); how each kind is drawn is in
the engine's `COMMANDS.md` (the `cons_*.dll` effects).

## How the game maps to CONscripter

The target is the WebAssembly build first, so every system screen is a
real graphical menu built in the script from the game's own parts
(`spbtn`/`btnwait`/`getmousepos`), and assets are packed into `.nsa`
archives.

- **Movies:** opcode `0x97` plays `HD/mvNNNN.bin` (MPEG-1 program stream);
  arg's low 16 bits are the number, the high 16 bits whether a click or
  key ends it (handler 0x478ab0, checked at 0x408e5b). The handler
  ends skip and auto mode first (`skip_state_a`, `0x51f82d`), as a
  `0x68` with arg 0 does (`op_68` `0x4784b0`): both are converted with
  `skipoff:autooff` before them. The boot script
  (opening, `mv0001`), script 7 (`mv0002`, not skippable) and the
  recollection screen (all three) use it. ffmpeg encodes them as H.264 +
  AAC MP4 (`movie/`, loose files, about 55 MB each) and the script runs
  `bgmstop` + `mpegplay "movie\mvNNNN.mp4",skippable`. The web build
  already played `mpegplay` in a `<video>` element over the game; it only
  needed a fix (it waited forever when the browser blocked autoplay with
  sound; now it plays muted). Windows and Android hand the file to the
  system player; the Linux and macOS builds skip movies.
- **BGM loop points:** `bm0000.bin`'s slot param 0 is the loop start in
  sample frames (the second before it matches each track's last second,
  correlation 0.99-1.0); the loop runs to the end of the track. The
  converter writes it into each `.ogg` as `LOOPSTART`/`LOOPEND` Vorbis
  comments, which SDL_mixer (2.0.2+) loops on by itself, sample-exact,
  when `bgm` plays music looped. SDL_mixer ignores `LOOPSTART` without an
  end. Being in the file, the loop survives saves and works in stock
  ONScripter. Not used: ONScripter's `bgm "(seconds)file"` loop-back,
  which restarts and seeks the file (and isn't saved). In ONScripter-Yuri
  it never fires, because `playSound` registers `Mix_HookMusicFinished`
  only when `Mix_PlayMusic` fails (ogapee's ONScripter registers it before).
- **Backlog speaker names and voice replay:** every text line starts with
  a `[name|voice]` tag (its page's first speaker shown on a name plate and
  first voice clip). ONScripter keeps a page's tag in the log, and in
  saves, when there is a `pretextgosub`; ours only runs `saveon`. The
  backlog reads it with `gettaglog`, splits it, shows the name above the
  page and replays the voice when the name (marked ♪) is clicked.
- **Ruby (`0x44`, 37 uses):** the record (u16 line, column, length,
  flags; u32 0; the reading) puts a reading over characters of the next
  message; lines count text lines without the speaker, 1-based. It
  becomes NScripter's `(base/reading)`, added after escaping (which
  turns the text's own parentheses full-width), and `rubyon` draws it.
  `rubyon` puts the ruby in the line gap and draws text a line gap
  lower, so `setwindow` and the backlog's `strsp` start a line gap
  higher and the text stays where it was. ONScripter reads the markup
  bytes singly, so the single-byte padding treats them like colour
  tags. Readings wider than their base push the rest of the line right,
  as in stock ONScripter. The 4 px gap of one full-screen style is
  smaller than its 9 px ruby.

**System screens** (measured from screenshots of the original, not
committed):
- **Text box:** the ADV box and seven full-screen styles at the
  scripts' positions, margins and metrics from the exe's table.
- **Backlog:** a full-screen dark window (like `TextWnd5`, 714×472) with
  names and voice replay.
- **Title:** FIRST START / load / option / EXIT over `Title.004`, with
  the title music (BGM 14) and the exe's intro (History, "Title intro").
- **Save/load:** 5 slots per page with thumbnails, 10 pages,
  delete/save/close, a slot label of date plus chapter name.
- **Settings:** the original is a native Windows dialog. Converted as an
  in-script screen: volumes, text speed and auto speed, "back to the
  title" and "quit", and a second page with the 13 voice switches (11
  named characters, "other women", "other men"; each `dwave` is guarded
  by `if %(970 + c) == 0`). Skip stops at unread text through
  `kidokuskip` / `kidokumode 1`; 「未読テキストもスキップする」 (global
  %908) switches to `kidokumode 0`.
- **Confirmations:** `YesNoDialog.004` with the exe's messages
  ("No.%02u のデータに上書きします。よろしいですか？" etc.) drawn with
  `strsp`, sprites 0-5 above the menus.
- **Button bars:** the bottom bar (auto/skip/log/save/load/setting) and
  the top bars (chapter name, blog/scene, level) from `wnd.arc`/`gd.arc`.
  Right-click can't be sent to a script label, so everything is on the
  bar.
- **Blog bar** (`BlogStackBar.004`): the link scenes entered form a
  stack of levels (0 = the root) that the bar browses like a web
  browser's history. Filmed under wine: back (⌄⌄) goes up one level and
  forward (⌃⌃) down one, exit to the root, and the drop-down lists levels
  1 to the deepest (current one marked », hover purple); every level is
  resumed where it was left, and the level numbers read "current /
  deepest". Back and exit need a level above, forward a level below.
  Going on from a link page, or a scene ending, drops the levels below.
  The list stays open and pushes the level row down 26 px per row.
  Converted with save slots: entering a link scene saves the link page
  to slot 900 + level, going to another level saves the current one
  there and loads the target's (`loadgosub` carries the deepest level
  and titles over the load). After a load from the save menu, the
  levels above the saved one can't be gone back to and the deeper ones
  are dropped (their slots belong to another playthrough). The game
  nests at most five levels; the list shows six.

## Walkthrough

Source: public guides (BIGLOBE, ifs.nog.cc, ugukan.net 最果てのイマ攻略;
Wikipedia for the round structure). Routes are chosen by clicking a
heroine's underlined link in 「最初の日」 (お茶会－０ opens each loop);
keep playing on the same system data, the game loops by itself.

| Loop | In 最初の日 | Chapter / route |
|---|---|---|
| 1 | no link | リピート |
| 2-5 | one heroine per loop | cycle 1: the four routes |
| 6 | no link | リピート again |
| 7 | no link | 死 |
| 8-11 | one heroine per loop | cycle 2 (truth phase, text changes) |
| 12 | none / automatic | 戦争編 → epilogue |

Heroine links (guide's wording / the game's link text): あずさ
「最初の頃」 (2nd link), さやか 「たまに早く出る」 (3rd), ふえこ
「違う制服」 (4th, 「一人違う制服を着た伊月笛子が合流する」), ようこ
「静かな朝」 (5th, 「いつも通りの平和な、静かな朝である」). Start with
あずさ. Every other link is lore; all links fill BLOG-1..3.

The guides list 死 right after cycle 1. COMPLETE's bytecode gives it on
the third no-link loop (`%33 <= 2` → リピート, 0x1a = setle in the
exe's table): the second no-link loop is a リピート identical to the
first (0021 has no conditions). The full-voice note in 愚者の館
(sagaoz.net, "one more no-link pass") matches this. The conversion
follows the bytecode, and `routesim.py` confirms both sides agree.

## Progression variables

The bytecode's variables are two tables (`load_svar` `0x46de00` sorts
them by id): bit 15 set, 150 places at `0x519554`, which the slot save
writes (`LocalSave.cpp` `0x461790` → `SVAR`, seen in the code and in
saves); bit 15 clear, 20 places at `0x519544`, which the system data
writes (`GlobalSave.cpp` `0x45f1a0`, the `SVAR` in
`Farthest2015_000.bin`, seen). The conversion does the same with
NScripter's border (`script_var` in `xuse/convert.rs`): the slot ones
are locals, saved with each slot (n 0-10 → `%10+n`, 11-20 → `%18+n`,
since %21-%28 are conv-cons'), the system ones globals (`globalon`)
from %230 (`%230+n`; %10-%38, the slot ones' old place, still hold
stale values in an older `gloval.sav`).

Slot variables (the 21 names of a slot's `SVAR`, seen; matched to the
ids by role and by the values in a save, inferred): %10-%12 `rv`,
`tmp1`, `tmp2`; %13 `debug`; %14-%17 `us_enable`, `us_disable`,
`us_hide`, `href_enable`; %18 `sentaku_now`; %19 `sentaku_all`;
%20, %29-%31 `sentaku_azusa`, `sentaku_fueko`, `sentaku_yoko`,
`sentaku_sayaka`; %32 `sentaku_zumi`; %33 `sentaku_bad`; %34 `loopsu`;
%35-%37 `loop1`-`loop3`; %38 `clear_count`. Used: all but `tmp1` and
`tmp2` (19). System variables (the 4 names of the global `SVAR`, seen;
which id is which inferred from where they are set and the values in a
100 % save, 1, 0, 0, 1): %231 set to 1 at the end of the epilogue
(`s3_l13`, `war_clear`), %232 set to 1 in あずさ's route (`0009`
`s9_fx110`, `azusa_tsuikabun`) and read in `0015` `s15_l1`
(sets `loop2` when not 0), %233 compared with 4603 (`s6_l1`; `proskip` or
`pkc_result`); id 0 unused. Before 2026-10-18 the conversion had the two
the wrong way round: the slot variables at %10-%38 (global, so loading
an older save kept today's loop and route state) and the system ones at
%10-%13 (in the slot, and cleared by the `reset` right after
`war_clear` is set).

Scripts:
0000 `!boot`, 0004 `00_プロローグ` (お茶会－０ and 最初の日), 0009 あずさ,
0010 さやか, 0011/0012 ようこ, 0013 ふえこ, 0021 リピート, 0022 死,
0019 `09_回想初期化` (scene replay setup).

| Var | Meaning | Set | Reset | Read |
|---|---|---|---|---|
| %13 | 1 debug entry, 0 release entry | 0001 | | |
| %14-%17 | constants 0, 1, 2, 1 (%17 = 0 in the scene replay) | 0000 `s0_l2` | | not read |
| %18 | heroine chosen this loop: 1 あずさ, 2 ふえこ, 3 ようこ, 4 さやか | each link scene (`s9_l1`, `s13_l1`, `s11_l1`, `s10_l1`) | each お茶会－０ (`s4_l2`), `s0_l4` | end of 最初の日: 1 → あずさ (`s9_l3`), else ふえこ (`s13_l2`); さやか and ようこ jump to their routes from the link scene (0x48) |
| %19 | scratch: all four heroines done (`%20 & %29 & %30 & %31`) | `s0_l6` | | `s0_l5`, `s0_l8` |
| %20, %29, %30, %31 | あずさ, ふえこ, ようこ, さやか done this cycle | their link scenes | `s0_l4` (start, and at each cycle's end) | 最初の日 hides the heroine's link (jumps past the 0x46) |
| %32 | a heroine link was clicked this loop | link scenes | each お茶会－０ | 最初の日 hides all heroine links; its end picks a route or no route |
| %33 | loops with no link | end of 最初の日 (+1) | `s0_l2` | `<= 2` → リピート (0021), else 死 (0022) |
| %34 | phase: 1 cycle 1, 2 cycle 2, 3 after it | `s0_l2`, `s0_l5`, `s0_l8`, end of the epilogue | | not read |
| %35 | 1 in cycle 1 | `s0_l2` | `s0_l5` (all four done) | route text variants (23 places) |
| %36 | 1 in cycle 2 | `s0_l5` (all four done) | `s0_l8` | route text variants (40 places) |
| %37 | 1 after cycle 2: the war arc | `s0_l8` (all four done again) | end of the epilogue | route ends: → `s5_l1` (00_共通前, お茶会－１/２) |
| %38 | routes finished | `s0_l5`, `s0_l8` (+1) | | not read |

A route's end calls `s0_l5` in cycle 1 or `s0_l8` in cycle 2, then goes
back to お茶会－０ (`s4_l1`). FIRST START runs `s0_l2`, which zeroes all
of this. Progress bits for the extras (0x99) are `%3600 + bit`.

Checked by `research/xuse/routesim.py` (route simulation of the bytecode
and of `0.txt`, `docs/new-game.md` step 11.4; its `0.txt` side reads
the cp932 scripts of before the UTF-8 builds and no named variables, so
it needs updating before its next use).

## Testing

Manual: none found in the install.

- **Scripted runs:** `python3 -I research/xuse/smoke.py <build>
  <workdir> "1,0,998,1,201,999"` patches a copy of the build and plays
  a click queue (format in its docstring: button numbers at `btnwait`s
  and text waits; bar 1004 save, 1005 load, 1006 settings; dialog 1 yes,
  2 no; settings 30 title, 31 quit; 999 = screenshot
  `<workdir>/shotNN.png`). It shortens effects and delays. `--goto
  conv_link2` jumps to a label after FIRST START (link scenes open with
  an `@` before their link, so queue a 0). It can't answer `systemcall`
  menus: those hang headless.
- **Debug trace** (`docs/new-game.md`, "Checking a build with the
  debug trace"): `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy
  conscripter --root . --save-dir <scratch> --trace t.jsonl
  --trace-ignore 22-48,lsp2:12-23 --auto-play 0`, then
  `engine/tools/cons_trace_check.py t.jsonl --script 0.txt`. The ignore
  list keeps auto-play off the bars and menus (sprite map below); the
  link words 51-58 stay in, since they are the route choices. The
  checker config is `research/xuse/cons_trace_check.json` (`--config`):
  the ignore list and the findings looked at and found meant (the frame
  overlay, sprite 61, that a link return puts back in `s9_records2`; the
  fade cover `lsp2 26`; `%190` after a load;
  ambient loops the scene stops itself; the ripple that stays on through
  the 「粉薬」 link scene `s61_l1`, as in the original). A single run with
  `--auto-choice first` keeps taking the same hub link and left 1,062
  labels unreached: cover the routes with `cons_explore.py`. Found so
  far (`sound`): `se068` twice in `s30_l1` ("二度。三度。", meant), `se088`
  and `se086` in `s13_l147`. A sound listed on two channels in one page
  under auto-play (`se125` on 2 and 8) is usually only auto-play's speed:
  the rule reports a sound started twice before the same click wait. Slots 900+
  hold the blog bar's levels (below), so give `--checkpoint-every` a
  `--checkpoint-slot` elsewhere (e.g. 500).
- **Control flow without the engine:** `python3 -I
  research/xuse/nstrace.py 0.txt 50000000 --pages N [--click K]`;
  routes on both sides: `research/xuse/routesim.py`.
- **Sprite numbers:** 0-5 yes/no dialog, 6-33 menus (more at 980-998, drawn there with `spz`), 34-48 bars, 49
  name text, 50 plate, 51-58 link words (`humanz 58`), 59 full-screen
  animation, 60 static, 61 overlay, 62+ characters; `lsp2` 0-11 save
  menu, 12-23 top bars, 24 flash, 25 a turning bust-up, 26 a fade to a
  colour. Save slots 900 + level hold the blog bar's
  levels.
- **Decompiling the exe:** `research/tools/ghidra/ghidra_export.sh
  <install>/Farthest2015.exe research/xuse/local/ghidra --names
  research/xuse/names.tsv` (pseudo-C per function, git-ignored). The
  opcode dispatch table is at `0x4fb270`; `names.tsv` names it and every
  handler and address this document mentions. Raw disassembly:
  `objdump -d -M intel --no-show-raw-insn Farthest2015.exe`.

## Running the original under wine

Used to film transitions, effects and screens (`docs/new-game.md`
step 9). Helper scripts of the sessions are in `research/xuse/local/`
(git-ignored).

- **Prefix:** a scratch `WINEPREFIX`, never `~/.wine`. In a default
  prefix `drive_c/users/<user>/Documents` (and Desktop, Downloads,
  Music, Pictures, Videos) are symlinks to the real home folders, and
  the game saves to `Documents\Xuse\最果てのイマ COMPLETE\`: replace the
  six symlinks with real folders.
- **Locale:** `LANG=ja_JP.UTF-8` and `LC_ALL=ja_JP.UTF-8`, as for every
  game under wine, so Japanese folder and file names (the save folder
  above) come out right.
- **Silence:** the old `WINEDLLOVERRIDES="winepulse.drv,winealsa.drv,
  winemenubuilder.exe=d"` is not silent under wine 11 (the game was
  heard). Use the null ALSA sink from `docs/games/muramasa.md`:
  `ALSA_CONFIG_PATH` pointing at a file with only `pcm.!default { type
  null }`, `PULSE_SERVER=unix:/nonexistent`, and `WINEDLLOVERRIDES=
  "winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"`
  (and `HKCU\Software\Wine\Drivers` `Audio=alsa`, `Graphics=x11`; the
  game starts without a `ctl.!default`). Stop wine (`wineserver -k`) and
  Xvfb as soon as the pictures are taken.
- **Display:** an Xvfb display of its own, never the user's desktop.
  The game draws at screen offset (3, 29). Clicks need `xdotool
  mousedown`, a short sleep, then `mouseup`; a plain `click` is
  ignored.
- **Install:** the exe reads `HKLM\Software\XUSE_CORP\Farthest2015`
  (32-bit view; `wine reg add ... /reg:32`): strings `Install` =
  `C:\Xuse\ima\`, `HD` = `C:\Xuse\ima\HD\`, `CD` = `C:\Xuse\ima\CD\`.
  Make `C:\Xuse\ima` a farm of symlinks to the install, so patched
  scripts never touch it.
- **Path to the story:** ENTER (405, 463) → click past the intro → wait
  ~14 s → FIRST START (675, 240). Hold Ctrl to skip (it stops at
  links). The wheel opens the log.
- **Patching a `.cd`:** overwrite an instruction in place and fix the MD5
  footer. Repointing a label in the file's own table does nothing
  (cross-script calls resolve elsewhere, probably through `0.cd`). The
  opening calls `s2_l17` before each page; its `0x43` (#73) is a safe
  slot for experiments (`0x91 1` there plays the staff roll at the
  first page).
- **Extras:** they need a cleared save. sagaoz.net has full saves (CG,
  replay and system voices 100 %: `farthest2015_muwokoerumono.zip`);
  copy its `Farthest2015_000.bin` into the prefix's save folder. Title →
  option (660, 315) opens the extras menu: cg 231, bgm 266, chapter 299,
  wss 331, exit 397 (game y, x 650). The "option" heading isn't a
  button; the original has no settings on the title, and in game the
  bar's SETTING opens a Windows dialog.
- **Breakpoint trace** (`docs/new-game.md` steps 18-19): start the exe
  as `winedbg --gdb --no-start 'C:\Xuse\ima\Farthest2015.exe'` (the
  port is on stderr), attach `gdb -batch -ex 'target remote
  localhost:PORT' -x bp.gdb` from `research/tools/wine_bptrace.py
  research/xuse/names.tsv op_6d cb_004420a0 op_66 wait_66_task …`, walk
  to the patched site as usual, and fold the log with `--fold`. Gdb
  stays attached until the game is stopped (`wineserver -k`): no detach.
  Break on the handlers and their per-frame callbacks, not on
  `frame_loop` (History, "Breakpoint trace").
- **Per-frame state trace** (2026-10-19; as 続・殺戮のジャンゴ's,
  docs/games/django.md): `xtrace.sh --frames` writes an `obj` record each
  frame (at frame_loop's Sleep) for every object of the scene whose state
  changed. The scene's object manager sits at `current_scene` +0x28c, a list
  from +0x2a4 (node +8 the object, +0x10 the next) to the sentinel +0x294.
  Each record holds `id` (+0x17c), `pic` (+0x1a4, the picture slot), x/y
  (+0x184/+0x188), `a` (opacity 0-256, +0x1b2), `sx`/`sy` (scale in percent,
  +0x1a8/+0x1ac) and `r` (+0x1b0, mod 3600), as the key-frame motion
  (`0x417580`) and `0x40f160`/`0x40f100` write them. The first 6 pages (102
  s at 1x) show the opening's objects at 100 % and opacity 256.
  `frame_diff.py` matches these nameless objects to our motions by where
  they start (within 64 px). First compare (2026-10-19), the first 300 pages
  at `SPEED=4` (337 s): 298 of 300 pages pair. The original's 10 objects
  (ids 1, 12-17, 111, 112) change state about 60 times, but none moves
  between two pages; ours has no sprite motion there either (only
  effects), so no motion differs. The parts of the game with moving
  pictures need a save to reach them. Not pursued (2026-10-19): the
  reference route's 17,274 pages hold no key-frame motion (`0x86`); the
  moving pictures are fx animations (`0x8b`) and screen effects, whose
  timing `trace_diff.py` already compares, so `--frames` stays as it is.
- **Trace speed**: `SPEED=4 GAME=<install> research/xuse/tracedll/xtrace.sh
  ...` plays 1.0 s a page instead of 3.8, its `f` and `ms` in game time
  (17 of 17 timed steps within 2 frames of a 1x run); `SPEED=8` drifts
  by up to 10 frames (`docs/new-game.md`, "Trace speeds").
- **Trace DLL** (`docs/new-game.md` step 22): `GAME=<install>
  research/xuse/tracedll/xtrace.sh -o t.jsonl [--save slot.bin]
  [--choices 12,34|none] [--pages N] [--seconds S] [--shot end.png]
  [--no-cover]` builds `xtrace.dll` and `xlaunch.exe` with mingw,
  keeps a prepared prefix in `out/xtrace/` (the steps above), starts
  the game on its own Xvfb and clicks to FIRST START (or loads slot 1),
  then the game plays in its own auto mode. One JSON line per
  dispatched instruction (`"t":"cmd"`: `op`, `sub`, `arg`, `src`, `"at"`
  = `<script id>:<instruction number>` as `cd dump` counts them, `ms`
  since the DLL loaded, `f` frames counted by `frame_loop`'s `Sleep`),
  one per message (`"t":"page"` with its segments and voice id) and one
  per page with links (`"t":"choice"`, `kind` `link`: each link's
  label-table index and row/column; `picked` from 1, 0 none). Links are
  taken as the click branch of `story_input_poll` takes them
  (`0x470cc0`, then `0x49d740` on the link list); with no `--choices`
  the first link of each page is taken once. The hooks and records:
  `xtrace.c`'s head comment. Xvfb picks its own display (`-displayfd`).
- **Branch coverage** (`docs/new-game.md` step 24): each xtrace run
  also arms one-shot breakpoints on the code addresses of `names.tsv`
  and of the findings table (`research/xuse/cover.py addrs`: 599 of 610
  are instruction starts) and logs each first hit (`"t":"hit"`).
  `python3 -I research/xuse/cover.py mark t1.jsonl [t2.jsonl ...]
  --write` fills `saihate-no-ima.findings.tsv`'s last column (`runs:`
  the addresses hit, `not:` those armed and never hit, or `never runs`;
  by opcode when a row cites no address).
- **Trace diff** (`docs/new-game.md` step 23), against a dry run with
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

  (`out/saiima-cons`: any full build, for its archives; the dry run's
  `ons_args` and `screens/` go over its own.)
- Dead ends: `winedbg` can read memory but detaching crashed the game;
  `/proc/<pid>/mem` is blocked by `ptrace_scope`.

**Saves:** format and the tool that writes them in "Save format" below.
To reach a moment, write a save with `research/tools/xuse_save.py` and
load it from the title's load (670, 262): slot rows at y 90, 165, 255,
then load (652, 544) and はい (370, 320).

**Measured screens:**
- **CG gallery:** tabs at x 624, y 160 + 33·i (selected tab = the
  pointer cell); thumbnails 5 × 4 from (44, 142), pitch 108 × 92; "PAGE"
  (31, 529), page numbers from (67, 521) every 24 px. The viewer's panel
  at (451, 545) slides down to its name row after a while (ours stays
  up); ◁ ▷ step through variants.
- **Scene replay:** `SceneList.004` (band #0 subtracts ~80 per channel,
  selector #3 added); rows at (16 + 258c, 58 + 21r), titles at (21, 63)
  size ~14, H scenes #FEC7C7; locked scenes are left out. Right-click in
  a replay is a Windows menu with "シーン選択に戻る"; the bar drops
  save/load. The exe's replay table is 16 × (used, ptr) at `0x4fe200`.
- **WSS** (the system-voice room, dropped): `VoiceMode.cpp`
  (`0x45bc00`-`0x45de40`), pictures `VoiceMode.004` (#0-13 the 14 events,
  2 × 4 cells of 19 px; #14-17 play / stop / save / set, 4 cells of
  25 px), clips `HD/wss.bin` (182 clips: event + 14 × bank).
  `0x4fb7a8` maps the character id to the bank (ids 1-4 and 13-21 →
  0-12); tabs: あずさ #0, 沙也加 #1, 葉子 #4, 笛子 #3, then
  SubModeCommon #32-40, 7 per page with NEXT #41 / PREV #42. Layout:
  header Title.004 #13 at (627, 134); tabs (624, 160 + 28 i); heading at
  (96, 97); events (114, 149 + 21 i); buttons (316 + 63 i, 466); exit
  (624, 470). Buttons are enabled by progress bit 4 + `0x4fbc00`[id].

## Save format

Folder `Documents\Xuse\最果てのイマ COMPLETE\` (`0x40b130`): the
global save `Farthest2015_000.bin`, `Farthest2015_sys.bin` (SYSM,
settings), slot `n` (1-50, 10 pages of 5) in `Farthest2015_NNN.bin`,
and `Farthest2015.log` (records `DATA`, a SYSTEMTIME, then the source
file, line and message as u32 length + text XORed with `0x53`: the
game's own error log, worth reading when a load fails). No compression
or encryption; every chunk ends in a CRC-16/XMODEM (poly `0x1021`, init
0, `crc16_ccitt` `0x4a7be0`), stored big-endian, and the loader refuses
a chunk whose CRC is wrong. Integers are little-endian.

Tool: `python3 -I research/tools/xuse_save.py dump <save>` prints a save
(slot or global) as JSON; `write <template> <json> -o <out> [--cd
<install>/HD/CDFiles]` applies an edited dump to a save the game wrote
and recomputes the CRCs (an empty `{}` gives the template back byte for
byte). A slot's position is set with `"scene": {"label": "s9_l3"}` (or
`script` + `index`), its variables with `"vars": {...}`; the screen
state and thumbnails stay the template's, and the script redraws the
screen at a scene's entry label. Checked: a slot made from a save at the
prologue's third page, moved to `s9_l3` with `sentaku_now` etc. set,
loads from the title into 「本鈴が鳴ると、生徒たちは一斉に席につく。」 in
the classroom, chapter bar 沙也加のいる平日, with no error in the log.

**Slot save** (writer `0x461790`, reader `0x461b90`; `LocalSave.cpp`).
Seen = read in saves the game wrote and changed by the tool with the
expected result; inferred = from the code or from values; unknown = kept
as is.

| Chunk | Layout | What it is |
|---|---|---|
| `LOCL` | tag, FILETIME saved, FILETIME (a later time, unknown), i32 version (1; the loader refuses others, `0x466bf0`), i32 0, 0x24 chapter name, 0x40 comment, crc over tag + fields; then two pictures, each u32 size + PNG: 100×75 and 400×300 | The slot list's line and thumbnails (seen) |
| `sRIO` | tag, 0x78 bytes (zero), 1 byte (0), 0x80 chapter title, i32 × 2 chapter start, i32 open chapter, crc over tag + fields | Live at `0x51eb78` + 0x80 (inferred). The open chapter is `0x51ecfc`: a chapter start (`0x36`, `op_36`) while one is open stops the game with 「チャプタが終了する前に開始点に遭遇しました」 (seen), so a save moved to a scene's entry label sets it to -1 |
| `SVAR` | tag, u32 count, count × (u32 length, name XORed with `0x65`, i32 value), crc over the entries only | The 21 progression variables by name (seen; table in "Progression variables"). Writer `0x471480` |
| `LSNL` | tag, u32 1, u32 0, crc over tag + both; then `LSNp` and the scene | The scene list: one scene in every save seen |
| `LSNP` | tag, 0x24 scene name, i32 × 4 (0, 0, -1, 0), 1 byte, i32 script id, i32 code byte offset (instruction × 12), 11 × i32, 0x172 bytes, i32, 20 × u32 call-stack scripts, 20 × u32 call-stack return offsets, i32 call depth, crc over tag + fields (`0x460a90`) | **The position** (seen): the message instruction on screen; the load runs it again. The call stack is inferred (depth 0 in the saves seen, stale entries left in the arrays) |
| `HyLk` | tag, 0x41 bytes, 6 × i32, 0x10 bytes, crc (`0x461160`) | The page's link words (inferred from the tag) |
| rest of `LSNp` | `0x412360`, `0x47cfb0`, `0x4874f0`, `0x4a7d20` and one `0x47cfb0` per layer | Screen state the load rebuilds: background, sprites, sound (unknown, kept from the template) |
| `CPHS` | tag, i32 count, i32, crc; then count × (`sRIO`, `SVAR`, `LSNL`…) | Chapter history (the chapter-history screen's jump-back points), written by `0x4654e0` (inferred) |

**Global save** `Farthest2015_000.bin` (`GlobalSave.cpp`): `GLBL`, then
chunks of tag, u32 size, data, crc over tag + size + data: CG ×2 (68
bytes: the seen bits at `0x51ea18`, then `0x51ea5c`), RMSN ×2 (76:
replay bits `0x51eaa0`, the second set = scenes replayed, no "new") +
RMSN (4), SWTC ×2 (unknown); then an `SVAR` as in a slot (the four
global variables); then `OTHR` (two 7-byte strings, the progress word
`0x51eb3c` at OTHR+18, a FILETIME, a u32 = 1) with one CRC over OTHR to
the end. The dump shows each chunk as hex, the variables and the
progress word, and `write` takes them back (setting CG or replay bits
there unlocks the extras; seen).

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-12 12:43 | - |
| First dry run | 2026-10-12 14:06 (end to end) | - |
| Story plays | 2026-10-12 14:06 | - |
| Title, save, settings | 2026-10-12 16:54-17:58 | - |
| Extras | 2026-10-13 17:19-18:44 | - |
| Progression verified | - | - |
| Finished | 2026-10-14 (reopened the same day: fixes, opcode audit) | - |

## Status: reopened (2026-10-14): fixes, then an opcode audit

Finished 2026-10-14, reopened the same day for fixes from the debug
trace and an opcode audit; both are done (History). Everything below is
optional leftover; nothing in it was asked for.

**What works.** Played from start to end: text, voices, sprites with
filters, backgrounds, BGM with loop points, sound effects, movies, every
transition kind, screen filters and moving effects, text modes, ruby,
links, chapter titles, the launch warning, the system screens drawn by
the engine (title, save/load, settings, backlog, button bars, yes/no)
and the extras (CG gallery, scene replay, BGM room). All 82 opcodes the
corpus uses are converted and agree with the exe; 84,710 of 84,713
calls convert, 3 are left out on purpose (the debug menu: `0x30`,
`0x62` and its message, `LEFT_OUT` in `xuse/build.rs`). Dropped by the
user: WSS (system-voice room), right-click menus, native Linux/macOS
movie playback.

The scene replay's lists, the CG gallery's slots and the two staff
rolls are read from `Farthest2015.exe` at build time (`xuse/exe.rs`;
addresses in `xuse/replay.rs`, `gallery.rs`, `staffroll.rs`), so the
build needs the exe; a missing or different exe stops it with an error.

**Open**
- Launch warning's visitor counter ("あなたは N 人目のお客様です"): not
  drawn. User (2026-10-18): not worth more than a one-line fix, so left
  out; likewise ENTER only (any click goes on). Data: the system data's
  u32 at `0x51eb50`, `digit.png` digits at (492, 173) (`0x488f30`).
- Title intro (History, "Title intro"): its ripple takes one direction
  every time (the exe picks one of 256 at random); closing the extras
  fades the title in over 1 s (exe: the words fade in 200 ms apart); after
  the extras' sub-modes were opened, the exe plays no title music on
  coming back from the story (ours does). Not compared with a frame of
  the original.
- Waited transitions take about 3 frames (90 ms) longer in the exe than
  their time (`0x76`+`0x71` 1000 ms = 33 frames; ours 1,009 ms). Read
  the transition task's code before adding a lag to every `print`.
- At a scene's start the exe spends a frame on each `0x43` (arg 1), the
  `0x76` arm and the `0x22` call (5 frames before `0004.cd` #8's
  background); 2 frames after an unskippable click wait `0x68` in auto
  mode. Ours none.
- Branch coverage of the original (`docs/new-game.md` step 24,
  `research/xuse/cover.py`): one FIRST START run hit 173 of 598 armed
  addresses (47 opcodes); marks are in the findings table's last
  column. Later runs (`--save` or `--choices`) add to them
  (`cover.py mark t1.jsonl t2.jsonl --write`).
- The trace DLL's `--save` start (title load, slot 1) is untested: it
  needs a slot save written by the game as `xuse_save.py`'s template.
- Findings table (`docs/games/saihate-no-ima.findings.tsv`): the volume
  curve of `sound_request` (`0x47aab0`) is unknown.
- Hide button and right-click hide the link words, bars and name plate
  too (History, "Hiding the window").
- Screen positions and the blog bar's level number from the exe's layout
  tables (History, "Screen positions from the exe").
- Settings not converted (six pages in the original; History item 26):
  antialiasing, the CG viewer's voices, the system voice, wheel
  sensitivity, frame rate, inactive-window behaviour, shortcut menu,
  gamepad. Autosave slots (original: right-click menu, dropped).
  Settings' yes/no questions still use `*conv_confirm`. The テキスト
  page's four skip/auto boxes (a choice or a click ends skip or auto)
  are on our 4th tab, スキップ・オート (2026-10-18); a link page counts
  as a choice.
- Hand-typed guesses a frame of the original would settle (History,
  "Value audit"): the save menu's thumbnail position, "New" mark and
  slot text; the yes/no message's y; text styles 3/4 rows (23 or 24)
  and the name plate's font (20); the CG gallery's header, headings and
  page marker; the top bars' row colours and font sizes; the quake's
  hold and shrink; the CG viewer's panel x (exe 461, ours 451 meets the art).
- Layout left open by the exe-table check: backlog (exe: one 720×20
  line per entry at x 52, y 49 + 35·i, 15 shown, `0x41ffa0`; ours wraps
  from (30, 44) at 24 px); top bars' text origins (`0x496fe0`: scene
  name (531, 6), ours 524, rows +24, ours +19; `0x49b1a0`: chapter name
  (73, 6), ours (72, 7)); the CG viewer's 21×21 button (image 9 at
  (566, 547), table `0x4fe3c0`) is not placed. Each needs a frame of
  the original.
- MeltDown's stretch for subs 46/47 is scaled from the exe (filmed for
  item 25, not yet compared).
- Media: `arc.nsa` is about 1.2 GB; if the web build can't handle it,
  split the voices into `arc1.nsa`. Check that saves, thumbnails
  (`savethumbN.bmp`) and `kidoku.dat` persist in the WebAssembly build.

**Next steps:** none asked for; take an open item only if the user
wants it.

## History

Done items: [docs/history/saihate-no-ima.md](../history/saihate-no-ima.md).
