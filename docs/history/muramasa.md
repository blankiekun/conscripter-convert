# 装甲悪鬼村正 (Nitroplus NSS): History

Done items moved out of [the game document](../games/muramasa.md)'s Status.

### New pictures zoomed from a size or on a path (2026-10-19)

An unwaited `Zoom` from a size other than 0 (ma04_018vs:365) or a `BezierMove`/`FadeFR` path (mc04_008:138) was held and cross-faded. sync.rs's new-picture loop now gives both as an `spmotion` (FadeFR's `Random` jitter adds 29 runtime `rnd` forks). Holds 1,525 -> 1,243.

### No hold under a cover (2026-10-19)

A sync under an opaque `Plain` cover waited up to 1 s for unwaited actions running under it (127 of 1,652 holds: cockpit gauge processes, clouds, 80 s background pans). Nothing under the cover is seen and the exe goes on, so the sync waits only the script's own time. Left: such an action is still drawn at its end when the cover goes (background-motions item).

### `csp 49`-`58` once per load, not per `setwindow` (2026-10-18)

Every `setwindow` was followed by `csp 49`-`58` (only to clear old sprites in saves from before #222): 2.7 MB (9 %) of `0.txt`. Now the `loadgosub` routine `conv_load_clear` (every game) clears them once. Sprites 49-58 are used by nothing else in any game.

### ma00_000 picture diff: what is left (2026-10-18)

#457's 24 blocks with `--real-time`: pages 57, 66 (`SetShade(…, MEDIUM)`) are about 2.5/255 darker in the exe (its 3×3 mean floors `sum / 9` per pass, `FUN_00465b10`; the engine's blur rounds); invisible, not fixed. Pages 55, 56, 62, 64, 76-79 differ only inside the text window (the original is still typing 1 s in; on 79 ours is 200 ms into an unwaited 300 ms `Move`).

### `DrawTransition` as the engine's `spfadeto` (2026-10-18)

`DrawTransition` on a picture (image or strip) is CONscripter's `spfadeto` (engine PR "spfadeto"): the sprite fades through its mask from `from` to `to` while the script goes on, the table made as the exe does (transition_start 0044fe00, 00457bf0). `from` is taken as given (the exe's start doesn't read it); the "from another opacity" note fires only for a `Plain` cover or other object, which keeps the masked print (13). Gaps 3,187 -> 2,569. `system/veil_*.png`, `transition.rs` and the `spmask` veils are gone; a waited call is an instant `print` and a `delay`. Checked: full sweep matches the old `effectsoft` within 1.3 % of pixels, partial ends on the same frame.

### Plain covers as screen copies (2026-10-18)

A `Plain` cover that is moved, zoomed, turned, blended (`Request` `AddRender`/`SubRender`/`MulRender`, `CreatePlainEX*`/`SP*`) or waved (`DrawEffect`) was not drawn (md05_016vs's 空間歪曲 flashes showed nothing). Now it is one sprite holding a copy of the screen under it (`spgrab n,-3`, new in the engine), taken when first shown while the screen under it is still what it froze; the engine zooms (`spzoom`, `spmotion` keys), blends (`spblend`), waves (`spwave`) it and a partial `DrawTransition`'s veil cuts it (`spmask`). 153 copies in Muramasa; Django has no `Plain` covers.

Also fixed: `msp` now comes before `spmotion` for a new picture (`msp` ends a kept motion, so a picture fading in to a partial opacity jumped to its end; ≈ 760 lines in Muramasa).

#### A Plain cover shown again (2026-10-18)

A `CreateEffect("Plain")` cover keeps the screen as it was when made, even after it is faded out (ma00_000 makes one per fighter, deletes the pictures under them and fades them in again). The cover now remembers the pictures it froze; once at 0 and the screen under it changed, it draws those (its shade or tone over them) at its own priority. 10 such fade-ins in 5 files, more through `CreatePlainEX`. A cover moved, zoomed, turned, shaken, blended, or that froze a strip or movie (md04_037's rain) still keeps the screen as before. Pages 62, 64, 76-78: 83-86 % of pixels differing before, only the text window after.

### Status as it stood before the 2026-10-18 prune

Moved whole from "Status"; its subsections are one level down.

#### Web crash on the second start (2026-10-17)

Web build v0.29.0 trapped "memory access out of bounds" on every start after the first: the engine read `kidoku.dat` through a 64 KB stack buffer, the web build's whole stack. Fixed for every game in CONscripter #148.

#### Trace diff (2026-10-17)

Original's trace against our `--auto-play 60 --real-time` run, first 50 pages of `ma00_000`: every page pairs by its `<PRE>` line. Waited `Fade`s end 30 to 60 ms after their time in the original (wine's frames, not a rule to copy). Every timed effect of the exe (Fade, Move, Zoom, Rotate, Shake, transitions, volume: 14 sites) builds `floor(time * 60 / 1000)` steps (`cb_Fade_0044f020`) and advances one per 16 ms of real time (`tempo_advance` 00449930, speed 1.0 at the default index 3), so it lasts `16 * floor(0.06 * time)` ms (144, 96, 480, 960), a little less than the time; the waiting script sees it at its next 16 ms poll (`wait_effect` 004489e0). Ours (time + 9 ms) is within two frames of the exe at 60 Hz, so nothing changed; making every effect 4 % shorter is possible, not done (lead's call). Django.exe has the same rule (advance 004348f0, fps table 0x531a90). In `trace_diff.py` the original's command ends when its next record starts, ours at its last `ev`: so the 4 to 161 ms after a box fade were not findings.

#### Exe mapping redone (2026-10-17)

`docs/games/muramasa.findings.tsv`: one row per built-in and branch the dry run sees, marked seen/inferred/unknown with the exe address (Muramasa.exe sha256 `1d67e4771a72…`; `research/nitroplus/names.tsv`). Fixed from it: unread arguments (#366), a shake growing from 0 px (#374), `SetVertex` lower case only (#380), colour names (#383), `DrawTransition`'s softness and fade-out direction (#386, engine #138). Open: `SetFrequency` on music (1,486); `CreateSurface`/`SetSurface`, `Position`, `CreateMovie`'s arguments, `Delete`'s 1-2 unknown. Names were carried from Django.exe by hand, not by a version-tracking match (step 21).

Findings audit (2026-10-18, `docs/new-game.md` step 17): 12 of 13 re-derived rows confirmed. `CreateWindow`'s argument 6 is not inert: anything but `true` clears the window's +0x9d4 (`create_window` 00443500), so `true` windows place children from the window's place (`init_object_place` 0044c8c0, `parse_object_place` 00458d50). Fixed: children placed from its place, their named places (`Center`, `InBottom`) count in its rectangle, they move with it (`Compiler::places_children`; Django's `中窓` busts too). `Shake`'s flip step is truncated in the exe, rounded in ours (≤ 1 ms); `DrawTransition`'s table is stored truncated (fldcw 0xc00 at 00457dee), as `effectsoft` does.

#### Accuracy (2026-10-17)

Every hand-typed value of the Nitroplus converter carries a `// src:`, `// guess:` or `// ours` comment. Open guesses a frame of the original could settle: the click-wait flame's frame times (53 ms × 19, 77 ms × 13, a 500 ms pause); a choice's text 40 px down its plate, red under the pointer; the title's glow (the exe's `SetBlur(…, 2, …)` is a trail of the last frames' copies, `FUN_00456810`; ours a 3 px blur); looped and paper `Shake`s turn every 1000 ÷ (n × 0.06) ms in the exe (`queue_shake` 004462d0), every 100 ms in ours.

Settled from the exe: tones (`0043fcc0` builds grey tables of r 28, g 151, b 77 per 256, red and blue swapped since the exe reads BGRA as RGB; `004401d0` Monochrome is that grey, `004403d0` Sepia adds 30 to green and red, `004405e0` KitanoBlue 20 to green and 80 to blue; `spfilter` kind 11 then an added colour); a camera's depth rule (`FUN_00489570` at `00489a52`: a picture under a camera scales by z ÷ (1000 − priority) about (512, 288)). The exe draws only the innermost of nested cameras, ours multiplies them; not checked whether a script nests them.

#### Save and load drawn by the engine (2026-10-17)

記録/読込 are CONscripter's `screen "save"`/`"load"` (`conv-cons/src/save_grid_screen.rs`, `SaveGrid` with `engine_screen`; engine #87): slots down the columns, frames over thumbnails, hover details, 前頁/次頁 with a 300 ms cross-fade, 削除 a toggle, the game's questions (`save_ask_*`), 記録しました. A save's text is "|play time|chapter|last line" (date is the file's). Checked on Xvfb against main's routine: 0 pixels in 8 of 12 frames, the rest only the play time's last digit and 3 px of 削除. Open: the play time is read when the screen opens; S and L no longer close the screens (right-click and 退出 do). EN's save screen is the engine's too (docs/games/muramasa-en.md).

#### Backlog drawn by the engine (2026-10-17)

ログ表示 is `screen "backlog"` (`conv-cons/src/line_log_screen.rs`, `LineLog`; engine #88, #100, #101): `*conv_backlog` lays the history out with `logrows`; rows (`source = "rows"`, `%170` rows back from the newest) with each voiced page's voice button, 逆/進 and the wheel a row (wheel-down past the newest closes), the knob a picture slider on `%170`, 退出, 300 ms fades. Against main's routine: 0 pixels at open, the rest only the knob (1 px) and 逆 at the oldest row. Open: the knob sits up to 1 px off after a wheel or drag (the slider rounds, the routine truncated), and a press on the sweep away from the knob jumps it there; 逆 at the oldest row and 進 at the newest are not buttons, so they don't light; a row's voice button follows the voice switches when the backlog opens (below, "History", "The backlog's voices and the voice switches"); `SetBacklog`'s speaker 鈴川 in ma01_021 has no switch (the other 52 voiced lines have one).

#### Extras' menu drawn by the engine (2026-10-17)

特別付加目録's menu is `screen "extras"` (`conv-cons/src/picture_extras.rs`, `PictureExtras`; engine #93): picture and buttons (a locked one at 51/255 and not a button), 500 ms fade-in. It closes holding its frame (`hold`), so the chosen mode and the title cross-fade straight from it. 0 pixels against main's routine (JP and EN builds). Open: the four modes' screens.

#### CG gallery drawn by the engine (2026-10-17)

画像映写機 is `screen "gallery"` (`conv-cons/src/gallery_screen.rs`, `PictureGallery`): each category's pages are the screen's pages (19), a slot its first seen CG's thumbnail (128/255 while some are unseen, 空白アイコン while none is), 2D/3D/BG tabs, 前/次 round inside the category, 300 ms cross-fades. A slot closes the screen holding its frame and the script's viewer runs; back from it the screen reopens on the same page. 0 pixels (1 px on an arrow). Open: the viewer is still the script's.

#### Scene replay's list drawn by the engine (2026-10-17)

回想映写機 is `screen "replay"` (`conv-cons/src/picture_replay.rs`; engine #100): thumbnails (空白アイコン while locked) under cursor buttons, 退出, 500 ms fade-in. A scene closes the screen holding its frame; `spgrab 999,-2` copies it so the freeze (`screenzoom` at 100 %) holds the list over the scene's first pictures; at the scene's end the list fades in again from black. 0 pixels (1 px at 6 s).

#### Sound room drawn by the engine (2026-10-17)

電磁蓄音機/動画映写機 is `screen "sound"` (`conv-cons/src/sound_screen.rs`, `SoundRoom`; engine #101): track buttons (a locked song's `000` picture), the turning A-icon by the playing track (340 ms), movie thumbnails, 退出. A click closes the screen holding its frame; the script plays or stops the track or plays the movie (fading to black and back over 300 ms). The room exists only in a build given `--movies`, so the snapshot's `0.txt` doesn't change. 0 pixels but for the movie's own frame.

#### Settings drawn by the engine (2026-10-17)

コンフィグ is `screen "settings"` (`conv-cons/src/settings_screen.rs`, `PictureSettings`): six picture sliders (engine #78), pairs as radios, the voice window a screen over it (`settings_voices`), 承認's question its own dialog (`settings_confirm`). Values are named globals (`%conv_set_bgm_volume`, …, `%conv_set_done`); a gloval.sav from before gets `%2920`-`%2938` copied once. 0 pixels against main's routine. Fades (engine #81): screen and voice window 300 ms, a pair's chosen side 100 ms, the reset question unrolls out of x 512 over 150 ms (back over 200). Open: the knobs at 207 and 407 sit 1 px lower (engine #80 rounds); a press on the knob keeps the grab offset (a drag ends 2 px off); 続・殺戮のジャンゴ's stepped sliders still use the script routine.

Muramasa EN's settings (`settings_en`) are the same screen: lying sliders as picture sliders with the lit bar as `fill` (engine #80; a negative `travel` lights from the left end, engine #86), check boxes toggles with `pressed`, ON/OFF pairs radios. 36 pixels differ at open (knob rounding); main's routine left the lit bar where the press began (its bug). Open: EN's 300 ms fade is a guess (the Japanese value), its pairs switch at once.

#### Queue (2026-10-16)

Old queue, condensed (the PRs hold the detail).

- `CreateProcess` (plan changed 2026-10-16, the user: played on paper at convert time, not Lua; the compiler draws the screen at its syncs, so a Lua process would have no sprite of its own). Done: A endless loops, B `system.lua` from the IR, C1 loops behind an `if` with a lead-in, C2 one-shots on paper (#151), `ComicProcess` (83, #167: `Shake` on paper by `shake_keys`; non-looping branches via `loops.rs` `run_loop` -> `play_once`; 16 of 18 signs are `Hideable`, so drawn at their end state; a zoom unlike in x and y is keyed in both, `MotionKey::zoom_y`, #188), C3 (#171: `Random` allowed on paper giving 0, so `MFlashProcess` (43) is a one-shot; 3 calls of 100 flashes exceed `spmotion`'s 64 keys). Lua (`Op::StartProcess`, `Script::lua`) stays for a process that needs the run (none left among one-shots); endless processes with `Random` (`BGMoveLengthProcess`, `ma03_moveAuto*`, `RandomMeimetsuSet*`, shakes) stay noted. Left: `EndSong` (12, #176: `if`s decided as it starts; `SetVolume`/`WaitAction`/`SetFrequency`/`SetLoop`/`Request` allowed; one starting a sound after a wait is put back and noted), `ma03_024`/`025`/`027vs` `_runsProcess` (23).
- Merged/done: `CreateText`'s box height (#52); trailing `*` check (#39); a wildcard doesn't reach a window's, name's or camera's children, `Delete` still does (#41); a picture's children are placed from its corner (#43); `SetVertex` for every picture (#61); `SetBlur`'s trail as `sptrail` (engine #25, converter #82; its last argument `over`, engine #83). `spgrab` (engine #32) could stand for `CreateTexture`'s render-to-texture: not used.
- Other agents' shared NSS changes: choice options set in runtime branches are picked at run time (`Op::RuntimeChoice`, `$3632` up, flags %783-%798; line-log entries `Op::RuntimeBacklogLine`, `$3638`; #60). `SetPan` converted (`chpan`, engine #35; Muramasa pans nothing off centre). `Shake` of a picture smaller than the screen moves the picture alone (83 of 144 calls). `WaitPlay` on a sound waits for its rest (#70) and a disused one-shot stays until its end (#76): ~130 new `delay`s (ma01_003's door 2,528 ms), 366 fade-outs; a `WaitPlay` after a `SetVolume` to 0 over time waits for the fade (67 new `delay`s).
- Known limits: a moving window (`絵演窓上`, `窓右`, ~56 `Move`s) has its clip jump to the end place at the move's start. Sprite movies don't play in CONscripter for Windows (only an outside player): engine item, not started.
- `Random` (#354, 2026-10-17) is picked when the game runs (exe: `rand() % n`, `eval_primary` 0043a780, reseeded each call). A statement that draws one is compiled once per pick and `rnd %81,N` chooses: each value for a single draw of up to 16, else 8 samples per draw (the `FadeFR*`/`MoveFRP*`/`MoveFTP*` jitter); picks span from the last statement where nothing waited to the one that waits, sounds every pick starts alike are played once (`convert/random.rs`). Left (771 calls, noted "Random picked when converting", a seeded pick): picks reaching text (SetKenki's 《…！》 in md05_008vs/009), leaving a picture at a random place (md04_043, `CP_AziChange`), or giving a loop to a picture shown later.
- Decided: the hover sound's 0.125 s of silence is in the original file and stays.

**Route runs** (2026-10-14): the full build played headless in CONscripter 0.19 with `--auto-play` and a debug trace, every page drawn, files checked against nstrace; ten runs passed (一条END 246 files/26,704 pages/24 min; 香奈枝END 349; 茶々丸END 300; five bad ends via `mz00_000` 436-455 files; 村正END 510/36,276/29 min; TRUE END 514/36,449/30 min). The checker (`cons_trace_check.py` with `research/nitroplus/cons_trace_check.json`) found seven bugs, fixed. Its expected remaining findings: `voice` (lines in 『』 with no voice in the source), `crowd`/`offscreen` (NSS layers full-screen objects, parks some off screen), `sound` (battle scenes layer one effect on several channels; `se戦闘_` is in the config's `known`), `memory` (pre-scaled zoom pictures of 67-77 MB). Other agents stop engines with `pkill -x conscripter`: run ours through a renamed symlink (`--engine out/conscripter-mura`); route runs and `--at` starts are in `research/nitroplus/engrun.py`.

Next, in order (as it stood):

1. **System screens** (step 7): the original filmed under wine from patched boots; every confirmation is drawn from `cg/sys/dialog` parts (the only `Message()` box is for a display option we leave out). Seen: right-click at a text wait opens the menu, wheel-up the backlog (a line a notch, wheel-down at the newest closes), right-click closes every screen (No in a dialog), Esc asks ゲームを終了しますか, the title's 終了する quits with no question, the backlog shows no names. 1-6 done (button strips, title, right-click menu, save/load, backlog, settings). 7 motion and movies done (title fire flicker and openings, dialog unroll, menu cascade) but for the quick bar sliding while text types (dropped by the user, 2026-10-15; needs the engine to move the bar). Movies: CONscripter 3f368bc draws the text window over an `mpegplay` movie and plays a movie given to `lsp` in the sprite's place (`spmoviewait n,1`): the cuts under the text play that way; a skipped movie stops the `dwave` sounds started with it (265942e). 動画 切 is `moviestop -1` (`MoviePlay(f, #SYSTEM_break_play_movie)`): the openings are always skippable, `mvOP`/`mvOP02` only with 動画 入 (default 切). The original under wine draws no movie (no 32-bit GStreamer plugins), so the smoke's strength (`SubRender` subtracts the white smoke; ours draws it black, lighter) is unchecked.
2. **Extras** (特別付加目録, `extra_*.nss`; step 8 of the old list, "extras 1-7" in the game doc), done: menu, 画像映写機 (CG gallery: 349 slots in 2D/3D/BG, 805 CGs, each a `#<cg name>` flag), 回想映写機 (ten H scenes, each unlocked by its file's `#ma01_020`-style flag, replayed from `$PLACE_reco`), 電磁蓄音機/動画映写機 (37 BGM, five songs, three movies; the track chosen stays the extras' music `#エキストラＢＧＭ`), 劒冑回転目録 (46 looping 400×400 armour movies with `#av_`/`#aw_`), the way back to the title and the ライナー画像 button.
3. The user's decisions (2026-10-14): the pressed state (`003`) is an engine feature on for every script; the settings CONscripter lacked (music/SE/voice mute, voice stops on click, auto waits for the voice, auto/skip stop at a choice, movie skip, game speed) are added to the engine the same way (since then no engine addition is opt-in); 選択肢に戻る is implemented (a save to a reserved slot before each choice). Set with `consset "key",value`, read with `consget` (keys in the engine's `COMMANDS.md`).
4. Gaps from the exe review (2026-10-15; `research/nitroplus/builtins.md`, `names.tsv`): text markup dropped (`<VALUE name=$mc02_時間>`, the airship clock in `mc02_031time`, no runtime strings in the IR; ASCII-art pages `PushText` + `NoLog`; `<voice mode="off">` (3)); a 4-argument `Fade` probably doesn't wait in the exe (49 calls wait in ours; check under wine).
5. Gaps from the unread-argument notes (2026-10-16): tempo of `SetVolume` and the text window's fades (a sound's volume has no curve, nor `windowfade`; a fade folded into motion keys is straight; 9 `SetVolume` fades over time keep a straight ramp, the exe steps them by the tempo, 00478570). A 0 ms `Fade`'s or `SetVolume`'s tempo, a 0 ms `Shake` stop's wait and `CreateWindow`'s last argument change nothing in Muramasa.exe (`cb_Fade` 0044f020, `sound_set_volume` 00475f50, `builtin_CreateWindow` 0040b3a0). Story `CreateText` sideways text turned by `Rotate` sits ~17 px lower than under wine (cause unknown; "Story `CreateText`"). `DrawTransition` (FUN_0044fe00, 00457bf0): the mask is a threshold table (from and to × 0.255 its ends, then a ramp losing 255 ÷ p × 255 ÷ softness a grey level; a fade-out's threshold falls); full sweeps (2,073 calls) use `effectsoft` (engine #138), partial ones (367) `spmask` ("Veils laid on the screen"); a mask cg.npa lacks (ma04_018vs/020vs's `slide_01_02_2.png`, `cg/data/7.png`) means no transition (the exe returns before it). `Conquest` returns 0. Done: a whole-screen `Shake` growing from 0 px (7 calls; `queue_shake` 004462d0); `SetVertex`'s names count in lower case only (nothing lowers them: `eval_arg` 00413740, `FUN_004891e0`), so `Center`/`Middle` are atol's 0 (絵演鎧* zooms about its left edge, Rollbg from its top). Dropped, meaning to check in the exe: `MoveCamera`'s arguments in a process (4), `LockVideo`, `WaitText`, `SetFrequency` (but 1000), `WaitAction`'s time limit.
6. Gaps from the script coverage (2026-10-15; `research/nitroplus/builtins.md`, "In the scripts: coverage"): `CreateEffect` Drop (1, mc04_052's snow) and Lens (2) not converted. Rain converted (#161): `convert/rain.rs` plays the exe's rain (`00435e60`, `00436390`) into a 24-frame strip (`$SYSTEM_effect_rain_dencity` drops, at most 128, grey streaks 196 px with the exe's alpha ramp, falling their speed per 16 ms, redrawn every 42 ms), looped; frame period 16 ms (`init_frame_tables`, `0048c5d0`). Tone kinds tint the pictures below (#137, #146); `SetTone` (228) draws the object in that tone (`FUN_00452000`); the sepia and KitanoBlue colours were still unverified then (settled later: "Accuracy"). `SetShade` (163): level passes of a 3×3 mean (`FUN_00465b10`; a second call adds passes, `NULL` restores) as one `spfilter` box blur. `DrawEffect` waves (Low/Middle/High/SuperWave) by `spwave` (CONscripter#76): the exe (`FUN_00430a70`, `FUN_00430de0`, `cb_00431230`) shifts each row by trunc(sin(2π·phase/256) × amp) px, phase = row × K / height (K 64, 512, 1024, 4096 by name) plus one per tick of more than 32 ms; amp goes `from` -> `to` over `time` by the tempo then stays (ramped to 0 is removed). Left: Ripple (4), Split kinds (7); a wave picked up later restarts its rest straight. `CreateStencil` (59) and `CreateMask` (28) converted (own sections); left a picture moved or zoomed over time under them, a zoom about a vertex, and a `Plain` cover under them; a colour mask is exact only for the alpha blend. `LoadImage` as a texture source on since 2026-10-17 (25 `CreateTexture`s from an object or the screen noted). `Request(…, NoIcon)` (4) dropped without a note (a `Request` on an object the converter doesn't keep returns silently). `system.ini`: the auto-mode icon (`icon_auto`, 15 frames) and the line-end kinsoku (`bad_end`) unused. Check under wine: `<BR>` (16 uses) isn't in the exe's tag list but breaks a line in ours. Small: 25 sounds missing from the archives, 2 transition masks missing, 2 statements that don't parse, 2 voices outside a text block, 3 BGMs without loop played looping.

**Leftovers** (dry run on main, 2026-10-17): 1,162,127 source calls, 1,131,605 converted fully, 21,928 with gaps, 8,594 left out on purpose.

- `CreateProcess` (510 calls): left 108 notes, most `TurboBlur3` (10), `DenziBladeChargeSet` (10) and the runs that don't end on paper (about 45). `Request(<process>, Stop)` (1,126 notes) is dropped: not checked whether a looped motion keeps running where the original stops it.
- Moves and zooms over time that come with a fade or a new picture are drawn as cross-fades: 38 left (about 1,200 before the motions).
- `Rotate` about z of other objects (31), a turn over (about x or y) over time, mirrored at once (10), and `CreateTexture` from an object or the screen (25) are left.
- Cameras (`CreateCamera` 40, `MoveCamera` 105) are drawn by 続・殺戮のジャンゴ's model, measured there under wine (a child of depth `p` seen `z / (1000 - p)` times about the screen's centre, the camera's place the other way; `Kind::Camera`); moves are engine motions with their tempo. A camera at `Center`, `Middle` is at 0, 0 (ma04_005a's pictures cover the screen only so). Unverified: zooms other than 1000.
- `BezierMove` (335 outside processes) moves the picture to the path's end and the sync's motion goes through the path (`loops.rs` `bezier_move`, `sync.rs` `along`); its tempo eases the whole motion; where the sync draws no motion the picture is only put at the end.
- 18 files whose `if`/`else if` chain on earlier flags has no `else`: one branch always matches by the story's logic; the fall-through ends the run.
- Engine: an exit with status 255 in vertical text was probably a script error in the text (a half-width `/`, `#`, `?` or `%`; before 0.18.0 every script error exited 255). Now a script error exits 2 and the trace's `error` record gives message and line; the checker's `text` rule finds those characters. CONscripter once segfaulted on `end` after a `dwave` voice with Ctrl held (not looked into).

### Fades that change nothing (2026-10-18)

`cb_Fade` 0044f020 (Django cb_00438fb0 the same) compares the target's opacity byte, round(n × 0.255), with the object's: equal, it sets it and starts no action, so a waited `Fade` doesn't wait and an unwaited one adds nothing to the next sync. A new object's byte is 255 (`FUN_00488740`), so `CreateColorSP(…); Fade(…, 1000, 1000, …)` shows it at once. The compiler skips such a `Fade` when every object it names keeps its byte and none has an action still unsynced (a `DrawTransition` under way keeps fading); objects left out (`untracked`) and the text window's boxes wait as before. 174 calls in Muramasa; 0.txt 159 hunks (77 only `delay`/`wait` gone or merged, the rest syncs no longer stretched).

### Actions on no object (2026-10-18)

A `Fade` or `Move` whose name matches no object starts no action in the exe (`cb_Fade` 0044f020 queues none; the wait, 00445ac0, waits on the objects' action slots), so it neither waits nor adds to the next sync. Above all `TypeEnd`'s unwaited `Fade("@Face_Sepia", 150, …)` (function.nss:1058; `@Face_Sepia` is never created) made ours wait 150 ms after each page (4329 `delay 150`s); the trace has the next statements 30 ms after. The compiler keeps the names of objects it leaves out (a `Create*` that makes no object, as the choice screen's plates) until a `Delete`; an action on one still waits as before.

### Auto mode and `WaitKey` (2026-10-18)

Auto mode ends a `WaitKey(N)` or `WaitKey()` after 1.5 s in the original (300-page trace, ma00_000:2496-2662: 1505 to 1527 ms, N = 2000 or none). The exe (builtin_WaitKey 0040f9d0; defaults 0047b400 set 1500 beside the auto settings; Django `reset_system_defaults` 0045cb30 too, not checked in a Django trace). The compiler puts `Op::AutoWait` (1500 ms less what the sync showed) before the wait when N is longer, and before `WaitKey()`'s click wait; conv-cons makes it `isskip` and a `delay` in auto mode.

### Boot logos (2026-10-18)

`0_boot.nss` runs `TitleLogo` before `TitleSet` while `$Logo` is 0: the first title of each run (a local, no save flag). Steps, ms: white over black 300; `media_a.jpg` in 500, `WaitKey(3000)`; `media_b.jpg` cross-fades in 500, `WaitKey(3000)`; out to white 500; `Wait(1000)`; `Nitroplus-logo.jpg` in 1000, `WaitKey(3000)`, out to black 500; `Wait(500)`; `注意事項.jpg` in 1000 with `WaitKey(1100)`, `WaitKey(8000)`, out 500 with `WaitKey(600)`; `Wait(100)`; the opening. A click ends a `WaitKey` hold (and the fade under it), never a waiting `Fade` or a `Wait`. `Game::boot_logos` compiles `TitleLogo` into the subroutine `conv_boot` that a `BootStep::gosub` runs once a run (`%2960`); 23,672 ms in `--real-time` against the script's 23,600. `WaitKey(ms)` after a fade that doesn't wait now counts the fade in its time.

### Eased sound ramps (2026-10-18)

`SetVolume` and `SetFrequency` build steps by the tempo as `Move` does (`FUN_00478570` -> `apply_tempo`). `Op::SoundEase` before the ramp; `chvolfade`/`chfreq` take the curve (engine #145; a stop's fade-out goes along it, then halts): 32 `chfreq`, 7 `chvolfade`. Still straight: the music's eased fade-out (mc02_013, `bgmfadeout`) and a sound's fade-in as it starts (`dwavefadein`).

### Unwaited fade-outs (2026-10-18)

A `Fade` to 0 that outlasts the sync's wait was part of the blocking cross-fade (the text came 918 ms after it in ours, 36 ms under wine; now 609, the rest the box's own fade-in). Now an engine motion to alpha 0 and the picture is taken away at a later sync. A picture disused as such a fade starts (`FadeDelete(…, false)`) is kept until a sync after the fade (the exe's frame on ma00_000:137 shows the old sky at a third after the 1 s `Wait`); not one that also zooms or turns (mb02_028vs:123) or that a `DrawTransition` wipes (md01_029vs:2107): those go at once. Limit: the later sync takes it away even if the reader clicks on before the fade's end.

### Zoom to 0 (2026-10-18)

A `Zoom` to 0 along an axis was kept at 1‰, leaving a 1 px line (32 static 1×1 copies; Django's dja023 one). The exe stores the zoom as `atol` gives it (`FUN_0044f2a0`, only the time 0 becomes 1), so a picture at 0 is not drawn. Now it isn't; one zoomed to 0 over time is squashed by the engine (`spmotion` to zoom 0) then removed; one not shown that grows from 0 is placed where it ends and keyed from 0 (`Compiler::pending_zoom_from`). `Random` over a picture at 0 (ma04_016vs:6680) is picked when converting (35 draws).

### Trace diff fixes (2026-10-17)

A `Move` of a picture not shown yet (ma00_000.nss:128-136, 370-385) put it where the move ends; the original pans it from where it was created while the script goes on. Now the next sync places it at its end and moves it from its start (`spmotion`, tempo kept) and, as a moving picture, it no longer stretches the sync's cross-fade to `UNWAITED_MS`: `Fade("背景30", 500, …, true)` is `print 10,500` again (509 ms against the original's 545, was 1009). About 1,360 `spmotion`s in Muramasa, ~200 in Django.

### Veils laid on the screen (2026-10-18)

A partial `DrawTransition`'s veil is read by the exe's draw (`00418320`) at the screen pixel, so it stays put while the picture moves (mc04_022vs's jittering rain strip). CONscripter's `spmask num,"mask"` (engine #153) draws a sprite times a mask fixed on the screen, applied after the sprite's motion, zoom, turn, wave or blend. Every veiled picture (images and strips) gets `spmask` with its `system/veil_*.png` (`Shown::mask`, `Op::SpriteMask`); a veil that comes or goes is placed again, never done by a motion, so its sweep stays a mask print. Stencils (`CreateStencil`/`CreateMask`) are still cuts. Checked: frame equals `bg + (strip − bg) × veil` within 1 on all 16,856 lit pixels (the old cut differed on 2,661). About 1,450 hunks in Muramasa and EN; Django unchanged. (Superseded by `spfadeto`: the `veil_*.png` are gone.)

### Partial `DrawTransition` (2026-10-17)

A `DrawTransition` ending between 0 and 1000 leaves its picture drawn times a table of the mask's grey, for good (exe: `FUN_0044fe00` builds the table, `00457bf0` steps it each of `time` × 60 ÷ 1000 frames and keeps it unless the level is 255; the draw `00418320` reads the mask stretched to 1024×576 at the screen pixel, the green byte, times the picture's own `Fade` opacity; only a new `DrawTransition` or `Delete` takes it away, `0045a490`). The converter replayed the table (`convert/transition.rs`, exact for the last frame) and cut the picture by it as by a stencil: 367 calls; from 2026-10-18 the 4 on rain strips are cut too; the 2 on md05_016vs's `Plain` covers unmasked. Partial ones from a level to 0 or 1000, or to a level under 4 (× 0.255 is 0), end unmasked or hidden. Checked: five veils equal a Python replay of the table (0 of 589,824 pixels).

### `Wait` and clicks (2026-10-17)

The exe's `Wait` (Muramasa `0040f7f0`, Django `0040bd80`) ends only on quit, a state change or skip mode; a click doesn't end it. Ours was `delay`, is `wait` now (CONscripter ends it in skip mode, engine #134). `WaitKey(ms)` stays `delay`. About 6,800 lines changed.

### The backlog's voices and the voice switches (2026-10-17)

The exe keeps each log line's speaker with its voice (`FUN_00403730`; a text's `<voice class>`, `SetBacklog`'s third argument) and its backlog (`FUN_00403910`) offers the voice only when the speaker is not in `#SYSTEM_voice_enable_list` (`ConfigCharacterVoice`): decided when the backlog is drawn. Now every voiced line runs `logvoice "<voice>",<character>` before its `if %970+c == 0 dwave`, `SetBacklog` lines `logadd "<text>","<voice>",<character>` (speaker as a class, or the class it starts: 村正 for 村正さん), and the backlog's rows list has `voice_off=970` (engine #103). 36,053 `logvoice` lines, 53 `logadd`s with a tag.

### Tempo of `Fade` and `DrawTransition` (2026-10-17)

Both ease by the tempo through `apply_tempo` (seen: `0044f020`, `FUN_0044fe00`). A sync's transition goes along the last eased fade's curve before its print (`effectcurve`, engine #85; `Op::TransitionEase`): 588 transitions in the story, 61 in Django (its exe eases the same: `cb_00438fb0`, `FUN_00439d20`, curve table `FUN_00434ea0`). `Game::fade_tempo` turns it on for these two and for Muramasa EN with the port's own reading of `AxlDxl` (muramasa-en.md, "Eased fades"). The rest of the 3,869 eased `Fade`s take no time, fade the text window, or become part of a picture's motion.

### The text window's fades (2026-10-17)

The box is an ordinary picture the scripts fade (`Request(…, Hideable)` only lets the player hide it): the text's preprocess fades it in over `$BoxFadeTime` (150 ms) and a waited `Fade(…, 0, …, true)` takes it away. Now CONscripter's `windowfade` (engine #82, `Op::WindowFade`): 5,644 hides become `windowfade 0,ms` with the script's wait after, 5,573 shows fade in from 0 with the next page (`windowfade 256,ms,0,0`, text held for the waited fade). An unwaited fade to 0 is a box switch and changes nothing. EN the same; Django unchanged.

### Pictures made from `LoadImage` (2026-10-17)

`CreateTexture(…, "name")` draws a picture `LoadImage` loaded under that name (`builtin_CreateTexture`, `FUN_004491a0`, `FUN_00488670`). `Game::loaded_textures`: the story's pictures made that way (scrolling battle backgrounds, `雑魚イメージ`, `イメージ１`) are drawn, ~11,000 lines of `0.txt`. The choice screen's plates (`select_img*`) stay the engine's choice: left out (1,776 `CreateTexture`, 504 `LoadImage`; Django's 40 and 10 the same).

### Tempo of `Rotate` (2026-10-17)

`builtin_Rotate` passes its tempo to `FUN_00446010` (kept at +0xa0); the step `FUN_0044f620` eases each of three angles through `apply_tempo`. A turn about z over time (`spmotion2` angle) now takes the curve: 187 motions (same in EN); x/y turns already did (`convert/turn.rs`). Django's 42 eased `Rotate`s now count as read.

### `Shake`'s `n` and tempo (2026-10-17)

`queue_shake` (`004462d0`) stores the step 1000 ÷ (`n` × fps ÷ 1000) ms and returns at once for `n` above 1000; `shake_frame` (`004578e0`) flips the offset once `timeGetTime` is more than a step past the last flip, the size eased by the tempo over `time` × fps ÷ 1000 frames. fps is the double at `005dcb40` + 8 × speed index `005dcaf0` (4, set by `init_frame_tables` `0048c5d0`): 60.0. So `n` 1000 flips every 17 ms, 500 every 33, 200 every 83. `Game::shake_jolts`: 1,284 screen shakes are `screenshake` mode 2 with that step and the tempo curve; 81 picture shakes' `spmotion` keys turn at that step. EN keeps the old shake (its exe not checked). `Shake`'s argument 8 (wait, 58 calls) is still unread.

### `CreateStencil` (2026-10-17)

Exe `00409fa0`: mode 128 (every call) turns the picture into its alpha; `false` leaves the object a container (a wildcard doesn't reach its children, which sit at their own place). The converter cuts each child's picture by the stencils over it (`Assets::stenciled`, a zoom about the centre), cached while picture, place, zoom and stencil places stay the same; a child created off the screen and moved in is cut where it ends up. Checked against the exe's rules in our build (ma04_013vs, ma05_019, ma02_009vs); not compared with the original (no wine run). Inferred, not checked: pictures under a cut child are no longer culled as under an opaque one.

### `CreateMask` (2026-10-17)

`CreateMask(name, priority, x, y, picture, wildcard)` draws nothing itself; its children are drawn multiplied by the picture, byte by byte (colour and alpha), and not at all outside it or where it is clear (28 calls; Django's 30 are in the choice screen, left out). Exe: `builtin_CreateMask` `00409ce0` (the last argument to `00443800`, as for `CreateStencil`; a wildcard reaches the children only if `true`); `00457e80` zeroes pixels of alpha 0 and makes a 1024×576 buffer; `00416860` draws the picture into it at its place, zoom, turn (inside another mask the two multiply per byte); the blitters (`00419210` normal, `0041a510` added, kin) skip a pixel where the buffer is black, copy where white, else multiply the child's pixel by it before blending. So a white child shows the mask's picture (ma01_019's `絵Ｍ１/絵白`), an added grey shows as smoke where the mask is light (ma01_021's `絵マスク１/もや`). The converter treats it as a stencil whose cut multiplies colour as well (`Assets::stenciled` with `colour`). A colour mask under a normal blend is exact only when the mask is grey (the exe's per-byte factor is a per-channel opacity); the story's colour masks are ev104_d (white child: exact) and 3d村正蜘蛛_正面. Checked in ma01_021: the smoke follows `cg/data/effect_01_00_0.png` (correlation 0.90 over 9,216 points). Not compared with the original.

### Processes: one-shots on paper (C2, 2026-10-16, #151)

`convert/one_shot.rs` walks the function at the process's start (`if`/`while` decided then, assignments run, 10,000-turn cap: a process waiting on what the script sets later, `ProcessAFB1`/`2`, is noted and its state put back), puts each picture's actions on a clock (`loops.rs` `paper_action`, `Pass::keys_to_end`) and gives it one loop-2 `spmotion` at the next sync: placed where the process leaves it at the highest opacity it reaches, keyed from where it was; once `sound_clock` passes the end it takes its last opacity or is deleted. A process run in line (the end roll) keeps its old path. `CreateProcess` notes 1,756 -> 236, ~800 new many-key motions; the script's waits change by +27 s (a `WaitAction` on a one-shot now waits for it). Real-time trace, 90 s from `*if0_else` (needs the settings' defaults: `--goto` leaves `%2920`-`%2938` at 0, i.e. `gamespeed` 10): 35 new motions, the blood-spray frame flips; 21 removed by the scene's `csp` after 18-43 ms only because auto-play clicked through the `print`.

### Processes: loops behind an `if`, after a lead-in (2026-10-16)

The cockpit meters' `CP_SpeedAuto`, `CP_HighAuto`, `CP_AltAuto` (703 notes) test the meter's value, wait in the process for the move `CP_*Move` started (`WaitAction`), may step once more, then wobble for ever. `loops.rs` decides an `if` the function ends in when the process starts (`split_if`, `ends_in_loop`). Waits in a loop process hold only the process: they add up to `process_lead` instead of holding the script (an in-line process still holds it), and a process's own time no longer lengthens a sync's wait. The loop starts after the lead in one motion with the move (`spmotion` loop 4, CONscripter #48, `MotionEnd::LoopFrom`; `lead_in_loop`). 579 lead-in loops, 792 fewer `delay`s.

### Tempo of `Move` and `Zoom` (2026-10-16)

Read (`tempo` in `nitroplus/convert.rs`); exe curves `apply_tempo` `00449f40` (t the progress 0-1): Axl1/2/3 t², t³, t⁴ and Dxl1/2/3 1-(1-t)², ³, ⁴ are `Op::MoveEase` 1 and 2 with exponents 2-4 (`spcurve` 11-13, 31-33); AxlAuto 1-cos(πt/2), DxlAuto sin(πt/2), AxlDxl ½-½cos(πt), DxlAxl acos(1-2t)/π are eases 4-7 (`spcurve` 70-73, 33 steps, `conv-cons/src/ease.rs`); any other name (`null` too) is straight. The curve goes on the `spmotion` key. A move and a zoom of one picture with different tempos before one sync take the last one's. Not compared with the original under wine.

### Render modes (2026-10-16)

`Request(obj, AddRender|MulRender|SubRender|NormalRender)` sets the blend; a picture placed with one is drawn with `spblend` 1 (added), 2 (multiplied), 3 (subtracted), times its opacity, keeping its place among the sprites (`Op::SpriteBlend`, shared with Django). A blend set on a shown picture changes it in place; `spblend` stays through `lsp`, so a layer a blended picture leaves is cleared first. A movie with `AddRender` (white smoke) is added on; `SubRender` movies keep their darkened re-encode. Dry run: 2,736 `spblend …,1`, 329 `…,2`, 27 `…,3`, 80 `…,0`. Engine check (CONscripter #34): over #808080, #404040 subtracted gives 65, screen 192.

### `SetBlur`

Exe `SetBlur(obj, on, passes, alpha, lag, wait)` (`FUN_00460230`, `FUN_00460530`, `FUN_00456810`) draws `passes` (1-128) copies under the picture at its opacity × `alpha`/1000; when a move, zoom or turn starts they are put on the picture, and copy i shows where it was i × `lag` ms before; a still picture shows no difference. The old Gaussian 1.5 px blur per pass and the `alpha`-ms wait went (1,049 blurred pictures, ~240 waits fewer). Copies as extra converter sprites were tried and dropped (the layers are packed: 849-878 scene re-layerings against 592, or 6,374 copies left out). Instead `sptrail layer,passes,alpha,lag` (engine #25, converter #82) draws the copies following its `spmotion` keys `lag` × i ms late, emitted before each motion of a trailed picture (blocking `Move` included, `spmotion` too since #101): 594 in the story; `sptrail layer` ends it when `SetBlur` is turned off (28). The last argument is no wait: the exe (`00454ca4`) draws the copies before the picture when `true`, after it otherwise, so the story's `false` (955 of 996 calls) puts them over: `sptrail …,lag,1` (engine #83), 659 of 704 trails.

Finished work, moved out of "Status".

**A picture's children (2026-10-16, wine):** a child of a picture is placed from its parent's corner; a window's or a name's child at its own place. `Compiler::parent_offset` adds a picture parent's place for every game (`Game::relative_children`: Django, a container's too).

**Wildcards and children (2026-10-16, exe and wine):** a trailing `*` that matches a window, name or camera doesn't reach its children; one that matches a picture does (the flag at `+0x9d4`). `Delete` still takes a matched container's children (a locked one's too), and deleting a window deletes what is in it. ~5,800 script lines changed, mostly battle scenes.

**Title button sounds late (2026-10-16, reported by the user):** the hover sound (`$TitleOverSE`, `se戦闘_動作_刀構え02.ogg`) opens with 125 ms of silence in the original's own file, so it stays. The rest was the mixer's 4096-sample buffer (93 ms); CONscripter `f33a0c0` uses 1024 (2048 in the web build, which mixes on the main thread).

**Window clip (2026-10-16, reported by the user):** `CreateWindow(name, prio, x, y, w, h, ?)` shows its children only inside its rectangle (the cockpit's speed and altitude tapes, 100×300). Every picture in a window (or several, their overlap) is drawn with `spclip` (`Op::SpriteClip`) at the window's place, which a `Move` of the window moves; named places read as for a picture. Not with `Game::relative_children` (Django unchanged). Known limit: story windows that move (`絵演窓上`, `窓右`, `絵窓上`/`下`: ~56 `Move`s) jump the clip to the end place at the move's start. Pictures under a clipped full-screen picture are no longer culled as covered.

**`SetVertex` (2026-10-16):** `builtin_SetVertex` (`0040ff40` -> `FUN_00447780` -> `FUN_00451e10`) sets the point (`left`/`center`/`right` or px, `top`/`middle`/`bottom` or px, from the picture's top left) that a `Zoom` or `Rotate` keeps in place; it doesn't move the picture. Django's rule (`Game::zoom_vertex`) is now every NSS game's. 1,226 places of `0.txt` change. Checked under wine from a patched `md04_006` (`research/nitroplus/local/patch/vertex*`): `ma03_016` zoomed ×2 about its left middle (RMSE 7 %, 24 % before), `mc01_023vs`'s strip turned -90° about the screen's middle (0.1 % differ), `ma01_022vs`'s sky (0.9 %, 2.6 % before), its far-off 村正 (`SetVertex(…, 512, 288)` on 1400×1000 zoomed to 1 %: now above the screen as in the original). Full build +310 KB, peak memory 1.39 GB.

**Story `CreateText` (2026-10-16):** the 27 calls the story reaches (殺害, 〝我は力〟, `mb03_022`'s screamed lines, date captions) are drawn by the converter into a picture (`system/strip/text_<hash>.png`, `convert/text_object.rs`) in the last `SetFont`'s style with `--font`. An `@` face is Windows' sideways font: upright letters turned a quarter counterclockwise along a line the script turns with `Rotate(…, 90)`; brackets, dashes and ASCII lie along the line. `HEAVY` thickens the letters; `RIGHTDOWN` is a shadow 2 px right and down after the turn; `inherit` for y is the screen's middle; a turn goes about their `SetVertex`. Under wine: same places within ~15 px (wine substitutes ＭＳ 明朝, so sizes can't be compared). The box's height (argument 5) clips (the exe draws into a surface of that size); no story text reaches its 500. Not drawn: the choices' texts (the engine's choice draws them), texts with markup (Django's song in `dja026`), the English build's.

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
