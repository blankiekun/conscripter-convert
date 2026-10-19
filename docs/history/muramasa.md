# 装甲悪鬼村正 (Nitroplus NSS): History

Done items moved out of [the game document](../games/muramasa.md)'s Status.

### New pictures zoomed from a size or on a path (2026-10-19)

A picture shown with an unwaited `Zoom` from a size other than 0
(ma04_018vs:365, from 1200) or a `BezierMove`/`FadeFR` path (mc04_008:138's
`Plain` copy) was held at the next sync and drawn as a cross-fade. sync.rs's
new-picture loop now gives both as an `spmotion` (the zoom faded in from 0
when the fade takes as long; the path from minus its last key, keys via
`along`, so FadeFR's `Random` jitter adds 29 runtime `rnd` forks). Holds
1,525 -> 1,243. Tests `a_new_picture_zoomed_without_waiting_zooms_from_its_size`,
`a_new_picture_on_a_path_without_waiting_runs_it`.

### No hold under a cover (2026-10-19)

A sync under an opaque `Plain` cover (nothing above it) waited for the
unwaited actions still running under it, up to 1 s (127 of the 1,652
holds): the cockpit gauges' processes (`CP_*Auto`/`*Move`, `CPW*`,
about 100, mc03_020vs*, md05_007vs, ma03_025), the clouds of
function.nss:4596, backgrounds panned or zoomed under the cover
(mc01_019vsa:80's 80 s pan, mc02_027:60). Nothing under the cover is
seen and the exe goes on, so the sync now waits only the script's own
time. Test `an_unwaited_action_under_an_opaque_cover_doesnt_hold`.
Left: such an action is still drawn at its end when the cover goes
(the 80 s pan doesn't run on), for the background-motions item.

### `csp 49`-`58` once per load, not per `setwindow` (2026-10-18)

Every `setwindow` was followed by `csp 49` to `csp 58`, which only
cleared the name plate's and link words' old sprites in saves made
before #222 and the link names: about 384k commands, 2.7 MB (9 %) of the
`0.txt` (JP 28.8 → 26.1 MB, EN 27.1 → 24.5 MB). A load is the only way
those sprites come back, so the `loadgosub` routine `conv_load_clear`
(every game) clears them once and goes on to the game's own routine.
Sprites 49-58 are used by nothing else in any game.

### ma00_000 picture diff: what is left (2026-10-18)

#457's 24 blocks, rerun with `--real-time`:

- Pages 57, 66 (66 is 57 kept in a cover): `SetShade(…, MEDIUM)`
  pictures are about 2.5/255 darker in the exe. Its passes of a 3×3
  mean floor (`FUN_00465b10`: `sum / 9` on unsigned ints), about
  0.4/255 lost a pass (4 passes on a frame of the game, simulated:
  1.5/255); the engine's blur rounds. Invisible; a fix would darken
  a shaded picture by its passes' loss.
- Pages 55, 56, 62, 64, 76-79: inside the text window, which ours
  moves to the same place (`SetTextEXR`/`EXL`, x 191 or 703): the
  original's frame 1 s into the block is still typing (62: 3 of 4
  columns), ours is typed out with the click-wait icon. 79: our
  frame is 200 ms into the unwaited 300 ms `Move("@StC*", …, @+10,
  @+5)` (the knight 4, 2 px short; the original's is done).

### `DrawTransition` as the engine's `spfadeto` (2026-10-18)

`DrawTransition` on a picture (image or strip) is now CONscripter's
`spfadeto` (engine PR "spfadeto"): the sprite fades through its mask from
`from` to `to` while the script goes on, the table made frame by frame as
the exe makes it (transition_start 0044fe00, 00457bf0), the mask stretched
over the screen. `from` is taken as given, whatever the picture showed
(the exe's start doesn't read it): 620 of the 633 calls noted "from
another opacity than the picture's" were pictures; the note now fires
only for a `Plain` cover or other object, which keeps the masked print
(13). Gaps 3,187 → 2,569. The `system/veil_*.png` pictures, the converter's copy of the table
(`transition.rs`) and its `spmask` veils are gone; a waited call is an
instant `print` and a `delay`. Checked headless on the full build: a
full sweep (ma05 blind_01 in and out) matches the old `effectsoft` print
at 25/50/75 % within 1.3 % of the pixels; a partial one (circle_01
0 → 500 over 4 s) ends on the same frame as the old veil (AE 0).

### Plain covers as screen copies (2026-10-18)

A `Plain` cover that is moved, zoomed, turned, blended (`Request`
`AddRender`/`SubRender`/`MulRender`, the `CreatePlainEX*`/`SP*` helpers)
or waved (`DrawEffect`) was not drawn at all, so md05_016vs's 空間歪曲
flashes (`CreatePlainEXadd` + `CreatePlainEX`, faded to 500 as they zoom
to 1500, a HighWave on one) showed nothing. Such a cover is now one
sprite holding a copy of the screen under it (`spgrab n,-3`, new in the
engine: the background and the sprites drawn before n), taken when it is
first shown while the screen under it is still what it froze; the engine
zooms it (`spzoom`, the zoom over time as `spmotion` keys from the start
zoom), blends it (`spblend`), waves it (`spwave`), and a partial
`DrawTransition`'s veil cuts it (`spmask`, laid on the screen,
md05_016vs 1049-1050). 153 copies in Muramasa (55 scenes: ma04_013vs's
additive zoomed flashes, mc01_020vsa's ghosted 1100/1300 copies, …);
Django has no `Plain` covers. Wave notes on covers 10 → 3.

The same change fixes the order of a new picture's opacity and its
motion: `msp` came after `spmotion`, and `msp` ends a kept motion, so a
picture fading in to a partial opacity over longer than the wait jumped
to its end (trace: `"ended":"cut short","ran":0`); `msp` now comes first
(≈ 760 lines in Muramasa, 7 in Django). Checked in a scene build of
md05_016vs with `--shot-at`: the flash's copy runs its motion (`"ran out",
"ran":305`), the frame 200 ms in is the additive zoomed copy (RMSE 0.19
against the frame before), 800 ms in the 50 % waved copy (0.06).

#### A Plain cover shown again (2026-10-18)

A `CreateEffect("Plain")` cover keeps the screen as it was when it was
made, and keeps it after it is faded out: ma00_000 makes one per
fighter (`絵鷺沼側`, `絵垣見側`, the split screen `絵相対`), fades each
out, deletes every picture under them and fades them in again (pages
62, 64, 66, 76-78). Ours froze whatever was on screen at the fade-in,
so the split screen stayed. The cover now remembers the pictures it
froze; once it has been at 0 and the screen under it changed, it draws
those (its shade or tone over them) at its own priority. 10 such
fade-ins in 5 files by a scan of the scripts, more through
`CreatePlainEX` (made at 0, faded in later: ma04_014vs's blurred
copy, where the white flash above it no longer stays). A cover that is
moved, zoomed, turned, shaken or blended (mc03_021vs's `SubRender`
copy) or that froze a strip or movie (md04_037's rain) still keeps the
screen as before. #457's picture diff of pages 62, 64, 76-78: 83-86 % of
pixels differing before, only the text window after.

### Status as it stood before the 2026-10-18 prune

Moved whole from "Status"; its subsections are one level down.

#### Web crash on the second start (2026-10-17)

In the web build (v0.29.0) Muramasa trapped with "memory access out of
bounds" (Firefox: "index out of bounds") on every start after the
first: the engine read `kidoku.dat` through a 64 KB stack buffer, the
whole of the web build's stack. Fixed for every game in CONscripter
#148 (the submodule points at it); repro: the site's `cons/player.html`
in headless Chromium with a persistent profile, opened twice.

#### Trace diff (2026-10-17)

The original's trace (`ntrace.sh`, wine section) against our
`--auto-play 60 --real-time` run, first 50 pages of `ma00_000`: every
page pairs by its `<PRE>` line with the same text (ruby as its base).
The unwaited moves are fixed (History, "Trace diff fixes"). The
waited `Fade`s that end 30 to 60 ms after their time in the original's
trace (150 ms, the box at function.nss:1158: 195 to 225 ms to the next
statement; 100 ms: 145 to 162; 500: 530 to 545; 1000: 1008 to 1028)
are wine's frames, not a rule to copy. Every timed effect of the exe
(Fade, Move, Zoom, Rotate, Shake, transitions, volume: 14 sites)
builds `floor(time * 60 / 1000)` steps (`cb_Fade_0044f020`: time × fps
table 0x5dcb40[4] / 1000.0, `_ftol`) and advances one step per 16 ms
of real time (`tempo_advance` 00449930: ms since the object's last
frame / 0x5dcb0c[4] = 16, speed 1.0 at the default index 3), so its
animation lasts `16 * floor(0.06 * time)` ms (144, 96, 480, 960), a
little *less* than the time. It ends at the first render frame after
that (`fade_step` 00456a20 checks the index before advancing), and the
waiting script sees it at its next 16 ms poll (`wait_effect`
004489e0). Measured from those 144/96/480/960, the trace's excess is
the same 48 to 81 ms for every length (the box fade's 208/224 are 13
and 14 polls), the latency of about two ~30 ms frames under wine plus
the poll; at 60 Hz it would be 17 to 50 ms. Ours (time + 9 ms) is
within two frames of the exe at 60 Hz, so nothing changed; making
every timed effect 4 % shorter (the 16/16.67 ratio) is possible but
not done (lead's call). Django.exe has the same rule
(`init_speed_tables`, its advance 004348f0, its counts at 0x422d40 …
the fps table 0x531a90 × 0.001). Reading the diff: the
original's command ends when its next record starts, ours when its last
`ev` is written (`trace_diff.py`), so the 4 to 161 ms after a page's box
fade were the next command's time against the box fade's: not findings.

#### Exe mapping redone (2026-10-17)

`docs/games/muramasa.findings.tsv`: one row per built-in and branch the
dry run sees, marked seen/inferred/unknown with the exe address
(Muramasa.exe, sha256 `1d67e4771a72…`; `research/nitroplus/names.tsv`,
43 more callbacks from the export loop, step 2). Fixed from it: unread
arguments that change nothing (#366), a shake growing from 0 px (#374),
`SetVertex` in lower case only (#380), the colour names (#383),
`DrawTransition`'s softness and fade-out direction (#386, engine #138).
Open: `SetFrequency` on music
(1,486); `CreateSurface`/`SetSurface`, `Position`, `CreateMovie`'s
arguments, `Delete`'s 1-2 unknown. Names were carried from Django.exe by
hand (SetPan, Fade, SetVertex, colours), not by a version-tracking match
(step 21).

Findings audit (2026-10-18, `docs/new-game.md` step 17, seed 20261018):
10 "seen" rows and the 3 `left_out` kinds (no such function, `Conquest`,
the choice screen's calls) re-derived from the decompile and the
disassembly: 12 of 13 confirmed, one wrong. `CreateWindow`'s argument 6
is not inert: anything but `true` clears the window's +0x9d4
(`create_window` 00443500, the value in EAX), so `true` windows place
their children from the window's place (`init_object_place` 0044c8c0,
`parse_object_place` 00458d50), as in Django; ours places them from the
screen (74 `true` calls; 絵演窓上's children 50 px off). Row corrected;
fixed (2026-10-18): a `true` window's children are placed from its place,
their named places (`Center`, `InBottom`) count in its rectangle, and
they move with it (`Compiler::places_children`, `frame`; Django.exe's
FUN_004403a0 does the same, so 続・殺戮のジャンゴ's `中窓` busts too). `Shake`'s flip step is truncated in the exe,
rounded in ours (≤ 1 ms); `DrawTransition`'s table is stored truncated
(fldcw 0xc00 at 00457dee), as the engine's `effectsoft` does.

#### Accuracy (2026-10-17)

Every hand-typed value of the Nitroplus converter that stands for the
original (`convert/muramasa/`, the shared NSS compiler) carries a
`// src:`, `// guess:` or `// ours` comment; most are the scripts'
own numbers (`sys_*.nss`, `extra_*.nss`, `0_boot.nss`), the rest the
exe or a shot. Open guesses a frame of the original could settle:

- the click-wait flame's frame times (53 ms × 19, 77 ms × 13, a 500 ms
  pause: watched, the film redraws once a second);
- a choice's text 40 px down its plate, and red under the pointer;
- the title's glow: the exe's `SetBlur(…, 2, …)` is a trail of the last
  frames' copies (`FUN_00456810`), sharp at rest; ours a 3 px blur;
- looped and paper `Shake`s turning every 100 ms (the exe: every
  1000 ÷ (n × 0.06) ms, `queue_shake` 004462d0; they don't read n).

Settled from the exe (2026-10-17): the tones (`0043fcc0` builds grey
tables of r 28, g 151, b 77 per 256, red and blue the other way round
since the exe reads BGRA as RGB; `004401d0` Monochrome is that grey,
`004403d0` Sepia adds 30 to green and red, `004405e0` KitanoBlue 20 to
green and 80 to blue; CONscripter's `spfilter` kind 11 then an added
colour); a camera's depth rule (`FUN_00489570` at `00489a52`: a picture
under a camera scales by z ÷ (1000 − priority) about the screen's
centre, 512, 288, as measured in 続・殺戮のジャンゴ). Where the exe
draws only the innermost camera of nested ones, ours multiplies them;
not checked whether a script nests them.

#### Save and load drawn by the engine (2026-10-17)

記録/読込 are CONscripter's `screen "save"`/`"load"`
(`conv-cons/src/save_grid_screen.rs`, from `SaveGrid` with
`engine_screen`; engine #87): the page's picture, the slots down the
columns (`down`), the frames over the thumbnails (`over`), the slot
under the pointer's details (`hover`: hover frame, large thumbnail,
date, play time, chapter and last line in columns), 前頁/次頁 going round
with a 300 ms cross-fade, 削除 a toggle whose mode picks the slots'
action (`when`), the game's own questions (`save_ask_save`, `_load`,
`_delete`) and 記録しました after a save (`save_saved`). A save's text is
"|play time|chapter|last line" (the date is the file's), so saves of the
routine ("date|…") show the same. Checked on Xvfb against main's routine
(`compare -metric AE -fuzz 2%`, 12 frames from open to delete and the
load screen): 0 pixels in 8, the rest only the play time's last digit
and 3 px of 削除. Open:

- the play time is read when the screen opens (the routine read it at
  the click); S and L no longer close the save and load screens (a
  right-click and 退出 do).
- Muramasa EN's save screen is the engine's too (popup, locks, tabs,
  quick page: docs/games/muramasa-en.md).

#### Backlog drawn by the engine (2026-10-17)

ログ表示 is CONscripter's `screen "backlog"` (`conv-cons/src/line_log_screen.rs`,
from `LineLog` with `engine_screen`; engine #88): `*conv_backlog` lays
the history out with `logrows` and opens the screen; a list of rows
(`source = "rows"`, `%170` rows back from the newest) with each voiced
page's voice button, 逆/進 and the wheel a row (wheel-down past the newest
closes), the knob a picture slider on `%170`, 退出, 300 ms fades. Checked
on Xvfb against main's routine after 20 prologue pages
(`compare -metric AE -fuzz 2%`): at open, after wheel-ups and 逆, at the
newest and the oldest rows, a voice icon hovered and clicked, closed by
the wheel: 0 pixels. Again after the rebase onto engine master (#88,
#100, #101): 0 pixels at open, the rest only the knob (1 px) and 逆 at the
oldest row. Open:

- the knob sits up to 1 px from the routine's after a wheel or a drag
  (the slider rounds its place, the routine truncated); a press on the
  knob's sweep away from the knob jumps it there (the routine only
  dragged the knob).
- 逆 at the oldest row and 進 at the newest are not buttons (the screen
  turns off a scroll that can't move), so they don't light under the
  pointer; the routine's did.
- A row's voice button follows the voice switches when the backlog opens
  (below, "History", "The backlog's voices and the voice switches");
  `SetBacklog`'s speaker 鈴川 in ma01_021, voiced in no earlier file, has
  no switch there (the other 52 voiced `SetBacklog` lines have one).

#### Extras' menu drawn by the engine (2026-10-17)

特別付加目録's menu is CONscripter's `screen "extras"`
(`conv-cons/src/picture_extras.rs`, `PictureExtras` with `engine_screen`;
engine #93): the picture and its buttons (a locked one at 51/255 and not a
button), fading in over 500 ms. It closes with the mode chosen holding
its frame (`hold`), so the mode (the gallery's pages, the
recollection's list and the sound room screens, the 3D viewer still a
script routine) and the title cross-fade straight
from it. Checked on Xvfb against main's routine (this build and the
English one, which shares it): open, a button hovered, the gallery, back
to the menu, back to the title: 0 pixels; frames mid cross-fade show no
fade to black; again after the rebase onto engine master 1f90e0f: 0 pixels
in the six frames, mid cross-fades not black. Open: the four modes' screens.

#### CG gallery drawn by the engine (2026-10-17)

画像映写機's pages are CONscripter's `screen "gallery"`
(`conv-cons/src/gallery_screen.rs`, `PictureGallery` with
`engine_screen`): each category's pages are the screen's pages (19), a
slot its first seen CG's thumbnail (128/255 while some are unseen,
空白アイコン while none is) under the cursor button, the 2D/3D/BG
buttons tabs (their pressed cell while open), 前/次 round inside the
category, 300 ms cross-fades. A slot closes the screen holding its frame
and the script's viewer runs as before (cross-fades, scrolling, unseen
pictures); back from it the screen opens on the same page over 300 ms.
Checked on Xvfb against main's routine: open, a slot hovered, the next
page, each category, the viewer, back, the menu: 0 pixels (1 px on an
arrow). Again after the rebase onto engine master e585b68 (Japanese and
English builds): the same, the cross-fade into it not black. Open: the viewer is still the script's.

#### Scene replay's list drawn by the engine (2026-10-17)

回想映写機's list is CONscripter's `screen "replay"`
(`conv-cons/src/picture_replay.rs`, `PictureReplay` with
`engine_screen`; engine #100): the picture, each scene's thumbnail
(空白アイコン while locked) under its cursor button, 退出, fading in over
500 ms. A scene closes the screen holding its frame; `spgrab 999,-2`
copies that frame so the freeze (`screenzoom` at 100 %) holds the list
over the scene's first pictures as before; at the scene's end the list
fades in again from black. Checked on Xvfb against main's routine: open,
hover, the frozen list 0.4 and 0.8 s after the click, the scene at 3 and
6 s, the way back to the menu: 0 pixels (1 px at 6 s). Again after the
rebase onto engine master 4f1e1bb (Japanese and English builds): open,
hover, the frozen list, the scene at 3 and 6 s: 0 pixels.

#### Sound room drawn by the engine (2026-10-17)

電磁蓄音機/動画映写機 is CONscripter's `screen "sound"`
(`conv-cons/src/sound_screen.rs`, `SoundRoom` with `engine_screen`;
engine #101): the track buttons (a locked song's `000` picture), the
turning A-icon by the track playing (its cells in turn, 340 ms), the
movies' thumbnails under their cursors, 退出. A click closes the screen
holding its frame; the script plays or stops the track (the screen again
at once) or plays the movie (fading to black and back over 300 ms). The
room exists only in a build given the movies (`--movies`): the snapshot's
dry run has none, so its 0.txt doesn't change. Checked on Xvfb against
main's routine (dry runs with `--movies` over a full build): open, a
track played and stopped, hovers, a movie and back, the menu: 0 pixels. Again
after the rebase onto engine master 4f1e1bb, this build and the English
one: 0 pixels but for the movie's own frame (capture timing).

#### Settings drawn by the engine (2026-10-17)

コンフィグ is CONscripter's `screen "settings"` (`conv-cons/src/settings_screen.rs`,
from `PictureSettings` with `engine_screen`): the six sliders are picture
sliders (knob dragged along the invisible track, engine #78), each pair's
sides radios (the chosen side's `003` while on), the voice window a screen
over it (`settings_voices`), 承認's question its own dialog
(`settings_confirm`, the game's picture). The values are named globals
(`%conv_set_bgm_volume`, …, `%conv_set_done`); a gloval.sav from before
gets its `%2920`-`%2938` values copied once. Checked on Xvfb against
main's routine (`compare -metric AE`): the screen at open, after clicking
two pairs, the voice window: 0 pixels; dragging each slider from y 100
to 180 sets the value (trace `var`/`screen set`, `*conv_ps_apply` run
at each step). Open:

- fades (engine #81): the screen and the voice window fade 300 ms, a
  pair's chosen side 100 ms, the reset question unrolls out of x 512 over
  150 ms (back over 200) with its backdrop fading, as the routine did;
  checked by frames mid-transition (they differ from the end frame) and
  the end frames against main's routine (the knobs at 207 and 407 sit
  1 px lower: positions round since engine #80);
- a press on the knob itself keeps the grab offset (the engine's rule),
  so a drag started on it ends 2 px from the routine's;
- 続・殺戮のジャンゴ's settings (stepped sliders) still use the script
  routine.

Muramasa EN's settings (`settings_en`) are the same screen: the lying
sliders are picture sliders with the lit bar as their `fill` (engine
#80), each moving its position (0 at the right end: a negative `travel`,
whose `fill` the engine lights from the left end since engine #86;
before it the screen moved a mirrored `_shown` variable), the check
boxes toggles with `pressed`, and the
ON/OFF pairs radios. Against main's routine: 36 pixels at open (the knobs
round to the nearest pixel now), drags of the five sliders give the same
values; after a drag main's routine left the lit bar where the press
began (its bug), the screen lights it to the knob.
Open: EN's screen fades 300 ms (guess: the Japanese value, not read from
`sys_config.nut` nor seen in the original), its pairs switch at once.

#### Queue (2026-10-16)

- Merged: `CreateText`'s box height (#52). Open, each waiting on its
  engine PR: `consset` (#9, engine #5 merged: #9 moves the pointer
  next), `chvolfade` (#10, engine #6 to rebase), `sprot` as
  `spzoom`'s angle (#16, engine #13), `chfreq` for `SetFrequency` (#22,
  engine #15), menu unrolls as motions (#34, after #18 and engine #14).
  Engine #5, #6, #13 and #14 remove commands main still emits, so each
  of #9, #10, #16, #18 moves the pointer right after its engine PR.
- Done: the trailing `*` check (exe and wine, #39). A wildcard doesn't
  reach a window's, name's or camera's children, `Delete` still does
  (#41); a picture's children are placed from its corner (#43).
- `spgrab` (engine #32, merged; a copy of the screen or a sprite into a
  sprite) could stand for `CreateTexture`'s render-to-texture: not used
  yet.
- `CreateProcess` (plan changed 2026-10-16, the user: played on paper at
  convert time, not Lua; the compiler draws the screen at its syncs, so a
  process running in Lua would have no sprite of its own to change). Done:
  **A** endless loops, **B** `system.lua` from the IR, **C1** loops
  behind an `if` with a lead-in, **C2** the one-shots played once on
  paper (#151) (History, "Processes"). `ComicProcess` (83, #167): `Shake` plays on paper (`one_shot.rs`
  `paper_shake`: the picture moved to and fro by `shake_keys`, its
  wait holding the process), and a loop process's branch that doesn't
  loop (`ComicProcess`'s signs 1, 2, 5-9, 12, 13, 16-18) is played on
  paper (`loops.rs` `run_loop` → `play_once`) instead of as an action
  process, so its waits don't hold the script. With that the whole
  process passes `is_loop`, and the signs that loop (3, 4, 10, 11, 15)
  take the loop path. Limits: 16 of the 18 signs are
  `Hideable`, so they are drawn into the text window's picture at their
  end state, not moved; the zoom is about the centre, not
  `SetVertex`'s point. A zoom unlike in x and y (signs 10, 12, 17) is
  keyed in both (`Pass::keyed` fills `MotionKey::zoom_y`; in review,
  #188). Left: `EndSong` (12, music with
  `if`s), `ma03_024`/`025`/`027vs` `_runsProcess` (23, wait on the
  script: noted).
  **C3** (#171): instead of Lua,
  `Random` is allowed on paper and gives 0 there, as everywhere else in
  the converter (`builtin.rs`), so `MFlashProcess` (43) is a one-shot:
  the flashes with a 1 s pause after the 6th and then every 3rd (the
  exe: every 3rd to 6th, chosen per run; a run-time pick is a possible
  follow-up); 3 calls of 100 flashes are over `spmotion`'s 64 keys
  (noted, drawn at their end). A refused paper run (10,000 turns) no
  longer counts its notes. Lua (`Op::StartProcess`, `Script::lua`)
  stays for a process that needs the run (none left among the
  one-shots); the endless processes with `Random`
  (`BGMoveLengthProcess`, `ma03_moveAuto*`, `RandomMeimetsuSet*`, the
  shakes) don't end on paper and stay noted. A's last,
  `ma00_FireMoveAutoProcess` (ma00_000's fire, zooming unlike in x and
  y), loops now with the same PR; it wasn't drawn before.
- Then the unread-argument gaps (item 5 below), most noticeable first;
  among them, `CreateText`'s box height is read (#52) and `SetVertex`
  for every picture (#61, "History", "`SetVertex`"). Tempo waits for
  engine #2 and converter #13 (the curves are in item 5).
- `EndSong` (12) in review (#176): a music process may
  have `if`s (decided as it starts) and call `SetVolume`, `WaitAction`,
  `SetFrequency`, `SetLoop` and `Request`; a `SetVolume` on a song
  already playing goes on its clock, which a `WaitAction` moves; a BGM
  played again while it plays is left to CONscripter's loop. One that
  starts a sound effect after a wait, or fades one, is put back and
  noted (md01_023's `ZansyuSet`). The script doesn't change: the roll
  starts `EndSong` only when it outlasts its song by a minute
  (`$EndName[…][4]`), never with these songs, and no ending has a
  second song (`[8]` is `false`).
- Changed by the Muramasa EN agent (#60, shared NSS code): choice
  options set in runtime branches are picked at run time
  (mc02_031innsyoku lists the items held; md05_004 offers 別れる with a
  companion): `Op::RuntimeChoice`, `$3632` up, choice-only flags at
  %783-%798. Their line-log entries (`『…』`, `【…】`) are picked at
  run time too (`Op::RuntimeBacklogLine`, `$3638`).
- Changed by the Django agent (shared NSS code): `SetPan` is converted
  (`chpan`, engine #35); Muramasa pans nothing off centre, so its
  script is the same and its 6,494 `SetPan` calls count as converted.
- Changed by the Django agent (shared NSS code): `Shake` of a picture
  smaller than the screen moves the picture alone (a one-shot
  `spmotion`, as the exe's `shake_frame`): 83 of the 144 calls; the rest
  are on pictures deleted or moved (`spmotion2`) at once, or never shown.
- Changed by the Django agent (shared NSS code): `WaitPlay` on a sound
  waits for its rest (#70, merged), and a disused one-shot (`OnSE`,
  `MusicStart`) stays until its end (#76), so `WaitPlay`,
  `Fade` and `SetVolume` on `SE*`/`OnSE*` still reach it: about 130 new
  `delay`s (ma01_003's door: 2,528 ms), 366 fade-outs and 29 other
  `chvolfade`s; `RemainTime` on a just-started SE now gives its length
  (ma01_025's `WaitKey($残時間)`, the shakes timed by it). A `WaitPlay`
  after a `SetVolume` to 0 over time waits for the fade, as the exe's
  fade thread stops the sound at its end (67 new `delay`s; Django's
  History, "A fade to 0").
- Known limits: a window that moves (`絵演窓上`, `窓右`, about 56 `Move`s)
  has its clip jump to the end place when the move starts. Sprite movies
  (scene cuts, the turntables) don't play in CONscripter for Windows,
  which only opens a movie in an outside player: an engine item for every
  game, not started.
- `Random` (#354, 2026-10-17) is picked when the game runs. The exe gives
  `rand() % n` (`eval_primary` 0043a780, reseeded from the clock each
  call; seen). A statement that draws one (itself, in a function it
  calls or a process it starts; not an endless process, which doesn't
  end on paper) is compiled once per pick, and `rnd %81,N` chooses: each
  value for a single draw of up to 16 (`Shake_Delete`'s, `MFlash`'s
  pause), else 8 samples of every draw (the `FadeFR*`/`MoveFRP*`/
  `MoveFTP*` jitter, which were straight paths at 0). The picks start at
  the last statement where nothing waited to be shown and go on to the
  one that waits for them, sounds every pick starts alike are played
  once between them (conv-cons hands out channels in order), so the
  screen is synced where it was before (`convert/random.rs`). Left
  (771 calls, noted "Random picked when converting", a seeded pick):
  picks that reach text (SetKenki's 《…！》 lines in md05_008vs/009, the
  airship kitchen), that leave a picture at a random place
  (md04_043's 量産 moves, the cockpit compass `CP_AziChange`), or that
  give a loop to a picture shown only later. The endless processes with
  `Random` (`BGMoveAuto`, `RandomMeimetsu*`, `GunFlash`) stay noted as
  before.
- Done: `SetBlur`'s trail behind a moving picture as CONscripter's
  `sptrail` (engine #25, converter #82; "History", "`SetBlur`").
  Its last argument draws the copies over the picture (`false`) or
  under it (`true`): `sptrail`'s `over` (engine #83, "History").
- Decided: the hover sound's 0.125 s of silence is in the original file
  and stays. The trace checker's `movie` rule will skip movies that start
  from black (an engine-tools change, once the engine queue drains).

The whole story compiles, the 立方陣 included, and every ending of the
chart is reached in the converted control flow. **Route runs**
(2026-10-14, see "Testing"), the full build played in CONscripter
0.19 headless with `--auto-play` and a debug trace, every page drawn
and the files checked against nstrace:

| Run | Ending | Files | Pages | Time | Result |
|---|---|---|---|---|---|
| 1 | 一条END | 246 | 26,704 | 24 min | passed |
| 2 | 香奈枝END | 349 | 26,085 | 25 min | passed |
| 3 | 茶々丸END | 300 | 29,136 | 26 min | passed |
| 4 | bad end (`mz00_000`) | 436 | 31,500 | 28 min | passed |
| 5 | bad end | 438 | 31,670 | 28 min | passed |
| 6 | bad end | 440 | 31,714 | 28 min | passed |
| 7 | bad end | 442 | 31,777 | 28 min | passed |
| 8 | bad end | 455 | 32,873 | 28 min | passed |
| 9 | 村正END | 510 | 36,276 | 29 min | passed |
| 10 | TRUE END | 514 | 36,449 | 30 min | passed |

(How the runs were made and what they fixed: "History", below.)

The checker (`cons_trace_check.py` with `research/nitroplus/cons_trace_check.json`)
found the seven bugs fixed in "History"; on the rebuilt game its `text` and
`duration` findings are gone, the favour panel no longer lingers, and
fewer loops double and characters stay off screen. The `voice`
findings left are lines in 『』 that have no voice in the source either.
The rest of its report is expected for this game: `crowd` and `offscreen`
(NSS layers many full-screen objects and parks some off screen),
`sound` (the battle scenes layer one effect on several channels a
few hundred ms apart, as the source does; `se戦闘_` is in the config's
`known`),
`memory` (a few pre-scaled zoom pictures of 67-77 MB, see "How the
compiler works").

Testing notes: other agents stop engines with `pkill -x conscripter`;
run ours through a symlink with another name (`--engine
out/conscripter-mura`). The route runs and `--at` starts are in
`research/nitroplus/engrun.py`.

Next, in order:
1. **System screens** (step 7), under way. The original was filmed
   under wine from patched boots (title, right-click menu, quick bar,
   save/load, confirmations, settings and its voice window, backlog,
   click-wait icon; transitions also slowed ×10); shots and videos are
   in `research/nitroplus/local/` (git-ignored). Every confirmation is
   drawn from `cg/sys/dialog` parts: no Windows dialog needs designing
   (the only `Message()` box is for a display option we leave out).
   Behaviour seen: right-click at a text wait opens the menu, wheel-up
   opens the backlog, the wheel scrolls it a line a notch and wheel-down
   at the newest line closes it, right-click closes every screen (No in
   a dialog), Esc asks ゲームを終了しますか, the title's 終了する quits
   with no question, the backlog shows no names. Plan, in order, each
   screen checked beside the original:
   1-6. Done: button strips, the title, the right-click menu, save and
   load, the backlog and the settings screen ("History", below).
   7. Motion and movies: the title's fire flicker and its openings, the
      dialog unroll, the menu cascade (step 3, in "History") are done. Left, by the
      user's choice (2026-10-15): the quick bar sliding while text types
      (it slides at click waits; doing it while text types needs the
      engine to move the bar);
      - How each was done: "History", below.
      - The trace's auto-play skips movies (CONscripter) but reports
        missing ones; the prologue (`--chain 2`) has three cuts. The
        checker's `movie` rule (CONscripter ca3904b) still sees what a
        movie leaves: the engine redraws the screen from before the
        movie as it ends, so sprites loaded after `mpegplay` show a
        moment late. The title's openings and the scene cuts load what
        follows before the movie ("History").
        CONscripter 3f368bc (after 0.24.0) draws the text window over
        an `mpegplay` movie and plays a movie file given to `lsp` in the
        sprite's place (`spmoviewait n,1` waits for it): the cuts under
        the text play that way ("History"). A skipped movie stops the `dwave` sounds
        started with it (CONscripter 265942e): the title openings'
        clash no longer plays over the title.
      - 動画 切 is `moviestop -1` (the script's flag, as
        `MoviePlay(f, #SYSTEM_break_play_movie)`): the title's openings
        are always skippable, `mvOP`/`mvOP02` only with 動画 入
        (default 切, as the original).
      - The original under wine draws no movie (`CreateMovie` leaves the
        scene as it was, `md04_045`'s first page matches its bare
        background): wine decodes MPEG-1 through 32-bit GStreamer
        plugins, which the test machine lacks, so the movies can't be
        compared there yet. The smoke's strength (`SubRender` subtracts
        the white smoke; ours draws it black over the scene, lighter) is
        unchecked.
      - On Windows CONscripter only opens a movie in an outside player.

   8. **Extras** (特別付加目録, `extra_*.nss`), done. Filmed in the
      original from a patched boot (`#ClearG` and a few CGs, a song, a
      movie and a 3D entry set; shots in `research/nitroplus/local/shots/
      extras/`). Five screens: the menu, 画像映写機 (CG gallery: 349 slots
      in 2D/3D/BG, 805 CGs, each a `#<cg name>` flag the story sets),
      回想映写機 (ten H scenes, each unlocked by its file's `#ma01_020`-style
      flag and replayed from the file's `$PLACE_reco` branch), 電磁蓄音機/
      動画映写機 (37 BGM, five songs, three movies; the track chosen stays
      the extras' music, `#エキストラＢＧＭ`), 劒冑回転目録 (46 looping 400×400
      movies of the armour with name plates and specs, `#av_`/`#aw_`).  Plan, in order, each beside the original:
      1-7. Done: the menu, the gallery, the sound room, the
         recollection, the 3D viewer, the way back to the title and the
         title's ライナー画像 button ("History", below).

   The user's decisions (2026-10-14): the pressed state (`003`) becomes
   an engine feature that is on for every script; the settings CONscripter
   lacks (music/SE/voice mute, voice stops on click, auto waits for the
   voice, auto/skip stop at a choice, movie skip, game speed) are added
   to the engine the same way (since then no engine addition is opt-in); 選択肢に戻る is implemented
   (a save to a reserved slot before each choice).
   Those settings are set with the engine's `consset "key",value` (read
   back with `consget`; keys in the engine's `COMMANDS.md`). Checked
   headless (2026-10-16): `consset "mutebgm"`/`"mutese"`/`"gamespeed"`
   then `consget` returned the values set, and the reset to 0/100 read back.
2. The largest leftovers below: `CreateProcess` (C2, C3 above), non-blocking
   `Move`/`Zoom`.
3. Optional: the 18 files with no `else` (below).
4. Gaps from the exe review (2026-10-15; the exe's side is in
   `research/nitroplus/builtins.md`, the functions in
   `research/nitroplus/names.tsv`), most noticeable first:
   - Text markup dropped: `<VALUE name=$mc02_時間>` (the airship clock in
     `mc02_031time` prints a gap: a string chosen by the runtime
     `$mc02_TimeCount`, and the IR has no runtime strings), the
     ASCII-art pages (`Request(…, PushText)` + `NoLog`: shown at once,
     not in the backlog), `<voice mode="off">` (3, meaning unknown).
   - Tempo (easing) of `SetVolume` and of the text window's `Fade`s is never read (`Move` and `Zoom`'s is:
     "History", "Tempo of `Move` and `Zoom`").
   - A 4-argument `Fade` probably doesn't wait in the exe (49 calls
     wait in ours); check under wine first.
5. Gaps from the unread-argument notes (2026-10-16; each built-in's
   arguments the converter doesn't read, with counts, in
   `research/nitroplus/builtins.md`, "Arguments not read"), most
   noticeable first:
   - Tempo (easing) of `SetVolume` and the text window's fades (`Move`,
     `Zoom`, `Rotate`, `Fade` and `DrawTransition` done: "History"): a
     sound's volume has no curve, nor `windowfade`; a fade folded into a
     picture's motion keys is straight. A move with a fade folded into its keys still
     goes straight. A 0 ms `Fade`'s or `SetVolume`'s tempo, a 0 ms
     `Shake` stop's wait and `CreateWindow`'s last argument are read:
     they change nothing in Muramasa.exe (`cb_Fade` 0044f020,
     `sound_set_volume` 00475f50, `builtin_CreateWindow` 0040b3a0); 9
     `SetVolume` fades over time keep a straight ramp (the exe steps them
     by the tempo, 00478570).
   - Story `CreateText`: a sideways text turned by `Rotate` sits about
     17 px lower than the original's under wine (cause unknown; "History",
     "Story `CreateText`").
   - `DrawTransition` (FUN_0044fe00, 00457bf0, seen): the mask is a
     threshold table, from and to × 0.255 its ends, shown up to the
     threshold, then a ramp losing 255 ÷ p × 255 ÷ softness a grey level; a
     fade-out's threshold falls, so the picture leaves the bright parts
     first. Full sweeps (0 → 1000, 1000 → 0: 2,073 calls) are drawn so with
     CONscripter's `effectsoft` (engine #138). A partial one (367 calls) now
     leaves its picture drawn through the table, laid on the screen
     (`spmask`, "History", "Veils laid on the screen"); its sweep is still a plain mask print, and one on a
     `Plain` cover is drawn unmasked (noted). 565 calls start from another
     opacity than shown (noted). A mask cg.npa lacks (ma04_018vs/020vs's
     `slide_01_02_2.png`, `cg/data/7.png`, an unset `$Transition`): the
     exe's step returns before the transition (seen), none drawn.
   - `Conquest` returns 0 whatever its arguments (`Random`: picked at run time, Status).
   - Done: a whole-screen `Shake` that grows from 0 px (7 calls, e.g.
     5000 ms to 20 px) shakes; it was dropped (`queue_shake` 004462d0: the
     size goes from arguments 2-3 to 4-5, seen).
   - Done: `SetVertex`'s names count in lower case only (nothing lowers
     them: `eval_arg` 00413740, `FUN_004891e0`, seen), so `Center` and
     `Middle` are atol's 0: 絵演鎧* zooms about its left edge (mb02_022,
     mb02_023vs, mb03_016vs, mc01_017vs), Rollbg from its top (ma01_022vs).
   - `MoveCamera`'s arguments in a process (4 calls). (`CreateEffect`'s
     place and size are read: every story call covers the screen, a tone
     or `Plain` over part of it is noted; #355.)
   - `LockVideo`, `WaitText`, `SetFrequency` (but 1000),
     the time limit (?) of `WaitAction` and the
     other waits: dropped; meaning to check in the exe.
6. Gaps from the script coverage (2026-10-15; every file, built-in,
   `Request` action, text tag, `$SYSTEM_` variable and `system.ini` key
   with its status in `research/nitroplus/builtins.md`, "In the
   scripts: coverage"). Every file has a status (no gaps there); new
   gaps, most noticeable first:
   - `CreateEffect` Drop (1, mc04_052's snow: an image per drop, swung
     and turned) and Lens (2). Rain is converted (#161; values checked
     against the exe again 2026-10-17): `convert/rain.rs` plays
     the exe's rain (`00435e60`, `00436390`) into a 24-frame strip of the
     effect's size (`$SYSTEM_effect_rain_dencity` drops, at most 128,
     grey streaks 196 px with the exe's alpha ramp, falling their speed
     per 16 ms, redrawn every 42 ms), looped; its place is read, and
     `Zoom` and `Rotate` reach the strip (z as the `spzoom` angle, x or
     y as a shorter zoom, no perspective). The frame period is the
     exe's 16 ms (`init_frame_tables`, `0048c5d0`). The
     tone kinds Sepia, Monochrome, NegaPosi and KitanoBlue tint the
     pictures below the effect (#137, #146). `SetTone` (228) draws the
     object in that tone with the same filters (the exe rewrites its
     pixels, `FUN_00452000`); on a `Plain` cover, the frozen pictures
     under it are drawn so; `null` takes it away. The sepia and
     KitanoBlue colours are still ours (unverified). `SetShade` (163)
     likewise blurs the object (or what a `Plain` cover froze): the exe
     runs its level's passes of a 3×3 mean over the pixels (`FUN_00465b10`;
     a second call adds passes, `NULL` restores), drawn as one `spfilter`
     box blur of the same spread, scaled with a zoom the converter
     renders. Unverified: the engine's blur keeps the alpha, which the
     exe blurs too.
   - `DrawEffect`: the waves (LowWave, MiddleWave, HighWave, SuperWave)
     are converted, drawn by the engine's `spwave`
     (CONscripter#76). The exe (`FUN_00430a70`, `FUN_00430de0`, `cb_00431230`)
     shifts each row by trunc(sin(2π·phase/256) × amp) px, the row's phase
     row × K / height (K 64, 512, 1024, 4096 by name) plus one per tick of
     more than 32 ms; amp goes `from` → `to` over `time` by the tempo, then
     stays (a wave ramped to 0 is removed). Left: the 10 waves on a
     `Plain` cover (noted; the frozen screen would wave), Ripple (4) and
     the Split kinds (7), noted; a wave picked up later (a picture shown
     after the call, or moved to a new layer) restarts its rest straight,
     without the tempo, and a picture cut to the screen loses its part's
     phase offset.
   - `CreateStencil` (59), converted ("History", "`CreateStencil`"):
     left a picture moved or zoomed over time under a stencil (none in
     this game), a zoom about a vertex (cut as about the centre) and a
     `Plain` cover under a stencil (noted only).
   - `CreateMask` (28), converted ("History", "`CreateMask`"): left
     the same as `CreateStencil`'s, and a colour mask is exact only for
     the alpha blend (per-channel factors on one picture).
   - `SetSurface`/`CreateSurface`, `LoadImage` as a texture source
     (on since 2026-10-17, "History"; 25 `CreateTexture`s from an
     object or the screen are still noted).
   - `Request(…, NoIcon)` (4) is dropped without a note: a `Request` on
     an object the converter doesn't keep returns silently, so the dry
     run's notes undercount those.
   - `system.ini`: the auto-mode icon (`icon_auto`, 15 frames) and the
     line-end kinsoku (`bad_end`) aren't used.
   - Check under wine: `<BR>` (16 uses) isn't in the exe's tag list but
     is a line break in ours. (The 13 `DrawEffect` names with a trailing
     space match: the exe allows a space or tab after a name.)
   - Small: 25 sounds missing from the archives, 2 transition masks
     missing (`cg/data/slide_01_02_2.png`), 2 statements that don't parse, 2 voices outside a text
     block, 3 BGMs without loop played looping (the notes give counts
     only, not where).

**Leftovers** (counts from the dry run's notes):
Counts from the dry run on main (2026-10-17, `metrics.json`): 1,162,127
source calls, 1,131,605 converted fully, 21,928 with gaps, 8,594 left
out on purpose.

- `CreateProcess` (510 calls; the battle HUD `CP_*` in
  `function_cockpit.nss`, fire flicker, comic panels): endless loops
  play as engine motions (`convert/loops.rs`), one-shots on paper
  (C2, "History"), the roll bar's wobble too. Left: 108 notes, most
  `TurboBlur3` (10), `DenziBladeChargeSet` (10) and the
  `ma03_*_runsProcess`/`ma03_moveAuto*Process`/`BGMove*Process` runs
  that don't end on paper (about 45). `Request(<process>, Stop)` (1,126
  notes over the paths; `プロセス*`, `プロセス１`, …) is dropped: not
  checked whether a looped motion keeps running where the original
  stops it.
- Moves and zooms over time that come with a fade or a new picture are
  drawn as cross-fades: 38 left (about 1,200 each before the motions).
- `Rotate` about z of a picture is CONscripter's `spzoom` angle (still) or an
  `spmotion` angle (over time); left: about z of other objects (31),
  a turn over (about x or y) over time, mirrored at once (10).
  `CreateTexture` from an object or the screen (25).
- Cameras (`CreateCamera` 40, `MoveCamera` 105): drawn by
  続・殺戮のジャンゴ's model, measured there under wine (a child of depth
  `p` seen `z / (1000 - p)` times about the screen's centre, the
  camera's place the other way; `Kind::Camera`), now for 装甲悪鬼村正 too,
  its moves engine motions with their tempo. A camera at `Center`,
  `Middle` is at 0, 0: only so do ma04_005a's pictures cover the screen
  through it. Unverified against the original: its zooms other than
  1000 (Django has none).
- `BezierMove` (335 in the script, outside processes) moves the picture
  to the path's end as `Move` does, and the sync's motion goes through
  the path (`loops.rs` `bezier_move`: the curves cut into straight
  pieces as in a loop, `sync.rs` `along`); its tempo eases the whole
  motion. Where the sync draws no motion (a cross-fade: a new picture,
  a fade with it), the picture is only put at the end.
- 18 files whose `if`/`else if` chain on earlier flags has no `else`:
  by the story's logic one branch always matches; the fall-through ends
  the run.
- Engine: an exit with status 255 in vertical text was put down to a
  half-width character under `tateyoko 1`, but CONscripter draws those
  (rotated) and nothing in drawChar exits. It was most likely a script
  error in the text (a half-width `/`, `#`, `?` or `%`; up to 0.18.0
  every script error exited with 255). Now a script error exits with 2
  and the trace's `error` record gives the message and the line; the
  checker's `text` rule finds those characters in text lines.
- Engine: CONscripter segfaults on `end` after a `dwave` voice with Ctrl
  held (seen once in a small test script; not looked into).


### Fades that change nothing (2026-10-18)

`cb_Fade` 0044f020 (Django.exe cb_00438fb0 the same) compares the
target's opacity byte, round(n × 0.255), with the object's: equal, it
sets it and starts no action, so a waited `Fade` doesn't wait and an
unwaited one adds nothing to the next sync. A new object's byte is 255
(`FUN_00488740`, called by every `Create*` through FUN_00488190), so
`CreateColorSP(…); Fade(…, 1000, 1000, …)` (md02_021's `黒幕１`,
md02_004, ma04_013vs's `絵色白`) shows it at once, not as a fade-in.
The compiler skips such a `Fade` when every object it names keeps its
byte and none has an action still unsynced (a `DrawTransition` under
way: ma04_018vs's `FadeDelete("絵窓/絵被弾効果*", 450)` right after
one keeps fading). Objects left out (`untracked`) and the text window's
boxes wait as before. 174 calls in Muramasa: `bloodC*`/`bloodL*` at 0
faded to 0, cockpit `@CPEn*` at 1000 faded to 1000, `FadeDelete` of
pictures already out, new pictures faded to 1000. The 0.txt: 159 hunks,
77 only `delay`/`wait` gone or merged; the rest follow from syncs no
longer stretched by the dropped fade (ma01_022vs's `絵色200` 500 ms
fade of a picture at 0 made the next 200 ms sync 500): sprite numbers
of the cockpit pieces and counters re-drawn, the same pictures.

### Actions on no object (2026-10-18)

A `Fade` or `Move` whose name matches no object starts no action in
the original (`cb_Fade` 0044f020 queues none; the wait, 00445ac0, waits
on the objects' action slots), so a waited one doesn't wait and an
unwaited one adds nothing for the next sync to play out: ma00_000:2501
`Fade("silver_son*", 500, 0, null, true)` after they went (+500 ms in
ours), and above all `TypeEnd`'s unwaited `Fade("@Face_Sepia", 150, …)`
(function.nss:1058), which made ours wait 150 ms after each page
(4329 `delay 150`s): the trace has the next statements 30 ms after it.
The compiler keeps the names of the objects it leaves out (a `Create*`
that makes no object, as the choice screen's plates) until a `Delete`,
and an action on one still waits as before. The earlier worry ("@boxCC
matches none") was these: the boxes are kept, `@Face_Sepia` is never
created.

### Auto mode and `WaitKey` (2026-10-18)

Auto mode ends a `WaitKey(N)` or `WaitKey()` after 1.5 s in the
original: every one in the 300-page trace (ma00_000:2496 to 2662, the
`Gin_song*` lines) ends after 1505 to 1527 ms, with N = 2000 or none.
The exe (builtin_WaitKey 0040f9d0) waits on its input events with N
as the timeout, and the defaults (0047b400) set a 1500 (0x5dc) beside
the auto settings; 続・殺戮のジャンゴ's defaults
(`reset_system_defaults` 0045cb30) set it too, so both games take it
(not checked in a Django trace). The compiler puts `Op::AutoWait`
(1500 ms less what the statement's sync showed) before the wait when N
is longer, and before `WaitKey()`'s click wait; conv-cons makes it
`isskip` and a `delay` in auto mode, the wait itself otherwise.

### Boot logos (2026-10-18)

`0_boot.nss` runs `TitleLogo` before `TitleSet` while `$Logo` is 0: at
the first title of each run (`$Logo` is a local, set once a game has been
started; no first-launch-only step, no save flag). Steps, ms: white over
black 300; `media_a.jpg` in 500, `WaitKey(3000)`; `media_b.jpg` cross-fades
in 500, `WaitKey(3000)`; out to white 500; `Wait(1000)` (no click);
`Nitroplus-logo.jpg` in 1000, `WaitKey(3000)`, out to black 500;
`Wait(500)`; `注意事項.jpg` in 1000 with `WaitKey(1100)`, `WaitKey(8000)`,
out 500 with `WaitKey(600)`; `Wait(100)`; then the title's opening. A
click ends a `WaitKey` hold (and the fade under it), never a waiting
`Fade` or a `Wait`. Before this PR none of it converted (the dry run went
from the notice to the title).

Now `Game::boot_logos` compiles `TitleLogo` from the script into a
subroutine (`conv_boot`) that a `BootStep::gosub` runs once a run,
before the title (`%2960`, as the title's openings). Checked with a
`--real-time` trace (`--file ma00_000.nss --chain 1` build): each step's
time is the script's within 10 ms, 23,672 ms in all against 23,600.
Also: `WaitKey(ms)` after a fade that doesn't wait now counts the fade in
its time, as `Wait` did (the fade was dropped before: `print 1` and the
whole wait).


### Eased sound ramps (2026-10-18)

`SetVolume` and `SetFrequency` build their steps by the tempo as `Move`
does (`FUN_00478570` → `apply_tempo`); ours were straight. Now the
compiler puts `Op::SoundEase` before the ramp and conv-cons gives
`chvolfade`/`chfreq` the curve (engine #145: an optional curve; a
stop's fade-out goes along it, then the sound is halted): 32 `chfreq`
and 7 `chvolfade` in the dry run. Still straight: the music's eased
fade-out (mc02_013, `bgmfadeout`; noted) and a sound's fade-in as it
starts (`dwavefadein`).

### Unwaited fade-outs (2026-10-18)

A `Fade` to 0 that outlasts the sync's wait used to be part of the
blocking cross-fade, so the script waited for it (up to `UNWAITED_MS`):
ma00_000:1624's `Fade("絵色100", 300, 0, null, false)` before `OnSE`
and the text (the text 36 ms after it under wine, 918 ms in ours, now
609 ms: the rest is the box's own fade-in, as on every page). Such a
fade is now an engine motion to alpha 0, as a fade-in from 0 was
already, and the picture is taken away at a later sync. A picture
disused as such a fade starts (`FadeDelete(…, false)`: `Fade` then
`Request(…, Disused)`) is kept until a sync after the fade, as the
exe's frame on page 3 shows (ma00_000:137, the old sky still at about
a third after the 1 s `Wait`); not one that also zooms or turns
(mb02_028vs:123) or that a `DrawTransition` wipes (md01_029vs:2107):
those go at once, as before. Limit: the later sync takes it away
even if the reader clicks on before the fade's end (by the script's
clock).

### Zoom to 0 (2026-10-18)

A `Zoom` to 0 along an axis was kept at 1‰, so a 1 px line or dot stayed
(ma04_015vs ×2, mc02_029, 32 static 1×1 copies in the dry run; django's
dja023 one). The exe stores the zoom as `atol` gives it
(`FUN_0044f2a0`, only the time 0 becomes 1), so a picture at 0 is not
drawn. Now it isn't (`sync`); one zoomed to 0 over time is squashed by
the engine on the sprite it was (`spmotion` to zoom 0, moved and turned
with it), then removed; one not shown that grows from 0 is placed where
it ends and keyed from its zoom of 0 (`Compiler::pending_zoom_from`).
`Random` over a picture at 0 (ma04_016vs:6680) is now picked when
converting (35 draws): its moves change only the unshown state.

### Trace diff fixes (2026-10-17)

A `Move` of a picture not shown yet (created since the last sync, as in
ma00_000.nss:128-136, 370-373, 380-385) put it where the move ends; the
original pans it from where it was created while the script goes on
(`PrintGO` cuts it mid-move). Now the next sync places it at its end and
moves it there from its start (`spmotion`, its tempo kept), and, as a
moving picture, it no longer stretches the sync's cross-fade to
`UNWAITED_MS`: `Fade("背景30", 500, …, true)` after the two pans (375) is
`print 10,500` again, 509 ms against the original's 545 (was 1009);
`FadeDelete("上背景", 500, true)` (387) the same. The script changed
wherever a new picture moves (about 1,360 `spmotion`s, fewer layer
changes and cross-fades; 続・殺戮のジャンゴ about 200, the English release
the same as this game).

### Veils laid on the screen (2026-10-18)

A partial `DrawTransition`'s veil is read by the exe's draw (`00418320`)
at the screen pixel, so it stays put while the picture moves:
mc04_022vs's `MoveFFP1("@絵効果線")` jitters its rain strip ±10 px (a
looped `spmotion`) under the veil. The picture was cut once where it
stood, so the cut moved with it. CONscripter's new `spmask num,"mask"`
(engine PR #153) draws a sprite times a mask fixed on the screen,
applied after the sprite is drawn (whatever its motion, zoom, turn,
wave or blend). The converter now gives every veiled picture (images
and strips) `spmask` with its `system/veil_*.png` in place of a cut
picture (`Shown::mask`, `Op::SpriteMask`). A veil that comes or goes is
placed again, never done by a motion, so its sweep stays a mask print
(`motion`). Stencils (`CreateStencil`/`CreateMask`) are still cuts.
Checked in an engine frame of mc04_022vs's strip (zoomed 2×4, turned,
moved (0, −10)) over its veil: the frame equals `bg + (strip − bg) ×
veil` at the screen pixel within 1 on all 16,856 lit pixels; the cut
moved with the strip differs on 2,661 pixels (max 58). Snapshots:
Muramasa and EN change (about 1,450 hunks: `lsp` of the uncut picture plus
`spmask` in place of `stencil_*` cuts (669); the layers get `csp` when
they are freed; a fade that changed nothing on a cut picture is gone; a
`DrawTransition` from one veil to another on the energy gauge is now a
mask print, where it was instant); Django's script doesn't change.

### Partial `DrawTransition` (2026-10-17)

A `DrawTransition` that ends between 0 and 1000 (to 200, 250 → 150)
leaves its picture drawn times a table of the mask's grey, for good
(seen in Muramasa.exe: `FUN_0044fe00` builds the table, `00457bf0`
steps it each of `time` × 60 ÷ 1000 frames and keeps it once done unless
the level is 255; the draw `00418320` reads the mask stretched to
1024×576 at the screen pixel, the green byte, and multiplies it with the
picture's own `Fade` opacity, which the transition leaves alone; only a
new `DrawTransition` or `Delete` takes it away, `0045a490`). The
converter now does the same with existing commands: `convert/transition.rs`
replays the table (exact for the last frame; earlier frames take a
straight curve, which only decides leftovers of the start's ramp), the
assets draw it as a screen-sized `system/veil_*.png`, and the picture
is cut by it as by a stencil (`Compiler::stenciled`, `lsp` of a new
picture), keeping its opacity (fire01's `焚き火光`: `Fade` 0, the
transition, then `Fade`s up): 367 calls, 6 of them on effects, drawn
unmasked at first; since 2026-10-18 the 4 on rain strips (mc01_021vsaba,
mc04_022vs, md01_029vs's two) are cut too, each cell by the veil at its
screen place through the strip's zoom and `lsp2` turn (an engine frame of
mc04_022vs's turned, 2×4-zoomed strip: cut ÷ uncut equals the veil exactly
wherever the veil is flat over 9×9 pixels, 4,748 lit pixels; it differs
only at the veil's steps, where the zoomed sprite is filtered), leaving
the 2 on md05_016vs's `Plain` covers unmasked (noted). The other partial ones (from a level to 0 or
1000, or to a level under 4, which × 0.255 is 0) end unmasked or hidden,
as before. Checked in a
scene build of ma04_013vs: the five veils equal a Python replay of the
exe's table over the source masks (0 differing pixels of 589,824 each).
Snapshots: Muramasa and EN change (≈ 1,300 hunks: cut pictures in place
of colours and pictures, `msp` opacity changes gone; 215 more cut
pictures); Django's script doesn't change.

### `Wait` and clicks (2026-10-17)

The exe's `Wait` (Muramasa.exe `0040f7f0`, Django's `0040bd80`) ends
only on quit, a state change or skip mode; a click doesn't end it. Ours
was `delay`; it is `wait` now (the part shown over the running actions
too), which CONscripter ends in skip mode (engine PR #134). `WaitKey(ms)`
stays `delay`. The script changed only in those lines (about 6,800).

### The backlog's voices and the voice switches (2026-10-17)

The exe keeps each log line's speaker with its voice (`FUN_00403730`; a
text's `<voice class>`, `SetBacklog`'s third argument) and its backlog
(`FUN_00403910`) offers the voice only when the speaker is not in
`#SYSTEM_voice_enable_list`, the classes switched off
(`ConfigCharacterVoice`): it decides when the backlog is drawn. Ours
logged a voice only when it played, so a voice switched off while read
never had a button and one switched off later kept it. Now every voiced
line runs `logvoice "<voice>",<character>` before its `if %970+c == 0
dwave` (CONscripter's history voice without playing it, with a tag),
`SetBacklog` lines `logadd "<text>","<voice>",<character>` (the speaker as
a class, or the class it starts: 村正 for 村正さん), and the backlog's rows
list has `voice_off=970`, which leaves a row's button out while `%(970 +
tag)` is set (engine PR blankiekun/CONscripter#103). 0.txt: 36,053
`logvoice` lines, 53 `logadd`s with a tag. Checked on the full build
(`--goto if0_else`, a `gosub *conv_backlog` after ma00_000's 卵 lines,
`--skip-screens none`): voice buttons on rows 6, 7, 9, 10, 11, 13-16
with `%971=0`, on 6, 7, 10, 13 with `%971=1` (the five rows of character
1 lose theirs).

### Tempo of `Fade` and `DrawTransition` (2026-10-17)

Seen in Muramasa.exe: the fade's per-object step (`0044f020`) and the
transition's (`FUN_0044fe00`) ease by the tempo through `apply_tempo`.
A sync's transition now goes along the last eased fade's curve before
its print (CONscripter's `effectcurve`, engine #85; `Op::TransitionEase`):
588 transitions in the story; 61 in 続・殺戮のジャンゴ, whose exe eases the
same way (`cb_00438fb0` and `FUN_00439d20` call its curve table
`FUN_00434ea0` on the tempo). `Game::fade_tempo` turns it on for these
two, and for Muramasa EN with the port's own reading of `AxlDxl`
(`docs/games/muramasa-en.md`, "Eased fades"). The rest of
the 3,869 eased `Fade`s take no time, fade the text window, or become
part of a picture's motion.

### The text window's fades (2026-10-17)

The box is an ordinary picture the scripts fade (`Request(…, Hideable)`
only lets the player hide it): the text's preprocess fades it in over
`$BoxFadeTime` (150 ms) and a waited `Fade(…, 0, …, true)` takes it
away. Ours showed and hid it at once (`textoff`); now CONscripter's
`windowfade` (engine #82, `Op::WindowFade`) fades it: 5,644 hides
become `windowfade 0,ms` with the script's wait after (most a `delay`,
the rest joining a sync's wait), and 5,573 shows fade the window in from
0 with the next page (after its `setwindow`: `windowfade 256,ms,0,0`,
`textclear:texton`), the text held for the waited fade (`delay ms`), as
the preprocess's `Fade(…, true)` holds the script before
`Request(…, Enter)`. An unwaited fade to 0 is a box switch and changes
nothing. Muramasa EN the same; 続・殺戮のジャンゴ's script doesn't change.

### Pictures made from `LoadImage` (2026-10-17)

`CreateTexture(…, "name")` draws a picture `LoadImage` loaded under that
name (seen: `builtin_CreateTexture` looks the source up by name,
`FUN_004491a0`, and copies its picture, `FUN_00488670`, before trying a
file). Muramasa now takes `Game::loaded_textures`: the story's pictures
made that way (the scrolling battle backgrounds, `雑魚イメージ`, `イメージ１`)
are drawn, about 11,000 lines of `0.txt` (new `lsp`s, `spmotion`s and
the re-layering they cause). The choice screen's plates (`select_img*`
in `function_select.nss`) stay the engine's choice: left out (1,776
`CreateTexture`, 504 `LoadImage`; 続・殺戮のジャンゴ's 40 and 10 the same,
its script unchanged).

### Tempo of `Rotate` (2026-10-17)

Seen in Muramasa.exe: `builtin_Rotate` passes its tempo to
`FUN_00446010`, which keeps it in the command (`+0xa0`); the turn's
step `FUN_0044f620` eases each of the three angles by it through
`apply_tempo`. A turn about z over time (an `spmotion2` angle) now
takes the tempo's curve as `Move` and `Zoom` do: 187 motions in the
story (the same in Muramasa EN); turning over about x or y already read
it (`convert/turn.rs`). 続・殺戮のジャンゴ's script doesn't change (its
42 eased `Rotate`s now count as read).

### `Shake`'s `n` and tempo (2026-10-17)

Seen in Muramasa.exe, as 続・殺戮のジャンゴ's: `queue_shake`
(`004462d0`) stores the step 1000 ÷ (`n` × fps ÷ 1000) ms
(`00446381`-`004463c3`) and returns at once for `n` above 1000;
`shake_frame` (`004578e0`) flips the offset once `timeGetTime` has gone
more than a step past the last flip, the size eased by the tempo over
`time` × fps ÷ 1000 frames. fps is the double at `005dcb40` + 8 × the
speed index `005dcaf0`, which `init_frame_tables` (`0048c5d0`) sets to
4 and nothing changes: 60.0. So `n` 1000 flips every 17 ms, 500 every
33 ms, 200 every 83 ms. Muramasa now takes `Game::shake_jolts`: the
1,284 screen shakes are `screenshake` mode 2 with that step and the
tempo's curve, and the 81 picture shakes' `spmotion` keys turn at that
step (the 100 ms and 16 ms guesses are gone). Muramasa EN keeps the old
shake (its exe is not checked). `Shake`'s argument 8 (wait, 58 calls)
is still unread.

### `CreateStencil` (2026-10-17)

The exe (`00409fa0`): mode 128 (every call) turns the picture into its
alpha; `false` leaves the object a container (a wildcard doesn't reach
its children, which sit at their own place). The converter cuts each
child's picture by the stencils over it (`Assets::stenciled`, a zoom
about the centre), kept while the picture, its place and zoom and the
stencils' places stay the same; a child created off the screen and
moved in (`CreateColorEX`, `CreateTextureEXover`) is cut where it ends
up. Checked in our build (out of 89 cuts, one empty: a one-frame
intermediate in mb03_011): ma04_013vs's red and blood over each
standing picture (seen: 皇路's red cut has exactly the picture's 83617
opaque pixels at its place; this was empty before the place was in the
cut's key), ma05_019's spider (seen: the event picture only inside the
spider, its wave on), ma02_009vs's zoomed bg201 inside 月山 (seen). Not
compared with the original (no wine run). Inferred, not checked:
ma04_013vs's `DrawTransition` of the red to 200 through
`slide_02_01_0.png` was drawn as a plain 20 % opacity (cut by the
mask's table since "Partial `DrawTransition`"). Snapshots: Muramasa and EN
change (the cut pictures; inferred: pictures under a cut child are
no longer culled as under an opaque one).

### `CreateMask` (2026-10-17)

`CreateMask(name, priority, x, y, picture, wildcard)` draws nothing
itself; its children are drawn multiplied by the picture, byte by byte
(colour and alpha), and not at all outside it or where it is clear
(28 calls in the story; 30 in Django's choice screen, left out with
it). From the exe:

- `builtin_CreateMask` (`00409ce0`) reads the name, priority, x, y and
  picture, and passes the last argument to `00443800`, which, as for
  `CreateStencil`, leaves the flag at +0x9d4 0 unless it is `true`
  (a wildcard then reaches the children).
- `00457e80` sets the mask up: every pixel of its picture whose alpha
  is 0 is made 0 (black and clear), and a 1024×576 buffer (576 row
  pointers) is made for the screen; the draw call is `00416860`.
- `00416860` clears the buffer to 0 (or copies the enclosing mask's
  buffer) and draws the picture into it at its place, zoom and turn
  (per-row texture steps); inside another mask, the two multiply per
  byte (black stays black, white leaves the other).
- The blitters that draw a child under a mask (`00419210` normal,
  `0041a510` added, and their kin with the same tests) skip a pixel
  where the buffer is black, copy it where it is white, else multiply
  the child's pixel by the buffer's, each byte (with the child's
  opacity), before blending.

So a white child shows the mask's picture (ma01_019's `絵Ｍ１/絵白`
over ev104_d: a copy of the event picture that zooms and fades), an
added grey shows as smoke where the mask is light (ma01_021's
`絵マスク１/もや`). The converter treats it as a stencil whose cut
multiplies colour as well as alpha (`Assets::stenciled` with
`colour`; the same cut cache, place and zoom shortcuts as
`CreateStencil`). A colour mask under a normal blend is drawn as the
child's colour times the mask's with alpha times the mask's alpha,
which is the exe's only when the mask is grey (the exe's per-byte
factor is also a per-channel opacity); the story's colour masks are
ev104_d (white child: exact) and 3d村正蜘蛛_正面 (Ｓ/Ｍ).

Checked with numbers in our build of ma01_021 (`--goto` at the もや,
shot with and without the masked sprite): the added smoke follows
`cg/data/effect_01_00_0.png` (correlation 0.90 over 9,216 points; mean
added red 10, 21, 32, 41 where the mask is 16-64, 64-128, 128-192,
192-256, against 0.29 × 187 × mask/255 expected; before, it was a
flat grey over the whole screen). Not compared with the original (no
wine run). Snapshots: Muramasa and EN change (the cut pictures of 28
masks' children); Django's 0.txt does not (its masks are in the
choice screen, left out).

### Processes: one-shots on paper (C2, 2026-10-16, #151)

`convert/one_shot.rs` walks the function at the process's start
(`if`/`while` decided then, assignments run, a 10,000-turn cap: a
process waiting on what the script sets later, `ProcessAFB1`/`2`, is
noted and its state put back), puts each picture's actions on a clock
(`loops.rs` `paper_action`, `Pass::keys_to_end`), and gives it one
loop-2 `spmotion` at the next sync: placed where the process leaves it
at the highest opacity it reaches, keyed from where it was; once
`sound_clock` passes the end it takes its last opacity, or is deleted
(`Delete`, `Request(…, Disused)`). A process run in line (the end roll)
keeps its old path. Dry run: `CreateProcess` notes 1,756 → 236 (the
counters `MyTr_*`/`MyLife_*`/`EnTr_*`/`EnLife_*`, `CP_LockOn*`,
`CP_IHPMove`/`CP_EHPMove`, `md02_023_ComicProcess*`, `MapBGLoop`, …
gone), about 800 new many-key motions; the script's total waits change
by +27 s over the game (a `WaitAction` on a one-shot now waits for it).
Real-time trace, 90 s from `*if0_else` (`--auto-play 0 --real-time
--vars` with the settings' defaults: `--goto` leaves `%2920`-`%2938` at
0, so the story ran at `gamespeed` 10 without them): main 57 loop-2
motions, C2 92 (and 17 more pages); the 35 new ones are the blood-spray
frame flips (`SL_rightdamfade`, `ma00_000`, 13 frames; three sets of 8
in `ma01`): 11 ran out, 3 cut short by the next set's `spmotion` on the
same sprites (after 34-86 of 80-90 ms), 21 removed by the scene's `csp`
after 18-43 ms because auto-play clicked through the 500 ms `print`
before it (a player sees them whole: motions go on during effects).
Every other ending count is the same as main's. Tests:
`a_one_shot_counter_flips_its_digits_on_paper`,
`a_one_shot_fading_out_then_disused_goes_after_its_fade`. `CP_LockOnBGMoveProcess` passes (`SetVertex` allowed).

### Processes: loops behind an `if`, after a lead-in (2026-10-16)

The cockpit meters' `CP_SpeedAuto`, `CP_HighAuto` and `CP_AltAuto`
(703 dry-run notes) test the meter's value, wait in the process for the
move `CP_*Move` started (`WaitAction`), may step once more, then wobble
for ever. `loops.rs` decides an `if` the function ends in when the
process starts (`split_if`, `ends_in_loop`: each branch loops or runs
once). Waits in a loop process (`Wait`, `WaitAction`, a blocking
action) hold only the process: they add up to `process_lead` instead of
holding the script (a process run in line, the end roll, still holds
it), and a process's own time no longer lengthens the script's wait at a
sync. The loop then starts after that lead, in one motion with the move
it waits for (`spmotion` loop 4, CONscripter #48, `MotionEnd::LoopFrom`):
the next sync turns a picture's pending move into the lead-in, places
the picture at its new place and keys it from where it was
(`lead_in_loop`), so neither motion replaces the other. Dry run: 579
lead-in loops, 792 fewer `delay`s; a cockpit run from `*if46_else`
(trace, master engine): of the lead-in loops 114 are replaced by the next
meter change's, 109 removed with the scene, 63 stopped, 8 replaced by
another motion. The test is
`a_loop_after_a_move_has_the_move_as_its_lead_in`.

### Tempo of `Move` and `Zoom` (2026-10-16)

The tempo argument of `Move` and `Zoom` is read (`tempo` in
`nitroplus/convert.rs`): the exe's curves (`apply_tempo`, `00449f40`;
t the progress 0-1) Axl1/2/3 t², t³, t⁴ and Dxl1/2/3 1-(1-t)², ³, ⁴
are `Op::MoveEase` 1 and 2 with exponents 2-4 (the MAGES ones, `spcurve`
11-13 and 31-33); AxlAuto 1-cos(πt/2), DxlAuto sin(πt/2), AxlDxl
½-½cos(πt) and DxlAxl acos(1-2t)/π are new eases 4-7 (`spcurve` 70-73,
33 steps, `conv-cons/src/ease.rs`); any other name (`null` too) is
straight. The curve goes on the `spmotion` key of the move or zoom: a
single blocking move (`SpriteMove`) and the engine motions made at a
sync (`SpriteMotion2`, the non-waiting calls, most of the story's:
Move 631 Dxl1, 576 Dxl2, 362 DxlAuto, 190 AxlDxl; Zoom 618 Dxl2, 385
Dxl1). A move and a zoom of one picture with different tempos before
one sync take the last one's. In the dry run: 615 motion keys on 70-73,
the "argument 4 ignored" notes of `Move` (3,420) and `Zoom` (2,213)
gone. Not compared with the original under wine.

### Render modes (2026-10-16)

`Request(obj, AddRender|MulRender|SubRender|NormalRender)` sets the
object's blend; a picture placed with one is drawn with CONscripter's
`spblend` 1 (added on), 2 (multiplied into what is under it) or 3
(subtracted), times its opacity, keeping its place among the sprites
(IR `Op::SpriteBlend`, shared with 続・殺戮のジャンゴ). A blend set on a
picture already shown changes it in place (`spblend` alone);
`spblend` stays through `lsp`, so a layer a blended picture leaves is
cleared before another picture takes it. Before, every blend was drawn
normal (`lsp2add` only fits pictures over every other sprite).
A movie with `AddRender` (white smoke) is now added on; `SubRender`
movies keep their darkened re-encode (the colour drawn black). In the
dry run: 2,736 `spblend …,1`, 329 `…,2`, 27 `…,3`, 80 `…,0`. Checked
in the engine (CONscripter #34): over #808080, #404040 subtracted gives
65, screen 192, the same zoomed or turned. Not compared with the
original under wine.

### `SetBlur`

The exe's `SetBlur(obj, on, passes, alpha, lag, wait)` (`FUN_00460230`,
`FUN_00460530`, `FUN_00456810`) draws `passes` (1-128) copies under the
picture at its opacity × `alpha`/1000 (1-1000); when a move, zoom or
turn starts they are put on the picture, and copy i shows where the
picture was i × `lag` ms before. A picture that doesn't move shows no
difference (AE 0 under wine). The converter drew a Gaussian blur of
1.5 px per pass and waited `alpha` ms as if it were a time; both went
(1,049 blurred pictures and about 240 waits fewer in `0.txt`). Copies as
extra converter sprites were tried and dropped: the layers are packed
(849-878 scene re-layerings against 592, or 6,374 copies left out).
Instead CONscripter's `sptrail layer,passes,alpha,lag` draws the copies
under the sprite, following its `spmotion` keys `lag` × i ms late; the
converter emits it before each motion of a trailed picture, a blocking
`Move` included (an `spmotion` too since #101): 594 in the story;
`sptrail layer` ends it when `SetBlur` is turned off on screen (28).
The last argument (`wait` in the old notes) is no wait: the exe
(`00454ca4`) draws the copies (`FUN_00456810`) before the picture when
it is `true` and after it otherwise, so the story's `false` (955 of
996 calls) puts them over the picture. CONscripter's `sptrail
…,lag,1` (engine #83) draws them over: 659 of the story's 704 trails.

Finished work, moved out of "Status" so a session reads only the open
state.

**A picture's children (2026-10-16, wine):** a child of a picture is placed
from its parent's corner (a `CreateColor` child at (100, 150) of a parent
at (50, 100) drew at (150, 250)); a window's or a name's child at its own
place. `Compiler::parent_offset` now adds a picture parent's place for
every game (with `Game::relative_children`, 続・殺戮のジャンゴ, a container's
too, as before). The whole-game script doesn't change: the story's
pictures with children (if any) sit at (0, 0) or their children are in windows and names (`絵窓`, `遠３`).

**Wildcards and children (2026-10-16, exe and wine):** a trailing `*` that
matches a window, a name or a camera doesn't reach its children; one
that matches a picture does (the exe's flag at `+0x9d4`; under wine
`Fade("@窓*")` left a window's child, `Fade("@絵*")` faded a picture's).
`Delete` still takes a matched container's children, a locked one's too
(under wine `Delete("*")` removed the standing pictures in the locked
`遠３`), and deleting a window deletes what is in it (`md01_006`'s
`Delete("絵窓")` used to leave its standing picture and background).
About 5,800 lines of the script change, mostly in battle scenes
(`ma04_016vs` 144 hunks, `md05_024vs` 64, `md01_029vs` 46): children of
windows that a wildcard `Fade`/`Move` used to show or hide.

**Title button sounds late (2026-10-16, reported by the user):** the
hover sound (`$TitleOverSE`, `se戦闘_動作_刀構え02.ogg`, our
`selectvoice`) started late. Loading takes 0 ms; the file opens with
125 ms of silence, but it is the original's file copied unchanged, so
the original plays that silence too, and it stays. The delay was the
mixer: CONscripter opened it with a 4096-sample buffer (93 ms at
44.1 kHz, so 140 ms late on average). CONscripter `f33a0c0` uses 1024
(23 ms), 2048 in the web build, which mixes on the main thread.

**Window clip (2026-10-16, reported by the user):** the cockpit's speed and
altitude tapes ran the whole screen height: `CreateWindow(name, prio, x,
y, w, h, ?)` (`CPWS`/`CPWH` at 260,138 and 665,138, 100×300, in
`CockpitSetSpeed`/`CockpitSetHigh`) shows its children only inside its
rectangle, and only the window's place was kept. Now every picture in a
window (or in several, their overlap) is drawn with CONscripter's
`spclip` (`Op::SpriteClip`) at the window's place, which a `Move` of the
window moves; a named place (`Center`, `OutRight`) is read as for a
picture. Checked headless in `ma01_022vs` beside the user's shot: the
tapes show between y 138 and 438 only. Not with
`Game::relative_children` (続・殺戮のジャンゴ: its windows that move in from
`OutRight` aren't checked, so its script is unchanged). The cockpit's
windows don't move (`FrameShake` shakes `@CP_Frame` only); story windows
that do (`絵演窓上`, `窓右`, `絵窓上`/`下`: about 56 `Move`s, 14 over
200-1000 ms) move the clip at once to the end place when the move
starts, not over its time. Pictures under a clipped full-screen picture
(the halves of a split screen) are no longer left out as covered: 50
more `print 1` in the whole game's script, none from `spclip` itself
(it is always given with its `lsp`, inside the transition).

**`SetVertex` (2026-10-16):** `builtin_SetVertex` (`0040ff40` →
`FUN_00447780` → `FUN_00451e10`) sets the point (`left`/`center`/`right`
or px, `top`/`middle`/`bottom` or px, from the picture's top left) that
a `Zoom` or `Rotate` keeps in place; it doesn't move the picture. 続・殺
戮のジャンゴ's rule (`Game::zoom_vertex`) is now every NSS game's, the flag
gone. In Muramasa 1,226 places of `0.txt` change: 1,152 `lsp` places,
36 `spmotion2`, 28 sprites shown, hidden or reordered, 10 `conv_move`
end points. Checked under wine from a patched `md04_006`
(`research/nitroplus/local/patch/vertex*`, shots in `research/nitroplus/
local/shots/vertex/`): `ma03_016`'s background zoomed ×2 about its left
middle (RMSE 7 % against the original, 24 % before), `mc01_023vs`'s
mountain strip `bg202` (1536×6112) turned -90° about the screen's middle
(0.1 % of pixels differ; turned about its own middle it was off screen),
`ma01_022vs`'s sky zoomed ×2 across about its left edge (RMSE 0.9 %,
2.6 % before), and its far-off 村正 (`SetVertex(…, 512, 288)` on a
1400×1000 picture, zoomed to 1 %: now above the screen, as in the
original; tried at 30 %, 41 pixels differ, 12,970 before). The full
build grows by 310 KB (3,858 MB; the pre-scaled zooms' crops), its peak
memory stays 1.39 GB.

**Story `CreateText` (2026-10-16):** the 27 calls the story reaches (殺 害,
〝我は力〟, the screamed lines of `mb03_022`, the date captions, the
two 「暗闇星人……」) are drawn by the converter into a picture
(`system/strip/text_<hash>.png`, `convert/text_object.rs`) in the
last `SetFont`'s style with the `--font`, then they are ordinary picture
objects. An `@` face is Windows' sideways font: the upright letters turned a
quarter counterclockwise along a line the script turns with `Rotate(…,
90)`; brackets, dashes and ASCII lie along the line. `HEAVY` thickens
the letters; `RIGHTDOWN` is a shadow 2 px right and down on the screen
after the turn; `inherit` for y is the screen's middle. A turn goes about their `SetVertex` (`ma01_009`, `mb04_015vs`). Checked under wine from a patched
`md04_006` (the captions, and the four kinds side by side, shots in
`research/nitroplus/local/shots/ctext*/`): same places within about
15 px; the original's letters are heavier and narrower, but wine
substitutes ＭＳ 明朝 (see "Running the original under wine"), so
sizes can't be compared. The box's height (argument 5) clips the
picture: the exe draws into a surface of the box's size (`builtin_CreateText`
makes it from w and h, or from the text's size for `auto`); no story text
reaches its box's height (500), so nothing changes in Muramasa. Not drawn: the choices' texts
(`function_select.nss`, the engine's choice draws them), texts with
markup (none in Muramasa; 続・殺戮のジャンゴ's song in `dja026`), and the
English build's (the port's own captions aren't known yet).

**Auto-play stuck in `mc04_041vs` (2026-10-15, reported by the user):**
the trace's auto-play ended as stuck (20000 waits) in a blocking sprite
move (`conv_move`: `gettimer` and `wait 10` steps): it shortened the
waits but `gettimer` ran in real time. CONscripter `38e45a0`: a wait
auto-play shortens advances the timer by the time skipped; the scene now
plays to its end under auto-play.

**Loop points (2026-10-15, script coverage):** `SetLoopPoint(snd, start,
end)` (ms; `CreateBGMEX`'s `msong04_arrange`, `CreateSEEX`'s four
battle loops) plays, when looped, a copy of the file tagged
`LOOPSTART`/`LOOPEND` (`<name>_loop<start>-<end>.ogg`). SDL_mixer loops
`bgm` by those tags; for `dwaveloop` CONscripter now loops inside the
sound itself (`dc48c0e`, also from a WAV `smpl` loop, so Opus builds
keep it), checked by recording the output (`SDL_AUDIODRIVER=disk`): the
intro once, then the loop. `CreateSEEX`'s sounds were all missing:
`"sound/se/" + "$音楽データ"` names the variable in quotes, now its
value (`Game::string_vars`, Muramasa only); 22 sounds back. Left: a
loop isn't kept through `dwaveload`/`dwaveplay` or a load; 25 sounds
named by variables never set (`$SE矢飛翔_ma04_014vs`, …) are still
missing.

**超速スキップ (2026-10-15, script coverage):** each file's `SkipOut`
test (`$SYSTEM_skip && #SYSTEM_skip_express && $AllRead`, 835 tests) is
a runtime test: the file's read flag (`#conv_read:<file>`, a global set
once the scene has run or been jumped over) and `Op::IfFastSkip` (the
bar's 超速スキップ button sets `%2905` and starts skip; スキップ clears it,
and a test that finds skip off clears it). Read and on, the scene is
jumped over as in the original (`SkipOut`: everything faded out, the
flags after the scene set). Approximations: "read" is "played through
once", not the exe's every line read; `#SYSTEM_skip_absolute` (skip
unread too) isn't offered; skip started with the `s` key or Ctrl right
after a 超速 skip stopped counts as 超速 until the next scene's test.
Not yet tried in a real skip run. Also fixed with it: `PassageTime` of
a sound is its length (`mc01_017vs`'s loop waiting for its song ran
10,000 rounds since `RemainTime` gave the length).

**`SetBacklog` (2026-10-15, script coverage):** the lines the scripts
put in the backlog by hand (83 story calls: lines shown as pictures, the
ASCII art, the crowd's 殺した！/悪鬼！ with each voice; and the choices'
`function_select` entries, the option taken between blank rows) are
`Op::BacklogLine`, CONscripter's `logadd "text"[,"voice"]` (after
0.25.3, `1ddd722`): a page added to the history before the page being
written, its voice played from the backlog's voice button. 1,958 in the
whole game; 続・殺戮のジャンゴ's 75 too.

**Staff roll (2026-10-15, exe review):** each ending (`TheEND`) now
plays its roll. The compiler evaluates `Array`/`AssocArray`/`Count`
(`$EndName[$ENDNumber][0]`: an element is the variable `v[i]`, a key
given to `AssocArray` means its place), `String` (printf) and
`VariableValue` (`#ClearRoll%02d`, a global flag: the first time the
roll can't be skipped, as in the original), and `RemainTime`/
`DurationTime` of a sound give its length. The roll's process
(`ProcessRoll01`) runs in line when it starts (the script waits for it
anyway): the staff pictures (`cg/sys/ed/staff01-02`, 4950 and 6650 px
wide) move across the screen by the engine (`spmotion2`) over the song,
悪鬼編's event thumbnails fade in turn in their frame, and the logo comes
up at the end. Checked headless from `mb04_025vs` (the credits scroll
across in step with the song). Left: the original's 5 s wait runs
beside the roll, ours after it.

**`<VALUE>` in text (2026-10-15, exe review):** `<VALUE name=$var>`
prints the variable; it is now filled in where the value is known when
converting (the airship kitchen's `$材料名前`, `$料理名前`; their `Random`
reaches text, so it is picked when converting: one ingredient always).

**`<FONT>` in story text (2026-10-15, exe review):** `incolor` (the red
違う。 in `mb03_006`, the yellow puzzle rules in `md05_025`) and `size`
(the shouted lines, 24-50 against the window's 20) were dropped; they
become text styles (colour, `!z` percent).

**Shakes die down (2026-10-15, exe review):** `Shake(obj, time, x0, y0,
x1, y1, …)` goes from (x0, y0) to (x1, y1) over its time (the exe's
`shake_frame`); nearly every call ends at 0, a jolt that eases out. The
converter kept the first strength throughout; it now passes both to
`screenshake` (linear: the tempo isn't read).

**Voice switches by class (2026-10-15, exe review):** the switches 村正,
その他男声, その他女声, 首領姉 and コヒカリ muted nothing (about 6,300
lines): the game's setting goes by the `<voice>` tag's `class` (村正's
voices are `村正さん`), the compiler grouped them by `name`. It now
takes `class`, or `name` when there is none.

**`#` flags off the menus' globals (2026-10-15, from the code audit,
in git history, docs/audit-2026-10.md §5):** the first 170 `#` flags ran %800-%969,
and flags 100-112 shared %900-%912 with the menus' globals: seeing
`bg006_小光の家_03b` set the text speed (%907) to 1, and opening the save
screen (its page, %900) unlocked the CG `bg004_鄙びた村b_02`. The first
100 now take %800-%899 and the rest %5200 up (`GLOBAL_ROOM` 100), so a
`gloval.sav` from an older build has some CG and replay unlocks in the
wrong place (the user's choice; local saves are unaffected).

**`#` flags across quit and relaunch (step 11.5, 2026-10-15):** every
`#` name the story both sets and tests is a global (%800-%899, then
%5200 up; `;$S1024,576,value800,V8192` + `globalon`), e.g.
`#悪鬼編終了 = true` in `md06_036.nss` is a `mov %54xx,1` there. Every
way out (the title's 終了する, Esc → ゲームを終了, the quick bar, the
story's end) is `end`, which writes `gloval.sav`, as closing the window
does; a save writes it too. Checked headless with a small script
(`;$S1024,576,value800,V8192`, `globalon`, %800/%969/%5200/%8191 set,
then `end` or SIGTERM in a click wait, relaunched with the same
`--save-dir`: all read back). Web: the engine writes the globals at a
wait when they changed and syncs IndexedDB; that was only at click
waits, so auto mode lost them when the tab closed. CONscripter now does
it at any wait (`COMMANDS.md`). Checked in headless Chromium
(Playwright): a value added in a loop of timed waits is read back after
a reload.

**The route runs, round 3 (2026-10-14):**

(Round 3, 2026-10-14: the build with every fix below up to the `[text0081]`
marker, engine 0.19.1, all ten paths side by side with `--all`; the
times are with ten at once. Rounds 1 and 2, the five endings on the
builds before, passed too. Round 3's checker report matches round 2's,
with no `error` or `missing`; the only new findings are `slow` waits in
`ma05_019` (7.8 s where the original takes about 4 s): since the alias
fix its non-blocking 2 s `Shake` really shakes, and we play it blocking
before the next fade.) The same runs on Xvfb in skip mode took 40-56 minutes. On CONscripter
0.18.0, run 9 aborted 508 files in with `malloc(): unaligned tcache
chunk detected` (heap corruption, noticed in `malloc` under `lsp` →
`loadImage`); on 0.19.0 it passes. A likely cause, fixed in between:
CONscripter's `DirectReader` copied a file name's extension over its
own buffer with an overlapping `strcpy` (engine `1d51abc`; undefined
behaviour on file reads). An AddressSanitizer build of 0.19.0 played
the whole of run 9 (510 files, 34 min, `ASAN_OPTIONS=detect_leaks=0`)
with no error, so no bad memory access is left on that route.

Fixed on the way (each found by the runs, all in the current build):
- A `?` in horizontal text: NScripter reads it as an array reference
  and exits with 255 (the message goes to `stderr.txt`). Every route
  passed one, in `ma03_027vs`: the NSS `<?>` tag (93 uses), passed
  through as text. Text lines now write `?` full-width, `<?>` is
  dropped, and lint reports a `?` in text.
- `<?>` at the end of a line joins the next text line to it, after the
  code between them (a face change, a typewriter's key sound). It was
  taken as a line break, so the typewriter captions of `ma01_026`
  (《興隆四一年一〇月一二日》) came out one character per column, and 30
  lines of dialogue were split in two. Found by the trace checker's
  `text` rule ("the text runs past the window") on the first route run.
- A `/* … */` block inside a text block was printed as text (`／＊`,
  three copies of 「暗闇星人……」, `＊／`, in 3 files). The NSS parser now
  leaves it out like a `//` line.
- Lines spoken in unison (66 `CreateVOICE` calls in 32 files) had no
  voice: the helper plays them only when the setting "other voices"
  (`#SYSTEM_voice_enable_another`) is on, which the compiler didn't
  know. It now starts with the value a first start gives it (on), and
  the voices play together on their own channels. Both found by the
  checker's `voice` rule.
- `Play` on a looping sound that was still playing started a second
  copy: `md04_048vs` calls `MusicStart` on its loop eight times to change
  its volume and speed, and `茶々丸の世界_L` ended up on eight channels at
  once (28 loop starts in the game). Found by the checker's `sound` rule.
- `WaitAction(obj)` waited for every action still running, not only
  `obj`'s: in `mb02_030vs`, `CP_EHPChange` waits on its own process
  after a non-blocking 15 s zoom, and the player sat through a 15 s
  fade (five such waits of 5 s or more). Found by the checker's
  `duration` rule.
- Branches of a runtime `if` that ended with different pictures left
  sprites up: the code after them follows the first branch's sprite
  layers, so a sprite another branch put elsewhere was never cleared.
  After the favour screen (`judgment_of_count`, 村正's icon hidden only
  while `$Muramasa_Flag` is 0) its full-screen background stayed in
  front of the story for 150 pages. The next sync now removes what any
  branch left and draws the merged pictures anew. Cost: after the merge
  every branch shows the first branch's pictures (the black box over
  村正's icon also shows when she is known, until the screen closes).
  Found by the checker's `linger` rule.
- A wildcard (`@StMR*`) only matched object names, not aliases: the
  character helpers create `遠４/StMR01` with the alias `StMR01`, so
  `Move("@StMR*", …)`, fades, shakes and deletes on them did nothing
  (`ma02_012`: ふき stayed 600 px off screen instead of sliding in).
  About 11,700 lines of the output changed (44 more screen shakes, real
  moves in place of 177 waits, characters deleted when the script
  deletes them). Found by the checker's `offscreen` rule.
- Non-blocking actions were cross-faded over their whole time at the
  next sync: a 270 s background scroll (`ma04_013vs`) blocked the
  player for 270 s, 51 cross-fades were over a minute. Now at most a
  `Wait`'s time, or 1 s where the script doesn't wait.
- The full build ran out of memory (7 GB) on 60x zooms; now 1.2 GB.

Fixed after the route runs (2026-10-14), from screenshots of the first
page each punctuation mark or symbol shows on (`--shot-pages`):
- Vertical text drew the marks CONscripter turns for it (《》「」『』（）
  ― ～ …) high and to one side of the column: 《 stuck out to the
  left. The engine now turns them about the centre of their cell
  (CONscripter 3c7c957), so ︽ sits on the text it opens.
- Vertical text sat 8 px left of the original's columns (page 5 of the
  prologue, filmed under wine): `rubyon` moves vertical lines left by
  the line gap, which the converter undid for choice windows only. Every
  vertical window now starts a line gap further right, and the columns
  match the original to the pixel.
- `[text0081]〝卵〟を…` (`md05_003`, the only marker with text after it
  on its line) printed the marker as text, ［ｔｅｘｔ００８１］. The NSS
  parser now reads it as a marker. Needs a full build to show.
- English lost most of its spaces: ONScripter skipped the space before
  each pair of half-width characters, so “Tell me the tales that to me
  were so dear” (`mb04_008vs`, 33 such lines) showed as “Tellmethe tales
  thattomeweresodear”. CONscripter now keeps a space after other text,
  in sprite text too (2179dbd).
- Built-in and NSS function names matched ignoring case; the exe
  matches them exactly (`dispatch_builtin`, `call_nss_function`,
  `research/nitroplus/builtins.md`, "In the exe") and skips a call to a
  name it doesn't know. So `Waitkey(500)` (`md04_004`, `md06_035`) was
  a half-second wait and `SL_Rightdown(…)` (`ma00_000`) ran
  `SL_rightdown`; the original does neither. Names are exact now, and
  the build's list counts the skipped calls (`skipped: no function`,
  13 names, 179 calls, `Boxframe` 153 of them).
- The screen kept shaking through the whole prologue, from the first
  `Shake` (`ma00_000`, 1 s) on: `screenshake` keeps its last size until
  its slot is cleared, and the converter never cleared it. An NSS
  `Shake` lasts its time, or until its picture is deleted (shakes of
  500 s or 36 minutes end that way), and 0 ms or 0 px stops it. The
  converter now gives each shake its time as `screenshake`'s new `end`
  (CONscripter 81d0b80) and clears the slot when the shaken picture
  goes or a stopping `Shake` comes. The checker missed it: its
  `overlay` rule looked for an effect left on after `bg` changed, and
  this game draws everything as sprites over `bg black`. It now also
  reports a shake with no end left on for 30 pages (`--shake`), and
  the trace counts the waits auto-play skips toward a shake's end. On
  the prologue (419 pages) the old build gives two `overlay` findings,
  the new one none.

**System screens done (Status, next item 4, steps 1-6):**

1. Done: button strips joined from the `000`-`003` part files at
   convert time (`Assets::strip`: normal, hover, disabled, pressed);
   the yes/no questions as the game's pictures (`ConfirmDialog`
   `pictures`, `backdrop`, `pressed`, `fade_ms`; built in
   `convert/muramasa/screens.rs`), with the pressed state from
   CONscripter's `spbtn num,no,pressed_cell` (625c598). Beside the
   original (quit question, normal, both hovers, 拒否 pressed): the
   same pixels, offset 0,0. The 150 ms sideways unroll is done (step
   7). Left for later: the 記録をしました notice (with the save
   screen), 選択肢まで戻りますか and 設定を初期状態に戻す (with their
   screens).
2. Done (static): the title at rest, the background, logo and
   copyright composed into one picture, the six buttons with their
   hover (`002`) and pressed (`003`) pictures, the hover sound (刀構え02)
   and each button's click sound, the title music, Start's 3 s music
   fade, 1 s fade to black and 2 s wait, 終了する quitting with no
   question, しおりから dimmed to half while there is no quick save
   (`TitleScreen` `pressed`, `hover_se`, `bgm`, `start_*`,
   `exit_without_asking`, `TitleAction::Continue`; CONscripter's
   `selectvoice` now works outside the define section, 325c9e1).
   Beside the original (normal, hover, pressed): mean difference
   0.1-3.5 levels, from the original's fire. The fire's glow is done
   (step 7); left: the opening (step 7); 特別付加目録 waits for the
   extras (step 8).
3. Done: the right-click menu (a new `CommandMenu`, in
   `conv-cons/src/command_menu.rs`) and the quick bar (`ButtonBar`
   with a `base` over the buttons, a `BarSlide` and the pressed cell),
   built in `convert/muramasa/screens.rs`. The menu: sixteen rows over
   メニュー黒バック, opened by a right-click or M at a click wait; the
   backdrop fades in over 100 ms, then the rows appear 10 ms apart from
   the top; closing, they go from the bottom, then the backdrop fades
   out (filmed: the rows go first, though `sys_menu.nss` starts the
   fade with them). Filmed at 60 fps beside the original, opening and
   closing land within a frame. It closes on キャンセル, M, a
   right-click, or after skip, 超速, auto or hide; save, load, log,
   settings, 好感度記録, title and quit come back to it. The game's
   shortcut letters work at any click wait (CONscripter's `getkey`,
   53555cf). The quick bar waits 32 px off the right edge and slides
   in over 200 ms (cubic ease-out, `Dxl2`) when the pointer passes
   x 968, out when it goes back: an invisible `btn` hotspot with
   `getmouseover` ends the click wait when the pointer crosses the
   edge, so no engine change was needed. While it is out a click that
   misses its buttons doesn't advance. Its base carries the words and
   lies over the buttons, which are colour washes behind it. Quick
   save (slot 890, しおり sound) and quick load (300 ms to black) ask
   nothing. 選択肢に戻る: every choice saves to slot 891 first; the item
   is greyed while there is none, asks 選択肢まで戻りますか, and loads it
   after 300 ms to black; a new game and every other load delete it
   (`loadgosub`), as the original does with 9999. Beside the original
   (no quick save, セーブ under the pointer): mean difference 0.36
   levels over the screen, 0.88 in the menu, offset 0,0; quick bar at
   rest 1.8, hovered 1.0-1.9, pressed 1.0, offset 0,0; the slide's
   pixel steps match frame for frame (ours starts about 130 ms sooner:
   the original polls the pointer in a process). Differences left:
   - The menu has no pressed picture: filmed at 30 fps, the original
     keeps the hover picture while the mouse button is held (its
     `003` files are never shown), so the menu does the same.
   - The bar moves only at click waits; the original's process also
     slides it while text is being drawn.
   - 超速スキップ skips like スキップ (pages shown): CONscripter has no
     skip that hides them.
   - Esc opens the menu (it is ONScripter's right-click); in the
     original it asks ゲームを終了しますか. Space reads on and Ctrl
     skips while held, as in ONScripter; in the original Space hides
     the window.
   - The bar is put away while the menu is open (the original's,
     drawn above everything, stays if it was out).
   - 好感度記録 (H) runs `sys_hmode.nss` compiled once as a subroutine
     (`Game::subroutines`, `conv_favour`) between `pushscreen` and
     `popscreen`. Its `judgment_of_count`, the favour screen, is now a
     port (`convert/muramasa/favour.rs`), in the story too, where only
     its background used to show: the NSS draws it with loops over
     arrays and counters known only at run time, which the compiler
     can't follow (after a runtime `if` it shows the first branch's
     pictures). The port is a new generic `Op::PictureScreen`
     (`conv-cons/src/picture_screen.rs`): pictures in the menus'
     sprites over the scene and its text window, each with its runtime
     test (the icon struck out and the column covered for a dead
     heroine, 村正's black box while her counter is 0, one gauge column
     per count), faded in over 300 ms, closed by a click. Beside the
     original at the first page (H): mean difference 5.0 levels, offset
     0,0; the panel is see-through, and the text under it differs
     (wine's substitute font), and our dimmed surround is a little
     darker. Left out from the story's visit: the new gauges blinking in
     and a heroine's icon being struck out with its sound the first
     time she is dead.
   Fixed on the way: the title's しおりから used `load`, which isn't a
   command (NScripter's is `loadgame`), so it stopped the game.
4. Done: the save and load screens (a new `SaveGrid`, emitted by
   `conv-cons/src/save_grid_screen.rs`, Muramasa's values in
   `convert/muramasa/screens.rs` from `sys_save.nss`/`sys_load.nss`).
   Five pages of twenty thumbnails over each page's own picture
   (数字01-20背景save … 数字80-100背景load), four columns from the right,
   under サムネールワク; 前頁/次頁 go round, 削除 toggles delete mode
   (削除ボタン-003 over it), 退出, a right-click or the screen's key
   (S, L) leave; it cross-fades in and out over 300 ms. The slot under
   the pointer gets the hover frame and its details: the date, the
   play time, the chapter (`$TITLE_NOW`, now `Op::Chapter`) and the
   page's text down in columns from the right (CONscripter's
   `vstrsp`, 3034b55), and a larger thumbnail. Each slot is an
   invisible `exbtn` whose hover command shows its details. A click
   saves (asking 記録を上書きして良いか over a full slot, then
   記録をしました over the dimmed page until a click), loads (asking)
   or deletes (asking). The thumbnail is the scene without the text
   window, as in the original. The play time is CONscripter's
   (`getplaytime`/`setplaytime`, 8b92e5b): the engine counts it, a
   save keeps it, a load puts it back, a new game starts at 0. The
   label of a save holds date|time|chapter|text (`savegame2`,
   `getsavestr`); the text comes from `getlog`, with the page's line
   breaks as the original keeps them. Beside the original under wine
   (save screen, empty slot hovered: mean difference 0.91 levels,
   offset 0,0; a filled slot hovered at the first page: 1.93, with no
   pixel different in the grid, thumbnails included; the rest is the
   text). Differences left:
   - The texts are drawn in ipam, the original in MS 明朝, which wine
     replaces with a narrower font sitting 2-10 px lower: our date
     and play time are wider, and a column of the last line holds
     20 characters where wine fits 22.
   - The converter breaks a page's lines to fit the window, and those
     breaks show in the last line; the original's `$SYSTEM_last_text`
     has only the source's.
   - No transitions inside the screen (the details appear at once;
     the original fades them).
5. Done: the backlog (a new `LineLog`, in
   `conv-cons/src/line_log_screen.rs`; Muramasa's values in
   `convert/muramasa/screens.rs`, `line_log()`): sixteen rows of 26
   characters over バックログ背景, the history laid out as rows by
   CONscripter (`logrows`, `getlogrow`, 8a7a34e: each page's lines cut
   at 26, ruby as its base text, a voiced page's voice on its first
   row). The speaker icon (x 102) replays the voice; the wheel and
   逆/進 scroll a row, wheel-down at the newest row, 退出 and a
   right-click close; the knob (x 860, y 50 oldest to 461 newest)
   can be dragged. It fades in and out over 300 ms. Wheel-up at a
   click wait and the menu's ログ表示 open it. The history used to end
   at the last window change: NScripter's `setwindow` flushes it, and
   the converter re-sets the window for each text block. Every game's
   window changes are now `setwindow3` (the same without the flush;
   Saihate's opening trace has the same pages either way). Beside the
   original (wine, the same pages): at the newest row the icons, knob
   and panel are the same pixels, text placed within 2 px (the font
   differs). Filmed at 60 fps (`film/backlog-wheel-4up4down.mkv`), the
   original moves a row a notch in two 15 px steps about 33 ms apart;
   ours moves the row at once.

   Done: the click-wait icon (`Script::wait_icon`, a new `WaitIcon`;
   `TextWindow::wait_icon_at`). The flame's 32 pictures
   (`改行改頁アイコン`, System.ini `icon_line`/`icon_page`) joined into a
   strip, looped, at each window's `$SYSTEM_position_{x,y}_text_icon`
   (read when the converter runs the window's `TypeBegin*` helper: 452,555
   for the usual vertical box, 194 and 706 for others). The bar's wait
   routine shows it as a sprite in the bar's range (over the text window)
   and removes it when the wait ends, so it doesn't show while text types.
   Beside the original: the same place, the frames fit the original's
   pictures at offset 0,0. Pictures 1-19 are the flame burning, 20-32 it
   withering to smoke; watched in the original on a real display (wine
   under Xvfb redraws the waiting screen only about once a second), it
   burns about a second, withers about as long and lights again after a
   pause: ours shows 1-19 for 53 ms each, 20-32 for 77 ms, then waits
   500 ms (the pause is a guess), a 2.5 s cycle (each picture's time is
   `WaitIcon::frame_ms`). Not done: the auto-mode icon (自動送りアイコン,
   15 pictures).
6. Done: the settings screen (a new
   `PictureSettings`, in `conv-cons/src/settings_screen.rs`; Muramasa's
   values in `convert/muramasa/screens.rs`, `settings()`, from
   `sys_config.nss`). コンフィグ over the screen, the controls laid over
   it: six vertical sliders (voice, effects and music volume, auto wait,
   text speed, game speed), nine 入/切 pairs (questions asked, a click
   stops movies, auto and skip stop at a choice, skip unread text, the
   voice plays on after a click, auto waits for the voice, music, effects
   and voices heard), 承認 (asks 設定を初期状態に戻す, a new
   `Switch::ConfirmReset`, then puts the defaults back), 実験 (a line of
   景明's), the version line, 退出, right-click or C to leave; it fades in
   and out over 300 ms and a pair's choice cross-fades over 100 ms. Each
   control's resting picture is erased in the original (the picture shows
   it): ours are strips with an empty first cell (`Assets::strip` takes
   `""`), the `002` under the pointer; a slider's track is an invisible
   `exbtn` that shows its knob's hover picture, and a press there drags
   the knob. The display options (left column) are drawn as a first start
   leaves them, without buttons. Values are globals %2920-%2941 (a
   slider's position 0-1000, a pair's 0/1), with the first start's
   defaults (the knobs found in the original's shot: voice and effects
   800, music 600, auto wait 100, text speed 200, game speed 3), applied
   at startup and as they change by CONscripter's new player settings
   (ddb3686: `soundmute`, `clickvoicestop`, `autovoicewait`, `selectkeep`,
   `moviestop`, `gamespeed`; volumes with `bgmvol`/`sevol`/`voicevol`,
   skipping with `kidokumode`). The game speed is the exe's table
   (`FUN_0047c250`, used by `Wait` and every action: time ÷ 0.5, 0.75,
   1 (3), 1.5, 2, 3, 4, 5.5, 8, 12, 16, 32); the click wait sets 100 and
   puts the story's back as it ends, so menus run at normal speed, as in
   the original. Volumes follow the exe's curve approximately
   ((p²/50 + p)/3 for 100·(v/1000)^1.661); the auto wait is the exe's
   per-character wait (0-200 ms a character); text speed is ours (0-100 ms
   a character, 20 at the default, unmeasured). Beside the original
   (wine, the first page): mean difference 1.6 levels, every pixel within
   40 levels but the version line (the font); 入 hovered matches.
   The voice window (別窓を開く, a new `VoiceWindow`): ボイスセレクトウィンドウ背景
   over black at 700/1000, each character's picture once the story has set
   its `#voice_on_<name>` (55 flags, and `#一章クリア`, declared by the
   game's `flags()`: the story sets them, only `sys_config` reads them),
   with ボイスチェック on its box while its voices play; clicking the box
   mutes or unmutes every voice of its class (`Story::voice_classes`, the
   compiler's character numbers, passed to `menus()` with the flags; the
   switches are the `%970+c` that `dwave` reads). Until 一章クリア it lists
   fourteen characters with their `1章_` pictures, after it all 55, as
   `ConfigVoiceArray` does. Classes of no entry (高官, 皇路, …) always
   play, as `#SYSTEM_voice_enable_another` is never turned off. Beside the
   original at the first page (その他男声 and その他女声 heard): mean
   difference 0.57 levels, every pixel within 40 but the version line.
   Left: the wheel over a slider.

**Extras, step 8 (2026-10-15):**

1. Done: the menu (a new `PictureExtras`, `conv-cons/src/
   picture_extras.rs`; Muramasa's values in `convert/muramasa/
   extras.rs`): MENU背景画像 and five buttons with their hover and
   pressed pictures; 画像, 回想 and 音楽 at 200/1000 and not clickable
   until `#ClearG` (set at any route's end roll); the hover sound
   (鎧_装着01), 装着06 on opening a mode, 剣戟弾く01 on leaving by
   退出 or a right-click; 500 ms cross-fades from and back to the
   title, whose music goes on (`TitleAction::Extras`; back from the
   extras the title skips `bgm`). The `#` flags past %969 now go on
   at %5200 (the gallery's would have run into the voice switches).
2. Done: the gallery (a new `PictureGallery`, `conv-cons/src/
   picture_gallery.rs`; the slots read from `extra_gallery.nss`'s
   `Cg`, its 805 `#<cg>` flags declared by `flags()`): the three
   categories (the open one pressed), 前頁/次頁 going round, 20 slots a
   page down four columns from the right, 空白アイコン, the first CG
   seen's thumbnail at half while some are unseen, 選択カーソル under
   the pointer; the viewer fades each CG in over 500 ms (未見 for an
   unseen one, black between two unseen), a picture larger than the
   screen scrolls 12 ms a pixel on a click (a click pauses, the next
   goes on, a right-click jumps to the end; `Vr`/`Hr` from the far
   end), a click past the last or a right-click closes it over 300 ms.
   Beside the original (2D, first page): mean difference 2.7 levels
   over the grid. Found on the way: with `usewheel` a timed `btnwait`
   returns -5, not -2, when the time runs out.
3. Done: the sound room (a new `SoundRoom`, `conv-cons/src/
   picture_sound.rs`; the entries read from `extra_sound.nss`'s
   `Sound`): 37 BGM, five songs (`000` picture until `#msong0N`), three
   movies (空白アイコン until `#mvOP02`) at the script's places; a
   track plays looping and stays the extras' music across games
   (global %2961: the title's at first, opened with the extras, the
   title's music back on leaving if it differs), the playing one
   clicked again fades out over 2 s; the ten-picture A-icon turns
   26 px left of the track playing, 340 ms a picture (filmed at 100
   frames a second with `ffmpeg -f x11grab -framerate 100 -video_size
   26x24 -i :N+482,282 -f framemd5`: a change every 340 ms, steady
   over 10 s, with the game at 17 % of a core); a movie fades the music
   and the screen to black over 1 s, plays (the settings' 動画 option
   decides whether a click stops it), the music comes back over 1 s
   and the black fades over 300 ms. Beside the original (the title's
   track playing): mean difference 1.3 levels, the icon's picture
   aside. Found on the way: `:c;` draws a PNG's transparent pixels;
   the extras draw PNGs with `:a;` and JPEGs with `:c;`
   (`picture_extras::pic`); and a duplicate label (`conv_ps_draw`,
   the settings') sent the room to the settings' drawing.
4. Done: the recollection (a new `PictureReplay`, `conv-cons/src/
   picture_replay.rs`; the scenes read from `extra_recollection.nss`'s
   `Reco`): the ten scenes in the gallery's grid, 空白アイコン until the
   scene's file has been read (`#ma01_020`, set at the file's end), its
   CG's thumbnail after. Each scene is its file's `_MAIN` compiled once
   more as a subroutine with `$PLACE_reco` true (a new `Game::replays`),
   which takes the files' own replay branches. A click fades the music
   out over 1 s; at the end the music fades over 2 s, everything (the
   text window too) is cleared, and the list fades back in with the
   extras' music. Played through `ma01_020` and back, then out to the
   title. During a replay, as in the original, save, load, しおり
   (quick save and load) and 選択肢に戻る are greyed in the menu and the
   quick bar (`BarScreens::picture_replay`, `menu::NO_SAVE`), and
   タイトルに戻る (asked, then `reset`) comes back to the list (global
   %2962, read first thing by the title). The list stays held over
   the scene's first picture, a freeze frame in screen-effect slot 4
   (`PictureReplay::hold`), until the scene's `RecoIn` fades it out
   over 500 ms and waits 1 s, as the original's `RecoScreen`.
5. Done: the 3D viewer, 劒冑回転目録 (a new `Turntable`, `conv-cons/src/
   picture_turntable.rs`; the entries read from `extra_dimension.nss`'s
   `Dx`): 46 entries, 24 name plates a page in two rows of twelve from
   the right, NO until the entry's `#av_` flag is set (九〇式指揮官騎
   and 九四式指揮官騎 open from the start, 九〇式 chosen first). The
   chosen plate is drawn pressed, its movie (`ムービー/3d/ex_NNN.ngs`,
   400×400, about 5 s) loops at 80,28, played by the engine as a sprite
   (`lsp …,":c/1,0,0;movie\3d_ex_NNN_mute.mp4"`, since 2026-10-16 at the
   user's choice; it was frames at 15 a second, 98 MB of JPEG strips in
   the archives, now 20 MB of loose H.264; CONscripter for Windows has no
   sprite movies, so there it shows no movie), its large
   name plate at 480,28 and its details at 60,436, the （開放） ones
   once `#aw_` is set. 前頁/次頁 turn the page round, 退出 or a
   right-click goes back to the menu; 500 ms in, 300 ms between pages
   and entries. The page and the entry are globals (%2963, %2964), as
   the original's `#DxPage`/`#SelectDxNumber`. Checked with a shot
   beside the original's (`--goto conv_pextras_turntable` with flags
   set). Movie paths with a folder (`dx/3d/…`) are found now (only the
   file name was looked up). Unchecked: the movie's speed beside the
   original (wine draws no movie).
6. Done: back to the title (`0_boot.nss` `TitleFade`, the
   `$エキストラタイトル` branch): the extras cut to white with
   se戦闘_攻撃_刀衝突01 (the click's 剣戟弾く01 on a channel of its own,
   so the clash doesn't cut it), and the title comes out of the white
   over 1 s with the blurred background and logo spreading away: the
   last second of the return opening, rendered from 2 s as
   `title_extras2` (`opening::after_extras`, a `TitleOpening` with
   `after_extras`). The music goes on (the title's, if the sound room
   chose another).
7. Done: the title's ライナー画像 button (`0_boot.nss`, `書記`): at
   (60, 524) once `#悪鬼編終了` is set, it opens Nitroplus's
   staff-comment page (`System("OPEN:http://www.nitroplus.co.jp/secret/
   muramasamune")`, still online in 2026) in the browser, out of full
   screen. Ours: `TitleAction::Link` and CONscripter's new `openurl`
   (a new tab in the web build).

**Motion and movies (Status, system screens step 7), done:**

- The yes/no questions (`sys_close.nss` and the others alike): the
  question and its buttons are zoomed sideways from nothing to
  whole in 150 ms (`Dxl1`) about the middle while they and the black
  fade in; closing, back to nothing in 150 ms while all fade out in
  200 ms (filmed ×10 slowed: the text is squeezed, not uncovered).
  Ours (`ConfirmDialog::unroll`, `Unroll { x: 512, close_ms: 200 }`):
  a frame about every 16 ms, timed by `waittimer`, each sprite
  `amsp`'d toward the line x 512 and `spzoom`'d sideways with its
  alpha, and the backdrop faded. The original's buttons turn about
  their own vertices (515 and 499), ours about 512: at most 13 px
  apart, for 150 ms. Filmed at 60 fps: five frames in, three out,
  as the original.
- The openings (`TitleFade`): at launch (`!$Logo`, 11.2 s) 善悪相殺
  drifts in, 村正鎮座 zooms out of the dark, two bursts of blood
  sprays (multiplied, swinging in about a corner), drops and red
  (added), the fire overlaid, a white flash, the title; on coming
  back to the title (3 s) the logo grows out of the dark, the
  background drops in from above, a white flash, a blurred glow of
  the background and logo spreading away. CONscripter can't blend
  sprites so, so the converter draws both frame by frame from the
  script's timeline (`convert/muramasa/opening.rs`: every `Fade`,
  `Zoom`, `Rotate` about its `SetVertex`, `Move`, the tempos,
  `MoveFRP`'s random Bézier shake from a fixed seed, `SetBlur` and
  `SetShade` as blurs; `MiddleWave` as a fade) into silent H.264
  movies (`movie/title_launch2.mp4`, `title_return2.mp4`) that end
  on the title at rest, buttons included (しおりから at half). The
  title (`TitleScreen::openings`) starts the music, the sound
  effect at the flash (se特殊_雰囲気_鎧登場演02, made with silence
  before it), plays the movie with `mpegplay "f",1,1`, and puts the
  sprites up at once; the glow then starts where the original's
  process is (`TitleOpening::glow_keys`). The global %2960 tells the
  launch (cleared in the define section; `reset` keeps it). In
  CONscripter the hand-over differs by 1.5 levels (the player drops
  the last frames: the last picture is held 200 ms). Beside the
  original's launch under wine (a shot about every 0.8 s, wine
  redraws slowly) the same pictures come in the same order. The
  launch picture is picked as `TitleSet` does (the user, 2026-10-15):
  the first launch after clearing 悪鬼編 shows 真生悪鬼ＥＤ, else
  after 魔王編 (悪鬼編 not cleared) 村正鎮座, after 英雄編 最終正宗,
  after 復讐編 香奈枝の凶眼, each once (`#…通過`, declared by
  `flags()` with the `#…終了` flags); otherwise one at random among
  the cleared routes' and 村正鎮座. Four movies
  (`title_launch2_ev267`, `_ev501`, `_ev165`, `_ev004`), chosen by
  `TitleOpening::when`/`sets`/`random` (NScripter `rnd` over the pool).
  Checked: with `#英雄編終了` set the launch plays 最終正宗's. The
  logos and the warning before it (`TitleLogo`) aren't converted.
- The fire: `TitleSet` puts `ef046_炎a.jpg` (1920×1200) over the
  title at (0,0), zoomed 2× about its centre, drawn in overlay
  (`OverlayRender`, darkening this dark picture), and `CreateTPO1`'s
  process fades it from 0 to 300 over 600 ms, holds until 1100 ms,
  back to 0 by 1700 ms, holds until 2200 ms, for ever. Ours: the
  converter draws the title with the fire overlaid at 300
  (`Assets::overlaid`) and CONscripter's new `spalpha` (0ae2e87)
  fades that picture in and out over the title with the same keys
  (`TitleScreen::glow`). Beside the original under wine: the left
  edge's red swings by about 10 levels in both (50-60 there,
  45-55 ours; wine's picture is 5 levels lighter throughout). Where
  the zoomed fire sits can't be told from shots (it is a soft
  picture and moves the screen by 3 levels at most).
and the movies (added by the user, 2026-10-15; done 2026-10-15).
They are MPEG-1 program streams (`.ngs`, 1024×576, MP2 sound) in
the disc's `ムービー/` folder, beside `<game>`, not in the archives
(33 files, 353 MB, and 46 more in `ムービー/3d/` for the extras); the
scripts name them `dx/<name>.ngs`. Each is encoded once to H.264 MP4
in `movie/` (ASCII names: the file's own, or `mv_` and a hash of a
Japanese one), which every CONscripter build plays (the web build in
a `<video>` element, MPEG-1 being unplayable there).
- `MoviePlay(file, skippable)` (full screen, the script waits:
  `mvOP` at the end of chapter 1 in `ma01_026`, `mvOP02` in
  `md01_018`, `mv剣戟_チャキーンチャキキーン` in `mb01_006`) is
  `Op::Movie` (`bgmstop` + `mpegplay`) for a movie with sound, and
  a new `Op::SceneMovie` for a silent one: `mpegplay "f",1,1`, a
  CONscripter addition (b0de6a0) that leaves the music playing.
  `moviestop` (the settings' 動画 pair) decides whether a click
  stops it. `mv_smg_PV` (the title's demo, every 35 s in
  `TitleProcess`) is not on the disc (only `mvPV.ngs`): it is left
  out, and the title's demo loop isn't converted.
- `CreateMovie(name, priority, x, y, loop, alpha, file)` is a movie
  object (`MovieObject`), as the user chose (2026-10-15: frames
  made by the converter, no engine movie sprite):
  - A cut that fills the screen, plays once, and that the script
    waits for (`WaitAction`/`WaitPlay` on it, or a `Wait`/`WaitKey`
    of at least half its remaining time, which the scripts take
    from `RemainTime`) plays as a movie with the music going on
    (`Op::SceneMovie`; its own sound left out, the script plays the
    `se特殊_mv用_*` effect beside it), then shows its last frame. The
    last frame is loaded before `mpegplay` (opaque, though
    `MovieSEStart2` fades the cut in only after its `WaitAction`) and
    shown with `print 1` right after it: the engine redraws the screen
    as the movie ends, and with the frame loaded after it the scene
    before the cut flashed and faded back (2026-10-16; the trace
    checker's `movie` rule no longer fires on the prologue's cut). The
    time played counts toward the script's following waits for it.
    This is `MovieSESet` + `MovieSEStart` (28 calls) and the single
    cuts with `WaitAction` (5) or `RemainTime` (2).
  - Any other plays as a sprite movie (since 55a8b78: `lsp` of an
    MP4, one cell, looped or stopping on its last frame, zoomed with
    `spzoom`; it was a strip of frames at 15 a second): `mvk100` (23 uses, a 9 s loop of smoke, top half
    colour and bottom half opacity, zoomed 3×5 and more; `SubRender`
    draws it black, `AddRender` as it is), and `mv剣戟_チャキーンチャキキーン`
    looping under the text (`mb02_023vs`), a 300 ms cut in `mb03_019`; checked headless (2026-10-16): the `lsp` of
    `mb02_023vs`'s cut plays (two screenshots 300 ms apart differ), but
    auto-play doesn't play it (its trace shots of those pages are
    black), so the trace can't show these. Paused (`Request(…, Pause)`
    after 100-300 ms, `md01_006`, `mb03_019`) it is that one frame.
  - Left as they are: a `Zoom` over time of the smoke jumps to its
    end (as other zooms over time, a cross-fade), `Rotate` (the
    smoke flipped) is ignored.
- Engine fixes found on the way (f8d4cf4): `spzoom` on an animated
  sprite showed only its first cell, and a screen update partly off
  the screen was dropped.

**Sound volume (2026-10-15):** every `SetVolume` is converted (it was
ignored unless it went to 0, so sounds faded to 1 kept playing at full
volume). The NSS volume becomes a percent of the player's setting by the
exe's curve, 100·(volume/1000)^1.661 (`percent` in `nitroplus/convert.rs`;
500 is 32 %, 1 is 0 %: silent but still playing, as in the original). A
sound starts at its volume (`Op::SoundVolume` right before `SePlay` or
`BgmPlay`, emitted as `chvolfade`/`bgmvolfade` for the channel it will
take; in a script that sets any, every other sound starts at 100), a
sound effect's fade-in is `SeFadeIn` (`dwavefadein`), a change while it
plays is a `SoundVolume` ramp, and reaching 0 stops it: `SeFadeOut`
(`dwavefadeout`) for a sound effect, `BgmStop` with its fade for the
music. The engine ramps linearly in percent, the original linearly in
its volume number (a fade sounds a little different midway). A
240-second auto-play gave the same trace-check findings as before.

**The figure in `ma01_002` (2026-10-15):** checked beside the original
under wine (a boot patched to start `ma01_002`; shots in
`research/nitroplus/local/shots/fig-ma01_002-orig-*.png`). The strokes
(／＼ around ○□卍□○) line up the same in both; the note that it came
apart in the vertical box was wrong. What differed: the original turns
the arrows a quarter turn in vertical text (`おれ→` points down at the
figure), so CONscripter now does too (c3d5ffc). It also draws ∧ turned
(as ＞), which the script's own horizontal sketch in the comments
doesn't want; probably wine's substitute font, so ∧ stays upright.

**The route run that stopped drawing (2026-10-15):** a replay with
`--shot-pages` (engine 0.19.0) once stopped at `f_md01_002`'s
`print 10,500` (page 18,573) with the process spinning. Not reproduced
with engine 0.22.0: the run through `md01_002` (`engrun.py --only 3 --at
md01_002`, ending in `md04_045aa`) played its 160 files to the end, and
again with a shot of each of its first 400 pages. Closed unless it comes
back. On the way, nstrace looped forever in `ma02_003a`: it skipped
`btnwait`, so the favour screen's wait loop never ended; `btnwait` now
sets its variable to 0, a click.

### One `*conv_boot_end` (2026-10-18)

The boot logos' subroutine was jumped over to a label of the same name
as conv-cons's boot guard end (`*conv_boot_end`, defined twice; the
engine keeps the first). It is `*conv_boot_body_end` now (dead code
today: the jump follows the story's last jump).
