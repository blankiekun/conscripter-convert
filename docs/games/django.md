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
python3 -I engine/src/onsyuri_web/conscripter_index.py -i out/django-web \
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
eleven paths in parallel and prints each run's pages, choices and the
bad-end pictures it showed: the good path none, `badNN` only its own.

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
  object deletes its children. Without it, pictures hung under a deleted
  window stayed up for whole chapters (two extra characters at
  dja004's first choice; image memory peaked at 131 MB, now 109).
- **A chapter file's own objects are deleted when it ends**, except those
  hung under an object made before it. *Inferred, not checked in the exe
  or under wine*: `function.nss` makes `窓` and `音` windows "to hang
  pictures and sounds on between files", and the fade to black `黒幕`
  at the end of `dja001_…_01` is never deleted otherwise.
- **System screens** (`django/screens.rs`): what is ours rather than
  the original's is written in each function's comment (where the log's
  text sits, the hover colour of the choices, the save screen's
  see-through parts for what Django lacks).
- **Choices** (`choice_style`): `SetChoice02`'s 530×68 plates one under
  the other from (138,244), white 22 px text. Where the text sits on the
  plate (40 px in, centred vertically) and the hover colour (the
  original swaps the plate's picture) are ours until measured.

General NSS fixes made for Django that every game gets: hex numbers
(`0x000000`), a string naming a variable as `CreateSound`'s path
(`"$場所指定"`), a voice `src` without its `voice/` folder, `CreateWindow`'s
place and the six-argument `LoadText` (a box's text layout), a runtime
`if` whose branch ends in `return`. CONscripter: a new page erases the
last one's text where the window picture doesn't reach (Django draws its
text on the scene, with no window picture).

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
- Choices: the option is centred on its plate (MS ゴシック 22, white),
  and the plate under the pointer turns to `select002.png`. Ours puts the
  text 40 px in and colours it.

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

## Status

| Area | Status |
|---|---|
| NPA archives (key), images, sounds | ✅ Every file decodes |
| Movies (MPEG-1) | 🟡 Found in `<game>/movie` (the default) and drawn as Muramasa's compiler draws them (strips of frames at 15 a second); two long ones get fewer frames so that their JPEG strip stays within 65,535 px (`mv_ブラックホーク`, 70 s: 163 frames, 2.3 a second) |
| Script parsing | ✅ All 249 files; 56 places that don't parse, none in the story |
| Story control flow (56 chapters, 10 choices, bad ends) | ✅ Compiled whole (`compile_story`); in the engine the good path plays to the end (6,496 pages) and each wrong answer ends the game with its own bad end (`research/nitroplus/django_routes.sh`, 2026-10-15) |
| Text | 🟡 In the boxes' places (box00: 39 columns at 50,472, white on the black bar); 3,424 voiced lines; not compared with the original yet |
| Pictures, sound in scripts | 🟡 Black bars, face windows in their frames, standing pictures, backgrounds; what Muramasa's compiler does for effects (moves and zooms as cross-fades); every sound is found |
| Choices | 🟡 Django's plates; text place and hover colour guessed |
| System screens | 🟡 From the game's pictures (`django/screens.rs`): title, right-click menu, save/load (3×20 slots), yes/no questions, text log, settings (bullet rows as `SliderSteps`), the click-wait bullet; not compared with the original (it wants its disc under wine) |
| Extras | 🟡 Menu, CG library, sound library, recollection, each compared with the original; left out: the doors' animation, the rifle over a category |
| Saves | 🟡 `--roundtrip-every 100` over the first 1,500 pages: no save/load mismatch; saves from the screens not tried by hand |
| Web build | 🟡 Plays in a browser (title, story pages); saves, sound and touch not checked yet |

### Next

1. Under wine (recipe above): the choices (centred text, the plate
   picture under the pointer), the save/load screens, the log and the
   settings beside ours; the chapter-end rule above.
2. The rest of the trace checker's list (`cons_trace_check.py` on
   `out/django-routes/good.jsonl`): one page runs past the window
   (dja023_02 `[text029]`: a 21-character line in box02, 20 columns of a
   fixed-width font; the original's proportional ＭＳ Ｐ ゴシック fits it);
   sounds started twice; pictures left up across 150 pages (old
   backgrounds hung under `窓` until their chapter ends, covered by the
   newer ones); image memory peaks at 131 MB (a long movie's strip).
3. The text window and name plates (`box00`-`box05`, face windows
   `frame03`-`frame05`) measured against the original under wine.
4. The ranked "not converted" list: `MoveCamera`, `CreateTexture` from
   the screen, `CreateColor` colours, movies.
5. Standing pictures placed
   through the camera helpers (`CmrSetBu…`, `MoveCamera`) still end
   above the bottom bar.
6. Camera moves (`MoveCamera`, 365 calls) and moves and zooms over time
   (drawn as cross-fades), shared with Muramasa's compiler.
7. Extras: the doors' animation and the menu's own music (left out).
8. The title's congratulations (`TitleCongratulations`, sets
   `#ClearAll`) and the click-wait bullet after the text.
