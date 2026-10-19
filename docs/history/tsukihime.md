# 月姫 -A piece of blue glass moon- (TYPE-MOON): History

Done items moved out of [the game document](../games/tsukihime.md)'s Status.

- 2026-10-18: **the Ending List** (Gallery row 4, `cGalleryEnding`,
  `FUN_0010d5e8`) as the extras' replay (`PictureReplay`, with per-entry
  places, locked pictures and cursors added to `ReplayEntry`). Its
  table (0x4374E8, 25 × 12 bytes: scene, `_RDST` flag, voice) lists
  the lessons `QA_01`-`QA_25` (scenes 1000-1024): first the three
  endings' (`QA_12` Arcueid, `QA_23`/`QA_24` Ciel; `ending_icon_*`
  432 × 544 at (416 + 480 i, 120) in 1080p, `ending_icon_non` while
  unread), then the 21 dead ends' (`ending_badend`'s cells, 180 × 88,
  the read ones 296 lower, 8 across from (384, 680), 180 and 96 apart),
  then `QA_25`'s (a 112 × 112 hit box at (1700, 892)) while
  `all_read_check(1)` holds, here `gallery::ALL_LESSONS`, set after each
  lesson's `_RDED`. An entry is lit and chosen once its scene is read;
  choosing it replays the lesson (`FUN_00105040`: replay mode), which
  `_RDED` ends (`Op::SceneEnd`, now a `return` for the picture extras'
  replay), in the language last chosen. Guessed: the dead ends' button
  `ending_badendbtn` and `QA_25`'s `ending_btn` centred on their hit
  boxes, the endings' cursor (a white frame, ours), the fades. Left out:
  the voice an entry plays, the pad's cursor moves (`FUN_0010f6f8`).
- 2026-10-18: **screen shake start delay**. `_SHOK`'s argument 4 (260
  uses, 50 ms-3.95 s) goes to `screenshake`'s new `delay` (engine PR
  blankiekun/CONscripter#167): the shake holds still that long, then
  runs as before (`ScreenEffect::Shake::delay_ms`).
- 2026-10-18: **story movies**. `_TOMP(n)` (`FUN_0016ef30`) opens overlay 0 with n; its start
  (`FUN_001402a0`) plays the movie table's entry n (0x50B890, 0x30 bytes
  an entry, the path first; `exe.rs` reads it): 4 `prologue`, 5-6 the
  openings, 7 `phaseout`, 8-0x22 the 27 day cards, 0x23-0x25 the
  endings, 0x26 `TM_CI`, 0x27-0x2D `credit00`-`06`. `_TOMW` waits for
  it: ours is one blocking `mpegplay` at the `_TOMP` (the commands
  between run after it). `_TOMP` stops skip and auto mode (`DAT_00be0568`,
  `0670` cleared) except n = 7, which plays only while not skipping; no
  script uses 7. The frame step (`FUN_00140480`, read in the disassembly:
  its locals are floats the decompile drops) lets a press fade the movie
  out, but for the prologue, the openings and the endings only once their
  seen bit is set (`FUN_00192400` bits 2, 0, 1, 3, 4, 5; set as the movie
  ends, `FUN_001923c0`): ours keeps the bits as globals %1904-%1909 and
  plays those six unskippable until then. Left out: the 48-frame fade in
  (day cards), the 1 s fade out on a press, and the hold to the exe's end
  frame (day cards 420 frames against their 5.07 s, credits 618), and
  music 2 the prologue's end starts. The English copy plays
  `movie/en/<name>` where the fan patch's file differs in bytes (34;
  not `phaseout` and the credits). Checked: the dry run has 124
  `mpegplay`, every file in the full build; a headless scratch run from
  `01_00_ARC01_1` plays `phasetitle_01_00` for 5,138 ms of game time
  (156 frames presented; the movie is 5.07 s).
- 2026-10-18: **screen shake**. `_SHOK(x, y, period = 300, turns = 3,
  delay = 0)` (`cmd_SHOK` → `FUN_0012ee70` kind 1, drawn each frame by
  `FUN_0012ef30`): one sine for both axes, a turn every `period` ms
  (2π · frames / (period · 3/50)), x and y in 1280 × 720 units (scaled
  by 640/1280 and 480/720), times a gain falling straight from 1 to 0
  over the turns, then it ends. Ours: `screenshake 1` mode 0, speed
  65536/period, size (x, y) to 0 over period · turns ms, `end` the
  same. `_SHAF()` (`shake_stop`) clears slot 1; `_SHAF(ms)` is unused.
  Left out: the start delay (argument 4, `FUN_0012ef30` counts it down
  in ms; 260 uses, 205 of them 50-200 ms, 9 over 1 s; it starts at once
  here, so it needs a delay in `screenshake`), and the exe's "not while
  skipping".
- 2026-10-18: **`_WTTM`'s second argument** (14,821 calls, the biggest
  gap) changes nothing seen. The decompile seems to take the frame
  counter `DAT_00be0a00` (0x1473cde) off the wait when a second
  argument is there, but only two instructions touch it (the reloc
  0xbe0a00 is its only pointer; a scan of the code for loads of that
  slot finds the read in `_WTTM`, 0x17fda8, and the tick's add,
  0x181b9c): it counts frames from boot and is never reset, so the
  waits would be one frame or minutes. The original's trace says
  otherwise: over `00_00`'s 208 pages, 33 of the 46 `_WTTM(ms,1)`
  waits are within 2 frames of ms, 11 are 35-168 ms shorter, and the
  one `_WTTM(1000,0)` is also shorter (810 ms). So shortfalls happen
  with `0` too, after heavy commands (loading). Ours waits ms for both,
  as before; the argument is now read.
- 2026-10-18: **lesson read flags**. `_RDED(n)` (`cmd_RDED`) sets
  global read flag n when n is the current scene (+0xdc) with its
  per-play bit set; `_RTST(n)` (`cmd_RTST`) sets both for a lesson
  (0x215-0x22d), and every lesson runs `_RTST(n)` before its
  `_RDED(n)`, so `_RTST` keeps nothing and `_RDED(533..557)` sets a
  global (`convert::lesson_flag`: 976-999, 557 `gallery::FLAG_557`).
  `_IFUR(1, L, LV)` (`all_read_check(1)`) goes to L when lessons
  533-556 are all read, else on to `QA_25`'s `_RTM_`: here back to the
  title. The scenes' `_RDED(1..532)` stay open: only the Flowchart
  (`read_flag_global`'s gallery callers) reads them, and 532 more
  globals don't fit beside the read flags. The Ending List draws its
  21 entries from the `_RDST` flags (`FUN_0010dda4`, table 0x437510)
  and its `QA_25` icon from `all_read_check(1)`.
- 2026-10-18: **volumes and their fades**. `_SEPL`/`_MPLY` option 17
  (0-100, default 100, clamped by `FUN_0016a340`) is the volume the
  sound starts at (`chvolfade` before it); `_SEPL` option 1 fades it in
  to that volume. `_SEVL(ch)` (`FUN_0016ae10`, channel ch % 20) and
  `_MVOL` (`cmd_MVOL`) go to option 17 (none: 0) over option 1 ms:
  the exe's step a frame is |to − now| · 1000 / 60 / ms
  (`FUN_0019d0b0`, `FUN_0019ca00`; the mixer `FUN_0019b7f0` adds it
  each frame), so the fade lasts ms; `_SEVL` with no time steps 2000 a
  frame, at once (`FUN_0019d0b0` sets it at 1666.67 or more). Both are
  `chvolfade` (channel, or -1 the music). Left: the exe skips both
  while skipping or replaying, and `_SEVL` on a waiting loop only
  stores the volume (ours sets it if the channel's sound plays).
- 2026-10-18: **voices on or off per character** (T2; the user: on/off
  each, no volumes). **Seen**: `_VPLY` (`script_command`, case "PLY")
  looks its name's first 3 letters up in the table at 0x54D970 (55
  prefixes, `FUN_00102360`; one not listed is character 0) and plays
  it only while that character's bit (the mask table at 0xBE04D8's
  pointer) is set in the settings' +0x24; the bit's log2 is the row of
  `voice_bg` (`FUN_00129fa0`: 19 rows, the line at stage y 370.01 +
  50.61 n, each with a face of `conf_faceicon` and a name of
  `conf_name` from 0x44D500 and 0x44D5A0, a volume slider, a mute box
  at x 476.33 and a test). A00 is row 0 (遠野志貴), A10-A12 row 1, B90
  shares row 0, C50, C60, C80, C90, D*, M* are その他 (row 18).
  **Ours**: `Exe::voice_rows` reads the table, `build.rs` gives each
  voice its row as its character, and the Sound tab goes on over three
  pages of 7, 7, 5 rows (▲ ▼), each row's mute box live
  (`SettingValue::Voices`, `%970 + row`), the slider at 50 % and the
  test drawn at rest. **Checked** headless from
  `*tm_01_00_ARC01_1__LV00000`, 4110 pages: with `%970=1` the 604 A00
  plays are gone and every other prefix plays as often as without.

- 2026-10-18: **the settings** (`typemoon/settings.rs`, the shared
  `PagedSettings`; the system menu's Config tab and the title's
  Config). **Seen**: `FUN_0012c810` runs the screen, `FUN_0012aa80`
  draws it, `FUN_0012a550` puts the settings (0x56B080) into its
  widgets; each page a table at 0x56CDB0 (0x9D0 bytes: its rows, the
  control kinds, widgets 0x37-0x4D and 0x4E-0xBD, each control's top
  left on the stage, the sliders' most: 250, 250, 10000, 10000 ms; 100
  %); a slider's knob moves 169.33 (stage) and its − + step 1, or 50
  from a most of 1000 (`FUN_0012b930`). Tabs システム設定, サウンド設定,
  操作説明 (`btn_base0`) over `tab_bg`; the panel clipped to (407, 165,
  1402 × 858) of 1080p and scrolled. System: `system_bg` (メッセージ
  設定: unread and read speed, auto page and click waits, 既読シーンの
  オートスキップ, メッセージの早送り, the two text fade-ins); Sound:
  `sound_bg` (BGM, SE, system SE with mutes, an SE test) and `voice_bg`
  (すべてのボイスの再生, メッセージ送り時の音声停止, then a row per
  character); 操作説明: `conf_manual0_ja`. `effect_bg` is never drawn;
  `SYSMES_TEXT.DAT`'s config help lines (1-160) describe another,
  older screen (Voice, Text pages, 5.1ch, window design), not this one.
  Defaults (the reset, case 8): BGM 40, SE 40, system SE 10 %, all
  voices 100 × a character's 50 %, unread 35 ms, read 15, auto page
  3000, auto click 2200 ms, skip read text only, a click stops the
  voice. **Ours**: the System tab in two pages (▲ ▼; the panel cut at
  its line under the auto skip), the language as our own row under
  the last one (`言語 / Language`, with `--english`: a pick in game
  closes the menu and switches, on the title it is kept for the next
  start), 閉じる under the menu bar's buttons, the bar's Title live in
  game; the menu bar drawn as the Config tab shows it. Live rows: the
  unread speed, the auto click wait, 全文/既読のみ, the BGM, SE and
  system SE volumes, the BGM and SE mutes, all voices, the click
  stopping the voice. Checked headless: each page shot against the
  parts, the knobs on their lines at the defaults, a language pick
  closing with 3 in game only, the mutes flipping.

- 2026-10-18: **the gallery viewer fits the CG**. `FUN_0010be84`
  opens a CG at the smaller of the screen's width and height over its
  size (the slot table's 1080p size), larger or smaller, centred
  (+0x40, the zoom's floor; +0x44, its ceiling, 1:1 or the fitted
  zoom when that is above 1). Ours: each CG resized to fit 1280 × 720
  (`sys/gallery/fit/`, a new folder so a build's older `view/` files
  aren't reused), centred, so nothing scrolls. Checked headless:
  `IMG1685` (2106 × 1266) at 1198 × 720 from x 41, the bars black.
  Guess: black under it (the viewer's backdrop widget,
  `DAT_00566738` tinted 0.847 grey, not read).

- 2026-10-18: **the gallery's slot conditions**. A slot's +8 (0–19)
  goes through `FUN_0010b58c`: 0 (270 slots) and past 19 always, the
  others a scene-read flag (`FUN_001926b0`, the `_RDST` words 110 +
  n/16): 1–4 flags 2–5, 5 0x17F, 6 0xD0, 7–9 flags 7–9, 10 0xE2, 11–14
  flags 10–13, 15 0x113 or 0x114, 16 flag 14, 17 0x129, 18 300, 19
  0x135. `FUN_00104470` leaves a slot's CGs out while it fails. Ours:
  `PictureSlot::shown_if` (any of the `_RDST` globals; compact
  galleries only), 188 slots; every flag is set by one `_RDST` in the
  scripts. Checked headless (the listed count, read back): flag 557
  alone lists 270 slots; with flag 0x129 331 (+61), with 0x113 291
  (+21), with 0x114 and 5 319 (+21 + 28).

- 2026-10-18: **`_STCG` and flag 557 in the gallery's seen test**.
  `script_command`'s `_STCG(name)` sets the CG flag of the picture's
  index (`FUN_001774f0`; none past 0xA8C), as showing it would;
  `FUN_00104470` shows a CG when its pictures' flags, its own, or flag
  557 say so (557 for every CG, not only the 926 with no picture).
  Ours: `_STCG` sets a global per picture name (39, `%1804` up in name
  order, `convert::cg_flags`) and a CG's seen test is any of its
  pictures shown, those globals, flag 557 (`Seen::Any`). Of the 39,
  6 CGs read one (`IMG0758`, `IMG1685`, `IMG1686`, `IMG2605`,
  `IMG2606`, and `IMG3864` by `IMG2602`/`IMG2604`); the others (with
  `img2460`, `img2461`, `img2601`, `img2636`, which an earlier note
  said were flagged only by it) are no gallery CG and unlock none, so
  they change nothing in the gallery.

- 2026-10-18: **the CG gallery lists only the slots with a CG to show**
  (`FUN_0010b830`: a slot goes in the list when `FUN_00105118` finds
  one of its CGs shown). Ours: `PictureGallery::compact`, each redraw
  counting every slot's seen CGs and drawing those on the page asked
  for, closing up (still in pages of 6 × 6; the exe scrolls); the
  arrows only past one page. Checked headless: 45 slots marked seen
  (every other one of the first 90) fill page 1 and 9 of page 2.

- 2026-10-18: **the CG gallery** (title > Gallery; `gallery.rs`).
  **Seen**: the menu's widgets (`FUN_00106124`, descriptors at
  0x54E2F8: texture, place in 1080p pixels; `GALLERY_PARTS` textures
  7179 up in BNTX order): `gallerymenu_window` at (56, 0),
  `config_innerbg` at (365, 0), `underline` at (0, 72); the buttons'
  list (0x54E438) at x 80, y 160 + 80 a row, cut from `gallerymenu_btn`
  (CG Gallery, Music Player, Flowchart, Movie, Ending List, Title;
  at rest, chosen, lit). The grid (`FUN_0010af78`): 6 across from
  (392, 109), 240 × 139 apart, each slot's first CG to show by its
  thumbnail, cell n of `CG_THUMB_PARTS`' `cg_thumb_all` (36 a row,
  154 × 87 every 154 × 88; n the slot table's number, 0–1977 but 911),
  drawn at 1.5; `cg_cursor` (8, 8, 248 × 144) at (384, 103); the list
  only of slots with a CG to show (`FUN_0010b830`), scrolled. The seen
  test (`FUN_001051e0`): of a CG's unlocking pictures the first found
  by name decides, else any sprite's shot letter; then its own name.
  Ours: `PictureExtras` with the six rows (CG Gallery opens the
  gallery, Title goes back, the others drawn, not clickable) and a
  `PictureGallery` of every slot in pages of 6 × 6 (13 pages), a CG
  seen once CONscripter's file log (`filelog`, `fchk`) has one of the
  pictures that set its flag (1,051 CGs) or by flag 557 (926;
  `_RDED(557)` now sets it, `FLAG_557`, global 1003). The viewer shows
  each CG covering the screen (`sys/gallery/view/`, written by the
  build from `pic/` and `cg/`), scrolling to its far part. Guesses:
  the buttons' anchor, what is under the menu (black), the fades,
  the page buttons and Back (ours, in the font).

- 2026-10-18: **`_SELS`, the lessons' yes/no question** (`QACALL`,
  `question.rs`). **Seen**: `FUN_00192800` opens the choice in mode 2
  with two slots (index 0, 1) and no texts, the pick going into `F000`
  for `_GOTS(_D000, SEL_NO, SEL_YES)`; `FUN_00185900` draws it on the
  640 × 448 stage from `allui`'s `DIALOG_PARTS`: `dialog_BG` (a dim
  over the screen) and `dialog_win` centred, system message 0x5A5 +
  `F027` (`FUN_00183b20`; `SYSMES_TEXT.DAT`, read by `FUN_0019a420`:
  u32 languages, u32 messages, a u64 table offset a language, u64
  offsets to NUL-ended UTF-8) centred at y 224 − 42.31 (「ヘンテコヒント
  コーナー(第n回)に入りますか？」, the fan patch's "Enter hint corner
  (No. n)？"), buttons 0xD9 (index 0) at (241, 238.53) and 0xDA
  (index 1) at (327, 238.53), left and right moving between them
  (`FUN_00185690`): No on the left, Yes on the right. Ours: one
  `ButtonScreen` per lesson (`F027` 1–25), the question drawn into
  `sys/question/q<n>.png` (`en/` for the English copy) with the
  build's font. Guesses: the glyph size (22 px), the buttons' anchor
  (their top left), the fade (167 ms); the window's pop from 105 % left
  out.

- 2026-10-18: **the gallery's tables** (`cGallery`, `FUN_001042d0`
  builds it, `FUN_00104470` lists the CGs it shows), **seen**, read
  with `exe_call.py`'s loader (relocations applied):
  - 458 slots (`FUN_0010b790`) at 0x54EC88, 0x20 bytes each: a pointer
    to the slot's CGs and a condition (0–19, `FUN_0010b58c`: 0 always,
    the others a global read flag through `FUN_001926b0`, words
    110 + n/16; 270 slots have 0). A slot's CGs (1–65, 1,977 in all),
    0x20 bytes each until a null name: the name with a prefix
    (`" IMG1685.MZP"`, 1,148; `"-IMG3864.MZP"`, 829), a number (758;
    not the flag), and the size it is shown at on 1080p (2106 × 1266,
    3840 × 3915, …). Every `allpaccg` `IMG*` (1,023) is a gallery CG;
    the other 954 are story pictures in `allpac`.
  - 105 CGs (`FUN_0010b798`) at 0x5525C8, 0x28 bytes: a CG and up to
    four story pictures that unlock it (`IMG3954` by
    `SIE_A_63_00_N00`; a sprite's shot letter tried as A, L, M and S,
    `FUN_001051e0`).
  - A CG shows when its slot's condition holds and: one of its
    pictures (from that table) has its CG flag, or its own name does
    (`FUN_00175900`: its place in the picture names, `FUN_00177610`:
    the flag), or global read flag 557 is set (`FUN_001922c0(0x22D)`,
    words 300 + n/16; `_RDST(557)`, the last lesson `QA_25`).
    **Not settled**: 926 CGs (most of `allpaccg`'s) are in neither the
    unlock table nor the first 0xA8C picture names, so by this reading
    only flag 557 shows them, which looks unlikely. `exe_call.py` on
    `FUN_00175900` returned 0 for every name tried (the lookup needs
    run-time state); the lookup (`FUN_00156720`) is the next thing
    to read.
  - The CG flags (bits at 0x7A8, under 0xC80): showing a picture sets
    its own (`FUN_0015d3a0` → `FUN_001774f0`: its table index up to
    0xA8C; a sprite past that through the table at 0x7533C0, name →
    flag), and so does `_STCG(name)`.
  - Ours: `typemoon media` (and `convert`) writes `allpaccg`'s 1,023
    `IMG*` CGs to `cg/` (`cg/en/` the fan patch's that differ); the
    flowchart's thumbnails (`GALLERY_FC_TH*`, 495) and a dummy are left
    for the flowchart.

- 2026-10-18: **`_STCF` waits a frame more**: `FUN_00162f90` always
  returns 1 (wait) and the wait is checked from the next frame, so
  every `_STCF` takes a frame (17 ms) after the change it waits for,
  or one frame alone (the trace: a 1 ms fade 17 ms, 50 ms 67, 1000 ms
  1016). Ours: a `wait 17` after its `spmotionwait` (all 5,179 uses).
  Trace diff: `_STCF` `00_00:55` now `quiet`; missing 3 to 2, time 73
  to 72.
- 2026-10-18: **trace diff `quiet`**: `trace_diff.py` counts a
  `missing` command the map has lines from as `quiet` (counted, not a
  finding). 13 of the 16 `missing`: 11 `_FADS(1,0,0)` after an
  `_STCC` of a layer `_STCP(n, 0)` had hidden and faded out (or after
  `_SEPL`), and `_FADS` `00_00:57` and `2278`. Nothing changes in
  either game; the original's take its 3 frames (50 ms), ours its
  `wait 50` and a `print 1` the engine trace doesn't record (nothing
  changed). Missing 16 to 3.
- 2026-10-18: **the boot logos** (`typemoon/boot.rs`, the shared
  `Script::boot` steps), from the exe's `FUN_001b5650` (one state a
  frame) and its step table at 0x4C98FC (0x18 a step: picture, hold
  frames, frames before a press skips, …, the colour behind): 15 frames
  of black; the TYPE-MOON logo movie `TM_CI` (movie table entry 0x26,
  6 s, white at both ends) on white, a press ending it once 120 frames
  (2 s) have played; then the caution screen (`parts.mrg` entry 1,
  `caution2`, loaded by `FUN_001b4a20(1, 0x47, 0x1B)`) faded in from the
  white still behind it at +0.025 a frame (41 frames, 683 ms), held 480
  frames (8 s) or until a press (any time), faded out to black at the
  same rate; then the title. Only once scene-read flag 485 is set
  (`_RDST(485)` at the start of `00_01`): with no such save, the first
  launch, the exe skips them (state 1 → 0x10, `PTR_DAT_00be0e98` starts
  at 1). The fan patch has its own `TM_CI` and caution picture
  (`movie/en/`, `sys/boot/en/`), shown while the language (%1800) is
  English. Left out: state 0xD (`parts.mrg` entry 0, `logo_fix`, held
  8 s), reached only with `DAT_01a4df08` set, a BSS word nothing writes;
  the movie's volume setting (the settings' +0x30); the title's "PRESS
  ANY BUTTON" step (title state 0, needs the title IR).

- 2026-10-17: a looping motion whose layer goes to another IR layer at a
  `_FADS`, or whose IR layers change, goes on where it was (CONscripter's
  `spmotionclock`, 2,305 in the Japanese build); it started again from
  its start before. `_STMA(n, 0)` on a loop stops it where it is
  (`spmotion n,2`, 4 places), as the exe's stop leaves the layer; it went
  back to its place. `_STCH` and a new motion still stop a loop back at
  its place (the layer is placed or moved anew next).

- 2026-10-17: a motion field passed through (−32766), **seen**:
  `FUN_00197710` takes a field from the last key before the time that
  doesn't pass it through to the next, along the easing of the last key
  before the time that gives the field and an easing (`FUN_001961b0`),
  once across the keys between; mode 2 and the stepped fields (0x24,
  0x28–0x3C) hold the value before. `exe.rs` (`Motion::value_at`) works
  it out the same way; each passed-through key takes its value there,
  and a key's way that `spmotion`'s one curve would draw more than 1
  unit off is cut into 8 straight steps of the exe's values (fewer past
  64 keys). Japanese build: 1,101 `spmotion`s changed, 12,387 keys
  added (94,302 → 106,689). Mode 2 on a passed-through key (594 keys)
  now holds the value before, as the exe, not a straight midpoint.
- 2026-10-17: looping motions, **seen**: `FUN_001999a0(m)` (a switch
  compiled to jump tables, so `exe.rs` runs it: `run_switch`, a few
  AArch64 instructions) gives where motion m loops from: 0 not at all
  (22,165), 1 the start (1,805), else a time in ms (222; 206 at a key).
  `FUN_00197b80` wraps the time past the length back to it. The
  converter writes `spmotion` loop 1, or 4 from the key nearest that
  time. The loop stops (`FUN_00199620`, `FUN_00160840`): `_STMA(n, 0)`
  (4 of its 92 uses stop a loop; the rest stop motions that have
  ended), `_STCH`/`_SCH2` on the layer (at once; with argument 18 the
  copy's loop goes on, here started again), another motion, the layer
  leaving the screen. Japanese build: 4,211 looping `spmotion`s (with
  the children's and the restarts), 4,057 stops. Checked: tests, the
  dry run, a trace from `tm_start_script`.

- 2026-10-17: motion easing, **seen** (`FUN_00130c80` = `FUN_001961b0`,
  `FUN_00197710`): a key's easing 0x04 (2–41: quad, cubic, quart, quint,
  sine, circ, expo, elastic, back, bounce, each in, out, in-out,
  out-in; Penner's, with the exe's constants: expo's 1.001, elastic's
  periods 0.3/0.45, back's 1.70158) shapes the way from it to the next
  key, until a key sets another; 0, 1 and the rest are straight. Each
  `MotionKey` carries its curve (the shared IR change, #364) and the
  emitter writes an `spcurve` table per curve used (31 in the Japanese
  build): of 87,146 `spmotion` keys, 19,524 are eased (13,065 quad
  out). `_STGS`'s easing (argument 4, 27 uses) the same. Checked: the
  curves against the decompile's formulas (`ease.rs` tests), the dry
  run, a trace from `tm_start_script` (3,097 pages, no script error,
  the same findings as before).

- 2026-10-17: `_STEC` exact, **seen** in
  `nexl_nxshader_pre_alter.bnsh`: the sampled texel (the sRGB texture,
  decoded by the sampler) is raised to 0x3EE8BA2E = 0.454545 and × 255
  before the levels (`lg2`, `ex2`), the output only clamped and ÷ 255;
  mode 3 (effect 82) keeps G and B at 0 where the texel's are. Converted
  with CONscripter's `spfilter` kinds 12–14 (engine PR 123: the levels
  on 255·srgb⁻¹(v/255)^(1/2.2) in float, one rounding; kind 13 the grey
  of those; 14 the zero rule): every effect 2–82 now decodes first, even
  with identity levels (an identity effect lifts 10 to 18, 30 to 35).
  Checked in the engine on test ramps against the formula in float64:
  exact, but for 30 of 7,680 grey pixels a level off at a .5 boundary.
- 2026-10-17: coloured panel frames (`_STCH` argument 17 1–6, 263
  uses), **seen** in `FUN_00189ad0`: the picture is cut at the frame's
  inner edge (the scissor), then the corner and edge pieces are drawn
  over it in the colour (alpha, r, g, b to `FUN_001b03e0`: 1 and the
  default black, 2 white, 3 red, 4 0.18, 0.07, 0, 5 0.97, 0.96, 0.91,
  6 1, 0.91, 0.82; case 1's r and g are a register Ghidra shows as a
  handle, read as 0): `alphagradation` from the rectangle's edge, then
  `_inv` one frame in; the scissor keeps only the `_inv` part, the
  colour fading from the cut inward over the frame width. Converted
  with CONscripter's `spclip` feather from a colour (engine PR 122):
  1,119 `spclip`s of the Japanese `0.txt`. Checked on page 14 from
  `tm_01_00_ARC01_1__LV00000`: the panel's right edge at x 664 goes
  black over 20 px (x 655: 57 of 149 without the frame; 3t² − 2t³ of
  8.5/20 gives 58).
- 2026-10-17: panel frames and `_FADS` type 1, **seen**.
  `FUN_00189ad0` picks a frame piece size by the frame width (≥ 10,
  32, 40, 64, 128, 256 px: tables 0x4528E0, 0x75BE00 (each size's
  3 × 3 cuts), textures 338–343, which `FUN_0013b9a0` loads from
  `allui`'s `BORDER_PARTS.NXGZ`: `alphagradation020`–`512`, then
  `_inv` and `_2`). With no colour (argument 17 0) it draws the layer's
  mask in blend 0 (replace): the inner rectangle, then the corner and
  edge pieces, so the picture fades in from the rectangle's edge over
  the frame width (the ramp is near 3t² − 2t³: 0.16, 0.52, 0.87 at a
  quarter, half, three quarters). Converted with CONscripter's new
  `spclip` feather (engine PR 118): 6,846 `spclip`s of the Japanese
  `0.txt` are feathered, and an axis as wide as the screen gets no
  frame, as the exe's. With a colour (1–6, 263 uses) the pieces
  (normal, then `_inv` one frame in) are drawn in that colour (1 and 7
  black, 2 white, 3–6 tints): not drawn yet; the picture is cut at the
  frame's inner edge as before. `_FADS` type 1 (20,842 uses, always
  0 ms and rule 0) is a cut that switches the rule mask off
  (`FUN_0014b4c0(-1)`) and goes on at once (`FUN_00160610` returns
  −1), as converted; its mode 0 (417 uses) sets nothing.

- 2026-10-17: **the system menu's Save and Load tabs**
  (`typemoon/menu.rs`, the shared `SystemMenu`), from the exe:
  `FUN_0014c370` draws the bar (`menu_window` at (18.67, 1), the
  buttons Save, Load, Config, FlowChart, Title at (26.67, 43.56 + 31.11
  a row), hit box 74 × 29 at table 0x691438; `system_titlebg`,
  `system_scenebg` at (250.67, 0), `underline` at (0, 29.87)),
  `FUN_0014f240` the save list (`config_innerbg` at (120, 0); 4 rows at
  (133.67, 36.92 + 93.75 a row) of `save_win`'s (32, 32)-(992, 304), and
  (31, 336)-(991, 608) for the row chosen; the thumbnail at (151, row +
  23.23); the route panel at (453.33, 39.41)), all on the 640 × 448
  stage. 100 slots, 25 pages turned by the wheel and Left/Right; a
  click saves (Save only in game) or loads, asking first over a save;
  Title goes back to the title. The title's Load opens the Load tab.
  Ours: `menu_btn`'s cells cut 240 × 104 from (24, 26), centred on the
  hit box. Guesses: the game dimmed under the menu, the thumbnail 192 ×
  108, the date and the slot's last line in the font where the picture
  digits and the slot text are, the fade 200 ms. Left out: the header's
  chapter title and scene name, the cursor's line and light, the route
  picture by the save's phase (the unknown route's is drawn), each
  save's chapter label, the 39 autosaves. Config and FlowChart are shown
  at rest. Engine: CONscripter #117 (a tab reached from a held tab
  keeps the game as its thumbnail), #119 (`hover.under`).

- 2026-10-17: **the title** (`typemoon/title.rs`), from the exe:
  `FUN_0018d2e0` sets it up, `FUN_0018e200` runs it, `FUN_0018d5d0`
  draws it, the parts from `allui`'s `TITLE_PARTS` (part ids 0x138–0x143
  map to its textures by the tables at 0x75BFC8, one per movie). The
  backdrop is one of four looping movies (table 0x50B890), chosen in
  `FUN_0018d1c0` by the scene-read flags `_RDST` sets: `titlebg2` after
  560 (Arcueid's ending), `titlebg3` after 383 too, `titlebg4` after 562
  too (with music `_MPLY 72` in place of 2). The logo at (340, 36.27),
  the copyright at (436, 507.73) and an added light pulsing over 4 s at
  (360, 278.04) on a 960 × 544 stage; the menu on a 640 × 448 stage at
  x 272, rows 23.23 apart from y 298.67 with Continue (shown only with a
  save to resume), 312.36 without (table 0x452AF4). Sounds
  (`FUN_001acef0`, table 0x4341A0): `WSD_MOON_SE00001` on a move,
  `SE00002` on a choice, `SE00011` for New Game, which fades the music
  over 1.5 s and the screen to white. The fade in: 1,389 ms (0.012 a
  frame). Ours: the menu strip's cells cut 312 × 88 from (24, 22) so
  the widget's hit box (table 0x75CBD8) centres on each label; New Game
  and Load at rest, chosen and lit (the strip's three columns) as normal,
  hover and pressed. Left out: the boot logo and caution screens (in
  since 2026-10-18, below), the "PRESS ANY BUTTON" step, the logo's particles, its three bands and
  the menu's rows fading in one after another, and the selection line
  (`titlecursor`). Movies are no longer packed into `arc.nsa`
  (CONscripter plays them from loose files only). The registry and GUI
  list the game.
- 2026-10-17: numbers with a `.` (`_STCH(19,img1729,…,242.25,…)`, 24
  `_STCH` and with them 12 `_STZ4` and 4 `_STRT` on their layers),
  **seen**: the expression reader (`FUN_00103160`) takes the digits
  with `strtol` and stops at the `.`, so 242.25 is 242. `_VPLY`'s
  argument 1 (`_VPLY(B50_01_01_0007_0,03978)`, 16,690 uses) is never
  read (`script_command`, case "PLY": the name, then argument 2 when
  there are three). `_SELS` (1 use, `QACALL`) is a two-slot choice
  with no texts (`FUN_00192800`: slots 0 and 1, the select screen in
  mode 2, `FUN_00183b20`); its look is the select screen's, left for
  the screens.

- 2026-10-17: media flags as the other games' (the user's choice): no
  re-encode by default. Pictures PNG, `--webp`/`--webp-quality`
  WebP (was lossy q85 always); the `RULE_*` masks always PNG, exact.
  Movies remuxed at 1080p (were re-encoded at 720p, 182 MB; the
  title background `titlebg2` is 150 MB as is, 2.3 MB at 720p CRF 23).
  Wave sounds WAV with `smpl`, `--audio-codec opus` / `--audio-quality`
  re-encode with loop tags. Vorbis loops checked as Opus: CONscripter
  reads its `LOOPSTART`/`LOOPEND` comments (`soundLoop`); a test pulse
  decodes at shift 0; the disk-audio `dwaveloop` of
  `WSD_mix_se032.ogg` repeats exactly every 10,603 frames; one of the
  196 looping files (`WSD_moon_se02067`, a loop from 0) decodes 128
  frames short of its end, which the engine then loops whole.
- 2026-10-17: `_SCH2` (4,674 uses) and `_STCH` argument 18, **seen**:
  both are `FUN_00160840` (0x160840), `_SCH2` with its first parameter
  0, which keeps the layer's alpha (0 on a layer with no picture),
  priority and turn where `_STCH` sets 255, layer × 10 and 0, and
  skips the live layer's reset (`FUN_00131f00`) and the tint's.
  Argument 18 first copies another layer's settings (place 0xa4/0x144,
  alpha 0x1e4, priority 0x234, turn 0x284/0xbe4, zoom 0x324, `_STZ4`'s
  0x7d4/0x824/0x874, parent 0x89c, clip 0xa2c, tint 0x16da, motion
  0xc34), and every layer whose parent was that layer gets this one.
  The scripts change a character's expression this way (`_SCH2(19,
  ari_…,…,16)`, then `_STCP(16,0)` and a cross-fade). The 2,418
  `_STZ4` and 330 `_STRT` "on a layer not staged" were these layers:
  14 and 5 are left. The motion isn't copied (the converter's runs at
  its `_FADS`).

- 2026-10-17: `_STRT(layer, from, to, ms, ease, cx, cy)`
  (`FUN_00162b20`), **seen**: `FUN_001326e0` sets the live layer's turn
  to `to` with `from − to` left to run off over ms · 60 / 1000 frames
  (none at 0 frames), `FUN_001325d0` the easing; `cx`, `cy` go to the
  live layer's 0x72, 0x74. All 21,691 uses are untimed with from = to,
  so `from`, the time and the easing are read as meaning nothing
  (21,356 now fully converted; 0.txt unchanged). The 335 left are on
  layers the converter hasn't staged (see `_STZ4`).

- 2026-10-17: the chapter-jump menus and options under a flag,
  **seen** in `script_command` (0x177790, case "ELR"): `_SELR` adds a
  slot in the order it runs, keeping its index (for `_GOTS`'s label)
  and its text as written: a `$` line, else the name itself
  (`JUMP00`–`JUMPQA`'s `SEL_JUMPn` menus list script names; only
  `TEST_SCRIPT`, the debug menu, goes there; `ARCHIVE` uses their
  `ARCHIVEJUMP` part to go to a scene by `G008`). Argument 2 = 1 marks
  the slot greyed, and the cursor skips it (`FUN_00183e50`), so it
  can't be picked. Eight choices have options under an `_IF__` (or
  greyed under one flag, shown under another): each such option sets
  a bit of a local the choice reads, and the choice is one `select`
  per set of them; a greyed option is left out, as `select` has no
  disabled option. `_SELS` (1 use, `QACALL`'s yes/no) is not converted.
- 2026-10-17: greyed options (`_SELR(i, text, 1)`) are shown greyed
  and can't be picked, with CONscripter's `seldisable` (engine #116:
  the off colour at half alpha, no mouse area, the cursor keys pass
  over them). A greyed option sets a second bit (16 up) of the
  choice's local when run under an `_IF__`; an option offered anywhere
  is offered (offered wins over greyed). Four choices show one
  (day 3's Ciel route, 「……二十七祖ってなんだ？」, 「……眼鏡は外さない。」,
  「……アルクェイドを、嫌いにはなれない。」), each in the `select` for
  the flags that grey it.
- 2026-10-17: the 1,300 wave sounds as Ogg Opus (`se/<name>.ogg`,
  ffmpeg's libopus at 48 kbit/s a channel, VBR): 1,395 MB as WAV, 83 MB
  as Opus. Their loops go in as `LOOPSTART`/`LOOPEND` tags (start, and
  the FWAV's end, the sample count). Checked: each decoded file is as
  long as its wave (its `LOOPEND`), libopus's delay is undone (a test
  pulse decodes at shift 0), and under SDL's disk audio driver the
  engine's `dwaveloop` of `WSD_mix_se032.ogg` repeats exactly every
  10,603 frames at 44.1 kHz, as the WAV did. Opus plays through the
  engine's own decoder (`decodeOpus`, in every build, the web one too).

- 2026-10-17: sound loops, **seen** in `knife1.bfsar`
  (`bfsar::stream_loops`): a stream's details (INFO sound, kind 0x2201)
  hold at 0x18 a reference (kind 0x2210) to u32 stream type (byte 1:
  loops), u32 loop start, u32 loop end. 131 streams loop (all the
  `BGM`s and `SONG`s, 105 after an intro); voices don't. The positions
  count decoded samples after the Opus pre-skip: in 59 of the 80 tracks
  with enough after their end to compare, the decoded music repeats
  itself exactly from end − 1 (the shift with the least difference over
  0.1 s, searched ±400 samples; the rest end in a fade or noise), so
  the end is taken as end − 1. `typemoon sounds` writes them as Ogg
  Opus `LOOPSTART`/`LOOPEND` tags, which CONscripter loops music and
  `dwaveloop` by (`decodeOpus`); the 196 looping wave sounds get their
  FWAV loop as a WAV `smpl` chunk. Checked in the engine with SDL's disk
  audio driver: `dwaveloop` of `WSD_mix_se032` (loop 16352–27892 of
  27893) repeats exactly every 10,603 frames at 44.1 kHz (the loop's
  11,541 at 48 kHz). The songs' loops end 1–6 s before their files do.

- 2026-10-17: the English (fan patch) text layout, **seen** in the
  code: when `FUN_0012df20` (0x12df20, the system language) says
  English, the text drawer `FUN_001391a0` (0x1391a0) starts lines at x
  30 instead of 88, wraps them at 584 instead of 464, puts the first
  line at (short)(65 − 9.882353) = 55 (state +0x670), and
  `FUN_001a7790` (0x1a7790) draws glyphs × 0.9. The wrap table at
  0x44DEFC (30 floats, window types 3–32 from state +0x8a2, or +0x8aa
  in English when it is above 0) holds 299.8, 405.6, 509.8 for each
  pair of types, the same in both languages and in both exes; types
  0–2 take 464 or 584. The English build (detected by its text: most
  lines without kana) uses x 60, y 88, 41 columns (1168 px / 28) and
  27 px glyphs (1280 / 32 × 0.76 × 0.9); a headless page fills x 61
  to 1206 and wraps there. The fan patch's text carries its own `^`
  line breaks at about 55 letters, so few lines reach the wrap.
- 2026-10-17: `_STEC` effects 2–82 (9,193 uses), **seen**: the record
  (83 × 20 f32 at 0x4358B0, `stec.rs`) goes to the shader constants
  (`FUN_0019f6c0`, defaults from `FUN_001ec000`: gamma 1, low 0, high
  255) of `shader/nexl_nxshader_pre_alter.bnsh` (fragment program at
  file offset 0x1080, `envydis -m gm107`; its constant bank 1 is 1/255,
  −1/255, 255, 127.5 at 0x2300), which draws an altered texture: per
  texel (0–255) the greyscale (54 R + 183 G + 19 B)/256 in mode 1, then
  per channel `low + (high − low)·g/255` with g = 0 for a gamma ≤ 0, else
  255·(c/255)^(1/gamma); then each colour layer L (r, g, b) at amount a
  (0–1) by its blend: 2 normal `c + a(L − c)`, 3 add `min(c + aL, 255)`,
  5 multiply, 8 colour dodge `255c/(255 − aL)` (black for c ≤ 3), 14
  add then mixed by a, 17 screen, 18 overlay; others none. Effect 1
  (mode 2) fills the texel with the colour instead, and the layer is then
  drawn in that colour too (the normal shader multiplies by it), so a
  white `_STEC(n,1,0xFFFFFF)` is a white silhouette, not "none" as
  converted before. The shader's last step (brightness and contrast,
  `FUN_001a3180`'s first two arguments) is 1.0 for these. Converted
  with CONscripter's `spfilter` kinds 9/10 and blends 4/5 (#99).
- 2026-10-17: `_STCH` arguments 7 and 8, **seen** unused: `FUN_00160840`
  reads arguments 0 and 2–6 (`FUN_00102c38`/`FUN_00102d3c`), 1 as the
  name (`FUN_00102c6c`) and 9–19, never 7 or 8, and `script_command`
  passes the split arguments straight to it. Argument 8 is `CC` in
  54,502 of 54,503 calls (the other has it as argument 7). They no longer
  count as gaps.

- 2026-10-17: `_STZ4`'s other arguments, **seen** in `FUN_00162d20`,
  `FUN_00131f30` and the draws (`FUN_00132e30`/`FUN_00137f50` for the
  live layers, `FUN_00189ad0` for `FUN_001884f0`'s list). The time is
  ms · 60 / 1000 frames (integer); at 0 frames the zoom is the target
  and x0, y0 and the easing `e` are never used, so the 29,897 static
  calls now read every argument. Argument 7 (`f`; empty keeps the
  layer's, `_STCH` and `_STCC` clear it) is the layer's byte 0xA2
  (state 0x874): when set, the quad is drawn at zoom 1 and the zoom
  goes only to the shaders' constants (`FUN_001b3780`, draw flag 0x40;
  **inferred**: for the blurs, `_STBR` 11 or 17 in most of its 55
  uses). A child's place still uses its parent's zoom
  (`FUN_00138a70` doesn't read the byte). The converter draws such a
  layer without its `_STZ4` (or motion) zoom: 78 `spzoom` and 23
  `spmotion` of the Japanese `0.txt` changed. Left: the 2 timed uses
  (jump to their end) and 2,418 calls on layers not staged.

- 2026-10-17: `_CLO3`/`_CLO4`/`_CLO5` fade the text window while the
  script goes on, with CONscripter's new `windowfade` (engine #82):
  `_CLO3` → `windowfade 0,67` (4 frames), `_CLO4(ms)` → `windowfade
  0,ms/2` (drawn at 2 × (f − 0.5), so gone at half time), `_CLO5(ms)` →
  `windowfade 256,ms/2,0,ms/2:textclear:texton`, and the first text after
  `_CLO2`/`_CLO3`/`_CLO4` gets `windowfade 256,150,0,150` (18 frames,
  the same curve). Tracked in script order: a text reached by a jump
  from elsewhere may get a needless fade-in, or keep the plain show.
  Japanese `0.txt`: 13,186 `textoff` became `windowfade` (`_CLO2`'s 5
  stay), 12,624 fade-ins (596 of them `_CLO5`); metrics full +1,574.

- 2026-10-17: `_CLO2`/`_CLO3`/`_CLO4` hide the text window (`textoff`),
  all **seen** in `script_command` (0x177790) and `FUN_00138d40`
  (0x138d40, the window's alpha each frame; ticks from `FUN_001ad180`,
  1 per 60 Hz frame, 2 after a dropped one). `DAT_00be0550` is the
  shown flag, `DAT_00be0558` the window alpha, `DAT_00be05b0` a second
  fade multiplied into the window art (pictures 0x1d, 0x1e) and the
  text as max(0, 2 × (f − 0.5)) (`FUN_001391a0`). `_CLO2` calls
  `FUN_00153ab0(1)` (new page) and sets flag and alpha to 0. `_CLO3`,
  when shown, clears the flag and sets `DAT_00be0560`, so the alpha
  falls 0.2604 a tick (4 frames) instead of the normal 0.06 a tick.
  `_CLO4(ms)` sets the second fade to 1 and its step to −16.67/`ms` a
  tick (the earlier note that it sets the flag was wrong); `_CLO5(ms)`
  sets flag and alpha to 1 and fades the second one up from 0. The
  next text (`FUN_0016e0d0`) restarts that fade from 0 at 0.0556 a tick
  when it is below 1 or after `_CLO2`/`_CLO3`. 13,191 `textoff` in the
  Japanese `0.txt`.

- 2026-10-17: `_PGST(n)` left out, **seen**: `script_command` stores
  `n` in the state (`DAT_00be0290` + 0x8c4) unless a seek is running;
  the save writes it (`FUN_0015a910`) and the load reads it back
  (`FUN_0015baf0`), the backup copies at `DAT_00be0338` (`FUN_001178c8`,
  `FUN_00117af0`) keep it, and its only reader is the load:
  `FUN_00182330` calls `FUN_00182c60(page)`, which finds the scene's
  `_PGST(page)` in the script (for a few scenes, chosen by
  `DAT_00450d58` and a flag, another page), sets the seek flag
  `DAT_00be0900` and the sought page `DAT_00be09c0`, and runs commands
  until `_PGST` reaches that page. CONscripter's saves restore the state
  themselves. (The +0x8c4 in `FUN_0015baf0`'s copy into `DAT_00be0258`
  and in `FUN_00160840` is another table, `DAT_00be0250`.) 17,202 calls
  move from gaps to `left_out`.

- 2026-10-16: children follow their parent's motions. `_STMA` on a
  parent gives each layer placed in it (and theirs) an `spmotion` of
  its own, from where the parent's keys place it; the motions start
  after every place of the `_FADS`. A child's alpha is its own times its
  parent's (`FUN_001884f0`: the parent's at +0x1E4, `_STCH` argument 5).
  Not followed: `_STGS` fades on a parent, and a child's own motion
  beside its parent's.

- 2026-10-16: a parent's turn for its children. `FUN_001892b0` and
  `FUN_00189420` (the child's x, y) turn its offset in the parent by
  the parent's angle (`FUN_0018c480`: the angles summed up the chain,
  in 1/100°) with x' = u cos a − v sin a, y' = u sin a + v cos a, y down:
  clockwise on the screen. So a positive angle turns a picture
  clockwise (it was drawn counter-clockwise before): `spzoom`'s angle
  is now −a/100. 20,757 `spzoom` and 1,725 places of the Japanese
  `0.txt` changed.
  - **The sign, checked again** (the same day). Two places in the game's
    own code put things clockwise for a positive angle: the child math
    above and `FUN_00189ad0`'s frame corners (it turns the corner
    offsets with x' = dx cos − dy sin, y' = dy cos + dx sin, y down,
    and draws the corner pieces with the same angle). 1,725 child
    places sit in turned parents, which the original couldn't have
    shipped if the picture turned the other way. The renderer itself
    (`FUN_001b1890` → `FUN_001bacf0` → `FUN_001b8990`: Euler z = −angle,
    row-vector model matrix, y-down ortho in `FUN_001d83bc`) reads as
    counter-clockwise, but that read leans on the view matrix and the
    shader's layout, neither traced; so clockwise is inferred from the
    game's code, not seen at the vertex.
  - **A child's own picture turns by its own angle only**:
    `FUN_001884f0` copies the layer's angle alone into the draw record
    that `FUN_00189ad0` reads (record +0x1c, ×1.82 into 1/65536 turns
    for `FUN_001b0480`), and `FUN_0018c480` (the sum) is called only by
    the two position functions. The scripts agree: of 1,831 children
    in turned parents, 14 set the parent's angle on the child too
    (1740 and 1740), which summing would turn twice. 1,405 `spzoom`
    lines of the Japanese `0.txt` changed.
- 2026-10-16: panels (`_STCH` arguments 12–17). The draw
  (`FUN_00189ad0`) reads arguments 12–15 as a rectangle x, y, w, h on a
  1920 × 1080 screen: w or h 0, or 1920 × 1080, is no clip; otherwise
  it is scaled to the render target (× width / 1920, × height / 1080)
  and set as a scissor. Argument 16 is a frame's width (1920 units):
  the scissor is the rectangle less the frame on each side, across if
  narrower than the screen, down if lower. With argument 17 ≥ 1 the
  frame is drawn in a colour (1 and 7 black, 2 white, 3, 4, 5, 6 tints;
  261 uses), with corner pieces. A child draws in its parent's
  rectangle (copied at `_STCH` and again at the draw), and an argument
  left out keeps the layer's value. The uses fit: 2,210 rectangles,
  mostly wide bands past both sides (−428, 158, 2775, 765) or tall ones
  past the top and bottom (698, −136, 957, 1353), cut-in panels.
  Converted with `spclip` (9,065 clips, 3,422 back to the full screen).
  Not drawn: the frames, and a turned layer's rectangle isn't turned.
  Checked headless: `01_00_ARC01_8`'s bedroom strip shows from y 166 to
  554 on the 720 screen.

- 2026-10-16: choices. `_SELR(index, $text[, 1])` adds an option (1:
  greyed), `_SELD(_D000, …)` shows them and `_GOTS(_D000, label, …)`
  goes to the picked option's label; an option is written once per
  branch of the `_IF__`s around it (shown or greyed), and the converter
  keeps the first text of each index, so a flag-gated option is always
  offered. `_GOTO(label)` jumps in the script. `_IFNX(,L)` and
  `_IFP4(,L)` (also `_IFSC`) hold the Switch's and the PS4's own lines
  (a picture placed a few pixels apart): the Switch's run, the others
  are skipped. Headless (300 s, auto-play): 29 choices, from `00_00`
  through the first four days of Arcueid's route into a bad end's
  Q&A (`QACALL`).

- 2026-10-16: turns, shaders, tints.
  - `_STRT(layer, from, to, ms, ease)` (`FUN_00162b20`) turns the
    staged layer to `to` in 1/100° (clockwise on the screen, as the
    child-layer math turns; see the parent-turn entry above); `_STCH` argument 19 the same, and
    `_STCH` resets it. No use is timed.
  - `_STBR(layer, n)` (`FUN_00162490`) picks the layer's pixel shader;
    `_STCH` resets it (to a global default, else none). The draw maps
    the byte through a table at 0x44DD70 (`FUN_00132d20`; 1–39 map to
    themselves) into the shader list at 0xBD2308 (56 names, 16 bytes
    each, relocated): 0 default, 3 fulladd, 7 monotone, 8 negative,
    11 gauss, 15 haze, 18 add, 19 multi, 20 dodge, 21 burn, 22 darken,
    23 lighten, 24 screen, 25 add_alpha, 26 sub, 27 ps_add, 28 ps_sub,
    29 ps_mul, 30 ps_screen, 31 overlay, 32 hardlight, 33 softlight, …
    Uses: 17 (3,475), 11 (3,158), 23 (1,909), 24, 22, 7, 10, 19. Drawn
    with `spblend` (add, multiply, subtract, screen; darken and burn as
    multiply, lighten and dodge as screen) or a filter (greyscale,
    negative, blur); forcealpha, haze, wave, overlay and the lights as
    normal.
  - `_STEC(layer, effect, colour)` (`FUN_00103de0`): effect 1 tints the
    layer by `colour` (0x000000 954 uses, 0xFFFFFF 410, 0xE5FFFF, …;
    multiplied, white none), 0 ends it; `_STCH` resets it. Effects 2–9
    and 10–82 (2 is the most used, 1,338): see Open questions.
  1,001,727 commands fully converted.

- 2026-10-16: parent layers. `_STCH` argument 11 names the layer a
  picture is placed in (6,028 uses, mostly effect pictures over a
  scene). From `FUN_001892b0` and `FUN_00189190`: the child's x, y are
  in its parent's picture, from the parent record's top-left
  (child = parent's place + parent's zoom × (x · shot − the parent
  record's centre), the shot ½, ¼, ⅛ for a parent sprite's `_L_`,
  `_M_`, `_S_`); its zoom is the parent's times its own and the shot.
  Resolved at each `_FADS`, so a moved parent places its children
  again. Not followed: the parent's rotation and motions.

- 2026-10-16: motions. `_STMA(layer, m)` (`FUN_00199620`) starts motion
  `m` on the shown layer at once (and its staged copy), while the
  script goes on; on a layer not shown yet it waits for the `_FADS`
  that shows it. Its keys are absolute (x, y in script units, alpha,
  the `_STZ4` zoom); the converter turns them into `spmotion` offsets
  and zooms from the layer's place (the IR's `SpriteMotion2`, kept at
  the end) and moves the layer's state to the last key. The motion
  table was read one motion late (its entries are each motion's end,
  not its start; see "The exe"). `typemoon exe --motion m,…` prints
  motions' keys. 961,377 commands fully converted.

- 2026-10-16: zoom and alpha fades. `_STZ4(layer, x0, y0, x1, y1, ms,
  …)` (`FUN_00162d20`) sets the staged layer's zoom across and down in
  ‰ (to x1, y1; `_STCH` resets it to 1000), times `_STCH`'s zoom
  (32,317 of 32,319 uses are static; the 2 timed ones jump to their
  end). `_STGS(layer, from, to, ms, ease)` (`FUN_001637a0`) fades the
  layer's alpha from `from` (default its own) to `to` (default 0) over
  `ms`, running from the next `_FADS` while the script goes on (4,877
  of 8,114 timed; easing not followed): an `spmotion` from the IR's
  `SpriteMotion2`, started inside the transition so its first key
  shows. 937,479 commands fully converted.

- 2026-10-16: layer priority. The draw list (`FUN_001884f0`) keeps the
  layers with a picture and a priority above 0 (`+0x234` per layer,
  set by `_STCP` and `_STCH` argument 6, default layer × 10) and
  stable-sorts them by priority, lower first (drawn first, so under).
  Each `_FADS` gives them IR layers in that order from 39 up and draws
  again only the IR layers whose picture changed. 897,047 of 1,082,419
  commands fully converted (was 845,480).

- 2026-10-16: the script conversion's first part (`typemoon/convert.rs`,
  `build.rs`; `typemoon convert`). Every script is lowered, 845,480 of
  1,082,419 commands fully. A picture is drawn at its record's size
  from its file's (the full art's header, read without decoding:
  `typemoon::picture_size`), its record's centre at the script's x, y
  (480 × 272 for the screen); the layers' order follows their number.
  `_FADS` 0 is a cross-fade, 2 a wipe through `pic/rule_NN.webp`. Text:
  each `_ZM`/`_MSAD` is a line of the page, `_WNTY(1)` before one
  ends the page, `@n` joins the next line without a click; `^` is a
  line break, `@x` codes and `[ber00]` are dropped, ♡/♥ are kept
  (UTF-8 `0.txt`). Flags: `F`/`C` locals from %196, `G` globals %800 + n.
  Found on the way: the Ogg Opus files' OpusTags said a vendor length
  of 13 for "conscripter" (11); ffmpeg read them, opusfile (the
  engine's voices and sound effects) refused every one; and an opaque
  WebP has no alpha channel, so `lsp ":a;…"` took its right half as
  the mask on native builds (fixed in CONscripter, engine PR #55; the
  corner-pixel workaround in `typemoon media` is gone). Checked headless (trace, 200 s): from `START_SCRIPT` through
  `00_00` into `01_00_ARC01_*`, every voice and sound loads.

- 2026-10-16: the media folder (`typemoon media`, see "Build"). The
  user's picks: WebP at full size, not the BC7 textures (a sample of
  101 `allpac` pictures: 63.1 MB in the archive, WebP q85 at full size
  7.2 MB, at stage size 2.6 MB), smaller movies, the Opus as it is.
  Japanese copy: 14,763 pictures (one per name: 1,279 bare
  backgrounds from their 2560 × 1440 `.JPG`, the rest from the larger
  `.NXGZ`/`.NXZ`), 2.37 GB from 16 GB; movies 1.7 GB → 182 MB; 15 min
  on 8 threads, 5.7 GB peak (32 threads went over 10 GB). Checked: RMSE
  against the lossless decode 0.3 % for `ARU_A_18_00_N01_10_10`
  (886 × 4160), 1.6 % for `IMG0020` against its JPG; CONscripter
  (native) draws both, the sprite's alpha whole.

- 2026-10-16: how big a picture is drawn, settled by matching the
  gallery's CGs to the story pictures (`cg_framing.py`): a record
  is on a 1280 × 724 stage, the screen; a background's whole texture
  fills it. The shaders were read on the way (they don't hold it).

- 2026-10-16: the stage: script coordinates are 480 × 272 for the
  screen (the draw's vertex math), record units 640 × 362; a bare
  pose's record is its expressions' less their padding. How big a
  picture is drawn is left open (see "The stage"). The user picked
  full-size WebP for the media folder.

- 2026-10-16: the exe's tables read in Rust (`typemoon/nso.rs`,
  `exe.rs`, `typemoon exe`): 14,713 pictures, 24,192 motions (71,903
  keys), 551 scene numbers. Picture sizes checked against every
  texture; the record's byte meanings corrected (0x43 the expression
  flag, 0x46 the shot). A WebP sample for the build's media folder.

- 2026-10-16: the walkthrough, from the scripts (`routes.py`) and
  checked against a public one: the route gates (Ciel's points, the
  Arcueid points, `G003`/`G004`), the three endings, the 21 dead ends
  and their lessons. `_JUMP` goes by scene number, a table in the exe.
  See "Walkthrough" and "Progression variables".

- 2026-10-16: the exe's commands. `_MPLY(n)` plays the bfsar's sound
  `n − 1` (a table in the exe); the music loops are in the bfsar's
  stream details; the picture and motion tables are in the exe; the
  arguments of `_STCH`, `_STCC`, `_STCP`, `_STZ4`, `_STMA`, `_FADS`,
  `_WNTY` and `_WTTM` read from their handlers; sprites turned out to
  be whole pictures, one per expression, not atlases. See "The exe".

- 2026-10-16: the exe decompiles (`nso2elf.py`,
  `CodePointerFunctions.java`, `research/typemoon/names.tsv`); the
  script dispatcher and `_MPLY` found. The official update 1.0.2 was
  unpacked and compared with the base and the fan patch. The script
  changes are split between them in "Script language", and the copies
  to convert are chosen in "Build".

- 2026-10-16: scripts read (`typemoon/script.rs`, `typemoon script`).
  Both languages: 549 scripts, 93 (96) command names ranked, every
  text reference within the 43,961 lines, every voice found, the
  sound effect names matched to the archive (all but `MIX_SE*`). See
  "Script language".

- 2026-10-16: sound (`typemoon/nxopus.rs`, `bfsar.rs`, `fwav.rs`;
  `conscripter-convert typemoon sounds <romfs> -o <dir>`, 9 s). Both
  RomFS copies: 16,601 streams to Ogg Opus (identical in both), 1,297
  (Japanese) and 1,300 (English) wave sounds to WAV. Checked: ffmpeg
  decodes every Ogg with no error; for 910 of them (every 150th voice
  and every music file) the decoded length equals the granule count
  less the pre-skip. DSP ADPCM: the 107 (108) waves that loop from
  inside store the decoder state at the loop start, and our samples
  before it match it on every channel (a change in the rounding makes
  106 of them fail).

- 2026-10-16: pictures (`typemoon/bntx.rs`, `typemoon extract --png`):
  BNTX parse, Tegra block-linear deswizzle, BC1/BC2/BC3/BC4/BC7 blocks
  through the `bcdec_rs` crate. `typemoon check` decodes every texture;
  each one's swizzled size matches the `BRTI` header's. Base (Japanese):
  29,561 textures (29,451 BC7, 98 BC4, 12 RGBA8); patched (English):
  29,571 (29,463 BC7, 98 BC4, 10 RGBA8). Looked at a background, a
  sprite atlas, UI parts, the RGBA8 parts and transition masks: right
  way up, colours right.

- 2026-10-16: the archive reader (`crates/conv-engines/src/typemoon/`:
  `mrg`, `mzp`, `mzx`, `nxx`) and `conscripter-convert typemoon
  list|extract|check`. Every entry of both RomFS copies reads and
  unpacks, and each wrapped entry's size matches the `.hed`. Inside:
  29,475 `allpac` entries (28,185 BNTX, 1,279 JPEG, 5 PNG, 6 MZP),
  `allpaccg` 919 BNTX + 600 JPEG, `allpachd` 14,705 unknown (`bin`),
  `allui` 26 BNTX. MZP sizes: mangetsu's and deepLuna's formula is
  64 KiB too big when rounding to the sector crosses a 64 KiB boundary;
  ours isn't.
- 2026-10-16: tool survey (mangetsu, deepLuna, PS-HuneX_Tools);
  the nesting of MZP, MZX and NXGX→BNTX checked on our files.
- 2026-10-16: NSP unpacked, engine identified, this document started.

### Status as it stood before the 2026-10-18 prune

| Area | State |
|---|---|
| Getting the files out of the NSP | ✅ |
| Engine identified | ✅ (TYPE-MOON, `mrgd00`/MZP) |
| Survey of existing tools | ✅ (see "Existing tools") |
| Archives (`hed`/`mrg`/`nam`, MZP, MZX, NXGX/NXCX) | ✅ every entry of both RomFS copies |
| Pictures (BNTX) | ✅ every texture of both RomFS copies decodes |
| Sound (`stream/*.opus`, `knife1.bfsar`) | ✅ every sound of both RomFS copies converts, with its loop (music and wave sounds) |
| Exe (`main`) in Ghidra | ✅ decompiles; dispatcher, the top commands' handlers, the music, picture and motion tables named |
| Exe tables (pictures, motions, scene numbers) | ✅ read from the NSO (`typemoon exe`); picture sizes checked against the textures |
| Scripts | 🔶 read: every command of both languages splits (one malformed in the original) and its text and voices join; the top commands' arguments read from the exe, the rest not yet |
| Media folder (`typemoon media`) | ✅ `allpac`'s pictures at full size (PNG, or WebP with the shared `--webp` flags), the movies as they are, the sound (WAV wave sounds, or Opus/Vorbis with the shared `--audio-*` flags), loops kept in every mode |
| Script conversion (`typemoon convert`) | 🔶 the flow (labels, `_JUMP`, `_IF__`, flags), text (pages, ruby, voices; the fan patch's styled letters, private-use U+E000–U+E2FE drawn by its own `ja/TT_TsukuMin-D.ttf`, as ASCII: italic and serif italic as italic (`!i1`), mirrored and upside-down plain (mirrored lines, stored right to left, put back in reading order), U+E200 as `^-35`; its accents (é ï ç â à ê, 46 uses) and em dashes kept, as the `0.txt` is UTF-8; a `^` break in a choice's option as a space), pictures (`_STCH`/`_STCC`/`_FADS`: record size and place, fades and rule wipes), sound effects and music; plays from the start through `00_00` into the first arc headless. Layers drawn in priority order (`_STCP`, `_STCH` argument 6), changed keeping their settings (`_SCH2`) or copying another layer's (argument 18), zoomed (`_STZ4`), faded (`_STGS`) and moved (`_STMA`, the exe's motions, a −32766 field eased once across its key as the exe eases it (the keys between cut into straight steps of its values where one curve a key would miss it), turned by its turn keys, held where a key's mode 2 says, only the fields its mask drives, each key eased as the exe's (`spcurve` tables of its Penner set; between keys the zoom and turn drawn in 1/100 steps each frame, CONscripter's `spmotion`, the place in whole pixels), looping where the exe loops until `_STMA(n, 0)` (where it is), `_STCH` or another motion stops it, going on with its clock when its layer moves to another IR layer; curves, priority and expression keys not followed), children placed in their parent layers, orbiting with their turn and following their motions (`_STCH` argument 11), clipped to their panels with their soft edges (`_STCH` arguments 12–17, CONscripter's `spclip` feather), turned (`_STRT`), drawn with their shader as far as CONscripter's blends and filters go (`_STBR`) and drawn with their `_STEC` effect (levels, greyscale and colour layers; effect 1 a colour fill, CONscripter's `spfilter` 9 and 10). Choices (`_SELR`, `_SELD`, `_GOTS`; options under a flag offered by it, greyed ones shown greyed and not selectable (`seldisable`); the chapter-jump menus), `_GOTO` and the Switch's own branches (`_IFNX`/`_IFP4`). The text window hidden and faded (`_CLO2`, `_CLO3`/`_CLO4`/`_CLO5` with CONscripter's `windowfade`, the script going on; the next text fades it back in). Not yet: the other layer commands, screens |
| Screens, extras | 🔶 the boot logo movie and caution screen (History, 2026-10-18); the title; the system menu's Save and Load tabs (see History, 2026-10-17), opened in game by a right click at a text wait (on its Save tab); Config (for now the language switch), FlowChart, the backlog not yet |
| The original under an emulator | ✅ boots to the title under Ryubing headless on Xvfb, 60 fps; ✅ the trace hook, diffed against ours over `00_00`'s first 297 pages (see "Testing" and "Trace diff" below) |
| Languages (`--english`) | 🔶 one `0.txt` with both copies (below); the switch is the system menu's **Config** tab for now (in game; at rest on the title), until the settings tab with its 日本語/English option is converted |

##### Languages

With `--english <game-en>` the build holds both copies of the
scripts (`language.rs`): the Japanese as before (labels `tm_<script>…`)
and the fan patch's (`pic/en/` for its own 155 pictures, the English
text layout) under CONscripter's label variants (`COMMANDS.md`, "Label
variants"; 2026-10-19): its labels are `tm_<script>…@en`, and a jump
names the plain label, which the engine takes to the `@en` one while
the language is English, so the copies' jumps and choices are the same
names in both. Only the labels a copy jumps to while the language is
still the other's keep their own names (`tm_en_…__lang_in`,
`tm_en_lang`, `tm_en_lang_new`). A save in the English copy from a build
before this has its label under the old name `tm_en_…`: it loads at the
line it kept, not by the label. The flags keep the Japanese
numbering (none is the English copy's alone). The language is the
copy the script runs in, so a save keeps it; the engine's `language`
is set at a new game's start, on each switch and after each load
(`loadgosub`). A new game starts in the language last chosen (global
%1800).

The switch goes into the other copy at once:

- At the same page when the script's ops up to it are the same in both
  copies but for the text (screen, sound and flags then are too): 29,197
  of 42,532 Japanese pages; 330 of the 515 scripts with text line up to
  the end. The
  rest differ from some point on (the fan patch splits and joins pages
  with `_MSAD`, counts them in `F823`, and has its own waits).
- Else where the scene was entered (a script's start or a label another
  script's `_JUMP` goes to, 1,088 of them), put back as it was there:
  each entry saves the game into slot 897, and the switch loads it,
  then goes on in the other copy. After a load from the save screen
  that slot is of another visit, so the entry is played again from a
  cleared screen (sprites, music, every sound channel) with the flags
  as they are (a flag the scene added to before the switch is added to
  twice).

Checked headless (trace, `--goto conv_title_start`, the switch
reached by a patched `0.txt` that `return *tm_switch`s from a `gosub`
at a page's wait as the menu does from the right click's `rgosub`):
Japanese page 5 of `00_00` → English page 5 (same sprites and sound);
the same with the page forced unaligned → slot 897 loaded, English from
`00_00`'s `START`; %1800 = 1 → a new game in English; an English save
loaded → `language "en"`, English pages; there a switch at an unaligned
page → the reset path, Japanese from the entry, the old loops stopped.
Again with the label variants (2026-10-19, same patches): page 5 →
`tm_lang_p1_5@en`; page forced unaligned → slot 897, English from
`00_00`'s `START`; %1800 = 1 → English from page 1; an English save
loaded from the Japanese copy → English from its page.
The Japanese-only build (no `--english`) is unchanged but for the right
click.


##### Trace diff

`START_SCRIPT` and `00_00`'s first 297 pages (2026-10-17, "Testing",
step 2): every page pairs with ours at the same command, in order.
`_FADS` takes the exe's 3 frames besides its fade, even when nothing
changed (`script_command`: `FUN_0012e0b0` asks for the pictures, a
frame's countdown, the fade starts once `FUN_001b6920` has them all, it
ends a frame after); a `_STMA` on a layer the next `_FADS` shows is
marked as the `_STMA`'s in the source map; `trace_diff.py` matches a
continued page by its own lines. `_STGS` on a layer already shown starts its fade (or sets its alpha) at
once, as the exe's `FUN_00132820` sets the live layer's, and `_STCF`
waits for it (`FUN_00162f90`, `FUN_001802c0`, `FUN_001329f0`: mask 8
the fade, 0x20 a motion that doesn't loop; ours an `spmotionwait`).
Left (was 46 `missing`, 149 `time`, 180 `text`; then 14 and 89):

- With `--merge-into _FADS`, `trace_diff.py` credits a `missing`
  command to the next one of its page that has ours when that is a
  `_FADS`, as `merged` (counted, not listed): 29, all `_STGS` on a
  layer the next `_FADS` shows (its fade or alpha is among the
  `_FADS`'s ops; marking them the `_STGS`'s would put the `_FADS`'s
  frames before them and add about 40 `time` findings). Left, 16
  `missing`: 11 `_FADS(1,0,0)` that change nothing visible in ours,
  so the trace has no event from them; `_STMA` `00_00:212` and `1121`
  (never drawn in the exe either: `_STCP` sets their layer's priority
  to 0); `_STCF` `00_00:55` and `_FADS` `00_00:57` (page 1) and
  `_FADS` `00_00:2278`. Totals: merged 29, missing 16, time 73.
- 73 `time`: chiefly `_FADS` after a new picture (12 after `_SEPL`,
  13 after `_CLO3`, 6 after `_SEFD`: the original's 2–10 frames
  loading it, left out), and `_MPLY(52)` at `00_00:4296`.

#### Next

1. The script conversion, next part: the layer commands the opening
   leans on, in order of use. `typemoon convert --dry-run` prints the
   counts. Splitting sprites into body and face crops is out unless
   the user asks.
2. The CG gallery (Gallery on the title), from the exe's tables (History,
   2026-10-18, "the gallery's tables"): its CGs are in the media folder
   (`cg/`). Before the screen: the seen flags. The gallery needs about
   1,100 of them (1,007 picture flags under 0xA8C, about 100 the
   sprites map to, read flag 557); the converter's
   `SlotCg::seen` is a whole variable, and the free globals are fewer.
   A bit flag (a shared `Var` bit, or a CONscripter seen-picture set the
   showing of a picture fills, as `kidoku`) is a shared change: its own
   PR first. Then `PictureExtras` with the CG mode (the music, movies and
   ending list later), slots from the 458-slot table and the thumbnails
   (likely `allui`'s `CG_THUMB_PARTS_JA`, 22 MB, not looked at;
   `allpaccg`'s `THUMCG_TOP.MZP` is a 2 KB stub). `allui` (71 MB) is
   still not in the media folder.
3. The menus: the settings tab (the defaults in "Accuracy", the
   日本語/English switch: move `SystemPage::Jump(language::SWITCH)`
   from the Config tab to its option), the backlog; then the save list's route picture and chapter labels
   (by the phase), and Config and Gallery on the title.
4. Still open in the exe: the motion keyframes' other fields.
5. The metrics (`tests/snapshots/tsukihime.metrics.json`) count each
   argument a converted command passes and doesn't read as a gap
   (`<command> argument <i> ignored`, `option <n>` for a backquote
   option); the list is in that file. The biggest: `_STZ4`'s 2 timed uses (and 2 on
   layers with no picture), `_WTTM` 1 (with it, the wait less
   `DAT_00be0a00`, a frame count `FUN_00181b30` adds to; when it is
   reset not found), `_SEPL` option 17.
6. Every kind has a row in `tsukihime.findings.tsv` (2026-10-17, from
   the decompile; 47 rows open). The open ones that matter most, by
   use: `_STCF` (5,179: the script waits for a layer's fade or
   motion; ours runs on), `_SEVL`/`_MVOL` (sound and music volume
   fades), the layer quakes `_SQK2`/`_SHK2`/`_SSW2` and the screen
   shake `_SHOK`/`_SHAF`, `_STPV` (the layer's pivot), `_SNX2`/`_SNX3`
   and `_STNS`/`_STRA`/`_STRR`/`_SACL`/`_SBAS`/`_SOXY` (layer fields
   whose drawing is not found yet), `_VCWT` (wait for the voice),
   `_VPL2`, `_SESP`, `_WKOR`, `_STTR`, `_STCL`, `_SGMA`, `_SPMA`, the
   chapter start (`_SELA`, `_TOMP`/`_TOMW`, `_ATSV`/`_WTSV`), the
   read flags (`_RTST`/`_RDED`, which `_IFUR` in `QA_25` needs), the
   gallery flag `_STCG`, `_SELD`'s choice number and `_SELS`'s
   yes/no choice. `_FANX`, `_STQK` and `_TRPY` have no case in the
   exe: the next converter PR adds them to `LEFT_OUT`.

#### Accuracy

The converter's hand-typed values carry a provenance comment (`src:`,
`guess:`, `ours`). Open guesses, each settled by one frame of the
original (Japanese, default text size):

- The text window's line pitch (45 px: 28 of the drawer's 448, the step
  it uses for its extra lines; the exe measures a line of the font) and
  so its 14 rows, and whether the glyph sits 16 px below its line's
  y (`FUN_001a7790` adds 14 + 10 of 1080 inside the line). Place,
  width and glyph size are the exe's (`FUN_001391a0`: from 88, 65 on
  a 640 × 448 stage, 464 wide; glyphs of the screen width / 32 × 0.76).
- The text speed: 35 ms a character, the exe's default for unread
  text (the settings' defaults at 0x56b080, the same as the reset in
  `FUN_0012c810`: +0xe unread 35 ms, +0x10 read 15 ms; `FUN_0013b2d0`
  shows 16.67 / ms characters a frame, all at once at 0). Read text is
  typed at the same 35 ms here (CONscripter has one speed); the other
  defaults (volumes +4/+8/+0xa = 40, 40, 10; auto wait +0x1c = 2200)
  wait for a settings screen.
- `_STBR` 11 (gauss) drawn as a blur of 8.
- Motions (`_STMA`): the keys' zoom is whole percents of the placed
  zoom (motion 2's 984‰ → 942‰ is 100 → 96 %, the exe's 95.7 %), their
  angle whole degrees and their offset whole pixels of 1280 × 720; between
  keys the zoom and angle move in 1/100 steps (before engine PR #147 they
  moved in whole steps: a slow zoom jumped 26 px at a time, the user's
  "choppy" report), the place still a pixel at a time (a 5 s pan of
  16 px steps every 300 ms; the exe draws it at sub-pixel places).
- Zooms are whole percents (`spzoom`): one under 10 % is coarse
  (`spzoom 95,5,9` may be 4.5–5.5 %), and one under 1 % is placed at
  1 %, not 0, with a motion's keys scaled from what was placed (a line
  drawn at 1‰ across that a motion opens to 1933‰, `05_00_ARC05INTERLUDE`).
  Exact would need a finer `spzoom` in the engine.

#### Open questions

The biggest commands not converted, looked up in `script_command`
(0x177790):

- `_SNX4(…)` (8,762), **seen**: `script_command` has no case for it
  (`_SNX2` and `_SNX3` have), so the Switch exe does nothing. Left
  out.

The mapped commands, argument by argument and branch by branch, are in
`docs/games/tsukihime.findings.tsv` (seen, inferred or unknown, and the
cheapest check for each; redone 2026-10-17 against docs/new-game.md's
steps 6–17: 67 seen, 16 inferred, 6 unknown). Open there, by size:

- Motion keys' priority (0x3C, 23,115 calls) and expression (0x32/0x34,
  5,254), and the stepped fields 0x24–0x2A (shader, effect and colour
  changes mid-motion?).
- `_SEPL`/`_MPLY` volume (option 17) and `_SEVL` (not converted, 1,285):
  every sound at full volume.
- Zooms under 10 % are coarse (`spzoom` takes whole percents).
