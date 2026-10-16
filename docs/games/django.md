# 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus NSS)

Zoku Satsuriku no Django: Jigoku no Shōkinkubi (Nitroplus, 2008), the
fourth game. It uses the same NPA archives and NSS script language as
装甲悪鬼村正 (`docs/games/muramasa.md`, the reference for the engine),
in an older version (`NitroplusSystem2.0`), so it reuses
`crates/conv-engines/src/nitroplus/`, with its own rules in
`nitroplus/convert/django.rs`. The general process is in
`docs/new-game.md`, general lessons in `PLAN.md` §13.

- **Status:** started 2026-10-15 (see "Status" below). Branch `django`.
- **Game files:** `<game>` is the install folder holding `Django.exe`,
  the `*.npa` archives and `movie/`, **with the 1.01 update applied**
  (`DjangoUpdate101`: its `Django.exe`, `nss.npa` and `cg2.npa` replace
  or join the installed ones). Version 1.00's scripts differ in about 60
  files. To keep the install untouched, make a folder of symlinks to the
  install's files with the update's three files linked over them.

## Scope

Asked 2026-10-15: the story first, the web build as the main platform,
and the extras (CG library, recollection, sound library) wanted too.
Running the original under wine with a no-CD exe the user owns is
allowed.

## Build

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
    -o out/django-web/onsyuri_index.json --title 続・殺戮のジャンゴ --lazyload
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
| `SetShade(…, HEAVY)` | 1 (dja048) | a blur preset | not done | `SetShade` 1 |
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
  can be compared. Start again when a run stalls.
- Clicks: `xdotool mousemove X Y`, then `mousedown 1; sleep 0.15;
  mouseup 1` (a bare `click` is often lost).
- **Patched boots**: `research/nitroplus/django_patch.py <table> <game>/nss.npa
  nss-patch.npa [CHAPTER [SKIP]]` writes an `nss.npa` without the logos,
  with every ending marked seen (the title shows EXTRA and LINER NOTES),
  and with CHAPTER, START plays from that chapter file (`dja004`), or
  from its call naming SKIP (`_05_`). `<table>` is `npa::DJANGO_TABLE`
  as a 256-byte file; `npa.py` takes it in place of the exe. Link the
  copy as `nss.npa` in `C:\django\`.

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
  LIBRARY, SOUND LIBRARY, EXIT. The doors swinging open and the zoom
  through them are left out.
- **CG library** (`extra_cg.nss`, `PictureGallery`): five categories
  (wanted posters), 20 thumbnails a page five across, ARTIFACT's three
  pages with BACK (next page) and NEXT (the one before). The script
  locks nothing: every picture is open once EXTRA is, except おめでとう画像
  (`#ClearAll`, set by the title's congratulations after every
  ending). Pictures larger than the screen are shown at half size; tall
  ones scroll up from their bottom. Beside the original under wine
  (2026-10-15): the page and the viewer match pixel for pixel, but for
  the rifle animation over a category under the pointer (left out).
- **Sound library** (`extra_sound.nss`, Muramasa's `SoundRoom`): 35
  songs (1.01's `cg2.npa` titles), the one playing in white with the
  running horse; pixel for pixel the original's but for the horse's
  frame. Ours keeps the title's music in the extras where the original
  starts `dj_INS12`, and a click on the song playing stops it.
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

## Status

### Queue (2026-10-16)

- Merged: windows clip their children (#45), the song's lyrics in
  dja026 (#47), unread-argument notes ranked in Next (#51), the song
  lines type out (#55), `WaitPlay` on a sound waits (#70). Open:
  disused one-shot sounds kept until their end (#76).
- After engine #2 / converter #6: the 60 moves and zooms that also fade
  or change picture as one `spmotion` with alpha; `Rotate` over time (343).
- `spgrab` (engine #32, merged) could draw the moves that fade (the 60
  above) as a copy fading out over the moved picture: not used yet.
- The unread-argument notes (#30) are sorted into Next, item 6: easing
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
| Pictures, sound in scripts | 🟡 Black bars, face windows in their frames, standing pictures, backgrounds; pictures turned over (`Rotate` by 180°, 315 calls: characters facing the other way; 12 movies) drawn mirrored (dja004's choice now as the original's); children of a picture moved over time (a face in its frame) move with it; cameras (`CreateCamera`, `MoveCamera`, 365 calls): a camera's child of priority p drawn about the screen's centre at (its place − the camera's) × z / (1000 − p), as large, z the camera's zoom (priority is depth: measured under wine with boxes at four priorities and six camera settings, and dja001's carriage scene then matched the original's frames); moves, zooms and camera moves over time (1,765) as real motion: the engine moves the picture (`spmotion2`, keeping its last place) while the script goes on, also through the cross-fade of what else changes; 60 that also fade or change picture are still cross-fades; a picture under an opaque full-screen one isn't drawn (old backgrounds the script leaves up under `窓`); the 400-vs-3 battle's muzzle flashes (`GunFire`) as a white sprite added on (`spblend`), flashing its number of times while the text goes on (`spalpha` keys), and its strobe the same way until deleted; the looping shakes (`ShakeVLoop*`) as the background's looped `spmotion`, and dja001's opening slideshow (`MoveBlackHawk`: the pictures fading and sliding in turn) as looped motions (`convert/loops.rs`, shared with 装甲悪鬼村正); every sound is found |
| Choices | ✅ Plates as picture buttons, text placed as the original's (compared under wine) |
| System screens | 🟡 From the game's pictures (`django/screens.rs`): title, right-click menu, save/load (3×20 slots), yes/no questions, text log, settings (bullet rows as `SliderSteps`), the click-wait bullet after the last character (`getcursorpos` in the bar's wait routine); compared under wine: right-click menu, save, log, settings (text speed starts at level 5 as the original's; the two SAVE-DATA question rows drawn as they stand, ON chosen; the display options and version panel left out on purpose) |
| Extras | 🟡 Menu, CG library, sound library, recollection, each compared with the original; left out: the doors' animation, the rifle over a category |
| Saves | 🟡 `--roundtrip-every 100` over the first 1,500 pages: no save/load mismatch; saves from the screens not tried by hand |
| Web build | 🟡 Plays in a browser (title, story pages); saves, sound and touch not checked yet |

### Next

1. The trace checker's remaining findings: "sounds on two channels at
   once" are the script's own (the same shot or horse on two sound
   objects, started 500 ms apart) and stay.
2. The rest of the "not converted" list, by what shows most:
   dja001's Black Hawk pan, then additive flashes, sepia.
3. The 60 moves and zooms that also fade or change picture (still
   cross-fades), and `Rotate` over time (343, drawn at once).
4. Extras: the doors' animation and the menu's own music (left out).
5. From the exe's coverage table ("The exe"): `SetShade HEAVY` (one use; sepia is
   item 2's).
6. The unread-argument notes (`<Builtin> argument <i> ignored`;
   dry run 2026-10-16: 33 kinds, 18,947 notes, fewer than #30's 39 and
   21,859 since later PRs read more), most visible first; counts are
   compiled occurrences, what the original does is from the scripts and
   `research/nitroplus/builtins.md`:
   3. **`Shake` 6 `n` and 7 tempo** (58 each): `n` (200-1000; above
      1000 the call does nothing) sets the jolt's rhythm and the tempo
      its decay; ours shakes every 16 ms, dying linearly. `n`'s meaning
      is lost in the exe's x87 code: one wine shot of a 200 and a 1000
      shake would settle it.
   4. **`Fade` 3, tempo** (77 non-`null`, mostly `DxlAuto`/`AxlDxl`): an
      eased fade; ours linear. Hard to see.
   5. **`Rotate` 5, tempo** (264): moot until `Rotate` over time (item 3)
      is converted, then as item 6.1.
   6. **`CreateWindow` 6** (975): `true` on every story window (the
      boxes, `中窓`, `移動窓`), `false` on the full-screen `窓`, the 0×0
      `音`, the system `WND` and the battle's `カウンタ窓`. Not the clip:
      `カウンタ窓` must clip its digit strip to 52 px. Its digits sit at
      x 346-410, inside the window only counted from the screen, so
      `false` may mean children placed from the screen rather than the
      window's corner. Nothing shows: `窓` is at 0,0 and the counter is
      `django/counter.rs`'s own.
   7. **`SetBacklog` 2, the speaker** (75; 73 `null`): two calls in
      `dja035_01` name ジュリアン and イライザ; what the original's log
      does with it (a name line, or the voice switches by character) is
      unchecked.
   8. **No change** (the original passes values that do what ours
       does; candidates for `UNREAD_HARMLESS`): `WaitAction` 1 (5,852),
       `WaitFade` 1 (1), `WaitMove` 1 (2), all `null` (no limit);
       `SetFrequency` 0, 1, 3 (1,283 each: `MusicStart`'s object, 0 ms,
       `NULL`; the speed, argument 2, is 1000 everywhere and checked);
       `LoadText` 0 (740, the
       script unit's name; the text is compiled where it stands);
       `CreateText` 5 (13, `Auto` height: dja026's song lines and the
       ending's two texts); `MoveCamera` 1 and 6 (15 each, a camera with
       no children yet: nothing moves); `CreateEffect` 2-5 (1, the
       `Sepia` effect, not converted itself: item 2).

7. Sounds kept until their end (shared NSS code, `convert/sounds.rs`):
   1. **Time across runtime branches**: the sound clock adds up the ops
      of both branches of a runtime `if`/`else`, so a sound started
      before one may be taken as ended too soon. Fine as is.

## History

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
