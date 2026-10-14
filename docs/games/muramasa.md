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
| Text: vertical windows, face windows and name plates, ruby, `<k>` waits, voices | ✅ Converted |
| Backgrounds, sprites, fades, mask wipes, zoom, shake, blur, BGM, sound effects | ✅ Converted (timed actions drawn as cross-fades, see "Leftovers") |
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
- **Headless engine runs:** an Xvfb display of its own (`-screen 0
  1024x576x24`), `SDL_VIDEODRIVER=x11 SDL_AUDIODRIVER=dummy`,
  `WAYLAND_DISPLAY` unset, the output folder as the working directory,
  `xdotool` clicks and `import -window root` screenshots. To start at a
  scene, copy `0.txt`, add a label before the line and `goto` it right
  after `erasetextwindow 0`, and symlink `arc.nsa`, `default.ttf` and
  `ons_args` next to the copy.
- **With the debug trace** (`docs/new-game.md`, "Checking a build with
  the debug trace"), none of that is needed: `SDL_VIDEODRIVER=dummy`
  and `--auto-play` run without Xvfb or xdotool, and `--goto <label>
  --vars <file>` starts at a scene with the story flags preset (the
  `--at` patch of `0.txt` did this by hand; a route replay of an hour
  became a 22-second reproduction). Write the vars file from
  `nstrace.py`'s flags, or let a run write it (`--dump-vars file`, at F8
  and at exit). `--trace-vars 300-899` records the story flags as they
  change, to compare with `nstrace.py --flags` file by file. Skip mode
  used to end on any key release, which made xdotool-driven skipping
  crawl; auto-play doesn't need it, and releasing Shift, Ctrl, Alt or a
  function key no longer ends it (other keys and clicks still do).
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

**Engine skip runs** (playbook step 4): `research/nitroplus/engrun.py`
plays each ending of the route paths in CONscripter, every page drawn,
on an Xvfb display of its own (Ctrl held down: the engine leaves skip
mode on any key release). nstrace records the options each playthrough
takes; the driver patches a copy of `0.txt` to take them by itself, skip
the 立方陣 with its recorded result, and log each story file; a run
passes if it reaches the end and passes the same files as nstrace. Five
runs (一条END, 香奈枝END, 茶々丸END, 村正END, TRUE END; `--all` adds the
bad ends), about 8 pages a second, one core and 200 MB each: run them
side by side, one display each.

```sh
for n in 1 2 3 9 10; do
  python3 -I research/nitroplus/engrun.py out/muramasa-cons \
    research/nitroplus/paths/{1-ichijou,2-kanae,3-chachamaru,4-muramasa}.txt \
    --engine <conscripter> --only $n --display :8$((n % 10)) > out/engrun-$n.log &
done
```

To look at one place deep in a route, `--at <file>` starts the run at
that story file with every variable as nstrace had it there (a crafted
save: the variables, not the pictures or music on screen); an engine
problem there shows in seconds instead of an hour. The engine writes its
errors to `stderr.txt` in the game folder, not to the terminal. To find
the line, add `lsp 998,"TRACE:L<n>",0,0` before each line of the file in
the patched copy: the last one in `stderr.txt` is the line that failed.

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

## Status: in progress (2026-10-14)

The whole story compiles, the 立方陣 included, and every ending of the
chart is reached in the converted control flow. **Engine skip runs**
(2026-10-14, see "Testing"), the full build played in CONscripter with
every page drawn and the files checked against nstrace:

| Run | Ending | Files | Time | Result |
|---|---|---|---|---|
| 1 | 一条END | 246 | 40 min | passed |
| 2 | 香奈枝END | 349 | 41 min | passed |
| 3 | 茶々丸END | 300 | 44 min | passed |
| 9 | 村正END | 508 of 510 | 56 min | **aborted** in `md06_032` |
| 10 | TRUE END | 514 | 56 min | passed |

Run 9 aborted with `malloc(): unaligned tcache chunk detected`
(SIGABRT): heap corruption in the engine, noticed in `malloc` under
`lspCommand` → `loadImage` → `createSurfaceFromFile` →
SDL (`cg\anime\slash01a_02.png`). The damage happened earlier: run 10
plays the same files without trouble, and an AddressSanitizer build
(`-fsanitize=address`, flags as in "Build" plus the SDL2_mixer
`PKG_CONFIG_PATH`) started at `md06_030` with `--at` reached the end
with no error. Not found yet: an ASan run of the whole route 9 (about
2-3 hours) would show the first bad write. Testing paused here
(2026-10-14) while CONscripter gets better debug options.

Fixed on the way (each found by the runs, all in the current build):
- A `?` in horizontal text: NScripter reads it as an array reference
  and exits with 255 (the message goes to `stderr.txt`). Every route
  passed one, in `ma03_027vs`: the NSS `<?>` tag (93 uses), passed
  through as text. Text lines now write `?` full-width, `<?>` is
  dropped, and lint reports a `?` in text.
- Non-blocking actions were cross-faded over their whole time at the
  next sync: a 270 s background scroll (`ma04_013vs`) blocked the
  player for 270 s, 51 cross-fades were over a minute. Now at most a
  `Wait`'s time, or 1 s where the script doesn't wait.
- The full build ran out of memory (7 GB) on 60x zooms; now 1.2 GB.

Testing notes: other agents stop engines with `pkill -x conscripter`;
run ours through a symlink with another name (`--engine
out/conscripter-mura`). The skip runs and `--at` starts are in
`research/nitroplus/engrun.py`.

Next, in order:
1. **Find run 9's heap corruption** (with CONscripter's new debug
   options, or an ASan build over the whole route 9), fix it in the
   engine, and rerun the five skip runs.
2. **System screens** (step 7): title, save/load, settings, backlog,
   built as in-script menus from `cg/sys/*` parts, as for 最果てのイマ
   (`conv-cons/src/menu.rs`); the settings screen also shows the build's
   version line (`version.txt`). A big item: check with the user first.
3. The largest leftovers below: `CreateProcess` (via Lua), non-blocking
   `Move`/`Zoom`, `AddRender` (needs a per-sprite blend mode).
4. Step 11.5: `#` flags (globals from %800) across quit and relaunch,
   and in the web build.
5. Optional: the 18 files with no `else` (below).

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
