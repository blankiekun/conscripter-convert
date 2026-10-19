# 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus NSS)

Zoku Satsuriku no Django: Jigoku no Shōkinkubi (Nitroplus, 2008), the
fourth game. It uses the same NPA archives and NSS script language as
装甲悪鬼村正 (`docs/games/muramasa.md`, the reference for the engine),
in an older version (`NitroplusSystem2.0`), so it reuses
`crates/conv-engines/src/nitroplus/`, with its own rules in
`nitroplus/convert/django.rs`. The general process is in
`docs/new-game.md`, general lessons in `docs/lessons.md`.

- **Status:** started 2026-10-15 (see "Status" below). Branch `django`.
- **Game files:** `<game>` is the install folder holding `Django.exe`,
  the `*.npa` archives and `movie/`, **with the 1.01 update applied**
  (`DjangoUpdate101`: its `Django.exe`, `nss.npa` and `cg2.npa` replace
  or join the installed ones). Version 1.00's scripts differ in about 60
  files. To keep the install untouched, make a folder of symlinks to the
  install's files with the update's three files linked over them.

## Scope

Manual: none found in the install (`Django.exe`, `.npa` archives, mp3s).

Asked 2026-10-15: the story first, the web build as the main platform,
and the extras (CG library, recollection, sound library) wanted too.
Running the original under wine with a no-CD exe the user owns is
allowed.

## Build

`conscripter-convert build <game> -o <output>` recognises the game and
runs `convert-nss` with `--title 続・殺戮のジャンゴ` (README, "Converting a game"); the
convert options (`--dry-run`, `--font`, …) go after `-o`.

Script only (a few seconds; for anything about logic):

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run
```

Without `--file`, `convert-nss` compiles Django's whole story
(`Game::story`: `dja000_開始スクリプト.nss`, scene `SelectStoryMode`).
The full build adds `--font <a Japanese TTF, e.g. ipag.ttf> --title
続・殺戮のジャンゴ` and drops `--dry-run`.

**Web build** (loose files; needs CONscripter 4d94b92 or later, which
finds loose files with Japanese names):

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-web \
    --font <ipag.ttf> --title 続・殺戮のジャンゴ --loose --webp --webp-quality 80
python3 -I engine/src/cons_web/conscripter_index.py -i out/django-web \
    -o out/django-web/cons_index.json --title 続・殺戮のジャンゴ --lazyload
cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/django-web/
python3 -m http.server 8000 -d out/django-web   # then open /conscripter.html
```

680 MB in 5,705 files (2026-10-15). Checked in headless Chromium: the
title, and with `?trace&auto-play=40` the story's first pages with
their pictures, face windows and text.

**Checking the routes.** After a full build into `out/django-cons`:

```sh
research/nitroplus/django_routes.sh out/django-cons <conscripter> out/django-routes
```

writes a `--choices` file per path (`django_paths.py`: the good answers,
or the good answers up to choice N and then the deadly one), plays the
eleven paths in parallel up to the story's end (`--until conv_run_end`)
and prints each run's pages, choices and the bad-end pictures it
showed: the good path none, `badNN` only its own.

## Files and formats

| File | Size | Contents |
|---|---|---|
| `Django.exe` | 3.1 MB (1.6 MB for 1.01) | The engine, `NitroplusSystem2.0` |
| `nss.npa` | 756 KB | 249 `.nss` scripts (1.01; plus SourceSafe's `vssver2.scc`), Shift-JIS, CRLF |
| `system.npa` | 1.4 KB | `system.ini`: 800×600, font ＭＳ Ｐ ゴシック 18 px (white, black outline), ruby 45 %, BGM 700 / SE 900 / voice 1000, click icons (`cg/sys/icon/line/Tumbleweed-%03d.png#20`, `page/Bullet-%03d.png#20`), voice names |
| `cg.npa` | 465 MB | 1,817 files: 1,233 PNG, 543 JPEG, 40 BMP, one WAV (`sys/config/test.wav`). Folders: `sys` 588, `bu` 471 (standing pictures), `bg` 271, `ev` 141, `thum` 130, `st` 120, `kw` 94 (face windows) |
| `cg2.npa` | 1.6 MB | 1.01 only: 119 music-room pictures (`sys/extra/sound/kyoku/`) |
| `sound.npa` | 178 MB | 628 Ogg Vorbis: BGM (`bgm/djNN.ogg`) and sound effects |
| `voice.npa` | 217 MB | 3,516 Ogg Vorbis |
| `movie/` | 140 MB | 35 MPEG-1 program streams without extension (800×600 or smaller, 29.97 fps, MP2 audio): 17 `mpg_*` (the opening, battle scenes), 18 `mv_*` (muzzle flashes, smoke, blood: effects played over the scene) |

Every image and every sound decodes (checked 2026-10-15 with PIL and
ffprobe over the whole corpus); no empty or 1×1 files.

**Screen size 800×600.** Text is horizontal.

**NPA**: the same format and key scheme as Muramasa's (`npa.rs`), with
another 256-byte substitution table, which `Django.exe` doesn't hold as
plain bytes. `npa::DJANGO_TABLE` was recovered by known plaintext from
the fixed bytes of the Ogg headers of `sound.npa` and `voice.npa`
(`python3 -I research/nitroplus/npa_table.py <game>/sound.npa
<game>/voice.npa`): every slot filled, no contradiction. The CLI picks it
for `Django.exe`. Tools: `conscripter-convert npa list|extract
<game>/Django.exe <archive>`.

## Executables

The builds the decompile addresses in this document belong to.

| File | Version | sha256 |
|---|---|---|
| `Django.exe` 1.01 (`DjangoUpdate101`, 1.6 MB) | 1.01 | `b40fb35cacc672c5ea377ab86e5358fbe85b2a05c30edf5981fd509c5fc390e7` |
| `Django.exe` 1.00 (installed, 3.1 MB) | 1.00 | `d4bed22dd7062880b96476fe423f495cc1872bfb6a3b17c593c8313976040537` |
| `Django.exe` 1.01 unpacked (no-CD, 0.9 MB; "The exe") | 1.01 | `3dbfe8f30e07a244ad294c80c0457b87bc3b1a49af4b6b0c86bf7140d63bc7bd` |

## Script language: what differs from Muramasa

Checked with `conscripter-convert nss check <dir>` and `nss refs <dir>`
on the extracted scripts.

- **`cut name { … }`** blocks, called with **`call_cut name;`** (in the
  same file): parsed as a scene and a `call_scene`. Every story file is
  `chapter main { InitChildChapter(…); call_cut main_stage; }` plus its
  cuts.
- **`wait 1000;`** without parentheses (once, the ending): `Wait(1000)`.
- **Pages joined by a commented-out block**: `{/*</PRE> … <PRE
  box00>[text011]*/}` inside a text block (the writers merged pages);
  the parser skips `{/* … */}` when it looks for `</PRE>`.
- **`TypeBegin("@box00", "@text005")` comes before** the text block it
  shows (`Request("@text005", Enter)`), where Muramasa's `TypeBegin…()`
  comes after. The compiler keeps the Enter and shows the block named
  `[text005]` when it runs.
- **Helper functions** are in `function.nss` (included by `boot.nss`),
  `function_select.nss` (the choices) and `function_badend.nss`, which
  only the chapter files include: function lookup also searches the
  current file's own `#include`s.
- `function.nss` has a `cut layer_number` that is never called: the
  developers' notes of the layer numbers (frames 10000-10001, text boxes
  `box00`-`box05` at 10300-10350, face windows `frame03`-`frame05`,
  backgrounds 10/20, standing pictures 1100-1500, choices 10400/10410).
- What still doesn't parse (56 places): the system screens
  (`sys_backlog`, `sys_load`, `sys_save`: `if(…)){`, `%=`), the notes
  above, a stray `#base_path`. None is in the story.

## How the compiler is set for Django

`nitroplus/convert/django.rs` (`DJANGO`) sets what differs from Muramasa:

- 800×600, the boot script `boot.nss` (its includes are the helper
  library), the whole story compiled from `SelectStoryMode`.
- **Children are placed from their parent**: `@frame03/顔窓31` at 25,25
  in `frame03` at 25,80 is at 50,105 (the comment in `SetWindowKwL` says
  so).
- **`CreateTexture` from a `LoadImage` picture** draws it: the face
  windows' wanted-poster frame (`cg/sys/windowframe.png`).
- **A `Zoom` keeps the `SetVertex` point in place**: `SetBu` sets a
  standing picture's to its bottom centre, so a picture zoomed to 700
  still stands on the bottom of the screen.
- **Children follow their parents** (`Game::family`): `@移動窓/立絵40`
  is a child of `窓/移動窓` (`移動窓` being its alias), and deleting an
  object deletes its children; a `*` before the end of a name
  (`@選択肢*/MouseUsual/*`) matches any characters, the first part
  aliases too (the choices' hover and pressed plates were drawn over
  the scene, the script's fades to hide them matching nothing). Without it, pictures hung under a deleted
  window stayed up for whole chapters (two extra characters at
  dja004's first choice; image memory peaked at 131 MB, now 109).
- **A chapter file's own objects are deleted when it ends**, except those
  hung under an object made before it. From the scripts: `function.nss`
  makes `窓` and `音` windows "to hang pictures and sounds on between
  files", and `dja001_…_01` ends under its fade to black `黒幕`
  (priority 20, never deleted) while `dja001_…_02` draws its first
  background at priority 10 and fades in with another object (`@黒板`):
  had `黒幕` stayed, the whole file would be black. Under wine a box made
  at top level in `_01` stays up to the file's last line; the step to
  `_02` wasn't filmed (the original stalls at random under wine; probe
  in `research/nitroplus/local/chapend_probe.py`, git-ignored).
- **System screens** (`django/screens.rs`): what is ours rather than
  the original's is written in each function's comment (where the log's
  text sits, the hover colour of the choices, the save screen's
  see-through parts for what Django lacks).
- **Choices** (`choice_style`, `ChoicePlates`): `SetChoice02`'s 530×68
  plates one under the other from (138,244) as picture buttons
  (`select002` under the pointer), the options in white 22 px text
  centred about x 388, 18 px below the plate's top: within a pixel of
  the original under wine.

General NSS fixes made for Django that every game gets: hex numbers
(`0x000000`), a string naming a variable as `CreateSound`'s path
(`"$場所指定"`), a voice `src` without its `voice/` folder, `CreateWindow`'s
place and the six-argument `LoadText` (a box's text layout), a runtime
`if` whose branch ends in `return`. CONscripter: a new page erases the
last one's text where the window picture doesn't reach (Django draws its
text on the scene, with no window picture).

## The exe

The 1.01 update's `Django.exe` is packed (every section encrypted);
Ghidra reads the unpacked 1.01 exe, the no-CD one used under wine:

- The packed exe: five sections, four unnamed and the last `.text`
  holding the entry point (0x57d000), each at 7.3-8.0 bits/byte; the
  stub's strings are "Application cannot be run with debugger or
  monitoring tool(s) loaded!" and the disc check's "[1103]"; 1.00 has a
  `.settec` section (SETTEC's disc protection). No public unpacker for
  it here, and its disc check runs before the game's entry point, so a
  dump under wine stops there without the disc.
- The unpacked one the user owns (`ver1.01_プロテクト障害対応`, sha256
  `3dbfe8f3…`; `ver1.01` `e1c690ef…` has the same `.text` byte for
  byte) is that image dumped at its entry point 0x4b4726, imports
  rebuilt: `.text`/`.rdata`/`.data`/`.data1`/`.rsrc` at 5.7-6.7
  bits/byte. Its addresses are the ones in `django-names.tsv`; the
  trace DLL hooks it (Running the original under wine).

```sh
research/tools/ghidra/ghidra_export.sh <unpacked 1.01 Django.exe> \
    research/nitroplus/local/ghidra-django --names research/nitroplus/django-names.tsv
```

(about 2 minutes the first time). The names file holds the
built-in table, the evaluator, the settings setter, the `Request` and
text-tag parsers and the pointer-only callbacks; what they do, and how
the engine differs from Muramasa's, is in `research/nitroplus/builtins.md`
("Django.exe"). The decoders: `python3 -I research/nitroplus/dispatch.py
<export>/decomp/00404320_*.c` (built-ins), `evalnames.py` on
`0042b150_*.c` (values), `stackstrings.py <export>/decomp` (the
`$SYSTEM_` names), `charchains.py <export>/decomp` (tags, `Request`
actions, constants).

**Coverage** (2026-10-15; counts from `nss builtins` and a grep of the
scripts, notes from the dry run). Everything the scripts call is in the
exe. Where the exe and the converter agree, in short: the 69 built-ins
the scripts use but those below; the text tags `voice` (3,576), `PRE`,
`RUBY` (245), `K` (19), `I` (14); `Request` `Start`, `Stop`, `Play`,
`Pause`, `Resume`, `Enter`, `Erase`, `Lock`, `UnLock`; `NoLog`,
`PushText`, `NoIcon` (18), which the exe doesn't know (they do nothing,
as in ours); `$SYSTEM_low_thread_priority` (12, thread priority only),
`#SYSTEM_play_speed` (13: the text speed, set to 3 on the title and the
ending and put back); `continuation` (162, only in `//` comments).
Where they differ:

| Feature | Uses | The exe | The converter | Dry-run note |
|---|---|---|---|---|
| The choice's shot (`function_select.nss`: `CreateMask` ×3, `CursorPosition`, `Rotate` −90° over 800 ms about the bottom, a shot sound) | every choice (10) | a bullet hole where the pointer clicked, the chosen plate tips over | the same (`spcut`, `spzoom` steps; CONscripter after 0.24.0); not compared under wine | `CreateMask` 30, `CursorPosition` 20 |
| `Request` `AddRender` / `SubRender` / `NormalRender` | 15 / 1 / 3 | blend mode additive / subtractive / normal (`request_render`) | `spblend` 1 (added on), `NormalRender` 0; the `SubRender` movie darkened (History, "Render modes") | `addrender` 11, `normalrender` 4 |
| `Request` `Smoothing` / `UnSmoothing` | 20 / 3 | texture filtering on / off | nothing | `unsmoothing` 3 |
| `$SYSTEM_skip`, `$SYSTEM_text_auto` = false at each choice | 25 / 31 | skip and auto mode stop before the choice | `skipoff:autooff` where `$SYSTEM_text_auto` goes false; the choice's `btnwait` also ends both (CONscripter after 0.24.0) | none |
| `$SYSTEM_menu_lock` = 1 (the title, the ending roll) | 8 | no right-click menu, skip off, the text window shown | `skipoff:autooff`; the roll's wait is a `click`, which takes no right-click | none |
| `SetShade(…, HEAVY)` | 1 (dja048) | a blur preset | done (#197: an `spfilter` box blur) | |
| `CreateEffect(…, "Sepia")` | 1 (dja038) | sepia over the screen | not done | `CreateEffect Sepia` 1 |

The system screens' tags (`FONT`, `INTERVAL`, 28 and 7, in
`sys_save`/`sys_load`/`sys_config`) and built-ins (`SetNextFocus`,
`SetBacklog`, `LoadFont`, `LoadColor`, `Save`, …, the rest of the
dry-run list) belong to screens the converter draws itself
(`django/screens.rs`).

## Running the original under wine

Used to compare screens and text placement (`docs/new-game.md` step 9).
The 1.01 exe asks for its disc ("[1103] オリジナルディスクを入れて…"):
use a no-CD 1.01 exe the user owns, copied into the scratch game folder
(never over the install).

- The silent, off-screen setup of 装甲悪鬼村正's recipe
  (`docs/games/muramasa.md`): a scratch `WINEPREFIX` with real user
  folders, an Xvfb display of its own, `Graphics=x11`, ALSA's null
  device, `LANG=ja_JP.UTF-8`. `C:\django\` is a folder of symlinks to
  `<game>` (with the 1.01 files).
- **Run it in wine's virtual desktop** (`wine explorer
  /desktop=django,1024x768 "C:\django\Django.exe"`): the 800×600
  window sits at (0, 0). Windowed on a bare Xvfb, or full screen, the
  window shows twice and loses the clicks.
- **It only advances while it gets input**: under wine every timed
  fade, move and page stalls until the next window message. Nudge the
  pointer a pixel and back about 30 times a second while it runs.
- **Timed actions hang it, at random** (zooms always; moves, fades and
  turns now and then; `WaitPlay`, the null sound device never ending):
  a patched boot with CHAPTER and SKIP (below) makes them take no time
  from the scene SKIP names on and leaves out `WaitPlay`, so end states
  can be compared. Start again when a run stalls. The cause (2026-10-17,
  below): `SuspendThread` on a thread waiting for a mutex; `ntrace.dll`
  works around it (a `WaitPlay` on the null device not tried with it).
- Clicks: `xdotool mousemove X Y`, then `mousedown 1; sleep 0.15;
  mouseup 1` (a bare `click` is often lost).
- **Patched boots**: `research/nitroplus/django_patch.py <table> <game>/nss.npa
  nss-patch.npa [CHAPTER [SKIP]]` writes an `nss.npa` without the logos,
  with every ending marked seen (the title shows EXTRA and LINER NOTES),
  and with CHAPTER, START plays from that chapter file (`dja004`), or
  from its call naming SKIP (`_05_`). `<table>` is `npa::DJANGO_TABLE`
  as a 256-byte file; `npa.py` takes it in place of the exe. Link the
  copy as `nss.npa` in `C:\django\`.

- **Trace speed**: `SPEED=8` before `ntrace.sh` plays 0.9 s a page
  instead of 7.0, `ms` in game time (93 % of steps within 2 frames of a
  1x run; the text decoration's `WaitAction`, `CreateTexture` and
  `CreateMovie` are not scaled: `docs/new-game.md`, "Trace speeds").
- **Trace DLL** (`docs/new-game.md` step 22; 装甲悪鬼村正's tool):
  `GAME=<game, 1.01 files over the install> EXE=<the unpacked 1.01
  Django.exe> research/nitroplus/tracedll/ntrace.sh -o orig.jsonl
  --pages 50` plays from the start of dja001 by itself (about 6 minutes
  for 50 pages) and writes `cmd` and `page` records as Muramasa's. The
  patched `nss.npa` (table from `npa.rs`) skips the logos and the title
  (boot.nss `$Logo = 1`, `$TitleSelect=1`) and turns auto mode on
  where `SelectStoryMode` unlocks the menu (dja000); no line moves.
  `ntrace.dll` hooks the unpacked exe (addresses in `ntrace.c`):
  run_statement's call of dispatch_builtin (`0x453cca`), and for the
  pages builtin_LoadText's layout of each `[textNNN]` (`0x40a3a0`) and
  `Request(text, Enter)` (`request_enter`, called at `0x43d3d7`,
  `0x43d6d8`): a block's pages are logged when its text is entered, at
  its `<PRE>` line (the converter's source map does the same). Two wine
  fixes in the DLL: the main loop's `GetMessageA` gets a `WM_NULL`
  every 10 ms without a message, and a thread waiting for the object
  mutex (`0x52985c`) is never suspended inside that wait (the game
  pauses its NSS process threads with `SuspendThread`, `00466010`;
  wine handed the mutex to a suspended waiter and every thread then
  waited for it: the random hangs on timed actions above, seen with
  `winedbg` and a log of who took the mutex). With the pointer nudged
  as above, 50 pages ran without a click (2026-10-17); should it stop
  for 60 s, the script clicks.
- **Per-frame state trace** (2026-10-19): `ntrace.sh --frames` writes an
  `obj` record each frame for every shown object whose state changed:
  x, y and alpha as drawn that frame (render record +0xc/+0x10, +0x14); zoom
  and rotation as drawn too (the texture record at render +0x20: zoom
  +0xb4/+0xb8 per mille, rotation +0xa8..+0xb0, as the Zoom and Rotate
  frame callbacks 0x43eec0 and 0x43f190 write them); the creating call (+0x180) on its
  first record; `gone`. It hooks the frame thread's call of step_objects
  (`0x435d4b`). A probe checked it: Move 500 ms in 30 frames, then the
  Fade frame by frame, Zoom 1500/2500 and Rotate 33 as asked. Played
  scene by scene from saves: `scene_saves.py --at end` puts the next
  `Load` at the end of each scene's entry block. Three scenes
  (dja001_02-04, 268 pages with `--clicks 700`, 400 s at 1x) gave 36,449
  `obj` records (4.6 MB). `research/tools/frame_diff.py` compares them with
  our `--real-time` trace's `sample` records. Pages pair by the
  instruction that shows them; motions match by picture (NFKC: the
  archive's `bu宇宙紳士ｃ` is the script's `bu宇宙紳士C`) in the same
  page window or the next or previous one. Our pre-zoomed copies compare
  by how far they move. With the drawn zoom (a 130-page run): all 130
  pages pair; missing 48, extra 213 (168 of them pre-zoomed copies still
  unmatched), end 29, path 33, duration 24, zoom 4, copy 13. The whole
  route runs overnight (2026-10-19, `scene_chain.py --quiet 120`).
- **The whole route, frame by frame** (2026-10-19): all 159 story scenes
  played from saves (`scene_saves.py --at end`, `scene_chain.py --quiet
  120`, `ntrace.sh --frames --clicks 700`, 1x, capped at 1.5 GB). It took
  42 segments, every scene reached, about 5 h 20 min, 9,380 s of game
  time, and 781 MB with the scene dumps. That is 5,654 pages and 708,368
  `obj` records. Ours: the full build (`--source-map`) in a scratch run,
  `--goto conv_title_start --auto-play 0 --real-time` with a `--choose` at
  each choice plate: 9,176 pages in 353 s. `frame_diff.py <segments>
  <ours> --map <map> --groups 15`: 5,576 of 5,654 pages pair. Missing
  1,656, extra 2,149, end 740, path 943, duration 688, zoom 174; copy 265
  are not differences. The largest groups (triaged 2026-10-19, History,
  "The whole-route frame diff's five largest groups"; run it now with
  `--alias LoadFrame=cg/sys/windowframe.png --ignore cg/sys/push/`):
  1. missing `color:…` (414): the original's colour boxes fade
     (`CreateColor` + `Fade`); ours draws most as a screen transition, not a
     sprite, so the pairing sees none. Check what is real.
  2. extra `cg/sys/windowframe.png` (174): our text window's frame sprite
     moves; the original's isn't an object that moves.
  3. missing `frame03`/`frame04` (89): the face windows' frames
     (`@LoadFrame`), drawn otherwise in ours.
  4. missing `cg/sys/push/長押しメッセージN.png` (40, dja050): the
     "hold" prompts' blink.
  5. extra pre-zoomed copies (`system/zoom/…`, about 150 over many
     pictures) and backgrounds that move in ours, but not as the original
     moves them in the same page window.
- **Memory snapshots** (2026-10-18): `ntrace.sh --probe
  research/nitroplus/sched_probe.nss --dumps
  research/nitroplus/local/dumps --pages 2 --seconds 240` runs the probe's
  statements where dja000 unlocks the menu and writes the process's
  memory at each `Delete("__dump_<label>")` (20 dumps of about 94 MB, the
  object mutex held and the game clock stopped; the whole run takes
  40 s at 1x). Run it at 1x: script waits follow the clock, while motions
  step once per frame. `exe_call.py`'s `load_dump` maps one, so the exe's
  own functions run against the game's state. The objects are a list from
  `0x50fb1c` (node +0xc the object, +0x18 the next node), with the name at
  +0, x/y/z at +0x3ec/+0x3f0/+0x3f4, and the render record at +0x804 (its
  alpha a byte at +0x14). Per-frame callbacks sit at +0x820..+0x83c, and
  `0x436040` steps every object once a frame.
  `ntrace.sh --scene-dumps <dir>` dumps instead at each script file the
  story enters: `s0000` whole (the base), and each later one only the
  4 KB pages that differ from the base, packed with zstd as the run goes
  (`load_dump` rebuilds a scene from both). In a 5 min test at SPEED=8
  (55 pages, seven files to dja001_03), the base was 74 MB and the diffs
  0.4-109 MB raw (65 % of the changed pages zero). Packed, the seven took
  46 MB, and one loads in 1-2 s.

What it showed (2026-10-15):

- Text: `box00`'s first row of ＭＳ Ｐ ゴシック 18 starts at y 480 (the
  box is at 472), rows 27 px apart; the converter puts horizontal text
  7 px below its box (`Game::text_drop`). The original's font is
  proportional and draws heavier; the click-wait bullet follows the
  last character.
- Text log: ＭＳ ゴシック 20 from (120,99), rows 24 px apart, 24
  characters wide, black with an #AAAA88 outline (ours too since
  2026-10-15: `strspoutline`, CONscripter after 0.22.0).
- Save screen: pixel for pixel ours but for the NEXT button's state.
- Choices: the option is centred about x 386 (MS ゴシック 22, white;
  ipag needs 388), and the plate under the pointer turns to
  `select002.png`; ours does the same since 2026-10-15.

## Extras

`boot.nss` shows EXTRA and LINER NOTES once the story is finished
(`#ClearG`); EXTRA calls `extra_menu.nss`. Converted in
`nitroplus/convert/django/extras.rs` on Muramasa's `PictureExtras`:

- **Menu**: the saloon with its doors shut, CG LIBRARY, RECOLLECTION
  LIBRARY, SOUND LIBRARY, EXIT, with its own music: `dj_INS12`, fading
  in over 3 s each time the menu opens from the title. On a library's
  button the buttons fade out, the doors swing open (`扉02`, `扉03`, with
  their sound) and the saloon zooms in through them to black; back
  from a library it zooms out of black, the doors shut and the buttons
  fade in (`ExtrasPassage`, `dr_open`/`dr_close`'s times).
- **CG library** (`extra_cg.nss`, `PictureGallery`): five categories
  (wanted posters), 20 thumbnails a page five across, ARTIFACT's three
  pages with BACK (next page) and NEXT (the one before). The script
  locks nothing: every picture is open once EXTRA is, except おめでとう画像
  (`#ClearAll`, set by the title's congratulations after every
  ending). Pictures larger than the screen are shown at half size; tall
  ones scroll up from their bottom. A category under the pointer shows
  the rifle (`gun_anime`: 25 frames, 50 ms each, looping), and choosing
  another fires it (`se_gun`). Beside the original under wine
  (2026-10-15): the page and the viewer match pixel for pixel.
- **Sound library** (`extra_sound.nss`, Muramasa's `SoundRoom`): 35
  songs (1.01's `cg2.npa` titles), the one playing in white with the
  running horse; pixel for pixel the original's but for the horse's
  frame. A click on the song playing stops it (the original starts it
  again).
- **Recollection** (`extra_recollection.nss`, `PictureReplay`): eleven
  scenes, all open; each `extra_N_*.nss` chapter is compiled once more as
  a subroutine (`Game::subroutines`) and played from the list. The list
  is pixel for pixel the original's; a replay plays to its end and comes
  back to the list. Save and load are refused during a replay (the
  shared replay code, as in the original).

## Story structure and walkthrough

`boot.nss` shows the logos and the title (`TitleSelect`); a new game
calls `dja000_開始スクリプト.nss`, whose `SelectStoryMode` calls
`main_root`: the 56 chapter files `dja001`…`dja056` in order (each a
`chapter main` calling its scene files, `dja001_フランコ馬車を襲う_01_…`).
There are no routes: ten two-way choices, each with one answer that
kills the player (a bad end: the `…be_…` file, then `ShowBadEnd`), the
other going on. The dead player's ending sets `#ClearNN`; finishing the
story sets `#ClearG`. Source: the scripts (`function_select.nss`,
`SetChoice02`, and the `$死亡NN` flags); no public walkthrough checked
yet.

| Chapter | Choice | Bad end (flag) | Goes on |
|---|---|---|---|
| dja004 | 手を引いてくれと頼む / 手を引かせる | 手を引いてくれと頼む (死亡01) | 手を引かせる |
| dja011 | 殺す / 殺さない | 殺さない (死亡05) | 殺す |
| dja011 | 信用する / 信用しない | 信用する (死亡02) | 信用しない |
| dja025 | ともかく撃つ / 慎重に狙って撃つ | ともかく撃つ (死亡06) | 慎重に狙って撃つ |
| dja027 | その場を退く / まだ退かない | まだ退かない (死亡03) | その場を退く |
| dja033 | 応戦する / 弾を詰め替える | 応戦する (死亡04) | 弾を詰め替える |
| dja041 | ７．７ｍｍ / ４５口径 | ４５口径 (死亡08) | ７．７ｍｍ |
| dja047 | 鎖を解く / まず銃を取り上げる | 鎖を解く (死亡07) | まず銃を取り上げる |
| dja049 | やるしかない / 付き合いきれない | 付き合いきれない (死亡09) | やるしかない |
| dja053 | その通り。あいつらは馬鹿だ / いいや馬鹿じゃない | その通り。あいつらは馬鹿だ (死亡10) | いいや馬鹿じゃない |

## Progression variables

| Variable | Meaning | Set | Read | Kept in |
|---|---|---|---|---|
| `$死亡01`-`$死亡10` | The player chose the deadly answer | the choice's file | the chapter file and `main_root` (end the game) | save |
| `#Clear01`-`#Clear10` | That bad end was seen | `main_root` | title, extras (to check) | system |
| `#ClearG` | The story was finished | `main_root` | title (`TitleCongratulations`), extras (to check) | system |
| `$ClearL` | The game just ended (back to the title) | `main_root` | title | save |
| `#ClearAll` | The congratulations were shown | `TitleCongratulations` | title | system |

The story's end (`conv_run_end`) runs the boot script's `InitBGM` and
`TitleCongratulations` (`Game::story_end`; it plays once every ending
has been seen, and only right after one, as `$ClearL` is set there) and
goes back to the title, as `main_root`'s loop does; the original runs
the check as the title comes up. The congratulations' pans move (each
starts under a white flash and goes on while it fades); not compared
with the original (it wants all eleven endings' flags under wine).

## Save format

装甲悪鬼村正's (`docs/games/muramasa.md`, "Save format") with this
game's table (`npa::DJANGO_TABLE`), the same per-file keys and inner
magic `c3 5f fc f1`. Folder `AppData\Roaming\Nitroplus\続・殺戮のジャンゴ
-地獄の賞金首-\ver1.00\` (`9999`: the save made at a choice, with
`script`, `frames`, `val`, `bklg`; `common`: `val.npf`, `bkmk.npf`).
`val.npf` is the same; the current page is `$textdecor` (`@text022`),
the scene file `$SYSTEM_spt_name`, its block `$単元名`. `script.npf`
(`FUN_004545d0`) also keeps the full text of every loaded script (one
block per `call_chapter` level: `boot.nss`, `dja000_開始スクリプト.nss`,
the chapter file, the scene file) but lays it out differently: 0x40
file slots (u32 1 before a text), blocks 0x400 apart.

`research/tools/nitroplus_save.py` dumps and round-trips these files
and writes variables and texts (checked on the original's `9999` and
`common`). Its `scene` option sends a save to any scene file (2026-10-18).
The template stands on the **last page** of its scene. In the chapter file
that called the scene (or, if no call follows there, `main_root` in
dja000), the calls after the scene's become one `call_chapter` of the
target, blanked with spaces to the same length in bytes. After the load,
the scene ends and the target starts. Cutting the scene after an earlier
page, as Muramasa's writer does, hangs the load: the scene file's
compiled records reach past the resume point.

A script `Save(n)` under wine writes empty files for every slot but
`9999`. `save_slot` (`00452410`) opens the files and then makes the
thumbnail (`FUN_0044a8f0`, a capture of the screen), which fails under
wine, and it gives up there. Slot `9999` (the choice save) skips the
thumbnail, so `Save(9999);` after a scene's last `</PRE>` makes a
template (here `dja001_フランコ馬車を襲う_01_革命とは`, `@text021`).

Per-scene snapshots as for 装甲悪鬼村正 (docs/games/muramasa.md,
"Per-scene snapshots"): `scene_saves.py` with this exe and that template,
then `scene_chain.py` (`--stall 60`). The run gets past the wine hang on
timed actions by starting again from the next scene. 2026-10-18: all 159
story scene files (`dja\d{3}_…_\d\d…`) in 15 min and 5 segments, 145 MB
packed.

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-15 00:42 (reuses the NSS reader) | - |
| First dry run | 2026-10-15 00:57 (whole story) | - |
| Story plays | 2026-10-15 00:57 | - |
| Title, save, settings | 2026-10-15 01:53-02:31 | - |
| Extras | 2026-10-15 08:45 | - |
| Progression verified | - | - |
| Finished | - | - |

## Status

What works (details in "History", "Status as it stood before the
2026-10-18 prune"): NPA archives, images, sounds, script parsing; the
whole story (56 chapters, 10 choices, bad ends; the good path plays to
the end, 6,496 pages, `research/nitroplus/django_routes.sh`); text in
the boxes' places, choices, pictures and motion, cameras, movies,
sounds; the system screens (title, right-click menu, save/load, log,
settings, yes/no) and extras, drawn by the engine from the game's
pictures and compared under wine; the settings' 各キャラ window
(`charavoiceconfig`, `VoiceWindow`: a check box per voice class muting
its voices, OK; checked headless, pixel-exact; it fades where the
original zooms); save round trips (`--roundtrip-every`).
Partial: the web build plays in a browser (title, story pages); saves,
sound and touch not checked there. Saves from the screens not tried by
hand; the log's scrolling and a slot's hover details not tried.

Open:

- `--original-archives` (`docs/original-archives.md`) plays the whole
  story; the web player's side (picking the files) is phase 2.
- Per-scene snapshots: all 159 scenes dumped from saves ("Save format").
  Next: the scene-start check against ours (as Muramasa's), then the
  interpreter under Unicorn from those dumps.
- `spgrab` (engine #32, merged) could draw a move that changes picture
  as a copy fading out over the moved picture: not used yet.
- `RemainTime` on a sound returns its whole length, not the rest
  (`sound_left` has it); right after the start, where most calls are,
  the two agree.
- Guesses, each settled by a frame of the original: the log's knob
  travel (its top from y 90 to 460; `CreateScrollbar`'s 646, 490 to
  646, 90 may be the knob's centre); the click-wait bullet's frame time
  (50 ms; not in the files). One value where the original has two: the
  save screen opens over 200 ms and closes over 300 (ours 300 both);
  the settings' pairs fade over 200, their bullets at once (ours at
  once both).
- Trace diff against the original (SPEED=1, dja001 to dja011, 1,049
  pages, 2026-10-18; recipe and the 50-page causes: History, "Trace
  diff, first 50 pages"); all pages pair. `missing` 3437, `extra` 112,
  `time` 878, known kinds only but for Next 1 and 2: `Request`s and
  0 ms resets give no command; `MoveZoom`'s `Move` folds into the
  `Zoom`; 736 of 840 `time` after page 50 are the exe's 100 ms wait
  polls; the rest is the harness (`WaitKey` waits 1.5 s in the
  original's auto mode, 0 under `--auto-play`; choices; movies take
  0 ms headless) or a gap put on the next record. `extra`:
  `call_chapter`, and `{...}` blocks inside text (their `MusicStart`s
  not seen on the original's thread 0).
- Time ledger (dry-run notes, `nitroplus/convert/sync.rs`): 127 syncs
  wait up to 1 s longer than the script asked for an action the exe
  runs on (`unwaited action held the script`; 62 in `TypeBegin`'s
  `WaitAction("$boxtype")`, function.nss:503-505, which waits only for
  the box in the exe); no motion ended at once (`timed action dropped
  by a sync`). dja006_02:23-29 (ours 0 ms, the original 14,081 ms) is
  not a scheduler case and #564 doesn't touch it: its `Wait(14000)` is
  spent inside the blocking `mpegplay` of the movie (`movie_wait`).

### Next

1. Source map: `spmotion` lines after a mapped line carry no entry of
   their own (~134 timed story motions show as `missing`, e.g.
   dja001_04:79-83, 525; ours has them). Map each.
2. The title, left from the scripts (History, "Title on coming back"):
   on coming back the logo cross-fades in over its 1 s turn (the
   original shows it at once, edge-on); a hovered button still jolts
   (the original pauses its `*Shake` process on MouseOver); from the
   extras the original fades to black over 300 ms first (ours
   cross-fades from the extras); after the settings or load screens
   the motions start again from their start. Not compared with the
   original's frames.
3. Extras: film the doors and the rifle beside the original once wine
   is next used (ours from the scripts' times only).
4. The unread-argument notes (`<Builtin> argument <i> ignored`; 33
   kinds, 18,947 notes at the 2026-10-16 dry run), most visible first;
   what the original does is from the scripts and
   `research/nitroplus/builtins.md`:
   1. `Fade` 3, tempo (77 non-`null`, mostly `DxlAuto`/`AxlDxl`): an
      eased fade; ours linear. Hard to see.
   2. `Rotate` 5, tempo (42 left): read for turns about x or y over
      time; the rest are about z (`spmotion` straight) or in loops.
   3. No-change candidate: (`MoveCamera` 5-6 on a camera with no
      children read, not noted, when its wait loses nothing: 15 gaps
      gone) `CreateEffect` 2-3 (1, the
      `Sepia` effect at 0,0 over the whole screen; Muramasa's undrawn
      effects pass 0 too). Left noted.
5. Sounds kept until their end (`convert/sounds.rs`): the sound clock
   adds up both branches of a runtime `if`/`else`. Fine as is.
6. `CreateEffect` Rain (37), Drop (4), Lens (2): 装甲悪鬼村正's, shared
   NSS code; noted, drawn as nothing.
7. The exe mapping (`docs/games/django.findings.tsv`; the census
   `argcensus.py` flags only the script's own values). Open, each from
   the decompile:
   1. `SetVertex` names: the exe compares lower case; ours any case,
      as 装甲悪鬼村正 writes `Center`/`Middle` (whether its reader
      lowers them first is unchecked).
   2. `CreateColor` `GREEN` is 0x008000 in the exe (`LIME` 0x00FF00),
      ours 0x00FF00; the other HTML names are missing. Not in Django's
      data.
   3. `SetPan` is DirectSound's (dB on the other side), `chpan`
      linear; the data uses only 0 and ±1000, where they agree.
   4. `Fade` with four arguments takes the fourth as the tempo, not
      the wait; the six such calls take 0 ms.

## History

Done items: [docs/history/django.md](../history/django.md).
