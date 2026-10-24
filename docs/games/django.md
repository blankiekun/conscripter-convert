# 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus NSS)

Zoku Satsuriku no Django: Jigoku no Shōkinkubi (Nitroplus, 2008). Same
NPA archives and NSS language as 装甲悪鬼村正 (`docs/games/muramasa.md`,
the engine reference), older version (`NitroplusSystem2.0`): reuses
`crates/conv-engines/src/nitroplus/`, own rules in
`nitroplus/convert/django.rs`. Goals: the story, the web build as main
platform, the extras (CG library, recollection, sound library). No manual.

`<game>` is the install folder (`Django.exe`, `*.npa`, `movie/`) **with
the 1.01 update applied** (`DjangoUpdate101`: its `Django.exe`, `nss.npa`,
`cg2.npa` replace or join the installed ones; 1.00's scripts differ in
about 60 files). Work from a folder of symlinks to the install's files
with those three linked over them.

## Status

Active.

| Area | State |
|---|---|
| Archives, images, sounds, movies, script parsing | ✅ |
| Whole story (56 chapters, 10 choices, 10 bad ends; 6,496 pages on the good path) | ✅ `django_routes.sh` |
| Text, choices, pictures, motion, cameras, sounds | ✅ |
| System screens (title, menu, save/load, log, settings with 各キャラ voices, yes/no) | ✅ headless, compared under wine |
| Extras (CG, sound, recollection) | ✅ headless |
| Save round trips | ✅ `--roundtrip-every` |
| Web build | 🟡 title and story pages play in a browser; saves, sound, touch, the log's scrolling unchecked there |
| Progression (`#Clear` flags to title and extras, congratulations) | 🟡 from the scripts, not played through |
| `CreateEffect` Sepia (the only `CreateEffect`, dja038) | ❌ drawn as nothing |

Next:
1. Play the web build through (saves from the screens, sound, the log, a
   bad end and back, the extras); fix what blocks or shows wrong.
2. Play to the end with every bad end seen; check the title's EXTRA,
   LINER NOTES and the congratulations.
3. Sepia (dja038).
4. Release.

## Build

`conscripter-convert build <game> -o <output>` recognises the game and
runs `convert-nss` with `--title 続・殺戮のジャンゴ`; convert options
(`--dry-run`, `--font`, …) follow `-o`. Script only (seconds; use it for
anything about logic):

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run
```

Without `--file`, `convert-nss` compiles the whole story
(`dja000_開始スクリプト.nss`, scene `SelectStoryMode`). The full build drops
`--dry-run` and adds `--font <a Japanese TTF, e.g. ipag.ttf> --title
続・殺戮のジャンゴ`.

**Web build** (loose files, about 680 MB in 5,700 files):

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-web \
    --font <ipag.ttf> --title 続・殺戮のジャンゴ --loose --webp --webp-quality 80
python3 -I engine/src/cons_web/conscripter_index.py -i out/django-web \
    -o out/django-web/cons_index.json --title 続・殺戮のジャンゴ --lazyload
cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/django-web/
python3 -m http.server 8000 -d out/django-web   # then open /conscripter.html
```

## Testing

- **Routes** (full build in `out/django-cons`):
  `research/nitroplus/django_routes.sh out/django-cons <conscripter>
  out/django-routes` plays the good path and the ten bad ends in
  parallel (`django_paths.py` writes the `--choices` files), printing
  pages, choices and bad-end pictures per run. Green: the good path
  shows none, `badNN` only its own.
- **Save round trips:** `--roundtrip-every` (`docs/tools.md`).
- **Wine** (last resort): Muramasa's silent off-screen recipe
  (`docs/games/muramasa.md`, "Testing") with the no-CD 1.01 exe in a
  scratch game folder, plus: wine's virtual desktop (`wine explorer
  /desktop=django,1024x768 "C:\django\Django.exe"`); it advances only on
  input (nudge the pointer ~30 times a second); click as `mousedown`,
  150 ms, `mouseup`. Timed actions hang at random (`SuspendThread` on a
  thread waiting for the object mutex); `ntrace.dll` works around it
  (comment in `research/nitroplus/tracedll/ntrace.c`).
- **Patched boots:** `research/nitroplus/django_patch.py <table>
  <game>/nss.npa nss-patch.npa [CHAPTER [SKIP]]` (no logos, every ending
  seen, START from a chapter file or its call naming SKIP; `<table>` is
  `npa::DJANGO_TABLE` as a 256-byte file).
- **Trace DLL:** `GAME=<game> EXE=<unpacked 1.01 Django.exe>
  research/nitroplus/tracedll/ntrace.sh -o orig.jsonl --pages 50` plays
  from dja001 by itself; `SPEED=8` for speed, `--frames` for per-frame
  `obj` records, `--scene-dumps`/`--dumps` for memory snapshots
  (`docs/tools.md`, "Matching the original"). Per-scene runs:
  `scene_saves.py --at end`, then `scene_chain.py --quiet 120`. Compare
  with ours:

  ```sh
  target/release/conscripter-convert convert-nss <game> -o out/django-dry --dry-run --source-map out/django.map
  research/tools/scratch.sh <a full build> <scratch> out/django-dry/0.txt
  cp out/django-dry/ons_args <scratch>/
  <scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
  research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/django.map --pages 50 --ms --thread 0
  ```

  Frame by frame: `research/tools/frame_diff.py <segments> <ours> --map
  <map> --groups 15 --alias LoadFrame=cg/sys/windowframe.png --ignore
  cg/sys/push/`. The exe's waits poll (100 ms on an event, then a 32 ms
  sleep, helper `00433bf0`), so its waited actions end later than ours
  (100 → 225 ms under wine); not modelled.

## Script language

Differences from Muramasa (`conscripter-convert nss check|refs <dir>`):

- **`cut name { … }`** blocks, called by **`call_cut name;`**: a scene
  plus `call_scene`. Every story file is `chapter main {
  InitChildChapter(…); call_cut main_stage; }` plus its cuts.
- **`wait 1000;`** (once): `Wait(1000)` without parentheses.
- **Pages joined by a commented-out block** (`{/*</PRE> … <PRE
  box00>[text011]*/}`): the parser skips `{/* … */}` when it looks for
  `</PRE>`.
- **`TypeBegin("@box00", "@text005")` precedes** the text block it shows;
  the compiler keeps the Enter and shows `[text005]` when it runs.
- **Helpers** live in `function.nss` (included by `boot.nss`),
  `function_select.nss` (choices) and `function_badend.nss` (both
  included only by chapter files); function lookup also searches the
  current file's own `#include`s. `function.nss`'s uncalled `cut
  layer_number` holds the developers' layer numbers.
- **Unparsed** (56 places, none in the story): the system screens
  (`sys_backlog`, `sys_load`, `sys_save`), those notes, a stray
  `#base_path`.

Where the exe and the converter differ (all else the scripts call
agrees; `NoLog`, `PushText`, `NoIcon` do nothing in both):

| Feature | Uses | The exe | The converter |
|---|---|---|---|
| `Request` `AddRender` / `SubRender` / `NormalRender` | 15 / 1 / 3 | additive / subtractive / normal blend | `spblend` add; the `SubRender` movie darkened |
| `Request` `Smoothing` / `UnSmoothing` | 20 / 3 | texture filtering | nothing |
| `CreateEffect(…, "Sepia")` | 1 (dja038) | sepia over the screen | not done |
| `CreateEffect` Rain / Drop | 0 (the 1.01 `nss.npa`; the counts once here were Muramasa's) | `parse_effect_name` 00422110 has Rain 00427810 (Muramasa's code), Drop 00425c30, FloatBubble 004273e0; no Lens | none needed |
| `$SYSTEM_menu_lock` = 1 (title, ending roll) | 8 | no right-click menu, skip off | `skipoff:autooff`; the roll waits on `click` |

The choice's shot (`function_select.nss`: a bullet hole where the
pointer clicked, the chosen plate tipping over) is drawn with `spcut`
and `spzoom` steps. System screens' tags and built-ins are replaced by
screens the converter draws itself (`django/screens.rs`; what is ours
rather than the original's is in each function's comment).

### How the compiler is set for Django

`nitroplus/convert/django.rs` (`DJANGO`) gives each rule's reason in its
comment. In short: 800×600; `boot.nss` as the helper library; children
placed from their parent, following and deleted with it (`Game::family`,
aliases, `*` wildcards in names); a chapter file's own objects deleted
when it ends, except those hung under an older object (`窓`, `音`);
`CreateTexture` from a `LoadImage` picture draws it (the face windows'
frame); a `Zoom` keeps the `SetVertex` point in place; text 7 px below
its box (`text_drop`); choices via `choice_style`, `ChoicePlates`. A new
page erases the last one's text where the window picture doesn't reach
(Django draws text on the scene).

## Extras

`boot.nss` shows EXTRA and LINER NOTES once the story is finished
(`#ClearG`). Converted in `nitroplus/convert/django/extras.rs` on
Muramasa's `PictureExtras`:

- **Menu:** the saloon, CG LIBRARY, RECOLLECTION LIBRARY, SOUND LIBRARY,
  EXIT, music `dj_INS12`; the doors swing open and the saloon zooms
  through them into a library (`ExtrasPassage`).
- **CG library** (`extra_cg.nss`, `PictureGallery`): five categories, 20
  thumbnails a page; all open with EXTRA except おめでとう画像
  (`#ClearAll`). Pictures larger than the screen show at half size, tall
  ones scroll. The rifle animates over the hovered category.
- **Sound library** (`extra_sound.nss`, `SoundRoom`): 35 songs from
  `cg2.npa`. A click on the playing song stops it (the original
  restarts it).
- **Recollection** (`extra_recollection.nss`, `PictureReplay`): eleven
  scenes, each `extra_N_*.nss` compiled again as a subroutine
  (`Game::subroutines`); save and load refused during a replay.

## Progression variables

| Variable | Meaning | Set | Read | Kept in |
|---|---|---|---|---|
| `$死亡01`-`$死亡10` | The player chose the deadly answer | the choice's file | the chapter file, `main_root` | save |
| `#Clear01`-`#Clear10` | That bad end was seen | `main_root` | title, extras | system |
| `#ClearG` | The story was finished | `main_root` | title (`TitleCongratulations`), extras | system |
| `$ClearL` | The game just ended | `main_root` | title | save |
| `#ClearAll` | The congratulations were shown | `TitleCongratulations` | title, CG library | system |

The story's end (`conv_run_end`) runs `InitBGM` and
`TitleCongratulations` (`Game::story_end`; it plays once every ending
has been seen, right after one), then returns to the title.

## Story structure and walkthrough

`boot.nss` shows the logos and the title (`TitleSelect`); a new game
calls `dja000_開始スクリプト.nss`, whose `SelectStoryMode` calls
`main_root`: the 56 chapter files `dja001`…`dja056` in order, each
calling its scene files (`dja001_フランコ馬車を襲う_01_…`; 159 in all).
No routes: ten two-way choices (`SetChoice02`), each with one deadly
answer (the `…be_…` file, then `ShowBadEnd`).

Deadly answer (flag), by chapter; the other answer goes on: dja004
手を引いてくれと頼む (死亡01), dja011 殺さない (死亡05) and 信用する
(死亡02), dja025 ともかく撃つ (死亡06), dja027 まだ退かない (死亡03),
dja033 応戦する (死亡04), dja041 ４５口径 (死亡08), dja047 鎖を解く
(死亡07), dja049 付き合いきれない (死亡09), dja053 その通り。あいつらは
馬鹿だ (死亡10). The alternatives are the scripts' `SetChoice02` lines.

## Executables

| File | Version | sha256 |
|---|---|---|
| `Django.exe` 1.01 (`DjangoUpdate101`, 1.6 MB, packed) | 1.01 | `b40fb35cacc672c5ea377ab86e5358fbe85b2a05c30edf5981fd509c5fc390e7` |
| `Django.exe` 1.00 (installed, 3.1 MB) | 1.00 | `d4bed22dd7062880b96476fe423f495cc1872bfb6a3b17c593c8313976040537` |
| `Django.exe` 1.01 unpacked (no-CD, 0.9 MB) | 1.01 | `3dbfe8f30e07a244ad294c80c0457b87bc3b1a49af4b6b0c86bf7140d63bc7bd` |

Every address here and in `research/nitroplus/django-names.tsv` is the
unpacked exe's. The 1.01 exe is packed (every section encrypted, a disc
check before the entry point; 1.00 has SETTEC's `.settec` section) and
no public unpacker exists; the user's unpacked one is that image dumped
at its entry point 0x4b4726 with imports rebuilt. Ghidra (about 2
minutes the first time):

```sh
research/tools/ghidra/ghidra_export.sh <unpacked 1.01 Django.exe> \
    research/nitroplus/local/ghidra-django --names research/nitroplus/django-names.tsv
```

What the named functions do and how this engine differs from Muramasa's:
`research/nitroplus/builtins.md` ("Django.exe"). Decoders:
`research/nitroplus/dispatch.py <export>/decomp/00404320_*.c`
(built-ins), `evalnames.py` on `0042b150_*.c` (values),
`stackstrings.py <export>/decomp` (`$SYSTEM_` names), `charchains.py
<export>/decomp` (tags, `Request` actions, constants). Findings of the
exe mapping: `docs/games/django.findings.tsv`.

## Files and formats

Screen 800×600, horizontal text.

| File | Size | Contents |
|---|---|---|
| `Django.exe` | 3.1 MB (1.6 MB for 1.01) | The engine, `NitroplusSystem2.0` |
| `nss.npa` | 756 KB | 249 `.nss` scripts (1.01), Shift-JIS, CRLF |
| `system.npa` | 1.4 KB | `system.ini`: 800×600, font ＭＳ Ｐ ゴシック 18 px, ruby 45 %, volumes, click icons, voice names |
| `cg.npa` | 465 MB | 1,817 PNG/JPEG/BMP (+1 WAV). Folders: `sys`, `bu` (standing pictures), `bg`, `ev`, `thum`, `st`, `kw` (face windows) |
| `cg2.npa` | 1.6 MB | 1.01 only: 119 music-room pictures |
| `sound.npa` | 178 MB | 628 Ogg Vorbis: BGM (`bgm/djNN.ogg`) and sound effects |
| `voice.npa` | 217 MB | 3,516 Ogg Vorbis |
| `movie/` | 140 MB | 35 MPEG-1 program streams without extension (MP2 audio): 17 `mpg_*` (opening, battles), 18 `mv_*` (effects over the scene) |

**NPA:** Muramasa's format and key scheme (`npa.rs`) with another
256-byte table that `Django.exe` doesn't hold as plain bytes:
`npa::DJANGO_TABLE`, recovered by known plaintext from the Ogg headers
(`python3 -I research/nitroplus/npa_table.py <game>/sound.npa
<game>/voice.npa`). The CLI picks it for `Django.exe`. Tools:
`conscripter-convert npa list|extract <game>/Django.exe <archive>`.

**Saves:** Muramasa's format ("Save format" there) with this table and
the same inner magic. Folder `AppData\Roaming\Nitroplus\続・殺戮のジャンゴ
-地獄の賞金首-\ver1.00\`. The current page is `$textdecor`, the scene
file `$SYSTEM_spt_name`, its block `$単元名`; `script.npf`
(`FUN_004545d0`) keeps the text of every loaded script, 0x40 file slots,
blocks 0x400 apart. `research/tools/nitroplus_save.py` dumps,
round-trips and edits them; its `scene` option sends a save to any scene
file from a template standing on the **last page** of its scene
(cutting earlier hangs the load). A script `Save(n)` under wine writes
empty files for every slot but `9999` (the thumbnail capture fails; 9999
skips it), so `Save(9999);` after a scene's last `</PRE>` makes a
template.
