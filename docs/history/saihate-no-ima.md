# 最果てのイマ COMPLETE (Xuse): History

Done items moved out of [the game document](../games/saihate-no-ima.md)'s Status.

Finished work, moved out of "Status" so a session reads only the open
state. Items are referred to elsewhere as "History, item N".

### Original archives (2026-10-18)

`convert-game --original-archives` (`docs/original-archives.md`):
pictures, voices and effects are read in place from `HD/` and `CD/`
(page shots AE 0 against today's build, folder 1.4 GB → 426 MB).

### The auto waits as two rows (2026-10-18)

The dialog's テキスト page has two auto sliders (exe `0x4833a4`, set
up in `0x484d60`): 改頁待ち 文章単位, ten steps of 0-5000 ms (control
`0x424`, default 2.5 s, `0x51ed3c`), and 文字単位, ten steps of
0-500 ms (control `0x485`, default 50 ms, `0x51ed68`; History item 9's
0-100 ms is the text speed slider). The settings screen now has both
rows (seven rows, 50 px apart): オート（文章単位） (`Setting::AutoWait`,
500 ms a level, `%906`) and オート（文字単位） (`Setting::AutoCharWait`,
50 ms a level, `%913`), applied as
`automode_time (10-%906)*500,(10-%913)*50`. Defaults: levels 5 and 9,
the original's. Old global saves get level 9 once (`%914`). Checked
headless with the generated routine and a 10-character page in auto
mode (page/character levels: ms waited, expected): 0/9 5520 (5500),
10/9 534 (500), 0/10 5038 (5000), 4/7 4538 (4500); with `%914` unset,
`%913` became 9.

### Title intro (2026-10-18)

Read from the exe (`Title\title.cpp`; `research/xuse/names.tsv` has no
names for it yet). Title mode 0 (`title_task` 0x45b6f0) runs at every
title entered from the warning screen (0x450870) or the story (opcode
0x96, back-to-title 0x478300); mode 2 (0x45b720) is the way back from
the extras' sub-modes, without the intro. Its states (table 0x561ee0,
filled by 0x4f0a70; per frame 0x45b5d0), from black, with `Title.004`:

| ms | state | what |
|---|---|---|
| 0-2000 | 1-3 (0x459cd0, 0x459d50, 0x459e00) | background #0 cross-fades in (`CrossFade` 0x442c70, 2000 ms) |
| 2000-3000 | 4 (0x459eb0) | nothing |
| 3000-5000 | 5 (0x459fa0) | ripple under the logo, opacity 0 → 256, size 0 → 25 px |
| 5000-7000 | 6 (0x45a070) | logo #10 at (0, 171) fades in; ripple size 25 → 50 px |
| 7000-8100 | 11-13 (0x45a330, 0x45a4b0, 0x45a530) | ripple off; COMPLETE #14 at (100, 383); the four words fade in, 500 ms each, 0/200/400/600 ms late (table 0x4fb808) |

The ripple (drawn by 0x4599a0, Filter-Replace's pixel loop 0x43d9d0):
the background in the map's 590 × 262 at (0, 171), each pixel taken from
d·sin(2π m/256) px away along a direction `rand() & 0xff` (256 steps a
turn, sine tables 0x501538, 0x501938), m the grey of map #11; drawn over
the background at the state's opacity. A click or key in any state
jumps to the title at rest (flag +0x370). The title's music: BGM slot 13
(track 14) looped, from the title's start (0x459320,
`0x47fe30(0xd, ...)`), unless a sub-mode of the extras exists (0x51f858,
whose music goes on); Start fades it out over 2000 ms (0x45abe0). This
corrects the earlier "No title BGM" (item list below). Picture #9, drawn
at (680, 444) with the logo's opacity (0x459b90), is fully transparent.

Ours: `xuse/titleintro.rs` draws every frame at 30 fps into
`movie/title_intro.mp4` (522 KB, 8.2 s, silent), played as the title's
opening at launch and after a `reset` (mpegplay, click skips) with
`bgm14` under it; closing the extras goes back without it and keeps the
music. The movie's last frame against the engine's title at rest:
RMSE 1.8 % (H.264), 0.24 % of pixels over 3 %.

### Exe tables read at build time (2026-10-18)

The replay, gallery and staff roll tables were hand-copied into the
source (~700 lines). They are now read from the exe; the parse equalled
the copies entry for entry (all 418 scenes, every gallery slot, both
rolls' gaps and key frames), and every game's `0.txt` stayed the same.
The replay buttons' pictures are not in the exe's table and stay in
`replay::BUTTONS`.

### Status as it stood before the 2026-10-18 prune

The former Status section, kept whole. All its PRs (converter and engine) are merged;
its still-open items are restated in Status.


#### Queue (2026-10-16)

- Open: the launch warning's visitor counter ("あなたは N 人目のお客様です",
  History, "The launch warning"): not drawn, its gap left empty. It is the
  system data's u32 at `0x51eb50` (a FILETIME's low half at the first
  start, plus `rand() % 10000` each launch, 0x44f150) in `digit.png`
  digits at (492, 173) (0x488f30, proportional cells whose widths and
  alignment are in the digit object's setup, not read). Needs a global
  variable (alloc) and the digit metrics. User (2026-10-18): not worth
  more than a one-line fix, so left out. Likewise ENTER only: any click
  goes on, since a button there needs more `BootStep` fields.
- Open: the title's own intro before its buttons (title_task 0x45b6f0,
  the state machine at 0x561ee0: "click past the intro, wait ~14 s",
  "Testing") is not converted; ours fades the title in over 1 s.
- Open: branch coverage of the original (`docs/new-game.md` step 24,
  `research/xuse/cover.py`): one FIRST START xtrace run (390 pages,
  first links, 2026-10-17) hit 173 of 598 armed addresses (47 opcodes);
  the findings table's last column holds the marks. The debug menu
  (`0x30`, `0x62`, `LEFT_OUT`) never runs: the start enters `0001.cd`
  at #7, past the menu's message #1. Rows marked `never runs` there
  (0x44 ruby, 0x49, 0x4a, 0x7f-0x87, 0x91 …) are only not on this
  stretch of the story: later runs (`--save` or `--choices`) add to
  the marks (`cover.py mark t1.jsonl t2.jsonl --write`).
- Open: waited transitions take about 3 frames (90 ms) longer in the
  exe than their time, ours the time to the ms (trace diff, History,
  "Trace diff"): `0x76`+`0x71` 1000 ms = 33 frames (1,096 ms), 1500 =
  47, 2000 = 63, 3000 = 93; ours 1,009, 1,509, 2,009, 3,009 ms.
  A fixed lag of the exe's frame loop (the transition starts on the
  next frame, the wait ends a frame after it?), to be read in the
  transition task's code before adding it to every `print`.
- Open: at a scene's start the exe spends a frame on each `0x43` (arg
  1, no fade) and on the `0x76` arm and `0x22` call: 5 frames (166 ms)
  before `0004.cd` #8's background, ours none; 2 frames from an
  unskippable click wait `0x68` in auto mode to the next command.
- Open: the trace DLL's `--save` start (title load, slot 1) is still
  untested: it needs a slot save the game wrote as `xuse_save.py`'s
  template, and none is kept outside the user's folders.
- Open: the extras' CG gallery, scene replay and BGM room drawn by the
  engine (`screens/gallery.lua`, `replay.lua`, `replay_list.lua`,
  `music.lua` from `ExtrasMenu` by `crates/conv-cons/src/extras_screens.rs`;
  engine #77: `gallery`, item lists, tabs, picture radios, `bgm`), in
  place of the `*conv_cg`, `*conv_replay` and `*conv_music` loops; the
  unlocks are read from `?conv_cg_seen`, `?conv_scene_unlocked` and
  `?conv_progress` when a screen opens. Same frames as main's (25 Xvfb
  steps with everything unlocked: 0 px apart except the viewer, 137 px
  in its top-left corner, and the list, under 50 px). It also fixes three bugs that
  main has: a cell, a list or the CG button counted as unlocked only if the
  first CG or scene of its chain was (a false `if` skips the rest of its
  line), and closing a list left the menu over the list's darkened
  background.
- Open: `0x6f`'s presets 6-8 (3 pulses) as an untinted copy: their
  blend 6 tint draws nothing in the exe (`0x443090`); they were laid
  over in white before.
- Open: skip and auto mode end at a movie (`0x97`, 5 uses) and at an
  unskippable click wait (`0x68` arg 0, 4 uses), as in the exe
  (`op_97` `0x478ab0`, `op_68` `0x4784b0`): `skipoff:autooff` before
  them. Before, skipping ran on past them, and auto mode passed the
  click wait after 2 s.
- Open: the backlog drawn by the engine (`screens/backlog.lua` from
  `Backlog`; engine #75: a `list` of the text log, scrolling a page with
  ▲▼ and the wheel, the speaker's name over each page and its voice
  from the name), in place of the `*conv_log` loops; wheel down on the
  newest page closes it unless 「マウスホイールで履歴を閉じる」 is off.
  Same frames as main's (12 Xvfb steps, 0 to 673 px apart: the old
  `lsp2`'s rounding of the base's last row and column and of the track).
  `--auto-play` closes it by kind, so the bar's log button no longer
  traps a run there.
- Open: the settings drawn by the engine (`screens/settings.lua` from
  `SettingsMenu` by `crates/conv-cons/src/screens.rs`; engine #74:
  slider, toggle, radio, text buttons, the screen's pages), in place of
  the `*conv_settings` loops; `*conv_settings_apply` stays and runs after
  each change, and the screen closes with 1 (back to the title, or to the
  replay list) or 2 (quit). Same frames as main's menu (15 Xvfb clicks,
  0 px apart but the version line and the bar labels the old `btnwait`
  showed again); a level survives a load and a restart (`gloval.sav`).
- Open: `--auto-play` without `--trace-ignore` no longer opens the
  backlog from the bar and gets stuck there (page 173, `*conv_log_wait`):
  `ons_args` lists the bar's sprites 34-48 for `--trace-ignore` (engine
  #73: the lists add up with the command line's). 300 pages from the
  start play through.
- Merged: slides, zooms, flips, cube turns and melts as warp maps (`cons_warp.dll`, #227, engine #67/#69; maps in `warp/`, melt with a fixed edge, seed 1). Open: the plugins retired (engine #72): a build from before #227 plays them as cross-fades until rebuilt.
- Open: the save and load menus and their yes/no dialog drawn by the
  engine (`screen`, engine #70/#71; `screens/save.lua`, `load.lua`,
  `confirm.lua` in `arc.nsa`, from `SaveMenu`/`ConfirmDialog` by
  `crates/conv-cons/src/screens.rs`), in place of the `*conv_sl` loops.
  Same look (the 16-click Xvfb comparison against main's build: 0 to
  1,102 px a frame, all from two bugs of the old menu: column 799 that
  `lsp2` dropped, and the bar's labels its `btnwait` showed again). Saves
  of older builds ("No.NN - date|chapter") show the chapter (`%T-1`) and
  the save file's date. Slots older builds only flagged deleted
  (`%910+n`) show again (never deleted for the player). Settings' yes/no questions still use
  `*conv_confirm`.
- Open: screen positions and the blog bar's level number from the exe's layout tables (History,
  "Screen positions from the exe").
- Open: the exe mapping redone against `docs/new-game.md` §2
  (2026-10-17): `docs/games/saihate-no-ima.findings.tsv`, one row per
  opcode argument, branch, special value and timing rule, each seen,
  inferred or unknown with its address in `Farthest2015.exe`. The
  argcensus leads are explained: `csp -1` (all sprites) and
  `screenpulse`'s colour (`0xFF8080` is the preset table's COLORREF
  `0x8080ff`; `0xFFFFFF` goes with no tint). Two differences were fixed
  in their own PRs (`0x6f`'s blend-6 tint, movies and the unskippable
  click wait ending skip and auto) and a click skipping the effects up
  to the next text in a third (History, "A click skips the effects").
  Open, not built:
  - The volume curve of `sound_request` (`0x47aab0`) was not looked up
    (findings row "volume curve": unknown).
- Open: the hide button and right-click hide the link words, bars and
  name plate too (History, "Hiding the window").
- Merged: `0x86`'s two trailing bytes as harmless (#163; History, item 45).
- Merged: `0x8c`'s sort key as harmless (#180; History, item 31).
- Merged: `0x49`/`0x4a`'s edge colours as harmless but the grey one (#181; History, item 29).
- Merged: links only while their variable is set (#183; History, item 28).
- Merged: the debug menu's message block as harmless (#189; History, item 42); now left out with the menu's two opcodes (`LEFT_OUT`).
- Open: `0x49`'s grey outline as `!o#808080` (engine #62; History, item 29).
- Merged: the ruby alignment as harmless (#192; History, item 33).
- Merged: command coverage per operand, PR #85 (supersedes #28): every
  operand given and not read, whatever its value (0 included), is a
  dry-run note and a gap in `metrics.json`, except the filler (`NO_SUB`,
  `NO_ARG`) and the harmless table ("Operand coverage"), each row with
  the values it covers. Merged (#85).
- Merged: `0x7a`'s size at +14 as harmless (#164; History, item 44).
- Merged: `0x46`'s last u32 as harmless (#170; History, item 43).
- Merged: screen filters under the bust-ups, `screenfilter`'s new z (#177, engine #56; History, item 30).
- Merged: speaker names from `sub` bit 0, PR #102 (History, item 27):
  4,867 notes of 20 kinds left; 80,285 of 84,713 calls fully converted
  (was 49,328).
- Merged: the red fill over the static in 0006.cd as a screen filter (#157; History, item 40).
- Merged: `0x43` and `0x52`/`0x53` `arg` as harmless (#111, #114; History, items 32, 34).
- Merged: `0x90` +2 as harmless (#119; History, item 35).
- Merged: Filter-Replace's begin angle as `screenripple`'s start phase (#134, engine #50; History, item 38).
- Merged: Zigzag's phase from its frequency, its begin angle and reduction as harmless (#153; History, item 39).
- Merged: the exe map's fixes and missed callbacks (#46).
- Merged: the rotation's interpolation byte read (#162; History, item 41).
- Closed unmerged: `0x59` as a real cross-fade (converter #25, engine #18).
- Merged: `bitand` for the three bitwise ops (#74, engine #11).

Declared finished on 2026-10-14, then reopened the same day: the 0.19
debug trace and the decompiled exe (`research/tools/ghidra/ghidra_export.sh`,
names in `research/xuse/names.tsv`) showed opcodes read wrong from
`objdump`. All 26 items of that list are done, and so is everything the
debug trace found ("History", below). Items 28-45, from the operand notes, are done too (History); the one
operand note left is the debug menu's: its message (`0001.cd` #1, 60
bytes after it) and two opcodes (`0x30`, `0x62`), which `metrics.json`
counts as left out on purpose (`LEFT_OUT` in `xuse/build.rs`, by script
and instruction), not gaps: 84,710 of 84,713 calls converted, 3 left out.

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

None left.

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
- none left: `0x4a`'s two edge colours (none, then the default) are
  the usual decoration.

**Accuracy.**
- Hand-typed values (audit of 2026-10-17, History "Value audit"): every
  one in `xuse/` says where it comes from (`src:`, `guess:`, `ours`).
  The guesses a frame of the original would settle: the save menu's
  thumbnail (the exe draws it at (13, 52 + 92 r), `0x46ac50`; ours sits
  in the frame's box at (14, 54 + 92 r)), its "New" mark (the exe puts
  it after the slot's text) and the slot text (124, 11; 14 px, 15 px
  lines); the yes/no message's y (its font is now the exe's 16 px); the
  rows of text styles 3/4 (23 or 24 fit) and the name plate's font
  (20); the CG gallery's header (621, 122), headings (96, 97) and page
  marker (3 px up); the top bars' row colours and font sizes; the
  title's 1 s fade (its states cross-fade 2 s, `0x459d10`, and 1 s,
  `0x45a240`); the quake's hold and shrink; a picture the title
  draws at (680, 444), not identified and not drawn; the CG viewer's
  panel: the exe draws it whole at x 461 (`0x44dc90`, no source rect,
  clipped at the edge, 10 px past it), but the art meets the buttons at
  our 451, which stays.
- Layout left open by the exe-table check (History, "Screen positions
  from the exe"): the backlog draws one 720×20 line per entry at
  x 52, y 49 + 35·i, 15 shown (`0x41ffa0`, `0x420070`; ours wraps from
  (30, 44) at 24 px); the top bars' text origins (`0x496fe0`: scene name at (531, 6),
  ours 524; rows at +24, ours +19; `0x49b1a0`: chapter name at (73, 6),
  ours (72, 7)); the CG viewer's 21×21 button (image 9 at (566, 547),
  table `0x4fe3c0`) is not placed. Each needs a frame of the original
  before it changes.
- Settings defaults (2026-10-17, from `0x46d000`, the defaults; ranges
  from the dialog, `0x484d60`, `0x482750`): BGM and SE 0x99 of 255
  (level 6), voices 255 (10), text 20 ms a character (`0x51ed38`;
  `0x486540` shows one character each that many ms; the dialog's
  slider is 10 steps of 0-100 ms, so the game's own `text_ms` table
  100, 90, … 0 and level 8), auto 2.5 s a page plus 50 ms a character
  (`0x51ed3c`, `0x51ed68`, `0x476f50`: level 5 with `char_step` 10;
  History item 9 said 20 ms, which is the text speed), window level 0
  (the 50 % of `0x51ed12` is in the window picture). The original's
  two auto sliders (0-5 s, 0-500 ms) are one row here, so only the
  default matches exactly. Saves made before keep their levels (the
  defaults are set once, `%909`). Checked headless: at the start
  bgm 6, se 6, voice 10, text level 8 = 20 ms, auto 5, window 0.
- Transitions: MeltDown's stretch for subs 46/47 is scaled from the exe
  (filmed for item 25, not yet compared).

**Media and packaging.**
- `arc.nsa` is about 1.2 GB; if the web build can't handle it, split the
  voices into `arc1.nsa`.
- Check that saves, thumbnails (`savethumbN.bmp`) and `kidoku.dat`
  persist in the WebAssembly build.



**The launch warning (2026-10-18):** at every launch, before the title,
the exe shows `Entrance.cpp`'s screen (`gd.arc` → `Entrance.004`):
background (#0), the text (#3, the 18+ and content warning) at
(113, 166) (0x44f400), the ENTER button (#2, cells of 225×35: normal,
under the pointer, pressed) at (288, 446) (0x44f150); `window.png` (#4)
is empty and not drawn. It cross-fades in over 1000 ms (0x44f7b0, `mov
eax,0x3e8` before 0x442c70), waits for ENTER or the Enter key (0x44f550
cases 3/4, 0x44f3a0 bit 0x10000), cross-fades out over 1000 ms
(0x44f720) and starts the title task (0x44f6b0 → 0x45b6f0); a click or
key ends a cross-fade early (0x449c40). No brand logo or movie comes
before it; the opening movie (`mv0001`) belongs to the story's start.
Ours: one `Script::boot` step (#425), the picture composed with ENTER's
normal cell, ended by any click; trace: the fade-in ends 1044 ms after
the black. Not converted: the visitor counter (Status).

**Slot variables (2026-10-18):** the original keeps the bytecode's
bit-15 variables (the route and loop state, `sentaku_*`, `loop1`-`loop3`,
`clear_count`, …) in each slot save and the others (`war_clear`,
`azusa_tsuikabun`, …) in the system data (`load_svar` `0x46de00`, the
two writers `0x461790`/`0x45f1a0`); the conversion had them the other
way round, so loading an older save kept today's loop and route state,
and `war_clear` was lost to the `reset` after the epilogue. Now the slot
ones are locals (%10-%20, %29-%38) and the system ones globals
(%231-%233), with NScripter's own border (200, unchanged, so an older
`gloval.sav` still loads its CG, replay and settings data). Checked
headless: a slot saved with %17 = 5, %36 = 7, %231 = 3, then %17 and
%36 set to 99 and %231 to 4, loads back 5, 7 and 4. Older saves: a slot
save still loads at its page, but it holds no route state (its %10-%13
held the system variables, %29-%38 nothing), so the variables read 0
there and the next loop or route branch may not be the save's: a new
game plays right. The system variables start at 0 (they lived in the
slots); the slot variables' last values stay unused at %200-%220 of
`gloval.sav`.

**Trace diff (2026-10-17):** `--source-map` (each `0.txt` line's
`<script>:<instruction>`) and `research/tools/trace_diff.py` (wine
section, "Trace diff"). An `xtrace.sh` run from FIRST START (60 pages,
first links) against our dry run's `--auto-play 60 --real-time` trace:
the first 50 pages pair one to one by their message instruction, with
the same text on every page (speaker, spaces and line breaks left out)
and no instruction one side ran and the other didn't, but `2:74`
(`0x8f`, remove the bust-ups, run twice before any was shown: our `csp
-1` changed nothing, so no `ev`). The 8 time findings are the two open
items in Status. Getting there took an engine fix (CONscripter #139):
`--real-time` let auto-play click through effects, and with
`clickskip` every later one up to the next page, so our times were 0.

**Trace DLL (2026-10-17):** `research/xuse/tracedll/` (wine section,
"Trace DLL"; `docs/new-game.md` step 22). `xlaunch.exe` starts the exe
suspended and loads `xtrace.dll` with `CreateRemoteThread(LoadLibraryA)`;
it worked at the first try under wine 11. A run from FIRST START with
the first link taken: 200 pages in 768 s (3.3 s a page in auto mode,
30.1 frames a second as in the breakpoint trace), the first link page
(`0004.cd` #232, label-table index 6) after page 173. Taking it entered
the link scene at #1230, whose `0x47` returned to #232 after 4.3 s; the
page is then shown again (a second `page` record) and the run went on.
Two lines of it:

```
{"t":"cmd","pg":13,"ms":80440,"f":2415,"at":"4:36","op":"0x40","sub":19,"arg":1326,"src":822}
{"t":"page","pg":14,"ms":80440,"f":2415,"at":"4:36","voice":-1,"line":13,"segs":["忍","「え？」"]}
```

Dead ends: holding Ctrl (xdotool) or setting `skip_state_a` to skip
ahead didn't: with the first the run stopped at page 1 with the text
window gone, the second stops at
"未読テキストはスキップすることはできません" (unread text); start from a
save instead. The title needs its clicks retried: the first ENTER can
come before the window takes input.

**Breakpoint trace (2026-10-17):** `docs/new-game.md` steps 18-21 tried
here first. The relay log (step 18) gives the frame rate: `frame_loop`
(`0x406f30`) sleeps once a frame for the period from the table at
`0x4fb4e8` (66, 33, 16, 1 ms by the frame-rate setting `0x51ed11`): 30.1
frames a second. The breakpoint trace (step 19) timed `0004.cd` #13-#18
patched (`cdpatch.py`) to a quake 80/20×20, `0x66` 500, a quake 80/40×40,
`0x72` 1000, `0x76` sub 9 (3,000 ms) and `0x71`, from the click at #12;
the time is from one handler to the next, frames are the callback's hits:

| Command | Exe: frames, ms | Ours (`0.txt`) |
|---|---|---|
| `0x6d` 80, 20×20 | 19, 660 | 133 + 400 ms |
| `0x66` 500 | 16, 530 | `wait 500` |
| `0x6d` 80, 40×40 | 31, 1,058 | 133 + 800 ms |
| `0x72` 1000 | 33, 1,086 | `print` effect 1000 ms |
| `0x76` 9 (3,000) + `0x71` | 93, 3,075 | `print` effect 3000 ms |

Every waited command takes a frame more than its time, as the next
handler runs on the next frame (not modelled). The quakes confirm
the shrink, 50 px/s (the 40×40 one takes 398 ms more), and show a hold of
5 frames at the full range (Status). Our debug trace can't time these:
`--auto-play` answers `delay` at once, even with `--real-time`; the
numbers are the `0.txt`'s.

**Value audit (2026-10-17):** the hand-typed values in `xuse/` checked
against the exe's tables (`Farthest2015.exe`, addresses in the code's
`src:` comments) and the wine shots of the original, after #223/#221.
The text windows sit at the script's preset (`0x50`), not centred, as
the exe draws them (`0x4a1360`) and the wine shots show to the pixel:
the ADV box at (53, 449), not (47, 452) (that was a scaled
screenshot), its text at +(28, 21) (`0x517678`, was (89, 474), now
(81, 470)), the name plate at (32, 416) with the name at (56, 427);
styles 1, 2 and 7 move 7 and 12 px right with their text within a
pixel of before (the wine measurements matched the text, not the
frame); style 8's line gap 14 (was 13), columns 34 (8) and 24 (2/7)
from the text width over the pitch (a weight-600 font advances one px
more), the name plate only in styles 0 and 6. Also: "COMPLETE" on the
title at (100, 383) (`0x4599a0`); the save preview at (389, 136)
(`0x46ac50`); the yes/no font 16 px (`0x4a6410`, was 20); the backlog's
and chapter history's base at the window opacity (`0x41fc50`,
`0x468be0`: 127, was 210 darkened and 140); the scene list's rows (`0x4529c0`,
`0x453130`): cursor x 14, titles at y 64, colours #FFFFFF and #FFC8C8
(were 15, 63, #FEFEFE, #FEC7C7); the unpinned bar shows from y 571
(`0x51b610`, was 573). Matched: the extras, CG tabs and gallery
tables, the BGM room, staff rolls, replay lists, surprise presets and
text size presets, the bar's log/save buttons, the save rows and
header, the CG exit and page label, the replay close button.

**Screen positions from the exe (2026-10-17):** the hand-measured
positions in `convert/screens.rs` checked against the exe's layout
tables and constants (read from `Farthest2015.exe`'s data at the
addresses given). The user's screenshots are scaled about 1.78× and
not evenly (the save frame, always at (0, 0), fits about 1 px right and
down in the lower half; the title background about 4 px right near the
buttons), so the exe wins where they differ. Changed: title buttons at
x 626, option y 300, exit y 369 (table `0x4fb808`; the extras menu's
`0x4fb8a8` already matched); save menu buttons at y 530, x 511/614/701,
the page strip at (457, 479) in even 34 px cells (`0x469850`, marker
`0x46b3a0`); button bar from (304, 581) by table `0x4fa558`: auto 366,
skip 420, load 606, setting 681, hide 754; top bars' rows 77 px left of
the level box, x 533 (`0x496610`); backlog close at (732, 580) in the
frame's bottom-right tab, not the top bar (`0x41f4b0`); yes/no buttons
10 px either side of the centre and above the bottom (`0x4a6270`);
chapter history columns of 20 rows (`i % 20`, `0x4682a0`), not 19.
Matched already: the extras menu, CG grid (44 + 108c, 142 + 92r) and
pages (`0x4fb970`), CG viewer buttons (from (461, 545), `0x4fe3c0`), BGM
room (`0x5001d0`), scene list rows, the level box and its buttons. The
chapter history's other positions were measured on exact wine frames
and stay (the text windows: History, "Value audit"); what is left is
under "Accuracy" in Status.

**Level numbers (2026-10-16, reported by the user):** the blog bar's
current level was drawn at x 664, over the slash that the level box's
frame layer (`BlogStackBar.004` #14) holds at x 669-672, so "0/0" read
as "0 0". It is now at 647, 13 px left of the slash as the deepest
level is 13 px right of it. No check saw it: the slash is part of a
picture, and the trace and its checker see sprites and text, not what
a picture holds under them.

**Hiding the window (2026-10-17):** the original's hide
(`hide_window_until_input` `0x408860` → `toggle_ui_windows` `0x407690`)
hides the eight UI window objects of the table at `0x51c090` (text
window, both top bars, button bar among them) until a click. Ours used
`systemcall windowerase` alone, which hides only the text window: a
link page's words and underlines, the name plate and the bars stayed
on the screen. The bar's hide action now hides those sprites around it
(`menu::hidden_while_erased`, shared by every game with a button bar)
and shows them again after the click.

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
    Later (#368): the exe holds the full range 5 frames (4 points, then
    the frame that switches to shrinking, `cb_00441e50`; breakpoint
    trace, "Breakpoint trace" below). Ours now holds it 4 points plus the
    switching frame (166 ms).
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
   The one `0x49` record with `0x808080` (`s29_l1`, black "503 Service
   Temporarily Unavailable" on white) is a grey outline: the exe's edge
   (`0x489c50`) is the glyph dilated by a short at the style's +0x14/+0x16
   (1 px) with the glyph cut out. Converted (2026-10-16) as CONscripter's
   new `!o#808080` … `!o0` (engine PR #62), a 1 px outline in place of the
   shadow. Checked with `--goto s29_l1 --shot-pages 1`: 854 grey pixels
   on the page against 196 without the tag.
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

**A click skips the effects (2026-10-17).** In the original a click (or
Ctrl, or a key in mask `0x20ee007`) that ends an effect or a clickable
wait sets `skip_state_b` (`0x51f82e`; the wait callbacks of `0x66`,
`0x6d`, `0x6e`, `0x6f`, `0x72`/`0x74`, `0x76`, `0x8d`, `0x8e`, e.g.
`cb_004418b0`, `FUN_00475bf0`), and while it is set every later effect
and clickable wait ends at once (`op_6d`, `script_fade` and `op_76` don't
start theirs) until a message (`op_40_41`), `0x68`, `0x69` or `0x91`
clears it. No NScripter command tells a script that a `delay` or an
effect was cut short, so CONscripter got `clickskip` (define;
`getclickskip %v` reads it): with it a click that ends a `delay` or an
effect makes every later `delay` and effect end at once (drawn as
`print 1`) until the next text, click wait or `mpegplay`. The converter
writes `clickskip` (`Script::click_skips_effects`) and the skip gates of
quakes, flashes, pulses and script fades (`Op::IfSkipping`) also jump
while `getclickskip` is 1 (601 gates, +1,203 lines). Unclickable waits
(`0x66` with high half 0, `wait`) are not affected, as in the original.
Checked with a test script under `--auto-play` (each delay is clicked)
and 1,500 auto-play pages of the game: same pages and page pixels, 87
pages with shorter waits (the delays and effects after a clicked one),
no errors. Not checked with a real mouse in a window.
