# 俺たちに翼はない (PS Vita)

The third game, started 2026-10-13. This is the game's committed
record: formats, how its features map to CONscripter, the routes and
progression variables, and its status. The general process is in
`docs/new-game.md`, general lessons in `docs/lessons.md`. `<pkg>` is the
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
`research/tools/ghidra/self2elf.py` decrypts it to an ARM ELF the way
Vita3K's `decrypt_fself` does (klicensee from `work.bin` at 0x50,
key revision 5); it reads the keys from a Vita3K checkout
(`vita3k/packages/src/sce_utils.cpp`) rather than keeping them:

    python3 -I ../tools/ghidra/self2elf.py <game>/eboot.bin work.bin <Vita3K>/vita3k/packages/src/sce_utils.cpp local/eboot.elf
    python3 disasm.py local/eboot.elf > local/eboot.asm   # needs capstone

**Decompiled with Ghidra** (`docs/new-game.md` §2), with the
[VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) extension
(the release built for the installed Ghidra, unzipped into Ghidra's user
`Extensions` folder; 1.09 for Ghidra 12.1.2):

    research/tools/ghidra/ghidra_export.sh <work>/eboot.elf <work>/ghidra \
        --names research/oretsuba/names.tsv \
        -- -loader ArmElfPrxLoader -postScript ThumbPointerFunctions.java

`<work>` is any folder outside the repository's tracked files (the
project and export are large and machine-local); `self2elf.py` only
needs `sce_utils.cpp` itself, which can be fetched alone from Vita3K's
repository (`vita3k/packages/src/sce_utils.cpp`).

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
  at 0x8115c5bc (the chapter card: set up at 0x81048c68, alpha
  0 → 100 → 0 at 0x81048ef2).
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

## Executables

The builds the decompile addresses in this document belong to.

| File | Version | sha256 |
|---|---|---|
| `eboot.bin` (NPDRM SELF; addresses are those of the decrypted `eboot.elf` made from it) | unknown (PCSG00299) | `56ebe9ed1a64d5e9e8d3af5aef0ce4ea21cf35ebffcf4c6cfae49dc6db343bd6` |

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

## Testing

Manual: `sce_sys/manual/001-010.png` in the decrypted app (Japanese; 8
used pages: contents, story, controls, title menu, start menu, system
menu, two config pages). Use it before asking for screenshots.

### The original under Vita3K

Step 1, the boot check (2026-10-17): **boots to the title**, drawn
right, at 30 frames a second by Vita3K's own counter (its performance
overlay, steady on the title and on the save dialog). The official
Vita3K AppImage (`Vita3K v0.2.1 4190-fab6bfc7`) runs it to the title;
past S01_01's start only a source build with a local patch (step 2). Run it only through `research/oretsuba/vita-emu.sh`, which
starts it on its own Xvfb with private XDG dirs.

Setup, in a folder of its own (`<emu>`, never `~/.local/share/Vita3K`
or `~/.config/Vita3K`, which the user plays with):

- `<Vita3K.AppImage> --appimage-extract` in `<emu>` gives
  `<emu>/AppDir`; the script starts `AppDir/bin/Vita3K` itself (not
  `AppRun`, whose hooks run a self-updater) and makes the links
  AppRun would (`/tmp/17-Tt`, `/tmp/8_P`, `bin/shaders-builtin`).
- The game: the decrypted package (`<game>`, "Decrypting the package")
  as `<emu>/home/data/Vita3K/Vita3K/ux0/app/PCSG00299` (`cp -al`, hard
  links, no copy), and its license, `work.bin`, as
  `ux0/license/PCSG00299/JP0745-PCSG00299_00-ORETSUBA0000VITA.rif`:
  without it the eboot fails (`No klic provided for encrypted App`).
- No firmware: the game boots without it. The save dialogs draw their
  text with the host's DejaVuSans (the script links `/usr/share/fonts`
  into the AppImage's font path); with no font at all the first save
  dialog ends the game.
- `<emu>/home/config/Vita3K/config.yml` (written by a first run):
  `backend-renderer: Vulkan` (OpenGL dies 1 s in, a null call, and
  the Qt window wants OpenGL 4.4, which Xvfb's GLX lacks; Vulkan runs on
  the host GPU), `show-welcome: false`, `warn-missing-firmware: false`;
  `performance-overlay: true` shows the frame counter.
- Run: `SHOT=10 KEYS="30:c 34:c" research/oretsuba/vita-emu.sh
  <emu>/home 60` (shots of the root window in `<emu>/logs`, the game
  in its top-left 960x544; log `<emu>/logs/vita3k.log`). Keys go to
  the game window by xdotool: `c` = circle, the OK of this Japanese
  game, `x` = cross, `Return` = start. The Qt front end takes them;
  `--frontend sdl` draws the game but ignores them.
- SDL connects to the desktop's Wayland (`$XDG_RUNTIME_DIR/wayland-0`)
  even with `WAYLAND_DISPLAY` unset: the script gives Vita3K an empty
  `XDG_RUNTIME_DIR` and `SDL_VIDEO_DRIVER=x11`.

What a boot does: CRIWARE, Navel and 5pb. logos (to about 22 s); on the
first boot "There is no saved data", then "Saving complete" (circle
each), on later ones "Loading complete" (circle, about 30 s in), then
the title ("Press START button") by 40 s. The title is still: the
counted distinct frames there are under 1 a second.

Firmware (2026-10-17): `vita-emu.sh` with `APP=` (no `-r`) and
`--firmware <PSVUPDAT.PUP>` installs 3.74 into `<emu>`'s `os0`/`vs0`
(7 MB, 105 MB). It changes little: Vita3K then loads libSceFt2, libpvf,
libnet*, np_* and libngs from it (LLE), and the title draws the same
(the "Press START" blink aside); the crash below happens with and without it.

Step 2, the trace (2026-10-17; runs since 2026-10-18 with a patched
source build of Vita3K): **the first 50 pages diff clean** (below).

- The record-to-instruction mapping, from the decompile: `task_start`
  (0x8100da30, r0 = the game context) starts every task. The context's
  current record is +0x4450, its scene stack +0x4440/+0x4444 (0x1f0-byte
  scene objects; empty: the scene at +0x424c). A scene object keeps the
  task numbers (the N of `[TaskN]`, one per task found, in file order) as
  a vector at +4, the records at +0x1c4, the current index at +0x1b8
  (0x81028204/0x81028280/0x810282d2: current, next, jump to tag). A
  record's first word is its task type (task_type_number: Message 0,
  MoviePlay 0x48, SceneJumpDialog 0x84, unknown types 0x7e); a Message's
  text (with every `:Message%d` appended) is at +0xb8. `scene_load`
  (0x8102840a) gets the scene object and its path (`app0:/media/script/S01_01`).
  Frames: 0x81000898 queues each frame (`sceGxmDisplayQueueAddEntry`).
- Vita3K's GDB stub is no use: it polls for a stop every 100 ms, a
  breakpoint in this build re-hits without stepping on, and the game hung
  after 64 stops. (With `gdbstub: true` the game also waits for a client
  before it starts.)
- The eboot patch works: `research/oretsuba/vpatch.py patch
  <emu>/home/data/Vita3K/patch/PCSG00299.txt` hooks the three functions
  (a `b.w` over each first push, to code in FUN_810a8c9e, which nothing
  calls) and `printf`s `@T scene task type`, `@M text`, `@S scene path`
  and `@F`; Vita3K logs them as `*** TTY:` lines (log level 0), and
  `vpatch.py log <emu>/logs/vita3k.log orig.jsonl` turns them into
  `trace_diff.py`'s "cmd"/"page" records, `at` = `S01_01:100`, `f` =
  frames (adding Vita3K's "Skipped N duplicate messages"). Checked: a run
  logs `@S … S01_01` and `@T … 100 84` (S01_01's Task100 is the
  SceneJumpDialog) and 30 `@F` a second.
- Our side: the converter now writes `--source-map` for this engine
  (each task's first 0.txt line → `<scene>:<task number>`, the same key).
- **The crash, found and worked around (2026-10-18)**: a source build
  of Vita3K (master 8084aa2, the code of 4191, RelWithDebInfo) under
  gdb stops in the renderer thread: `TextureCache::upload_texture`
  (`renderer/src/texture/cache.cpp`) → `VKTextureCache::upload_texture_impl`
  → memcpy of a 960×965 RGBA texture at guest 0x63580000. The game freed
  that CDRAM block (0x63580000, 0x3C0000 bytes) 300 ms before, as
  S01_01 starts, and allocated a smaller one inside it; a draw still
  names the old texture. On the Vita the GPU reads whatever is left
  there; Vita3K's unmapped pages fault (`Unhandled access to
  0x463780000` = host base 0x400000000 + guest 0x63780000). The local
  patch `research/oretsuba/vita3k-freed-texture.patch` (18 lines, not
  upstreamed) reads only the mapped part of a texture and zeros the
  rest, with a warning in the log (`reads past mapped memory`). The
  game then plays on: S01_01, S01_01A, S01_02A in a 5-minute run.
  The RetroArch core (older, hangs on Xvfb) is not needed.
- 4191's "Found 0 patches" was the patch file missing from `<emu>`
  (a later run had deleted it), not Vita3K: rerun `vpatch.py patch`.
- **First trace diff (2026-10-18)**: 50 pages from S01_01, original
  (`vpatch.py log`) against ours (`mages convert --dry-run --scenes
  S01_01,S01_01A,S01_02A,S01_03A --source-map`, run with `--trace
  --auto-play 0 --real-time` over a full build's `arc.nsa`), `trace_diff.py
  --map <map> --fps 30`: **no text, missing or extra findings**, once the
  speaker is taken off our page text (`vpatch.py` logs the record's
  message alone; ours has `【鷹志】` before it). 9 time findings, all
  within 3 frames: the original takes 2-3 frames between tasks ours runs
  in the same frame (its loading), and 32 frames for 1000 ms waits.
- Speed: the game runs at its 30 frames a second (8809 `@F` in 305 s);
  about 2.6 s a page with a circle every second (a click finishes the
  text, the next turns the page). No unthrottled run: the game times
  its waits by the clock, not by frames (with `fps-hack: true` the
  1000 ms wait takes 62 frames instead of 32, the page rate stays), and
  Vita3K has no faster clock. Timings stay rough (the user: Vita
  emulation is shaky).
- Run it (the source build): clone github.com/Vita3K/Vita3K with
  submodules into `<src>`, `git apply research/oretsuba/vita3k-freed-texture.patch`,
  `cmake --preset linux-ninja-clang && cmake --build --preset
  linux-ninja-clang-relwithdebinfo` (system Qt 6, clang, lld; no other
  packages needed here), then a folder `<v3k>` with `bin` linked to
  `<src>/build/linux-ninja-clang/bin/RelWithDebInfo` and empty `lib/`
  and `share/`, and `V3K=<v3k> research/oretsuba/vita-emu.sh <emu>/home
  300` (`WRAP="gdb -batch -x <cmds> --args"` runs it under gdb; gdb
  needs `handle SIGSEGV nostop noprint pass`, Vita3K's memory traps).
  Keys that reached S01_01 from a boot with a save: circle every 2 s
  from 46 s with Return (start) every 8 s, then circle every second.
- Nothing may open on the desktop: Vita3K's Qt front end opened a web
  page in the user's browser through D-Bus (2026-10-18, an update
  prompt the keys answered). `vita-emu.sh` now runs it with
  `DBUS_SESSION_BUS_ADDRESS=disabled:`, `BROWSER=true` and `xdg-open`,
  `gio`, `kde-open`, `gnome-open` shims first in `PATH` that only log
  to `<emu>/nourl/calls.log` (empty in every run since); `config.yml`:
  `show-welcome: false`, `check-for-updates-mode: 0`,
  `discord-rich-presence: false`.

Step 3, the full-route reference trace (2026-10-18): **6216 pages,
S01_01 to S02_05H**, in the run's 2 hours (about 1.1 s a page), on
the route check's run 4 (every first option, 日和子's ending S02_16A;
ours reaches it at page 23205). The game waits at a choice
(MessageSwitch_Tag, task type 6) until a key lights an option: the
key loop presses Down, then circle, whenever the trace's last task is
type 6 (a first run sat 18 minutes at S01_02C's choice on circle
alone). Vita3K is the step 2 build, its only patch
`vita3k-freed-texture.patch`; the `Unhandled SIGSEGV` lines it logs
every few seconds from boot are its own memory traps, not a crash.
Commands (`<emu>`, `<v3k>` as in step 2, `<out>` a scratch folder):

```sh
python3 -I research/oretsuba/vpatch.py patch <emu>/home/data/Vita3K/patch/PCSG00299.txt
# <keys.sh secs log>: circle every 2 s and Return every 8 s from 46 s to 100 s; then circle
# every 0.4 s, with Down before it while the log's last @T/@M line is a task of type 6
research/tools/headless-run.sh bash -c "<keys.sh> 7200 <emu>/logs/vita3k.log & \
  MEM=3G SHOT=300 V3K=<v3k> research/oretsuba/vita-emu.sh <emu>/home 7200"
python3 -I research/oretsuba/vpatch.py log <emu>/logs/vita3k.log <out>/orig.jsonl
conscripter-convert mages convert <game> -o <out>/game --dry-run --source-map <out>/map.tsv
# <out>/game: a full build's arc.nsa, default.ttf, movie and ons_args linked in
python3 -I research/oretsuba/routes.py <out>/game <renamed engine> <out>/ours --only 4
# take the speaker (【…】) off our page text: vpatch.py logs the message alone
python3 -I research/tools/trace_diff.py <out>/orig.jsonl <out>/ours-nospk.jsonl --map <out>/map.tsv --fps 30 --pages 10000
```

Diff over the 6216 pages: **1 text finding** (S02_03G:1530: the
original breaks the line inside 「…作れ\nるかッ！」, a `\n` in the
script; ours runs it on), 176 missing: CharacterActionEnd 154 (0x21),
SeStop 12 (0x38), 0x3e/0x3f (EyeCatch) 3 each, SpriteDelete,
CharacterDelete, 0x20, 0x26 once each; 181 quiet. The 1024 time
findings mean nothing (ours ran without `--real-time`).
`routes.py` reads START's story menu as the button screens it now is
(it looked for a `select`).

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-13 22:41 | - |
| First dry run | 2026-10-13 23:15 (lowering) | - |
| Story plays | - | - |
| Title, save, settings | 2026-10-13 23:30 (title); engine-drawn menus 2026-10-17 | - |
| Extras | 2026-10-14 08:47 | - |
| Progression verified | - | - |
| Finished | - | - |

## Status: the whole story plays (2026-10-18)

Open state only; the per-feature write-ups and the earlier queue are in
"History", "Status as it stood before the 2026-10-18 prune".

**Works:** all 463 scenes convert (no task left as a comment); the full
build lints clean and the route check reaches all eight endings and the
four after-stories. Messages, voices, sprites, characters and their
motions, transitions, effects, movies, title and START's story menu,
button bar, save/load, backlog, settings (the Vita's look and
defaults), the CG gallery, music room and movie room, boot logos. Web
build tested headless. Output 960×544.

**Open:**
- Looks against the original (needs Vita frames; Vita3K is shaky, so
  timebox it and prefer the decompile and our traces): text window
  size and place, character heights (where the eboot anchors them:
  `:Center` default 0,-1, where `OffsetY` comes in untraced), the eye
  catch's speed, the wave transition's size, the particles' motion,
  the skip dialog's and text's font sizes, CONFIG's lit cells (ours cut
  1-3 pixels short of the eboot's 90/180-wide cells at 0x8107145c).
  Full-route reference trace (Testing, step 3): 6216 pages of
  日和子's route (to S02_05H); text clean; to look at: SeStop gives
  none of ours 12 times, EyeCatch 6. The rest of the route: a further
  run from a save at S02_05H.
- A missing `:Position` with no picture to replace: the eboot puts
  Sprite at (640, 560) and SpriteBG at (640 - w/2, 360 - h/2); ours
  centres it (no script relies on it). The characters' has-a-place
  flag (+0x44 in CharacterOpen's record) has no reader traced.
- Unknown in the eboot (`docs/games/oretsuba.findings.tsv`: 88 rows,
  5 unknown): keys with no value (6; inferred 0), the
  voice ducking's level and ramp (ours 50 %, 200/400 ms), where `Tag`
  and `End` are handled, SpriteQuake's `:Param` layout, whether the
  shake's `:Param` 0, 1 are pixels, EffectStart's variant code
  (`:EffectNumber` argument 1, always 0). Not yet checked against the
  eboot: `chract`'s per-key `:Priority`/easing and the `.txt` tables
  (`adv.txt`, `chr*.txt`, `CGMODE`).
- Not converted: the Vita's save-data check at boot; OP00 replayed from
  the title (probably an idle timeout); `:Image 0,-1`'s colour panels;
  the particles' real motion; `spgrab` (engine #32, merged) could let
  SpriteBG/SpriteBS with `:FadeWait 0` (890 uses, which still wait for
  their fade) cross-fade without waiting; not used yet.
- A build from before #227 plays InOut_Act 3 and 1 as cross-fades
  (the plugins retired, engine #72): rebuild.
- Script errors the eboot can't read either (we convert or skip): five
  tasks with a missing or misplaced `[TaskN]` header (S01_04L, S03_03J2,
  S03C_03C twice, S04_03D), a SeStop inside S01_02A's EyeCatch (missing
  `}`), S03_06E2's task with two `:Tasktype`s (we take the first).

**Next:** the user picks fixes from "Arguments not read" (the table below). The dry
run gives 2,783 notes of 80 kinds (`convert.rs`'s `NOT_NEEDED` lists the
harmless ones, with reasons; every argument the converter never reads
is a note "<Task> [<sub-block>] :<Key> [argument <i>] ignored"). The
ones that still lose something:

| Task | Argument | Uses | What is lost |
|---|---|---|---|
| SpriteMove | `:Roll` | 12 | dropped: turns (radians, kept after the move: S02_03F, S03_02A). The turn goes to the GPU (0x8103d53c): direction and centre untraced; `spzoom`'s angle could carry it |
| SpriteMove, SpriteQuake | every argument | 8, 16 | dropped: the picture they name isn't up by our tracking (removed too early?) |
| SceneJumpDialogEnd | `:Fade` | 531 | dropped: the eboot keeps the skip dialog up after the click (task 0x85 fades it out over `:Fade` frames, default 18, 0x8112a490), so the tasks between happen under it; ours fades on the click. On the skip path it stays through NextScene (382 of 894 paths); before 22 choices no End comes: untraced |
| EffectStart | `:Priority`; `:EffectNumber` 1; `:Param` past those read | 208; 208; up to 202 | dropped: where the effect sits among the pictures; the particles' motion |
| InOut_Act | `:Param` 1 (2, 3 once) | 72 | dropped for the `:Type`s that read only `:Param` 0 (which not yet checked) |
| Window | `:Time` with `:Visible 1` | 2 | dropped: the window fades in over it (ours at the next message) |
| SpriteBG | `:SpriteDelete` (with pictures an earlier scene left) | 8 | dropped |
| EyeCatch, EyeCatchEnd, MovieStop, MoviePlay | `:Color`, `:Fade`, `:EndWait` | 7 each | dropped with `:Image 0,-1`'s panels and the movies under the script |
| CharacterMove, CharacterAction | `[CharaN]` `:Priority` | 7, 1 | dropped |
| CharacterOpen | `[CharaN]` `:FadeWait` | 6 | dropped |
| CharacterActionEnd | `:Copy`; `:Number` (read before `:Character`) | 5; 1 | dropped: stops the character without the copy |
| EffectEnd | `:Delete` | 7 | dropped |
| others | | 1-3 each | CharacterAction `:EndWait` on a loop; repeated keys; FlagOperation `:CGFlag`; CharacterOpenBS `:Fade 30`; EffectQuakeEnd `:Fade 1`; EffectQuake `:Texture` |

**Commands.**

    conscripter-convert mages check <game>
    conscripter-convert build <game> -o out/oretsuba-cons   # mages convert with --title 俺たちに翼はない
    conscripter-convert mages convert <game> -o out/oretsuba-cons [--scenes S01_01A] [--webp] [--dry-run]
    conscripter-convert mages convert <game> -o out/oretsuba-web --loose   # web build, see History

Full build about 4 minutes, 1.7 GB `arc.nsa`. Debug trace of a build and
the steered route runs (`research/oretsuba/routes.py`, run with a
renamed engine copy; `--only 9,10,11,12` the after-stories): recipes in
"History", "Checking a build with the debug trace" and "Steered route
runs" (`engine/tools/cons_trace_check.py t.jsonl --script 0.txt
--config research/oretsuba/cons_trace_check.json`).

## History

- Two skip dialogs in one scene (S01_12B, S01_13B: one per path) shared
  their `*<scene>_offerN` label, so the 宮子 path's skip went to the
  明日香 path's dialog: the picture dialog returned before counting
  itself. Counted first now; every label in 0.txt is defined once
  (2026-10-18).

- SpriteMove `:Scale -2` (S01_05B, once): the eboot turns the picture
  180° at 2× (the zoom goes straight through 0 to it); ours clamps to 0,
  so the picture shrinks away. Found by running the sprite quad
  (0x8103d53c) under `research/tools/exe_call.py` (2026-10-18): a
  negative scale negates both corner offsets, same UVs and vertex order.
  Converted (SpriteZoom's `to` is signed; `spzoom`/`spmotion` zoom a negative percent as a flip both ways = the 180° turn): the zoom goes through 0 to -200 as in the eboot. Dry run: S01_05B's `spmotion 270` ends at -200,-200. Not filmed.

- Reference trace findings (2026-10-18). S02_03G:1530's line break:
  ours breaks too; the eboot's record keeps the script's `\n` as two
  characters and its line layout (0x81016eca and three like it, lines
  of 0x854 bytes) starts a new line there, so `vpatch.py log` now turns
  `\n` into a newline (the diff: text 0). CharacterActionEnd
  `:EndWait 1` after a message gave nothing (154 of the trace's
  missing): it now waits for the rest of the action with
  `spmotionwait` (2,126 more in the dry run, replacing 12 `delay`s);
  151 of the 154 now give a command, the other 3 (S02_02A:1200,
  S02_03E:4380, S02_03G:5950) name a character with no action, a
  no-op in the eboot too.

Finished work, moved out of "Status" so a session reads only the open
state.

### Status as it stood before the 2026-10-18 prune

#### Queue (2026-10-16)

- Merged: InOut_Act 3 (flip) and 1 (wave) as warp maps (`cons_warp.dll`, #227, engine #67/#69; maps in `warp/`); 6 (blur) stays `cons_blur`. Open: the plugins retired (engine #72): a build from before #227 plays them as cross-fades until rebuilt.
- Merged: the chapter card's names (#48). Open: motions and fades as
  engine motions (#13, #18, waiting on engine #2 → #7 → #14).
- The user picks the next fixes from "Arguments not read" below.
- Dropped arguments picked by the user (2026-10-17): the SE fades and waits, CharacterDelete's `:FadeOut 0` and the window's fade-out are done ("History"); SceneJumpDialogEnd's `:Fade` is left (table below). Next PR: BacklogMessage `:Message`, the shakes' `:Param` 2-5 and SpriteQuake's `:Wait`, SpriteMove `:Roll`, CharacterWait waiting for fades and CharacterActionEnd stopping moves.
- Dropped arguments picked by the user (2026-10-17), second part: BacklogMessage `:Message` (a `logadd`), the shakes' swing (`:Param` 2-5) and SpriteQuake `:Wait` are done ("History"). SpriteMove `:Roll` is left (table below). CharacterWait waiting for fades needs nothing: no CharacterWait follows a `:FadeWait 0` character fade without a message between, whose click outlasts it (a dry run that waited for them added no line). CharacterActionEnd leaving moves running is what the eboot does: 0x81127a00 → 0x811277c0 ends only the action (+0x2e8).
- Looks against the Vita: the original runs under a source-built
  Vita3K with a local patch for a texture the game frees ("Testing",
  2026-10-18), and the first 50-page trace diff (S01_01, S01_01A) has
  no text or order findings, only loading frames. Next, if wanted:
  longer runs (a scene at a time, by keys), and frames of the original
  for the open guesses below. Vita3K timings stay rough; the decompile
  and our own traces stay the main checks.
- `spgrab` (engine #32, merged) could let SpriteBG/SpriteBS with
  `:FadeWait 0` (890) cross-fade without waiting: grab the screen, change
  the picture under the copy, fade the copy out. Not used yet.

#### Accuracy (value audit, 2026-10-17)

Every hand-typed value in `crates/conv-engines/src/mages/` carries a
`// src:` (eboot address or measured SystemData picture), `// ours` or
`// guess:` comment. The guesses still open; the decompile could settle
the first ones, a frame of the Vita game the rest:

- A missing `:Position` (no script relies on it). Settled for Sprite,
  SpriteBG and SpriteBS: `task_run` (0x81021070) reads the has-a-place
  flag (Sprite's +0x560, one byte per picture; SpriteBG's +0x94) and,
  clear, puts the picture where the picture it replaces (same
  SpriteNumber, the texture by default) was; ours does too. With none to
  replace, Sprite gets (640, 560) and SpriteBG's corner goes to
  (640 - w/2, 360 - h/2), in placement units not traced against our
  centre: ours still centres it (guess). The characters' flag (+0x44 in
  CharacterOpen's record) has no reader traced yet. EffectStart's
  missing `:Param`s are 0 in the eboot (`cfg_number`); ours keep usable
  fallbacks, which no script reaches.
- Frames: the text window's place, its text origin and font size (23)
  and line gap; the skip dialog's font size (21); the particles'
  motion (ETC.cpk's sheets, our own drift).
- CONFIG's lit options: the eboot (0x8107145c) draws 90/180-wide cells
  from x 289; ours cut each to the word, losing the glow's last 1-3
  pixels.

`crates/conv-engines/src/mages/` reads the archives (`cpk.rs`), the
scripts (`tasks.rs`), lowers scenes to the IR (`convert.rs`) and writes
the assets (`assets.rs`; sounds become Ogg Vorbis through ffmpeg, BGM
with its ADX loop as `LOOPSTART`/`LOOPEND`):

    conscripter-convert mages list <game>/media/CG.cpk
    conscripter-convert mages extract <game>/media/SCRIPT.cpk -o <dir>
    conscripter-convert mages check <game>
    conscripter-convert mages convert <game> -o out/oretsuba-cons \
        [--scenes S01_01A,S01_02A]

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
| Backgrounds and sprites (`SpriteBG`, `Sprite`, `SpriteBS`, deletes), by priority | converted (`:Scale` zooms the picture about its centre, which `:Position` places, with `spzoom`, as SpriteMove's: below): `SpriteBG` and `SpriteBS` replace every picture (a background, a black cover, a white flash, an event CG, in one fade), the characters too unless `:SpriteDelete 0` (the scripts never delete the characters a location change leaves, and after 37 of the 0s they delete them themselves); a scene's first picture also removes whatever the scene before left (the eboot loads each scene's textures anew). `Sprite` adds one, and `SpriteDelete`/`SpriteDeleteBG` remove the picture of their `:SpriteNumber` (a texture). Our reading of the scripts; positions not yet checked against the original |
| Characters (`CharacterOpen`, `…BS`, `CharacterDelete`, `CharacterMove`) and `SpritePriority`/`CharacterPriority` | partial (CharacterMove's `[CharaN]` `:Priority` dropped: below; a `:Wait 0` move that also fades the character in or out still waits). CharacterMove `:Wait 0` (668 uses) runs the move under the text (`spmotion` without `spmotionwait`); `:Wait 1`, the default, waits for every character's move, as CharacterWait (651 uses) waits for their moves and actions (task_run cases 0x1e, 0x23). At y 0 a character stands on the screen's bottom edge (1209 of the 1333 pictures, 630 pixels tall, are cut off along their bottom row); the eboot scales `OffsetY` (-68 for everyone standing) by 544/720, but centred that leaves them 8 pixels above the edge, so where it anchors them is still to be found. Shorter pictures (the 960×540 cut-ins) are centred plus `OffsetY`. CharacterMove's y counts from that height. `CharacterDelete` without a list (`:SpriteMax 0`, 962 of 1414) removes every character; `CharacterOpenBS` removes the characters it doesn't name (the scripts re-open or delete-all them afterwards, and only twice delete one by name). S04_16B's `:Number <ST_ASU02H>>` (a placeholder the script's tool never filled in) is resolved by the picture's name through the character tables (Asuka 0,0,2,8,0) |
| `:Color` tints (dusk 255,236,220 on 3,892 pictures, night 220,222,255 on 2,105), `:Grayscale` (479), with `:Gamma` the sepia flashbacks (188), `:Floor`/`:Ceil` film negatives (42), `:Color` alpha 0 | converted as sprite filters: a tint multiplies (`spfilter` colour, multiply), sepia is a monotone with each channel of the grey raised to 1/gamma as an offset at mid-grey (+75, +28, −47), and a picture with filters fades in inside a transition, filtered. Alpha 0 shows a character see-through; a CharacterMove that changes the alpha cross-fades to the new place and opacity over its `:Time` |
| `InOut` transitions with the RULE masks | converted |
| `InOut_Act` (the eboot's effect 25: `:Type` picks the class, `:Param` 0 mostly a direction 0-3 = right, down, left, up) | 0 (the old screen squeezes to that edge while the new one grows from the other; 21 uses) and 7 (a push; 89) as ONScripter's scrolls; 5 (to or from white or black; 9) through white; 2 (a mosaic growing to `:Param` 1 pixels of 720 halfway while the screens cross-fade; 66) as ONScripter's mosaic out and in; 3 (a card turning over in perspective, old in front, new behind, about the vertical axis for `:Param` 0 below 2; 47) as `cons_flip`, the new screen turning in. 1 (a fade with the rows waving sideways, `:Param` 1-2 the wave; 57) as CONscripter's `cons_wave.dll` (the size and wave length a guess: the shader's constants are uniforms, `:Param` 1 × 100 per mille of the screen, `:Param` 2 waves down it); 6 (a fade blurred sideways, the most halfway, by `:Param` 1 pixels of 1280, `:Param` 0 = 1 only the new screen; 130) as `cons_blur.dll`; 4 (2) a plain fade |
| FadeOut/FadeIn, TimeWait, BGM, SE, Window hide | partial (the window's fade-in, `:Visible 1` with a `:Time`, dropped: below). SePlay `:Div` n > 1 fades the SE in over n frames of 60 a second (`dwavefadein`), SeStop's out (`chvolfade …,1`); SePlay `:Wait 1` waits for the SE's length (from its ADX header), SeStop's for the fade-out. Window `:Visible 0` fades the window out over 2 × `:Time` frames (500 ms without one: a step of 1/15 an update) and waits, when a message showed it last (`windowfade`, `delay`) |
| Tags, jumps, flag switches and operations, choices with points, NextScene | converted (BacklogMessage's chosen answer goes into the log as a `logadd` line), not yet checked against a walkthrough |
| SceneJumpDialog (skip a scene already read) | partial (SceneJumpDialogEnd's `:Fade` dropped: below): a global "read" flag per scene (`%1200` + the scene's index in name order). A read scene shows the eboot's dialog (layout below): SystemData's `ss_bg1`…`ss_bg5` by `:BG` 0 (the protagonist), このシーンをスキップしますか？, the date (`:BG` 2), the emblem (`:BG` 1), the `:CharaIcon` faces, the summary (`:Text`) drawn with the `--font`, and Yes (skip) / No buttons that glow under the pointer. The line the eboot also writes at (81, 99) is left empty (its text isn't in the task; perhaps the chapter's name). The original asks only when its setting is on, and so does ours (シーンスキップ機能 in the settings). |
| Flash; EffectQuake/End (a screen shake for `:Time` (60 frames if none), dying down with `:Fade 1`, at full size with `:Fade 0`, the default; `:Loop 1` or a negative `:Time` until EffectQuakeEnd; `:Time 0` otherwise stops it); SpriteQuake (one picture, a background, shaken by `:Param` 0, 1 for `:Time`, dying down with `:Fade 1`; all zeros stop it; `:Time 0` with a size, once in S04_07A after a `// wt` comment, waits for the shake before it, which the message in between outlasts: nothing); SpriteMove (`:Wait 0`, 198 uses, runs under the text until a SpriteMoveEnd of that picture or every one, 171 uses: cases 0x27, 0x29) | partial (SpriteMove `:Roll`: below). Without `:Param` 0, 1, a shake swings by `:Param` 2, 3 (x, y) with 4, 5 half-waves over `:Time` (0x81008a3c; SpriteQuake's the same, 0x81087114): EffectQuake a sine `screenshake`, SpriteQuake `spmotion` keys four a half-wave; `:Curve` (12 uses) bows the move as the eboot does (0x8103cf20: "Small coverage gaps" in "History"), one `spmotion` key a frame (IR `MoveArc`); `:Scale` (90 uses) zooms the picture about its centre with CONscripter's `spzoom` (after 0.19.1), which keeps it in its place among the sprites; the zoom stays for the moves after it. The two values after `:Position` are the eboot's easing (ease_value, 0x8103cc18: type 1 in, 2 out, 3 in and out, raising t² to the power): converted for moves and zooms (IR `MoveEase`, the power × 2 a whole exponent), 65 of them; the rest are straight. Only the place is eased: the zoom goes straight with the time (0x81086a0c), so an eased move that zooms (37) is one `spmotion` key a frame, as `:Curve`'s (IR `MoveArc` with bow 0) |
| CharacterAction (`chract`'s 661 keyframed moves: bows, shakes, bobbing; 72 uses loop until CharacterActionEnd) and CharacterActionEnd | converted with CONscripter's `spmotion` (after 0.19.1): offsets from the character's place, straight between keyframes, run by the engine while the text goes on; a loop goes back to its first key at the action's `:Frame` length. `:EndWait 1` waits for it (64 uses; 1,902 don't wait). CharacterActionEnd stops a loop, or with `:EndWait 1` waits for what is left of the action (`spmotionwait`; at once when it has ended). SpriteQuake is a one-shot `spmotion` on its picture too. The eboot (0x81008008) reads each field of a frame (`:X`, `:Y`, `:Roll`, `:Scale`, `:Alpha`, `:Priority`) as a track of its own, keyed where it is given, each key with an easing; we sample the tracks at every key frame, straight between keys (one action eases a key; `:Priority` is not used). Actions with `:Scale` (zoom about the centre, 60 uses), `:Roll` (radians; turned clockwise on the screen, our guess; 5 uses) or `:Alpha` (60 uses) are CONscripter's `spmotion2`. `:EndSet 1` (read only when the action doesn't loop) leaves the character where the last key has it: 475 uses end with the character moved, faded out or zoomed (Asuka stepping aside and fading out in M02_04, Kinako rising into view in S02_09D1), made with `spmotion2`'s loop mode 2; a move, a new picture or another action on the character first ends it there (so skipping lands right) |
| EyeCatch `:Image 0,N` (CG.cpk ID N + 457, at least 458, as the eboot's 0x81048978 loads it: `EC01`-`EC19` for 1-19, but the archive has no `EC20` or `EC30`, so 20 is `EC21` (京's after-story) and 27, the common one, `EC28`, the framed title logo) | partial (`:Image 0,-1`'s colour panels and their `:Color`, `:Fade` dropped) as in the eboot (0x81048b8a): the picture's top and bottom halves (960×272) close from above and below like shutters (500 ms, our guess) and open again at EyeCatchEnd over `:Fade`, showing the FadeOut's colour (or the scene) behind; made as 10-frame animations (`system/shutters_N_in/out.png`). `:Image 0,-1` (two panels of a colour in the eboot, 7 uses) shows nothing |
| EyeCatch `:Image 1,card,place,date` (114 uses: the chapter cards) | converted like `:Image 0`, as one screen made from SystemData: the card `ec1_bg`…`ec6_bg` (ID 67 + card: 0-3 the four protagonists' chapters, 4-5 Karura's, 6 奈落の底) with the place (ID 74 + place: `ec_bgNN`, its times of day as A/B/C) in its frame and the date (0 = 11月21日, -1 none) cut from the calendar under the card. The original fades the three in and out and pans the place 40 pixels while it fades in; drawn where the pan ends |
| MoviePlay: the USM movies (MPEG-1 960×544 + ADX) demuxed (`usm.rs`) and made H.264/AAC MP4s in `movie/` (loose) | partial (`:EndWait 0`, a movie under the script, plays to its end first); `:Movie` N is the Nth of OP00, OP01, OP02, the seven `ED_*` (alphabetical), Poem, OP03 (each heroine's ending scene plays her ED). `:Skip -1` (every use, the default) makes a movie skippable only once it has been seen, as the eboot does (0x81044b74; its seen table is set as the movie starts): a global flag per `:Movie` N (%1680 + N) |
| EffectStart/End 53 (ノイズ: static at `:Param` density) and 42 (ブラー: a blur screen filter); SetSceneName (chapter title) | converted |
| EffectStart/End 57 (画面キャプチャースプライト, 50 uses: a frozen copy of the screen zoomed onto a point at an impact; the eboot's class at 0x81040714: `:Param` x, y where its centre goes, scale, frames, ease type 0-3, power) | converted with CONscripter's `screenzoom` (after 0.19.1): the copy's scale goes from 1 to `:Param` 2 and its centre from the screen's to (x, y) over half of `:Param` 3's frames (the eboot steps 2/frames a frame), with the eboot's easing (0x8103cc18: straight, in, out, in and out, to a power); it fades in over `:Fade` and out over EffectEnd's `:Fade` (the slot cleared inside a transition). Scale 1 is a freeze frame |
| EffectStart/End 52 (光玉 light balls), 54 (紙吹雪 confetti), 51 (雪 snow) | converted to looping 16-frame animations made from ETC.cpk's `lightball.png`, `paper.png`, `snow.png` (`:Param` count): the balls twinkle and circle in place, snow and paper fall one screen per 2 s loop; the original's motion is not known |
| EffectStart/End 40 (a wave over the starry monologue background, `:Param 400,3000`: the eboot's shader 0x8115f304 moves each row's picture along the diagonal by 1.5% of the width × sin(2π(600y/(height × `:Param` 0) + 2t/`:Param` 1 ms)), growing in with the effect's fade; a negative `:Param` 0 waves the columns, unused) | converted as `screenripple` at 45° through a grey ramp (`system/waveN.png`, N/100 waves down the screen); EffectEnd stops it without its fade (2 uses of 30 frames) |
| Achievement (trophies) | nothing to show |
| Title screen (`title_bg.png`'s picture and labels: START, LOAD, GALLERY) | converted, the buttons where the eboot draws them (0x81083fd8: 173×36 cells centred at x 786, a row every 41.25 px from y 258 in the order QUICK LOAD, LOAD, START, GALLERY; the eboot places its menu pictures by their centres, as "Press START button", 258 wide at x 769, shows; we have no quick save, so its place stays empty); settings are in the extras and the in-game bar. `SystemSE.AWB`'s two sounds here and in every other menu and on the button bar (IR `Script::menu_sounds`): the tick (0, the eboot's cursor move) when the pointer comes onto a button (`selectvoice`), the confirm (1, 0x8105e7a6) on a click (CONscripter's `btnse`, after 0.22.0), both at システムボリューム (`sysevol`) |
| START's story menu: 始めから and the after-stories ある日の林田美咲/渡来明日香/山科京/軽部狩男 (P03/P01/P02/P04) | converted as the eboot has it (0x8105e69c, 0x81083086, 0x81084c26): START opens the menu once 明日香's, 京's, 亜衣's or 鳴's ending is seen or part 1 or 2 cleared (global flags 0-5; otherwise the story starts), and the three heroines' after-stories are then open; 狩男's needs part 1, 2 or 3 cleared (the Vita shows ？？？？？ until then). Each starts its adv.txt start scene directly. Drawn as the Vita draws it (0x81083086): over the title, `title_bg.png`'s 242×60 cells centred at x 788: 始めから at y 225, the 〜Prelude〜 header at 305, the after-stories at 365, 395, 425 and 455 (狩男's ？？？？？ until it opens), lit under the pointer; the buttons are the labels' 30 rows, so they don't overlap. The header stays unlit (the Vita lights it while the cursor is on an after-story). |
| Button bar (Auto Skip Log Save Load Config Hide), save/load (3 slots × 10 pages), backlog, yes/no dialog | converted in the Vita's look, our layout (2026-10-14, the user's choice): SystemData's backgrounds (`save_bg`, `load_bg`, `bklog`), slot plates (`sl_data`), dialog frame (`dlg_bg_26`, the one without words), headers cut out where they differ from a header-less background; the Vita's controller hints and page box painted over; the bar's Log, Save, Load and Config are the Vita's own labels from `ss_bg1.png` (y 746, 60×24 cells: Q-Save, Q-Load, Return, Save, Load, Log, Config; rows white, glowing blue, navy, faded), given a dark halo where they cross the window's feather; Auto, Skip and Hide (no label in the game: `SKP1_ICON.png` only has the running indicators) are drawn with the `--font` in that style. Other labels are drawn with the `--font` in the game's style (white, blue halo; gold under the pointer). Backgrounds behind conv-cons's white words are darkened. The save and load menus are drawn by the engine (`screen "save"`/`"load"`, `screens/save.lua` and `load.lua` from `SaveMenu`, as in 最果てのイマ); saves from older builds load. The backlog is drawn by the engine (`screen "backlog"`, `screens/backlog.lua` from `Backlog`, as in 最果てのイマ). |
| Settings (CONFIG) | converted in the Vita's look and with its settings (2026-10-15, the user's choice), from SystemData's `bg_config_01.png` (基本設定) and `bg_config_02.png` (サウンド設定): each is the page with its rows' labels and options drawn in, over an atlas of the options lit (a glow and a cloud), a snowflake and the rows' cursor plates (`mages/config.rs`, IR `PagedSettings`, conv-cons `settings_pages.rs`; drawn by the engine since 2026-10-17: CONscripter's `screen "settings"`, `screens/settings.lua`). The option words are cut out of the page (the page minus the page with them painted over, kept near the lit version's letters) and lit with the atlas's glow when current; the bars are the page's own, with the snowflake a knob at the fill's end (a picture slider: dragged, or a press on the bar puts it there, or stepped with ◀ ▶, the wheel and Left/Right; the volume applies while it moves) and the line filled up to it (the slider's `fill`; the knob's place rounds to the nearest pixel where the earlier per-level pictures floored, so at some levels it sits 1 pixel right of them); the question after タイトルに戻る / ゲームを終了 is asked over the open screen (`screens/confirm.lua`, the yes/no dialog in the Vita's frame); no fades or unrolls, none being known from the Vita's CONFIG or dialog; ← L / R → switch pages. Page 1: シーンスキップ機能 ON/OFF (the scene jump dialog), 音声スキップ する/しない (the click stops the voice: CONscripter's `clickvoicestop`), スキップモード 既読のみ/全て, 選択後のスキップ 持続/解除 (skip and auto go on after a choice: `selectkeep`), オート速度 (ours, on the row of 選んだ選択肢の色を変更, which with 自動クイックセーブ is not converted and painted over), メッセージ速度 遅い/普通/早い/一括 (text speed 4, 7, 9, 10 of 10), ウインドウの不透明度. Page 2: Voice/BGM/SE/システム ボリューム (システム: the menus' sounds, `Setting::SystemVolume`), 音声再生時にBGM音量を下げる (to 50 % while a line is voiced), キャラクター音声 全てON/全てOFF (every voice is character 0, one switch). A third page in the same look holds our switches (mouse wheel, confirmations). The hints' place holds デフォルト設定 (every setting back to its default) and 戻る; from the game, タイトルに戻る and ゲームを終了 at the left. The defaults are the Vita's, from its デフォルト設定 (0x81070e70), in our steps of 10: Voice 85 (9), BGM 55 (6), SE 55 (6), BGM lowered for voices on (ours to 50 %, the Vita's to 55), every character voice on, メッセージ速度 普通, ウインドウの不透明度 70 % システムボリューム 55 (6) (IR `SettingsMenu::defaults`, `switches_off`) |
| Extras menu: the title's GALLERY shows ALBUM, MOVIE, MUSIC over the title, in its first three button rows, as the Vita does (0x81083c0c; `title_bg.png`'s labels), and ours adds CONFIG and 戻る in the two rows below (the Vita's has no CONFIG; a right click also goes back). The modes have the album's background (`ExtrasMenu::menu_background`). The CG gallery from `CGMODE` (8 heroine tabs from `gm_bg.png`'s cards, 4×5 thumbnails a page, up to 3 pages, a viewer with ◀ ▶ HIDE 戻る); scenes mark CGs seen from their `:CGFlag` | converted in the Vita's look, our layout. The album, the music room and the movie room are drawn by the engine (`screens/gallery.lua`, `music.lua`, `movies.lua`, from `ExtrasMenu` with `engine_screen`): the music room's player bar is a toggle (▷), buttons running the script's song labels and a picture slider for the volume; the viewer centres the 967×725 CGs as `bg` does (engine #91). The extras menu itself stays a script loop |
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

    conscripter-convert mages convert <game> -o out/oretsuba-web --loose
    python3 -I engine/src/cons_web/conscripter_index.py -i out/oretsuba-web \
        -o out/oretsuba-web/cons_index.json --title oretsuba --lazyload
    cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/oretsuba-web/
    python3 -m http.server 8000 -d out/oretsuba-web   # then open /conscripter.html

2.2 GB, 27,472 files (index first, then the engine's files, so they
aren't listed). `--webp` would store the pictures as WebP (smaller);
not tried in the web build yet.

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
`engine/COMMANDS.md`): a `bgmfadeout 16:bgmstop` while `bgmfadein
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

**Coverage against the eboot (2026-10-15).** The whole eboot mapped
as in `docs/new-game.md` §2, "Mapping the whole exe": the 31 code
addresses reached only by pointer made functions (`cb_<addr>`, two
rounds, mostly CRI middleware), the import stubs named (`strcmp`,
`sprintf`, …), then every key `scene_load` (0x8102840a) reads
(`"TaskList/Task%d/<key>"`, read with `cfg_find`/`cfg_number`) set
against the keys the 463 scenes use and those the converter reads.
`task_type_number` knows 111 task types; the scenes use 52, all
converted. The messages hold no markup but `\n`. Keys the scenes use
that the eboot reads and the converter doesn't (typos such as
`:Prioriry`, `:TIme`, `:fFade` are ignored by both):
1. SpriteBG and SpriteBS with `:FadeWait 0` (890 uses) still wait for
   their fade: they replace the whole picture, which a sprite's fade
   (`spmotion`) can't cross-fade under the text. A character changing with a motion, zoom
   or look waits too. Fades are `spmotion` motions (loop 0 in, loop 3
   out); a CharacterAction hop started while its character fades in
   takes the fade into its keys (11 in the snapshot), since a sprite has
   one motion.

**Arguments not read (2026-10-16).** The converter marks every
argument it reads (`tasks.rs`, `Block::read`); after each scene, each
argument a task was given that it never read is a dry-run note,
"<Task> [<sub-block>] :<Key> [argument <i>] ignored", whatever its value
(an argument we don't read gets our behaviour, which differs unless it
is the value we reproduce). `convert.rs`'s `NOT_NEEDED` lists the notes
known not to matter, each with its reason: a row naming a value (`SpriteMove
:Wait = 1`, `MoviePlay :EndWait = 1`) covers only
that value, the others are still noted; rows without one cover only
what is harmless whatever its value (FadeIn `:Color`, always the
FadeOut's, 579 of 579; trophies; typos and nameless blocks the eboot
doesn't read either). `:SpriteMax` limits only CharacterOpen and
CharacterOpenBS: scene_load (0x8102840a case 0x1b) shows Chara0-Chara9
in order, those with a `:Number`, until `:SpriteMax` of them (0 when
missing or empty: none), and ours does too (2026-10-17: 7 tasks named
one character more, S04_12B's `:SpriteMax 0` and S04_13B's empty one
show none); CharacterDelete, Move and Action (cases 0x1c, 0x1e, 0x20)
read all ten whatever it says, as ours.

Traced in the eboot (2026-10-16; scene_load 0x8102840a reads a task,
0x8100da30 starts it, task_run 0x81021070 runs it), each now a
`NOT_NEEDED` row with its address where ours matches:
- Shakes (EffectQuake, SpriteQuake): the shake (0x81008a3c) is a
  pattern table × `:Param` 0, 1 percent, plus a swing of `:Param` 2, 3
  pixels making `:Param` 4, 5 half-waves over `:Time`, eased. `:Fade 1`
  shrinks it to nothing over `:Time`; `:Fade 0` (the default) keeps it at
  full size and stops it at the end. `:Loop 1` or a negative `:Time`
  repeats it until EffectQuakeEnd, ignoring `:Fade` and `:Wait`; `:Time 0`
  without a loop stops the shake (0x810089a4). SpriteQuake (0x81120894)
  likewise: mode 7 full size, 8 (`:Fade 1`) dying down, 9 (a negative
  `:Time`, never used) a loop, `:Wait` 1 by default (all converted).
  `:Window 1` (the default; the scripts always write 0) also shakes the
  text window; ours shakes only the picture, as with 0. The 1,1,1,1 of
  `:Param` 2-5 is a 1-pixel swing: nothing to see. EffectQuakeEnd's
  `:EndWait` is never used (scene_load clears the flag it has just
  set); its `:Fade` > 0 lets the shake run out over that many frames, 0
  stops it (ours).
- FadeOut/FadeIn `:Type`: the transition's rule (0x81049292), 0 none: a
  plain fade, as ours.
- MessageSwitch_Tag `:Frame`: which look of the choice box (0x810160e2
  moves its cell down `:Frame` rows); 0 the first, ours.
- `:Time` without a fade: Sprite takes `:Time` over `:FadeIn` (scene_load
  case 9), but every Sprite with `:FadeIn 0` has `:Time 0`; SpriteBG and
  SpriteDelete use it only with their fade flag, as ours. SpriteDeleteBG
  reads no `:FadeOut` at all and always fades over `:Time` (converted).
- EffectStart `:EffectNumber` argument 1: handed to the effect object's
  method +0x24 (effect_create 0x8103f75a), which stores it at +0x18 as a
  variant code (0x8112a860; read e.g. by 0x81040f76); effect 57 never
  reads it. 0 in every use; whether 0 is each class's plain form is not
  traced, so it stays noted.

**Exe mapping redone (2026-10-17, `docs/new-game.md` §2 steps 6-16).**
`docs/games/oretsuba.findings.tsv` has one row per task argument or
branch the scenes use: 88 rows, 71 seen, 10 inferred, 6 unknown (each
with its check). The game updates at 30 fps (0x810009b4 waits two
vblanks a frame) and every script time counts 60 fps frames, halved
into updates or turned into ms, as ours. Fixed from it: MoviePlay
`:Skip -1` (skippable only once seen, #359) and CharacterOpen's
`:SpriteMax` (#360). `argcensus.py` on the dry run flags two values,
both explained: `chvolfade -1` (65,497: the music ducking under each
message, -1 is the music channel) and `spmotion` 20000 ms (9: SpriteMove
`:Time 1200`, S03C_02B's 20-second pan). Open questions:
- A key with no value (6: two `:FadeWait`, two `:Fade`, a `:Rule`, a
  `:SpriteMax`) is 0 if the parser (0x8108cc48) keeps the key, ours the
  default: inferred, the parser's value states not decompiled.
- The voice ducking's level and ramp (conv-cons's 50 %, 200/400 ms) are
  not traced in the eboot.
- `Tag` and `End` are not in task_type_number (0x81035094); where they
  are handled is not traced.

Not covered yet: only scene tasks are checked; `chract`'s actions (the
eboot reads a `:Priority` and an easing per key there) and the `.txt`
tables (`adv.txt`, `chr*.txt`, `CGMODE`) are not.

The full dry run gives 2,783 notes of 80 kinds; by how much they show,
with their uses:

| Task | Argument | Uses | State: what is lost |
|---|---|---|---|
| SpriteMove | `:Roll` | 12 | dropped: turns (radians, kept after the move: S02_03F and S03_02A turn a background, show text, turn it back). Not converted, because the turn goes to the GPU (0x8103d53c), so which way a positive `:Roll` turns on the screen, and about which point, isn't traced; `spzoom`'s angle could carry it |
| SpriteMove, SpriteQuake | every argument (`:Scale` too) | 8, 16 | dropped: the picture they name isn't up by our tracking (one we removed too early?) |
| SceneJumpDialogEnd | `:Fade` | 531 | dropped: the eboot keeps the skip dialog up after the click (task 0x85 fades it out over `:Fade` frames, 18 by default, 0x8112a490, only while it is up), so the tasks between (BgmStop, SpriteBG black, …) happen under it; ours fades it out on the click. Not converted: on the skip path the dialog stays through NextScene (382 of 894 paths) and before 22 choices no End comes, which isn't traced |
| EffectStart | `:Priority`; `:EffectNumber` 1 (always 0, above); `:Param` past those an effect reads (mostly 0s; 52's 1, 54's 1-2) | 208; 208; up to 202 | dropped: where the effect sits among the pictures; the particles' motion |
| InOut_Act | `:Param` 1 (2, 3 once) | 72 | dropped for the `:Type`s that read only `:Param` 0 (which ones not yet checked) |
| Window | `:Time` with `:Visible 1` | 2 | dropped: the window fades in over it (ours shows it at the next message). A third, with `:WindowNumber 0`, the eboot doesn't read |
| SpriteBG | `:SpriteDelete` (with the pictures an earlier scene left) | 8 | dropped |
| EyeCatch, EyeCatchEnd, MovieStop, MoviePlay | `:Color`, `:Fade`, `:EndWait` | 7 each | dropped with `:Image 0,-1`'s panels and the movies under the script (`:EndWait 0`) |
| CharacterMove, CharacterAction | `[CharaN]` `:Priority` | 7, 1 | dropped |
| CharacterOpen | `[CharaN]` `:FadeWait` | 6 | dropped |
| CharacterActionEnd | `:Copy`; `:Number` (read before `:Character`) | 5; 1 | dropped: it stops the character without the copy |
| EffectEnd | `:Delete` | 7 | dropped |
| others | | 1-3 each | CharacterAction `:EndWait` on a loop; repeated keys (Message `:Name`, SpriteBG `:Priority`); FlagOperation `:CGFlag` (a CG never marked seen); CharacterOpenBS `:Fade 30`; EffectQuakeEnd `:Fade 1`; EffectQuake `:Texture` |

Script errors, which the eboot can't read either: five tasks whose
`[TaskN]` header is missing or misplaced (S01_04L, S03_03J2, S03C_03C
twice, S04_03D: the eboot looks tasks up as `TaskList/Task%d`), which
we convert; a SeStop inside S01_02A's EyeCatch (a missing `}`), which we
skip; S03_06E2's task with two `:Tasktype`s (SpriteBG, then Sprite: we
take the first).

Next, in order:
1. Looks against the original: text window size and place, character
   heights (where the eboot anchors them: its quad puts the picture's
   `:Center` point, by default 0,-1, the top edge's middle, at the
   position; where `OffsetY` comes in is not traced), the eye catch's speed, the
   wave transition's size. Needs screenshots of the Vita game.
2. The "Arguments not read" table, from its top. Open in the eboot:
   SpriteQuake's `:Param` layout, the shake's pattern table (whether
   `:Param` 0, 1 are pixels, as ours takes them), EffectStart's variant
   code.


**The boot (2026-10-18).** The eboot's boot (scene 7 then 1, 0x8103a7b0):
the logos `media/img/logo/` makerlogo_WH_vita (MAGES.), brandlogo_5pb_vita,
Navel_logo, criware960x544sw, each on white (0x810815bc, drawn 960×544),
alpha +6 % an update at 30 updates a second (17 updates, 567 ms), held
4000 ms from the fade-in's end, circle (input bit 0x1000, or 0x100)
ending it after 3000 ms, faded out the same (0x81081c16); then, at every
boot, the opening OP00.usm, stopped by circle or bit 0x10 (0x8108205c),
then 30 updates of black (0x8108237a) and the title. Vita3K's log has
the logos opened 5.22, 5.13, 5.17 s apart; our trace 5.21 s each.
Ours: `Script::boot` (#425), before the port's notice and the title.
Not converted: the Vita's save data check between the logos and the
movie (scene 0xf, "There is no saved data" / "Loading complete"), and
OP00 replayed from the title (its result 4 → scene 1; likely an idle
timeout, not read).

**CharacterStop and FlagSwitch_Tag's operator 6 (2026-10-18).**
CharacterStop (task_run case 0x24) stops every character action and
settles every move (0x81127620 → 0x81000c10 per slot): ours stops each
character motion still running (`spmotion N`), which leaves a move
(a loop-2 `spmotion`) at its end and an action back at its place, as
CharacterActionEnd. Its 2 uses (S03C_11C) follow a two-character
CharacterMove `:Wait 0`. FlagSwitch_Tag/GlobalFlagSwitch_Tag with no or
an unknown `:Operator` is operator 6 in the eboot (scene_load), a
three-way branch on `:NextTag`'s three tags (flag <, ==, > `:Param`);
ours emits the three Ifs. No script uses it.

**SE fades and waits, the window's fade-out, characters leaving at once
(2026-10-17).** scene_load cases 0x37 and 0x38 (0x8102840a) read
SePlay's and SeStop's `:Div` and `:Wait` (0 by default); 0x8100da30 fades
the SE in (0x810370d4) or out (0x81037794) over `:Div` frames of 60 a
second when it is 1 or more, and task_run waits with `:Wait 1` while the
SE plays (SePlay, 32 uses) or until its fade-out ends (SeStop, 4). Now
SePlay `:Div` > 1 (31) is `dwavefadein`, SeStop `:Div` > 0 (279) a
`chvolfade` to 0 that stops the sound, and the waits a `delay` of the
SE's length (its ADX header) or of the fade. CharacterDelete's
`[CharaN]` `:FadeOut 0` (85; case 0x1c reads a character's `:Fade`, then
its `:FadeOut`, then the task's) takes that character away at once,
before the others fade. Window `:Visible 0` (case 0x51; 0x8112a19c: no
`:Time`, a step of 1/15 an update) fades the window out over 1/`:Time`
an update and task_run waits for it: ours `windowfade 0,ms` then `delay
ms` (500 ms without `:Time`, 1,508 of them), when a message showed the
window last. Checked with debug traces of M02_08 on, before and after
(same route, 21,408 pages compared): 383 pages wait 500 ms more (the
window), others an SE's length (875, 1095 ms, …); 387 `windowfade`, 112
SE `chvolfade` stops and 15 `dwavefadein` ran, no new findings.
**Backlog answers, the shakes' swing (2026-10-17).** BacklogMessage
(scene_load case 1: `:Message`, the answer just chosen; 56 uses) is a
`logadd` line. A shake's `:Param` 2-5 (0x81008a3c for EffectQuake,
0x81087114 for SpriteQuake, which reads the same layout in case 0x2c)
add a swing: per axis, sizes 2, 3 and half-wave counts 4, 5 over
`:Time`, each half-wave eased out to the size and back, every other one
the other way (a count of 0 or less: none); `:Param` 0, 1 shake along a
10-point table (±100 % of the size, ±5 × size/5 for a sprite). Only
shakes without 0, 1 use 2-5 (19 EffectQuakes, 4 SpriteQuakes), so ours
take the swing alone: EffectQuake a sine `screenshake` (speed 32768 ×
count / ms), SpriteQuake `spmotion` keys four a half-wave. Four
SpriteQuakes with a swing were stops before. SpriteQuake's `:Wait` on a
stop (40) has nothing to wait for (task_run case 0x2c: 0x811280a0 is
false once stopped). Checked with a debug trace from M05_02: its swing
ran as `screenshake 1,300,16,0,0,109,0,22,0,22,300` (one half-wave of
22 px down over 300 ms), no new findings.

**Guesses settled from the decompile (2026-10-17).** The title fades in
from black over 833 ms, not 500: once its pictures are loaded, the title
update (0x81085a4c) starts the screen fade object (vtable 0x8115c634,
global 0x814f2214), whose update (0x8104a056) adds 4 % an update to a
black cover's progress (25 updates of 2 frames). A Sprite, SpriteBG or
SpriteBS without `:Position` takes the place of the picture it replaces
(above, "Accuracy"); no script's `0.txt` changes for it.

**Zooms straight in time (2026-10-16).** 0x81086a0c steps a move's
zoom (+0x38, +0x48) by the raw frame count and hands only the place to
0x8103cf20's easing; we eased both with one `spmotion` curve. The 37
eased SpriteMoves that zoom are now keyed every frame, the place eased,
the zoom straight (`MoveArc { bow: 0 }`, pushed by the mages converter
only, so the other games' scripts are unchanged).

**Moves under the text (2026-10-16).** scene_load reads CharacterMove's
and SpriteMove's `:Wait` (1 by default: 0x811297f4 sets +0x588); with 0,
task_run (0x81021070) cases 0x1e and 0x27 go on to the next task at once
and the move runs on. CharacterMove `:Wait 1` waits until no character
moves (0x811275bc with -1), CharacterWait (0x23) until no character
moves, acts (0x81127760) or fades, SpriteMoveEnd (0x29) until its
`:Sprite` (-1: every picture) stops; skipping ends them. Ours were all
waited for: the 866 `:Wait 0` moves now leave out `spmotionwait` (IR
`MoveNoWait`), and CharacterWait, SpriteMoveEnd and a waiting
CharacterMove wait for the moves still running (`MotionWait`). A move,
action or new picture on a moving one first ends it where it goes.
Shortcuts: CharacterWait doesn't wait for fades (ours under the text are
`SpritesFading`), and CharacterActionEnd leaves moves running.

**SpriteDeleteBG always fades (2026-10-16).** scene_load case 0x26
(0x8102840a) reads `:Texture`, `:Group`, `:ReleaseTexture`, `:Rule`,
`:Time` and `:FadeWait`, never `:FadeOut`; 0x8100da30 case 0x26 deletes
the picture at once when `:Time` is below 1 and otherwise fades it over
`:Time` with its rule (`:Time` 30 of its frames by default, from
0x81129ccc). Ours followed `:FadeOut`, so 4 deletes vanished at once:
one with `:FadeOut 0` (S03_02A, under the text: `spfade`) and three
without a `:FadeOut` (S03_06G1, S03_06G2, S03_07A1: `print` with
the rule, 500 ms). Checked with a debug trace of S03_07A1: `csp 181`
then `print 5` (rule039, 500 ms) at line 248.

**The shakes' `:Fade 0` and `:Loop 1` (2026-10-16).** scene_load case
99 (0x8102840a; the record's defaults from 0x8112a26c: `:Fade` 0,
`:Wait` 1, `:Time` 30 of its 30 fps frames, `:Loop` 0) sets the loop
flag for a negative `:Time` or `:Loop 1` and then doesn't read `:Fade`
or `:Wait`; 0x8100da30 passes the fade frames (`:Time` with `:Fade 1`,
else 0) to 0x810089a4, which turns the shake off for a `:Time` of 0
without a loop. Before, ours always let a shake die down. Now `:Fade 0`
(1,054 EffectQuakes, 85 SpriteQuakes; SpriteQuake's modes 7-9 in
0x81120894, scaled by the frames left only in mode 8) shakes at full
size for `:Time` and stops: `screenshake`'s first and last range are
equal, and SpriteQuake's `spmotion` keys keep their size. A loop (30
`:Loop 1`) shakes at full size until EffectQuakeEnd and never waits.
The 4 `:Time 0` shakes without a loop (P04_01B, P04_01C, P04_02B), which
ours ran until EffectQuakeEnd, are a `screenshake 1` stop; the 21
without a `:Time` shake 1 s (was 0.5 s); the one without a `:Wait`
waits for its shake. Checked with a debug trace of S03_07A1: 鳴's
「いいい、すびばぜん…」 shakes (`screenshake 1,83,16,0,2200,2900,7,0,7,0`,
no end) through her line until EffectQuakeEnd's `screenshake 1` a page
later, where before the 83 ms shake had ended.

**CharacterOpen's own `:Color` (2026-10-16).** 17 uses (13 with alpha
0, before a CharacterMove that fades the character in) and one on
CharacterOpenBS. The eboot doesn't read it. scene_load's CharacterOpen
case (0x8102840a, cases 0x1b and 0x80) reads `Fade`, `FadeWait`,
`SpriteMax`, `Walk` and each `[CharaN]`'s keys, its `:Color` among
them, but no task-level `:Color`. So these characters open solid in
the original too, as ours do, and the notes are `NOT_NEEDED` rows.

**SpriteBG, SpriteBS, Sprite `:Scale` (2026-10-16).** 49 uses (15, 33,
1): close-ups, ×1.5-10, mostly zoomed out by a SpriteMove after.
scene_load (0x8102840a, cases 9, 10 and SpriteBS's) stores `:Scale` at
the task's +0xa0, which task_run puts in the sprite's scale (+0x48);
the quad (0x8103d53c) scales the picture's corners about its `:Center`
point (the centre for these), which `:Position` places. So the picture
centre goes to `:Position` and the picture is zoomed about it, as
SpriteMove's `:Scale` already was. Now shown that way with `spzoom` in
the same print (a 0 ms transition when it doesn't fade), and the
SpriteMove after it zooms from there. S02_09C's HIY01 (967×725, ×4,
centred at 1800,1800) now shows the star hair ornament the lines are
about. Before, the picture was unzoomed with its centre off the screen,
so the screen was black.

**The character keys (2026-10-16).** CharacterMove, CharacterDelete and
CharacterAction look a character up by its whole key (number, pose,
dress, face, size: 0x8111ffb4, 0x81006648, 0x81120254) and do nothing
when another picture is shown, but only when key argument 1 is not -1.
scene_load (0x8102840a, cases 0x1c, 0x1e, 0x20) fills the key only from
`:NumberDirect`; from `:Number` it reads the character alone and the
key stays -1 (the record's constructor, 0x811297f4), so the task takes
whatever is shown under the number, as ours does. The scripts never use
`:NumberDirect` (all 22,904 are `:Number`), so `:Number` arguments 1-4
of the three tasks (2,391, 880, 482 uses) are `NOT_NEEDED` rows; an
earlier note here had them as not harmless. (Applying the whole-key
rule anyway would have skipped 52 blocks, where the face or cut-in
named isn't the one shown.)

**Small coverage gaps (2026-10-16).** Read in the eboot:
- **SpriteMove `:Curve c`** (12 uses: blood splashes `bld*` zoomed ×2-2.5
  over 12 frames with ±3, S04_07B's `ef08` with 0.5; converted
  2026-10-16, IR `MoveArc`): stored at the task's +0x38, handed to the sprite's move
  (+0x148, 0x8100da30 → 0x810867c2) and applied each frame by
  0x8103cf20: with t the eased progress (the same ease_value types) and
  d = (dx, dy) the move, the position is start + t·d + h·(dy, −dx)/|d|,
  h = c · sin(πt) · |d|/2 (`_Sin(a, 0)`, `_Sin(a, 1)` of d's angle in
  whole degrees). So c > 0 bows to the left of the direction on the
  screen (a move up swings left), up to c/2 × the distance at
  halfway; the zoom stays straight in raw t.
- **CharacterOpen `:Chara%d/Center cx,cy`** (4 uses, all `0,1`, at y 720
  or 840): copied to the character (+0x1e0, +0x1e4; default 0,-1 from
  `task_charopen_new` and 0x8100322a) and used by the draw (0x8103d53c)
  as the point of the quad at the position: x from −w/2·(1+cx) to
  w/2·(1−cx), y likewise with cy, so 0,1 puts the bottom edge's middle
  there. Converted: with `:Center` the y counts from the picture's
  centre raised by cy × half its height (S04_17B's 京, S03_24's and
  S04_05A's cut-ins now stand on the bottom edge; they were drawn below
  the screen); cx is not read (always 0).
- **EffectEnd `:Delete 1`** (7, on effects 54 and 51): the EffectEnd task
  (0x8102119a, case 0x3f) waits for the fade-out with or without it, and
  then `:Delete` only frees the effect's slot (0x8103f59c). Nothing to
  convert.
- **Window `:WindowNumber`**: -1 (1,506 uses) is the constructor's
  default (0x8112a19c), the text window itself, with `:Time` its fade.
  n ≥ 0 (S01_17A, S01_19A) only sets the shown flag of extra text box n
  (0x81120abc, the boxes' text drawn by 0x8100c002), which the next
  message sets again; the window is left alone. Converted as nothing
  (S01_19A's `:Visible 0` no longer hides the window).

**Coverage gaps closed (2026-10-15).** From the list in Status:
- **CharacterOpen/CharacterDelete fades.** The records
  (`task_charopen_new` 0x8112963c, `task_chardelete_new` 0x81129758,
  read in `scene_load` 0x8102840a) step 1/9 per update by default;
  `:Fade F` steps 2/F (F × 30/60 updates: F counts 60 fps frames, as
  every other time), `:Fade 0` (and CharacterDelete's `:FadeOut 0`
  without a `:Fade`) is at once, and CharacterOpenBS never fades. Ours
  now fades over `:Fade` frames, 18 (300 ms) without one, instead of
  200 ms for all (CharacterDelete's per-character `:Chara%d/Fade` is not
  read: one fade per task).
- **`:FadeWait 0`** (16,428 uses: the picture fades while the script
  goes on). In `task_run` (0x81021070) CharacterOpen and CharacterDelete
  go to the next task at once when their record's FadeWait (+0x10) is 0,
  else wait for every fade (0x811274c4); the Message task (types 0 and
  2) waits only for its text objects, never for the pictures, so the
  text starts while the characters fade. Ours: CharacterOpen,
  CharacterDelete, Sprite, SpriteDelete and SpriteDeleteBG with
  `:FadeWait 0` are IR `Op::SpritesFading`, which conv-cons writes with
  CONscripter's `spfade` (in, and `spfade num,-ms` out, which then
  removes the sprite; engine 535e460), then `print 1`. A character
  changing (12,276 of the 14,146: an expression) goes to another slot of
  its priority, the new picture fading in as the old one fades out (the
  two linear, so the picture thins by a quarter halfway; the eboot's own
  mix not checked); slots in use are now skipped when one is chosen. A
  character with a motion, zoom or look, and SpriteBG/SpriteBS (which
  replace the whole picture: 890 uses), still wait. In `--auto-play`
  spfade is at once (engine 08a782a), so traces show the end of a fade.
- **`:Rule` masks.** A sprite record's `:Rule` N (scene_load: SpriteBG
  +0x18) picks texture 200000 + table[N] (0x811635fc: 0, 0, 1, 2, …,
  110), ETC.cpk's file N - 1, whose TOC ID k is `RULE<k>`: `RULE<N-1>`,
  0 a plain fade. InOut's `:Type` goes through the same table
  (0x81162e38, in 0x81049292), so it is `RULE<Type-1>` too: ours used
  `RULE<Type>`, and `:Type 0` (422 uses), a plain fade, used `RULE000`.
  Fixed; SpriteBG, Sprite, SpriteDelete and SpriteDeleteBG with a
  `:Rule` now fade through it (a transition, waited on even with
  `:FadeWait 0`).
- **Message `:FontScale`** (435 uses; scene_load keeps it at +0x8fc
  of the message record): the whole message at that scale, as a
  `!z<percent>` text style (150 for the scripts' `1.5 // font 32`).
  Line breaks at the large sizes are CONscripter's own; not compared
  with the Vita.
- **`:Chara%d/Copy`** (51 uses): the eboot adds Copy × 1000 to the
  character number; ours keys the character the same way (`c1300`,
  `c2300`), in CharacterOpen(BS), CharacterDelete, CharacterMove and
  CharacterAction, so S03_02A's two cut-ins (character 300, Copy 1 and
  2) show side by side.

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
- **Findings audit (2026-10-18, seed 20261018):** 10 "seen" rows and 10 dropped-argument rows re-derived from the decompile: 17 confirmed, 2 stale (FlagSwitch_Tag, drawn twice: its converter cell), 1 unsupported (CharacterStop is not "nothing": the eboot stops every character action and settles their moves; left inferred, converter gap), none wrong. CharacterOpen `[CharaN]` `:FadeWait` and CharacterMove/Action `[CharaN]` `:Priority` are never read by the eboot: now seen.
