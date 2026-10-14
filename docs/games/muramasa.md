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

- Start at `ma00_000.nss`, the new game of the boot script (the
  prologue, and the file that unlocks 魔王編). Starting anywhere else
  skips the unlock.
- `--file X.nss` without `--chain` compiles one scene.
- `NSS_TRACE=1` prints each file as it is compiled and a `flag %N name`
  line per story flag.
- Every build prints a ranked "not converted or approximated" list and
  runs `conscripter-convert lint` on the output.
- The output is deterministic: two runs give the same `0.txt`.

## What converts

| Area | Status |
|---|---|
| NPA archives, images (PNG, JPEG), BGM, sound effects, voices | ✅ Decoded (whole-corpus check not done) |
| Movies (`.ngs`, MPEG-1) | ❌ Out of scope for now (decided 2026-10-13) |
| Script parsing | ✅ All 913 files; 17 places that don't parse are all typos or dead code |
| Story control flow (files, scenes, functions, `if`/`while`, choices, story flags, route ranking, heroine deaths) | ✅ Compiled; all 883 story files reached from `ma00_000` |
| Text: vertical windows, face windows and name plates, ruby, `<k>` waits, voices | ✅ Converted; vertical columns where the original's are (checked under wine), turned marks (《「―) centred, English keeps its spaces |
| Backgrounds, sprites, fades, mask wipes, zoom, shake, blur, BGM, sound effects | ✅ Converted (timed actions drawn as cross-fades, see "Leftovers"); volume changes on a playing sound ❌ not yet (only a fade to 0 is), see "Next" |
| Choices | ✅ Drawn like the original's `SetChoiceBase` plates |
| Parallel processes (`CreateProcess`: battle HUD, fire flicker, …) | ❌ Not converted; planned through Lua |
| Additive / multiply blends, `Rotate`, `SetFrequency`, camera moves | ❌ Not converted |
| 立方陣 minigame (`function_magiccube.nss`) | ✅ Ported (see "The 立方陣" below) |
| System screens (title, save/load, settings, backlog) | ❌ Not started |
| Extras | ❌ Out of scope for now (decided 2026-10-13) |

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

The checker (`cons_trace_check.py` with `research/nitroplus/cons_trace_check.json`)
found the seven bugs fixed below; on the rebuilt game its `text` and
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

Testing notes: other agents stop engines with `pkill -x conscripter`;
run ours through a symlink with another name (`--engine
out/conscripter-mura`). The route runs and `--at` starts are in
`research/nitroplus/engrun.py`.

Next, in order:
1. **Sound volume** (half done). The converter ignores `SetVolume`
   (1,406 changes of a playing sound in the build's list; starting
   volumes too) unless it goes to 0, so sounds faded to 1 keep playing
   at full volume, and loops faded that way never stop (the checker's
   `loop` rule). The engine side is in CONscripter (2712225):
   `chvolfade ch,vol[,ms]` and `bgmvolfade vol[,ms]`, a script volume
   in percent of the setting, faded while the script goes on, kept
   with the channel for its next sounds (set it before `dwave`) and in
   saves. Left: an IR `Op::SoundVolume { file, percent, ms }`, its
   emitter in `conv-cons` (the channel the next `SePlay` takes when it
   comes right before it; a script that has any gives every sound a
   volume), and in `nitroplus/convert.rs` `set_volume` and
   `flush_sounds` (the ramp from the volume at `Play`) with the exe's
   curve (`research/nitroplus/builtins.md`, `SetVolume`): percent =
   100·(volume/1000)^1.661, so 1 is silent and 500 is 32 %. A fade to
   0 over time could use `dwavefadeout` (CONscripter 922b73f).
2. Check against the original: the figure in `ma01_002` (おれ→ ○□卍□○
   ←？ with ／＼ strokes) only lines up written horizontally, and comes
   apart in its vertical box.
3. A replay of route 9 with `--shot-pages` (engine 0.19.0) stopped
   drawing at `f_md01_002:158856` (`print 10,500`, page 18,573) with
   the process spinning; the same run passed before. Not reproduced.
4. **System screens** (step 7), under way. The original was filmed
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
   1. Button strips joined from the `000`-`003` part files at convert
      time; confirmations as pictures (`ConfirmDialog` + a picture per
      question, backdrop, the 記録をしました notice).
   2. Title: layers, hover and click sounds, BGM, `Continue` (quick
      load) and `Extras` actions.
   3. Right-click menu (a new `CommandMenu`) and the quick bar
      (`ButtonBar` with a base picture, hidden at the right edge and
      sliding in as in the original).
   4. Save/load grid (a new `SaveGrid`: 20 slots a page, 5 pages,
      hover details).
   5. Backlog (line scrolling, voice icon, scroll knob) and the
      click-wait icon.
   6. Settings drawn as one picture (a new `PictureSettings`: vertical
      sliders, 入/切 pairs, voice window, version line).
   7. Motion: fire flicker, opening, menu cascade, dialog unroll, quick
      bar slide.

   The user's decisions (2026-10-14): the pressed state (`003`) becomes
   an engine feature that is on for every script; the settings CONscripter
   lacks (music/SE/voice mute, voice stops on click, auto waits for the
   voice, auto/skip stop at a choice, movie skip, game speed) are added
   to the engine the same way (since then no engine addition is opt-in); 選択肢に戻る is implemented
   (a save to a reserved slot before each choice).
5. The largest leftovers below: `CreateProcess` (via Lua), non-blocking
   `Move`/`Zoom`, `AddRender` (needs a per-sprite blend mode).
6. Step 11.5: `#` flags (globals from %800) across quit and relaunch,
   and in the web build.
7. Optional: the 18 files with no `else` (below).

**Leftovers** (counts from the dry run's notes):
- `CreateProcess` (510 calls; the battle HUD `CP_*` in
  `function_cockpit.nss`, fire flicker, comic panels): parallel NSS
  processes. Planned as Lua coroutines through CONscripter's `USE_LUA`
  bridge (`NSCALL_animation` per frame).
- Non-blocking `Move`/`Zoom` over time (about 1,200 each), drawn as
  cross-fades.
- `AddRender` (2,536) and `MulRender` (418): drawn normally. `lsp2add`
  only fits top-layer effects (the engine draws all `lsp2` sprites above
  `lsp` ones); an additive background needs a per-sprite blend mode in
  CONscripter.
- `Rotate` (620), `SetFrequency` (1,170), `MoveCamera`, `SetTone`,
  `SetShade`, `BezierMove`, `CreateTexture` from an object or the screen
  (1,864).
- 18 files whose `if`/`else if` chain on earlier flags has no `else`:
  by the story's logic one branch always matches; the fall-through ends
  the run.
- `SetBlur` strength is a guess (1.5 px per pass), unverified.
- Engine: an exit with status 255 in vertical text was put down to a
  half-width character under `tateyoko 1`, but CONscripter draws those
  (rotated) and nothing in drawChar exits. It was most likely a script
  error in the text (a half-width `/`, `#`, `?` or `%`; up to 0.18.0
  every script error exited with 255). Now a script error exits with 2
  and the trace's `error` record gives the message and the line; the
  checker's `text` rule finds those characters in text lines.
- Engine: CONscripter segfaults on `end` after a `dwave` voice with Ctrl
  held (seen once in a small test script; not looked into).
