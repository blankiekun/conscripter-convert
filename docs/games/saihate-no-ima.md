# 最果てのイマ COMPLETE (Xuse)

The first game converted, and the reference for the Xuse engine. This
is the game's committed record: file formats, opcodes, how its features
map to CONscripter, the progression variables, and what is left. The
general process is in `docs/new-game.md`, general lessons in `PLAN.md`
§13. Paths below are relative to the repository unless they say
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

Full build (2-3 min):

```sh
target/release/conscripter-convert convert-game <install> \
  -o out/saiima-cons --font <a Japanese TTF, e.g. ipag.ttf> --title "Saihate no Ima COMPLETE"
```

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
| Not converted | The system-voice room (WSS), the chapter history screen, autosave, some settings of the original's settings dialog |

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

**Variables:** bit 15 set = global. Globals map to `%200+n` with
`globalon`, locals to `%10+n`. The game uses 3 locals and 19 globals.
`0x01`–`0x0e` are the arithmetic (`=`, `+=`, `*=`, `/=`, `&=`, `|=`
with a constant or a variable; `0x03` is `-= var`, an engine quirk).
`0x16`-`0x1b` are `if v[a] <cmp> b goto` (see the table). `0x22`/`0x23`
are gosub/return (engine stack depth 20).

**Messages** (`0x40`, `0x41` = voiced, one handler): record `id, size,
line number, segment count`, then that many XOR-`0x53` strings:
pre-wrapped display lines, often with the speaker name first, no inline
markup. A second block follows (`u32 4, u32 0` in every message). Bit 0
of `sub` means "has voice"; the display code (`0x475f40`) checks the
voice through `0x4679d0` and drops the flag if the check fails. All
46,578 messages (15,620 voiced) decode.

**Voices:** clip index = (id − 1) % 100000; `id / 100000` is the
character (3 あずさ … 13 ジーン, 0 minor women, 1 minor men). The rule
hid behind `imul 0x14f8b589; sar 13` (÷ 100000). An earlier statistical
alignment (`research/xuse/voice_align.py`) only matched 87 %.

**Hyperlinks** (`0x46`, 214 uses): the route choices and the
encyclopedia. One record right before the message it applies to, per
linked word: `row, column, length` in characters (1-based, from where
the next message starts), style `0x8000 | 7` (205) or `| 18` (9), and a
label-table index. Targets are full scenes (link scenes, label names
`blog?<title>`) that can change the whole screen; `0x47` returns from
one and `0x48` returns to the root scene. Converted as buttons over the
linked words that `gosub` the scene inside `pushscreen`/`popscreen`.
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
| `0x66`, `0x72`, `0x74` | 205, 217, 11 | low 16 bits of arg = ms (the high half a flag, meaning unknown) | timed wait; a click ends it |
| `0x77`, `0x71` | 234, 3,908 | `0x77`: u32 mask (`mask%04u.png` in `HD/mask.arc`), i32 softness (sign = direction), then `0x76`'s timing block | `0x77` arms a mask wipe (`Trans_MaskBlend2`), `0x76` (below) any other transition; `0x71` waits for it, revealing the changes made since |
| `0x8f` | 414 | arg -1 | remove every bust-up |
| `0x43` | 578 | arg 0 waits for the fade, 1 doesn't (handler `0x4772a0`) | hides the text window, before most transitions: `textoff` |
| `0x52`, `0x53` | 1,117 / 501 | | text tags "brn" (continue below after a click) and "p" (new page), only in full-screen modes |
| `0x4c`, `0x4f`, `0x50` | 9 each | mode, style (`TextWnd{style}.004`), position | text window preset parts, only in script 2's preset subroutines |
| `0x47` | 160 | | return from a link scene |
| `0x62`, `0x30` | 1 each | | not converted: only in the unreachable debug menu |
| `0x8e` | 10,271 | **solved**: `sub` × 44-byte entries of `mode, character, sprite number, depth, x, y` (rest mostly uninitialised memory). The depth (10-150, and 1100-5000 in 62 entries) is the bust-up's drawing order, not a fade time: bust-ups fade over a fixed 500 ms. The sprite number indexes `bu*.004` (128 per file, 0-based), and the file names' character codes match the `character` field | show `sub` character sprites |
| `0x88` | 45 | **solved** (handler `0x415b50`, `AdvObjectScript_Filter.cpp`) | define one module of an effect preset: u32 preset, u32 slot (0 = clear), u32 module field, u32 kind (module type = kind + 0x63; creator table at `0x501434`: 1 Smooth, 2 Negative, 3 HueSatura, 4 Brightness, 5 GrayScale, 6 Monotone, 8 Color, 9 BrightnessInvert), u16 blend (index into Alpha/Add/Multi/Screen/SignedAdd/Copy/dissolve; the game uses Multi and Copy), u16 opacity (256 = full), then kind data (colour: R, G, B; brightness: i16). Field names from the module's debug output (`Opacity=%u, Blend=%s, Brightness=%d`). Script 2's `s2_l6`–`s2_l16` set preset 1 to one tint per lighting mood |
| `0x90` | 651 | **solved** (handler `0x434960`) | attach a preset to a bust-up: u16 character, u32 module field, u32 preset (0 = remove). Module type 1 = `CModule_Preset`. Converted to CONscripter's `spfilter`, reading the preset from variables (`PLAN.md` §4.2) |
| `0x54` | 2,195 | **solved**: `arg` = background number, 1-based (0 = none, skipped by the handler); `bg*.004` holds 64 per file (`n >> 6` in the exe's loader at `0x40bff0`). The most common is 1 = `Black` | set background (sub 0/1: transition variant?) |
| `0x46` | 214 | **solved** (from the handler at `0x477460` plus a user-supplied example) | **hyperlink** on the next message: row, column, length, style, target (see 3c) |
| `0x49` | 565 | **solved** (handler `0x477990`): u16 line, u16 column, u16 length (30000 = to the end of the message; the game wraps enlarged messages short enough to fit), u16 size (percent below 0x1000; 0x1000 + n = n × 25 percent from the table at `0x4fa850`, 0x1003 for mumbling, 0x1005-0x1007 for shouting; 0xf000 = keep), u32 colour (COLORREF, 0xff000000 = keep), u32 edge colour; lines and columns count as for ruby; later records override earlier ones | size and colour for characters of the next message: `#RRGGBB` and CONscripter's `!zNNN` size tag (`PLAN.md` §4.2) |
| `0x4e` | 24 | **solved** (handler `0x494750`): arg = indent << 16 \| 1 (once 2); the indent is in half-width units of the first line (ASCII, half-width kana and Greek count 1), up to the text after the chat log's handle | hanging indent of the next message; the game puts it only before chat messages long enough to wrap. Converted to `indent N` around the message, N measured on the emitted line |
| `0x70` | 457 | **solved** (handler `0x415a30`, objects `filter-<slot>`): u32 slot (0 = clear all), u32 layer \| depth << 8 (layer byte through the table at `0x503e68`: 0/2 → 0x10, 1 → 0, 3 → 0x20, 4 → 0x30; depth = the sort depth within that band, mostly 1000, above the bust-ups; no fade of its own), u32 kind (0 = clear the slot), u16 blend, u16 opacity, then 0x88's kind data; monotone (6) is a u32 palette, `pal%04u.pal` in `HD/pal.arc` (RIFF PAL, 256 entries for the grey levels) | filter on the whole screen, from 0x88's modules. Converted to `screenfilter` + `print 1` (or the surrounding transition); Smooth (1, u8 x, y range), HueSatura (3, i16 hue, saturation) and BrightnessInvert (9) as `screenfilter` kinds 6-8; Filter-Replace (13: u32 map, i32 direction, distance, begin angle in 1/100 turn) as a still `screenripple` in slot 4 + n. Kinds from the (type, creator) pairs at `0x501428`. Zigzag (12, one use: i32 centre x, y, direction, distance, frequency, begin angle, reduction; kernel `0x43f0b0`: each pixel from distance × sin(2π·i/1024) along the direction, i = d × frequency/100 for the distance d from the centre) as a still `screenripple` over a generated ring map; its Multi blend is drawn as alpha (the darkening is missing) |
| `0x59` | 47 | **solved** (handler `0x473610`) | BGM crossfade: u32 fade ms (capped at 30000), u16 track to stop (0 = all but the new one), u16 track to play; the new track starts fading in, then the old fades out, at once. Converted to `bgmfadein ms` + `bgm` (NScripter has one BGM: the old track stops at once) |
| `0x82` | 38 | **solved** (handler `0x413f10`) | set an object's opacity (field +0x1b2, 0-256): u8 layer, u32 depth, u16 opacity; the object is found by key = depth (layers 0, 1), depth + 10 (layer 2: character `depth`'s bust-up, as 0x90 finds it), + 110, + 120 (3, 4). Converted for bust-ups (34) and the 0x55 overlay (layer 1 depth 2, 3) as `msp n,0,0,-255:msp n,0,0,alpha`; a new picture is opaque again. Not converted: layer 1 depth 1 (one use after `0x56`) |
| `0x84`, `0x7d`, `0x7c` (+ `0x7a`/`0x7b` on layer 2) | 10, 5, 6 (6, 8) | **solved** (handlers `0x414180`, `0x413940`, `0x414460`; objects found as in `0x82`) | `0x84` loads a picture into an object: u8 layer, u32 depth, u8 kind, u32 number, u8 ?; image id kind × 10,000,000 + number - 1 (kind 2 = bust-ups, numbered as in `0x8e`). `0x7d`: u8 layer, u32 depth, u8 horizontal, u8 vertical anchor (0-2: left/top, centre, right/bottom; a new object's is 0). `0x7c`: u8 layer, u32 depth, u8 visible. The blog scenes make a hidden layer 2 object with `0x7a` (depth = a character, band 4 above the bust-ups, at (650, 320) or (250, 375)), load a bust-up, centre it and show it; 0135.cd steps a 0-100 % meter (`bu99_*`) a picture a second. `0x7b` on layer 2 removes it, or a bust-up `0x8e` made (0011-0013.cd). Converted as the character layer's sprite (`Op::Sprites`) |
| `0x99` | 8 | **solved** (handler `0x478b00`) | sets progress bit `sub` of the global word `0x51eb3c` (saved with the system data, `0x45f801`): bits 0-3 at the start (0000.cd, 0003.cd), 4-7 at the end of each heroine's route (0009-0013.cd). Read by the extras: the title (bit 3, `0x45cce9`), getters for bits 1, 2, and the voice mode (`VoiceMode.cpp`, bit 4 + route via `0x4fbc00`). Converted as global flags `%3600 + bit` for the extras to come |
| `0x9d` | 6 | **solved** (handler `0x478b30`) | marks CG `arg` (1-based, as in `0x54`) as seen without showing it: bit arg - 1 of the global set `0x51ea18`, read by `CGMode.cpp` (`0x44cd56`, `0x44d4af`); `0x55`'s "mark as seen" byte sets the same set (`0x42236b`). Used right after a `0x54` of a related CG (467 → 523). Converted as global flags `%2200 + n` (n up to 576; above the link flags, below the 4096 variables) for a CG gallery |
| `0x56`, `0x83`, `0x67` | 3, 1, 2 | **solved** (handlers `0x4224a0` → `0x412fe0` kind 1, `0x414050`, `0x473d20`) | `0x56` fills a background slot with a colour: u8 slot (0 removes slots 1-10), u8 layer, u32 depth, u32 COLORREF; a screen-sized object with fill module 7 (`0x55` is the same with kind 0, a picture). Slot 1 = the background, 2 = the overlay. `0x83` sets an object's blend (u8 layer, u32 depth, u8 0-6: Alpha, Add, Multi, Screen, SignedAdd, Copy, dissolve). `0x67` yields one frame. Uses: black background at boot; the shared impact routine `s2_l2` (9 calls) puts white on the overlay, sets dissolve and fades it out over 1 s (`0x86` opacity motion); 0006.cd fills the background red at opacity 64 (`0x82`). Converted with generated `system/fill_rrggbb.png`: `bg` / the overlay; the fade-out as a 1 s cross-fade removal that the following 1 s wait no longer adds to; the red at 64/256 as `fill_3f0000` (over black) |
| `0x39` | 4 | **solved** (handler `0x475af0`, `ScriptStd.cpp`) | clears the chapter history (an error inside a chapter; nothing in replay modes 1/2): start of 0004.cd, after the ends of 0021/0022.cd. Nothing emitted: no chapter history yet |
| `0x3b`, `0x3c` | 2, 0 | **solved** (handlers `0x478650`, `0x4786c0`) | `0x3b` copies `sub` bytes of the record into the game state at +4 (an entry name: the boot script sets "debug entry" or "release entry"); `0x3c` jumps when the name matches and is never used. Nothing emitted |
| `0x91` | 2 | **solved** (handler `0x4788d0`, task `0x454a60`, `StaffRoll\Staff.cpp`; descriptors via `0x517f30`) | the staff roll: arg 0 → `StaffRoll1.004` (63 s, BGM 23), 1 → `StaffRoll2.004` (223 s, BGM 22, the CG slideshow). A panel of key frames (clip rectangle, two picture layers with dx, dy, opacity) and a column of credits scrolling from `start` to `end`; details in `xuse/staffroll.rs`. Rendered by the converter into an MP4 with its song, played with `mpegplay` (not skippable, like the original). Checked frame by frame against the original under wine |
| `0x48` | 2 | **solved** (handler `0x477900`, `ScriptText.cpp`) | returns to the root scene: leaves every open link scene (`0x467670`); at the root it logs "既にルートシーンにいるので何も行いません。". Routes 10 and 11 (entered from links) start with it and end with jumps. Converted as `Op::ReturnToRoot`: `return` through each link page, whose cleanup puts its screen back and then goes to `*conv_unwind` instead of showing the page again; at depth 0 it jumps to the label after the 0x48 |
| `0x4a` | 2 | **solved** (handler `0x477af0`; values through `0x491d80`/`0x491e30` as for `0x49`) | the style of the following messages: u16 size (0x1000 + n preset, < 0x1000 percent, 0xf000 + k: 0 none, 1 default, 2 kept), u32 COLORREF (0xff000000 + k likewise), u32 edge colour (stored at +0x60 and +0x64). 0011.cd: four one-character pages at 125 % orange, then the defaults. Converted as a whole-message style under the message's own `0x49` styles; edge not converted |
| `0x17`-`0x1b` | 2 | **solved** (handlers `0x474460` setne, `0x4745e0` setl, `0x474520` setg, `0x474580` setle, `0x4744c0` setge) | `if v[a] <cmp> b goto label[sub]` like `0x16` (==), the jump taken when the comparison holds. Converted as `Op::If` |
| `0x96`, `0x00` | 1, 1 | **solved** (handlers `0x478330`, `0x473d20`) | `0x96` stops the sounds and starts the title task (`0x45b6f0`, mode 0), at the end of the epilogue: converted as `reset`. `0x00` shares 0x67's handler: a one-frame yield, nothing emitted |
| `0x8d` | 1 | **solved** (handler `0x4155c0`, wait task `0x4154b0`) | waits until environment slot n's module (object key n + 0x78) has finished: u32 slot, u8 0xff. 0007.cd waits out a 3 s ripple, then clears it. Converted as a `delay` of the module's time (exact there: it follows the 0x8c that set the slot) |
| `0x80` | 1 | **solved** (handler `0x413cd0`; fields +0x1a8/+0x1ac, 100 for a new object; key-frame motions set them with flag 0x80) | an object's scale: u8 layer, u32 depth, i32 x, y percent. 0013.cd: character 12 at 80 % after 0x82 made it translucent. Converted as a pre-scaled copy of the bust-up centred on the same point, then the opacity again |
| `0x69` | 32 | **solved** (handler `0x473940`, wait callback `0x473810`; checked under wine with a patched `0002.cd`) | no record: a click wait showing the picture alone (text window and bars hidden, the wait icon in the corner), passed by skip mode. Converted like 0x68 with `skippable` |
| `0x86`, `0x87`, `0x7f` | 19, 9, 12 | **solved** (handlers `0x413540`/`0x413350`, `0x413820`, `0x413bb0`; objects found as in `0x82`) | `0x86` adds a module to an object (u8 layer, u32 depth, u32 slot, u32 kind; kind `0x80000001` removes the slot's module) — always a key-frame motion (`CModule_Animation_KeyFrameMotion`, setup `0x416fe0`): the kind is its flags, which fields each key frame has after its u32 ms (0x10 u8; 0x20/0x40/0x80 i32, i32, u8 with 0x40 the position; 0x100 i32 rotation, u8; 0x200 u16 opacity, u8), then a u16 key frame count; the first frame is replaced by the object's state; each u8 is the field's interpolation (`0x4176f0`: 0 kept, 1 a jump halfway, 2 linear, 3 a curve). `0x87` waits for it (u8 layer, u32 depth, u32 slot, u8 `0xff`); `0x7f` puts the object's centre at i32 x, y. The game slides bust-ups (9: start, wait, remove, `0x7f` to the end point; all inside an armed transition, so the original fades and slides at once) and fades the overlay out once (`0x200`, `0002.cd` #14). Converted as `Op::SpriteMove`: `*conv_move` (`getsppos`/`getspsize`, `amsp` steps on the timer); the transition is shown first, then the slide. The rotation (field +0x1b0, tenths of a degree mod 3600, clockwise; `0x81` sets it: u8 layer, u32 depth, i32 angle) of -900 in `0043.cd` #440 is drawn by `Op::SpriteTurn` (lsp2/amsp2, the sprite ends off screen). Key-frame group 0x80 is the scale (+0x1a8/+0x1ac) |
| `0x8c` | 240 | **solved** (handler `0x4153c0`, `AdvObjectScript_Env.cpp`, objects `environment-<slot>`): u32 slot (0 = clear all), u32 layer \| depth << 8 (the word goes to `0x4109a0` as the object's sort key, see `0x4126c0`; the game uses depth 1000), u32 kind (0 = clear the slot), then the kind's module data (setup table `0x5013bc`; module class from the creator table `0x501434` + RTTI): 1 Animation-Simple on `HD/env.arc` image 50,000,000 + n (light rays, unused), 2 Animation-RandomNoise (88 uses), 3 Filter-LensFlare (unused), 4 Animation-Replace (25), 5 Animation-Vibrate (12). Layouts and maths in `cd.rs` (`Noise`, `Ripple`, `Shake`) | static: an animated sprite (4 generated frames, `system/noise.png`) at the end opacity, sprite 60, kept across `csp -1` like `0x55`; the opacity rise isn't converted. Ripple (draw `0x41a2d0`, per pixel `0x43d9d0`: each pixel from amp × sin(2π(grey/256 + phase/1000)) pixels away along the angle, map `mask%04u.png`, blended at the opacity) → `screenripple`; shaking (draw `0x41ac70`: the picture blitted onto itself at a sine or 7-point-path offset) → `screenshake` |
| `0x6d` | 198 | **solved** (handler `0x442000`, object `0x441ee0` at sort key `0x06000000`, the bottom of the text window's band): u32 speed, u32 x and y range in pixels (mostly 80, 20, 20). The picture moves along 0x8c's 7-point path, `speed` points a second; after 4 points both ranges shrink by max(speed / 4, 50) px/s until 0. The script waits; a click ends it. Skipped while skipping or with effects off | a quake, often right after a sound effect. Converted to `screenshake 4,…` (mode 1, ranges shrinking from the start over the same total time) + `delay` + `screenshake 4` |
| `0x6e` | 88 | **solved** (handler `0x441550`, object `0x4413d0` at sort key `0xffffffff`, over everything): `arg` = COLORREF filled over the screen with blend 1 (Add) at opacity 128 for 100 ms; the script waits, a click ends it. White for lightning and shocks, red (`0x0000ff`) for blows | a flash. Converted to `lsp2add` of a full-screen colour rectangle (`>800,600,#rrggbb`, alpha 127) for a 100 ms `delay`. Difference: sprite2 is under the text window, the original flash also covers it |
| `0x6f` | 87 | **solved** (handler `0x441a00`, `Surprise.cpp`, object at sort key `0x06000000`): u32 time (1-32767 ms), u32 preset 0-8 from the table at `0x500b68` (zoom x/y 16.16 = 10/15/20 %, opacity 100/150/200, tint colour/opacity/blend: none for 0-2, light red `0x8080ff` Multi for 3-5, white blend 6 for 6-8). Takes a snapshot of the picture, tints it, and lays it over the picture scaled by 1 + f × zoom about the centre at f × opacity, f = sin(π t) over the time; the script waits | a "surprise" pulse. Converted to CONscripter's `screenpulse 3,…` (zoom 100/150/200 ‰; Multi → `tint` 1, blend 6 assumed to lay the white over the copy → `tint` 2) + `delay` + `screenpulse 3`. Not checked against the original |
| `0x57` | 542 | **solved** (handler `0x473530` → sound request `0x47aab0`, channel 0) | play BGM: `arg` = track (1-based, `bm0000.bin` slot `arg - 1`), `sub` = fade-in ms (capped at 30 s) |
| `0x58` | 579 | **solved** (channel 0, stop flag) | stop BGM: `arg` 0 = all, `sub` = fade-out ms |
| `0x5a` | 2,633 | **solved** (channel 1) | play SE: packed 9-byte record `u32 ?, u8 loop, u32 sound` (1-based across `k0000.bin` + `k0001.bin`) |
| `0x5b` | 890 | **solved** (channel 1, stop flag) | stop SE: `arg` = sound or 0 for all, `sub` = fade ms |
| `0x76` | 3,670 | **solved** (handler `0x449df0`): `sub` picks the `Trans_*` class through a jump table (0-3 Wave, 4 Dot4x4, 5-8 SoftEdgeWipe, 9 CrossFade, 10-17 Slide in, 18-25 Slide out, 26 Dissolve, 27 Mosaic, 28-31 BoxRolling, 32-35 RollFlip, 36 SmoothCrossFade, 37-40 Zoom, 41-44 Scroll, 45-47 MeltDown, 48 TvPowerSwitch); the record is duration, flags, easing | screen transition |

`0x76`'s `sub` → transition kind is in `transition_style`
(`crates/conv-engines/src/xuse/convert.rs`); how each kind is drawn is in
`PLAN.md` §4.2 (the `cons_*.dll` plugins).

## How the game maps to CONscripter

The target is the WebAssembly build first, so every system screen is a
real graphical menu built in the script from the game's own parts
(`spbtn`/`btnwait`/`getmousepos`), and assets are packed into `.nsa`
archives.

- **Movies:** opcode `0x97` plays `HD/mvNNNN.bin` (MPEG-1 program stream);
  arg's low 16 bits are the number, the high 16 bits whether a click or
  key ends it (handler 0x478ab0, checked at 0x408e5b). The boot script
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
- **Text box:** ADV box at the measured positions; seven full-screen
  styles (metrics estimated).
- **Backlog:** a full-screen dark window (like `TextWnd5`, 714×472) with
  names and voice replay.
- **Title:** FIRST START / load / option / EXIT over `Entrance.004`.
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
the third no-link loop (`%215 <= 2` → リピート, 0x1a = setle in the
exe's table): the second no-link loop is a リピート identical to the
first (0021 has no conditions). The full-voice note in 愚者の館
(sagaoz.net, "one more no-link pass") matches this. The conversion
follows the bytecode, and `routesim.py` confirms both sides agree.

## Progression variables

Global variables (bit 0x8000 in the bytecode, `%200 + n` in `0.txt`;
the original's table of 20, `0x46de00`). Assumed kept in the system
data, not in saves, as the conversion does (`globalon`); not checked
against the original yet (`docs/new-game.md` step 11.5). Scripts:
0000 `!boot`, 0004 `00_プロローグ` (お茶会－０ and 最初の日), 0009 あずさ,
0010 さやか, 0011/0012 ようこ, 0013 ふえこ, 0021 リピート, 0022 死,
0019 `09_回想初期化` (scene replay setup).

| Var | Meaning | Set | Reset | Read |
|---|---|---|---|---|
| %203 | 1 debug entry, 0 release entry | 0001 | | |
| %204-%207 | constants 0, 1, 2, 1 (%207 = 0 in the scene replay) | 0000 `s0_l2` | | not read |
| %208 | heroine chosen this loop: 1 あずさ, 2 ふえこ, 3 ようこ, 4 さやか | each link scene (`s9_l1`, `s13_l1`, `s11_l1`, `s10_l1`) | each お茶会－０ (`s4_l2`), `s0_l4` | end of 最初の日: 1 → あずさ (`s9_l3`), else ふえこ (`s13_l2`); さやか and ようこ jump to their routes from the link scene (0x48) |
| %209 | scratch: all four heroines done (`%210 & %211 & %212 & %213`) | `s0_l6` | | `s0_l5`, `s0_l8` |
| %210, %211, %212, %213 | あずさ, ふえこ, ようこ, さやか done this cycle | their link scenes | `s0_l4` (start, and at each cycle's end) | 最初の日 hides the heroine's link (jumps past the 0x46) |
| %214 | a heroine link was clicked this loop | link scenes | each お茶会－０ | 最初の日 hides all heroine links; its end picks a route or no route |
| %215 | loops with no link | end of 最初の日 (+1) | `s0_l2` | `<= 2` → リピート (0021), else 死 (0022) |
| %216 | phase: 1 cycle 1, 2 cycle 2, 3 after it | `s0_l2`, `s0_l5`, `s0_l8`, end of the epilogue | | not read |
| %217 | 1 in cycle 1 | `s0_l2` | `s0_l5` (all four done) | route text variants (23 places) |
| %218 | 1 in cycle 2 | `s0_l5` (all four done) | `s0_l8` | route text variants (40 places) |
| %219 | 1 after cycle 2: the war arc | `s0_l8` (all four done again) | end of the epilogue | route ends: → `s5_l1` (00_共通前, お茶会－１/２) |
| %220 | routes finished | `s0_l5`, `s0_l8` (+1) | | not read |

A route's end calls `s0_l5` in cycle 1 or `s0_l8` in cycle 2, then goes
back to お茶会－０ (`s4_l1`). FIRST START runs `s0_l2`, which zeroes all
of this. Progress bits for the extras (0x99) are `%3600 + bit`.

Checked by `research/xuse/routesim.py` (route simulation of the bytecode
and of `0.txt`, `docs/new-game.md` step 11.4).

## Testing

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
  link words 51-58 stay in, since they are the route choices. Sprite 61
  `bg\frame01.png` is the game's frame overlay, a known `linger` false
  positive (put it in the game's checker config). A single run with
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
- **Sprite numbers:** 0-5 yes/no dialog, 6-33 menus, 34-48 bars, 49
  name text, 50 plate, 51-58 link words (`humanz 58`), 59 full-screen
  animation, 60 static, 61 overlay, 62+ characters; `lsp2` 0-11 save
  menu, 12-23 top bars. Save slots 900 + level hold the blog bar's
  levels.
- **Disassembly:** `objdump -d -M intel --no-show-raw-insn
  Farthest2015.exe > research/xuse/local/exe.asm` (git-ignored). The
  opcode dispatch table is at `0x4fb270`.
- After a rebuild delete `kidoku.dat`: its offsets go stale.

## Running the original under wine

Used to film transitions, effects and screens (`docs/new-game.md`
step 9). Helper scripts of the sessions are in `research/xuse/local/`
(git-ignored).

- **Prefix:** a scratch `WINEPREFIX`, never `~/.wine`. In a default
  prefix `drive_c/users/<user>/Documents` (and Desktop, Downloads,
  Music, Pictures, Videos) are symlinks to the real home folders, and
  the game saves to `Documents\Xuse\最果てのイマ COMPLETE\`: replace the
  six symlinks with real folders. `WINEDLLOVERRIDES=
  "winepulse.drv,winealsa.drv,winemenubuilder.exe=d"` keeps it silent.
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
- Dead ends: `winedbg` can read memory but detaching crashed the game;
  `/proc/<pid>/mem` is blocked by `ptrace_scope`.

**Save files:** `Farthest2015_000.bin` (global), `Farthest2015_sys.bin`
(SYSM), `Farthest2015.log`. The global save is "GLBL", then chunks of
tag(4), size(u32), data, crc(u16 big-endian), the CRC being
CRC-16/XMODEM over tag + size + data: CG ×2 (68 bytes: the seen bits at
`0x51ea18`, then `0x51ea5c`), RMSN ×2 (76: replay bits `0x51eaa0`, the
second set = scenes replayed, no "new") + RMSN (4), SWTC ×2, SVAR, then
OTHR (two 7-byte strings, the progress word `0x51eb3c` at OTHR+18, a
FILETIME, a u32 = 1) with one CRC over OTHR to the end. Patching bits in
a copy of the save works.

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

## Status: finished (2026-10-14)

Converted and playable from start to end: text, voices, sprites with
colour filters, backgrounds, BGM with loop points, sound effects, movies,
every transition kind (Wave, Dot4x4, SoftEdgeWipe, Dissolve, BoxRolling,
RollFlip, Zoom, Slide, Scroll, Mosaic, TvPowerSwitch and MeltDown each
drawn their own way, most filmed in the original under wine), screen
filters and moving effects, text modes, ruby and text sizes, clickable
links, chapter titles, the system screens (title, save/load, settings
with voice switches and read-only skip, backlog with names and voice
replay, button bars, yes/no confirmations) and the extras (CG gallery,
scene replay, BGM room). Every story instruction is converted; the two
left as comments are the start point's debug menu, which the user didn't
want. Dropped by the user: WSS (the system-voice room), right-click
menus, native Linux/macOS movie playback.

**Found by the debug trace (2026-10-14).** A full exploration with the
engine's debug trace (`cons_explore.py`, 62,185 pages, 1,220 labels;
`docs/new-game.md`, "Checking a build with the debug trace") found these:
- **A text line that stopped the game (fixed).** `0.txt:24036`
  (`*conv_link19`), 「嘘ー！?  投稿者: イマたん（管理人）…」: the
  half-width `?` after 「！」 was read as an array reference, and the
  engine stopped with "Array No. is not declared" (exit 2). Text lines now
  get every character the engine's text parser reads specially (`%`
  `?` `$` variables, `@` `\` `_` waits, `!` `#` `(` `/` `)` `<` `>` `[`
  `]` tags, `` ` `` `"` `;` `*`) in full-width form (`escape_text` in
  `crates/conv-cons/src/lib.rs`, tested by `text_is_made_safe`); a run
  from `--goto conv_link19` goes on past it.
- **`%190` after a load (meant).** The loadgosub `*conv_scene_loaded`
  runs `if %2900 == 0 mov %151,%150:mov %190,%150:return`, so every load
  from the save menu sets `%190` to `%150`, while in play `%190` stays 0.
  `%190` is not one of the game's variables (they are `%200` and up, see
  "Progression variables") but the converter's blog-bar floor
  (`SCENE_FLOOR` in `crates/conv-cons/src/menu.rs`): back, exit and the
  level list only reach levels at or below it. Each level's page lives in
  save slot 900 + level, which belongs to the current playthrough, so
  after a load the levels above the loaded one can't be gone back to (they
  would load another playthrough's pages) and the deeper ones are dropped
  (`%151`). A switch between levels (`%2900` = 1) carries the floor over
  instead. The original keeps the whole stack in its saves (the exe has
  `SaveControl\LocalSaveStack.cpp` and `CLocalSceneStack`), so a load
  there can still go back: a known difference, limited to a save made
  inside a link scene. Matching it needs a copy of slots 900+ for each
  save slot, which NScripter commands can't make (an engine command that
  copies a save file would).
- **Pictures "too large for the web build" (not a problem).** The
  `fx\fx0*.png` animation strips (from `HD/fx.arc`, sprite 59: the
  frames side by side in playing order) are up to 37600×600 (`fx0102b`,
  47 frames; about 90 MB decoded, 24000×600 about 55 MB), and the trace
  warned about every side over 4096 px. They work in the web build: in
  headless Chromium it loads, draws and animates a 24000×600 strip as the
  native build does, since pictures are decoded inside the wasm and drawn
  in software (only the screen becomes a texture). The warning was the
  engine's mistake, fixed in CONscripter: the trace now reports a single
  picture over `--trace-max-image` MB (64 by default) as a "large image"
  under the checker's `memory` rule.
- **Long effects (fixed).** 55 transitions asked for 15 s (`print` with
  15000 ms; `s10_l51` and others), 23 for 11 s and one for 50 s. All were
  bust-up changes (`0x8e`): the converter read the entry's u16 at +6 as a
  fade time in 10 ms units. It is the bust-up's depth: handler 0x434130
  passes it to the object it creates (0x4109a0) in the place where the
  overlay's handler (0x412fe0) passes its depth (1000). Filmed under wine
  (0004.cd #40, 30 fps): the entry as shipped (50) and patched to 1500
  both fade over 15 frames, 0.5 s, which is the 500 ms the 0x8e handler
  (0x4347b0) starts. Every bust-up change now fades over 500 ms. Not
  converted: the depth itself (bust-ups stack by character number;
  1100-5000 probably put a bust-up over a `frameNN` overlay, which ours
  draws over every bust-up).

Fixed in CONscripter by the same work: half-width katakana in Shift_JIS
text (「影（ｼｬﾄﾞｳshadow ）」 in `s7_l8` drew as a blank and lost the next
character), and saves losing the speaker name (`strsp` 49) and looping
sound effects on `dwave` channels.

What follows is the optional leftover list, kept for whoever comes back
to the game. Nothing in it was asked for.

**Settings screen.** The original's settings dialog has six pages; these
parts are not converted:
- text: auto-mode wait per page and per character,
  stopping skip/auto on a choice or click, font, antialiasing, decoration;
- voice: "次の音声まで停止しない" (keep a voice playing into the next
  line), auto-play of voices in the CG viewer;
- sound: per-channel mute switches, the system voice;
- mouse: wheel sensitivity (the wheel switches and the confirmation
  switches are done);
- other: effect and animation switches, save preview, inactive-window
  behaviour, shortcut-menu position, gamepad.

**Other system screens.**
- Chapter history (`ChapterHistory.004`; 0x37/0x38 only end a chapter and
  clear its name, converted) and the autosave slots.
- Extras: the CG gallery, the scene replay ("chapter") and the BGM room
  are done; the system voices (WSS, title image 13) are dropped (the
  user's decision; the item stays greyed out). No title BGM (the
  original's title code plays none, as far as was checked).
- Button bar lock button (`SysTray.004` #9).

**Script opcodes:** all converted (the converter reports 2 unsupported
instructions, both in the unreachable debug menu). Notes on the last
ones done:
- `0x70`: every kind is converted (Zigzag last, with `screenripple`'s
  blend);
- `0x93`/`0x94` are done: `0x93 N` unlocks scene N of the scene replay
  (global flag `%3000+N`), `0x94` ends a replayed scene (a no-op in play:
  `if %152 == 1 goto *conv_replay_end`);
- `0x7a`/`0x8b`/`0x8a`/`0x7b` are done: full-screen animations from
  `HD/fx.arc` (an `.004` of frames and an `.ap` script each), drawn
  opaque as an animated sprite (sprite 59, a strip of frames in `fx/`)
  that outlasts background changes; `0x8a` waits for a one-shot's last
  frame. Not converted: the rarer layer-2 `0x7a`/`0x7b` records (6 and 7
  uses), which load no animation;
- `0x6d` quakes, `0x6e` flashes and `0x6f` pulses are done. `0x55` is done: a second background object, mostly the
  `frameNN` overlays (letterbox bars, speed lines), drawn as sprite 61
  (over the characters) or 999 (under them) and kept across background
  changes; `0x8c`'s static is sprite 60, over it;

**Accuracy.**
- Transitions: only SmoothCrossFade (8 uses) is still a plain cross-fade
  (the original eases it). BoxRolling is a push where the original
  squeezes the pictures like cube faces; MeltDown's stretch for subs
  46/47 is scaled from the exe, not filmed. Quakes, flashes and pulses
  were never compared with the original (our flashes are under the text
  window; the original's cover it).
- Full-screen text styles 1-7: metrics are estimates; calibrate from
  screenshots of the original (style 5, route 10, is the most used).
- Unchecked assumptions: negative mask softness inverts the mask; text
  window opacity 55%; save menu base opacity; the brightness filter's
  scale (the game uses -80 and -100, read as -255..255).

**Media and packaging.**
- `arc.nsa` is about 1.2 GB; if the web build can't handle it, split the
  voices into `arc1.nsa`.
- Check that saves, thumbnails (`savethumbN.bmp`) and `kidoku.dat`
  persist in the WebAssembly build.
