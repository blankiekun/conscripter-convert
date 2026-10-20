# 装甲悪鬼村正 (Nitroplus NSS)

Full Metal Daemon Muramasa (Nitroplus, 2009), the reference for the
Nitroplus NSS engine (`docs/engines/nitroplus-nss.md`). Scripts ship as
**plain source text** in an encrypted NPA archive, so the converter is a
compiler from NSS: no bytecode to reverse. The English release is a
port to another engine, read into the same build
(`docs/games/muramasa-en.md`).

- **Game files:** the retail DVD (InstallShield 10.5). Install it, or
  unpack `product/data1.hdr` from the disc with `unshield`: the game
  data lands in `ゲームデータ/` (this is `<game>`), the movies in
  `ムービー/`. No registry entries needed. Keep `<game>` on a local
  disk: the full build reads 3 GB of archives.
- **Code:** `crates/conv-engines/src/nitroplus/` (reader and compiler),
  this game's rules in `nitroplus/convert/muramasa.rs` and
  `convert/muramasa/`; research scripts in `research/nitroplus/`; the
  built-ins' semantics in `research/nitroplus/builtins.md`.

## Scope

The user owns the retail PC DVD (version 1.00) and the English release
(JAST USA 2021). One build holds both languages (`docs/string-tables.md`):
English code and text per language, pictures in `layer\en\`, differing
event CGs in `layer\uncensored\`, Language and Pictures rows in the
settings; a language change restarts. Converted: the whole story, the
system screens, the extras, the 立方陣 minigame, the movies. No manual
in digital form (the retail one was printed); the English release's
`Manual.pdf` shows the same screens in the port's look.

## Build

`conscripter-convert build <game> -o <output>` recognises the game and
runs `convert-nss` with `--file ma00_000.nss --chain 2000 --title
装甲悪鬼村正` (README, "Converting a game"); the convert options go after
`-o`. Script only (about 10 s, 1.2 GB peak; use it for anything about
logic):

```sh
target/release/conscripter-convert convert-nss <game> \
  --file ma00_000.nss --chain 2000 -o out/muramasa-dry --dry-run
```

Full build: the same without `--dry-run`, plus `--font <a Japanese TTF,
e.g. ipam.ttf> --title 装甲悪鬼村正` (about 3 minutes, 1.2 GB of memory,
3.5 GB written; one at a time). The English: `build <english game>
--japanese <game>` (`docs/games/muramasa-en.md`).

- `--webp` (near-lossless) or `--webp --webp-quality 80` stores pictures
  as WebP for the web build (`--chain 2`: 66.5 MB PNG, 13.3 MB at 80).
- Start at `ma00_000.nss` (the prologue, which also unlocks 魔王編).
  `--file X.nss` without `--chain` compiles one scene.
- Movies are read from the disc's `ムービー/` (a `ムービー/`, `movie/` or
  `movies/` in `<game>` or beside it; `--movies <dir>` names another)
  and encoded to `movie/` (H.264) once. Without them a full build fails
  unless given `--no-movies`.
- `NSS_TRACE=1` prints each file as it compiles and a `flag %N name`
  line per story flag. Every build prints a ranked "not converted or
  approximated" list and lints the output. The output is deterministic.

## Executables

| File | Version | sha256 |
|---|---|---|
| `Muramasa.exe` (1.2 MB) | 1.00 | `1d67e4771a7229224daf8b8508e74313fadd950f70ea17f6ce18d089882722b5` |

Addresses in this document, `research/nitroplus/names.tsv` and
`docs/games/muramasa.findings.tsv` are this exe's. Decompile:
`research/tools/ghidra/ghidra_export.sh` with `names.tsv`.

## Files and formats

All in `ゲームデータ/`: NPA v1 archives, zlib-compressed and encrypted;
after decryption every asset is a standard format. **Screen 1024×576**.

| File | Size | Contents |
|---|---|---|
| `Muramasa.exe` | 1.2 MB | The engine; holds the NPA substitution table |
| `nss.npa` | 4.1 MB | 913 `.nss` scripts, Shift-JIS, CRLF (19 MB unpacked) |
| `system.npa` | 1.5 KB | `system.ini`: start script, font (ＭＳ ゴシック 18 px, black outline), volumes, icons, voice names |
| `cg.npa` | 1.3 GB | 5,469 images (3,167 PNG, 2,293 JPEG); folders `sys/extra`, `fw` (face windows), `ev`, `bg`, `st`, `sys/magiccube`, `sys/config`, `sys/telop`, `data` (transition masks), `anime` |
| `sound.npa` | 213 MB | 760 Ogg Vorbis: 56 BGM (`bgm/mbgmNN.ogg`), 704 sound effects |
| `voice.npa` | 1.5 GB | 35,530 Ogg Vorbis, `voice/<script>/<id>` |
| `../ムービー/*.ngs` | 353 MB | 78 MPEG-1 program streams that ffmpeg reads as is; 46 are extras (`3d/ex_NNN`) |

**NPA** (`npa.rs`; `conscripter-convert npa list|extract`):
- Names: `byte[x] += 0xFC·x − (bytes of key1·key2) − (bytes of the entry
  index)`, the published Nitroplus scheme.
- Data: only the first `0x1000 + len(name)` bytes are encrypted:
  `b = inv[b] − k − i`, `inv` the inverse of a 256-byte permutation at
  exe file offset `0x134830`, `k` = low byte of
  `((0x87654321 − Σname) · len(name) + key1·key2) · size`.
- `research/nitroplus/npa.py` decodes it and `--pack DIR NEW.npa` writes
  a copy with the files in `DIR` replaced (for patched scripts).

## Script language (NSS)

C-like source text, interpreted at run time. The parser (`nss.rs`,
module doc) accepts all of it; `conscripter-convert nss
check|builtins|refs|route` inspect the corpus.

**Files** (913): story scripts `ma` 159, `mb` 115, `mc` 251, `md` 353,
`mz` 4 (`NN_NNN` = part and scene; suffixes `a`/`b`/`c` branches, `vs`
battles); the library `function*.nss` (8), menus `sys_*` (15), extras
`extra_*` (5), the boot scripts `0_boot*.nss`, a debug select.

**Constructs:** `chapter`, `scene`, `function`, `call_scene`,
`call_chapter`, `#include`, `if`/`else`, `while`, `select`/`case @label`
(choices), `return`, `break`, labels. `$name` variables live with the
save, `#name` ones in the system data. Arrays (`$a[0]`, `$Name[word]`),
`Array()`/`AssocArray()`, string `+`. **Text** is `<PRE @box0> … </PRE>`
blocks with `<voice name= class= src=>` (speaker and file), `<RUBY>`,
`<k>` (a click wait mid-line), `<FONT>`, `<BR>`, inline code `{ … }`.

Easy to get wrong:
- **Parameters are bound by token**, like macros: `function
  FadeM("ナット名", 描画時間, t, $x)` uses `"ナット名"` and `描画時間` as
  written. Missing trailing arguments are empty; extra ones ignored.
- **Strings can name variables**: `Fade("$現ナット名", …)`.
- **Function lookup is per file**: the current file, then the library;
  later definitions win (65 names are defined in several files).
- **Block comments nest** (`function.nss` comments out old copies of
  `TypeBegin*`/`SetText*` inside `/*=====*/` banners).
- Typos the engine tolerates: `$a++:`, a missing `;` or comma, bare `@`,
  `$$name`, `*/=====`, `ture`.

**Flow between files:** `0_boot_開始スクリプト.nss` loops
`while($GameCircle) call_scene "nss/"+$GameName+"->"+$GameName+"_MAIN"`.
Each story file has `chapter main`, `scene X.nss_MAIN` (skip check, CG
flags, runs the content scene, sets `$GameName` to the next file) and
`scene X.nss` (the content). `$GameCircle = 0` ends the game.

**Built-ins:** about 1,700 functions are defined in NSS (`FwC`,
`SetTextEXC`, `TypeBeginCI*`, `CreateSE`, `MusicStart`, `PrintBG`, …)
over about 120 engine built-ins (`Fade`, `Move`, `Delete`, `Zoom`,
`Wait`, `Request`, `SetVolume`, …). Objects ("nuts") are named by
strings; the number after the name is the draw priority. Most timed
calls take a last "wait" argument: with `false`, animations run at once
until `Wait`/`WaitAction`. Each built-in: `research/nitroplus/builtins.md`.

## Approach

Compile NSS to NScripter (functions as `gosub` labels, variables as
NScripter variables, object names as sprite numbers), with Lua
(`luasub`/`luacall`, built on every target) only where NScripter can't
express it.

## How the compiler works

`convert.rs` (read its module doc first) is the plain NSS compiler; what
only this game needs is in `convert/muramasa.rs` (`MURAMASA`, a `Game`,
chosen when the folder has `Muramasa.exe`). It **runs the story at
convert time**: helpers are inlined with parameters bound by token, the
engine's objects are modelled, and what the player sees becomes IR ops
at the points where the original waits (a blocking action, a `Wait`, a
text page: the changes since the last one as one cross-fade). Zooms are
scaled copies (`system/zoom/`), cut to the screen when large. Text
windows come from the `SetText*` helper's layout, face windows and name
plates drawn into a generated window picture. Each file is compiled once
from the first state that reaches it; variables both set and tested by
story files are story flags, and a condition on one compiles to a
run-time comparison or a fork. `Random` is picked at run time (`rnd`)
where the result stays in the script; 771 calls are still picked when
converting. The heroine ranking and death (`RouteChicker*`,
`SatugaiSystem`) are NScripter comparisons (`RANKINGS`).

**The 立方陣** (`function_magiccube.nss`): a 5×5×5 magic cube; the player
fills 18 cells of layers 4-5 with numbered tiles. Ported
(`convert/muramasa/magiccube.rs` → `Op::TilePuzzle` →
`conv-cons/src/puzzle.rs`, a `btnwait` loop over `transbtn` sprite
buttons); its result goes to `$立方陣正解` and the `_MAIN` scene forks to
`md05_025end` or `md05_025gameover`. Failing offers a retry, then the
easy mode (`$立方陣簡易フラグ`).

**Variables in `0.txt`:**

| Range | Use |
|---|---|
| %40-%69 | fork variables (which branch of a choice or runtime `if`) |
| %70 | leaf variable (which way through the last fork) |
| %100-%299 | the backend and its menus |
| %300-%799 | story flags (`$`, saved with each slot), in first-use order |
| %800 up | `#` flags, `globalon` (header `value800`); past %969 at %5200 |

## System screens

Title, right-click menu, quick bar, confirmations (`cg/sys/dialog`
parts), save/load, backlog, settings, favour screen and the click-wait
icon, each filmed in the original under wine. Built in
`convert/muramasa/screens.rs` and drawn by engine screens where one
exists (`SaveGrid`, `conv-cons/src/save_grid_screen.rs`). Seen in the
original: right-click at a text wait opens the menu, wheel-up the
backlog, right-click closes every screen, Esc asks ゲームを終了しますか.

## Extras

特別付加目録 (`extra_*.nss`; values in `convert/muramasa/extras.rs`, menu
`conv-cons/src/picture_extras.rs`): 画像映写機 (CG gallery, 805 CGs, each
a `#<cg name>` flag), 回想映写機 (ten H scenes), 電磁蓄音機/動画映写機
(BGM, songs, movies), 劒冑回転目録 (46 armour movies), the ライナー画像
button. 画像, 回想, 音楽 unlock with `#ClearG` (any route's end roll). The
sound room and 3D viewer need the movies; `tests/extras_smoke.py
<build>` checks a build.

## Story structure and walkthrough

Walkthrough: rodemass, "ゲーム攻略メモ～装甲悪鬼 村正～"
(http://www5d.biglobe.ne.jp/~rodemass/game_k/gk_muramasa.html). The
route map (`conscripter-convert nss route`, `route.rs`) reaches all 883
story files from `ma00_000` (new game) and `ma02_001` (a title option
that starts part 2); 84 files contain choices.

| Route | Heroine | Files | Ending | How it opens |
|---|---|---|---|---|
| common | — | `ma00_000` prologue, `ma01`-`ma05` | — | new game |
| 英雄編 | 綾弥一条 | `mb01`-`mb04` | `mb04_026` 一条END | affection ranking at `ma05_023` |
| 復讐編 | 大鳥香奈枝 | `mc01`-`mc04` (airship exploration `mc02_031*`) | `mc04_052` 香奈枝END | affection ranking at `ma05_023` |
| 魔王編 | 足利茶々丸 | `md01`-`md04` | `md04_045aa` 茶々丸END | both endings above, then a new game (`#魔王編解放`) |
| 悪鬼編 | 村正 | `md05`-`md06` (village exploration `md05_004*`, 立方陣 at `md05_025`) | `md06_033a` 村正END, `md06_036` TRUE END | the other option at `md04_045` |
| game over | — | `mz00_000`, `mz01_*`, `*gameover` | — | battles, failed explorations |

Order: 一条 and 香奈枝 (either order) → 茶々丸 → 村正 (load the save made
before `md04_045`). Every choice of the chart, with its file and option
number, is in the committed paths `research/nitroplus/paths/*.txt`;
the 茶々丸 options need `#復讐編終了` / `#魔王編解放`, and where the chart
lists two, take the one that keeps `$Muramasa_Flag` ahead.

Explorations: each room stores its destination in `$Next_GameName` and
goes through a clock file (`mc02_031time`, `md05_004time`) that advances
the time; past the limit, game over. The heroine with the most affection
dies at the end of part 3 (`SatugaiSystem`).

## Progression variables

`research/nitroplus/progress.py` lists the 207 variables story files set
and read. Those that gate routes (`%N` from `NSS_TRACE=1`; they move
when the compiler changes):

| Variable | Kept in | Set | Read | Meaning |
|---|---|---|---|---|
| `#英雄編終了` | system | `mb04_026` | `ma00_000` | 英雄編 cleared |
| `#復讐編終了` | system | `mc04_052` | `ma00_000`, `ma02_013`, `ma04_005` | 復讐編 cleared: third options |
| `#魔王編解放` | system | `ma00_000` (both above set) | `ma02_014`, `ma03_020` | 魔王編 open |
| `#飛行船探索終了` | system | `mc02_031end` | `mc02_030` | offer to skip the airship exploration |
| `#光探索終了`, `#立方陣終了` | system | `md05_004end`, `md05_025end` | `md05_003`, `md05_024vs` | done once |
| `$Ichizyou_Flag`, `$Kanae_Flag`, `$Muramasa_Flag`, `$Others_Flag` | save | choices in `ma02`-`ma04` | `ma03_015`, `ma05_021vs` | affection |
| `$RC_N[…]`, `$RC_N2[…]` | save | the ranking | route dispatch | rank per heroine |
| `$Ichizyou_Dead`, `$Kanae_Dead` | save | `mz01_001`, `mz01_002`, `mb01_002a` | `ma04_*` | who died in part 3 |
| `$md05_２週目`, `$md05_３週目` | save | `md05_004start02/03` | `md05_004*` | village round |
| `$mc02_TimeCount` | save | `mc02_031*` (+30 a move) | `mc02_031time` | airship clock (≥ 60: game over) |
| `$badEnd_Flag` | save | `ma05_021vs`, `ma05_023`, `mz01_003` | `mz01_003` | bad end after the ranking |

## Testing

- **Logic:** the dry run. **Route check** (`docs/new-game-reference.md`, "Route simulation"):
  `nstrace.py` runs `0.txt`'s control flow along each committed path and
  checks every file-to-file step against the route map; every ending
  and five bad ends are reached. `PUZZLE 1`/`PUZZLE 0` in a path solves
  or fails the 立方陣; `--flags <file> --watch '$Muramasa_Flag,RC_N['`
  prints flags where they change.

  ```sh
  conscripter-convert npa extract <game>/Muramasa.exe <game>/nss.npa -o out/muramasa-nss
  conscripter-convert nss route out/muramasa-nss > out/muramasa-route.txt
  python3 -I research/nitroplus/nstrace.py out/muramasa-dry/0.txt \
    research/nitroplus/paths/{1-ichijou,2-kanae,3-chachamaru,4-muramasa}.txt \
    --route out/muramasa-route.txt --quiet
  ```
- **A few files in the engine:** `--file md05_025.nss --chain 6` without
  `--dry-run` builds only their assets (seconds).
- **Engine runs** (`docs/tools.md`, the debug trace): headless with
  `SDL_VIDEODRIVER=dummy`, `--auto-play 0`, `--trace t.jsonl`, its own
  `--save-dir`; `--goto <label> --vars <file>` starts at a scene with
  the story flags preset; `--trace-ignore 999` (the runs' marker
  sprite). Check with `engine/tools/cons_trace_check.py`, including its
  `long wait` and `large image` warnings. Other agents
  `pkill -x conscripter`: run long ones through a renamed copy.
- **Route runs:** `research/nitroplus/engrun.py` plays each ending in
  CONscripter, every page drawn, taking nstrace's options, and passes if
  it reaches the end through the same files (`--all` adds the bad ends;
  `--at <file>` starts at a story file with nstrace's variables;
  `--wrap 'valgrind -q'`, or an ASan build as `--engine`):

  ```sh
  P=$(ls research/nitroplus/paths/*.txt)
  for n in 1 2 3 9 10; do
    python3 -I research/nitroplus/engrun.py out/muramasa-cons $P \
      --engine <conscripter> --only $n > out/engrun-$n.log &
  done
  ```

### Running the original under wine

The last resort (CLAUDE.md, Rules). Also the recipe for 続・殺戮のジャンゴ.
- A scratch `WINEPREFIX` (real user folders), an Xvfb of its own
  (`-displayfd`, `-nolisten tcp`), `WAYLAND_DISPLAY` unset,
  `HKCU\Software\Wine\Drivers` `Graphics=x11`; `LANG`/`LC_ALL=ja_JP.UTF-8`
  (without it the save folder's name is garbled and a new game goes back
  to the title).
- **Audio:** the game needs DirectSound. Use `Drivers\Audio=alsa`,
  `WINEDLLOVERRIDES="winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"`,
  `PULSE_SERVER=unix:/nonexistent` and `ALSA_CONFIG_PATH=<file>` holding
  only `pcm.!default { type null }` (+ `ctl.!default`, `pcm.null`).
- `C:\muramasa\` a folder of symlinks to `<game>`'s files; `wine
  Muramasa.exe` from there. Fullscreen 1024×576 is drawn at (128, 125)
  on 1280×800 (`import -display :N -window root -crop 1024x576+128+125`;
  film with `ffmpeg -f x11grab -video_size 1024x576 -i :N+128,125`).
  Stop with `wineserver -k`, kill Xvfb by PID.
- Wine has no ＭＳ 明朝: its substitute steps about 18 px a character at
  size 20, not 20. Compare where columns start, not the step.
- **Patch first, click last:** unpack `nss.npa` with `npa.py`, edit a
  script (cp932), `--pack` it into a copy linked as `nss.npa`. For the
  title with no logos, uncomment `//$Logo = true;` in `0_boot.nss`. Or
  write a save that goes to any scene ("Save format"); `Load(3);` after
  `#play_speed_plus = #SYSTEM_play_speed;` in `0_boot.nss` loads it.
- **Trace DLL** (`docs/new-game-reference.md`, "Trace DLL"): `GAME=<game>
  research/nitroplus/tracedll/ntrace.sh -o orig.jsonl --pages 60` plays
  from `ma00_000` with no clicks and writes a `cmd` record per call
  statement (`op`, `at` = `<file>:<line>`, `ms`, `th`, 0 the story) and
  a `page` record per page (written when the block loads; the page shows
  at its `Request($textnumber, Enter)`). `SPEED=8` plays 8× faster in
  game time. `--frames` gives a per-frame state trace (as in
  `docs/games/django.md`). Hook details in `ntrace.c`.
- **Reference-trace patches:** for `reftrace.sh muramasa`, `mura-patch/`
  turns the first H-mode screen's `select{` (`sys_hmode.nss`) into
  `if(0){` and the 立方陣's (`function_magiccube.nss` line 261) into
  `$立方陣正解=true;$Loop_end=false;$Answer=true;if(0){`, each on its
  own line.
- **Trace diff** (`docs/new-game-reference.md`, "Trace diff"): build with
  `--source-map`, run ours, diff:

  ```sh
  target/release/conscripter-convert convert-nss <game> --file ma00_000.nss \
    --chain 3 -o out/mura3 --font ipam.ttf --title 装甲悪鬼村正 --source-map out/mura3.map
  research/tools/scratch.sh out/mura3 <scratch>
  <scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
  research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/mura3.map \
    --pages 50 --ms --fps 60 --thread 0
  ```

  Not findings: the reader's wait after `Request(…, Enter)`; `extra`
  from a page's own code; `procAutoIcon`; `missing` calls that change
  nothing drawn; the text box's own calls; calls folded into one of ours.

## Save format

What `builtin_Save` (`00411ea0` → `0046eb70`) writes and
`0046fa80`/`00471b40` read back; `research/tools/nitroplus_save.py`
(`dump`, `write`, `selftest`) reads and writes it, checked under wine.

- Folder `AppData\Roaming\Nitroplus\装甲悪鬼村正\1.00\`: one per slot
  (`0001` = slot 1) and `common` (`val.npf` the `#` variables, `cqst.npf`
  the read marks); thumbnails in `images\`.
- Every `.npf`: `NPSF`, u32 plain size, u32 1, u32 1, then zlib whose
  first 0x1000 bytes are `table[b + key]` (`FUN_00401650`, `npa.py`'s
  table), the key fixed per file name (`val` 100, `script` 164, `frames`
  138, `bklg` 248, `cmt` 93, `cpt` 72, `date` 8, `tm` 52, `thum` 8,
  `cqst` 32). Binary files start with a tag (`NPSL`, `NPSG`, `NPSS`,
  `NPSF`, `NPSB`, `NPSI`), `c5 57 f2 f1`, u32 size. `cmt`, `cpt`,
  `date`, `tm` are cp932 text.
- `val.npf`: u32 count, then a 1024-byte entry per variable: u8 type (0
  null, 1 string, 2 number), name, 4 NULs, f64, string, 4 NULs, array
  key, 4 NULs. The page is `$SYSTEM_present_text`, the file `$GameName`.
- `script.npf` (`NPSS`, `FUN_00471580`): per `call_chapter` level, the
  **full text of every script loaded**, then a record per function,
  scene and chapter: how far it was compiled. Loading recompiles the
  saved texts, not `nss.npa`. `frames.npf`: the objects as the calls
  that recreate them; `bklg.npf` the backlog; `thum.npf` 160×90 BGRA.

So `write` with `"scene": "<file>.nss"` cuts the current scene's text
after the current page and its `_MAIN` after `call_scene
$MainGameName`, setting `$GameName` to the target. The template must
stand on a story page, not at a choice.

```
python3 -I research/tools/nitroplus_save.py dump <game>/Muramasa.exe <saves>/0001
echo '{"scene": "md05_025.nss", "vars": {"$md05_２週目": true}}' > edit.json
python3 -I research/tools/nitroplus_save.py write <game>/Muramasa.exe <saves>/0001 edit.json -o <saves>/0003
```

## Per-scene snapshots

A memory dump of the original at the start of every story scene, from
saves (`docs/new-game-reference.md`, "Snapshot oracle"):

```sh
python3 -I research/nitroplus/npa.py <game>/Muramasa.exe <game>/nss.npa <nss>
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> > manifest.tsv
GAME=<game> research/nitroplus/tracedll/ntrace.sh -o scenes.jsonl --patch <patch> --saves <saves> \
    --scene-dumps research/nitroplus/local/<dir> --clicks 500 --pages 1 --seconds 5400
```

`scene_saves.py` writes slot i sent to the i-th scene and patches each
scene's `_MAIN` to `Load(i+1)` the next, so one wine process dumps every
scene (about 2.5 s each; 881 story scenes in 41 min).
`research/tools/dumppack.py` packs them; `muramasa-scene-dumps.tsv`
indexes them; `exe_call.py`'s `load_dump` rebuilds one. A loaded save
lacks moves in flight, sound positions, `CreateProcess` threads, timers,
the backlog, and the route's flags beyond the template's.

### Scene-start check

The original's objects at each scene's first text block against our
sprites on that page:

```sh
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> --at page > manifest.tsv
research/tools/headless-run.sh env GAME=<game> research/nitroplus/scene_chain.py manifest.tsv <patch> research/nitroplus/local/<dir> \
    --stall 40 -- --patch <patch> --saves <saves> --scene-at page --clicks 300
research/tools/.venv/bin/python -I research/tools/scene_check.py research/nitroplus/scene_objects.py <game>/Muramasa.exe \
    research/nitroplus/local/<dir>/index.tsv <start dumps> research/nitroplus/muramasa-scene-dumps.tsv <scratch> --jobs 3 --out check.jsonl
```

`scene_objects.py` reads a dump's objects, creating calls and render
records (x, y, alpha; zoom and rotation not compared yet). 640 of 865
scenes are clean; the real findings are in Status.

## Milestones

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-13 13:00 | - |
| First dry run | 2026-10-13 14:21 (first scene) | - |
| Story plays | - | - |
| Title, save, settings | 2026-10-14 18:18 to 10-15 00:37 | - |
| Extras | 2026-10-15 | - |
| Progression verified | - | - |
| Finished | - | - |

## Status

Active (Japanese and English in one build). Done: `CLAUDE.md`, "Goal".

| Area | State |
|---|---|
| Archives, pictures, sounds, voices, movies, script parsing | ✅ |
| Whole story (883 files, every ending; route check and 10 route runs) | ✅ in the engine, headless |
| Text, choices, pictures, motion, processes, blends, sounds | ✅ (approximations parked) |
| System screens, extras, 立方陣 | ✅ each checked beside the original |
| English text and pictures, censored/uncensored layer | ✅ (`docs/games/muramasa-en.md`) |
| Web build | 🟡 not played through |
| Dry-run `score` (`metrics.json`) | 99.90 (picture 99.67) |

Known bugs:

- **The trace diff's missing pages** (`ma03_015` and others; lead's
  lane R): pages the original shows that our run lacks.
- **Scene-start findings:** the battle HUD (`cg/cp/*`) shows from the
  first page in 17 battle scenes, where the original has none yet;
  `md05_027a`/`b`… (8 scenes) miss their opening `ev253_飢餓虚空の内側.jpg`
  (black); `mc04_014vsa`… (7 scenes) miss `ev187_銘伏の妖剣_i.jpg`.
- **`--original-archives`** (`docs/original-archives.md`) works for the
  Japanese build, not with `--english`.
- **Engine:** CONscripter segfaulted once on `end` after a `dwave`
  voice with Ctrl held (not looked into).
- **Layer:** `layer\uncensored\` unchecked on a censored CG.

### Next

1. Play the web build through, Japanese then English; fix what blocks
   or shows wrong.
2. The known bugs above.
3. Release.
