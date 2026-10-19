# 装甲悪鬼村正 (Nitroplus NSS)

Full Metal Daemon Muramasa, the second game, and the reference for the
Nitroplus NSS engine. This is the game's committed record: file
formats, the script language, how the compiler maps it to CONscripter,
the routes and progression variables, and what is left. The general
process is in `docs/new-game.md`, general lessons in `docs/lessons.md`.
Paths below are relative to the repository unless they say otherwise;
`<game>` is the folder holding `Muramasa.exe` and the `*.npa` archives.

- **Status:** in progress (see "Status" below). Branch `nitroplus`.
- **Engine:** Nitroplus's NSS engine (`Muramasa.exe`, 2009). Scripts are
  shipped as **plain source text** in an encrypted NPA archive, so no
  bytecode is reversed: the converter is a compiler from NSS.
  Semantics of the engine built-ins come from the corpus, the NSS
  helpers that wrap them, and two open reimplementations read for
  semantics only (FGRE/libnpengine, nitrosharp): see
  `research/nitroplus/builtins.md`.
- **Game files:** the retail DVD (InstallShield 10.5). Install it, or
  unpack `product/data1.hdr` from the disc with `unshield`: the game
  data lands in `ゲームデータ/` (this is `<game>`), the movies in
  `ムービー/`. The game needs no registry entries, only the files. Keep
  `<game>` on a local disk: the full build reads 3 GB of archives.
- **Reader and compiler:** `crates/conv-engines/src/nitroplus/`;
  research scripts in `research/nitroplus/`.

## Build

`conscripter-convert build <game> -o <output>` recognises the game and
runs `convert-nss` with `--file ma00_000.nss --chain 2000 --title
装甲悪鬼村正` (README, "Converting a game"); the
convert options (`--dry-run`, `--font`, …) go after `-o`.

Whole game, script only (about 10 s, 1.2 GB peak; writes `0.txt`, no
assets; use it for anything about logic):

```sh
target/release/conscripter-convert convert-nss <game> \
  --file ma00_000.nss --chain 2000 -o out/muramasa-dry --dry-run
```

Full build: the same without `--dry-run`, plus
`--font <a Japanese TTF, e.g. ipam.ttf> --title 装甲悪鬼村正` (the font
is copied into the output as `default.ttf`). It takes
about 3 minutes and 1.2 GB of memory (2026-10-14; it was 7 GB until
big zooms were cut to the screen, see "How the compiler works"), and
writes 3.5 GB. Run one at a time.

`--webp` (near-lossless) or `--webp --webp-quality 80` (lossy) stores
the pictures as WebP, for the web build: the first two files
(`--chain 2`) have 497 pictures, 66.5 MB as PNG, 13.3 MB at quality 80.
The other convert options (`--loose`, `--audio-codec`,
`--audio-quality`) are listed in the README.

- Start at `ma00_000.nss`, the new game of the boot script (the
  prologue, and the file that unlocks 魔王編). Starting anywhere else
  skips the unlock.
- `--file X.nss` without `--chain` compiles one scene.
- Movies are read from the disc's `ムービー/` folder: by default a
  `ムービー/`, `movie/` or `movies/` (any case) in `<game>` or beside it
  (as on the disc); `--movies <dir>` names another. They
  are not in the archives. The first build encodes them to `movie/`
  (loose files, H.264; a few minutes, kept by later builds into the same
  output). A full build without the folder, or without ffmpeg, fails
  unless given `--no-movies` (then it leaves them out); a dry run only
  says so.
- `NSS_TRACE=1` prints each file as it is compiled and a `flag %N name`
  line per story flag.
- Every build prints a ranked "not converted or approximated" list and
  runs `conscripter-convert lint` on the output.
- The output is deterministic: two runs give the same `0.txt`.

## What converts

| Area | Status |
|---|---|
| NPA archives, images (PNG, JPEG), BGM, sound effects, voices | ✅ Decoded (whole-corpus check not done) |
| Movies (`.ngs`, MPEG-1) | ✅ `MoviePlay` and the battle cuts the script waits for play as movies (H.264 MP4, the cuts without stopping the music); the other `CreateMovie`s (the smoke loop `mvk100`, the cuts under the text) play as sprite movies (`lsp` of an MP4; an alpha movie's opacity as the right half); a paused cut is its frame (2026-10-16, see "History") |
| Script parsing | ✅ All 913 files; 17 places that don't parse are all typos or dead code |
| Story control flow (files, scenes, functions, `if`/`while`, choices, story flags, route ranking, heroine deaths) | ✅ Compiled; all 883 story files reached from `ma00_000` |
| Text: vertical windows, face windows and name plates, ruby, `<k>` waits, voices | ✅ Converted; vertical columns where the original's are (checked under wine), turned marks (《「―) centred, English keeps its spaces |
| Backgrounds, sprites, fades, mask wipes, zoom, shake, blur, BGM, sound effects | ✅ Converted (timed actions drawn as cross-fades, see "Leftovers"); volume changes on a playing sound ❌ not yet (only a fade to 0 is), see "Next" |
| Choices | ✅ Drawn like the original's `SetChoiceBase` plates |
| Parallel processes (`CreateProcess`: battle HUD, fire flicker, …) | 🟡 Endless loops of timed actions (`FlyMoving*`, `TurboBlur*`, `RotetoLoop*`, the clouds, `ma_03_SBProcess`, …) as looping `spmotion`, also behind an `if` and after a lead-in (the cockpit meters); the one-shots next (Status) |
| Additive / multiply / subtractive blends (`Request` `AddRender`/`MulRender`/`SubRender`) | ✅ `spblend` 1/2/3 (History, "Render modes") |
| `Rotate`, camera moves | 🟡 `Rotate` about z; cameras as 続・殺戮のジャンゴ's (Status) |
| `SetFrequency` (pitch bends) | ✅ `chfreq` (along its tempo) |
| 立方陣 minigame (`function_magiccube.nss`) | ✅ Ported (see "The 立方陣" below) |
| System screens (title, save/load, settings, backlog) | ✅ Title, right-click menu, quick bar, confirmations, save/load, backlog, settings, favour screen, click-wait icon (each checked beside the original); 🟡 the title's fire flicker and openings left ("Next" item 4, step 7) |
| Extras | ✅ The menu, CG gallery, sound room, recollection, 3D viewer, the way back to the title and the title's ライナー画像 link ("History", extras 1-7) |

## Files and formats

All in `ゲームデータ/`. Every archive is NPA v1, zlib-compressed and
encrypted. After decryption every asset is a standard format.

| File | Size | Contents |
|---|---|---|
| `Muramasa.exe` | 1.2 MB | The engine; holds the NPA substitution table |
| `nss.npa` | 4.1 MB | 913 `.nss` scripts, Shift-JIS source text, CRLF (19 MB unpacked) |
| `system.npa` | 1.5 KB | `system.ini`: start script, font (ＭＳ ゴシック 18 px, black outline), volumes, icons, voice names |
| `cg.npa` | 1.3 GB | 5,469 images: 3,167 PNG, 2,293 JPEG |
| `sound.npa` | 213 MB | 760 Ogg Vorbis: 56 BGM (`bgm/mbgmNN.ogg`), 704 sound effects |
| `voice.npa` | 1.5 GB | 35,530 Ogg Vorbis (mono, 44.1 kHz), `voice/<script>/<id>` |
| `../ムービー/*.ngs` | 353 MB | 78 MPEG-1 program streams (1024×576, MP2 audio) that ffmpeg reads as is; 46 are extras (`3d/ex_NNN`) |

`cg.npa` folders by size: `sys/extra` 1,225, `fw` 869 (face windows),
`ev` 602 (+313 `ev/resize`), `bg` 381, `st` 380 (+137 `st/resize`,
standing pictures), `sys/magiccube` 147 (the minigame), `sys/config`
135, `sys/telop` 131, `data` 131 (transition masks), `anime` 131.

**Screen size 1024×576**, written as CONscripter's `;$S1024,576`
header.

**NPA** (`npa.rs`; tool `conscripter-convert npa list|extract`):
- Names: `byte[x] += 0xFC·x − (bytes of key1·key2) − (bytes of the entry
  index)`, the published Nitroplus scheme.
- Data: only the first `0x1000 + len(name)` bytes are encrypted:
  `b = inv[b] − k − i`, where `inv` is the inverse of a 256-byte
  permutation in `Muramasa.exe` at file offset `0x134830`, and
  `k` = low byte of `((0x87654321 − Σname) · len(name) + key1·key2) · size`.
- Found by scanning the exe for 256-byte permutations and using the
  zlib header `78 9c` as known plaintext; the formula holds for all 913
  scripts.

## Script language (NSS)

C-like source text, interpreted by the engine at run time. The parser
(`nss.rs`, module doc) accepts every construct below; `conscripter-convert
nss check|builtins|refs|route` inspect the corpus.

**Files** (913): story scripts `ma` 159, `mb` 115, `mc` 251, `md` 353,
`mz` 4 (`NN_NNN` = part and scene; suffixes `a`/`b`/`c` are branches,
`vs` battles); a helper library `function*.nss` (8 files), menus
`sys_*` (15), extras `extra_*` (5), the boot scripts `0_boot*.nss`, and
a debug select.

**Constructs:** `chapter`, `scene`, `function`, `call_scene`,
`call_chapter`, `#include`, `if`/`else`, `while`, `select`/`case @label`
(choices), `return`, `break`, labels. `$name` variables live with the
save, `#name` ones in the system data (CG flags, route clears). Arrays
(`$a[0]`, `$Name[word]`), `Array()`/`AssocArray()`, string `+`.

**Text** is `<PRE @box0> … </PRE>` blocks (14,796) with `<voice
name= class= src=>` tags that name the speaker and the voice file
(35,116; no voice rule to reverse), `<RUBY>`, `<k>` (a click wait
mid-line), `<FONT>`, `<BR>`, and inline code in `{ … }`.

Rules that are easy to get wrong:
- **Parameters are bound by token**, like macros: `function
  FadeM("ナット名", 描画時間, t, $x)` and the body uses `"ナット名"`,
  `描画時間` and `t` as written. Missing trailing arguments are empty;
  extra ones are ignored.
- **Strings can name variables**: `Fade("$現ナット名", …)`.
- **Function lookup is per file**: the current file first, then the
  library (the files the boot scripts `#include`); later definitions
  win. 65 names are defined in several files.
- **Block comments nest.** `function.nss` comments out old copies of
  `TypeBegin*`/`SetText*` inside `/*=====*/` banners; reading them as
  flat comments made the old copies win (stale text-box parts in 35,944
  window pictures).
- The engine tolerates typos, and so must the parser: `$a++:`, a
  missing `;` or comma, bare `@`, `$$name`, `*/=====`. Two flags are
  set and compared with the word `ture`, which works either way.

**Flow between files:** `0_boot_開始スクリプト.nss` loops
`while($GameCircle) call_scene "nss/"+$GameName+"->"+$GameName+"_MAIN"`.
Each story file has `chapter main` (when started directly), `scene
X.nss_MAIN` (skip check, CG flags, runs the content scene, then sets
`$GameName` to the next file, inside `if`/`case` for branches) and
`scene X.nss` (the content). `$GameCircle = 0` ends the game.

**Built-ins:** about 1,700 functions are defined in NSS (the story
calls the helpers: `FwC` 18,920, `SetTextEXC` 12,092, `TypeBeginCI*`,
`CreateSE`, `MusicStart`, `FadeDelete`, `PrintBG`, …). Under them,
about 120 engine built-ins: `Fade` 12,253, `Move` 5,991, `Delete`
5,063, `Zoom` 4,765, `Wait` 4,048, `Request` 3,860, `SetVolume` 2,568,
… Objects ("nuts") are named by strings; the number after the name is
the draw priority. Most timed calls take a last "wait" argument:
with `false`, several animations run at once until `Wait`/`WaitAction`.
The table of what each built-in does is `research/nitroplus/builtins.md`.

## Approach

Decided 2026-10-13: **compile NSS to NScripter, with Lua
as the fallback.** NSS is structured source, so functions become
`gosub` labels, `if`/`while`/`select` labels and jumps, `$`/`#`
variables NScripter variables, object names sprite numbers. CONscripter
builds with `USE_LUA` on every target, the web build included
(`LUAHandler.cpp`: `luasub`/`luacall` and callbacks such as
`NSCALL_animation`, per frame). Lua coroutines fit `CreateProcess` and
overlapping timed actions; use them only for what NScripter can't
express, and add to CONscripter what Lua can't drive efficiently.
Rejected: Python in the engine (Pyodide adds 10+ MB and a slow start to
the web build, and the hard part is the built-ins' semantics, not the
language).

## How the compiler works

`convert.rs` is the plain NSS compiler. What only this game needs lives
in `convert/muramasa.rs` (`MURAMASA`, a `Game`): functions it compiles
itself, scenes it replaces with a port, story flags its own code sets.
The CLI uses it when the game folder has `Muramasa.exe`; another NSS
game would add its own module.

`convert.rs` (read its module doc first) **runs the story at convert
time**: helpers are inlined with their parameters bound by token, the
engine's named objects (textures, colours, sounds, text) are modelled,
and each built-in updates that model. What the player would see becomes
IR ops at the points where the original waits:

- **Pictures:** every visible texture or colour object is a sprite
  layer ordered by draw priority. A blocking `Fade`/`Move`, a `Wait` or
  a text page shows the changes since the last such point as one
  cross-fade over the action's time. Non-blocking actions join the next
  blocking point. A zoomed picture is a scaled copy (`system/zoom/`);
  one wider or taller than 4096 px (zooms go up to 60×, dives into a
  picture) is cut to its part on the screen, so it stays loadable. `PrintBG` freezes the screen under a cover and
  rebuilds the scene; the cover's fade-out is the cross-fade.
- **Text:** the window comes from the `SetText*` helper's layout
  (`LoadText` size and pitch, `Rotate` 90° = vertical, `Move`). Face
  windows and name plates are `Hideable` textures over the box, drawn
  into a generated window picture (`system/window_<hash>.png`, a new
  `setwindow` when it changes); speaker names aren't printed. A blank
  line ends a page, `<k>` becomes NScripter's `@`, `{code}` runs before
  the page it starts (seen under wine). Vertical text is full-width.
  A half-width letter, digit or punctuation mark under `tateyoko 1`
  draws rotated and doesn't stop the engine (checked in Shift_JIS and
  UTF-8 scripts), but `/` outside ruby, `#`, `?`, `%`, `$` and `@` in
  text are read as script syntax, and a script error ends the run.
- **Sound:** `Request(snd, Play)` with the `SetVolume`/`SetLoop` calls
  after it starts BGM or a sound effect; volume 0 on a playing sound
  stops it.
- **Files:** `compile_run` keeps a worklist of (file, state). Each file
  runs its `_MAIN` scene; its end jumps to the next file, or dispatches
  on a fork variable or on a flag holding the next file's name. A file
  is compiled once, from the first state that reaches it; story flags
  are reset to "unknown" at each file start, so that's safe. A file
  that names itself as the next one replays (a retry); one that names
  none ends the run. Statements per file are capped (`MAX_STEPS`).
- **Runtime values:** variables that story files both set and test are
  story flags, compiled to NScripter variables (below). A condition on
  one becomes `Value::Runtime`: `jump_if` compiles it to a comparison,
  `runtime_if` compiles both branches and either merges them or forks
  (when the next file differs). After a fork, values the branches set
  differently (a room or file name) are handed on through `%70`, the
  leaf variable, which every branch sets to an id unique in the file.
- **Choices:** `select` blocks hold their own input loop; the compiler
  takes the `SetChoiceNN` texts and emits one NScripter `select`
  (one per variant when options sit behind a runtime `if`). Drawn like
  `SetChoiceBase`: n vertical plates (`cg/sys/select/select001.png`,
  50×440) 100 px apart, option 1 at the right, black text, red on
  hover.
- **Route ranking and deaths:** `RouteChicker`/`RouteChicker2` and
  `SatugaiSystem` (`sys_hmode.nss`) rank the heroines by affection with
  loops and computed names. They are written as NScripter comparisons
  (`RANKINGS` in `convert/muramasa.rs`), so the ranking and the death of the heroine with the
  most affection happen at run time as in the original.
- **The 立方陣** (`function_magiccube.nss`, the one library file the
  story names as a next file): compiled like a story file, except that
  its content scene becomes the ported puzzle (see "The 立方陣").

**The 立方陣.** A 5×5×5 magic cube drawn as five layers: layers 1-3 are
given, the player fills the 18 empty cells of layers 4 and 5 with 18
numbered tiles (click a tile, then a cell; click a filled cell to take
its tile back; right-click drops the picked tile), then clicks 解答する.
The NSS builds it from `CreateChoice` buttons and an input loop, which
don't compile, so it is ported: `convert/muramasa/magiccube.rs` holds the
game's numbers (tray, cell positions, the solution, the six cells the
easy mode fills) and builds an IR `Op::TilePuzzle`, which the backend
(`conv-cons/src/puzzle.rs`) writes as a `btnwait` loop over sprite
buttons. Its result goes to the story flag `$立方陣正解`, and the
minigame's own `_MAIN` scene then forks to `md05_025end` or
`md05_025gameover` at run time. Failing offers a retry (する), then the
easy mode (下げる, `$立方陣簡易フラグ`). Not ported: the debug button
(off screen, solves it) and the smooth blink of the picked tile (two
steps instead). The cells are diamonds whose rectangles overlap, so the
puzzle turns on `transbtn` (buttons hit by their pictures' alpha); a
filled cell's button is the cell's shape at opacity 8/1000. Checked in
the engine: solving, a wrong answer, reset, help, the easy mode, and
both forks after the answer.

**Variables in `0.txt`:**

| Range | Use |
|---|---|
| %40-%69 | fork variables (which branch of a choice or runtime `if`) |
| %70 | leaf variable (which way through the last fork) |
| %100-%299 | the backend and its menus |
| %300-%799 | story flags (`$`, saved with each slot), in first-use order |
| %800 up | `#` flags, `globalon` (header `value800`) |

## Routes

Walkthrough source: rodemass, "ゲーム攻略メモ～装甲悪鬼 村正～"
(http://www5d.biglobe.ne.jp/~rodemass/game_k/gk_muramasa.html, a
choice chart per route). The route map (`conscripter-convert nss route`,
`route.rs`) reaches all 883 story files from the two starts:
`ma00_000` (new game) and `ma02_001` (a title option that starts part
2). 1,124 edges; 84 files contain choices.

| Route | Heroine | Files | Ending | How it opens |
|---|---|---|---|---|
| common | — | `ma00_000` prologue, `ma01`-`ma05` | — | new game |
| 英雄編 | 綾弥一条 | `mb01`-`mb04` | `mb04_026` 一条END | affection ranking at `ma05_023` |
| 復讐編 | 大鳥香奈枝 | `mc01`-`mc04` (airship exploration `mc02_031*`) | `mc04_052` 香奈枝END | affection ranking at `ma05_023` |
| 魔王編 | 足利茶々丸 | `md01`-`md04` | `md04_045aa` 茶々丸END | both endings above, then a new game (`#魔王編解放`) |
| 悪鬼編 | 村正 | `md05`-`md06` (village exploration `md05_004*`, 立方陣 at `md05_025`) | `md06_033a` 村正END, `md06_036` TRUE END | the other option at `md04_045` (load the save made before it) |
| game over | — | `mz00_000`, `mz01_*`, `*gameover` | — | battles, failed explorations |

Order in the chart: 一条 and 香奈枝 (either order) → 茶々丸 → 村正
(load the save made before `md04_045`). The chart's last lines are two
columns flattened: 殺す → 村正END; ……殺せない → `md06_034` 選んだ道を進む
→ TRUE END. Choices of the chart, with the file that offers them (option
number in brackets):
- 一条/香奈枝 common: `ma02_003` 信用できる (1) · `ma02_006` 未熟だ (2) ·
  `ma02_013` お断りします (2) · `ma02_014` むしろ綾弥の方が (2) · `ma02_018`
  綺麗な方 (1) · `ma03_005` 是 (1) · `ma03_011` 一条の様子を見に行く (2) ·
  `ma03_011b` 間違ってはいない (2) · `ma03_021` 香奈枝 (2) or 一条 (1) ·
  `ma04_014vs` 役割に徹する (2) · `ma04_022` 娘だ (1).
- 英雄編: `mb04_004` ……待て · `mb04_004b` 救えない · `mb04_004bb` 噛み締めて
  → 一条END.
- 復讐編: battles `mc01_009vs`-`mc01_021vsb`, the airship exploration
  `mc02_031*` (about 25 room choices), `mc03_017vs`-`mc04_049vs`; two bad
  ends at `mc04_014vsa` / `mc04_017vsa` → 香奈枝END.
- 茶々丸: `ma02_006` 率直だ (1) · `ma02_013` ばあや殿なら…… (3, needs
  `#復讐編終了`) · `ma02_014` 俺には村正だろう (3, needs `#魔王編解放`) ·
  `ma02_018` 豪快な方 (2) · `ma03_005` 否 · `ma03_011` この場に留まる /
  香奈枝の様子を見に行く · `ma03_020` 頼み込む / 黙らせる · `ma03_021`
  ライガー (3) · `ma04_005` 一条を見る / ばあや殿を見る (3, needs
  `#復讐編終了`) · `md02_032` そうしよう · `md02_043` ……なれない · save ·
  `md04_045` 捨てられる → 茶々丸END. Where the chart lists two options for
  one select, take the one that keeps `$Muramasa_Flag` ahead. On a later
  run `mc02_030` offers to skip the airship exploration.
- 村正: load, `md04_045` 捨てられない · village exploration `md05_004*`
  (3 rounds) · bad ends at `md05_010vs` / `md05_012` / `md05_013` ·
  `md05_014` 助けない · the 立方陣 (solutions for Lv.4/Lv.5 in the chart) ·
  `md05_027`-`md05_030` 肯定/否定 · `md05_031` 村正 · `md06_033` (above).
- 100 % CG: follow the chart and see every ending.

Explorations: each room file stores its destination in `$Next_GameName`
and goes through a clock file (`mc02_031time`, `md05_004time`), which
advances the time and dispatches; past the limit it goes to game over.
The heroine with the most affection dies at the end of part 3
(`SatugaiSystem`), so the route a player gets depends on both the
ranking and who died.

## Progression variables

From `research/nitroplus/progress.py` over the UTF-8 scripts: 207
variables are set and read by story files; most are per scene. The ones
that gate routes (numbers from `NSS_TRACE=1`, 2026-10-13):

| Variable | Kept in | Set | Read | Meaning | In `0.txt` |
|---|---|---|---|---|---|
| `#英雄編終了` | system | `mb04_026` | `ma00_000` | 英雄編 cleared | %806 |
| `#復讐編終了` | system | `mc04_052` | `ma00_000`, `ma02_013`, `ma04_005` | 復讐編 cleared: third options | %804 |
| `#魔王編解放` | system | `ma00_000` (both above set) | `ma02_014`, `ma03_020` | 魔王編 open | %808 |
| `#飛行船探索終了` | system | `mc02_031end` | `mc02_030` | offer to skip the airship exploration | %807 |
| `#光探索終了` | system | `md05_004end` | `md05_003` | village exploration done once | %802 |
| `#立方陣終了` | system | `md05_025end` | `md05_024vs` | 立方陣 solved once | %805 |
| `#変な柱フラグ` | system | `mc04_050vs` | `mc04_050vs` | counter inside one battle | %803 |
| `$Ichizyou_Flag`, `$Kanae_Flag`, `$Muramasa_Flag`, `$Others_Flag` | save | choices in `ma02`-`ma04` | `ma03_015`, `ma05_021vs` | affection | %308, %310, %311, %313 |
| `$RC_N[…]`, `$RC_N2[…]` | save | the ranking | route dispatch | rank per heroine | %315-%322 |
| `$Ichizyou_Dead`, `$Kanae_Dead` | save | `mz01_001`, `mz01_002`, `mb01_002a` | `ma04_*` | who died in part 3 | %307, %309 |
| `$Sayo_Flag` | save | `ma02_013c`, `ma04_005c` | `mc04_022vs` | さよ scenes | %323 |
| `$md05_２週目`, `$md05_３週目` | save | `md05_004start02/03` | `md05_004*` | village round | %384, %385 |
| `$mc02_TimeCount` | save | `mc02_031*` (+30 per move) | `mc02_031time` | airship clock (≥ 60 → game over) | %374 |
| `$badEnd_Flag` | save | `ma05_021vs`, `ma05_023`, `mz01_003` | `mz01_003` | bad end after the ranking | %326 |

Story flag numbers depend on first use, so they move when the compiler
changes: take them from `NSS_TRACE=1`, not from this table.

**Route check** (`docs/new-game.md` step 11): `research/nitroplus/nstrace.py`
runs the converted `0.txt`'s control flow along a choice path and
checks each file-to-file step against the source route map (format in
its docstring; `--flags`/`--watch` print story flags where they
change). The committed paths in `research/nitroplus/paths/` follow the
chart:

```sh
conscripter-convert npa extract <game>/Muramasa.exe <game>/nss.npa -o out/muramasa-nss
conscripter-convert nss route out/muramasa-nss > out/muramasa-route.txt
python3 -I research/nitroplus/nstrace.py out/muramasa-dry/0.txt \
  research/nitroplus/paths/{1-ichijou,2-kanae,3-chachamaru,4-muramasa}.txt \
  --route out/muramasa-route.txt --quiet
```

Result (2026-10-13): 0 problems, every ending reached: 一条END,
香奈枝END (through the whole airship exploration), 茶々丸END, 村正END and
the TRUE END, with five bad ends checked on the way (four battles, and
giving up at the 立方陣). A path entry `PUZZLE 1` / `PUZZLE 0` solves or
fails the puzzle. Not done: state across relaunch and in the web build
(step 11.5).

## Testing

Manual: none in digital form (2026-10-18). The game disc holds only
the InstallShield installer (its file table lists the game, its
archives and the setup), and the Sofmap bonus disc holds drama
tracks, the theater videos and system voices. The retail manual was
printed. The English port's `Manual.pdf` (muramasa-en.md) shows the
same screens in the port's look.

- **Logic:** the dry run (see "Build") and the route check (above).
  `NSS_TRACE=1` gives the `flag %N name` lines for `nstrace.py --flags
  <file> --watch '$Muramasa_Flag,RC_N[,死亡'`, which prints those flags
  at each file where they change.
- **A few files in the engine:** `--file md05_025.nss --chain 6` without
  `--dry-run` builds only their assets (3 s, 32 MB).
- **Engine runs: the debug trace** (`docs/new-game.md`, "Checking a
  build with the debug trace"). `SDL_VIDEODRIVER=dummy` and
  `--auto-play 0` play the build headless, with no Xvfb, no xdotool and
  no held Ctrl, and `--trace t.jsonl` records every page, choice,
  missing file, script error and crash. Give each run its own
  `--save-dir` (the `#` flags live there). `--goto <label> --vars
  <file>` starts at a scene with the story flags preset (the vars file
  from `nstrace.py`'s variables, or `--dump-vars` of another run);
  `--trace-vars 300-899` records the story flags as they change, to
  compare with `nstrace.py --flags` file by file. The route runs below
  use all of this through `engrun.py`. Sprite 999 is the runs' marker
  (`lsp 999,"TRACE:..."`): pass `--trace-ignore 999`. The game has no
  UI sprites of its own yet.

  ```sh
  cd out/muramasa-cons
  SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 \
      ../../engine/build/conscripter --root . --save-dir /tmp/mura-saves \
      --trace /tmp/mura.jsonl --auto-play 0 --goto f_md06_030 --vars v.txt
  python3 -I ../../engine/tools/cons_trace_check.py /tmp/mura.jsonl --script 0.txt
  ```
- **Crashes:** the 村正END run died of heap corruption 508 files in,
  with only a core dump (malloc under `lsp`). With `--trace`, a crash
  now ends the trace with a `crash` record: the signal, the script line,
  the gosub chain and the last 50 commands. A script error exits with
  2 (it was 255) and is in the trace's `error` record, so it no longer
  looks like a silent exit.
- **Requested durations:** auto-play shortens effects, so check the
  trace's `long wait` warnings: `print 10,270000` (a 4.5-minute
  cross-fade) would have blocked a real player. A single picture that
  takes a lot of memory (the converter once almost wrote a 61440×34560
  zoom, about 8 GB) gives a `large image` warning (`--trace-max-image`,
  64 MB by default; CONscripter after 0.18.0 draws pictures in software,
  the web build too, so only their memory limits them, not their width).
- **The 立方陣:** patch a copy of `0.txt` to jump straight to the puzzle
  and print the result (`lsp 999,"RESULT:"+...` on stderr), then click
  through it: solving and the easy mode give 1, a wrong answer 0.
- **Checked forks** (all correct): `md05_004urayama01`'s runtime
  `call_scene`; `ma04_002` `$Ichizyou_Dead` (0 → an extra select in
  `ma04_005`); the `ma05_023` ranking (an all-zero tie goes to `mb01`, as
  the source says); the `mc01_010vs` choice. Routes share files that
  branch on `$PreGameName` (`mb01_003_3`); that is the source's design.

**Route runs** (playbook steps 4 and 11.4): `research/nitroplus/engrun.py`
plays each ending of the route paths in CONscripter, every page drawn.
nstrace records the options each playthrough takes; the driver patches
a copy of `0.txt` to take them by itself, skip the 立方陣 with its
recorded result, and log each story file; the engine plays it headless
with `--auto-play` and a debug trace (`<out>/<N>/trace.jsonl`, with the
story flags as `var` records). A run passes if it reaches the end and
passes the same files as nstrace; a run that dies names the trace's
`crash` or `error` record. Five runs (一条END, 香奈枝END, 茶々丸END,
村正END, TRUE END; `--all` adds the bad ends), one core each: run them
side by side.

```sh
P=$(ls research/nitroplus/paths/*.txt)
for n in 1 2 3 9 10; do
  python3 -I research/nitroplus/engrun.py out/muramasa-cons $P \
    --engine <conscripter> --only $n > out/engrun-$n.log &
done
```

Until 2026-10-14 the runs drove the engine on Xvfb with Ctrl held down
(skip mode), about 8 pages a second and 40-56 minutes a route.

To look at one place deep in a route, `--at <file>` starts the run at
that story file with every variable as nstrace had it there (a crafted
save: the variables, not the pictures or music on screen); an engine
problem there shows in seconds instead of an hour. `--wrap '<command>'`
runs the engine under another program (`valgrind -q`); an
AddressSanitizer build is just another `--engine` (configure
`engine/` into its own folder with `-fsanitize=address
-fno-omit-frame-pointer` in the C, C++ and linker flags, plus the
SDL2_mixer paths of a normal build, and set
`ASAN_OPTIONS=detect_leaks=0`).

## Running the original under wine

Used to check what the engine does (`docs/new-game.md` step 9).
- A scratch `WINEPREFIX` (user folders as real folders, not symlinks to
  home), an Xvfb display of its own (`-nolisten tcp`), `WAYLAND_DISPLAY`
  unset and `HKCU\Software\Wine\Drivers` `Graphics=x11`, so nothing
  reaches the desktop.
- **Audio:** the game refuses to start without DirectSound
  ("DirectSoundの初期化に失敗しました"), so the driver can't simply be
  disabled. Use `Drivers\Audio=alsa`, `WINEDLLOVERRIDES=
  "winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"`,
  `PULSE_SERVER=unix:/nonexistent` and `ALSA_CONFIG_PATH=<file>` whose
  only content is `pcm.!default { type null }` (+ `ctl.!default`,
  `pcm.null`). Without the system `alsa.conf` the real cards can't be
  opened, so the sound is discarded.
- No registry keys: make `C:\muramasa\` a folder of symlinks to every
  file in `<game>` and run `wine Muramasa.exe` from there.
- Fullscreen 1024×576 is drawn at (128, 125) on a 1280×800 display.
  Clicks: `xdotool mousedown 1; sleep 0.15; mouseup 1`. Screenshot:
  `import -display :N -window root -crop 1024x576+128+125`. Six clicks
  through the logos and the opening reach the title; 最初から始める at
  (780, 260) starts `ma00_000`.
- Stop with `wineserver -k` (with `WINEPREFIX` set) and kill Xvfb by PID.
- `LANG=ja_JP.UTF-8` (and `LC_ALL`): without it the save folder
  (`AppData\Roaming\Nitroplus\装甲悪鬼村正\1.00`) gets a garbled name,
  the first save fails, and 最初から始める goes straight back to the
  title. With it, clicking 最初から始める starts `ma00_000`.
- Wine has no ＭＳ 明朝; fontconfig's substitute has a taller cell, so at
  size 20 the original steps about 18 px per character instead of 20.
  Compare where columns and lines start, not the character step.
- **Patch first, click last** (`docs/new-game.md` step 9): the scripts
  are plain `.nss` in `nss.npa`, so a copy of the archive with a patched
  boot script can start any scene with its flags set and auto mode on
  (`$SYSTEM_text_auto = true`), with no clicks. `research/nitroplus/npa.py`
  decodes the archive and `--pack DIR NEW.npa` writes a copy with the
  files in `DIR` replaced (same index, keys, compression and
  encryption). Link the patched copy as `nss.npa` in `C:\muramasa\`.
  Example, straight to the title with no logos (uncomment the debug
  line `//$Logo = true;` in `0_boot.nss`):

  ```
  python3 -I research/nitroplus/npa.py <game>/Muramasa.exe <game>/nss.npa nss/
  # edit nss/0_boot.nss into patch/0_boot.nss (cp932)
  python3 -I research/nitroplus/npa.py <game>/Muramasa.exe <game>/nss.npa --pack patch/ nss-title.npa
  ```

  Or start from a save: `research/tools/nitroplus_save.py` writes one
  that goes to any scene file one click after loading ("Save format"
  below). A patched boot that loads it with no clicks: insert `Load(3);`
  after `#play_speed_plus = #SYSTEM_play_speed;` in `0_boot.nss`.
- Film transitions with `ffmpeg -f x11grab -video_size 1024x576 -i
  :N+128,125`; single shots with `import` as above.
- **Trace speed**: `SPEED=8 GAME=<game> research/nitroplus/tracedll/ntrace.sh
  ...` plays 0.45 s a page instead of 3.1, `ms` in game time (97 % of
  steps within 2 frames of a 1x run; a `WaitText` on a voiced page and
  `CreateTexture` are not scaled: `docs/new-game.md`, "Trace speeds").
- **Trace DLL** (`docs/new-game.md` step 22):
  `GAME=<game> research/nitroplus/tracedll/ntrace.sh -o orig.jsonl
  --pages 60` plays the game from the start of `ma00_000` with no
  clicks (its own prefix in `out/ntrace`, Xvfb `-displayfd`, null audio,
  ja_JP; about 8 minutes for 60 pages) and writes one `cmd` record per
  call statement (built-in or NSS function: `op`, `at` =
  `<file>:<line>`, `ms`, `th` the OS thread, 0 the story) and one
  `page` record per page of a text block (split at blank lines and
  `<k>`, tags and code left out). The `page` records come when the
  block is loaded, every page of the block at the same `ms`, before its
  `TypeBegin*` runs; the first page is shown at that function's
  `Request($textnumber, Enter)` (after its waited box `Fade`). Ours
  writes `page` at the page's click wait, after the text is typed: to
  time a page, compare the original's `Request` with our last command
  before the text. A patched copy of `nss.npa` skips the
  logos and the title (0_boot.nss lines 85, 157) and turns auto mode on
  where TypeBeginCIO unlocks it (function.nss line 1139; set at the
  boot, while locked, it stays off); no line moves. `xlaunch.exe` from
  `research/xuse/tracedll/` loads `ntrace.dll`, which redirects
  run_statement's two calls (`0x470b7c` dispatch_builtin, `0x470c9d`
  the text block's start `0x475000`); details in `ntrace.c`. The
  loaded script text turns the CR LF of a text block's header and code
  lines into LF LF, so a file line is a CR LF or two bare LFs. Times are
  ms: the script's waits are timeGetTime waits in each process's thread
  (the device's Present, hooked once, was never called through its
  vtable slot). The same tool traces 続・殺戮のジャンゴ's unpacked exe
  (docs/games/django.md); not ported: the English release (another
  engine, Squirrel).
- **Trace diff** (`docs/new-game.md` step 23): the same files built with
  `--source-map` (each `0.txt` line's `<file>:<line>`: the statement
  being run when the op was made, a page from its `<PRE>` block, a
  function's ops from the function's own line), then our run and the
  diff:

  ```sh
  target/release/conscripter-convert convert-nss <game> --file ma00_000.nss \
    --chain 3 -o out/mura3 --font ipam.ttf --title 装甲悪鬼村正 --source-map out/mura3.map
  research/tools/scratch.sh out/mura3 <scratch>
  <scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
  research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/mura3.map \
    --pages 50 --ms --fps 60 --thread 0
  ```

  Not findings: the times from a page's `Request(…, Enter)` to the box
  fading out (the reader's wait), `extra` from `Request(…, Enter)` or
  a page's own code (the original's page records are all written as
  its block starts), `procAutoIcon` (a process in the original),
  `missing` `Request`/0 ms `Move`/`Fade` that change nothing drawn.

## Save format

What `builtin_Save` (`00411ea0` → `0046eb70`) writes and
`0046fa80`/`00471b40` read back; `research/tools/nitroplus_save.py`
reads and writes it (`dump`, `write`, `selftest`), checked on saves the
original wrote in a scratch prefix and by loading a written one there
(below).

- Folder: `AppData\Roaming\Nitroplus\装甲悪鬼村正\1.00\`, one folder
  per slot (`0001` = 記録 slot 1) and `common` (`val.npf`: the `#`
  variables; `cqst.npf`: the read marks; `Save(0)` writes only these);
  thumbnails in `images\`.
- Every `.npf`: `NPSF`, u32 plain size, u32 1, u32 1, then zlib whose
  first 0x1000 bytes are `table[b + key]` (`FUN_00401650`; the table is
  `npa.py`'s, at exe offset 0x134830). The key is fixed per file name
  (`val` 100, `script` 164, `frames` 138, `bklg` 248, `cmt` 93, `cpt`
  72, `date` 8, `tm` 52, `thum` 8, `cqst` 32) and also follows from the
  zlib header. Inside, the binary files start with a tag (`NPSL`,
  `NPSG`, `NPSS`, `NPSF`, `NPSB`, `NPSI`), `c5 57 f2 f1` and u32 size.
- `cmt`, `cpt`, `date`, `tm`: plain cp932 text (the load screen's last
  line, chapter title, date, play time).
- `val.npf` (`NPSL` slot, `NPSG` common; `FUN_00462e30`,
  `FUN_00462b60`): u32 count, then one 1024-byte entry per variable: u8
  type (0 null, 1 string, 2 number; `true` is 2 with 1.0), name, 4 NULs,
  f64 number, string, 4 NULs, array key (for `$X/0`), 4 NULs. The
  current page is `$SYSTEM_present_text` (`@text0010`), the scene file
  `$GameName`.
- `script.npf` (`NPSS`, `FUN_00471580`): the program's state, one block
  per `call_chapter` level (here two: `0_boot.nss`, and
  `0_boot_開始スクリプト.nss` with the scene file). Each holds the **full
  text of every script file it has loaded** (`nss\x.nss`, 4 NULs, u32
  length, text), then one record per function, scene and chapter in
  compile order: its parameters and how far its code has been compiled
  (the game compiles a script as it runs it). Loading recompiles the
  saved texts, not `nss.npa`, and puts the records back in order.
- `frames.npf` (`NPSF`): the objects on screen as the calls that make
  them again (`CreateSound(…`); `bklg.npf` the backlog; `thum.npf` the
  thumbnail (`NPSI`, 160×90 BGRA).

So a written save can stand anywhere the saved text leads: `write`
with `"scene": "<file>.nss"` cuts the current scene's text after the
current page and its `_MAIN` after `call_scene $MainGameName`, where
it sets `$GameName` to the target; code before the current positions
stays byte-identical, so the records still fit. Checked under wine
(2026-10-17): the prologue save at `text0010` written with
`ma02_001.nss` loads on that page, and two clicks later `ma02_001`
starts (its 山脈坑道 background, soldiers, then 長坂's face window).

```
python3 -I research/tools/nitroplus_save.py dump <game>/Muramasa.exe <saves>/0001
echo '{"scene": "md05_025.nss", "vars": {"$md05_２週目": true}}' > edit.json
python3 -I research/tools/nitroplus_save.py write <game>/Muramasa.exe <saves>/0001 edit.json -o <saves>/0003
```

The template must stand on a page of a story scene (not at a choice,
whose `_SELECT` scene has already passed `call_scene $MainGameName`).

## Per-scene snapshots

A memory dump of the original at the start of every story scene, for the
snapshot oracle (`docs/new-game.md`, "Snapshot oracle"). They come from
saves, not from playing the route (2026-10-18):

```sh
python3 -I research/nitroplus/npa.py <game>/Muramasa.exe <game>/nss.npa <nss>
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> > manifest.tsv
GAME=<game> research/nitroplus/tracedll/ntrace.sh -o scenes.jsonl --patch <patch> --saves <saves> \
    --scene-dumps research/nitroplus/local/<dir> --clicks 500 --pages 1 --seconds 5400
```

`scene_saves.py` writes slot i as the template (a save the game wrote on
a story page; `common` beside it is copied too) sent to the i-th scene
file with `nitroplus_save.py write` ("scene"). It also writes patched
copies of `0_boot.nss` (`Load(1);` after the first game-speed line) and
of every scene (`Load(i+1);` as the first statement of its `_MAIN`
scene), each edit on its line. One wine process then loads save 1, the
scene starts, `--scene-dumps` dumps it at that first statement, the
`Load` goes on to the next save, and so on: about 2.5 s a scene, no
clicks needed. `research/tools/dumppack.py` packs the dumps as they come
(zstd with a keyframe as prefix: 1.35 MB a scene against 9 MB alone).

The run of 2026-10-18 dumped all 881 story scenes (the files
`m?NN_NNN*.nss` with a `_MAIN` scene) and 5 boot and system files in
41 min (886 dumps, 1.2 GB, an average of 1.35 MB a scene; base 160 MB
raw). `research/nitroplus/muramasa-scene-dumps.tsv` indexes them (scene,
slot, dump label, game ms, packed KB). `exe_call.py`'s `load_dump`
rebuilds one in about 2 s; the dumps stay in `research/nitroplus/local/`.

What a loaded save lacks against a played run: moves and fades in flight
(the save holds the objects as `frames.npf` re-creates them), sound
positions (a BGM starts again), CreateProcess threads (the text
decoration's, the quick bar's), timers and the backlog, and the route's
flags (the template's are kept; `write` can set `vars`). So the oracle's
cases start at scene starts; a case that needs state in flight starts
from a played dump (`ntrace.sh --dumps` and a probe).

### Per-frame state trace

`ntrace.sh --frames` works here as for 続・殺戮のジャンゴ (docs/games/django.md,
"Per-frame state trace"; 2026-10-19). It hooks the frame thread's call of
the object stepper (`0x44b465` → `0x44b850`, which calls each object's
slots +0x123c..+0x1258) and reads the render record (+0x1200: x/y +0xc/+0x10,
alpha +0x18, texture +0x24 with zoom +0xcc and rotation +0xc0). The probe
shows CreateTexture at 123,45, the Fade to 178, Zoom 1500/2500, Rotate 33
and the Move to 133,65 frame by frame.

### Scene-start check

The original's objects at each scene's first text block against our
engine's sprites on that page (2026-10-19):

```sh
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> --at page > manifest.tsv
research/tools/headless-run.sh env GAME=<game> research/nitroplus/scene_chain.py manifest.tsv <patch> research/nitroplus/local/<dir> \
    --stall 40 -- --patch <patch> --saves <saves> --scene-at page --clicks 300
research/tools/.venv/bin/python -I research/tools/scene_check.py research/nitroplus/scene_objects.py <game>/Muramasa.exe \
    research/nitroplus/local/<dir>/index.tsv <start dumps> research/nitroplus/muramasa-scene-dumps.tsv <scratch> --jobs 3 --out check.jsonl
```

`--at page` puts the next `Load` after every text block of a scene, so
the first one played goes on. `scene_chain.py` starts a new segment from
the next scene whenever no new scene is dumped for `--stall` seconds.
Nine scenes never reach a page from the template (`md05_004byoushitsu03*`
and others, whose text the template's flags skip). `scene_check.py`
runs `--goto f_<scene> --auto-play 1` in a scratch run of our build
(`research/tools/scratch.sh`) and matches the objects by picture.
`scene_objects.py` reads the dump: the object list, each object's
creating call at +0x180, and its render record (x, y, alpha). The render
record's texture record (+0x24) holds the rotation (degrees, +0xc0..+0xc8)
and the zoom (per mille, +0xcc/+0xd0) as drawn (2026-10-19; the Zoom and
Rotate frame callbacks 0x456f30 and 0x457240 write them, and the probe
dump read back 1500/2500 and 33). The check doesn't compare them yet.

The run of 2026-10-19 checked 865 scenes in 11 min (3 workers): **640
clean**. The differences: extra 477, alpha 112, missing 105, position
74. Not differences: occluded 73, effect 30, no-dump 9, inherited 4. The
worst are the battle scenes (`*vs*`), mid-animation at their first page.
The top findings, to triage:

1. Battle HUD (`cg/cp/*`, 17 battle scenes, about 20 sprites each): ours
   shows it from the first page; the original has no HUD object yet.
2. `md05_027a`/`b`… (8 scenes): the opening `CreateTextureSP("絵巨大光",
   …ev253_飢餓虚空の内側.jpg)` doesn't show on our first page (black).
3. `mc04_014vsa`… (7 scenes): `ev187_銘伏の妖剣_i.jpg` missing likewise.
4. Rain: ours draws `CreateEffect` Rain as `system/strip` sprites (16
   scenes, counted extra). The checker should pair them.
5. Alpha 240 against 255 on standing pictures: the original's block
   start falls mid-fade. That is the check's timing, not a difference.

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-13 13:00 | - |
| First dry run | 2026-10-13 14:21 (first scene) | - |
| Story plays | - | - |
| Title, save, settings | 2026-10-14 18:18 to 10-15 00:37 | - |
| Extras | 2026-10-15 | - |
| Progression verified | - | - |
| Finished | - | - |

## Status: in progress (2026-10-18)

The whole story compiles and every ending is reached in the converted
control flow (10 route runs passed, 2026-10-14). The system screens
(title, menu, save/load, backlog, settings, extras, gallery, replay,
sound room), movies, processes, `Random` and the effects are done; their
write-ups, the route-run table and the old queue are in "History"
("Status as it stood before the 2026-10-18 prune"). Dry-run `score`
(`metrics.json`, unread arguments by role): 99.90; picture 99.67, movie unmeasured (no
movie folder in a dry run).

Japanese and English in one build (`docs/string-tables.md`): English
code and text per language, pictures in `layer\en\`, differing event
CGs in `layer\uncensored\` (unchecked on a censored CG), Language and
Pictures rows, the English font; a language change restarts. Later, a live
switch ("islands", `docs/string-tables.md`).

Testing notes: other agents stop engines with `pkill -x conscripter`; run
ours through a symlink with another name (`--engine out/conscripter-mura`).
The route runs and `--at` starts are in `research/nitroplus/engrun.py`.

Open, by area (each is small; most noticeable first):

- **Original archives** (`--original-archives`, `docs/original-archives.md`):
  works (3.6 GB → 889 MB, page shots AE 0); not with `--english` yet.

- **Engine-drawn screens**
  - Save/load: the play time is read when the screen opens (the routine
    read it at the click); S and L no longer close the screens.
  - Backlog: the knob sits up to 1 px off after a wheel or drag; a press
    on the sweep jumps the knob; 逆 at the oldest row and 進 at the newest
    are not buttons (no hover light); `SetBacklog`'s 鈴川 in ma01_021 has no
    voice switch.
  - Extras: the four modes' screens; the gallery's viewer is still the
    script's.
  - Extras: a build without the disc's movies (`--no-movies`) has no sound
    room or 3D viewer (both read movies): their buttons are drawn faint,
    as locked ones. Build with the movies; `tests/extras_smoke.py <build>`
    fails on such a build. A songs-only sound room without the movies is
    not done.
  - Settings: a press on a knob keeps the grab offset (a drag ends 2 px
    from the routine's); EN's screen fades 300 ms (guess) and its pairs switch at once.
  - The quick bar sliding while text types (user's choice, 2026-10-15:
    left; needs the engine to move the bar).
- **Movies**: sprite movies don't play in CONscripter for Windows (opens
  an outside player): an engine item for every game, not started. The
  original under wine draws no movie (no 32-bit GStreamer), so movies
  and the smoke's strength (`SubRender`; ours draws it black, lighter)
  can't be compared. The trace checker's `movie` rule should skip movies
  that start from black.
- **Exe mapping** (`docs/games/muramasa.findings.tsv`): unknown
  `SetFrequency` on music (1,486), `CreateSurface`/`SetSurface`,
  `Position`, `CreateMovie`'s arguments, `Delete`'s 1-2; names carried
  from Django.exe by hand (`docs/new-game.md` step 21).
- **Accuracy guesses** a frame of the original could settle: the
  click-wait flame's frame times; a choice's text 40 px down its plate
  and red under the pointer; the title glow (`SetBlur(…, 2, …)` is a
  trail of copies, ours a 3 px blur); looped and paper `Shake`s turning
  every 100 ms (exe: 1000 ÷ (n × 0.06) ms). Nested cameras: the exe draws
  only the innermost, ours multiplies (not checked whether a script
  nests). Camera zooms other than 1000 are unverified. Sepia and
  KitanoBlue colours of `CreateEffect` are ours; the engine's blur keeps
  the alpha (the exe's too, unverified). Making every timed effect 4 %
  shorter (16/16.67) is possible, the lead's call.
- **Processes**: `ma03_024`/`025`/
  `027vs` `_runsProcess` (23, wait on the script) noted; endless
  processes with `Random` (`BGMoveLengthProcess`, `ma03_moveAuto*`,
  `RandomMeimetsuSet*`, `GunFlash`, shakes) stay noted; 108 process notes
  (`TurboBlur3` 10, `DenziBladeChargeSet` 10, about 45 `*_runsProcess`);
  `Request(<process>, Stop)` (1,126 notes) is dropped, unchecked whether a
  looped motion keeps running; `MFlashProcess`'s pause is picked at
  convert time, not run time; `ComicProcess`: 16 of 18 signs are drawn
  at their end state, zoom about the centre not `SetVertex`'s point.
- **`Random`** (771 calls, "Random picked when converting"): picks that
  reach text (SetKenki's lines in md05_008vs/009), leave a picture at a
  random place (md04_043, `CP_AziChange`), or give a loop to a picture
  shown only later.
- **Effects**: `CreateEffect` Drop (1, mc04_052) and Lens (2); `DrawEffect`
  waves on a `Plain` cover that is shaken or whose screen changed (3),
  Ripple (4), Split (7), a wave picked up
  later restarts straight; a moved/zoomed/blended `Plain` cover (History, "Plain covers as
  screen copies") first shown over a screen that changed since it froze,
  or shaken, keeps the screen; one placed again on another layer is copied
  again from the screen then; `OverlayRender` draws normally; 565 calls start from
  another opacity (noted); `CreateStencil`/`CreateMask`: a moved or zoomed picture
  under them, a zoom about a vertex, a `Plain` cover under a stencil, a
  colour mask exact only for alpha blend; 25 `CreateTexture`s from an
  object or the screen; `Rotate` of other objects about z (31), a turn
  over time about x or y mirrored at once (10); 38 moves/zooms with a
  fade or new picture drawn as cross-fades; a window that moves (`絵演窓上`,
  `窓右`, about 56 `Move`s) has its clip jump to the end place.
- **Time ledger** (`NSS_TRACE=1`: each hold's place): 1,243 syncs hold up
  to 1 s (not under a cover or for new pictures' motions: History); ~1,200
  zooms, paths, moves, fades drawn as cross-fades (`spmotion` keys: own item).
  9 motions end at once (mc01_009vs:439, mb04_025vs:2987, md05_007vs).
- **Tempo/arguments**: easing of `SetVolume` (9 fades over time keep a
  straight ramp) and the text window's `windowfade`; a move with a fade
  folded into its keys is straight; story `CreateText` rotated sideways
  sits about 17 px lower than the original's (cause unknown);
  `MoveCamera`'s arguments in a process (4); `LockVideo`, `WaitText`,
  `SetFrequency` (but 1000), `WaitAction`'s time limit, other waits:
  dropped, meaning to check in the exe; `Conquest` returns 0.
- **Text and small**: `<VALUE name=$mc02_時間>` (airship clock in
  `mc02_031time` prints a gap), ASCII-art pages (`PushText` + `NoLog`
  shown at once, not in the backlog), `<voice mode="off">` (3);
  `Request(…, NoIcon)` (4) dropped without a note; `system.ini`'s
  `icon_auto` (15 frames) and kinsoku (`bad_end`) unused; a 4-argument
  `Fade` probably doesn't wait in the exe (49 calls wait in ours; check
  under wine first); `<BR>` (16 uses) isn't in the exe's tag list;
  25 sounds and 2 transition masks missing from the archives, 2
  statements that don't parse, 2 voices outside a text block, 3 BGMs
  without loop played looping.
- **Optional**: 18 files whose `if`/`else if` chain has no `else`.
- **Engine**: CONscripter segfaults on `end` after a `dwave` voice with
  Ctrl held (seen once; not looked into).
- **Trace diff, 300 pages** (2026-10-18; the original at `SPEED=8`
  against our full build, `trace_diff.py --pages 300 --ms --thread 0`;
  with CONscripter #150's `ev` for `wait`/`delay`: missing 1741, time 73,
  extra 57). Most "missing" are artefacts: the text box's own calls
  (`Request` Lock/Hideable/Enter/UnLock/Disused, the text's and `@box*`'s
  `Fade`/`Move`, `@Face_Sepia`: ~700), calls folded into one of ours
  (`OnSE`'s `SetVolume` + `Request(Play)`, `CreateBGM`'s `Lock`), staged
  changes credited to the statement that flushes them (`Gin_song*`,
  `絵背景*` before a `Wait`/`WaitKey`), calls on no object (`Comic*`);
  the 30 to 300 ms steps where the original is slower are its frames and
  loads under wine. (A `Fade` that leaves the opacity as it was, or
  on no object: History, "Fades that change nothing", "Actions on no
  object".)
- **Scene-start check** (2026-10-19; "Per-scene snapshots"): 640 of 865
  scenes clean; its five top findings are listed there, to triage. Then
  run the scenes in the interpreter under Unicorn from the dumps.

## History

Done items: [docs/history/muramasa.md](../history/muramasa.md).
