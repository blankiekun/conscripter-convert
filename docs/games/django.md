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
`common`); its `scene` option is Muramasa's only. For Django it would
cut the scene file after the current page, the chapter file after the
scene file's `call_chapter`, and `main_root` after the chapter's
`call_chapter`, going on from the target's line (Status).

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
- Save tool: `nitroplus_save.py write` can't yet send a save to another
  scene ("Save format" has the plan). Untried because the original
  hangs under wine when a script `Save` runs from a process (the files
  stay empty) and on timed actions; a template saved on a page from the
  menu is needed first.
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
   3. No-change candidates: `MoveCamera` 6 (15, a camera with no
      children yet: nothing moves) and `CreateEffect` 2-3 (1, the
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

### FadeFade after an unwaited motion (2026-10-18)

`FadeFade` (function.nss:887) is `Fade(n, 0, 0, null, true)` then the
timed fade. After an unwaited `Move` (dja010_01:76-78: `CreateBGz`,
`Move("背景21",16000,@320,@0,…,false)`, `FadeFade(…,1000,…)`) the 0 ms
fade's sync hid the picture over the move's time (`csp`, `print 11`)
and the move ended with it, so the fade-in placed it at the move's
end. Now a sync leaves a hidden picture's unwaited move (its start,
path, tempo) for the sync that shows it again, and doesn't draw over
its time (`Move` keeps a shown picture's start for this); only to the
next sync, as a later one would replay the move from its start. Dry run:
hide at once (`print 1`), then `lsp` at the end, `spmotion` from -320
over 16 s and `print 11`; the same at dja055_01:99 (8 s). Calls after
an unwaited timed motion: dja010_01:78, 125; dja055_01:99, 133, 173,
358, 255; dja055_04:575; dja056_09:59 (after 0 ms ones nothing
changed: dja003_02:215, dja011_03:488, dja011_03be:49, 80,
dja017_01:569, 631, dja049_02be:120, dja050_03:1198, dja056_06:173).
Django: 34 more `spmotion`s. 装甲悪鬼村正 (and the English build, same
lines): ~70 motions kept, children hidden while their parent slides
(ma02_020vs:1024-1027: `絵立絵` moves its 1.5 s with its window instead
of appearing at its end; ma03_016:131: the 7 s pan goes on behind the
white flash), and one more `effect` time (every `print` renumbered).

### Status as it stood before the 2026-10-18 prune

#### Queue (2026-10-16)

- Save tool: `nitroplus_save.py write` can't yet send a save to another
  scene ("Save format" has the plan). Untried because the original
  hangs under wine when a script `Save` runs from a process (the files
  stay empty) and on timed actions; a template saved on a page from the
  menu is needed first.
- Merged: windows clip their children (#45), the song's lyrics in
  dja026 (#47), unread-argument notes ranked in Next (#51), the song
  lines type out (#55), `WaitPlay` on a sound waits (#70). Open:
  disused one-shot sounds kept until their end (#76).
- `spgrab` (engine #32, merged) could draw a move that changes picture
  as a copy fading out over the moved picture: not used yet.
- The unread-argument notes (#30) are sorted into Next, item 4: easing
  first.
- `RemainTime` on a sound returns its whole length, not the rest
  (`sound_left` has it); right after the start, where most calls are,
  the two agree.

| Area | Status |
|---|---|
| NPA archives (key), images, sounds | ✅ Every file decodes |
| Movies (MPEG-1) | 🟡 Found in `<game>/movie` (the default); the ones the script waits for play full screen (`mpegplay`), the others (smoke, the Black Hawk, cuts under the text) as sprite movies (H.264 MP4 given to `lsp`, the alpha ones with their opacity as the right half, CONscripter after 0.24.0), a paused or finished one as its frame; the 400-vs-3 battle's six play while the button is held (`mpeghold`), its counter between them rolls as the original's |
| Script parsing | ✅ All 249 files; 56 places that don't parse, none in the story |
| Story control flow (56 chapters, 10 choices, bad ends) | ✅ Compiled whole (`compile_story`); in the engine the good path plays to the end (6,496 pages) and each wrong answer ends the story with its own bad end, then the game goes back to the title (`research/nitroplus/django_routes.sh`, which stops each run there, 2026-10-15) |
| Text | 🟡 In the boxes' places (box00: 39 columns at 50,472, white on the black bar; five rows, as many as end inside the 130 px box); 3,424 voiced lines; `box00`, `box01` (x 250) and `box02` (x 390) start within 1 px of the original's under wine (`box03`-`box05` are never used) |
| Pictures, sound in scripts | 🟡 Black bars, face windows in their frames, standing pictures, backgrounds; pictures turned over (`Rotate` by 180°, 315 calls: characters facing the other way; 12 movies) drawn mirrored (dja004's choice now as the original's); turned about x or y they are drawn squashed by |cos| about their vertex, their children with them (no perspective), and turning over time (the face windows' `WindowRotateR`/`WindowCloseR`) they unfold or fold away as one `spmotion` (checked frame by frame in dja002); children of a picture moved over time (a face in its frame) move with it; `CreateEffect` Sepia (dja038_03) tints the pictures below it (`spfilter`); cameras (`CreateCamera`, `MoveCamera`, 365 calls): a camera's child of priority p drawn about the screen's centre at (its place − the camera's) × z / (1000 − p), as large, z the camera's zoom (priority is depth: measured under wine with boxes at four priorities and six camera settings, and dja001's carriage scene then matched the original's frames); moves, zooms and camera moves over time (1,765) as real motion: the engine moves the picture (`spmotion2`, keeping its last place) while the script goes on, also through the cross-fade of what else changes; those that also fade are one motion with alpha keys, and those drawn on another layer (the layers spread out again) are placed where they end and moved from where they were; movies (smoke, explosions, `集中線21`, `背景21`) move and zoom the same way and keep playing; a picture under an opaque full-screen one isn't drawn (old backgrounds the script leaves up under `窓`); the 400-vs-3 battle's muzzle flashes (`GunFire`) as a white sprite added on (`spblend`), flashing its number of times while the text goes on (`spalpha` keys), and its strobe the same way until deleted; the looping shakes (`ShakeVLoop*`) as the background's looped `spmotion`, and dja001's opening slideshow (`MoveBlackHawk`: the pictures fading and sliding in turn) as looped motions (`convert/loops.rs`, shared with 装甲悪鬼村正); every sound is found |
| Choices | ✅ Plates as picture buttons, text placed as the original's (compared under wine) |
| System screens | 🟡 From the game's pictures (`django/screens.rs`): title, right-click menu, save/load (3×20 slots; drawn by the engine, `screen "save"`/`"load"`), yes/no questions, text log (drawn by the engine, `screen "backlog"`: rows from `logrows`, the #AAAA88 rim as the text's `outline`, engine #104), settings (bullet rows as `SliderSteps`; drawn by the engine, `screen "settings"`: each bullet a radio lit from the chosen one down its row with `on_range`, engine #98), the click-wait bullet after the last character (`getcursorpos` in the bar's wait routine); compared under wine: right-click menu, save, log, settings (text speed starts at level 5 as the original's; the two SAVE-DATA question rows set `ConfirmSave`/`ConfirmLoad`, DEFAULT resets (no question) and the voice test plays `sys/config/test.wav`; the display options and version panel left out on purpose) |
| Extras | 🟡 Menu (drawn by the engine, `screen "extras"`; the libraries stay script routines) with its music and the doors into each library and back, CG library (with the rifle over a category), sound library, recollection, each compared with the original; the doors and the rifle not filmed beside the original |
| Saves | 🟡 `--roundtrip-every 100` over the first 1,500 pages, and `--roundtrip-every 1` over the 569 pages one auto-play reaches: no save/load mismatch (a music played once before the next part, `BgmLater`, is `mp3save`, so a load plays it again); saves from the screens not tried by hand |
| Web build | 🟡 Plays in a browser (title, story pages); saves, sound and touch not checked yet |

#### Screens drawn by the engine (2026-10-17)

Save/load, the text log and the extras' menu are CONscripter `screen`s,
as in 装甲悪鬼村正 (`SaveGrid`, `LineLog` and `PictureExtras` built by
the shared code; `django/screens.rs` and `extras.rs` only switch them
on). Checked against main's script routines on Django's full build
(headless, `--shot-after`, `compare -metric AE -fuzz 2%`): the save
screen with two saves, the load screen, the extras' menu, and the text
log after 32 pages: 0 pixels each. Not tried: the pointer's hover
details on a slot, scrolling the log, a save or load from the screens
by hand.

#### Accuracy (value audit, 2026-10-17)

Every hand-typed value in `convert/django.rs` and `convert/django/`
carries its source (`// src:`, `// guess:`, `// ours`). Fixed: the save
screen's last line in `SetFont`'s 19 px (18 characters, was 18 px, 19),
リリィ's name plate in the CG library's open category 6 px up, as at
rest. Open guesses, each settled by a frame of the original:

- The log's knob travel (its top from y 90 to 460; `CreateScrollbar`'s
  646, 490 to 646, 90 may be the knob's centre).
- The click-wait bullet's frame time (50 ms; not in the files).

One value where the original has two: the save screen opens over
200 ms and closes over 300 (ours 300 both); the settings' pairs fade
over 200, their bullets at once (ours at once both).

#### Trace diff (2026-10-17)

The original's trace (`ntrace.sh`, wine section) against ours, the
first 50 pages from dja001 (`docs/new-game.md` step 23):

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run --source-map out/django.map
research/tools/scratch.sh <a full build> <scratch> out/django-dry/0.txt
cp out/django-dry/ons_args <scratch>/   # the dry run's encoding, not the build's
<scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/django.map --pages 50 --ms --thread 0
```

All 50 pages pair, with the same text (the window's fade between
blocks since 2026-10-17, History). Not findings: page 1's 50
`missing` (the patched boot skips the title, ours plays it),
`call_chapter` lines (no call record in the original), 0-72 ms gaps.
Open, not fixed:
- Smaller, maybe wine's timing: dja001_02:50 (`Move("背景10", 1000, …,
  true)`) then `Zoom` (function.nss:877): 419 + 2 ms in the original,
  0 + 300 in ours; a 0 ms waited `Move` (function.nss:910) took 618 ms
  in the original (page 44).

#### Next

0. Boot logos (History, "Boot logos"): the Nitroplus logo's fade-out
   during its flight is lost (it goes at the flight's end), and its
   `Rotate` @60 isn't drawn; `TitleSet`'s first-title opening (the
   background in over 4.9 s, the logo slammed in, the tumbleweed) isn't
   converted: the title fades in over 1 s.
1. The trace checker's remaining findings: "sounds on two channels at
   once" are the script's own (the same shot or horse on two sound
   objects, started 500 ms apart) and stay.
2. Extras: film the doors and the rifle beside the original once wine
   is next used (ours from the scripts' times only).
3. From the exe's coverage table ("The exe"): `SetShade HEAVY` (one use): done (#197).
4. The unread-argument notes (`<Builtin> argument <i> ignored`;
   dry run 2026-10-16: 33 kinds, 18,947 notes, fewer than #30's 39 and
   21,859 since later PRs read more), most visible first; counts are
   compiled occurrences, what the original does is from the scripts and
   `research/nitroplus/builtins.md`:
   4. **`Fade` 3, tempo** (77 non-`null`, mostly `DxlAuto`/`AxlDxl`): an
      eased fade; ours linear. Hard to see.
   5. **`Rotate` 5, tempo** (42 left): read for turns about x or y over
      time; the rest are about z (`spmotion` straight) or in loops.
   8. **Left of the no-change candidates**: `MoveCamera` 6 (15, a
       camera with no children yet: nothing moves; noted, as the value
       alone can't tell) and `CreateEffect` 2-3 (1, the `Sepia` effect at
       0,0, drawn over the whole screen: left noted, as Muramasa's
       undrawn effects pass 0 too). The rest is done (History).

5. Sounds kept until their end (shared NSS code, `convert/sounds.rs`):
   1. **Time across runtime branches**: the sound clock adds up the ops
      of both branches of a runtime `if`/`else`, so a sound started
      before one may be taken as ended too soon. Fine as is.
6. `CreateEffect` Rain (37), Drop (4), Lens (2): 装甲悪鬼村正's, shared
   NSS code; noted, drawn as nothing.
7. The exe mapping redone against `docs/new-game.md` steps 6-17
   (2026-10-17): every argument and branch the converter maps, with
   seen/inferred/unknown, in `docs/games/django.findings.tsv`. The
   census (`argcensus.py`) flags only the script's own values (shake
   amplitudes going to ±1, 300000 ms shakes, 20000-80000 ms moves, the
   slideshow's 1 ms jump keys). Fixed: `SetVertex` px outside the
   picture. Open, each from the decompile:
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


### Title opening (2026-10-18)

The first title of a run comes up as `TitleSet` draws it (`$Logo` 0):
the function compiled after the boot logos (`Game::title_opening`,
twice: at `conv_title_script` with `#ClearG` 0 and `conv_title_script1`
with it 1, so its `if (#ClearG)` needn't be merged) and played as the title's opening
(`TitleOpening::script`: from black, its sprites replaced by the
title's at once as it returns, the boot's music going on). The
background in over 4.9 s (`Axl2`); at 3 s the tumbleweed fades in and
rolls off to the right (`TumbleweedMove`'s lead-in, below); a shot and
the logo zooms in over 200 ms, a double shot and it jolts (300, 900,
600, 1000 per mille, a frame each); the buttons fly in turning and
fading, a panned shot each, 50 ms apart (5.55 s in all, as the
script's times add up). What it took, shared NSS code:

- A number as a timed action's wait (`Fade(…, 300, 1000, null, 50)`)
  waits that long, the action going on (Django.exe `004310e0`); it
  waited the whole action. 装甲悪鬼村正's scripts have 56 such calls
  (its findings' census); its snapshot's chain gets 26 shorter
  cross-fades and 15 shorter `delay`s from them:
  Muramasa.exe's `Fade` (`0040e050` → `00445ac0`) passes `true` as -1
  and a number through `atol` to the wait loop `004489e0`, which
  returns once that many ms (`timeGetTime`) have passed.
- A new picture moving in also turns (`Rotate` about z) and fades in
  from 0 with the move when the fade takes as long and the script
  doesn't wait it out (it showed upright, faded in by the cross-fade).
  Story pages change too: 21 pictures sliding in now fade in over their
  slide (e.g. 300 ms, not the 100 ms cross-fade); 125 in 装甲悪鬼村正.
- A loop process whose timed actions come after a wait before the loop
  (`TumbleweedMove`: still 3 s, faded in and rolled once, 3 s, then the
  loop) plays them on paper as the motion's lead-in (`spmotion` loop 4);
  they were drawn at once and the loop started at the process's start.
- A picture turning about x or y as it zooms, moves along a path or
  fades (the boot's Nitroplus logo: `BezierMove`, `Zoom` 500, `Rotate`
  @60, `Fade` 0) does all of them in one motion: the zoom from the
  sync's `Zoom`s, the fade over its own time (a turned picture's
  children with it); the turn's motion dropped the rest (the logo kept
  its size and went at the end).

Checked with a `--real-time` trace: the opening from 12,354 to
17,960 ms (5,606; the script's 5,551 plus frames). Its last frame
against the title's (the title waits for the opening's last motions,
EXIT's turn, before taking over): only the logo differs, in 5,044
pixels (the opening's is the 600 px picture the engine zooms up, the
title's the original).

The tumbleweed is no longer in the title's background: it is never at
rest there in the original (rolled off at the opening's end, then by
every 9 s), and its loop isn't drawn on the title.

### Title on coming back, motions at rest (2026-10-18)

Coming back to the title (from the story, or from the extras) plays
`TitleSet` with `$Logo` 1, as the boot loop runs it again: compiled
twice more (`Game::title_opening`'s third name, `$Logo`, set to 1; at
`conv_title_script_back` and `…_back1`) and played as the openings for
coming back and for after the extras (no music started there: the
extras' way back sets it). The background in over 1 s, the logo
zooming from 200 % and turning in about x over 1.2 s (1 s waited),
the buttons flying in as at launch: 2,660 ms in a `--real-time` trace
(the script's 2,450 plus frames).

At rest the title now has TitleSet's processes: each button's `*Shake`
(a jolt every 10-15 s, its first at 2, 3, 5, 7 or 8 s) and
`TumbleweedMove` (the tumbleweed, a title picture at 300,420, rolling
by every 9 s), as `TitleScreen::motions`, taken from the loops the
compiled opening leaves on its pictures (`opening_loops`, matched by
picture). Each goes on from the opening's (`TitleOpening::clocks`,
`spmotionclock` before the opening's sprites are cleared, so the
title clears only the story's sprites 62-999, not `csp -1`). Checked:
the trace's title motions start `in` 1 ms (buttons) and 5,797 ms
(tumbleweed, started 5,812 ms before at the opening's start); frames
at the title's 0-10 s differ from its first only where the schedule
says (LOAD at 3.05-3.15 s, START 5 s, CONFIG 7 s, EXIT 8 s; the
tumbleweed from the left edge at 3 s, gone right by 9 s); the title at
rest after the coming-back opening is pixel for pixel the one after
the first.

Shared NSS code: a loop process with a fade-in lead-in started on a
picture not drawn yet, then an instant `Fade` of it before any time
passes (the `$Logo` 1 branch starts `TumbleweedMove` before
`Fade("タイトル*", 0, 0)`), now holds that opacity in the motion's keys
until the process's own fade; it set the picture's opacity to 0, so it
was never drawn. No change in 装甲悪鬼村正's scripts.

### Boot logos (2026-10-18)

`boot.nss`'s loop: `InitBGM` (the music objects), `TitleCongratulations`,
then `TitleLogo` while `$Logo` is 0 (the first title of each run; set by
`TitleSet`; no save flag): `media_a.jpg` in 500, `WaitKey(2000)`;
`media_b.jpg` cross-fades in 500, `WaitKey(2000)`, out 500; `Wait(500)`,
`WaitKey(1000)`; the title's music `dj23_harmonica` starts; `nitroplus.jpg`
in 1000, `Wait(3000)`, a gunshot, the logo shot away in 500
(`BezierMove`, `Zoom` to 500, `Rotate` @60, fading). Then `TitleSet`'s
first-title opening. Converted from the script (`Game::boot_logos`, a
`BootStep::gosub` once a run); the music goes on into the title instead
of starting again. `--real-time` trace: the music at 7,027 ms (script
7,000), the shot at 11,036 (11,000), the title at 11,536 (11,500).

### Sounds in the source map (2026-10-18)

A sound's ops are written when its `Play` is flushed (after the
helper's volume and loop calls), and took the mark of whatever
statement was running then: TitleLogo's `SoundPlay` (boot.nss:153)
that of the `if` around it (140), a scene's first sound the chapter
file's call line. Now they take the statement that called the helper
holding the `Play` (`Compiler::played_at`); 0.txt unchanged, the trace
diff's `boot.nss:153` pairs.


- **Unwaited fades run on** (2026-10-17, trace diff): dja001_01:126-128
  (`Fade("黒幕", 3000, 1000, null, false)`, then `TypeBegin`): the
  exe's `Fade` (`cb_00438fb0`, Muramasa.exe `0044f020` alike) doesn't
  block, so the text shows 100 ms later (240 under wine) while the
  screen darkens for 3 s; ours held the text for a 1 s cross-fade
  (1009 ms). A new picture faded in from 0 for longer than the sync
  waits is now placed and faded in by the engine (`spmotion` alpha
  keys, the fade's tempo as its curve), as a shown picture's fade
  already was, and a later `WaitFade`/`WaitAction` on it waits for it
  (`spmotionwait`; also for moves the script went on from). Trace diff:
  109 ms there. 装甲悪鬼村正 and its English build change too (733
  such fade-ins, 199 `spmotionwait`).
- **The text window fades between blocks** (2026-10-17, trace diff):
  each `TypeEnd()` fades `@box*` out over 100 ms and waits
  (function.nss:510), and the next `TypeBegin` fades the box in over
  100 ms and waits (502-503) before it enters the text: the text, a
  child of `box00`, is drawn at the window's opacity × its own
  (`drawn_opacity`, Django's `relative_children`). Ours hid the text at
  once (`textoff`) and waited 2 × 100 ms; now a waited fade that leaves
  the shown text at 0 is `windowfade 0,ms` (the window picture is
  transparent: only the text fades). 装甲悪鬼村正 is unchanged: its
  exe leaves a faded `CreateWindow`'s children at full (builtins.md,
  wine test), and its `TypeEnd` fades `Hideable` boxes, already
  `windowfade`. Trace diff: the 502/510 records now pair (findings
  178 → 41 lines, missing 164 → 146, all of them page 1's boot).

- **Notes that only inflated the gaps** (2026-10-17): `SetFrequency`
  on music is noted only when it changes the rate (every Django call,
  `MusicStart`'s, sets 1000; 装甲悪鬼村正's 1,486 notes are 3), and
  `LoadFont` (`load_font_object` `004308b0`: glyphs pre-rendered into a
  font object no text names) is converted as nothing. Django's gaps
  116 → 34, 装甲悪鬼村正's 4,925 → 3,442; no 0.txt changes.
- **`SetAlias`, one per object** (2026-10-17): the exe (`00432da0`,
  Muramasa.exe `004475b0`) keeps one alias slot per object (+0x100),
  which a new alias overwrites; ours kept the older aliases too. Now a
  new alias (or the object's own name) drops the object's older ones.
  Django's 0.txt doesn't change (its re-aliasing, such as the save
  screen's `video`, never uses the old name again); in 装甲悪鬼村正 14
  `Request(…, Stop)` calls on an alias the object had lost now find
  nothing, as in the exe (14 fewer gaps, 0.txt the same).
- **`Draw`** (2026-10-17): the exe's `Draw` (`0040ba10`, Muramasa.exe
  `0040f470`) waits at most 4 ms for a frame and lets running actions
  go on; ours waited for all of them (as `WaitAction`). It is a sync
  now, as where the script doesn't wait (running actions cross-faded
  up to `UNWAITED_MS`, 1 s). Neither game's 0.txt changes: no `Draw`
  follows an action over 1 s. dja019_02's camera move still starts
  500 ms late (its pictures' 500 ms fade-in is a blocking cross-fade
  before the move); leaving the fade pending for the move's wait gets
  the time right but places the new pictures where the move ends, the
  280 px pan lost, so it stays.

- **`Wait` and clicks** (2026-10-17): the exe's `Wait` (`0040bd80`,
  Muramasa.exe `0040f7f0`) ends only on quit, a state change or skip
  mode, not on a click; ours was `delay`. It is `wait` now, which
  CONscripter ends in skip mode (engine PR #134); `WaitKey(ms)` stays
  `delay`.

- **`SetBacklog`'s third argument** (2026-10-17): the voice's
  character name (`0040cd90`: with a voice, it goes with the voice
  entry, as `<voice NAME=>`), what the converter already reads it as
  (`backlog_character`); with `null` as the voice it does nothing.
- **`SetVertex` outside the picture** (2026-10-17): the exe
  (`004666c0`) leaves an axis whose px lies outside 0..size as it was;
  dja004's `SetVertex("立絵30", center, 704)` on the 600 px robber kept
  `bottom`, while ours moved his pivot 104 px down and the zoomed
  picture 20 px with it.

- **The extras' doors, music and rifle (2026-10-17).** Left out when the
  extras reused Muramasa's `PictureExtras`, now from `extra_menu.nss`
  and `extra_cg.nss`: the doors (`ExtrasPassage`; the menu screen closes
  holding the display, the script runs the doors, `spmotion` zooms about
  each picture's centre, t² in and out as `Axl1`/`Dxl1`; back, the screen
  opens with `"back"` and its buttons' `open` motion), the menu's music
  (`SoundRoom::menu_track`, `bgmfadein 3000`), the rifle (`CategoryHover`:
  an `exbtn` showing an animated sprite, `exbtn_d` hiding it). Checked
  headless against main's build (`compare -metric AE -fuzz 2%`): the menu
  as it opens and after coming back 0 px; the gallery with the pointer on
  リリィ differs only in the rifle's 50×52 box at 252,442, which steps
  through its frames; the trace has `bgm` dj_INS_スィートウォーター讃歌民衆
  with `bgmfadein 3000` at the menu's start. The roundtrip sound bugs
  once listed in Queue were fixed by #292 and engine #102.

- **`Shake` 6 and 7, `CreateWindow` 6 (2026-10-17).** From the
  decompile (`Django.exe` 1.01), each finding marked. `Shake(obj, time,
  x0, y0, x1, y1, n, tempo, wait)` (`builtin_Shake` → `FUN_00431de0`,
  frame `FUN_0043f570`): seen: `n` above 1000 does nothing, not even
  the wait; the object is drawn at ±(x0 + (x1 − x0) × curve(t)), the
  sign flipping, + first, once 1000 ÷ (`n` × 0.001 × fps) ms have
  passed since the last flip; fps is 60 at the default game speed
  (`FUN_00469a20`'s table: 960, 480, 240, 120, **60**, 50, … and
  1, 2, 4, 8, **16** ms a frame); the curve is the tempo's table
  (`FUN_00434ea0` by the name's byte sum; `Dxl2` read as 1 − (1 − t)³),
  over time × 0.06 frames. Inferred: the other tempos' curves are
  Muramasa's (the same dispatch); `n` 0 flips each frame (the exe divides
  by 0). Ours: a full-screen `Shake` is CONscripter's new `screenshake`
  mode 2 (to and fro, the full size; engine PR #79) with the tempo as
  its curve, every 17 ms (`n` 1000) to 83 ms (`n` 200); a picture's
  `spmotion` keys flip at the same step (longer when the 64 keys run
  out), eased the same. `CreateWindow(name, priority, x, y, w, h,
  relative)` (`FUN_0042fee0`): seen: anything but `true` clears the
  object's +0x3c8, which every new object has at 1 (`FUN_00436de0`);
  the place parser (`FUN_004403a0`) and `FUN_00437980` then count its
  children's places from the screen, not from the window. It is not the
  clip. Ours: such a window's children are placed from the screen. No
  `0.txt` change: `窓` is at 0,0 and the counter is drawn by
  `django/counter.rs`. Dry run: 52 `screenshake` and 3 `spmotion` lines
  changed, the completion went from 55,499 to 56,529 of 57,053 calls (Shake
  58 → 3 gaps, CreateWindow 975 → 0).

- **Calls that change nothing, and calls left out (2026-10-16, shared
  NSS code).** Arguments whose value does what ours does are in
  `UNREAD_HARMLESS` (`convert/builtin.rs`), each for the value it covers:
  a `null` limit on `WaitAction`, `WaitPlay`, `WaitFade`, `WaitMove`,
  `WaitZoom`, `WaitText`; a `null` tempo on `SetFrequency` and
  `MoveCamera`; `LoadText` 0 and 1 (the unit and preprocess names), and
  `WaitText` 0 (the page's own text); `LockVideo` (a group of changes
  shown at once, as ours shows them at the next wait). Calls left out on
  purpose (`LEFT_OUT` in `convert.rs`, `metrics.json`'s `left_out`):
  names no function or built-in has (the exe skips them too), and
  `Conquest` (whole-scene read marks for the skip; the engine keeps its
  own). Django's gaps went from 9,411 to 1,554. `Random` (returns 0) and
  the built-ins of `function_select.nss`'s choice code (reached, replaced
  by our choice screen) stay gaps.
- **Movies moved or zoomed over time (2026-10-16, shared NSS code).**
  The last 10 moves still drawn as cross-fades were all movies played
  as sprites (smoke, explosions, `集中線21`, `背景21`): `motion` took
  only pictures. A movie's zoom is its cell's (`spzoom`), so the keys
  now take the ratio of the two, and the movie keeps playing as it moves
  (`spmotion2`, which CONscripter applies to movie sprites); one placed
  again on another layer gets its `lsp` before the motion (an `lsp`
  after it stopped it, which also fixes a rain strip's motion in
  装甲悪鬼村正's `ma00_000` chain). Dry run: no cross-fade notes left.
- **Pans over a picture cut to the screen (2026-10-16, shared NSS
  code).** A picture zoomed past the screen is drawn cut to it
  (`Assets::scaled`); a move or zoom from one cut to another (dja015's
  `立絵50` at zoom 4000, dja001's carriage interior behind the zooming
  camera) was a cross-fade. Now the whole picture is placed, zoomed by
  the engine (`spzoom`, as many pixels as the cut), and moves; the
  converter still records the cut, so the next change leaves it moving
  and replaces it afterwards. Checked with `--real-time --shot-at` in
  dja001 (`wait 1000` after the change): the interior zooms with the
  characters instead of fading (AE 15k/9k/2k at +50/+100/+150 ms; 1.4k
  at the end, the engine's resampling against the converter's).

- **Fading moves in a spread, uneven zooms (2026-10-16, shared NSS
  code).** Two of the moves still drawn as cross-fades. (1) A picture
  that starts to move as it fades out (dja047's `イベント絵23`, twice)
  when the layers spread out: it now keeps its layer like a running
  motion, and the one motion with alpha keys runs (trace: it ran its
  400 ms). (2) A zoom more along one axis than the other (dja023's
  イライザ unfolding from a line, `Zoom` x 0 to 500 over 200 ms): the
  new sprite is placed where it ends and zooms its width from 0
  (`uneven_motion`, its own keys, since `motion` takes even zooms
  only). Checked with `--real-time --shot-at`: dja023 at +80 ms is a
  narrow strip widening instead of a cross-fade of the whole picture
  (AE 23k to 13k over +40 to +160 ms, the same at +400).

- **Moving pictures spread with no room keep moving (2026-10-16, shared
  NSS code).** When the moving pictures leave no room for a new one
  between them (dja049's zoom as the old man stands up: a picture
  between two zooming ones), all are spread as before, and each one
  whose move still runs now goes on with it on its new layer from where
  it is (the straight line's point, as for camera children), instead
  of being put where it ends. Moves onto another layer are now also
  followed by new pictures in their camera. Checked with `--real-time
  --shot-at` at that zoom (`wait 1000` after the change): at +100 ms
  the scene is mid-zoom instead of at its end (AE 62k/48k/32k at
  +50/+100/+150 ms, the same at +300).

- **Running motions go on (2026-10-16, shared NSS code).** Two cases
  cut a motion short, its picture placed at its end. (1) When the layers
  spread out again, pictures whose motion still runs on the ops' clock
  (or that loop) now keep their layer and the others spread between
  them; all of them spread as before only when there is no room (7 of
  Django's 8 such spreads keep them; in Muramasa 1,181 restarted loops
  fewer). (2) A new picture in a camera that is moving (dja022's
  expressions: `ReplaceFace` right after a `MoveCamera` over time) was
  put at the camera's end while its neighbours still zoomed: now it
  gets their motion, from where the camera draws it (mapped from a
  child at the same depth; begun later, from the straight line's point).
  Checked with `--real-time --shot-at` at dja022's `0200280ku` (a
  `wait 1000` patched after the page's `print 4`): at +150 ms クラウス's
  new expression is mid-zoom with the background instead of already at
  its end; frames +50 to +350 differ from before (AE 70k to 22k), and
  at +600 they are the same.

- **A picture replaced or turned over as it moves (2026-10-16, shared
  NSS code).** A `CreateTexture` of the same name with another picture
  (an expression, a CG) or a turn over at once while a move or zoom is
  pending was a cross-fade. The exe replaces the picture at once, so now
  the new picture is placed where the old one was (its size from the
  new picture at the old zoom, about the old centre) and moves from
  there, as a move onto another layer (#175). No `spgrab` needed. Dry
  run: 23 → 14 notes. Checked with `--shot-effects` in dja001
  (`*conv_title_start:1896`): Eliza, turned to face right, slides in
  from the left during the scene's 500 ms cross-fade.

- **A zoom out of a picture cut to the screen (2026-10-16, shared NSS
  code).** A picture zoomed past 4096 px is drawn as only its part on
  the screen (`Assets::scaled`), so a zoom out of it (dja001's camera
  backgrounds diving in and out) was a cross-fade: the engine zooming
  the cut part down would leave the rest empty. `Shown::whole` keeps the
  whole picture's place, the motion goes by it, and a zoom out of a cut
  picture places the uncut end and runs the motion from the cut start
  (as a move onto another layer, #175). A zoom into a cut picture keeps
  the uncut start, zoomed by the engine. Dry run: 33 → 23 notes. Checked
  with `--real-time --shot-at` in dja001's carriage (`*conv_title_start:1013`):
  mid-zoom, the wall behind zooms out with the four passengers.

- **A move that fades out to 0 (2026-10-16, shared NSS code).** A
  picture moving or zooming while a `Fade` over time takes it to 0 was
  left out of the sync and so cross-faded away where it stood. Now it
  stays for one motion with alpha keys to 0 (as #156's fades) and is
  drawn until its time is up by the clock of the ops
  (`Compiler::vanishing`), or until another picture takes its layer;
  then it goes. Only a timed `Fade` counts (`pending_fade`): a `Fade` at
  once while a move is pending (dja005's `FadeFade`: to 0 at once, then
  in) sets the opacity at once and is not faded over the move's time,
  which #156 did. Dry run: 8 such motions in Django; one that can't be a
  motion (another layer, a cut picture) is still a cross-fade and now
  counted in the notes (装甲悪鬼村正 245 → 252). Checked in the engine:
  the trace of the first 1,560 pages has the same findings minus the 3
  "shown at alpha 0" (sprites #156 left faded to 0), and dja012's 5 s
  zoom-out keeps moving through the page after it (`--shot-at`, frames
  differ by RMSE 0.2). #173's `spovertext` checked in Django too:
  dja026's song lines type out over the black bar, the explosion's
  white flash stays clipped to its window.

- **A move onto another layer is still a motion (2026-10-16, shared NSS
  code).** When a new picture has no free layer between its neighbours,
  every picture is spread onto new layers, and a picture moving or
  zooming then was drawn as a cross-fade (27 of the 60 notes, mostly
  dja001's camera pans). Now it is placed on its new layer where it
  ends and the same `spmotion` runs from where it was (keys taken from
  the end, after the placement in the same change). A fade with it
  stays a cross-fade (its alpha keys are for the old sprite). Checked
  with `--real-time --shot-at` on dja001's pan (`*conv_title_start:1685`,
  a `wait` in place of the `delay`): the two women and the background
  slide off at their depths over 500 ms.

- **A move that also fades is one motion (2026-10-16, shared NSS
  code).** A `Move` or `Zoom` over time with a `Fade` of the same
  picture was drawn as a cross-fade; now `motion` puts the fade in the
  `spmotion` keys' alpha. The engine multiplies a key's alpha by the
  sprite's, so the sprite is first set to the larger of the two
  opacities (`fade_base`) and the keys go from old/larger to new/larger
  (loop 2 then leaves the sprite at the new opacity). That `msp` is not
  drawn first (conv-cons drops its `print 1` before an `spmotion` of the
  same sprite), or a fade-in would show the picture whole for a frame.
  Dry run: the cross-fade notes went from 133 to 59 (60 after the turns, #168).

- **`Rotate` about x or y over time (2026-10-16, shared NSS code,
  `convert/turn.rs`).** The 343 calls drawn at once were nearly all the
  face windows turning in and out (`WindowRotateR` 77, `WindowCloseR`,
  `WindowRotate`: about y by 90°, `Dxl1`) and the choice plates. A
  picture turned about x or y is now drawn squashed: across by |cos y|,
  up and down by |cos x|, about its vertex (or centre), its children
  (`Game::relative_children`) about the same point; no perspective. Over
  time it is an `spmotion` (loop 2) with a key each 10° or so along the
  tempo's curve (the eases as conv-cons' `eased`), from the sprite
  placed at the larger end: a window turning in from edge-on is placed
  flat and starts at width 0; one turning away keeps its sprite and
  folds to 0, then goes at the next change. A turn that passes edge-on
  midway (a picture turned over in one go) is still mirrored at once
  (noted: none left in Django, 10 in 装甲悪鬼村正). Checked with
  `--real-time --shot-at` in dja002 (a `wait` patched in where
  auto-play would click on): the window unfolds about its centre with
  its face in 300 ms and folds away in 200 ms.

- **Sepia, and the rest of the "not converted" list (2026-10-16,
  shared NSS code).** The list's first two entries were already done:
  dja001's Black Hawk pan (`MoveBlackHawk`) is a looped motion since
  #105, the additive flashes `spblend` since #99. Its last, dja038_03's
  `CreateEffect("効果2K", 2000, …, "Sepia")`, is now `Kind::Tint`: while
  the effect shows, every picture below its priority gets `spfilter`
  greyscale and then a multiply by cream (255,235,195), its strength the
  effect's opacity; the black bars (10000) above it stay black.
  `Monochrome` (greyscale) and `NegaPosi` (negative) the same way
  (装甲悪鬼村正's 13 and 4; Rain, Drop, Lens still noted).
  `KitanoBlue` (装甲悪鬼村正's 4) followed: grey times a cool blue
  (170,200,255), a guess like sepia's; a tint over part of the screen
  (mc01_011vsa's 200×150 zoom box) is noted and left out, since it
  would tint the whole screen.
  The exe's sepia code (`parse_effect_name` → 0x425a10) is encrypted on
  disk, so the tone is ours: black stays black, grey 132 becomes
  (132,121,100) (headless test script). A change of tint alone is an
  `spfilter` on the layer (the engine filters the loaded picture again),
  no new `lsp`; a moving picture keeps its motion.

- **A face slides with its frame; a movie turned over plays mirrored
  (2026-10-16).** `WindowMoveM` moves only the frame (`frame04` in
  dja003_02, `frame03` in dja001_03): with `Game::relative_children`, a
  `Move` over time now takes the target's children along, so the face
  (`frame04/顔窓41`) gets the frame's `spmotion` instead of jumping to
  its end place in a cross-fade; the window slides of `窓/移動窓` and the
  camera moves move their children the same way (where a child also
  fades in or changes picture it is still a cross-fade, now counted in
  the "move or zoom over time with a fade" note: Move 11 → 54).
  `Rotate` y 180 on a movie (12 calls: dja003_02's, dja004's, dja025's
  and dja055's blood, muzzle flashes, `NMA射撃`): the exe's `Rotate`
  callback (0x439590) writes the angles into a movie object's quad (its
  field 0x804, set by `CreateMovie`) as into a picture's, so the original
  draws it mirrored; wine plays no movie there, so its shot settled
  nothing. Ours now plays a mirrored copy (`movie/<name>_x.mp4`, ffmpeg
  `hflip`/`vflip`, an `_alpha` movie's two halves each in place) and
  shows its last frame mirrored. Checked: a headless trace of the good
  path to dja004 plays `mv_2736c7e6_mute_x.mp4` and moves sprites 657
  (the face, 945 → 545) and 658 (the frame, 920 → 520) together; the
  ffmpeg graphs' frames match the source flipped (RMSE 0.009).

- **Easing (2026-10-16, shared NSS code).** `Move`'s and `Zoom`'s
  tempo (argument 4) ease their motions by the exe's curves since #112
  (`Axl`/`Dxl` 1-3 as powers, the four sine tempos as `spcurve` 70-73);
  `MoveCamera`'s (argument 5) the same for its children's motions. The
  tempo notes left are the 15 `MoveCamera`s not drawn over time.

- **dja055's chorus (2026-10-16).** dja055_02's song is 15 voice parts
  (`CreateUnit`: `voice/劇中歌パーツ/…`) and the harmonica (`dj02c`),
  all started at once and looping, the parts panned hard left or right
  and faded in and out one by one as the text goes on. The script makes
  the parts music objects, and ours plays one music at a time, so only
  the harmonica was heard. `django.rs` compiles `CreateUnit` as a
  sound-effect object: the parts play as looping `dwaveloop`s with their
  `chpan` and `chvolfade`s (they follow the sound-effect volume
  setting, not the music's). For them the shared channel allocator
  (`conv-cons`, `SeChannels`) no longer gives a channel holding a looped
  sound to another sound, and uses channels 11-49 when all of 1-9 loop.
  Checked headless (`--goto if40_else`): the 15 loops start on their
  channels, no missing file.

- **A fade to 0 stops the sound at its end (2026-10-16, shared NSS
  code).** The exe's `SetVolume` (`FUN_00457dd0`) runs a fade over time
  in a thread (`0x459980`, not a Ghidra function: `objdump -D` of the
  unpacked exe) that sets the target volume at the end and, at 0 on a
  playing sound (state bit `0x10`), stops it (`FUN_00458420`), as an
  at-once 0 does. Ours already stopped it with the fade
  (`chvolfade ch,0,ms,1`, `bgmfadeout`), but forgot it at once, so a
  `WaitPlay` right after went on at once; now it waits for the rest of
  the fade (or the sound's end if sooner), and a disused sound faded
  out is forgotten then. A locked sound is never forgotten (装甲悪鬼村正's
  `CreateBGM` music, which its `MusicStart` marks disused, had been
  forgotten after a one-shot play). Dry run: Django 8 new `delay`s,
  Muramasa 67 (mostly 350-500 ms) and 3 music fade-outs that its lost
  objects had missed.

- **Render modes (2026-10-16, shared NSS code, engine #34).**
  `Request(obj, AddRender|SubRender|NormalRender)` on a picture is drawn
  with `spblend` (Muramasa's History, "Render modes"): 15 `AddRender`
  uses in the dry run, the movies' `AddRender` now added on; the one
  `SubRender` is a movie, which keeps its darkened re-encode.
- **`SetPan` pans a sound (2026-10-16, shared NSS code, engine #35).**
  The exe's `builtin_SetPan` (`0040d140`) is shaped like `SetVolume`: a
  pan over time. CONscripter's new `chpan ch,pan[,ms]` pans a `dwave`
  channel (ramped in its track effect, saved, not the music); the
  converter emits it before the `dwave` of a sound that starts panned
  (`MusicStart`'s 4th argument) or over time on one playing, and
  centres a panned channel again for its next sound. Django: dja001_03's
  footsteps run in from the left (`chpan 4,-1000`); the title's four
  gunshots are `boot.nss`'s, not converted.
- **A small picture shakes alone (2026-10-16, shared NSS code).** The
  exe's `shake_frame` moves the shaken object's own place by ±(x, y),
  flipping each step, the size going from args 2-3 to 4-5. Ours shook
  only full-screen pictures (`quake`); a smaller one now gets a one-shot
  `spmotion` (`convert/shake.rs`, the keys `ShakeVLoop` uses, now with
  x), started when the next sync shows the picture if it isn't shown
  yet. Django: dja033's three sideways jolts of ガス (`立絵40`, 20-30 px
  over 1 s). The 30 s tremble in dja012_06 and dja029_02's jolts shake
  pictures zoomed past the screen, already a `quake`. A `Move` started
  at once on the same picture still replaces its shake (one `spmotion`
  per sprite).
- **Disused one-shot sounds stay until their end (2026-10-16, shared
  NSS code).** `Request(…, Disused)` on a one-shot forgot it at its
  start; the exe (`FUN_00436040`) deletes a disused object only once its
  sound has stopped, and `Request(…, Play)` on one still playing does
  nothing (`FUN_00457d20`). Now it is kept until its build-time end (the
  ops' time since its start: waits, fades, an hour per page), so
  `WaitPlay`, `Fade` and `SetVolume` on it still act. `WaitPlay`'s limit
  counts from the call, and the actions still running are shown over the
  wait, as `Wait`. Django: dja041's `delay 3906` after a page is gone
  (the sound played during it); the 4,345 ms wait before dj11 shows its
  2,500 ms fade over the wait (`print 17` + `delay 1845`).
- **`WaitPlay` on a sound waits (2026-10-16, shared NSS code).** The
  exe's `builtin_WaitPlay` (`0040b8a0`) waits until the sound stops, or
  at most its second argument in ms (`null`, `true` or none: no limit).
  Ours waited only for movies; now it waits for the rest of the sound
  (its length less the script's `Wait`s since it started), at most the
  limit: `delay`. dja003_02 and dja046_01 wait 500 ms after the shot,
  the ten bad ends' `サウンド４` 2,500 ms, and 8 more waits are for whole
  sounds (`delay` 739-4,345 ms).
- **The song types out (2026-10-16).** In the exe's `CreateText`
  layout (`layout_create_text`) a `.` adds a 100 ms pause before the
  next letter (`,` adds 500), timed in game time (play speed NORMAL =
  1×). Under wine (`nss-d26` boot, 45 shots a second of the lyric band)
  the first two lines came a letter every 125 ms, the full-width space
  included: 9 steps in 1,170 ms, then 10 in 1,213 ms. The first letter
  comes about one step after `Enter`. `django/song.rs`, through the
  game's `call` hook: `CreateText` keeps a `<I>` line's letters,
  `Request(…, Enter)` turns the line into a strip of its first 0, 1, …
  letters (`lsp ":a/N,125,1;…_typed.png"`, shown at once), and its
  `WaitAction` waits until the last letter (`delay`). Checked headless
  (`--goto if16_else`, `--shot-after` every 50 ms): one more letter
  every 125 ms from the `lsp`. Ours keeps 125 ms whatever the text
  speed setting.
- **The song in dja026 (2026-10-16).** `ShowWords` makes 13
  `CreateText("songtxtNN", 10301, center, 0 or 27, auto, auto,
  "<I>荒.野.開.く…</I>")`, takes them away (`Request(…, Erase)`), moves
  them by (15, 460) and shows two at a time (`Request(…, Enter)`), until
  a click. Under wine the lines are the letters alone, upright, white,
  their middles at the screen's (the 15 doesn't show) and their ink 16 px
  below their place; ours are drawn with `text_object.rs` without the
  markup and dots, 15 px left and 13 px lower than its layout (a
  measured nudge for these lines), now within 2 px of the original's.
  A `CreateText` picture stays hidden from `Erase` to `Enter`.
- **Windows clip their children (2026-10-16).** `CreateWindow` draws its
  children only inside it (`spclip`, from 装甲悪鬼村正's cockpit, #27),
  now for Django too, the window's place counted from its parents'
  (`窓/中窓` at 0,70 in `窓`). Under wine (dja002_02) a standing
  picture in `中窓` (`CreateMiddleWin`, the 800×400 band between the
  black bars) stops at y 70: the top bar stays black (mean 0 in the
  original and ours, 0.07 before). Pictures in a window still off the
  screen (`CreateWinBG` at x −800, before it slides in) aren't drawn. A
  window seen through a camera (`カメラ/移動窓`) doesn't clip (it would
  need the camera's zoom and depth). A sliding window's clip is its end
  place: the converter draws a window's slide as its children appearing
  at their end places (a cross-fade) or moving under a fixed clip, so
  the clip needn't move; where the original's moving window would hide
  a moving child's edge, the old picture sliding out covers it.

- **Text windows and face windows compared under wine (2026-10-16).**
  dja001_02 (earlier) and dja003_02: the three boxes' text starts within
  1 px of the original's; the three face windows (`frame03`-`frame05`,
  with the script's `WindowMove` nudges) within 410 pixels of the
  picture. What the comparison fixed: every face was one object
  (`"@frame03" + "/" + "$顔ナット名"` named `frame03/$顔ナット名`, so a
  new face replaced the last one and `ReplaceFace` deleted the new one):
  `Game::string_vars` is now on for Django; `SetAlias`'s second argument
  naming a variable (`"$顔ナット名"`) is its value, so `Delete("@顔窓41")`
  and `Rotate("@顔窓41", …, @180, …)` (a face looking the other way) find
  the face; a face is drawn with its frame's opacity (hidden while the
  frame is), and a picture turned edge-on (90° or 270° about x or y, as
  `WindowRotate`/`WindowCloseR` leave a frame) isn't drawn, nor are its
  children; a trailing `*` doesn't reach into the children of an object
  it matches (`Fade("@frame0*", …)` leaves the faces as they are), with
  `Game::family`. Under wine the original stalls at random in timed
  moves, fades, turns and in `WaitPlay` (silent sound never ends): the
  patched boot now makes them instant from the SKIP scene on and leaves
  out `WaitPlay`; `research/nitroplus/local/wine/pages_run.sh`
  (git-ignored) clicks through, shooting each page, and starts again
  when 8 clicks change nothing.

- **The ending roll's credits (2026-10-16).** `SetRoll` (dja056_09,
  `staff_roll`) fades the screen to red (#c80000) over 2 s and starts
  the process `ProcessRoll`: it stops the insert song (`dj_INS01`,
  started 5 s into dja056_08's last line, looping) looping, makes
  `cg/sys/good/EDROLL.jpg` (800×9000) at y 300 and moves it up by
  its height + 65 − 300 (to y −8465) over the song's time left
  (`RemainTime`) + the closing song's length (`dj_INS11`, 3:31) + 6 s;
  the closing song plays when the insert song ends. The first time
  through (`#ClearG` 0) `SetRoll` waits for the process, then for a
  click; later a click ends the roll at once. After the click the music
  fades out over 6 s and the picture over 4 s. `django/roll.rs`
  compiles it as one `Op::MusicRoll` at the process's `Request(Start)`
  (its `WaitAction` and `SetRoll`'s `WaitKey` are the op's): the engine
  reads how far the song is through its pass (`getbgmpos`, a new
  CONscripter command), starts the closing song at its end (`bgmlater
  …,0`), and a loop moves the picture (`amsp` at `gamespeed 100`:
  `SetRoll` sets the play speed to its normal for the roll; the first
  time in 50 ms steps, each until its time by `waittimer`, later by
  `gettimer` with a timed `btnwait` for the click) until that time +
  3:31 + 6 s, then waits for a click (a click on the later times is
  that click). A roll reached before the
  insert song started (skipping through the last page within 5 s)
  starts it from the beginning (`getbgmpos` is then minus the time
  until the `bgmlater` song starts). Checked headless (`--goto if46_end
  --real-time`, `%811=0`, `%2921=100`): the picture rolls from y 300
  to −8465 over the 379 s the run computes; without `--real-time` the
  auto-play goes through it in 19 s and on to the title, and with
  `%811=1` its click ends the roll at once; the run reaches the roll
  before the song's `bgmlater` fires, so the song started from its
  beginning there. The roll's speed against the original is unchecked
  (no wine run, stopped 2026-10-15). The original's picture keeps
  moving through the fade-outs after a click; ours stops where it is.

- **Looping shakes and the strobe (2026-10-15).** `ShakeVLoop`,
  `ShakeVLoop01`-`05` (9 processes, dja001, dja016, dja050): the
  process first makes its background (as its child: `Move` and
  `Delete` of the process move and delete it; before, these
  backgrounds weren't drawn at all), then loops `Shake(bg, ms, 0, y, 0,
  y_end, …)` and `Wait`s. `convert/loops.rs` (now shared NSS code) runs the part before the
  `while` and gives the background a looped `spmotion` through the
  same times: ±y every 100 ms (longer when 32 keys don't reach), y
  going to y_end (`Object::shake`, emitted where `sync` places it;
  `ShakeVLoop`'s own background, made before, stops at the process's
  `Delete`). A move of the background over time (`spmotion2`, as the
  slide-ins of dja050_02) replaces the shake. `strobe` (dja050_03, 3):
  a `Kind::Flash` 800×400 at (0, 70) at 300 for a frame, out over 80
  ms, a frame at 0, for ever (`Op::SpriteFlashing` times 0: `spalpha`
  loop 1), named as the process so its `Delete` removes it. Neither
  compared under wine; the shake is a turn every 100 ms, not the
  exe's frame-by-frame jitter. Since 2026-10-16 a loop process runs at
  its `Request(…, Start)`, not at `CreateProcess`: dja050_02 slides the
  empty process in first, and its background now appears where that
  put it when the shake starts, instead of sliding in.

- **The choice's shot (2026-10-15).** `ChoiceA02`/`ChoiceB02`
  (`function_select.nss`): the shot sound, the bullet hole
  (`dankon.png`, a mask under the chosen plate: black is see-through)
  with its top left 47, 41 above and left of the pointer, 300 ms later
  the other plate fades out (300 ms), and 500 ms after that the
  chosen plate and its text turn −90° about the x axis (their bottom)
  in 800 ms, `Axl2` (cubic ease-in), while fading out over 1000 ms.
  `ChoicePlates::shot` (`ChoiceShot`): `getmousepos`, the new engine
  command `spcut` cuts the hole into the plate's and the text's
  pictures (still under the pointer cell), then 15 steps of `spzoom`
  (height cos θ, no perspective) and `amsp` keep their bottom at the
  plate's. A choice made with the keyboard puts the hole where the
  pointer is, as the original does. Not compared under wine.

- **Auto mode stops at a choice (2026-10-15).** The scripts set
  `$SYSTEM_skip`/`$SYSTEM_text_auto` false before each choice
  (`function_select.nss`; put back afterwards only with the setting
  `#keep_auto_and_skip`, off by default) and set `$SYSTEM_menu_lock`
  and `$SYSTEM_text_auto` before the ending roll. CONscripter's
  `btnwait` now ends auto mode as it ended skip, and the converter
  emits `skipoff:autooff` (new command `autooff`) where the script
  sets `$SYSTEM_text_auto` false or `$SYSTEM_menu_lock` true
  (`Op::StopAutoSkip`; Muramasa's choices and locks get it too). The
  roll's click wait is a `click`, which ignores the right button: no
  menu there, as in the original.

- **Muzzle flashes (2026-10-15).** `GunFire(times, ms, opacity)`
  (`function_count.nss`, 11 calls in dja050) starts the process `Fire`:
  a white 800×600 colour `仮色` at priority 150, `AddRender`, set to
  `opacity` (0-1000) for a frame (`Draw`), faded out over `ms`, black
  for a frame, `times` times (1 to 100) while the script goes on. The
  converter (`django/battle.rs`, through the `call` hook) makes it a
  `Kind::Flash` object, placed by `sync` among the other pictures; its
  first placement emits `Op::SpriteFlashing`: `spblend <sprite>,1`
  (CONscripter: drawn added on, times its alpha) and `spalpha
  <sprite>,<times>,0,a,17,a,17+ms,0,34+ms,0`. Placed again (another
  layer, or after an opaque picture hid it) it stays at its end, since
  how far it got isn't known. A frame is taken as 17 ms; not compared
  with the original under wine. Checked headless: the 60-flash burst
  over the MG42 picture adds 65 to its brightness, the black bars and
  the counter (above it) unchanged.
- **Movies as sprite movies (2026-10-15).** A `CreateMovie` that plays
  while the script goes on (smoke, the Black Hawk, cuts under the text)
  was a strip of frames at 15 a second, the two long ones at fewer
  (2.3 a second for `mv_ブラックホーク`, 75 MB of image memory each).
  The NSS compiler now gives the engine the movie itself
  (`Assets::movie_sprite`: `movie/<name>_mute[_dark][_alpha].mp4`,
  `lsp` with `:c/1,0,0;` when it loops, `:c/1,0,1;` once), at its own
  frame rate; an alpha movie (colour on top, opacity below) is
  re-encoded with the opacity, inverted, as its right half for `:a`, a
  `SubRender` one with its colour black. Paused and finished movies
  stay one frame. Checked headless with a test script: the smoke over a
  picture, a looping pan moving between shots, Muramasa's dark smoke
  darkening.
- **The 400-vs-3 counter (2026-10-15).** `function_count.nss`'s counter
  (three digits from `cg/sys/count/count.png`, 9 at the top, in a
  194×52 window at 296,10, over the top black bar) is drawn by the
  converter (`django/counter.rs`, through the game's `call` hook):
  each `CounterPlay(next, time, shake, attack)` becomes one strip of
  frames (33 ms) of the window, shown once while the script goes on
  and stopping on the new count (the NSS compiler's new `Kind::Strip`,
  an animated sprite; placed again on another layer it shows its last
  frame): the places that change roll down a digit a step, the shakes
  (`shake` 1-4) jitter the digits, and from the 380th man the digits
  fade out over 2 s as the original's do. `CountFadeIn` (three blurred
  zeros flying in from 610 and 146 as they shrink from five times their
  size, a flash over the places) and `CountFadeOut` (at 400: the 4 and
  the last 0 flying apart as they grow and fade) are strips over the
  top of the screen, drawn at half size and zoomed 200%. A
  `CounterPlay`, and the script's `WaitAction("カウント１")`, wait for
  the last one's end, counting the waits and fades since (a page in
  between ends it). Under wine (fade-in's zooms made instant) the
  counter after `CounterPlay(7, 0, 3, 1000)` reads 007, and ours
  matches it pixel for pixel.

- **The 400-vs-3 battle's movies (2026-10-15).** dja050's six `VS400`
  calls play a movie while the player holds the button down (a
  full-screen choice: `MouseClick` resumes the movie, the release pauses
  it), the "hold" prompt (`PushShow`, ten pictures at 690,20, 100 ms
  each) going round over it, its gunfire sound with it: paused and
  resumed with the movie, or (the fourth argument 0) started again
  looped at each press and faded out over 500 ms when let go. The
  user chose to keep the holding. CONscripter's new `mpeghold` does it
  (Linux/macOS and the web build; the mouse, a touch, Enter or Space),
  from `Op::HoldMovie` (`django/battle.rs`, through the game's `call`
  hook, which now gets the call's arguments). Checked in the engine
  under Xvfb and in headless Chromium: the movie stands on its first
  frame until pressed, goes on only while held; the movies draw their
  own 0-400 counter.

- **Music processes (2026-10-15).** Four `CreateProcess` functions only
  play music on a timer: dja050's and dja056's themes (an intro, its loop
  7,720 and 10,700 ms later), dja056_08's insert song five seconds after
  a line, dja003's jingle in two parts. A process whose function calls
  only `SoundPlay`, `MusicStart`, `Wait`, `WaitPlay` and `CreateBGM` is
  run when `Request(…, Start)` starts it, its waits counted on a clock
  (`WaitPlay` by the Ogg's length), and each music becomes `bgmonce` or
  `bgm` at once or CONscripter's `bgmlater ms` after
  (`nitroplus/convert/process.rs`). The other processes are still noted.
- **Music after dja009 (2026-10-15).** dja009 ends with `Delete("*")`,
  which deleted every object, the music objects (`dj01`…, made once by
  `InitBGM` before the first chapter) with them: 30 of the 50 chapters
  with music played none. The script waits for that music right after
  the `Delete("*")` (`WaitPlay("@dj*")`), so in the original it deletes
  only the chapter's own objects, as a chapter's end does
  (`Game::chapter_objects`).
- **Motion (2026-10-15).** `Move`, `Zoom` and `MoveCamera` over time were
  drawn as cross-fades from the old place to the new; the congratulations'
  pans came out as cuts. Now a picture that only moves or zooms is moved
  by the engine (`spmotion2` in its keep mode, a zoom about the centre
  of its place), shared with Muramasa's compiler; CONscripter steps
  motions during `print` effects too (fe00598), so a pan that starts with
  a fade runs through it.
- **Trace checker (2026-10-15).** The page past the window (dja023_02
  `[text029]`, a 21-character line in `box02`'s 20 columns) fits: a
  horizontal box holds as many rows as end inside it (five in 130 px at
  a 27 px pitch, not 130 / 27 = 4). Old backgrounds left up under newer
  ones aren't drawn while an opaque full-screen picture covers them (the
  checker's "crowd" findings are scenes with five or six characters at
  once, and stay).
- **Bad end 10's picture (2026-10-15).** Each death is
  `if($死亡NN == 1) { return; }` in the chapter right after its choice,
  then the caller's `if($死亡NN == 1) { call_chapter …be.nss; }`. The
  caller's branch started from the state the *other* answer left (its
  pictures hung under `窓`), so in path 10 the good path's 荒野
  background stayed over the bad end's title. The branch that returned
  now hands its own state (with the chapter's objects deleted) to the
  caller's `if` on the same test.

### Trace diff, first 50 pages (2026-10-18)

The Status item as it stood before the 1,049-page diff:

Trace diff against the original (`ntrace.sh`, wine section; first 50
  pages from dja001, `docs/new-game.md` step 23); all pair. 2026-10-18,
  50 lines left (`missing` 145, `extra` 9, `time` 21), by cause:
  - Not findings: page 1's boot (the original's trace has a patched
    boot: no logos, so ours' `boot.nss:128-171` are `extra`, as is
    `TitleSet`'s opening now; `SetVolume` 0 on created
    sounds and 0 ms fades give no command of ours); `call_chapter`
    lines; 0-72 ms gaps; page 18's `WaitFade` (auto mode's reading
    time differs, so the 3 s fade had or hadn't ended).
  - The exe's waits poll (the wait helper `00433bf0`: a 100 ms wait on
    an event set only by requests, then a 32 ms sleep): a waited
    action returns at the first poll after its end, under wine 100 →
    225 ms (every `TypeBegin`/`TypeEnd` box fade, 21 of 22), 300 → 431
    (`Zoom` function.nss:877), 500 → 532-622 (function.nss:911, 948),
    1000 → 1026-1031; ours ends with the action (109). The same code
    runs on Windows, but whether its timer gives the same steps is
    unchecked; not modelled (it would slow every NSS game's text
    window ~125 ms a fade): the user's call.

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run --source-map out/django.map
research/tools/scratch.sh <a full build> <scratch> out/django-dry/0.txt
cp out/django-dry/ons_args <scratch>/   # the dry run's encoding, not the build's
<scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/django.map --pages 50 --ms --thread 0
```

### One `*conv_boot_end` (2026-10-18)

The boot logos' subroutine was jumped over to a label of the same name
as conv-cons's boot guard end (`*conv_boot_end`, defined twice; the
engine keeps the first). It is `*conv_boot_body_end` now (dead code
today: the jump follows the story's last jump).
