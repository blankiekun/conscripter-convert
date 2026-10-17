# 装甲悪鬼村正 (Nitroplus NSS)

Full Metal Daemon Muramasa, the second game, and the reference for the
Nitroplus NSS engine. This is the game's committed record: file
formats, the script language, how the compiler maps it to CONscripter,
the routes and progression variables, and what is left. The general
process is in `docs/new-game.md`, general lessons in `PLAN.md` §13.
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
  output); without the folder the build leaves them out and says so.
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
| `SetFrequency` (pitch bends) | ✅ `chfreq` (tempo curve drawn straight) |
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

  Saves (`.npf`) are the fallback: `builtin_Save` (`00411ea0`) and
  `builtin_Load` (`00411fb0`) would give their format.
- Film transitions with `ffmpeg -f x11grab -video_size 1024x576 -i
  :N+128,125`; single shots with `import` as above.

## Status: in progress (2026-10-14)

### Queue (2026-10-16)

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
- Done: `SetBlur`'s trail behind a moving picture as CONscripter's
  `sptrail` (engine #25, converter #82; "History", "`SetBlur`").
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
   - Tempo (easing) of `Fade`, `Rotate`, `SetVolume`,
     `DrawTransition`, `Shake` is never read (`Move` and `Zoom`'s is:
     "History", "Tempo of `Move` and `Zoom`").
   - A 4-argument `Fade` probably doesn't wait in the exe (49 calls
     wait in ours); check under wine first.
5. Gaps from the unread-argument notes (2026-10-16; each built-in's
   arguments the converter doesn't read, with counts, in
   `research/nitroplus/builtins.md`, "Arguments not read"), most
   noticeable first:
   - Tempo (easing) of `Fade`, `Rotate`, `SetVolume`, `DrawTransition`
     (`Move` and `Zoom` done: "History", "Tempo of `Move` and `Zoom`"):
     fades and turns over time aren't engine motions yet, and a sound's
     volume has no curve. A move with a fade folded into its keys still
     goes straight.
   - Story `CreateText`: a sideways text turned by `Rotate` sits about
     17 px lower than the original's under wine (cause unknown; "History",
     "Story `CreateText`").
   - `DrawTransition`'s start opacity and softness; `Shake`'s `n` and
     tempo; `SetBlur`'s `wait`.
   - `Random` and `Conquest` return 0 whatever their arguments.
   - `CreateEffect`'s place and size; `SetBacklog`'s speaker;
     `MoveCamera`'s arguments in a process (4 calls).
   - `LockVideo`, `WaitText`, `SetFrequency` (but 1000),
     the time limit (?) of `WaitAction` and the
     other waits: dropped; meaning to check in the exe.
6. Gaps from the script coverage (2026-10-15; every file, built-in,
   `Request` action, text tag, `$SYSTEM_` variable and `system.ini` key
   with its status in `research/nitroplus/builtins.md`, "In the
   scripts: coverage"). Every file has a status (no gaps there); new
   gaps, most noticeable first:
   - `CreateEffect` Drop (1, mc04_052's snow: an image per drop, swung
     and turned) and Lens (2). Rain is converted (in review, #161): `convert/rain.rs` plays
     the exe's rain (`00435e60`, `00436390`) into a 24-frame strip of the
     effect's size (`$SYSTEM_effect_rain_dencity` drops, at most 128,
     grey streaks 196 px with the exe's alpha ramp, falling their speed
     per 16 ms, redrawn every 42 ms), looped; its place is read, and
     `Zoom` and `Rotate` reach the strip (z as the `spzoom` angle, x or
     y as a shorter zoom, no perspective). Unverified: the exe's frame
     period (a run-time table; 16 ms assumed), so the fall speed. The
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
   - `DrawEffect` waves (79), `CreateStencil` (59), `CreateMask` (31),
     `SetSurface`/`CreateSurface`, `LoadImage` as a texture source
     (Muramasa leaves `loaded_textures` off).
   - `Request(…, NoIcon)` (4) is dropped without a note: a `Request` on
     an object the converter doesn't keep returns silently, so the dry
     run's notes undercount those.
   - `system.ini`: the auto-mode icon (`icon_auto`, 15 frames) and the
     line-end kinsoku (`bad_end`) aren't used.
   - Check under wine: `<BR>` (16 uses) isn't in the exe's tag list but
     is a line break in ours; 13 `DrawEffect` names have a trailing
     space (`"LowWave "`) the exe may not match.
   - Small: 47 sounds missing from the archives, 3 transition masks
     missing, 2 statements that don't parse, 2 voices outside a text
     block, 3 BGMs without loop played looping (the notes give counts
     only, not where).

**Leftovers** (counts from the dry run's notes):
- `CreateProcess` (510 calls; the battle HUD `CP_*` in
  `function_cockpit.nss`, fire flicker, comic panels): parallel NSS
  processes. One that ends in an endless loop of `Fade`/`Move`/`Zoom`/
  `Rotate`/`BezierMove`/`Wait` (or `Shake`/`Wait`) loops its pictures'
  motion in the engine from its `Start` (`convert/loops.rs`: the loop
  played twice on paper, the second pass keyed; 2,581 looped motions,
  the dry run's `CreateProcess` notes 4,205 → 2,459, then 1,756 with
  C1). The rest: Status. A process that only runs
  `Rotate`/`Move`/`Fade`/`Zoom` (the roll bar's `CP_RollBar`,
  `CP_RollBar_NoBG`) runs at its start without blocking, and a
  `WaitAction` on it waits for it; the roll bar's endless wobble
  `CP_RollBar_Auto` (59) is still left.
- Non-blocking `Move`/`Zoom` over time (about 1,200 each), drawn as
  cross-fades.
- `Rotate` about z of a picture is CONscripter's `spzoom` angle (still) or an
  `spmotion` angle (over time); left: about z of other objects (70),
  about x or y over time (14, drawn at once). `CreateTexture` from an
  object or the screen (1,864).
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

## History

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
converting (the airship kitchen's `$材料名前`, `$料理名前`; `Random` is 0
then, so the ingredient is always 豚肉).

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
`docs/audit-2026-10.md` §5):** the first 170 `#` flags ran %800-%969,
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
   `conv-cons/src/save_grid.rs`, Muramasa's values in
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
   `conv-cons/src/line_log.rs`; Muramasa's values in
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
   `PictureSettings`, in `conv-cons/src/picture_settings.rs`; Muramasa's
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
