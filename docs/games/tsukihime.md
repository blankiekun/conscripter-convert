# 月姫 -A piece of blue glass moon- (TYPE-MOON)

The 2021 remake of 月姫, Nintendo Switch release (title ID
`01001DC01486A000`). TYPE-MOON's own engine, the family of the
魔法使いの夜 remake. Reader and compiler: `crates/conv-engines/src/typemoon/`;
engine notes: `docs/engines/typemoon.md`.

## Scope

The user owns the Switch release and dumped it (NSP plus their
console's `prod.keys`). Asked for: the whole game with every extra, on
every platform, in Japanese (the release) and English (the
Tsukihimates fan patch, `tsukihimates_patch_2023.09.05.5c86a2e06.nsp`).
Nothing has been dropped.

## Build

Get the RomFS and ExeFS out with
[hactool](https://github.com/SciresM/hactool). hactool 1.4 stops at a
key whose value isn't a multiple of 32 hex digits; filter the keys
first (`awk -F' *= *' 'NF==2 && length($2)%32==0' prod.keys`). The
Program NCA is the 22 GB one.

```sh
hactool -k prod.keys -t pfs0 --pfs0dir=nsp <base .nsp>
# official update 1.0.2 (title-key crypto: the key is 16 bytes at 0x180 of its .tik)
hactool -k prod.keys -t pfs0 --pfs0dir=upd <1.0.2 .nsp>
hactool -k prod.keys --titlekey=$(xxd -p -s 0x180 -l 16 upd/*.tik) \
  --basenca=nsp/<base Program NCA> --exefsdir=<game-102>/../exefs \
  --romfsdir=<game-102> upd/<update Program NCA>
# fan patch (an update NSP, title 01001DC01486A800, BKTR over the base)
hactool -k prod.keys -t pfs0 --pfs0dir=patch <patch .nsp>
hactool -k prod.keys --basenca=nsp/<base Program NCA> \
  --exefsdir=<game-en>/../exefs --romfsdir=<game-en> patch/<patch Program NCA>
```

Convert Japanese from `<game-102>` (base + 1.0.2) and English from
`<game-en>` (the fan patch is 1.0.2 plus the translation). The exe is
read from `<RomFS>/../exefs/main` (`--exe` elsewhere); the base game's
`main` has its tables elsewhere and is refused.

```sh
conscripter-convert build <game-102> -o out/tsukihime-cons            # Japanese
conscripter-convert build <game-en> --japanese <game-102> -o out/tsukihime-cons  # both
# the same, by hand:
conscripter-convert typemoon convert <game-102> [--english <game-en>] -o out/tsukihime-cons \
  --font <ttf> [--webp | --webp-quality 85] [--audio-codec opus | --audio-quality 4] [--dry-run] [--loose]
conscripter-convert typemoon media <game-102> [--english <game-en>] -o out/tsukihime-cons  # media only, ~15 min
```

`build` picks the title (`月姫 -A piece of blue glass moon-` or
`Tsukihime -A piece of blue glass moon-`). The media folder: every
`allpac` picture once from its full art (the `.JPG` of a bare
background, else the larger of a name's two `.NXGZ`) as
`pic/<name>.png|webp`, the gallery's 1,023 `allpaccg` CGs as `cg/`,
the 98 `RULE_*` masks always PNG; movies remuxed, not re-encoded, as
`movie/<name>.mp4`; `stream/*.ogg` (the game's Opus repacked, loops as
tags) and wave sounds as `se/*.wav` (loops as `smpl`) or `se/*.ogg`.
Pictures and movies already written are kept, so a stopped run goes
on. With `--english`, the fan patch's pictures and movies that differ
go to `pic/en/`, `cg/en/`, `movie/en/`. Sound is the same in
both copies and comes from `<game-102>`.

## Executables

The Ghidra addresses in this document are 1.0.2's.

| File | Version | sha256 |
|---|---|---|
| `main` (NSO), base game | 1.0.0 | `6e41c7094303a48b3223147b0fe52a2995c48f59af9d480b7e523ed4a1f103c7` |
| `main` (NSO), official update | 1.0.2 | `c128d79d3abd02b5581348618aa6403fdd651e77f4955f99ae43b34b90395faa` |
| `main` (NSO), fan patch | 2023.09.05 | `d28cb7c2391e74b2e3e2add120c208933aa314724efd2e47bfdf6c5c13aa6174` |

The fan patch's `main` differs from 1.0.2's by 65 bytes and reads the
same tables. The ExeFS also holds `subsdk0`/`subsdk1`.

## The exe

Decompile: `research/tools/ghidra/nso2elf.py <main> <main.elf>` (an ELF
with the dynamic section, loaded at 0x100000), then
`research/tools/ghidra/ghidra_export.sh <main.elf> research/typemoon/local/ghidra
--names research/typemoon/names.tsv -- -postScript CodePointerFunctions.java`
(5 min, 5,284 functions; without the post-script 1,420 are missed).
Names and addresses: `research/typemoon/names.tsv`.
`typemoon/exe.rs` reads the tables from the NSO (`nso.rs`: three
LZ4 segments); `conscripter-convert typemoon exe <exefs-102>/main
[--romfs <game-102>] [--motion m,…]` prints counts, checks every
picture's record against its texture, prints a motion's keys.

- **Commands**: `script_command` (0x177790) switches on the letter
  after `_`, then the next three packed into an int (`0x504c59` =
  `PLY`), so no command name is a string in the exe. Arguments are
  expressions (`FUN_00102d3c`), at most 20 (`FUN_00102b24`).
- **Picture table**: 14,713 names of 32 bytes at 0x6E02A4 (empty name
  ends it): plain colours (`BLACK` …), `IMG*`, sprites. A name's
  picture is `<name>.NXGZ` in `allpac`. Beside each, a 0x48-byte
  record at 0x56F830: f32 width 0x2C, height 0x30 (half the texture's
  width, 724/1440 of its height: record units on a 1280 × 724 stage);
  f32 centre 0x34/0x38 (`_STCH` puts it at x, y; `FUN_00155d30`);
  byte 0x41 bit 0 opaque (3 for plain colours); byte 0x43 1 for an
  expression; u16 0x46 a sprite's shot (`_A_` 0 … `_S_` 3), which
  scales a child's offsets ×1, ½, ¼, ⅛ (`FUN_0012dfd0`). A bare pose's
  record is its expressions' box less their padding (3,091 of 3,092;
  not `IMG1481`); when `_STCH` swaps an expression it moves the layer
  by the difference of the two records' centres (`FUN_00160840`).
- **Sprites** are whole pictures, one per expression, not atlases:
  `<pose>_<a>_<b>` from `_STCH`'s arguments 9 and 10; the bare
  `<pose>` is a small copy.
- **The stage** (draw list `FUN_001884f0`, draw `FUN_00189ad0`):
  script coordinates are 480 × 272 for the screen; on screen x, y and
  sizes are script × 8/3 and record units on 1280 × 724. A
  background's texture fills the screen at zoom 1000. Settled from the
  gallery, not the decompile: `research/typemoon/cg_framing.py`
  matches 147 of 235 16:9 gallery CGs with story pictures whole (none
  by their middle half). Plain colours fill the screen. A child
  layer's place is its parent's plus the parent's zoom times its
  offset (`FUN_001892b0`, `FUN_00189420`). Draw scale: `_STZ4` ‰ ×
  `_STCH` arg 4 ‰ · W/320.
- **Layers**: 40, staged by the commands below, shown by `_FADS`.
  - `_STCH` (`FUN_00160840`): 0 layer; 1 picture (empty clears); 2, 3
    x, y (left out: kept); 4 zoom ‰; 5 alpha; 6 priority (default layer
    × 10); 7, 8 never read; 9, 10 the expression (`_a_b` added unless
    both 0); 11 parent layer (−1 none); 12–17 shorts (a clip?); 18 a
    layer to copy every setting from; 19 rotation in 1/100°.
  - `_STCC(n)` clears layer n (none: 0, negative: all) and its priority
    to 0. `_STCP(layer, p)` sets the priority.
  - `_STZ4(layer, x0, y0, x1, y1, ms, e, f)` (`FUN_00162d20`): zoom ‰
    over ms·60/1000 frames, easing `e` (`FUN_00130c80`); `f` = byte
    0xA2, the zoom goes only to the shaders (inferred: for blurs).
  - `_STRT` turns in 1/100° clockwise. `_STBR(layer, n)`
    (`FUN_00162490`) picks a pixel shader via table 0x44DD70 into the
    56 names at 0xBD2308 (0 default, 7 monotone, 8 negative, 11 gauss,
    18 add, 19 multi, 23 lighten, 24 screen, 31 overlay, …).
    `_STEC` effect 1 tints (multiplied), 0 ends it.
  - `_STMA(layer, m, mode, p)` (`FUN_00199620`): motion m (0 stops).
    `mode` 1 hides the text window until the next text; 2 hides it
    while settings byte 99 is 1 (its default).
  - `_FADS(type, ms, rule, mode)` (`FUN_00160610`): type 0 cross-fade,
    2 wipe through `RULE_<rule>` (`FUN_0014b4c0`), 1 a running mask
    returning at once.
- **Motion table**: `_STMA` motion m (1–24,192; run by
  `FUN_00197b80`). u32 at 0x4B1940 + 4(m−1): one past its last key
  (71,905 keys); u32 at 0x499F40 + 4(m−1): length in ms; u64 at
  0x46AB38 + 8m: the fields it drives (x 0x40880, y 0x5100000, alpha
  0x4000, zoom 0x80200000/0x100400000, turn 0x88000, priority 1);
  `FUN_001999a0(m)` its loop point (0 none, 1 start, else ms; over
  9,859 a jump table). Keys, 0x40 bytes at 0x75CE18 (`FUN_00199ff0`):
  u32 time ms; s16 0x04 easing (0–41, `FUN_001961b0`), 0x06 mode (2
  hold then jump, 1 curve `FUN_001972c0`), 0x08 x, 0x0A y, 0x0C alpha,
  i32 0x14 turn in 1/100°, 0x18/0x1A zoom ‰, 0x2C–0x30 effect colour,
  0x32/0x34 expression, 0x3C priority; others unknown. −32767 is
  "unchanged", −32766 "no key here" (the field runs straight on
  between its neighbours, `FUN_00197710`). A key's way (easing, mode)
  is set by the last key before it that sets one. Values are absolute.
- **Scene numbers**: `_JUMP(n[,label])` goes by scene number, 551 s16
  at 0x450D58 in `allscr` order (`START_SCRIPT` 110, `00_00` 240,
  `QA_01`–`QA_25` 1000–1024, `QACALL` 3100, `JUMP00`–`JUMPQA`
  8001–8017).
- **Music**: `cmd_MPLY` (0x1649e0) plays `_MPLY(n)` as the bfsar's
  sound n − 1 (table of 132 u32 at 0x433F90 via `FUN_001006f0`;
  sounds 0–130 are the music streams); `n < 0x418` sets heard-music
  flag `n + 0xC80`. Options (three digits as decimal): 1 fade-in, 11 a
  second fade time, 17 volume, 69 start offset. Two players (0x3c-byte
  slots, `sound_update` 0x19b7f0): without option 1 a new track cuts
  the old (`music_stop_now`); with it the two cross-fade. Loops are in
  `knife1.bfsar` (below), not the exe.
- **Sound effects**: `_SEPL(channel, loop, sound, …)`
  (`FUN_0016a340`) replaces the channel's sound, except the same sound
  already looping there goes on.
- **Text**: `_WNTY(n)` (`FUN_00138ea0`) the window type, 0 or 1 (the
  drawer `FUN_001391a0` offsets a line by 3.5 or 5.5). `_WTTM(ms)`
  waits ms·3/50 frames.
- **Story movies**: `_TOMP(n)` plays entry n of the table at 0x50B890
  (0x30 bytes): 4 prologue, 5–6 openings, 8–0x22 day cards, 0x23–0x25
  endings, 0x26 `TM_CI`, 0x27–0x2D credits. Seen bits
  (`FUN_00192400`) make some skippable.
- **Gallery** (`cGallery`, `FUN_001042d0`): 458 slots at 0x54EC88 (CG
  pointer, condition) over 1,977 CGs; 105 unlock entries at 0x5525C8
  (CG + up to 4 story pictures); CG flags are bits at 0x7A8. 926 CGs
  show only after flag 557. The Ending List (`FUN_0010d5e8`): 25 × 12
  bytes at 0x4374E8 (scene, `_RDST` flag, voice), the lessons. Other
  rows (`FUN_00107f38`): 1 `cGalleryMusic` (`FUN_00119518`), 2
  `cFlowchart` (`FUN_0010f8fc`, 495 `GALLERY_FC_TH*` thumbnails), 3
  `cGalleryMovie` (`FUN_00117cd0`).
- **Read flags**: `_RDST(n)` sets global flag 1200 + n
  (`FUN_00192630`); 560, 383, 562 choose the title's movie. Flag 485
  unread sends the title setup (`FUN_00190180`) to the first launch's
  prologue (scene 0x104).

## Files and formats

The RomFS, 21 GB, 16,729 files:

| Files | What |
|---|---|
| `allpac.{hed,mrg,nam}` (16 GB) | Main archive: `.hed` index, `.mrg` data, `.nam` 32-byte names (`.NXGZ`, `.NXZ`, `.JPG`, a few `.MZP`/`.PNG`) |
| `allpaccg`, `allpachd`, `allpacml`, `allpachdml` | Same triple: gallery CGs; `allpachd` holds `MASK_TOP.MZP` and the `RULE_*` masks |
| `allui.{hed,mrg,nam}` (71 MB) | UI pictures (not yet in the media folder) |
| `allscr.mrg` | Scripts (an MZP, below) |
| `script_text.mrg` | Text: MZP of 5 pairs (big-endian u32 offset table, UTF-8 lines); pair 0 is the game's, the rest blank |
| `stream/*.opus` (16,601) | Voices and music, Nintendo's Opus framing (`nxopus.rs`): 48 kHz, pre-skip 120 |
| `movie/*.mp4` (46) | H.264 1920 × 1080 + AAC |
| `knife1.bfsar` (386 MB) | Nintendo sound archive (`bfsar.rs`, `fwav.rs`): 1,297 wave sounds `WSD_*` (DSP ADPCM; 1,300 in the patch) and the stream sounds `STRM_*` naming `stream/` |
| `ja*/`, `shader/`, `exfont_nx01.bffnt` | Fonts, pixel shaders, Switch font |

Formats (all checked over both copies by `conscripter-convert typemoon
check <game>`, 8 min; `typemoon list|extract [--png] <archive>`):

- **MZP** (`mrgd00`): u16 count, 8-byte entries `u16 sector, u16 byte,
  u16 size_sectors, u16 size_low`, sector 0x800; size =
  `(size_sectors*0x800 & ~0xFFFF) | size_low`.
- **MZX0**: LZ-like on 16-bit words (`mzx.rs`). The fan patch's
  scripts (mangetsu's compressor) leave the last literal chunk a byte
  short; accepted when it still fills the size.
- **NXGX/NXCX**: 16-byte header, then gzip/deflate, inflating to
  **BNTX** (`bntx.rs`): `BRTI` textures, one layer and mip,
  block-linear swizzled (GOBs of 64 B × 8 rows, `2^n` high). BC7 sRGB
  (`0x2006`) mostly, BC4 (`0x1D01`) for `RULE_*`, RGBA8 (`0x0B06`) for
  some UI. BC blocks via `bcdec_rs`.
- **Music loops**: a stream sound's INFO (kind 0x2201) holds at +0x1C
  a reference (0x2210) to 12 bytes: u8 type (3 Opus), u8 loop flag,
  2 bytes, u32 loop start, u32 loop end (samples after pre-skip); end
  − 1 is taken. All 131 music streams loop; vgmstream agrees on the
  wave sounds' loops.

## Script language

Read by `typemoon/script.rs`. `conscripter-convert typemoon script
<romfs> [-o <dir>]` ranks the commands, checks every text and sound
reference, and with `-o` dumps each script, one command a line with its
text (`_SELR(1,$042614)  « ……部屋で外の様子をうかがう。 »`).

- **`allscr.mrg`**: entry 0 the script names (32 bytes each); entry 1
  an MZP of label tables, one per script (CRLF lines `LBL_000
  016,0002e`, the second number a byte offset); entry 2 two small
  tables, unread; entry 3 + i script i, MZX-compressed. 549 scripts
  (553 entries in the patch, the last empty).
- **A script** is one ASCII line of `_NAME(arg,…);`. Text is a
  reference: `$000107` is line 107 of `script_text.mrg`. `_Z?`
  commands carry a 5-hex-digit id (`_ZM01602`). Arguments after a
  backquote are options `` `001:4000`` (number:value).
- **Text markup**: ruby `<base|ruby>`, `@g`/`@b`/`@k`/`@t` line codes,
  inline pictures `[ber00]`/`[zap00]`, `\n`. 43,961 lines, the same
  numbering in both languages.
- **Variables**: `F…`, `G…`, `C…` + number; `_WKST(F823,eq,0)` sets,
  `_WKAD` adds, `_IF__(F026,ne,0,LV00000)` skips to local label
  `_ZY…(LV00000)` unless it holds. `_JUMP(scene,label)` (see "The
  exe"), `_ZZ` labels, `_RTM_` returns, `_SELR(i,$text[,1 greyed])`
  options then `_SELD`/`_GOTS`.
- **Sound**: `_VPLY(<stream name>,<id>)`; `_SEPL(channel, loop, <WSD_
  name less prefix>, …)` (7 Japanese uses name `MIX_SE007`–`012`,
  which no copy has); `_MPLY(n, …)` by number.
- **Malformed commands**: `_STTR,0,26)` in 1.0.2's
  `14_01A_CIEL14A_2` (fixed in the patch).
- **Versions**: 1.0.2 changes 521 scripts (adds `_END_()`, `_SPFC`);
  the fan patch changes 213, keeps every text reference in order,
  turns 303 `_ZM` into `_MSAD`, adds `_NOP_`.

## Story structure and walkthrough

### Walkthrough

From the 1.0.2 scripts (the English copy's flow is the same), checked
against NookGaming's walkthrough
(<https://nookgaming.com/tsukihime-a-piece-of-blue-glass-moon-walkthrough-guide/>):

```sh
conscripter-convert typemoon script <game-102> -o <dump>
conscripter-convert typemoon extract <game-102>/allscr.mrg -o <raw>
python3 -I research/typemoon/routes.py <dump> <raw>/00000.bin <main-102.elf>
```

**Layout.** `START_SCRIPT` → `00_00` (prologue), then per day
`NN_00_ARCNN_*` (Arcueid, 「月姫」, days 1–14) and `NN_01_CIELNN_*`
(Ciel, 「夜の虹」, days 1–14, then `15_01A`/`15_01B`). Days 1–3 are
common; shared scenes branch on `F058`. 82 story choices.
`JUMP00`–`JUMP15`, `ARCHIVE`, `JUMPQA` are the chapter select (scene
number in `G008`); `*_FLAG0`/`*_FLAG1` set `F069` and jump on.

**The first play is Arcueid's route.** Ciel's opens on day 3, greyed
until Arcueid's ending is seen (`G003`):

1. Ciel's points, days 1–2 (`F050` → `F075` → `F076` → `F077`): day 1
   「ちょっとした疑問がわいてしまった。」 or 「せっかくなのでじっくり観賞するしかなかった。」
   (1); lunch 「廊下に出てから考えよう。」 (+1, only with the first);
   after school 「学校をぶらつく。」 (+1 with the first; sets `F054`);
   day 2 lunch 「とりあえず、廊下に出て考えよう。」 (+1, only with `F054`).
2. Day 3 lunch (`03_00_ARC03_4`): at `F077` ≥ 4 a third option
   「茶道室に行ってみようか……？」 sets `F057`.
3. Day 3 night (`03_00_ARC03_5B`): 「……いや、俺の起こした現実だ。」 needs
   `F057` = 1 and `G003` = 1; it goes to `03_01_CIEL03_3_2`; day 4
   (`04_01_CIEL04_1`) sets `F058`.

Day 7's 「……二十七祖ってなんだ？」 (`07_00_ARC07_3_1`) is greyed until
`G003`.

**Endings.**

| Ending | Last scenes | Sets | Then |
|---|---|---|---|
| Arcueid | `13_00_ARC13_6` → `14_00_ARC14_1` | `G003` | lesson `QA_12` |
| Ciel, normal | day 14 `14_01_CIEL14_1C`: 「……アルクェイドには従えない。」 → `14_01A_*` → `15_01A_CIEL15A_1` | `G004` | `QA_23` |
| Ciel, extra | the same choice: 「……アルクェイドを、嫌いにはなれない。」 → `14_01B_*` → `15_01B_CIEL15B_1` | | `QA_24` |

The extra option needs `G004` = 1 and `F081` ≥ 5. `F081` is Ciel
route's Arcueid points (`F051` → `F078` → … → `F081`): +1 day 4
「ホテルに戻ろうと思う。」, +2 day 8 「アルクェイドの方が、ほっとけない。」, +1 day 9
always (`09_01_CIEL09_4_1`), +1 day 9 「『それ以上進んだら嫌いになるからなっ！』」.
`QA_25` is reached only from the Gallery's Ending List once the other
24 lessons are read (`_RDED`).

**Dead ends.** 21, each followed by a 「教えて！シエル先生」 lesson
(`_JUMP(3100,QA_nn)` into `QACALL`).

| Lesson | Day | Scene | Reached by |
|---|---|---|---|
| `QA_01` | 4 | `04_00_ARC04_6_1` | 「……酔狂はここまでだ。逃げよう。」, then 「屋敷に戻ろうと思う。」 |
| `QA_02` | 4 | `04_00_ARC04_9_2` | 「……部屋で外の様子をうかがう。」 |
| `QA_03` | 5 | `05_00_ARC05_3_1` | 「……今すぐヴローヴを捜すべきだ。」 with `F058` = 0 |
| `QA_04` | 5 | `05_00_ARC05_6_1` | 「下りていく。」 |
| `QA_05` | 5 | `05_00_ARC05_9_1` | 「……投擲だ。」 |
| `QA_06` | 5 | `05_00_ARC05_9_3` | 「……牽引だ。」 |
| `QA_07` | 8 | `08_00_ARC08_7_2` | 「……嘘をついて誤魔化す。」 (`F060` = 0) |
| `QA_08` | 9 | `09_00_ARC09_3_2_1` | 「……槙久の書斎を調べてみる。」 |
| `QA_09` | 9 | `10_00_ARC10LIVING` | 「アルクェイドに会いに行く。」 |
| `QA_10` | 10 | `10_00_ARC10_7_3` | 「……学校に戻っているかも？」 |
| `QA_11` | 10 | `10_00_ARC10_8_1` | 「ただ、欲しいだけなんだ。」 |
| `QA_13` | 5 | `05_01_CIEL05_2_1` | 「―――このまま押し切る！」 |
| `QA_14` | 5 | `05_01_CIEL05_2_2_1` | 「先輩に駆け寄る。」 |
| `QA_15` | 10 | `10_01_CIEL10_5_2` | 「自分は足手まといだ。ここで待っていよう。」, or 「問題はない。話は放課後にすればいいし。」 before it |
| `QA_16` | 11 | `11_01_CIEL11_2_2` | day 9's 「じゃあ、痛くて効かないものを。」 (`F061`) |
| `QA_17` | 13 | `13_01_01_CIEL13_3_2` | 「後方に逃げる。」 |
| `QA_18` | 13 | `13_01_01_CIEL13_4_1` | 「……眼鏡を外す。」; forced when `F069` = 1 (day 9's 「……因果応報だ。女を手本にして処理しよう。」, `F063`) |
| `QA_19` | 13 | `13_01_02_CIEL13_5_1` | 「最短で、あの吸血鬼を殺す。」 |
| `QA_20` | 13 | `13_01_02_CIEL13_6_1` | 「面倒だ。殺してしまえ。」 |
| `QA_21` | 14 (extra) | `14_01B_02_CIEL14B_3_1` | 「諦めない。」 |
| `QA_22` | 14 (extra) | `14_01B_03_CIEL14B_4_2` | 「もう一度上へ。何か見落としがある筈だ。」 |

Other conditional options: day 2's 「ファミレスに行ってみる。」 (`F056`, from
「パンを買って教室で済ませよう。」); day 10's questions to Ciel
(`10_01_CIEL10_3_2`), each gone once asked (`F065`–`F068`).

## Progression variables

`G…` are presumably system data, `F…` save data. The many `G0nn |= 2`
are gallery unlocks, not mapped.

| Variable | Meaning | Set | Read |
|---|---|---|---|
| `G003` | Arcueid's ending seen | `14_00_ARC14_1` | day 3 Ciel option, day 7 option |
| `G004` | Ciel's normal ending seen | `15_01A_CIEL15A_1` | day 14 extra option |
| `F050`, `F075`–`F077` | Ciel's points, days 1–2 | walkthrough | `F077` ≥ 4 on day 3 |
| `F054` | took 「学校をぶらつく。」 on day 1 | `01_01_CIEL01_3_3` | day 2 lunch |
| `F057` | went to the tea room on day 3 | `03_01_CIEL03_2_3` | day 3 night |
| `F058` | on Ciel's route | `04_01_CIEL04_1`, `04_01_CIEL04_1C` | shared scenes |
| `F051`, `F078`–`F081` | Arcueid points | "Endings" | `F081` ≥ 5 on day 14 |
| `F060` | told the truth on day 8 | `08_00_ARC08_5_1` | `08_00_ARC08_7` |
| `F061` | 「痛くて効かないもの」 on day 9 | `09_01_CIEL09_1_2` | `11_01_CIEL11_1D` |
| `F063` | 「因果応報」 on day 9 | `09_01_CIEL09_3_1` | `10_01_CIEL10_8`, `12_01_CIEL12_1`, day 13 |
| `F069` | day 12–13 variant (from `F063`) | `12_01_CIEL12_1_A`/`_B`, `*_FLAG*` | days 12–13, the glasses choice |
| `G008` | scene number for the chapter select | `JUMP*`, `QACALL` | `JUMP*` |

## Testing

No manual (the NSP's `Manual` NCA is legal text only).

**Route runs in ours**: the three endings and 21 dead ends, headless,
in one save folder so the globals carry over (run `arcueid`,
`ciel-normal`, `ciel-extra` in that order; each dead end alone with
`--goto tm_<scene>` and `button tm_QACALL 2`). All reach their lesson
and the title.

```sh
python3 -I research/typemoon/route_choose.py <out>/0.txt <out>/text.ja.txt \
  research/typemoon/routes/<route>.txt > <route>.choose
conscripter --root <out> --save-dir <save> --auto-play 0 --trace <route>.jsonl \
  --goto tm_START_SCRIPT --until conv_title $(sed 's/^/--choose /' <route>.choose)
```

**The original under Ryubing** (flatpak `io.github.ryubing.Ryujinx`,
`--no-gui`): boots to the title at full speed. Run it only through
`research/typemoon/switch-emu.sh` (own Xvfb, no Wayland, its own data
dir; setup: firmware NCAs into
`bis/system/Contents/registered/<id>.nca/00`, `prod.keys` and
`title.keys` in `system/`, 1.0.2 via
`games/01001dc01486a000/updates.json`). The firmware must not need a
newer master key than the keys (22.5.0 works). The data dir needs
`bis/user` from a first boot. Inputs: `KEYS="40:z"` (z = A), `IDLEA=1`
(auto mode; `0.3` fast), `AVOID="<$ ids>"` (skip those options at a
choice). Ryubing holds 5.5 GB.

**Command trace of the original**: an ExeFS mod logs every command
(`research/typemoon/switchtrace/`: `hook.s` hooks `script_command`'s
loop head 0x177828 and writes `TMT <script> <offset> <command>` through
`svcOutputDebugString`; `mkmain.py` builds the patched `main` into
`<emu>/data/mods/contents/01001dc01486a000/tmtrace/exefs/main`;
`tmtrace.py` turns Ryubing's log into `trace_diff.py` records):

```sh
clang --target=aarch64-none-elf -c research/typemoon/switchtrace/hook.s -o hook.o
ld.lld -Ttext=0x2542c0 --defsym=SCRIPT_INDEX_PTR=0xbe04f0 --defsym=SCRIPT_BUF=0x1473e04 \
  --defsym=RETURN=0x17782c -e hook hook.o -o hook.elf && llvm-objcopy -O binary hook.elf hook.bin
python3 -I research/typemoon/switchtrace/mkmain.py <exefs-102>/main hook.bin <mod>/exefs/main
SHOT=60 IDLEA=1 research/typemoon/switch-emu.sh <emu>/data 780 --graphics-backend Vulkan \
  --skip-user-profiles-manager --disable-ptc "<base nsp>"
python3 -I research/typemoon/switchtrace/tmtrace.py <emu>/logs/ryu.log <scr> <txt> -o orig.jsonl
```

`<scr>`, `<txt>`: `typemoon extract` of `allscr.mrg`, `script_text.mrg`.
Ours: a `--dry-run --source-map map.tsv` build traced with `--trace --real-time`, then `trace_diff.py orig.jsonl ours.jsonl --map map.tsv --ms --merge-into _FADS`; over `00_00`'s first 297 pages the text matches.

## Milestones

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-16 | - |
| First dry run | - | - |
| Story plays | 2026-10-16 (opening) | - |
| Title, save, settings | 2026-10-17; settings 2026-10-18 | - |
| Extras | 2026-10-18 | - |
| Progression verified | 2026-10-19 (3 endings, 21 dead ends, in ours) | - |
| Finished | - | - |

## Status

Paused 2026-10-20.

What works: extraction from the NSP; every archive, picture (BNTX) and
sound (with loops) of both copies; the exe's tables (`typemoon exe`);
`typemoon media`; the scripts convert and play from the prologue to all
three endings and 21 lessons (flow, text, pictures, layers, motions,
zooms, shaders, sound, choices, the 45 story movies); boot logos,
title, Save/Load, Config, the Gallery's CG gallery and Ending List;
`--english` builds both languages in one `0.txt`, switched in game.

Missing:

- Layer commands not converted (in `tsukihime.findings.tsv`, open
  rows): quakes `_SQK2`/`_SHK2`/`_SSW2`, `_STPV`, `_SNX2`/`_SNX3`,
  `_STNS`/`_STRA`/`_STRR`/`_SACL`/`_SBAS`/`_SOXY`, `_VCWT`, `_VPL2`,
  `_SESP`, `_STTR`, `_STCL`, `_SGMA`, `_SPMA`; motion keys' priority,
  expression and stepped fields.
- Gallery's Music Player, Movie and Flowchart (drawn, not open); the CG
  viewer's zoom and pan; `allui` not in the media folder.
- Backlog; several Config rows drawn without effect; the save list's
  route picture and chapter labels.
- The first launch's prologue (ours goes straight to the title).
- Never played through in the web build.

### Next

1. To resume: rebuild (`conscripter-convert build <game-102> -o
   out/tsukihime-cons`) and play it in the web build; fix what blocks
   a play-through before anything in "Missing".
