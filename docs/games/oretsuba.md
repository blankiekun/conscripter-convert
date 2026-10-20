# 俺たちに翼はない (PS Vita) (5pb./MAGES "TaskList" ADV)

The game's committed record: formats, how its features map to
CONscripter, the routes and progression variables, and its status. The
general process is in `docs/new-game.md`, general lessons in
`docs/lessons.md`. `<pkg>` is the folder holding the game's Vita
package and its license; `<game>` the decrypted app folder.

## Scope

The **PS Vita version** (Prototype's port of 俺たちに翼はない R plus
林田美咲's route), the whole game **with extras**. Nothing dropped.
Menus: the Vita's look with mouse buttons of our own. Output 960×544.

## Build

```sh
conscripter-convert mages check <game>
target/release/conscripter-convert build <game> -o out/oretsuba-cons \
  [--scenes S01_01A] [--webp] [--dry-run]   # = mages convert --title 俺たちに翼はない
```

Full build about 4 minutes, a 1.7 GB `arc.nsa` plus 12 MP4s.
Web build: loose files (`mages convert <game> -o out/oretsuba-web
--loose`, 2.2 GB, 27,472 files), then `docs/new-game-reference.md`, "The web build", with
the index made `--lazyload`.

### Decrypting the package

`<pkg>` holds `俺たちに翼はない.pkg` (3,267,708,688 bytes, content ID
`JP0745-PCSG00299_00-ORETSUBA0000VITA`, APP_VER 03.010) and `work.bin`
(512 bytes, its license). Two layers: the package (AES-CTR) and PFS.

1. zRIF from the license: `python3 rif2zrif.py work.bin` (in
   github.com/lusid1/pkg2zip).
2. `pkg2zip -x 俺たちに翼はない.pkg <zRIF>` (pkg2zip built with `make`)
   writes `app/PCSG00299/`, still PFS-encrypted.
3. PFS: Vita3K's fork of psvpfsparser (github.com/Vita3K/psvpfstools,
   with submodules) has the F00D step built in (no web service). Build
   the library with cmake (`-DCMAKE_POLICY_VERSION_MINIMUM=3.5`) and
   link a three-line `main` that calls `execute(zrif, src, dst,
   F00DEncryptorTypes::native, arg)`: `pfsdec app/PCSG00299 PCSG00299_dec
   <zRIF>`. It reports "keystone: matched retail hmac"; `media/SCRIPT.cpk`
   then starts with `CPK `. `PCSG00299_dec` is `<game>`.

## Executables

| File | Version | sha256 |
|---|---|---|
| `eboot.bin` (NPDRM SELF; addresses are those of the decrypted `eboot.elf`) | unknown (PCSG00299) | `56ebe9ed1a64d5e9e8d3af5aef0ce4ea21cf35ebffcf4c6cfae49dc6db343bd6` |

The eboot is read for the few tables the data lacks. Decrypt it to an
ARM ELF (the way Vita3K's `decrypt_fself` does; klicensee from
`work.bin`, key revision 5; the keys are read from Vita3K's
`vita3k/packages/src/sce_utils.cpp`, which can be fetched alone):

    python3 -I research/tools/ghidra/self2elf.py <game>/eboot.bin work.bin <sce_utils.cpp> <work>/eboot.elf
    research/tools/ghidra/ghidra_export.sh <work>/eboot.elf <work>/ghidra \
        --names research/oretsuba/names.tsv \
        -- -loader ArmElfPrxLoader -postScript ThumbPointerFunctions.java

The loader is the [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux)
extension (the release for the installed Ghidra, unzipped into Ghidra's
user `Extensions` folder); it names the imports in its NID list.
`ThumbPointerFunctions.java` makes a function at every Thumb pointer
stored as data (C++ virtual methods). About 4 minutes the first time.
Code loads at 0x81000000 (file offset 0xe0), data at 0x8115e000 (file
offset 0x15d440). The decompiled C shows ASCII strings inline
(`"TaskList/Task%d/Tasktype"`): grep `decomp/` for them. Every function
identified is named in `research/oretsuba/names.tsv` (the reference for
addresses: task switch, effects, eye catch, skip dialog, sound channels,
config defaults, music room); unknowns are in
`docs/games/oretsuba.findings.tsv`. `research/oretsuba/disasm.py` gives
a linear Thumb-2 listing (capstone). Shaders: `research/oretsuba/gxp.py`
cuts every GXP program out of the ELF; `research/oretsuba/gxp2glsl/`
builds Vita3K's shader recompiler alone and prints one as GLSL
(its `CMakeLists.txt` says how).

Tables found in the eboot and used by the converter:
- Music room: int32 BGM.AWB IDs at 0x8116310c (67, in the order of
  `mu_bg.png`'s titles); a second 8-song table follows (no titles; left out).
- Movie room: 100-byte name records at 0x81163348 (OP00-OP03.usm).
- Sound: music alternates channels 4 and 5, each its own CRI AtomEx
  player; BgmPlay cross-fades over `:Div` frames, a channel already
  fading out fades on under the new music (`bgm_start`, `bgm_stop`).

## Files and formats

| File (`<game>/media/`) | Format | Contents |
|---|---|---|
| `SCRIPT.cpk` | CPK, TOC (names + IDs) | 499 scripts: `adv.txt` (start scenes, flags, windows, characters), `scn.txt` (scene numbers), `chrNNN.txt` (sprite tables), `chract`, `CGMODE`, 494 scenes (`S*`, `P*`, `M*`) |
| `CG.cpk` | CPK, TOC | 946 PNGs: backgrounds, event CGs (`BG_BLACK` is ID 0) |
| `CHR.cpk` | CPK, TOC | 1,333 PNG standing pictures |
| `ETC.cpk` | CPK, TOC | 124 PNGs: transition rules, effects |
| `SystemData.cpk` | CPK, TOC | 182 PNGs: menus, windows, logos (the system screens are pictures, drawn by ID) |
| `BGM.AWB` | CPK, ITOC only (IDs) | 96 ADX tracks, 48 kHz stereo, loop points in the header |
| `SE.AWB`, `SystemSE.AWB` | CPK, ITOC only | 701 + 2 ADX sound effects |
| `VOICE.AWB` | AFS2 | 26,500 HCA voices (22,050 Hz mono, not encrypted) |
| `data/*.usm` | Sofdec2 USM | OP00-03, Poem, 7 heroine endings |

Nothing is CRILAYLA-compressed. ffmpeg decodes ADX and HCA;
`research/oretsuba/cpk.py` lists and extracts all three containers.
Manual: `sce_sys/manual/001-010.png` in the app (Japanese).

## Script language

Plain Shift-JIS text, one scene per file: a list of `[TaskN]` blocks
(`:Tasktype "Message"`, …) with `:Key value` parameters. Every number
is a file ID in an archive (the scripts' comments confirm it:
`:Texture 0 // BG_BLACK`):

- `:Texture N`: CG.cpk ID N. `:Voice N`: VOICE.AWB; `:SE N`: SE.AWB; BGM: BGM.AWB.
- `:Number c,pose,dress,face,size` (CharacterOpen): the `chrNNN.txt`
  whose `:CharacterNumber` is `c`; its `[SpriteN]` with `:Character
  pose,dress,face,size` gives the CHR.cpk ID.
- Coordinates 1280×720, origin top left, `:Position` is the picture's
  centre. Times are frames at 60 fps.

Tasks by use over the 494 scenes: Message 65,526 · CharacterOpen 16,954 ·
SePlay 4,739 · SeStop 2,884 · SpriteBG 2,843 · CharacterAction 2,312 ·
CharacterActionEnd 2,212 · BgmStop 1,672 · Window 1,570 · BgmPlay 1,564 ·
SpriteBS 1,415 · CharacterDelete 1,414 · Tag 1,195 · EffectQuake 1,077 ·
InOut 975 · CharacterOpenBS 895 · Sprite 850 · EyeCatch/End 847 ·
CharacterMove 838 · CharacterWait 651 · FadeOut 590 · FadeIn 585 ·
NextScene 537 · SceneJumpDialog/End 447/531 · End 469 · Flash 461 ·
InOut_Act 443 · TimeWait 406 · SpriteMove 389 · SpriteDeleteBG 278 ·
EffectStart 208 · then 25 more under 200. The eboot knows 111 task
types; the scenes use 52, all converted (`crates/conv-engines/src/mages/`).

Control flow: `Tag` (`:Name`) is a label; `Jump_Tag` jumps;
`FlagSwitch_Tag`/`GlobalFlagSwitch_Tag` (`:FlagName`, `:Operator`,
`:Param`, `:NextTag`) jump on a condition; `FlagOperation`/
`GlobalFlagOperation` (`:Action "=",1`) set flags; `MessageSwitch_Tag`
is a choice (`:AnswerMessageNumberN "<tag>","<text>"`, `:MessageNFlag
"<flag>","+",1` adds points); `NextScene` goes to another file;
`SceneJumpDialog` offers to skip a scene already read (`:JumpNextTag`,
`:NextTag`, `:UnreadTag`). Messages hold no markup but `\n` (two
characters, a new line).

Script errors the eboot can't read either: five tasks with a missing or
misplaced `[TaskN]` header (S01_04L, S03_03J2, S03C_03C twice, S04_03D;
skipped, as the eboot does), a SeStop inside S01_02A's EyeCatch (missing
`}`), S03_06E2's task with two `:Tasktype`s (the first is taken).

## Story structure and walkthrough

Read from every scene's NextScene, flag switches and choices (`mages
check` parses them). One person's four personalities:

1. **Part 1, 羽田鷹志** (S01_01 … S01_05): choices give `asu_pts`,
   `miy_pts`, `miya_flag`. At S01_04O, `miya_flag` ≠ 0, `asu_pts` ≥ 2
   and `miy_pts` ≥ 1 set `one_second`.
2. **Part 2, 千歳鷲介** (S02_01A … S02_10): `hiy_pts`; at S02_06H,
   `one_second` = 0 and `hiy_pts` ≥ 4 set `two_second`.
3. **Part 3, 成田隼人** (S03_01A …): if a second half opened,
   `kyoutsu_rt` = 1 (the common route, part 3 short); otherwise choices
   give `nar_pts`, `nar_sister`, and S03C_01D (黙る) opens 亜衣's route
   (`ai_rt`). S03_07A1 sets `three_second`.
4. **Part 4, ヨージ** (S04_01A … S04_06J1), which picks the second half:
   - `one_second` → S01_05A: 明日香 (S01_16A) or 京 (`miya_clear` and
     `miy_pts` ≥ 2 at S01_10F, S01_18A); at S01_14I, `asu_pts` < 4 →
     美咲 (M01_01 … M09_02). S01_19A sets `one_clear`.
   - `two_second` → S02_10A: 日和子 (S02_16A); S02_17A sets `two_clear`.
   - `three_second` → S03_16A: 鳴 (S03_22A); `ai_rt` → S03C_06A: 亜衣
     (S03C_20A); S03_23A sets `three_clear`.
   - none → S04_06L, a bad end.
5. **True ending, 小鳩**: with the three clears, S04_05I offers
   救いをもたらす → S04_06J2 … S04_28B, S04_29A (`four_clear`).
6. **After-stories** P01-P04 (明日香, 京, 美咲, かるお): start scenes 1-4
   of adv.txt, title extras; each sets `Pre_<name>`.

Route check: `research/oretsuba/nstrace.py` runs a `0.txt`'s control
flow along choice paths (globals kept from run to run) and reports the
ending; all eight endings are reached with:

    python3 -I research/oretsuba/nstrace.py out/oretsuba-cons/0.txt --quiet \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:1,S01_08D4:2,S01_09C0:1,=s01_19a" \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D0:3,S01_08D4:1,S01_09C0:2,S01_12B:1,S01_13B:1,=s01_19a" \
      "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D4:2,S01_09C0:3,=m09_02" \
      "=s02_17a" \
      "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:1,=s03_24" \
      "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:2,=s04_06l" \
      "S02_05A:2,=s03_24" \
      "S04_05I:2,=s04_29a"

(明日香, 京, 美咲, 日和子, 亜衣, bad end, 鳴, 小鳩, in that order.)

## Progression variables

Declared in adv.txt: 33 per-save `[Flag]`s (`%60`-`%92`, in adv.txt's
order) and 30 `[GlobalFlag]`s kept across playthroughs (`%410`-`%439`).

| Flag | Meaning | Set | Read |
|---|---|---|---|
| asu_pts, miy_pts, hiy_pts, nar_pts | affection points | choices | S01_04O, S01_10F, S01_14I, S02_06H, S03_05C |
| miya_flag, miya_clear, miya_h, miya_hflag | 京's route | S01 choices, S01_10G1 | S01_04A/K/O, S01_10F, S01_11A … S01_14G |
| one_second, two_second, three_second | a second half opens | S01_04O, S02_06H, S03_07A1 | S02_06H, S02_09F, S02_10, S04_06J1/K |
| kyoutsu_rt | the common route | S02_10, S03_05C | S03_*, S04_01A, S04_06J1 |
| ai_rt, nar_sister, HIY25 | 亜衣's route, 鳴's sister, 日和子 | S03C_01D, S03_03D/S03_05C, S02_15C | S03C_*, S04_*, S03C_02B |
| asuka_ed … misaki_ed, kobato_ed (global) | ending seen | the ending scenes | extras; S01_19A, S02_17A, S03_23A |
| one_clear … four_clear (global) | part cleared | S01_19A, S02_17A, S03_23A, S04_29A | S04_05I, the `NEXT_HINT` messages |
| Pre_asuka, Pre_miyako, Pre_misaki, Pre_karuo (global) | after-story read | P01_01D, P02_01E, P03_01D, P04_02B | the title menu |

## Testing

- Trace check: `engine/tools/cons_trace_check.py t.jsonl --script 0.txt
  --config research/oretsuba/cons_trace_check.json` (recipe in
  `docs/new-game.md`).
- Steered route runs: `research/oretsuba/routes.py <game dir> <renamed
  engine> <out>` plays the eight paths in one save folder (`--only
  9,10,11,12`: the after-stories). Run with a renamed engine copy. Each
  run ends at auto-play's loop guard (exit 1); the ending scene is what
  counts.

### The original under Vita3K

Shaky (the user): timebox it; prefer the decompile and our traces. Run
it only through `research/oretsuba/vita-emu.sh` (own Xvfb, private XDG
dirs, no D-Bus, URL openers shimmed; its header lists the options).

- Vita3K must be a source build with `research/oretsuba/vita3k-freed-texture.patch`
  (the game draws from a freed CDRAM block as S01_01 starts; unpatched
  Vita3K faults there; the AppImage only reaches the title). Clone
  github.com/Vita3K/Vita3K with submodules, apply the patch, `cmake
  --preset linux-ninja-clang && cmake --build --preset
  linux-ninja-clang-relwithdebinfo`; a folder `<v3k>` with `bin` linked
  to the build's `bin/RelWithDebInfo` and empty `lib/`, `share/`.
- Setup in a folder `<emu>` of its own (never the user's Vita3K dirs):
  `<game>` as `<emu>/home/data/Vita3K/Vita3K/ux0/app/PCSG00299` (`cp -al`),
  `work.bin` as `ux0/license/PCSG00299/JP0745-PCSG00299_00-ORETSUBA0000VITA.rif`
  (without it: `No klic provided`). No firmware needed. `config.yml`:
  `backend-renderer: Vulkan` (OpenGL fails under Xvfb), `show-welcome:
  false`, `warn-missing-firmware: false`, `check-for-updates-mode: 0`,
  `discord-rich-presence: false`.
- Keys by xdotool: `c` = circle (OK), `x` = cross, `Return` = start. A
  boot reaches the title by 40 s (circle at the save dialogs); keys that
  reach S01_01 with a save: circle every 2 s from 46 s, Return every 8 s,
  then circle every second; at a choice (task type 6) press Down, then circle.
- Reference trace: `python3 -I research/oretsuba/vpatch.py patch
  <emu>/home/data/Vita3K/patch/PCSG00299.txt` hooks `task_start`,
  `scene_load` and the frame queue to `printf` `@T/@M/@S/@F` records
  (Vita3K logs them as `TTY` lines); `vpatch.py log <emu>/logs/vita3k.log
  orig.jsonl` makes `trace_diff.py` records keyed `<scene>:<task number>`,
  the key of the converter's `--source-map`. Diff with
  `research/tools/trace_diff.py orig.jsonl ours.jsonl --map map.tsv --fps
  30` after taking the speaker (`【…】`) off our page text. The game runs at
  30 fps, about 1.1-1.4 s a page; route 4 (日和子) is 23,327 pages in
  about 9 hours, and diffs with no text finding.
- Memory dumps: `gdbstub: true` in `config.yml`, port 2159 once the log
  says `GDB Server is listening`, one connection only;
  `research/tools/gdbdump.py 127.0.0.1:2159 <prefix> --scan
  10000:fffe0000:10000` dumps the readable stretches. Breakpoints are no
  use (they re-hit without stepping).

## Milestones

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-13 | - |
| First dry run | 2026-10-13 | - |
| Story plays | - | - |
| Title, save, settings | 2026-10-17 | - |
| Extras | 2026-10-14 | - |
| Progression verified | - | - |
| Finished | - | - |

## Status

Paused 2026-10-20.

**What works:** all 463 scenes convert with no task left as a comment;
the full build lints clean and the route check reaches all eight endings
and the four after-stories. Messages, voices, sprites, characters and
their motions, transitions, effects, movies, the title, START's story
menu, the button bar, save/load, backlog, settings (the Vita's look and
defaults), the CG gallery, music room and movie room, boot logos. The
web build passed a headless browser test. A 23,327-page reference trace
of route 4 diffs with no text finding against ours.

**Missing:**
- A play-through of the web build (the definition of done).
- Arguments still dropped (dry-run notes "<Task> … ignored"; harmless
  ones listed in `convert.rs`'s `NOT_NEEDED`): SpriteMove `:Roll` (12,
  turns); SceneJumpDialogEnd `:Fade` (the eboot keeps the skip dialog up
  after the click, fading over `:Fade` frames, default 18); EffectStart
  `:Priority` and the particles' motion; `EyeCatch :Image 0,-1`'s colour
  panels (7); Window `:Time` with `:Visible 1` (2).
- Not converted: the save-data check at boot; OP00 replayed from the
  title (probably an idle timeout).
- Looks not checked against the original: text window place, character
  heights, eye catch speed, the skip dialog's font sizes.

**To resume:** decrypt the package (Build), run `mages check <game>` and
a full build, then play the web build through one route and fix what
shows.
