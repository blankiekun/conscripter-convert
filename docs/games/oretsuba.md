# 俺たちに翼はない (PS Vita)

The third game, started 2026-10-13. This is the game's committed
record: formats, how its features map to CONscripter, the routes and
progression variables, and its status. The general process is in
`docs/new-game.md`, general lessons in `PLAN.md` §13. `<pkg>` is the
folder holding the game's Vita package and its license.

- **Status:** the whole story plays, with title, menus, the CG
  gallery, the music room and the movie room (see "Status" below). Branch `oretsuba`.
- **Scope (2026-10-13):** the **PS Vita version**, the
  whole game **with extras**. Nothing dropped yet. Menus (2026-10-14):
  the Vita's look with mouse buttons of our own.
- **Engine:** 5pb./MAGES' "TaskList" ADV engine on CRI middleware
  (CPK archives, ADX/HCA audio, Sofdec USM movies). Scripts are plain
  Shift-JIS text, one scene per file: a list of tasks
  (`:Tasktype "Message"`, `"SpriteBG"`, …) with key/value parameters.
  The eboot is decrypted (below) for the few tables the data lacks:
  so far the music room's track list.

## Source

- `俺たちに翼はない.pkg` (3,267,708,688 bytes): content ID
  `JP0745-PCSG00299_00-ORETSUBA0000VITA`, title ID PCSG00299, APP_VER
  03.010, SDK 3.00, built 2014-02-06 (Prototype's port).
- `work.bin` (512 bytes): the license (RIF) for the same content ID.

### Decrypting the package

Two layers: the package (AES-CTR) and the PFS file encryption.

1. zRIF from the license: `python3 rif2zrif.py work.bin` (in
   github.com/lusid1/pkg2zip).
2. `pkg2zip -x 俺たちに翼はない.pkg <zRIF>` (pkg2zip built from source
   with `make`) writes `app/PCSG00299/`, still PFS-encrypted.
3. PFS: Vita3K's fork of psvpfsparser (github.com/Vita3K/psvpfstools
   with its submodules) has the F00D step built in, so no web service
   is needed (`cma.henkaku.xyz`, which the original tool calls, is
   gone). Build the library with cmake (`-DCMAKE_POLICY_VERSION_MINIMUM=3.5`)
   and link a three-line `main` that calls
   `execute(zrif, src, dst, F00DEncryptorTypes::native, arg)` (the
   fork's example `main` only parses options):
   `pfsdec app/PCSG00299 PCSG00299_dec <zRIF>`. It reports
   "keystone: matched retail hmac", and `media/SCRIPT.cpk` then starts
   with `CPK `.

`<game>` below is the decrypted folder (`PCSG00299_dec`).

### The eboot

`<game>/eboot.bin` is still an NPDRM SELF after the steps above.
`research/oretsuba/self2elf.py` decrypts it to an ARM ELF the way
Vita3K's `decrypt_fself` does (klicensee from `work.bin` at 0x50,
key revision 5); it reads the keys from a Vita3K checkout
(`vita3k/packages/src/sce_utils.cpp`) rather than keeping them:

    python3 -I self2elf.py <game>/eboot.bin work.bin <Vita3K>/vita3k/packages/src/sce_utils.cpp local/eboot.elf
    python3 disasm.py local/eboot.elf > local/eboot.asm   # needs capstone

**Decompiled with Ghidra** (`docs/new-game.md` §2), with the
[VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) extension
(the release built for the installed Ghidra, unzipped into Ghidra's user
`Extensions` folder; 1.09 for Ghidra 12.1.2):

    research/tools/ghidra/ghidra_export.sh research/oretsuba/local/eboot.elf \
        research/oretsuba/local/ghidra --names research/oretsuba/names.tsv \
        -- -loader ArmElfPrxLoader -postScript ThumbPointerFunctions.java

About 4 minutes the first time; later runs reuse the project and only
apply `names.tsv` and export (2 minutes). The stock ELF loader also takes
the SCE executable (type 0xfe04, `-processor ARM:LE:32:v7`) and finds the
Thumb code, but leaves the imports unnamed; VitaLoaderRedux names the
ones in its NID list (`sceKernel…`, `sceSaveDataDialog…`; libc calls stay
`SceLibc_<NID>`). Analysis puts the C++ virtual methods, reached only
through vtables, in no function (the eye catch's update at 0x81048b8a);
`ThumbPointerFunctions.java` makes one at every Thumb pointer stored as
data (618), 4,922 functions in all, 83% of the code (the rest is mostly
literal pools and rodata). The decompiled C shows the ASCII strings
inline (`"TaskList/Task%d/Tasktype"`), so grep `decomp/` for them;
`strings.tsv` only lists cp932 ones. Every function identified goes into
`research/oretsuba/names.tsv`, and the export is rerun.

The ELF loads its code at 0x81000000 (file offset 0xe0) and its data at
0x8115e000 (file offset 0x15d440). `disasm.py` is a linear Thumb-2
listing with every movw/movt address resolved and the string it points
at, still handy for grepping the raw instructions. The rodata holds
the script engine's keys (`TaskList/Task%d/...`, `CGMODE/...`) and its
task names, the kanji of the font's glyph table, and no menu text: the
system screens are pictures.

Found so far:
- Music room: an int32 table at 0x8116310c, 67 BGM.AWB IDs (0-10,
  13-15, 20-31, 40-43, 45, 47-51, 53, 60-64, 46, 70, 71, 73-78, 80-83,
  85, 52, 86-95), in the order of `mu_bg.png`'s titles. The code steps
  an index 0-0x42 over it (7 rows shown) and plays `table[i]` on the
  BGM player. A second table follows it (34, 35, 29-33, 36), stepped
  0-7 with 4 rows shown; it has no titles in the picture.
- Movie room: 100-byte name records at 0x81163348 (OP00.usm, OP01,
  OP02, OP03, then OP00.sfd, OP02.sfd). The room plays record
  `cursor` (0-3) when the cursor's unlock byte is set; `mv_bg.png`'s
  thumbnails are CS Opening, Opening, Interlude and Prelude (OP00's
  pictures are the Vita's opening, OP03's the camcorder prelude).
- Task handlers: the name → number function at 0x81035xxx (InOut
  0x82, InOut_Act 0x83, EffectStart 0x3e); the task switch is the `tbh`
  at 0x8102119a (InOut_Act at 0x81022f9a). Effects are objects made by
  0x8103f75a (effect number − 1 into a `tbh`); InOut_Act is effect 25
  with `:Type` + 10, whose class switch (0x81041812) makes one object
  per type. Types 0, 4, 5, 7, 8 draw the old and new screens as two
  quads (positions lerped by the progress, tables built in each init);
  1, 2, 3, 6 load a vertex and a fragment GXP (fragment: `decal`,
  `decal2`, `value` = progress and the `:Param`s) at 0x8115f63c,
  0x8115fb80, 0x8115fe60, 0x81160510 (735-1711 bytes).
- Shaders: `research/oretsuba/gxp.py` cuts every GXP program out of the
  ELF; `research/oretsuba/gxp2glsl/` builds Vita3K's shader recompiler
  on its own (a sparse Vita3K checkout plus glslang and SPIRV-Cross at
  its submodule commits; the CMake file says how) and prints a shader
  as GLSL. The fragment shaders read `value` as (progress, `:Param` 0,
  1, 2), after each type's init has scaled them (type 2 divides
  `:Param` 1 by 720).
- EyeCatch: the task is number 0x6f; `:Image` type 0 makes the object
  at vtable 0x8115c59c (two half-screen sprites, y −136 → 136 and
  680 → 408 as the progress goes 0 → ½, back after), type 1 the one
  at 0x8115c5bc (the chapter card, 0x81048c68; alpha 0 → 100 → 0).
  Both are drawn from SystemData by ID, not by name.
- SceneJumpDialog: task 0x84 (handler 0x810265b4: Yes goes to
  `:JumpNextTag`, No to `:NextTag`, an unread scene or the setting off
  to `:UnreadTag`). The dialog's class (vtable 0x8115befc) lays out
  SystemData 62 + `:BG` 0 at 0x8103e7d6 and sets its cells at 0x8103e248
  (rectangles of the atlas, centred at a point): the screen (0, 0,
  960×544); the question (606, 746, 302×39) at (480, 184); the date n
  (n % 6 × 138, 545 + n / 6 × 21, 138×20) at (129, 48); the emblem e
  (158e, 839, 158×173) at (164, 406); up to 10 faces (id % 25 × 37,
  671 + id / 25 × 37, 37×37), the last at (858, 60), each one before it
  38 to the left; Yes and No (425, 746 / 768, 60×22, the selected one
  60 to the right) at (415, 241) and (545, 241). It draws (0x8103ebf2)
  two strings in white, lines 27 apart: one at (81, 99), the summary at
  (300, 319). It fades over 15 frames.
- EffectStart 40: the object made at 0x8103f8f4 (constructor
  0x810427b2) draws the screen once through vertex GXP 0x81161628 and
  fragment GXP 0x8115f304 with `value` = (frames/30, `:Param` 0,
  `:Param` 1, the effect's fade level, +0x4c, which the effect base
  steps from 0 to 1 and back in states 6 and 7).

### Files (`<game>/media/`)

| File | Format | Contents |
|---|---|---|
| `SCRIPT.cpk` | CPK, TOC (names + IDs) | 499 scripts: `adv.txt` (engine setup: start scenes, flags, windows, character list), `scn.txt` (scene numbers), `chrNNN.txt` (character sprite tables), `chract`, `CGMODE`, 494 scenes (`S*`, `P*`, `M*`) |
| `CG.cpk` | CPK, TOC | 946 PNGs: backgrounds, event CGs (`BG_BLACK` is ID 0) |
| `CHR.cpk` | CPK, TOC | 1,333 PNG standing pictures (`ST_*`) |
| `ETC.cpk` | CPK, TOC | 124 PNGs: transition rules (`RULE000`…), effects |
| `SystemData.cpk` | CPK, TOC | 182 PNGs: menus, windows, logos |
| `BGM.AWB` | CPK, ITOC only (IDs) | 96 ADX tracks, 48 kHz stereo, loop points in the header |
| `SE.AWB`, `SystemSE.AWB` | CPK, ITOC only | 701 + 2 ADX sound effects |
| `VOICE.AWB` | AFS2 | 26,500 HCA voices (22,050 Hz mono, not encrypted) |
| `data/*.usm` | Sofdec2 USM | OP00-03, Poem, 7 heroine endings |

No file is CRILAYLA-compressed (`ExtractSize` = `FileSize`
everywhere). ffmpeg decodes both ADX and HCA. `research/oretsuba/cpk.py`
lists and extracts all three containers.

### Script references

Every number in a task is a file ID in one of the archives, which the
scripts' own comments confirm (`:Texture 0 // BG_BLACK`, `:Voice 22562
// TAK1004`): all 4,000+ commented texture numbers match CG.cpk's IDs.

- `:Texture N` (Sprite, SpriteBG, SpriteBS, …): CG.cpk ID N.
- `:Number c,pose,dress,face,size` (CharacterOpen): character `c` is the
  `chrNNN.txt` whose `:CharacterNumber` is `c`; its `[SpriteN]` with
  `:Character pose,dress,face,size` gives the CHR.cpk ID (`:Texture`).
- `:Voice N`: VOICE.AWB ID N. `:SE N`: SE.AWB ID N. BGM: BGM.AWB ID.
- Coordinates are 1280×720 with the origin at the top left;
  `:Position` is the picture's centre. Times are frames at 60 fps.

### Tasks (use counts over the 494 scenes)

Message 65,526 · CharacterOpen 16,954 · SePlay 4,739 · SeStop 2,884 ·
SpriteBG 2,843 · CharacterAction 2,312 · CharacterActionEnd 2,212 ·
BgmStop 1,672 · Window 1,570 · BgmPlay 1,564 · SpriteBS 1,415 ·
CharacterDelete 1,414 · Tag 1,195 · EffectQuake 1,077 · InOut 975 ·
CharacterOpenBS 895 · Sprite 850 · EyeCatch/EyeCatchEnd 847 ·
CharacterMove 838 · CharacterWait 651 · FadeOut 590 · FadeIn 585 ·
NextScene 537 · SceneJumpDialog/End 447/531 · End 469 · Flash 461 ·
InOut_Act 443 · TimeWait 406 · SpriteMove 389 · SpriteDeleteBG 278 ·
EffectStart 208 · SpriteMoveEnd 171 · EffectEnd 136 · Jump_Tag 104 ·
SpriteQuake 87 · SpriteDelete 72 · FlagSwitch_Tag 61 · SpritePriority 59 ·
BacklogMessage 56 · CharacterPriority 45 · SetSceneName 42 ·
MessageSwitch_Tag 25 · GlobalFlagSwitch_Tag 21 · GlobalFlagOperation 19 ·
MoviePlay 13 · FlagOperation 13 · Achievement 11 · MovieStop 7 ·
CharacterQuake 5 · CharacterStop 2 · VoicePlay/VoiceStop 1.

Control flow: `Tag` (`:Name`) is a label; `Jump_Tag` jumps to one;
`FlagSwitch_Tag`/`GlobalFlagSwitch_Tag` (`:FlagName`, `:Operator`,
`:Param`, `:NextTag`) jump on a condition; `FlagOperation`/
`GlobalFlagOperation` (`:Action "=",1`) set flags; `MessageSwitch_Tag`
is a choice (`:AnswerMessageNumberN "<tag>","<text>"`, `:MessageNFlag
"<flag>","+",1` adds points); `NextScene` (`:NextScene "S01_01A"`) goes to
another file. `SceneJumpDialog` offers to skip a scene already read
(`:JumpNextTag`/`:NextTag`). Flags are declared in `adv.txt`: 33 per-save
`[Flag]`s (affection points such as `asu_pts`, route switches) and 30
`[GlobalFlag]`s (endings seen such as `asuka_ed`, `two_clear`).

## Walkthrough

Source: the scripts themselves (no public choice walkthrough was found;
the route map below is read from every scene's NextScene, flag switches
and choices, `conscripter-convert mages check` parses them). The game
(the Vita port of 俺たちに翼はない R plus 林田美咲's route) follows one
person's four personalities:

1. **Part 1, 羽田鷹志** (S01_01 … S01_05): choices give `asu_pts`,
   `miy_pts`, `miya_flag`. If at S01_04O `miya_flag` ≠ 0, `asu_pts` ≥ 2
   and `miy_pts` ≥ 1, `one_second` = 1 (part 1's second half opens).
2. **Part 2, 千歳鷲介** (S02_01A … S02_10): `hiy_pts`; at S02_06H, if
   `one_second` = 0 and `hiy_pts` ≥ 4, `two_second` = 1.
3. **Part 3, 成田隼人** (S03_01A …): if part 1's or part 2's second half
   opened, `kyoutsu_rt` = 1 (the common route) and part 3 is short;
   otherwise its choices give `nar_pts`, `nar_sister`, and S03C_01D
   (黙る) opens 亜衣's route (`ai_rt`). S03_07A1 sets `three_second`.
4. **Part 4, ヨージ** (S04_01A … S04_06J1). At S04_06J1 the flags pick
   the second half:
   - `one_second` → S01_05A: 明日香 (S01_16A, `asuka_ed`) or 京
     (`miya_clear` and `miy_pts` ≥ 2 at S01_10F, S01_18A, `miyako_ed`);
     at S01_14I, `asu_pts` < 4 → 美咲's route (M01_01 … M09_02,
     `misaki_ed`). S01_19A sets `one_clear`.
   - `two_second` → S02_10A: 日和子 (S02_16A, `hiyoko_ed`); S02_17A
     sets `two_clear`.
   - `three_second` → S03_16A: 鳴 (S03_22A, `naru_ed`); `ai_rt` →
     S03C_06A: 亜衣 (S03C_20A, `ai_ed`); S03_23A sets `three_clear`.
   - none → S04_06L, a bad end.
5. **True ending, 小鳩**: with `one_clear`, `two_clear` and
   `three_clear` set, S04_05I offers 救いをもたらす → S04_06J2 …
   S04_28B (`kobato_ed`), S04_29A (`four_clear`).
6. **After-stories** P01 (明日香), P02 (京), P03 (美咲), P04 (かるお):
   start scenes 1-4 of adv.txt, extras of the title menu; each sets
   `Pre_<name>` when finished.

### Route check

`research/oretsuba/nstrace.py` runs the converted `0.txt`'s control
flow along choice paths (globals kept from run to run, as when playing
the routes in a row) and reports the ending scene. Paths from the route
map above, in one session (2026-10-13, all as expected):

    python3 -I research/oretsuba/nstrace.py out/oretsuba-cons/0.txt --quiet \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:1,S01_08D4:2,S01_09C0:1,=s01_19a" \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D0:3,S01_08D4:1,S01_09C0:2,S01_12B:1,S01_13B:1,=s01_19a" \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D4:2,S01_09C0:3,=m09_02" \
      "=s02_17a" \
      "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:1,=s03_24" \
      "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:2,=s04_06l" \
      "S02_05A:2,=s03_24" \
      "S04_05I:2,=s04_29a"

| Run | Ending |
|---|---|
| 1 | 明日香 (S01_16A) |
| 2 | 京 (S01_18A) |
| 3 | 美咲 (M09_01) |
| 4 | 日和子 (S02_16A), every first option |
| 5 | 亜衣 (S03C_20A): 鳴's points kept under 2 so part 3 joins the common route; then the hint (S03_24), as parts 1 and 2 are cleared |
| 6 | the bad end (S04_06L) |
| 7 | 鳴 (S03_22A); with parts 1-3 cleared, S03_23A goes on to the ヨージ hint (S03_24) |
| 8 | 小鳩, the true ending (S04_28B, S04_29A), offered only after the three clears |

Not yet: a source-side simulator to diff page by page (step 11.4).

## Progression variables

Declared in adv.txt: 33 per-save `[Flag]`s (`%60`-`%92` in the
output, in adv.txt's order) and 30 `[GlobalFlag]`s, kept across
playthroughs (`%410`-`%439`).

| Flag | Meaning (adv.txt's comment) | Set | Read |
|---|---|---|---|
| asu_pts, miy_pts, hiy_pts, nar_pts | affection points | choices (`:MessageNFlag`) | S01_04O, S01_10F, S01_14I, S02_06H, S03_05C |
| miya_flag, miya_clear, miya_h, miya_hflag | 京's route switches and counters | S01 choices, S01_10G1 | S01_04A/K/O, S01_10F, S01_11A … S01_14G |
| one_second, two_second, three_second | the part's second half opens | S01_04O, S02_06H, S03_07A1 | S02_06H, S02_09F, S02_10, S04_06J1/K |
| kyoutsu_rt | the common route (part 3 shortened) | S02_10, S03_05C | S03_*, S04_01A, S04_06J1 |
| ai_rt, nar_sister, HIY25 | 亜衣's route, 鳴's sister, 日和子 | S03C_01D, S03_03D/S03_05C, S02_15C | S03C_*, S04_*, S03C_02B |
| asuka_ed … misaki_ed, kobato_ed (global) | ending seen | the ending scenes | the extras; S01_19A, S02_17A, S03_23A |
| one_clear, two_clear, three_clear, four_clear (global) | part cleared | S01_19A, S02_17A, S03_23A, S04_29A | S04_05I (true ending), the `NEXT_HINT` messages |
| Pre_asuka, Pre_miyako, Pre_misaki, Pre_karuo (global) | after-story read | P01_01D, P02_01E, P03_01D, P04_02B | not in the scenes (the title menu) |

## Status: the whole story plays (2026-10-15)

`crates/conv-engines/src/mages/` reads the archives (`cpk.rs`), the
scripts (`tasks.rs`), lowers scenes to the IR (`convert.rs`) and writes
the assets (`assets.rs`; sounds become Ogg Vorbis through ffmpeg, BGM
with its ADX loop as `LOOPSTART`/`LOOPEND`):

    conscripter-convert mages list <game>/media/CG.cpk
    conscripter-convert mages extract <game>/media/SCRIPT.cpk -o <dir>
    conscripter-convert mages check <game>
    conscripter-convert mages convert <game> -o out/oretsuba-cons \
        --font <ipag.ttf> [--scenes S01_01A,S01_02A]

`--webp` (and `--webp-quality N`) stores the pictures as WebP, for the
web build (S01_01A's 57 pictures: 3.9 MB as PNG, 1.7 MB near-lossless);
`--dry-run` writes `0.txt` only. The other options are in the README.

`mages check` parses all 463 scenes (65,485 messages) and finds every
texture, voice, SE and BGM ID they use in the archives.

Output: 960×544, the Vita's screen. Script coordinates (1280×720) are
scaled by 3/4 across and 544/720 down; CG and CHR pictures are already
at that scale, and so are SystemData's (the screens are 960×544): the
text window is `meswin.png`'s left 960 pixels (two loose feathers
follow) drawn 1:1 above the button bar, its text four lines of 30 at
23 pixels in x 120-840, y 40-144 of the window, enough for the longest
message (109 characters with the name; there is no paging).

| Feature | State |
|---|---|
| Messages, speaker names (from the scripts' `:Name N // name` comments), voices | converted |
| Backgrounds and sprites (`SpriteBG`, `Sprite`, `SpriteBS`, deletes), by priority | converted: `SpriteBG` and `SpriteBS` replace every picture (a background, a black cover, a white flash, an event CG, in one fade), the characters too unless `:SpriteDelete 0` (the scripts never delete the characters a location change leaves, and after 37 of the 0s they delete them themselves); a scene's first picture also removes whatever the scene before left (the eboot loads each scene's textures anew). `Sprite` adds one, and `SpriteDelete`/`SpriteDeleteBG` remove the picture of their `:SpriteNumber` (a texture). Our reading of the scripts; positions not yet checked against the original |
| Characters (`CharacterOpen`, `…BS`, `CharacterDelete`, `CharacterMove`) and `SpritePriority`/`CharacterPriority` | converted. At y 0 a character stands on the screen's bottom edge (1209 of the 1333 pictures, 630 pixels tall, are cut off along their bottom row); the eboot scales `OffsetY` (-68 for everyone standing) by 544/720, but centred that leaves them 8 pixels above the edge, so where it anchors them is still to be found. Shorter pictures (the 960×540 cut-ins) are centred plus `OffsetY`. CharacterMove's y counts from that height. `CharacterDelete` without a list (`:SpriteMax 0`, 962 of 1414) removes every character; `CharacterOpenBS` removes the characters it doesn't name (the scripts re-open or delete-all them afterwards, and only twice delete one by name). S04_16B's `:Number <ST_ASU02H>>` (a placeholder the script's tool never filled in) is resolved by the picture's name through the character tables (Asuka 0,0,2,8,0) |
| `:Color` tints (dusk 255,236,220 on 3,892 pictures, night 220,222,255 on 2,105), `:Grayscale` (479), with `:Gamma` the sepia flashbacks (188), `:Floor`/`:Ceil` film negatives (42), `:Color` alpha 0 | converted as sprite filters: a tint multiplies (`spfilter` colour, multiply), sepia is a monotone with each channel of the grey raised to 1/gamma as an offset at mid-grey (+75, +28, −47), and a picture with filters fades in inside a transition, filtered. Alpha 0 shows a character see-through; a CharacterMove that changes the alpha cross-fades to the new place and opacity over its `:Time` |
| `InOut` transitions with the RULE masks | converted |
| `InOut_Act` (the eboot's effect 25: `:Type` picks the class, `:Param` 0 mostly a direction 0-3 = right, down, left, up) | 0 (the old screen squeezes to that edge while the new one grows from the other; 21 uses) and 7 (a push; 89) as ONScripter's scrolls; 5 (to or from white or black; 9) through white; 2 (a mosaic growing to `:Param` 1 pixels of 720 halfway while the screens cross-fade; 66) as ONScripter's mosaic out and in; 3 (a card turning over in perspective, old in front, new behind, about the vertical axis for `:Param` 0 below 2; 47) as `cons_flip`, the new screen turning in. 1 (a fade with the rows waving sideways, `:Param` 1-2 the wave; 57) as CONscripter's `cons_wave.dll` (the size and wave length a guess: the shader's constants are uniforms, `:Param` 1 × 100 per mille of the screen, `:Param` 2 waves down it); 6 (a fade blurred sideways, the most halfway, by `:Param` 1 pixels of 1280, `:Param` 0 = 1 only the new screen; 130) as `cons_blur.dll`; 4 (2) a plain fade |
| FadeOut/FadeIn, TimeWait, BGM, SE, Window hide | converted |
| Tags, jumps, flag switches and operations, choices with points, NextScene | converted, not yet checked against a walkthrough |
| SceneJumpDialog (skip a scene already read) | converted: a global "read" flag per scene (`%1200` + the scene's index in name order). A read scene shows the eboot's dialog (layout below): SystemData's `ss_bg1`…`ss_bg5` by `:BG` 0 (the protagonist), このシーンをスキップしますか？, the date (`:BG` 2), the emblem (`:BG` 1), the `:CharaIcon` faces, the summary (`:Text`) drawn with the `--font`, and Yes (skip) / No buttons that glow under the pointer. The line the eboot also writes at (81, 99) is left empty (its text isn't in the task; perhaps the chapter's name). The original asks only when its setting is on, and so does ours (シーンスキップ機能 in the settings). Without `--font`, the summary and two text choices |
| Flash; EffectQuake/End (a screen shake dying down over `:Time`); SpriteQuake (one picture, a background, shaken by `:Param` 0, 1 dying down over `:Time`; all zeros stop it; `:Time 0` with a size, once in S04_07A after a `// wt` comment, waits for the shake before it, which the message in between outlasts: nothing); SpriteMove (always waited on) | converted; `:Scale` (90 uses) zooms the picture about its centre with CONscripter's `spzoom` (after 0.19.1), which keeps it in its place among the sprites; the zoom stays for the moves after it. The two values after `:Position` are the eboot's easing (ease_value, 0x8103cc18: type 1 in, 2 out, 3 in and out, raising t² to the power): converted for moves and zooms (IR `MoveEase`; conv-cons's eased loops in integer arithmetic, the power × 2 a whole exponent), 65 of them; the rest are straight |
| CharacterAction (`chract`'s 661 keyframed moves: bows, shakes, bobbing; 72 uses loop until CharacterActionEnd) and CharacterActionEnd | converted with CONscripter's `spmotion` (after 0.19.1): offsets from the character's place, straight between keyframes, run by the engine while the text goes on; a loop goes back to its first key at the action's `:Frame` length. `:EndWait 1` waits for it (64 uses; 1,902 don't wait). CharacterActionEnd stops a loop, or with `:EndWait 1` waits for an action started since the last message. SpriteQuake is a one-shot `spmotion` on its picture too. The eboot (0x81008008) reads each field of a frame (`:X`, `:Y`, `:Roll`, `:Scale`, `:Alpha`, `:Priority`) as a track of its own, keyed where it is given, each key with an easing; we sample the tracks at every key frame, straight between keys (one action eases a key; `:Priority` is not used). Actions with `:Scale` (zoom about the centre, 60 uses), `:Roll` (radians; turned clockwise on the screen, our guess; 5 uses) or `:Alpha` (60 uses) are CONscripter's `spmotion2`. `:EndSet 1` (read only when the action doesn't loop) leaves the character where the last key has it: 475 uses end with the character moved, faded out or zoomed (Asuka stepping aside and fading out in M02_04, Kinako rising into view in S02_09D1), made with `spmotion2`'s loop mode 2; a move, a new picture or another action on the character first ends it there (so skipping lands right) |
| EyeCatch `:Image 0,N` (CG.cpk ID N + 457, at least 458, as the eboot's 0x81048978 loads it: `EC01`-`EC19` for 1-19, but the archive has no `EC20` or `EC30`, so 20 is `EC21` (京's after-story) and 27, the common one, `EC28`, the framed title logo) | converted as in the eboot (0x81048b8a): the picture's top and bottom halves (960×272) close from above and below like shutters (500 ms, our guess) and open again at EyeCatchEnd over `:Fade`, showing the FadeOut's colour (or the scene) behind; made as 10-frame animations (`system/shutters_N_in/out.png`). `:Image 0,-1` (two panels of a colour in the eboot, 7 uses) shows nothing |
| EyeCatch `:Image 1,card,place,date` (114 uses: the chapter cards) | converted like `:Image 0`, as one screen made from SystemData: the card `ec1_bg`…`ec6_bg` (ID 67 + card: 0-3 the four protagonists' chapters, 4-5 Karura's, 6 奈落の底) with the place (ID 74 + place: `ec_bgNN`, its times of day as A/B/C) in its frame and the date (0 = 11月21日, -1 none) cut from the calendar under the card. The original fades the three in and out and pans the place 40 pixels while it fades in; drawn where the pan ends |
| MoviePlay: the USM movies (MPEG-1 960×544 + ADX) demuxed (`usm.rs`) and made H.264/AAC MP4s in `movie/` (loose) | converted; `:Movie` N is the Nth of OP00, OP01, OP02, the seven `ED_*` (alphabetical), Poem, OP03 (each heroine's ending scene plays her ED) |
| EffectStart/End 53 (ノイズ: static at `:Param` density) and 42 (ブラー: a blur screen filter); SetSceneName (chapter title) | converted |
| EffectStart/End 57 (画面キャプチャースプライト, 50 uses: a frozen copy of the screen zoomed onto a point at an impact; the eboot's class at 0x81040714: `:Param` x, y where its centre goes, scale, frames, ease type 0-3, power) | converted with CONscripter's `screenzoom` (after 0.19.1): the copy's scale goes from 1 to `:Param` 2 and its centre from the screen's to (x, y) over half of `:Param` 3's frames (the eboot steps 2/frames a frame), with the eboot's easing (0x8103cc18: straight, in, out, in and out, to a power); it fades in over `:Fade` and out over EffectEnd's `:Fade` (the slot cleared inside a transition). Scale 1 is a freeze frame |
| EffectStart/End 52 (光玉 light balls), 54 (紙吹雪 confetti), 51 (雪 snow) | converted to looping 16-frame animations made from ETC.cpk's `lightball.png`, `paper.png`, `snow.png` (`:Param` count): the balls twinkle and circle in place, snow and paper fall one screen per 2 s loop; the original's motion is not known |
| EffectStart/End 40 (a wave over the starry monologue background, `:Param 400,3000`: the eboot's shader 0x8115f304 moves each row's picture along the diagonal by 1.5% of the width × sin(2π(600y/(height × `:Param` 0) + 2t/`:Param` 1 ms)), growing in with the effect's fade; a negative `:Param` 0 waves the columns, unused) | converted as `screenripple` at 45° through a grey ramp (`system/waveN.png`, N/100 waves down the screen); EffectEnd stops it without its fade (2 uses of 30 frames) |
| Achievement (trophies) | nothing to show |
| Title screen (`title_bg.png`'s picture and labels: START, LOAD, GALLERY) | converted, the buttons where the eboot draws them (0x81083fd8: 173×36 cells centred at x 786, a row every 41.25 px from y 258 in the order QUICK LOAD, LOAD, START, GALLERY; the eboot places its menu pictures by their centres, as "Press START button", 258 wide at x 769, shows; we have no quick save, so its place stays empty); settings are in the extras and the in-game bar. `SystemSE.AWB`'s two sounds here and in every other menu and on the button bar (IR `Script::menu_sounds`): the tick (0, the eboot's cursor move) when the pointer comes onto a button (`selectvoice`), the confirm (1, 0x8105e7a6) on a click (CONscripter's `btnse`, after 0.22.0), both at システムボリューム (`sysevol`) |
| START's story menu: 始めから and the after-stories ある日の林田美咲/渡来明日香/山科京/軽部狩男 (P03/P01/P02/P04) | converted as the eboot has it (0x8105e69c, 0x81083086, 0x81084c26): START opens the menu once 明日香's, 京's, 亜衣's or 鳴's ending is seen or part 1 or 2 cleared (global flags 0-5; otherwise the story starts), and the three heroines' after-stories are then open; 狩男's needs part 1, 2 or 3 cleared (the Vita shows ？？？？？ until then). Each starts its adv.txt start scene directly. Drawn as the Vita draws it (0x81083086): over the title, `title_bg.png`'s 242×60 cells centred at x 788: 始めから at y 225, the 〜Prelude〜 header at 305, the after-stories at 365, 395, 425 and 455 (狩男's ？？？？？ until it opens), lit under the pointer; the buttons are the labels' 30 rows, so they don't overlap. The header stays unlit (the Vita lights it while the cursor is on an after-story). Without `--font` (no menus) a text choice |
| Button bar (Auto Skip Log Save Load Config Hide), save/load (3 slots × 10 pages), backlog, yes/no dialog | converted in the Vita's look, our layout (2026-10-14, the user's choice): SystemData's backgrounds (`save_bg`, `load_bg`, `bklog`), slot plates (`sl_data`), dialog frame (`dlg_bg_26`, the one without words), headers cut out where they differ from a header-less background; the Vita's controller hints and page box painted over; the bar's Log, Save, Load and Config are the Vita's own labels from `ss_bg1.png` (y 746, 60×24 cells: Q-Save, Q-Load, Return, Save, Load, Log, Config; rows white, glowing blue, navy, faded), given a dark halo where they cross the window's feather; Auto, Skip and Hide (no label in the game: `SKP1_ICON.png` only has the running indicators) are drawn with the `--font` in that style. Other labels are drawn with the `--font` in the game's style (white, blue halo; gold under the pointer). Backgrounds behind conv-cons's white words are darkened. Without `--font` there are no menus |
| Settings (CONFIG) | converted in the Vita's look and with its settings (2026-10-15, the user's choice), from SystemData's `bg_config_01.png` (基本設定) and `bg_config_02.png` (サウンド設定): each is the page with its rows' labels and options drawn in, over an atlas of the options lit (a glow and a cloud), a snowflake and the rows' cursor plates (`mages/config.rs`, IR `PagedSettings`, conv-cons `settings_pages.rs`). The option words are cut out of the page (the page minus the page with them painted over, kept near the lit version's letters) and lit with the atlas's glow when current; the bars are the page's own, emptied past the level, with the snowflake at its end, and ◀ ▶ beside them; ← L / R → switch pages. Page 1: シーンスキップ機能 ON/OFF (the scene jump dialog), 音声スキップ する/しない (the click stops the voice: CONscripter's `clickvoicestop`), スキップモード 既読のみ/全て, 選択後のスキップ 持続/解除 (skip and auto go on after a choice: `selectkeep`), オート速度 (ours, on the row of 選んだ選択肢の色を変更, which with 自動クイックセーブ is not converted and painted over), メッセージ速度 遅い/普通/早い/一括 (text speed 4, 7, 9, 10 of 10), ウインドウの不透明度. Page 2: Voice/BGM/SE/システム ボリューム (システム: the menus' sounds, `Setting::SystemVolume`), 音声再生時にBGM音量を下げる (to 50 % while a line is voiced), キャラクター音声 全てON/全てOFF (every voice is character 0, one switch). A third page in the same look holds our switches (mouse wheel, confirmations). The hints' place holds デフォルト設定 (every setting back to its default) and 戻る; from the game, タイトルに戻る and ゲームを終了 at the left. The defaults are the Vita's, from its デフォルト設定 (0x81070e70), in our steps of 10: Voice 85 (9), BGM 55 (6), SE 55 (6), BGM lowered for voices on (ours to 50 %, the Vita's to 55), every character voice on, メッセージ速度 普通, ウインドウの不透明度 70 % システムボリューム 55 (6) (IR `SettingsMenu::defaults`, `switches_off`) |
| Extras menu: the title's GALLERY shows ALBUM, MOVIE, MUSIC over the title, in its first three button rows, as the Vita does (0x81083c0c; `title_bg.png`'s labels), and ours adds CONFIG and 戻る in the two rows below (the Vita's has no CONFIG; a right click also goes back). The modes have the album's background (`ExtrasMenu::menu_background`). The CG gallery from `CGMODE` (8 heroine tabs from `gm_bg.png`'s cards, 4×5 thumbnails a page, up to 3 pages, a viewer with ◀ ▶ HIDE 戻る); scenes mark CGs seen from their `:CGFlag` | converted in the Vita's look, our layout |
| Music room: 67 songs from the eboot's table, their titles from `mu_bg.png`, and its player bar | converted, our layout: three columns of 23 titles at 3/5 size on the Vita's list panel (stretched under the player bar), the song playing in blue, every song open, as on the Vita (0x81065f02 has no lock check). The player bar as the eboot draws it (0x8106592a, 0x810655a4; IR `MusicPlayer`): ◁◁ and ▷▷ play the song before and after the one playing (none past the ends), ▷ stops it or plays the last one again and stays lit while a song plays (the lit cells under the titles in `mu_bg.png`), and the room's music volume (0x81065f02: the Vita's ← → change the BGM volume, 0x810664ca puts it back on leaving) is ◀ ▶ (by 10) or a click on the bar, shown by the snowflake knob at x 605 + volume × 203/100. A song loops until another is chosen: the room's repeat and play-on flags (+0x2c, +0x2d) come from a mode (+0x30) that only the constructor (0x8106551c) sets, to 0 (repeat on, play-on off), so the Vita never plays on to the next song (no engine support needed). The Vita's button hints are covered with the album's (`gm_bg.png`) snow, which has the same frame. The second 8-song table is left out: the Vita only plays it in a list mode nothing switches on (0x81066198). The large title of the song playing (the panel above the list on the Vita) is left out: our list fills that space |
| Movie room: OP00 (CS Opening), OP01 (Opening), OP02 (Interlude), OP03 (Prelude), the eboot's order, on `mv_bg.png`'s slots with its thumbnails (its frame under the pointer) | converted: a movie shows once a scene has played it (a global flag per movie, `%2800` + its place), OP00 always (no scene plays it; our rule). The Vita's button hints here and in the album are covered with the snow to their left, feathered in |

Every task of all 463 scenes converts (none left as a comment, and the
converter's "not converted" list is empty); the
full build (`mages convert` without `--scenes`, about 4 minutes, 1.7 GB
`arc.nsa` plus 12 MP4s in `movie/`) lints clean (one false positive:
the save menu's thumbnail name, built at run time), and the route check
reaches all eight endings.

**Checking a build with the debug trace** (`docs/new-game.md`, "Checking
a build with the debug trace"): in `out/oretsuba-cons`,
`SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy conscripter --root .
--save-dir <scratch> --trace t.jsonl --trace-ignore 34-48 --auto-play 0
--roundtrip-every 500`, then `engine/tools/cons_trace_check.py t.jsonl
--script 0.txt --config research/oretsuba/cons_trace_check.json`. 34-48
are the button bar; the title's buttons (6-8) must stay in, or auto-play
has nothing to press. The config lists the false positives looked at:
the cut-ins (`st_cin*`, 960×540 but mostly transparent: centred 2 px
above the bottom edge, and gone when moved out to x -220 though their
box isn't), the static and particle overlays (59, 60), impact and battle
sounds played several times a few hundred ms apart (as the source does),
the timed move and zoom loops, and characters faded out (alpha 0) before
the script deletes them. Route exploration: `engine/tools/cons_explore.py
out/oretsuba-cons --engine-args "--trace-ignore 34-48" --skip-choices
'^\*conv_(title|sl_|settings|confirm|extras|music|movie|cg|log|backlog|load|save|bar)'`.

The debug trace's first findings, all fixed, are in "History", below.

**Web build.** The game as loose files (the page fetches each file as
it needs it; one 1.9 GB `arc.nsa` would be fetched whole), the web
engine (`cd engine/script && EMSDK_HOME=<emsdk> bash ./cross_web.sh`) and
an index:

    conscripter-convert mages convert <game> -o out/oretsuba-web --font <ipag.ttf> --loose
    python3 -I engine/src/onsyuri_web/conscripter_index.py -i out/oretsuba-web \
        -o out/oretsuba-web/onsyuri_index.json --title oretsuba --lazyload
    cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/oretsuba-web/
    python3 -m http.server 8000 -d out/oretsuba-web   # then open /conscripter.html

2.2 GB, 27,472 files (index first, then the engine's files, so they
aren't listed). `mages convert` has no `--webp` yet (`convert-game`
has).

Tested in headless Chromium (2026-10-15, CONscripter 0.21.0, driven
over its DevTools port with `?trace`): the title in about a second, the
opening movies, the settings screen and its pages, skip, a choice, a
looping character action (S01_02E's bobbing, still running after a
load), a zoom (S01_04L's `spzoom`), saving and loading, also after
reloading the page. The test found five bugs, all fixed: the text read
as GBK (the web engine lost `ons_args`, so `--enc:sjis`), the settings,
read marks and unlocks lost when the tab closed (ONScripter writes them
only on quit or save), a click on the button bar during a choice
stopping the script (`select` took the bar's button number), the
settings' page arrows printing `0:goto *conv_settingsdraw` (`%3` for
modulo), and the slots' chapter names running past their plates.

**Steered route runs (2026-10-15).** `research/oretsuba/routes.py` plays
the route map's eight paths in the engine one after another in one save
folder (the global clears carry over), each with the debug trace and
save/load round trips: `--auto-choice untaken` with a `--taken` file that
marks every option but the path's `avoid`. Run it with a copy of the
engine under another name: other agents' `pkill -x conscripter` stops the
engine, and SDL turns that SIGTERM into a quit, so the trace just ends
(`{"t":"end"}` without a reason, exit 0) mid-scene. Each run replays the
route from the story menu after its ending until auto-play's loop guard
stops it ("3000 pages without a new label", exit 1): what matters is the
ending scene reached. Rerun on 2026-10-15's build (all eight in one
folder): 明日香 (S01_16A, page 25,742), 京 (S01_18A, 25,852), 美咲
(M09_01), 日和子 (S02_16A), 亜衣 (S03C_20A), the bad end (S04_06L), 鳴
(S03_22A) and 小鳩 (S04_28B) all reach their endings. 小鳩's run first
stopped for good in S04_12B: a CONscripter bug (fixed in 6837994,
`engine/ADDITIONS.md`): a `bgmfadeout 16:bgmstop` while `bgmfadein
1000` was still fading the music in left it playing, and the next `bgm`
waited forever. `cons_trace_check.py` on each finds no new bug: SEs
played on two channels at once (as the source does: S04_07B's two
arrows), backgrounds up for 150 pages in long scenes, characters placed
off screen before they move in (`spmotion`), `Sprite` pictures laid
over a background, the eased moves' loops (S04_03G's 20-second pans);
the dying `EffectQuake` shakes it reported as never ending were given
an end. Runs 9-12 of `routes.py` (`--only 9,10,11,12`, after the
routes, in the same folder) play the four after-stories from the story
menu: all four reach their last scene (P03_01D, P01_01D, P02_01E,
P04_02B) with nothing new in their traces (京's eye catch EC21, 540 px
tall, is centred, 2 px from each edge).

Next, in order:
1. Looks against the original: text window size and place, character
   heights (where the eboot anchors them), the eye catch's speed, the
   wave transition's size. Needs screenshots of the Vita game.

## History

Finished work, moved out of "Status" so a session reads only the open
state.

**Found by the debug trace (2026-10-14).** Whole routes play to their
end without a script error (18,974 pages and 16 choices; with today's
fixes another route of 23,333 pages and 46 save/load round trips, all
equal), then auto-play tours the title's menus. The checker found:
- **Pictures left from the scene before (fixed).** The eboot loads each
  scene's textures anew, but our scenes kept the previous one's
  pictures: 347 scenes end under a black `Sprite` cover, which then lay
  over the whole next scene (S01_02B black for 150 pages), and split
  scenes doubled their backgrounds and cut-ins. A scene's first picture
  (in each branch of its opening, skipped or not) now clears everything.
- **Characters through a change of place (fixed).** `SpriteBG` kept the
  characters, so Hiyoko from a flashback stood behind Kobato (S02_04A)
  and Kinako (S02_06A). The scripts show that `SpriteBG`/`SpriteBS`
  remove them unless `:SpriteDelete 0` (above).
- **Zooms sliding pictures off the screen (fixed).** `SpriteMove :Scale`
  moved the centre to the point zoomed in on without zooming (bg71 250
  px down over black in S04_05I); now a real zoom (`spzoom`).
- **Not converted, found on the way (now converted):** the `:Color`
  tints, greyscale, sepia and negatives (above).
