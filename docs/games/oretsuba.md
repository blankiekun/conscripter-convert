# 俺たちに翼はない (PS Vita)

The third game, started 2026-10-13. This is the game's committed
record: formats, how its features map to CONscripter, the routes and
progression variables, and its status. The general process is in
`docs/new-game.md`, general lessons in `PLAN.md` §13. `<pkg>` is the
folder holding the game's Vita package and its license.

- **Status:** starting (see "Status" below). Branch `oretsuba`.
- **Scope (2026-10-13):** the **PS Vita version**, the
  whole game **with extras**. Nothing dropped yet.
- **Engine:** 5pb./MAGES' "TaskList" ADV engine on CRI middleware
  (CPK archives, ADX/HCA audio, Sofdec USM movies). Scripts are plain
  Shift-JIS text, one scene per file: a list of tasks
  (`:Tasktype "Message"`, `"SpriteBG"`, …) with key/value parameters.
  Nothing in the eboot is needed so far.

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
      "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:1,=s03_23a" \
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

Not yet: a source-side simulator to diff page by page (step 11.4), and
the after-stories (P01-P04), which no menu starts yet.

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

## Status: first scenes play (2026-10-13)

`crates/conv-engines/src/mages/` reads the archives (`cpk.rs`), the
scripts (`tasks.rs`), lowers scenes to the IR (`convert.rs`) and writes
the assets (`assets.rs`; sounds become Ogg Vorbis through ffmpeg, BGM
with its ADX loop as `LOOPSTART`/`LOOPEND`):

    conscripter-convert mages list <game>/media/CG.cpk
    conscripter-convert mages extract <game>/media/SCRIPT.cpk -o <dir>
    conscripter-convert mages check <game>
    conscripter-convert mages convert <game> -o out/oretsuba-cons \
        --font <ipag.ttf> [--scenes S01_01A,S01_02A]

`mages check` parses all 463 scenes (65,485 messages) and finds every
texture, voice, SE and BGM ID they use in the archives.

Output: 960×544, the Vita's screen. Script coordinates (1280×720) are
scaled by 3/4 across and 544/720 down; CG and CHR pictures are already
at that scale, SystemData's are drawn at 1280×720 and are scaled by 3/4
(`meswin.png`, the text window).

| Feature | State |
|---|---|
| Messages, speaker names (from the scripts' `:Name N // name` comments), voices | converted |
| Backgrounds and sprites (`SpriteBG`, `Sprite`, `SpriteBS`, deletes), by priority | converted; positions not yet checked against the original |
| Characters (`CharacterOpen`, `…BS`, `CharacterDelete`, `CharacterMove` x only) | converted; height guessed (centre + `OffsetY`) |
| `InOut` transitions with the RULE masks | converted; `InOut_Act` as a plain fade |
| FadeOut/FadeIn, TimeWait, BGM, SE, Window hide | converted |
| Tags, jumps, flag switches and operations, choices with points, NextScene | converted, not yet checked against a walkthrough |
| SceneJumpDialog (skip a scene already read) | converted: a global "read" flag per scene (`%1200` + the scene's index in name order); a read scene shows its summary and このシーンを読む / このシーンをスキップする |
| Flash; EffectQuake/End and SpriteQuake (a screen shake dying down over `:Time`); SpriteMove (no `:Scale`, always waited on) | converted |
| CharacterAction (`chract`'s 661 keyframed moves: bows, shakes) | converted as straight moves between keyframes, waited on; the looping ones are left out |
| EyeCatch (a wipe with rule N to an eye-catch picture, then a fade) | a 0.5 s pause: the picture is not known |
| MoviePlay: the USM movies (MPEG-1 960×544 + ADX) demuxed (`usm.rs`) and made H.264/AAC MP4s in `movie/` (loose) | converted; `:Movie` N is the Nth of OP00, OP01, OP02, the seven `ED_*` (alphabetical), Poem, OP03 (each heroine's ending scene plays her ED) |
| EffectStart/End 53 (ノイズ: static at `:Param` density) and 42 (ブラー: a blur screen filter); SetSceneName (chapter title) | converted |
| EffectStart/End 57 (screen-capture sprite), 52 (光玉 light balls), 54 (紙吹雪 confetti), 51 (雪 snow), 40 | not yet |
| Achievement (trophies) | nothing to show |
| Title screen (`title_bg.png`'s picture and labels: START, LOAD, CONFIG; GALLERY greyed) | converted; the buttons' place is ours (the layout table is in the encrypted eboot) |
| Save/load, settings, backlog, extras | not yet (CONscripter's built-in menus) |

Next, in order:
1. Full build of every scene; fix what the lint and a headless run of
   the first chapter show. Text window layout and character heights
   from reference screenshots of the Vita game.
2. Effects: EffectStart 57/52/54/51/40, InOut_Act types, the eye-catch
   picture.
3. Walkthrough (this document), then the route check of the converted
   control flow.
4. System screens and extras (CGMODE, scene replay, music).
