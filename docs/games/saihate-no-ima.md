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
| `0x90` | 651 | **solved** (handler `0x434960`) | attach a preset to a bust-up: u16 character, u32 module field, u32 preset (0 = remove). Module type 1 = `CModule_Preset`. Converted to CONscripter's `spfilter`, reading the preset from variables (`PLAN.md` §4.2) |
| `0x54` | 2,195 | **solved** (`op_54` `0x422380`): `arg` = background number, 1-based (0 = none); `bg*.004` holds 64 per file (`n >> 6` in the exe's loader at `0x40bff0`). The most common is 1 = `Black`. The handler first removes background slots 1-10 (`0x4132d0`: the background and the `0x55` overlays), puts the picture on slot 1 (layer 1, depth 0, marked seen) and, for `sub` 1-3 (41 uses of 1), picture `sub + 5` (`frame01`-`03`) on slot 2 (layer 1, depth 1: under the bust-ups). It starts no fade: 2,190 uses are inside an armed transition, the other 5 show at once. Before the audit the overlay stayed up across backgrounds (86 places), `sub` was ignored and backgrounds outside a transition faded over a guessed 500 ms | set background: `Op::Overlay(None)`, `bg`, then the frame as the overlay |
| `0x46` | 214 | **solved** (from the handler at `0x477460` plus a user-supplied example) | **hyperlink** on the next message: row, column, length, style, target (see 3c) |
| `0x49` | 565 | **solved** (handler `0x477990`): u16 line, u16 column, u16 length (30000 = to the end of the message; the game wraps enlarged messages short enough to fit), u16 size (percent below 0x1000; 0x1000 + n = n × 25 percent from the table at `0x4fa850`, 0x1003 for mumbling, 0x1005-0x1007 for shouting; 0xf000 = keep), u32 colour (COLORREF, 0xff000000 = keep), u32 edge colour; lines and columns count as for ruby; later records override earlier ones | size and colour for characters of the next message: `#RRGGBB` and CONscripter's `!zNNN` size tag (`PLAN.md` §4.2) |
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
| `0x6d` | 198 | **solved** (handler `0x442000`, object `0x441ee0` at sort key `0x06000000`, the bottom of the text window's band): u32 speed, u32 x and y range in pixels (mostly 80, 20, 20). The picture moves along 0x8c's 7-point path, `speed` points a second; after 4 points both ranges shrink by max(speed / 4, 50) px/s until 0. The script waits; a click ends it. Skipped while skipping or with effects off. Filmed (speed 80, 20 × 20): the picture jumps to the next point every frame (30 a second), at the full range for the first points, then shrinking | a quake, often right after a sound effect. Converted to `screenshake 4,…` (mode 1, at most 30 points a second, the full range for 4 points, then shrinking at the same rate) + `delay`, twice, then `screenshake 4` |
| `0x6e` | 88 | **solved** (handler `0x441550`, object `0x4413d0` at sort key `0xffffffff`, over everything): `arg` = COLORREF filled over the screen with blend 1 (Add) at opacity 128 for 100 ms; the script waits, a click ends it. White for lightning and shocks, red (`0x0000ff`) for blows | a flash. Converted to CONscripter's `screenflash 128,"#rrggbb"` (`Op::ScreenFlash`; the colour added over everything, the text window included, as the original's sort key puts it) for a 100 ms `delay` |
| `0x6f` | 87 | **solved** (handler `0x441a00`, `Surprise.cpp`, object at sort key `0x06000000`): u32 time (1-32767 ms), u32 preset 0-8 from the table at `0x500b68` (zoom x/y 16.16 = 10/15/20 %, opacity 100/150/200, tint colour/opacity/blend: none for 0-2, light red `0x8080ff` Multi for 3-5, white blend 6 for 6-8). Takes a snapshot of the picture, tints it, and lays it over the picture scaled by 1 + f × zoom about the centre at f × opacity, f = sin(π t) over the time; the script waits | a "surprise" pulse. Converted to CONscripter's `screenpulse 3,…` (zoom 100/150/200 ‰; Multi → `tint` 1, blend 6 assumed to lay the white over the copy → `tint` 2) + `delay` + `screenpulse 3`. Not checked against the original |
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
| `0x40` | `-60` | the one block | after the unreachable debug menu's message (`0001.cd` #1), which stays out (History, item 42) |
| `0x44` | `+6` | u16 3 or 1, u32 0 | the ruby's alignment: spread (as `rubyon`) or centred over the base, the same here; the u32 isn't drawn (History, item 33) |
| `0x8c` | `+4` | layer 0, depth 1000, 1100, 2000, 500; 0 when clearing | the effect's sort key: every one sorts over all the bust-ups, as ours draws them (History, item 31) |
| `0x49`, `0x4a` | `-4` | `0xff000000`; `0x4a` also `0xff000001` | no edge colour of the record's own, or the default one: the default decoration (History, item 29) |
| `0x70` | `-8` | 10, 0 | Zigzag's begin angle and reduction, which its draw (`0x41dab0`) doesn't read (History, item 39) |
| `0x56` | `+2` | u32 0, 2000 | a slot-1 fill's sort depth in its band, where it is alone (History, item 40) |
| `0x90` | `+2` | u32 1, 0, 2 | the preset module's place among the bust-up's modules, where it is alone (History, item 35) |
| `0x8e` | the six module names | byte 0 is 0 | an empty module name (leftover memory after it) |

A parsed
field the converter never used counts as not read: `cd.rs` doesn't
parse those (ruby flags, edge colours,
`0x70`'s depth); link styles and
message line numbers are read only for `cd messages`, outside the
tracking.

Per operand or behaviour (counts: instructions; "notes" are the dry
run's):

| Opcode | Operand or behaviour | State |
|---|---|---|
| every opcode the corpus uses, except as below | every operand | converted |
| `0x40`, `0x41` | `sub` bit 0: the first line is the speaker's name (not in full-screen windows: the exe drops it there) | converted (History, item 27) |
| `0x49` | the one edge colour of its own (0x808080, a grey outline; History, item 29): 1 note | dropped (no outline for message text) |
| `0x56` | u32 depth at +2 (0 and 2000): 2 notes | dropped |
| `0x59` | the old track fading out under the new one | partial: it stops at once (one BGM channel) |
| `0x6f` | the pulse | converted, not checked against the original |
| `0x70` | the depth in the second u32 among the bust-ups (80, in 2 negative flashes) | partial: drawn over all the bust-ups (History, item 30) |
| `0x76` | MeltDown's stretch for `sub` 46/47 | converted (scaled from the exe), not compared with the film |
| `0x8f` | removes the bust-ups only | partial: `csp -1` also removes the fade cover (no script shows the difference) |
| `0x30`, `0x62` | everything | dropped (the unreachable debug menu) |

Two doc mistakes corrected on the way: `0x8e`'s last 32 bytes are an
(always empty) module name, not junk; `0x76`'s third u32 is a sort key,
not an easing curve. `0x8f` removes only the bust-ups (keys 11-110), yet
is converted with `csp -1` (the overlay, static and animation are put
back after it; the fade cover goes, which no script shows the
difference of).

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
- After a rebuild delete `kidoku.dat`: its offsets go stale.

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

## Status: reopened (2026-10-14): fixes from the debug trace, then an opcode audit

### Queue (2026-10-16)

- Merged: `0x86`'s two trailing bytes as harmless (#163; History, item 45).
- Merged: `0x8c`'s sort key as harmless (#180; History, item 31).
- Merged: `0x49`/`0x4a`'s edge colours as harmless but the grey one (#181; History, item 29).
- Merged: links only while their variable is set (#183; History, item 28).
- Merged: the debug menu's message block as harmless (#189; History, item 42).
- Open: the ruby alignment as harmless (History, item 33).
- Open: command coverage per operand, PR #85 (supersedes #28): every
  operand given and not read, whatever its value (0 included), is a
  dry-run note and a gap in `metrics.json`, except the filler (`NO_SUB`,
  `NO_ARG`) and the harmless table ("Operand coverage"), each row with
  the values it covers. Merged.
- Open: `0x7a`'s size at +14 as harmless, PR #164 (History, item 44).
- Merged: `0x46`'s last u32 as harmless (#170; History, item 43).
- Merged: screen filters under the bust-ups, `screenfilter`'s new z (#177, engine #56; History, item 30).
- Open: speaker names from `sub` bit 0, PR #102 (History, item 27):
  4,867 notes of 20 kinds left; 80,285 of 84,713 calls fully converted
  (was 49,328).
- Merged: the red fill over the static in 0006.cd as a screen filter (#157; History, item 40).
- Merged: `0x43` and `0x52`/`0x53` `arg` as harmless (#111, #114; History, items 32, 34).
- Open: `0x90` +2 as harmless, PR #119 (History, item 35).
- Merged: Filter-Replace's begin angle as `screenripple`'s start phase (#134, engine #50; History, item 38).
- Merged: Zigzag's phase from its frequency, its begin angle and reduction as harmless (#153; History, item 39).
- Merged: the exe map's fixes and missed callbacks (#46).
- Merged: the rotation's interpolation byte read (#162; History, item 41).
- Open: `0x59` as a real cross-fade, converter #25 with engine #18 (asked
  for a web-build measurement of the old track's decode first);
  `bitand` for the three bitwise ops, converter #74 (takes over #12;
  engine #11 in main's pointer since #72).

Declared finished on 2026-10-14, then reopened the same day: the 0.19
debug trace and the decompiled exe (`research/tools/ghidra/ghidra_export.sh`,
names in `research/xuse/names.tsv`) showed opcodes read wrong from
`objdump`. All 26 items of that list are done, and so is everything the
debug trace found ("History", below). Open: items 28-45 below, from
the operand notes.

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

What follows is the optional leftover list, kept for whoever comes back
to the game. Nothing in it was asked for.

**Settings screen.** The original's settings dialog has six pages; these
parts are not converted:
see History, item 26: the auto waits as two rows, the skip/auto
switches (their defaults are converted), antialiasing, the CG viewer's
voices, the system voice, wheel sensitivity, frame rate, inactive-window
behaviour, the shortcut menu and gamepad.

**Other system screens.**
- The autosave slots: the original's are in its right-click menu
  (オートセーブ ▶ 「1 - 開始位置へ戻る」, back to where the session
  started), which was dropped.
- Extras: the CG gallery, the scene replay ("chapter") and the BGM room
  are done; the system voices (WSS, title image 13) are dropped (the
  user's decision; the item stays greyed out). No title BGM (the
  original's title code plays none, as far as was checked).

**Items from the operand notes (2026-10-16).** The dry run counts the
operands the converter is given and never reads ("Operand coverage"
under "Script bytecode"); the user made every note an item. Numbered
after History's 26, most noticeable first (notes = instructions):

29. **The grey outline** (`0x49` edge colour `0x808080`, 1 note): one
    message's characters get a grey edge; message text has no outline
    in CONscripter (`strspoutline` is for `strsp` only).

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
  frame. The rarer layer-2 `0x7a`/`0x7b` records (6 and 7 uses), which
  load no animation, are the blog scenes' bust-ups, converted with
  `0x84`/`0x7d`/`0x7c` as the character layer's sprite (see the opcode
  table);
- `0x6d` quakes, `0x6e` flashes and `0x6f` pulses are done. `0x55` is done: a second background object, mostly the
  `frameNN` overlays (letterbox bars, speed lines), drawn as sprite 61
  (over the characters) or 999 (under them), kept across `csp -1` but
  removed by a background change (`0x54`, which with `sub` 1-3 puts
  `frame01`-`03` there itself; found in the audit); `0x8c`'s static is
  sprite 60, over it;

**Whole-exe map (2026-10-15, `docs/new-game.md` §2 "Mapping the whole
exe").** The pointer-only callbacks (323 `cb_<addr>` in `names.tsv`,
five rounds until none was left; 3,759 → 4,081 functions; 2026-10-16:
53 more that are passed as arguments or cast, 4,134), the static
CRT helpers and the settings/skip flags are named. The exe's own
`..\Src\…\*.cpp` strings in `strings.tsv` map its modules; the
`..\Library\…` ones are its in-house framework (gaf004, DX), no
statically linked third-party libraries (libpng comes from a DLL).
Every wait (text page, `0x68`, the wait icon) polls `story_input_poll`
(`0x477d90`), which opens hide-window, save, load and the chapter
history; `menu_allowed_mask` by `replay_mode` (`0x51ec70`) allows
save and load in play only (mode 0), none in a scene replay (1). Mode 2,
an ExtraMode script with save and load, is unreachable in this release
(its script field is only ever set to -1). Coverage: all 82 opcodes the corpus uses
(`cd opstats`, 84,713 instructions) are converted and agree with their
handlers (audit table above); the dry run reports `0x30`/`0x62`
(debug menu) as unsupported, plus the operand notes above. Lip sync (`VoiceSyncParts`, `lq.arc`) and animation parts
are in the exe but unused here (no `lq.arc`; `0x8e`'s module names are
empty). Gaps found, where the exe and the converter differ:
- `0x49`'s edge colour (1 of 565 records, `0x808080`) and `0x4a`'s (2)
  are not converted.

**Accuracy.**
- Transitions: MeltDown's stretch for subs 46/47 is scaled from the exe
  (filmed for item 25, not yet compared).

**Media and packaging.**
- `arc.nsa` is about 1.2 GB; if the web build can't handle it, split the
  voices into `arc1.nsa`.
- Check that saves, thumbnails (`savethumbN.bmp`) and `kidoku.dat`
  persist in the WebAssembly build.

## History

Finished work, moved out of "Status" so a session reads only the open
state. Items are referred to elsewhere as "History, item N".

**Effects off (2026-10-15, a gap from the whole-exe map):** with
画面エフェクトを使用 off (`setting_effects` `0x51ed14` bit 0) the
original draws no quake (`0x6d`), pulse (`0x6f`, `surprise_start`
`0x441960`) or script fade (`0x72`/`0x74`: `script_fade` draws no cover
at all). Each of these now sits behind `if <effects off> goto *sN_fxI`
(`Op::IfSwitch` on `Switch::Effects`), so none of them runs; before,
`effectoff` only cut `print` effects and the fade cover appeared at once
and stayed until the next background.

**Skip mode (2026-10-15, a gap from the whole-exe map):** while
skipping (`skip_state_a`/`_b`, `0x51f82c`/`0x51f82e`) the original draws
none of `0x6d`, `0x6e` (flash), `0x6f`, `0x72`, `0x74`. Each now starts
with `isskip %105` / `if %105 == 1 goto *sN_fxI` (`Op::IfSkipping`, the
flash too; it isn't cut by effects off). Before, only their waits were
skipped: the shake, flash and pulse still ran and the fade cover showed
at once. Kept in the converter, not the engine: the exe's skip check is
in these handlers only, so `0x8c`'s environment shakes and ripples
(the same engine commands) are taken to go on in skip mode. A skip by holding Ctrl isn't
seen by `isskip` and still draws them.

**Variables (2026-10-15, from the code audit):** the animation's state
per link-scene level moved from %13-%20 to %21-%28: the game's local
%13 (read once in script 6, `if %13 != 4603`, never set) shared it.
No visible change: the test went the same way either way. A save made
inside a link scene while an animation plays (from an older build) may
show that animation's opacity wrong once after a load.

**The list after the reopening (2026-10-14), all done:**
1. **Done: `0x72`/`0x74` as fades** (table above): `Op::ScreenFade`, an
   opaque `lsp2 26` faded in under the text window and bars, removed with
   the next background, `csp -1` and a link return. Filmed in the
   original first (`0004.cd` #23 patched); a trace of the first route
   shows the colour only at scene ends and on `s11_l54`'s 7 pages over
   white, as the script has it.
2. **Done: the static after 存在消滅 (`s22_l2`)** is cleared at `0x39`
   (table above), as the original shows it; the trace's four "static up
   for 150 pages" findings in `s4_l2` are gone.
3. **Done: `pushscreen: too many saved screens`.** Encyclopedia link
   scenes that link to each other (`s99` ↔ `s137` …) nested one level per
   click, past `pushscreen`'s 32 (and past level 7 the per-level
   variables collided with `%190`). Link scenes now nest at most 7 deep
   ("Hyperlinks" above). Random runs from `conv_link98` (`--seed 2`,
   `3`) that stopped at the error now run 3,000+ pages.
4. **Done: the opcode audit** ("Opcode audit" under "Script bytecode"):
   five fixes (`0x8e` mode bit 1, `0x54` overlay/frame and no fade,
   `0x66` unclickable waits), each checked with a replayed trace, then
   sound-effect fades (`0x5a`/`0x5b`) with CONscripter's new
   `dwavefadein`/`dwavefadeout` (the user's choice).
5. **Done: `cons_explore.py` over all routes** (the build after items
   1-4 and the sound fades; `--seed 1`, output in `out/saiima-explore`,
   not in `/tmp`, whose quota a 35 MB-per-run exploration fills): 4 runs,
   98,618 pages, 1,411 labels (1,220 before), 1,767 never reached (the
   explorer's `untaken` choices stop finding new ground after run 1; the
   heroine routes need choices in a set order). The checker on every
   trace found, and this fixed:
   - colour tags shown as text after a half-width name and a space
     (`Ghost #FFFFFF`, `conv_link35_1`): CONscripter's new "spaces after
     text are kept" left an odd half-width run that took the `#` as its
     pair. Fixed in CONscripter (a half-width character isn't paired with
     `#`, `!` or `(`);
   - pages that ran past the full-screen window because a line wraps
     there though it fits the original's proportional font (blog headers
     with half-width spaces): the converter now counts each line's rows as
     the engine lays them out (`line_rows`) before joining a message onto
     the page (`conv_link34_1`).
   Left, for whoever comes next: the rest of the findings are the known
   kinds (sounds started twice, bust-ups at one place, looping ambience,
   the 37600 px `fx` strips).
6. **Done: messages taller than the window** (the user's list after the
   exploration, 2026-10-14). `s7_l22`'s blog message is 24 lines whose
   two headers wrap: `line_rows` counted the converter's text, before the
   script writer makes `/ ( ) !` and the other special characters
   full-width (`escape_text`), so a header like
   「ほっとした  投稿者:ToolSmith004/どらいばあ(米) …」 looked like one row.
   It now counts them as written, and a message taller than the
   full-screen window goes on as many pages as it needs (`split_rows`):
   at line ends, before the last post separator (`━`) when one is in the
   second half of the page, with links and styles cut in two at the split
   (a reading across it is dropped; none is). Voice, name and hanging
   indent stay with the first page. A trace from `--goto s7_l22` has no
   page past the window.
7. **Done: scenes entered by a jump got the previous file's window.**
   `s157_l1:262733` ran past the message box when reached from link scene
   `conv_link23`. A script file starts with two label markers (`sN_l0`,
   then `sN_l1`), and `falls_into` took the empty `sN_l0` for code
   falling into `sN_l1`, so `sN_l1` never took the window its callers
   agree on and kept the one the file before it ended with. In s12 and
   s157-s169 (the COMPLETE scenes) that was a full-screen window: the
   names were written into the text (【シャーリー】「…」) and the pages
   laid out for 40 columns, in the message box. Now markers right before
   a label are passed over, and the start of a file falls into nothing
   (only `s4_l0` is ever jumped to). The rows a message takes also count
   what the script writer adds: a full-width space before a line that
   starts with ASCII, and the hanging indent (`indent`) on wrapped rows
   (`line_cells`, `hang_cells`; the chat log in `conv_link35_1`), and a
   message taller than any window, not only a full-screen one, goes on
   two pages. Explorations of the same route (`--seed 1`, 68,619 pages)
   had 5 pages past the window before these fixes, 1 before the indent
   fix (`conv_link35_1:48911`) and none after it; the other findings
   didn't change. Starting at `--goto conv_link35_1`
   still shows pages past the window: the full-screen window is set just
   before that label, so a start there is in the message box (not a
   problem in play).
8. **Done: sound effects in skip mode.** `sound_request` leaves a
   sound effect that doesn't loop out while skipping; the converter now
   marks those (`Op::SeNotSkipped`) and the script writes `isskip %105`
   and `if %105 != 1` before their `dwave` (and fade-in).
9. **Done: the auto wait per page and per character.** The original
   waits a time per page plus a time per character (`0x476f50`; the
   settings' 改頁待ち時間 文章単位, 0-5 s, and 文字単位, 0-100 ms;
   defaults 2.5 s and 20 ms, `0x46d000`). CONscripter's `automode_time`
   takes a second number for the time per character (added for this), and
   the settings' one オート速度 row sets both (`Setting::AutoWait`): at
   level v, (10 - v) × 500 ms plus (10 - v) × 4 ms a character, so the
   default level 5 is the original's default. Two rows, as in the
   original, don't fit: the settings screen's sprites are all used.
10. **Done: a save inside a link scene keeps the levels around it.**
   The original keeps its scene stack in a save (`CLocalSceneStack`);
   the port kept only the level saved, so after a load the levels above
   couldn't be gone back to. A save from the menu now also copies the
   levels' slots (900 + level, those between `%190` and `%151`) to save
   slots 1000 + 10 × slot + level with CONscripter's new `copysavegame`
   (`*conv_scene_keep`), deleting a slot's old ones; a load
   (`*conv_scene_loaded`, the slot in `%2906`) copies them back and keeps
   the floor and the deepest level as saved; deleting a save deletes
   them. Checked with `smoke.py` from `conv_link2`: enter the link, save
   to slot 1, load it, "back" goes to the root page, "forward" is there.
11. **Done: 「次の音声まで停止しない」.** The original's voice page has
   it, on by default (field 0x72, `0x51ed59` = 1 in `0x46d000`): a voice
   plays on after a click until the next voice, as the port always did.
   It is now a switch on the settings' third page (renamed その他 from
   マウス・確認), `Switch::VoiceKeepsPlaying`; off, the click that reads
   on stops the voice (`dwavestop 0` in `*conv_wait_end`). Its flag is
   `%2910`: the switches' flags from `%3640` end where the scene
   replay's start (`%3650`), so switches after the eighth (Muramasa's
   `ConfirmBackToChoice`, `%3647`) start at `%2910`.
12. **Done: flashes cover the text window.** `0x6e`'s object sorts over
   everything (key `0xffffffff`); the port's flash was an `lsp2add`,
   which the engine draws under the text window. CONscripter's new
   `screenflash opacity,colour` adds the colour at the very end of a
   redraw, over the window, the bars and the buttons
   (`Op::ScreenFlash`; `Op::Flash` stays as it was for the other games).
   Whether the original's flash also covers its button bar (a window of
   its own, `CSystemTray`) wasn't checked.
13. **Done: the static rises in.** `0x8c`'s Animation-RandomNoise goes
   from opacity 0 to its end opacity over `time_ms` (37 of the 88 uses
   take 1-2 s) while the script goes on; the port showed it at its end
   opacity at once. CONscripter's new `spfade num,ms` fades a sprite in
   by itself, and the static's `lsp 60` is followed by `spfade 60,ms`
   (`Op::NoiseFadeIn`). Inside a transition the transition's snapshot
   catches the static at the start of its rise, and the rest of the rise
   follows the transition.
14. **Done: a guided run through all twelve loops.** The explorer's runs
   stop finding new ground because the heroine routes need their links
   clicked in a set order. `research/xuse/route.py <build>/0.txt` writes
   a `--choices` file (the heroine links of 最初の日 found by their text:
   none in loops 1, 6 and 7, あずさ, さやか, ふえこ, ようこ in 2-5 and
   8-11); `cons_explore.py <build> --route route.txt --runs 1 --stop-known
   0` then plays the whole game (70,186 pages, to the title after the
   epilogue). Its findings are the known kinds, nothing new. It reaches
   1,415 labels, as the plain exploration did: of the 1,766 left, all
   but a few dozen are the extras (the CG viewer's 425, the scene
   replay's 440) and the scene replay's own scripts (0014-0018, 0020,
   `09_回想*`), which only the extras menu enters.
15. **Done: drawing order, filmed under wine.** The test site is
   `0004.cd` #13-#18 (a CrossFade, background, filter, one bust-up of
   depth 50, the wait), patched and filmed at 30 fps from the click at
   #12 (`research/xuse/local/wine`: `run.sh`, `cdpatch.py`; recipe in
   "Running the original under wine"). Objects sort by a key: a band from
   the layer byte, then the depth in the low 24 bits (`0x4126c0`; for
   transitions `op_76` maps layer 0 to 4 through the table at
   `0x503e54`: 1 → band 0, 2 → `0x04000000`, 3 → `0x06000000`, 0 and 4
   → `0xfe000000`, the top). Filmed:
   - a transition keyed layer 2 depth 45 leaves the depth-50 bust-up out:
     it pops in at the start while the background cross-fades; keyed
     layer 3 depth 0 it fades with the rest. The default key `0x271000`
     is the top band: everything. Going through the 12 keyed records
     with this, 11 cover everything they change (the two Zooms change a
     bust-up of depth 40 and a frame of depth 1, under their key at
     depth 45); the twelfth (`0011.cd` #193) clears a layer-0 filter at
     once (converted: item 20);
   - a layer-0 frame (0x55, depth 1000) is under the depth-50 bust-up, a
     layer-2 one over it; a layer-0 brightness filter (0x70, depth 1000)
     darkens the bust-up too (0x70's layer table differs). Overlays are
     now over the bust-ups only from layer 2 at a depth over 150
     (`overlay_over`): the 7 layer-2 frames at depth 0-100 go under them;
   - bust-ups at depth 1100-5000 (62 entries) sort over a layer-2 frame
     at 1000: CONscripter's new `spz num,over` draws them right over the
     overlay's sprite 61 (`Op::SpriteInFront`), under the static and the
     animations (top band). `s10_l51` shows the translucent close-up of
     character 7 over the letterbox bars;
   - SmoothCrossFade (3 s): the old picture stays a third of the time,
     cross-fades in the second third, the new one stays the last third;
     the pictures blur slightly either side (item 22). Converted as
     `delay`, a cross-fade of a third, `delay`.
16. **Done: quakes, flashes and pulses compared with the original.**
   Filmed the same way, each in place of message #21 (after two text
   pages), and ours on the same picture (`ours.sh`):
   - flash (`0x6e`, white): +127 on every channel for 100 ms over
     everything, the text window and the top bars too: as our
     `screenflash 128` (item 12);
   - pulse (`0x6f` 1000 ms, presets 5 and 1): the sky's green goes from
     253 to 155 and back over 1 s in both; it lies under the text window
     in both;
   - quake (`0x6d` speed 80, 20 × 20): the original jumps along the
     7-point path one point a frame, offsets (20, 15), (0, 10), (5, 0),
     (-10, -10)… at the full range for about 5 points, then shrinking,
     15 frames in all. Ours slid between the points (CONscripter's mode 1
     interpolated, though documented as jumping: fixed) and shrank from
     the start, at most 12 px. Now (`lower_quake`): at most 30 points a
     second, the full range for 4 points, then shrinking: 14 frames,
     starting at 20 px.
17. **Done: the button bar's lock button** (`SysTray.004` #9, filmed under
   wine). By default the bars are pinned: always up, the button an open
   padlock (cells 4-7). Clicked, the padlock closes (cells 0-3) and the
   bars unpin: the top bars stay hidden, and the bottom bar shows only
   while the pointer is below y 573, fading in over about 100 ms (item
   21). `BarAction::Lock { reveal_y }`: the global
   `%2908`, and while unpinned an invisible `btn` hotspot on the other
   side of the line (`getmouseover`) ends the click wait when the
   pointer crosses it, so the bar is drawn again. Checked on our build
   on its own Xvfb display (`og.sh`): the same four states as the
   original.
18. **Done: mute switches.** The original's サウンド page has a ミュート
   box per channel. They are three more switches on the settings' third
   page, on by default: 「ＢＧＭを鳴らす」, 「効果音を鳴らす」,
   「音声を鳴らす」 (`Switch::BgmOn`, `SeOn`, `VoiceOn`; flags `%2911`-
   `%2913`). Off, the channel's volume is 0 and its level is kept; a
   toggle applies at once. The page's rows are 36 px apart now (was 50)
   to fit eleven switches.
19. **Done: the chapter history** (`ChapterHistory.004`, filmed under
   wine). Clicking the top chapter bar opens it over the scene (text
   window hidden): the chapters reached since 0x39 last cleared it (at
   each loop's start), three columns of 19 rows, the current one
   bracketed, the row under the pointer highlighted. Choosing one asks
   「チャプタ "…" に移動します。よろしいですか？」 and goes back to the
   start of that chapter; the chapters after it stay listed until a new
   one is reached. The port (`crates/conv-cons/src/chapters.rs`,
   `TopBars::chapter_history`): each `Op::Chapter` keeps the game there
   (`savepoint`, `savegame2` with the title) in save slot 700 + entry;
   the entries and the current one are locals `%193`/`%194` (a save
   keeps its own history), carried over the load that goes back
   (`%2909`); `Op::ChapterHistoryClear` (0x39) empties it. The rows
   follow the pointer with a 50 ms `btntime`; the chapter bar's button
   is an invisible sprite in the bar's slot 48, so auto-play's
   `--trace-ignore 22-48` keeps out of it. Checked on our build on its
   own Xvfb display against the original: the list, the hover, the
   question, and going back keeps the later chapters.
20. **Done: a filter change inside a keyed transition shows at once.**
   Of the three `0x76` records keyed below the top band (the record's
   third u32 has layer byte 1-3), `0011.cd` #193 (a 0.5 s CrossFade
   keyed layer 2 depth 100) clears a layer-0 filter (`0x70`) before its
   `0x71`. A layer-0 filter sorts over the key, so the original clears it
   at once while the rest cross-fades (item 15). The converter now moves
   such a filter change to just before the transition, where it is drawn
   with its own `print 1`; the `csp` and bust-up change after it still
   fade.
21. **Done: the unpinned bar fades in.** When the pointer comes down to
   the hidden bar (item 17) its sprites fade in over 100 ms
   (`BarAction::Lock { fade_ms }`, `spfade` on each; `%148` remembers
   whether the bar was already shown, so a new page doesn't fade it
   again). This found two bugs in CONscripter's `spfade`, fixed there: a
   sprite loaded without an alpha (`trans` -1) was drawn at nothing until
   the fade ended, and each fading sprite was drawn in its own screen
   update, so under the software renderer the buttons came up one after
   another. Filmed on our build at 60 fps: the eight buttons rise
   together over six frames.
22. **Done: SmoothCrossFade's blur.** Measured on item 15's film (30 fps,
   fitting each frame against the sharp picture blurred by a Gaussian):
   the new picture comes out of the cross-fade blurred by about σ 1,
   about σ 0.5 halfway through the last third and sharp after three
   quarters of it, then stays; the old picture blurs at the start (its
   sky is too smooth to measure the radius: taken as the same). The
   converter now sets a 3×3 mean blur (`screenfilter 4,6,0,256,1,1,0`,
   the closest to σ 1; slot 4, the game's filters use 1-3) with
   `print 1` before the first third's `delay`, cross-fades with it on,
   then clears it with a fade over three quarters of the last third and
   waits the rest (`end_transition`). 8 uses in the game. Filmed on our
   build: blurred at the start, sharp at the end.
23. **Done: the four unchecked assumptions, checked under wine.**
   - Text window opacity: the same page shot with the box and hidden
     (the bar's × button), fitted row by row as scene × (1 - a) + C × a:
     a = 0.495 on every row, C the base picture's grey gradient (226 to
     19). The converter had 55 %; now 50 % (`WINDOW_OPACITY_PERCENT`).
   - Save menu base: also 0.50 (the same constant). The scene behind
     it is also blurred slightly (best fit Gaussian σ 1): converted as
     `SaveMenu::backdrop`, a 3×3 mean in `screenfilter` slot 4 while the
     menu is open in game, off while `savegame2` writes the save (saves
     keep the screen filters). Checked on our build: blurred in the
     menu, sharp after closing it and after loading the save made there.
   - Brightness filter: `0004.cd` #16 patched to kind 4 (brightness),
     -100, against no filter: every channel goes down by 99, clipped at
     0. So the value adds on a 0-255 scale, as converted.
   - Negative mask softness: #13 patched to a `0x77` with mask 9 (bright
     in the centre, dark at the edges), 2 s, softness -30 and +30
     (`patch/Mn`, `patch/Mp`). With -30 the new picture grows from the
     centre (bright first), with +30 from the edges (dark first), as
     converted. Negative is the common case: 200 of the 234 `0x77`.
24. **Done: the full-screen text styles, measured.** `0002.cd`'s ADV
   preset (#79-#82: `0x4c` mode, `0x4f` style, `0x50` position, `0x4d`
   instant) patched to each style in turn (`patch/S1`-`S8`), the opening
   shot after five pages; ours drawn with the same lines under each
   style's `setwindow` (`--goto` a test label, `--shot-pages`). The
   white text with its dark shadow was found row by row in both and the
   line tops, left edges and character pitch compared. Changes: style 1
   is a 24 px font (was 21), 24 columns; styles 2/7 start 24 px right and
   15 px lower, line gap 12; styles 3/4 an 18 px pitch (17 px font), 21
   px right; style 5 (the footnotes, the most used) an 18 px pitch and
   28 px lines (was 17 and 27), 36 columns to stay in the box, 16 px
   right; style 8 a 21 px pitch, 33 px lines, 42 px lower. All now within
   2 px of the original. Style 6 isn't used by the game and keeps its
   estimate. The original draws with a substitute for MS Gothic under
   wine, so the strokes' weight wasn't compared. Styles 3/4 write a
   speaker's name after the line (「…」忍), 5 on a line of its own, but
   no full-screen scene has a speaker.
25. **Done: BoxRolling as a turning cube.** `0004.cd` #13 patched to subs
   29 and 31 slowed to 3 s (`patch/T29`, `T31`; `T46`, `T47` film
   MeltDown). The step function (0x442420) sums -(cos + sin) / 2: two
   faces of a cube as wide as the screen turning a quarter turn, seen
   straight on, each picture squeezed to its face (sin, cos) and cut at
   the screen's edges; fitted on the film at the middle (edge at x 380,
   the old picture's left 560 columns in 420 px; the model: 360, 583 in
   440). CONscripter's new `cons_cube.dll/dx,dy` draws it
   (`TransitionStyle::Cube`); it was a push of both pictures. 8 uses.
   The exe also runs three passes over the new face (0x4291f0), probably
   shading, not converted. MeltDown 46/47 (3 uses; 45, which the
   converter's melt was fitted to, is never used) differs from our melt:
   the films show columns starting to fall between 0.4 and 0.8 of the
   time and the old picture smeared into narrow fingers. The draw code
   (0x4458e0, 0x4456b0) works per row: one shift grows with time, a
   column starts when the shift passes its random curve value, and the
   old picture's rows below that point are pulled down by an offset that
   grows by the acceleration (0x4451e0: ((b/255)^2 × 0.45 + 0.03) /
   height) on each row. Not ported: the decompiler lost some arguments.
26. **Done: four more of the original's settings** (its dialog filmed
   under wine, all six pages, for the options and defaults): 画面エフェ
   クトを使用 (`Switch::Effects`: off, every transition shows at once;
   CONscripter's new `effectoff`), セーブファイルのプレビューを表示
   (`Switch::SavePreview`: the save menu's large picture), 文字装飾
   (`Switch::TextShadow`: the message text's shadow, CONscripter's new
   `textshadow`; from the next page) and チャプタ履歴の上書き確認
   (`Switch::ConfirmChapter`: the chapter history's question), all on by
   default as there. The switches page now holds 15, 26 px apart (flags
   `%2915`-`%2918`). Left out: the two auto waits as two rows (one row
   sets both: the settings' sprites are all used by the six rows, and a
   seventh means renumbering every game's menu sprites), the four
   "選択/クリックでスキップ/オートモードを解除" boxes (on by default,
   which is how the port behaves: links stop skip and auto, a click
   stops them), antialiasing, the frame rate, the shortcut menu's
   position and keys (no right-click menu), running while inactive,
   gamepad, the settings dialog's own OK/cancel questions (ours applies
   at once), the autosave's load question (no autosave) and wheel
   sensitivity.

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
  (`SCENE_FLOOR` in `crates/conv-cons/src/menu/mod.rs`): back, exit and the
  level list only reach levels at or below it. Each level's page lives in
  save slot 900 + level, which belongs to the current playthrough, so
  after a load the levels above the loaded one can't be gone back to (they
  would load another playthrough's pages) and the deeper ones are dropped
  (`%151`). A switch between levels (`%2900` = 1) carries the floor over
  instead. The original keeps the whole stack in its saves (the exe has
  `SaveControl\LocalSaveStack.cpp` and `CLocalSceneStack`), so a load
  there can still go back. So does the port since 2026-10-14 (History,
  item 10): a save from the menu keeps the levels' slots beside it and a
  load puts them back, with `%190` as saved; only a save made by an
  older build loads as described above.
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
  (0x4347b0) starts. Every bust-up change now fades over 500 ms. The
  depth itself: bust-ups stack by character number here; those at
  1100-5000 are drawn over the `frameNN` overlay (History, item 15).
- **Scenes laid out for the wrong text window (fixed).** A second run
  (CONscripter 0.19.0) found 1,320 pages whose text ran past the window,
  all in `s13_l3`-`s13_l37` and `s10_l98`, and 30 voices cut off on the
  same pages. Each of those scenes comes after a footnote scene in the
  file (`s13_l1`, `s10_l97`) that sets the full-screen window and
  returns. The converter followed the window in file order, so it laid
  the next scene's text out for the full-screen window (16 rows to a
  page, names in 【】) while the game was still in the message box (3
  rows). Now a label that the code before it can't fall into takes the
  window its jumps, gosubs and links have when they all agree, and a
  gosub to a subroutine that calls a preset (`s19_l1` calls `s2_l18`)
  counts as setting that window (`Presets::entry_window`, `sub_window`;
  tested by `windows_follow_the_callers`). The same route replayed
  (`--replay`) has no page past the window (122 before), no voice cut
  off and no save round-trip difference.

Fixed in CONscripter by the same work: half-width katakana in Shift_JIS
text (「影（ｼｬﾄﾞｳshadow ）」 in `s7_l8` drew as a blank and lost the next
character), and saves losing the speaker name (`strsp` 49) and looping
sound effects on `dwave` channels.

27. **Done: speaker names from `sub` bit 0** (2026-10-16). `0x40` and
   `0x41`'s `sub` bit 0 says the first line is the speaker's name; the
   converter used to guess it (a 1-8 character first line before a
   「『（ line), which took three lines for names: `0010.cd` #5832
   (象牙の単眼鏡。) and #5839 (小皿。), the encyclopedia's item lines,
   and `0078.cd` #22 (a blank line before 『凶器に関する疑問』). It now
   reads the bit. In a text mode that can't show names (`0x4c` mode 1,
   the full-screen windows: `text_mode_allows_name` `0x4679d0`, mode 0
   yes, 1 no, 2 by bit 6), `show_message` (`0x475f40`) clears the bit
   and the name is a line of text; the converter does the same, though
   no named message falls in a full-screen window (the 0.txt diff is the
   three messages only). Every `0x40` message has id 0: none is voiced.
32. **Done: window hide's `arg`** (2026-10-16). `0x43`'s `arg` (1 in
   530, 0 in 48) doesn't make it wait for the window's fade-out:
   `op_43` (`0x4772a0`) starts the hide and returns 2 (go on in the
   same frame) for 0, 0 (next frame) for anything else. Ours (`textoff`)
   goes on at once for both; one frame isn't worth converting, so the
   values 0 and 1 are a row of the harmless table ("Operand coverage").
   The 578 notes are gone; `0.txt` is unchanged.
34. **Done: the click waits' `arg`** (2026-10-16). `0x52`/`0x53`'s
   `arg` (odd numbers, 421 and 221 distinct values: the tag's source
   line, between the messages' even lines) is the wait's read-flag
   number: `FUN_00476a80` keeps it at the window's +0x10 and the wait
   (`0x476750`) marks (file, it) read with `set_read_flag`
   (`0x4bb450`), as `show_message` does with a message's line.
   CONscripter's kidoku keys the same wait by its place in `0.txt`, so
   any value is a row of the harmless table ("Operand coverage"). The
   1,618 notes are gone; `0.txt` is unchanged.
35. **Done: `0x90`'s module field** (2026-10-16). The u32 at +4 (1 in
   649, 0 in `0010.cd` #4379, 2 in `0011.cd` #5216) goes to the new
   preset module's +0xc (`op_90` `0x434960`), the key `FUN_0040e2a0`
   sorts an object's modules by (its list at +0x68, the order their
   draw is chained in). The preset is the bust-up's only module there:
   the module a bust-up gets from its `0x8e` entry (type 0x77, id 0x15,
   `FUN_00434130`) needs a module name, and all of the corpus's are
   empty; the 0x86 motions (slot 1) are in `0010.cd` #9151 on, on
   another character, and none in `0011.cd`. So the three values draw
   the same and are a row of the harmless table ("Operand coverage").
   The 651 notes are gone; `0.txt` is unchanged.
36. **Done: `0x88`'s module field** (2026-10-16). The u32 at +8 (0 in
   41 records; 2 in `0010.cd` #7950, 2000 in #8881, 1000 in #9207 and
   `0013.cd` #990) goes to the module's +0xc (`op_88` `0x415b50`), and
   `FUN_0040e2a0` (insert `FUN_004128d0`) keeps a preset's modules in
   ascending key order, a tie after the modules already there. Where
   modules of one preset are set together, the keys rise with the slot:
   the mood tint (slot 1, key 0) under `0010.cd` #7950's grey (slot 2,
   key 2) and `0007.cd` #4075's brightness (slot 3, key 0, added after
   it); `0011.cd` #2391's tint (preset 2 slot 1, key 0) under the
   negative that `0010.cd` #9207 left in slot 2 (key 1000). `0013.cd`
   #990 is preset 2's only module (cleared at `0011.cd` #2403 and
   `0012.cd` #1487). So the converter's slot order is the exe's draw
   order, and the field is a row of the harmless table. The 45 notes
   are gone; `0.txt` is unchanged.
37. **Done: the blog bust-ups over the text window** (2026-10-16). The
   six blog scenes (`0087`, `0117`, `0125`, `0130`, `0135`, `0145.cd`)
   make their picture with a layer-2 `0x7a` whose group byte (+5) is 4
   and depth (+6) 2000. `op_7a` (`0x414590`) turns the group into the
   object's band through the table at `0x503e68` (4 → 0x30), and
   `FUN_004126c0` makes 0x30 the top band `0xfe000000`, over the text
   window (`0x06000000`). Ours drew the picture as the character layer's
   sprite, under the window: the full-screen blog window's grey and
   border lay over the bust-up. Now the `0x7a` adds `spovertext` on that
   layer (`Op::SpriteOverText`), so the picture is drawn over the window
   and its text, as in the exe; the depth, 2000, is the only one in the
   top band there, so it is a row of the harmless table. `0.txt` gains
   one `spovertext` per scene (and a `print 1` in `0135.cd`, where no
   transition is armed). Checked with `--goto s87_l1 --shot-pages 6`:
   the bust-up is over the window, the bottom bar's buttons over it.
   Not checked: whether the original's top bars are under the picture
   too (ours: the hair covers the level bar's background, not its
   buttons).
38. **Done: Filter-Replace's begin angle** (2026-10-16). Filter-Replace
   (`0x70` kind 13) ends with a begin angle, 1 in all 16 records. The
   draw (`0x41d390`) turns it into the start of the sine-table index
   (1024 a turn) as `(begin << 16) / 100 >> 6` = 10, and the per-pixel
   loop (`0x43d9d0`) reads entry `start + grey × 4`: the ripple starts
   10/1024 of a turn in, where ours started at 0 (up to 2π × 30/100 ≈
   1.9 px off at distance 30). CONscripter's `screenripple` takes an
   optional start phase after its blend (engine #50; 1/1000 turn), and
   the converter writes it (`ScreenEffect::Ripple`'s new `phase`):
   `,0,10` on the 16 ripples. The other 204 notes the item counted under
   this field are the cleared slots' trailing u32 (kind 0, or slot 0):
   `op_70` (`0x415a30`) clears them without reading blend and opacity,
   so they are a row of the harmless table, as `0x88`'s are.
39. **Done: Zigzag's begin angle and reduction** (2026-10-16). The
   Zigzag record (`0x70` kind 12) ends with a begin angle (10) and a
   reduction (0); the debug output (`0x41da30`) names the fields at
   +0x30 Freq, +0x34 BeginAngle, +0x38 Reduc. The draw (`0x41dab0`)
   reads only +0x20 to +0x30: it passes `(freq << 16) / 100` to the
   kernel (`0x43f0b0`) as both the ring scale (d × it >> 16, as before)
   and the start phase (it >> 6 = 102 of 1024 sine entries). So the
   begin angle and the reduction do nothing, and the ripple starts 102
   entries in: the converter writes `screenripple`'s phase (`,1,100`,
   1/1000 turn) and lists the two fields as harmless. The Multi blend
   the row called missing had been converted already (blend 1).
40. **Done: `0x56` fill depth** (2026-10-16). `op_56` (`0x4224a0`)
   passes the record's u32 at +2 to `object_create` as the fill's sort
   depth in its layer's band (`FUN_00412fe0`). Of the two slot-1 fills,
   the boot's black (layer 0, depth 0) is alone on the screen. The other
   (0006.cd, the "401 Unauthorized" page) is on layer 3 at depth 2000:
   band 0x20, over the bust-ups and over the static `0x8c` had just put
   on layer 0 (band 0x10); `0x82` then sets it to 64/256. So the red lies
   over the static (red × ¼ + static × ¾ over black), where ours made it
   the background under the static. Now a slot-1 fill on layer 3 or 4 is
   a black background plus `screenfilter 4` colour at the `0x82`
   strength, removed by the next `0x54`; checked headless: a sprite at
   alpha 128 under `screenfilter 4,1,0,64,255,0,0` reads (63, 95, 0)
   for green, as the original's maths. The depths themselves (0, 2000)
   are a row of the harmless table. Not filmed: whether the original's
   band 0x20 also covers the text window (ours: under it, as for the
   `0x70` filters).
41. **Done: one `0x86` byte** (2026-10-16). The one byte was the last of
   `0043.cd` #440, the slide with a rotation: the rotation group (0x100)
   of its last key frame is i32 angle, u8 interpolation, and the
   converter read the angle only. The byte is 2 (linear), which is how
   `Op::SpriteTurn` turns the sprite; it is now read, and a turn that
   isn't linear would no longer be drawn turning. `0.txt` is unchanged.
45. **Done: `0x86`'s last two bytes** (2026-10-16). `op_86` (`0x413540`)
   reads the layer, the depth and the slot, and `0x413350` hands the
   record from +9 to the new module's load with a 0x400-byte budget.
   The key-frame motion's load (`0x416fe0`, through `0x40af80`) reads
   the u32 flags, the u16 count and each key frame's fields, then
   stops: the two zero bytes after the 9 slides' key frames are never
   read, a row of the harmless table.
44. **Done: `0x7a` u16, u16 at +14** (2026-10-16). `op_7a` (`0x414590`)
   passes them to `FUN_0040f060`, which stores them as the object's
   width and height (+0x198, +0x19c): the size of the blank surface
   `0x40f380` makes for an object without a picture (+0x1a0 = -1, which
   `op_7a` sets), and the base of `0x7d`'s anchor (`0x40f220`: half or
   all of it). Once a picture is loaded (`0x84`, through `0x40f380`
   again), the object's size becomes the picture's (`FUN_0040f060` with
   the image's width and height). The 6 blog records give 0 × 0 and
   load a bust-up right after, before `0x7d` centres it: a row of the
   harmless table.
43. **Done: `0x46`'s last u32** (2026-10-16). `op_46` (`0x477460`)
   builds the link on its stack and `0x492f00` copies it from the
   record's +14 on, so the u32 is the stored entry's +0. The rest of the
   entry: +4/+6 row and column - 1, +8 length, +0xc the record's +0xa,
   +0x10 the style (+6), +0x14/+0x18 an 8-byte descriptor of the label
   (file, label). Readers: the save (`0x493290`) writes all the fields
   and the load reads them back. The drawing (`0x493140` →
   `0x48e500`) passes row and column, length, +0xc, the callback
   `0x492bb0` and the entry pointer. The callback reads only the
   label (+0x14/+0x18) to pick the seen/unseen colour, and for the
   hover/click messages 3 and 4 (sent by `0x4991f3` and others through
   the text field's handler `0x48f5d0`) it returns the entry pointer,
   which `0x48f5d0` stores at the text field's +0x1a0. Nothing in the
   text code (`0x480000`-`0x4a0000`) reads that +0x1a0 back. So no
   reader of entry +0 turned up besides save and load (one bounded pass;
   0 in all 214 links anyway): a row of the harmless table.
31. **Done: effect depth** (2026-10-16). `0x8c`'s second u32 is the
   effect object's sort key, layer | depth << 8 as in `0x70`: always
   layer 0 (band 0x10, the bust-ups' band). The effects set are at depth
   1000 (120), 1100 (2), 2000 (2 ripples) and 500 (one static, 0011.cd
   #8534); 0 only in clearing records. All are over every bust-up
   (10-150), where ours draws the static (sprite 60) and the ripples and
   shakes (on the whole picture). The static at 500 would sort under a
   0x55 overlay at 1000, but none comes before it is cleared (#8542);
   the static at 1100 (0008.cd #3904) is over its bust-ups (10-20) as at
   1000. So the key is a row of the harmless table.
30. **Done: screen filter depth** (2026-10-16). `0x70`'s second u32 is
   layer | depth << 8: the filter object sorts in its layer's band
   (layer 1 → band 0 with the background and the frame overlays, 0/2 →
   0x10 with the bust-ups at depth 10-150 and the 0x55 overlays and the
   static at 1000, 3 → 0x20, 4 → 0x30), and what sorts above it is drawn
   unfiltered. Of the 253 filters set, 9 sit under every bust-up: band
   0x10 at depth 0 (0008.cd's blur and darkening, 5; 0010.cd, 0012.cd,
   0079.cd) and band 0 (0009.cd #5080, a blur). Those now use
   CONscripter's new `screenfilter` z (engine #56; the slot applies right
   after sprite z is drawn) as z = 999, the lowest sprite: the
   background and the overlay under the bust-ups are filtered, the
   bust-ups, the overlay over them, the static and the animations not
   (`Op::ScreenFilterUnderSprites`). The rest sort over every bust-up
   (depth 500-2000, or bands 0x20/0x30) and filter the whole picture as
   before. Left: the two negative flashes at depth 80 (0007.cd #3918,
   0011.cd #171) filter all the bust-ups, where those deeper than 80 (one
   at 100 in 0011.cd, the 1500 one in 0007.cd) stay unfiltered in the
   original; and objects made later at the filter's own depth (a 0x55
   overlay or the static at 1000) sort over it there, ours under it.
   Checked: the dry run (9 `screenfilter` lines gain `,999`), and a
   headless test of the engine change.
29. **Done in part: edge colours** (2026-10-16). `0x49` (+12) and
   `0x4a` (+6) end with an edge colour, decoded by `0x491e30`:
   `0xff000000` becomes -1, `0xff000001` the default edge, `0xff000002`
   the current one, any other value itself. `FUN_00492000` sets a
   character range's edge flags (0x30) only for a value other than -1.
   So 564 of the 565 `0x49` records (`0xff000000`) give no edge of
   their own, and `0x4a`'s two (0011.cd #5052: `0xff000000` for the four
   orange one-character pages, then `0xff000001`) the default: all drawn
   with the message's usual decoration, a row of the harmless table.
   Left: one `0x49` record with `0x808080`, a grey outline on its
   characters, which message text can't draw (Status item 29).
28. **Done: link style 18** (2026-10-16). `0x46`'s u32 at +6 isn't a
   style: `op_46` checks it as a scenario variable (bit 15 = global,
   `0x4673f0`/`0x45ff30`) and keeps it at the link entry's +0x10. When
   the text field adds the link (`0x48e500`), it asks the link callback
   `0x492bb0` (message 0xb), which answers 1 only if the target scene
   isn't open already (`0x464870`) and the variable isn't 0; on 0 the
   word is never made a link and stays plain text. The 205 links use
   global 7, which the boot sets to 1 (0000.cd #13) and the scene
   replay's set-up to 0 (0019.cd, 回想初期化); the 9 others use global
   18, which says the second part is playing (0000.cd sets 0x8011,
   0x8012, 0x8013 for parts one to three; 0015.cd, あずさ's replay, sets
   18 too): 6 words in 0009.cd and 3 in 0013.cd are links only in the
   second part. Now each link page is preceded by `if %2NN == 0 goto`
   a copy of the page without links (`sN_plainM`). So the scene replay
   shows no links, as the original; the variable stays a note-free read.
   Not checked in the game: the second part's links (dry run only).
42. **Done: the debug menu's message block** (2026-10-16). The extra
   60 bytes after `0001.cd` #1, the debug menu's one message, belong to
   a menu that stays out (the lead's call, left to it by the user): a
   row of the harmless table.

33. **Done: ruby flags** (2026-10-16). `op_44` maps the u16 at +6 to
   0-3 and the ruby goes to the text field with it (| 0x10, `0x494e20` →
   `0x48b3f0`, item +6). The layout (`0x48cb50` → `0x48a0a0`) reads it
   as the alignment over the base: 0 at the start, 1 centred, 2 at the
   end, 3 spread (the slack divided among the reading's characters,
   half a share first; when the reading is longer than the base, the
   centred case). 32 rubies are 3, which is how `rubyon` lays out a
   short reading (`FontInfo::initRuby`); the 5 that are 1 are a single
   reading character or longer than their base, where centred and
   spread come out the same. The u32 after it (0 in all) goes to the
   ruby entry's +0, which the drawing (`0x494e20`) doesn't pass on. So
   both are a row of the harmless table. (Not compared: how a reading
   longer than its base pushes the next characters; `rubyon` widens
   the base.)
