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
- Sound channels (2026-10-19, `names.tsv` `channel_*`, `cri_player_*`):
  the sound manager at 0x814e907c is a list of channels, each its own
  CRI AtomEx player with a fader (`criAtomExPlayer_SetFadeOutTime`
  0x8109df04, `_Stop` 0x810a173e: the fader fades that player alone).
  Music uses channels 4 and 5 in turn. BgmStop (`bgm_stop` 0x81120910)
  fades out whichever of them plays over `:Div` frames (`:Div` 0: a cut).
  BgmPlay (`bgm_start` 0x8111ee38) plays on 5 when 4 is active (playing
  or fading), else on 4, then looks at the other channel: fading out, it
  is left alone and **fades on under the new music** at BgmStop's rate;
  playing and `:Div` > 0, it is faded out over `:Div` frames (a
  cross-fade); otherwise it is cut. A third track while both still sound
  restarts the channel it lands on (its tail is cut).

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
  MEM=3 SHOT=300 V3K=<v3k> research/oretsuba/vita-emu.sh <emu>/home 7200"
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

The whole route (2026-10-18): the same commands with 32400 for 7200
(9 hours; about 1.4 hours of it paused) give **23327 pages, S01_01 to
S02_17A**, the route's end. Diffed with `--pages 23332` (ours goes on
into a new game after page 23332) against the ours trace from before
#505, using #505's `vpatch.py`: **no text finding**; 1 extra
(S03C_03C:7700); 5 page (our `*_jump` pages at S01_02E:2990,
S02_03B:4050, S02_05E:3310, S02_06B:1580, S02_06F:3230); 721 missing:
0x21 (CharacterActionEnd) 643, SeStop 56, 0x27 4, EyeCatch 0x3e/0x3f
3 each, 0x2d 3, CharacterDelete 3, SpriteDeleteBG 2, CharacterAction
2, SpriteDelete 1, 0x1b 1; 733 quiet. The 0x21 count predates #505's
CharacterActionEnd wait. Ours again on main with #581 (route 4, 23200
pages): 0 text, 0 page, 0 extra; 93 missing: SeStop 56 (each right after
another SeStop, nothing playing), CharacterActionEnd 14 (the action
already waited for by CharacterWait, or never started), InOut once
(S03C_03C:4030, nothing staged after the headerless block the eboot skips), and
the 22 in Status.

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
build tested headless. Output 960×544. Dry-run gaps 947 (`:SpriteMax`
read as the eboot does: ignored on moves and actions).

**Open:**
- Looks against the original (needs Vita frames; Vita3K is shaky, so
  timebox it and prefer the decompile and our traces): text window
  size and place, character heights (where the eboot anchors them:
  `:Center` default 0,-1, where `OffsetY` comes in untraced), the eye
  catch's speed, the wave transition's size, the particles' motion,
  the skip dialog's and text's font sizes, CONFIG's lit cells (ours cut
  1-3 pixels short of the eboot's 90/180-wide cells at 0x8107145c).
- Reference trace (Testing, step 3): `EyeCatch :Image 0,-1` + `:Color`
  gives none of ours (S01_02C:960, S01_03B:490, S02_03G:3810), likely
  the eboot's two panels in that colour; SpriteMove 0x27 ×4 (S03_02A:6530),
  CharacterAction ×2, SpritePriority ×3, 7 deletes/opens act on pictures ours let go.
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
- Script errors the eboot can't read either (we convert or skip): five
  tasks with a missing or misplaced `[TaskN]` header (S01_04L, S03_03J2,
  S03C_03C twice, S04_03D; skipped because the reference trace shows the
  eboot does), a SeStop inside S01_02A's EyeCatch (missing
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
    conscripter-convert build <game> -o out/oretsuba-cons [--scenes S01_01A] [--webp] [--dry-run]   # mages convert --title 俺たちに翼はない
    conscripter-convert mages convert <game> -o out/oretsuba-web --loose   # web build, see History

Full build about 4 minutes, 1.7 GB `arc.nsa`. Debug trace of a build and
the steered route runs (`research/oretsuba/routes.py`, run with a
renamed engine copy; `--only 9,10,11,12` the after-stories): recipes in
"History", "Checking a build with the debug trace" and "Steered route
runs" (`engine/tools/cons_trace_check.py t.jsonl --script 0.txt
--config research/oretsuba/cons_trace_check.json`).

## History

Done items: [docs/history/oretsuba.md](../history/oretsuba.md).
