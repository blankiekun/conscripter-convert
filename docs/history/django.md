# 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus NSS): History

Done items moved out of [the game document](../games/django.md)'s Status.

### The whole-route frame diff's five largest groups (2026-10-19)

Triaged from scripts and both traces, no wine; none is a difference in
what is drawn, each was how `frame_diff.py` read the traces, and the tool
now handles each: (1) missing `color:000000:800x600` (414) are `上下枠中`/
`黒板` fades ours draws as screen transitions (kind `screen`); one real
difference left: a `黒板` fade running at the next page is cut by `csp`
(dja003_01:36, at ~60 %). (2, 3) extra `windowframe.png` (174)/missing
`frame03`/`frame04` (89): `CreateTexture(…, "@LoadFrame")` isn't traced
(`--alias LoadFrame=cg/sys/windowframe.png`); y-turns compare drawn width
zx·|cos ry|; 205 findings left (headless ~30 fps). (4) missing
`長押しメッセージN` (40): `mpeghold`'s cell sprite (`--ignore cg/sys/push/`).
(5) extra pre-zoomed copies (~150) are camera children (kind `camera`).
Result: missing 1,656 to 1,132, extra 2,149 to 935; `screen` 411, `camera`
963 (not differences).

### A 0 ms `Move` stops the move in flight (2026-10-18)

The exe stops a running move where it has got to when a 0 ms `Move`
comes (snapshot oracle); `Compiler::stop_move` does so (straight line; an
eased run is elsewhere), as a 0 ms `Fade` already did. Tests in
`nitroplus/convert/tests/scheduler.rs`.

### Timing parity (2026-10-18)

The dry run's `metrics.json` holds `parity` (`--parity-report`): 450 s
short in 76 scenes before the FadeFade fix, 301 s in 74 after. Worst left:
dja048_01 (44 s), dja011_03 (25), dja047_01 (21), dja043_02 (18),
dja027_01 (16): lost or shortened motions, or parallel fades read twice.

### FadeFade after an unwaited motion (2026-10-18)

`FadeFade` (function.nss:887) is `Fade(n,0,0,null,true)` then the timed
fade. After an unwaited `Move` (dja010_01:76-78, dja055_01:99) the 0 ms
fade's sync hid the picture over the move's time and the move ended with
it; now a sync leaves a hidden picture's unwaited move for the next sync
that shows it (only the next: a later one would replay it). Django +34
`spmotion`s; 装甲悪鬼村正 (and English) ~70 motions kept, one more `effect`
time (every `print` renumbered).

### Status as it stood before the 2026-10-18 prune

#### Queue (2026-10-16)

- Save tool: `nitroplus_save.py write` can't send a save to another
  scene ("Save format" has the plan); the original hangs under wine when
  a script `Save` runs from a process and on timed actions.
- Merged: windows clip children (#45), dja026 lyrics (#47), notes ranked
  in Next (#51), song lines type out (#55), `WaitPlay` on a sound (#70);
  open then: disused one-shot sounds kept (#76). `spgrab` (engine #32)
  unused. `RemainTime` on a sound returns its whole length, not the rest
  (`sound_left` has it).
- State then: everything decodes; 249 script files parse (56 places don't,
  none in the story); the story compiles whole (56 chapters, 10 choices;
  good path 6,496 pages; `research/nitroplus/django_routes.sh`); boxes
  within 1 px (box00: 39 columns at 50,472, five rows; `box01` x 250,
  `box02` x 390); cameras: a child of priority p drawn about the screen
  centre at (place − camera's) × z / (1000 − p) (measured under wine);
  system screens in `django/screens.rs` (engine #104, #98; display options
  and version panel left out on purpose); `--roundtrip-every 1` over 569
  pages no mismatch (`BgmLater` music is `mp3save`). Web: saves, sound,
  touch unchecked.

#### Screens drawn by the engine (2026-10-17)

Save/load, text log and extras menu are CONscripter `screen`s as in 装甲悪鬼村正; against main's script routines 0 pixels each. Not tried: hover on a slot, scrolling the log.

#### Accuracy (value audit, 2026-10-17)

Every hand-typed value in `convert/django.rs` carries `// src:`, `//
guess:` or `// ours`. Open guesses: the log knob's travel (top y 90 to
460; `CreateScrollbar`'s 646, 490 to 646, 90 may be the centre), the
click-wait bullet's frame time (50 ms). The original has two values where
ours has one: the save screen opens over 200 ms, closes over 300 (ours
300 both); settings pairs fade over 200, bullets at once (ours at once).

#### Trace diff (2026-10-17)

First 50 pages from dja001 against the original's trace: all pair. Open:
dja001_02:50 `Move(…,1000,…,true)` then `Zoom` (function.nss:877): 419 + 2
ms original, 0 + 300 ours; a 0 ms waited `Move` (function.nss:910) took
618 ms in the original (page 44). Recipe: "Trace diff, first 50 pages".

#### Next

0. Boot logos: the Nitroplus logo's fade-out during its flight is lost
   and its `Rotate` @60 isn't drawn (first-title opening since done:
   "Title opening").
1. Trace checker: "sounds on two channels at once" are the script's own
   (same shot or horse on two objects 500 ms apart) and stay.
2. Extras: film the doors and the rifle beside the original.
3. `SetShade HEAVY` (one use): done (#197).
4. Unread-argument notes (2026-10-16: 33 kinds, 18,947 notes):
   4. **`Fade` 3, tempo** (77 non-`null`): ours linear.
   5. **`Rotate` 5, tempo** (42): the rest are about z or in loops.
   8. **No-change candidates**: `MoveCamera` 6 (15, a camera with no
      children) and `CreateEffect` 2-3 (1, `Sepia` at 0,0) left noted.
5. Sounds kept to their end (`convert/sounds.rs`):
   1. **Time across runtime branches**: the sound clock adds both
      branches of a runtime `if`/`else`; fine as is.
6. `CreateEffect` Rain (37), Drop (4), Lens (2): shared NSS code; drawn as
   nothing.
7. Exe mapping (2026-10-17): `docs/games/django.findings.tsv`;
   `argcensus.py` flags only the script's own values. Fixed: `SetVertex`
   px outside the picture. Open:
   1. `SetVertex` names: exe compares lower case, ours any case.
   2. `CreateColor` `GREEN` is 0x008000 in the exe (`LIME` 0x00FF00), ours
      0x00FF00; other HTML names missing; not in Django's data.
   3. `SetPan` is DirectSound's (dB), `chpan` linear; data uses 0 and
      ±1000 only, where they agree.
   4. `Fade` with four arguments takes the fourth as tempo, not wait; the
      six such calls take 0 ms.

### Title opening (2026-10-18)

The first title of a run plays `TitleSet` with `$Logo` 0
(`Game::title_opening`, compiled twice: `conv_title_script` with `#ClearG`
0, `conv_title_script1` with 1; `TitleOpening::script`): background in over
4.9 s (`Axl2`), tumbleweed from 3 s, shot and logo zoom in over 200 ms,
double shot jolts it (300, 900, 600, 1000 per mille), buttons fly in 50 ms
apart (5.55 s in all; `--real-time` 5,606 ms). Shared NSS code it took: a
number as a timed action's wait (`Fade(…,300,1000,null,50)`) waits that
long with the action going on (Django.exe `004310e0`; Muramasa.exe
`0040e050` → `00445ac0` → `004489e0`, `timeGetTime`; 装甲悪鬼村正 has 56
such calls: 26 shorter cross-fades, 15 shorter `delay`s); a new picture
moving in also turns and fades in with the move when the fade is as long
and unwaited (21 in Django, 125 in 装甲悪鬼村正); a loop process whose
timed actions follow a wait plays them as the motion's lead-in
(`spmotion` loop 4); a picture turning about x/y while it zooms/moves/
fades is one motion. The tumbleweed is not in the title's background
(never at rest there in the original).

### Title on coming back, motions at rest (2026-10-18)

Coming back plays `TitleSet` with `$Logo` 1 (`conv_title_script_back`,
`…_back1`; no music started, the extras' way back sets it): background in
over 1 s, logo zooming from 200 % turning about x over 1.2 s: 2,660 ms
(`--real-time`). At rest the title has TitleSet's processes
(`TitleScreen::motions` from `opening_loops`): each button's `*Shake` (a
jolt every 10-15 s, first at 2, 3, 5, 7 or 8 s) and `TumbleweedMove`
(300,420, every 9 s), continuing the opening's clocks (`spmotionclock`;
the title clears only sprites 62-999, not `csp -1`). Shared NSS code: a
loop process with a fade-in lead-in on a picture not drawn yet, then an
instant `Fade`, holds that opacity in the motion's keys (no change in
装甲悪鬼村正).

### Boot logos (2026-10-18)

`boot.nss`'s loop: `InitBGM`, `TitleCongratulations`, then `TitleLogo`
while `$Logo` is 0 (first title of each run; no save flag): `media_a.jpg`
in 500, `WaitKey(2000)`; `media_b.jpg` cross-fades in 500, `WaitKey(2000)`,
out 500; `Wait(500)`, `WaitKey(1000)`; music `dj23_harmonica`;
`nitroplus.jpg` in 1000, `Wait(3000)`, gunshot, logo shot away in 500.
Converted from the script (`Game::boot_logos`, a `BootStep::gosub` once a
run); the music goes on into the title. `--real-time`: music 7,027 ms
(script 7,000), shot 11,036 (11,000), title 11,536 (11,500).

### Sounds in the source map (2026-10-18)

A sound's ops now take the statement that called the helper holding the
`Play` (`Compiler::played_at`), not the one running at the flush
(TitleLogo's `SoundPlay` boot.nss:153 got the `if` at 140). 0.txt same.

- **Unwaited fades run on** (2026-10-17): the exe's `Fade` (`cb_00438fb0`, Muramasa.exe `0044f020`) doesn't block (dja001_01:126-128: text 100 ms later, 240 under wine). A new picture faded in longer than the sync waits is placed and faded by the engine (`spmotion` alpha keys), `WaitFade`/`WaitAction` waits (`spmotionwait`); 装甲悪鬼村正 and English change too.
- **The text window fades between blocks** (2026-10-17): `TypeEnd()` fades `@box*` out 100 ms and waits (function.nss:510), `TypeBegin` in 100 ms (502-503); a waited fade leaving the text at 0 is `windowfade 0,ms` (text drawn at window opacity × its own, `drawn_opacity`). 装甲悪鬼村正 unchanged.
- **`SetAlias`, one per object** (2026-10-17): the exe (`00432da0`, Muramasa.exe `004475b0`) keeps one alias slot per object; a new alias drops older ones.
- **`Draw`** (2026-10-17): the exe's `Draw` (`0040ba10`, Muramasa.exe `0040f470`) waits at most 4 ms; now a sync (`UNWAITED_MS`, 1 s). dja019_02's camera move still starts 500 ms late (fixing it loses the 280 px pan).
- **`Wait` and clicks** (2026-10-17): the exe's `Wait` (`0040bd80`) ends only on quit, state change or skip, not a click: now `wait` (ended in skip mode, engine #134); `WaitKey(ms)` stays `delay`.
- **The extras' doors, music and rifle (2026-10-17).** From `extra_menu.nss`/`extra_cg.nss`: doors (`ExtrasPassage`, `spmotion` zoom about each centre, t² in and out), menu music (`bgmfadein 3000`), the rifle (`CategoryHover`). Against main's build: menu 0 px, gallery differs only in the rifle's 50×52 box at 252,442.
- **`Shake` 6 and 7, `CreateWindow` 6 (2026-10-17).** From the decompile
  (`Django.exe` 1.01). `Shake(obj,time,x0,y0,x1,y1,n,tempo,wait)`
  (`FUN_00431de0`, frame `FUN_0043f570`): `n` above 1000 does nothing, not
  even the wait; drawn at ±(x0 + (x1 − x0) × curve(t)), sign flipping (+
  first) every 1000 ÷ (`n` × 0.001 × fps) ms; fps 60 at default speed
  (`FUN_00469a20`: 960, 480, 240, 120, **60**, 50, …; 1, 2, 4, 8, **16** ms
  a frame); curve = the tempo's table (`FUN_00434ea0`; `Dxl2` = 1 − (1 −
  t)³) over time × 0.06 frames. Inferred: other tempos are Muramasa's; `n`
  0 flips each frame. Ours: full-screen `Shake` is `screenshake` mode 2
  (engine #79) every 17 ms (`n` 1000) to 83 ms (`n` 200); a picture's
  `spmotion` flips at that step. `CreateWindow(name,priority,x,y,w,h,
  relative)` (`FUN_0042fee0`): anything but `true` clears the object's
  +0x3c8 (1 on new objects), and its children's places count from the
  screen, not the window (not the clip). No `0.txt` change.
- **Calls that change nothing, and calls left out (2026-10-16).** `UNREAD_HARMLESS` (`convert/builtin.rs`): arguments whose value does what ours does; `LEFT_OUT` (`convert.rs`): names no function has (the exe skips them) and `Conquest`. Gaps 9,411 → 1,554; `Random` (returns 0) and `function_select.nss`'s choice built-ins stay gaps.
- **Motion cases, all shared NSS code (2026-10-16), each now a real
  motion instead of a cross-fade.** Movies moved/zoomed (a movie's zoom is
  its cell's `spzoom`; `spmotion2` keeps it playing). Pans over a picture
  cut to the screen (`Assets::scaled`; `Shown::whole` keeps the whole
  picture's place, zoom in keeps the uncut start, zoom out places the
  uncut end and runs from the cut start; #175). Fading moves in a spread,
  and uneven zooms (dja023, `uneven_motion`). Moving pictures spread with
  no room keep moving on their new layer; running motions go on across a
  re-spread (7 of Django's 8; new pictures in a moving camera get their
  neighbours' motion). A picture replaced or turned as it moves is placed
  where the old was and moves from there (no `spgrab`). A move that fades
  out to 0 stays for one motion with alpha keys until its time is up
  (`Compiler::vanishing`; only a timed `Fade` counts, `pending_fade`). A
  move onto another layer is placed at its end and runs from where it
  was. A move that also fades is one motion: the sprite is set to the
  larger opacity (`fade_base`) because the engine multiplies a key's alpha
  by the sprite's, and conv-cons drops the `print 1` before it. Cross-fade
  notes 133 → 59.
- **`Rotate` about x or y over time (2026-10-16, `convert/turn.rs`).** Face windows (`WindowRotateR`, `WindowCloseR`: y by 90°, `Dxl1`) and choice plates (343 calls): drawn squashed (across |cos y|, up/down |cos x|, about vertex/centre, children along, no perspective); over time an `spmotion` with a key each ~10°. A turn passing edge-on midway is still mirrored at once (10 left in 装甲悪鬼村正).
- **Sepia and tints (2026-10-16).** `CreateEffect(…, "Sepia")` (dja038_03) is `Kind::Tint`: `spfilter` greyscale then multiply by cream (255,235,195) on pictures below its priority; the exe's sepia code is encrypted, so the tone is ours. `Monochrome`, `NegaPosi`, `KitanoBlue` (guess) likewise; Rain, Drop, Lens noted.
- **A face slides with its frame; a movie turned over plays mirrored (2026-10-16).** A `Move` over time takes `relative_children` along. `Rotate` y 180 on a movie (12 calls): the exe writes the angles into the movie's quad (0x439590) as for a picture, so it mirrors; ours plays `movie/<name>_x.mp4` (ffmpeg flip).
- **Easing (2026-10-16).** `Move`/`Zoom` tempo (arg 4) and `MoveCamera`'s (arg 5) ease by the exe's curves since #112 (`Axl`/`Dxl` 1-3 powers, four sine tempos `spcurve` 70-73).
- **dja055's chorus (2026-10-16).** 15 looping voice parts (`CreateUnit`), panned and faded one by one: compiled as sound-effect objects (`dwaveloop`, `chpan`, `chvolfade`; they follow the sound-effect volume); `SeChannels` keeps a looped sound's channel and uses 11-49 when 1-9 loop.
- **A fade to 0 stops the sound at its end (2026-10-16).** The exe's `SetVolume` (`FUN_00457dd0`) fade thread (`0x459980`) stops a playing sound at 0 (`FUN_00458420`); `WaitPlay` right after now waits for the fade; a locked sound is never forgotten.
- **Render modes (2026-10-16, engine #34).** `Request(obj, AddRender|
  SubRender|NormalRender)` is `spblend` (Muramasa's History, "Render
  modes"): 15 `AddRender` uses; the one `SubRender` is a movie, which
  keeps its darkened re-encode.
- **`SetPan` (2026-10-16, engine #35).** The exe's `builtin_SetPan` (`0040d140`) pans over time; `chpan ch,pan[,ms]` pans a `dwave` channel (saved, not music), emitted before a sound that starts panned and re-centred for the next.
- **A small picture shakes alone (2026-10-16).** The exe's `shake_frame` moves the object's own place ±(x, y) per step; a non-full-screen picture gets a one-shot `spmotion` (`convert/shake.rs`); a `Move` at once replaces it.
- **Disused one-shot sounds stay until their end (2026-10-16).** The exe (`FUN_00436040`) deletes a disused object only once its sound stopped; `Request(…, Play)` on a playing one does nothing (`FUN_00457d20`). Kept until the build-time end so `WaitPlay`, `Fade`, `SetVolume` act.
- **`WaitPlay` on a sound waits (2026-10-16).** The exe's `builtin_WaitPlay` (`0040b8a0`) waits until the sound stops, at most its second argument ms; now `delay` for the sound's rest. The ten bad ends' `サウンド４`: 2,500 ms.
- **The song types out (2026-10-16).** In the exe's `CreateText` layout `.` pauses 100 ms (`,` 500). Under wine a letter every 125 ms, the full-width space included. `django/song.rs`: a strip of the first 0, 1, … letters (`lsp ":a/N,125,1;…"`), `WaitAction` waits for the last; 125 ms whatever the text speed.
- **The song in dja026 (2026-10-16).** 13 `CreateText` lines, two at a time. Under wine letters alone, white, centred on the screen (the 15 offset doesn't show), ink 16 px below their place; ours (`text_object.rs`, nudge 15 px left, 13 px lower) is within 2 px. A `CreateText` picture stays hidden from `Erase` to `Enter`.
- **Windows clip their children (2026-10-16).** `CreateWindow` draws children only inside it (`spclip`, #27), the place counted from its parents' (`窓/中窓` at 0,70 in `窓`; dja002_02 under wine: stops at y 70). Windows still off-screen draw nothing; a window seen through a camera doesn't clip; a sliding window's clip is its end place.
- **Text windows and face windows compared under wine (2026-10-16).** dja001_02, dja003_02: text within 1 px; face windows within 410 pixels. Fixed: faces were one object (`Game::string_vars` on; `SetAlias`'s second argument naming a variable is its value); a face takes its frame's opacity; a picture edge-on (90°/270° about x/y) isn't drawn, nor its children; a trailing `*` doesn't reach into children (`Game::family`). Under wine the original stalls at random in timed actions and `WaitPlay`: the patched boot makes them instant.
- **The ending roll's credits (2026-10-16).** `SetRoll` (dja056_09) fades to red (#c80000) over 2 s; `cg/sys/good/EDROLL.jpg` (800×9000) at y 300 moves up to y −8465 over the insert song's time left (`RemainTime`) + the closing song (`dj_INS11`, 3:31) + 6 s; first time (`#ClearG` 0) it waits for a click, later a click ends it at once; after it music fades out 6 s, picture 4 s. `django/roll.rs`: one `Op::MusicRoll`; `getbgmpos` (new) reads the song's position, closing song by `bgmlater …,0`, `amsp` at `gamespeed 100`. Headless 379 s; speed against the original unchecked.
- **Looping shakes and the strobe (2026-10-15).** `ShakeVLoop`, `ShakeVLoop01`-`05` (9 processes): the background gets a looped `spmotion` (±y every 100 ms, `convert/loops.rs`, shared); a move over time replaces it. `strobe` (dja050_03, 3): `Kind::Flash` 800×400 at (0,70), 300 for a frame, out over 80 ms, forever. Not compared under wine. A loop process runs at its `Request(…, Start)`.
- **The choice's shot (2026-10-15).** `ChoiceA02`/`ChoiceB02`: bullet hole (`dankon.png`) with top left 47, 41 above and left of the pointer, 300 ms later the other plate fades (300 ms), 500 ms after that the chosen plate and text turn −90° about x in 800 ms (`Axl2`) fading over 1000 ms (`ChoicePlates::shot`, new `spcut`). Not compared under wine.
- **Auto mode stops at a choice (2026-10-15).** The scripts set `$SYSTEM_skip`/`$SYSTEM_text_auto` false before each choice (kept only with `#keep_auto_and_skip`, off by default) and `$SYSTEM_menu_lock` before the roll: `btnwait` ends auto mode, the converter emits `skipoff:autooff` (`Op::StopAutoSkip`; Muramasa too).
- **Muzzle flashes (2026-10-15).** `GunFire(times, ms, opacity)` (11 calls, dja050): white 800×600 at priority 150, added on, `opacity` for a frame, faded over `ms`, black for a frame, `times` times (`django/battle.rs`: `Kind::Flash`, `spblend`, `spalpha`). A frame is 17 ms; not compared under wine.
- **Movies as sprite movies (2026-10-15).** Movies playing while the script goes on were frame strips (75 MB each); now `Assets::movie_sprite` (`movie/<name>_mute[_dark][_alpha].mp4`, `lsp` `:c/1,0,0;` looping, `:c/1,0,1;` once; alpha = inverted opacity as the right half).
- **The 400-vs-3 counter (2026-10-15).** `function_count.nss`'s counter (194×52 window at 296,10, digits from `count.png`) is drawn by `django/counter.rs` as strips (33 ms; `Kind::Strip`); from the 380th man the digits fade out over 2 s. Under wine `CounterPlay(7,0,3,1000)` reads 007, ours matches pixel for pixel.
- **The 400-vs-3 battle's movies (2026-10-15).** dja050's six `VS400` calls play a movie while the button is held (release pauses), the hold prompt (`PushShow`) over it, its gunfire sound with it. The user chose to keep the holding: `mpeghold` (Linux/macOS/web; mouse, touch, Enter, Space), `Op::HoldMovie`.
- **Music processes (2026-10-15).** Four `CreateProcess` functions only play music on a timer (dja050/dja056 themes, dja056_08's insert song, dja003's jingle): run at `Request(…, Start)` on a clock, `bgmonce`/`bgm` or `bgmlater ms` (`nitroplus/convert/process.rs`).
- **Music after dja009 (2026-10-15).** dja009's `Delete("*")` deleted the music objects (30 chapters played none); it deletes only the chapter's own objects (`Game::chapter_objects`).

### Trace diff, first 50 pages (2026-10-18)

Original's trace (`ntrace.sh`, wine section) against ours, first 50
pages from dja001 (`docs/new-game.md` step 23); all pair; 50 lines left
(`missing` 145, `extra` 9, `time` 21). Not findings: page 1's boot (the
original's patched boot has no logos, so ours' `boot.nss:128-171` and
`TitleSet`'s opening are `extra`); `call_chapter` lines; 0-72 ms gaps;
page 18's `WaitFade` (auto mode's reading time). The exe's waits poll
(helper `00433bf0`: a 100 ms wait on an event set only by requests, then a
32 ms sleep): a waited action returns at the first poll after its end,
under wine 100 → 225 ms (every `TypeBegin`/`TypeEnd` box fade), 300 → 431
(function.nss:877), 500 → 532-622 (911, 948), 1000 → 1026-1031; ours ends
with the action. Whether Windows gives the same steps is unchecked; not
modelled (it would slow every NSS game's text window ~125 ms a fade): the
user's call.

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run --source-map out/django.map
research/tools/scratch.sh <a full build> <scratch> out/django-dry/0.txt
cp out/django-dry/ons_args <scratch>/   # the dry run's encoding, not the build's
<scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/django.map --pages 50 --ms --thread 0
```

### One `*conv_boot_end` (2026-10-18)

The boot subroutine's label clashed with conv-cons's boot guard end
(`*conv_boot_end`, defined twice); it is `*conv_boot_body_end` now.
