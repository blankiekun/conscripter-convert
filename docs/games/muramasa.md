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

Whole game, script only (about 10 s, 2.2 GB peak; writes `0.txt`, no
assets; use it for anything about logic):

```sh
target/release/conscripter-convert convert-nss <game> \
  --file ma00_000.nss --chain 2000 -o out/muramasa-dry --dry-run
```

Full build: the same without `--dry-run`, plus
`--font <a Japanese TTF, e.g. ipam.ttf> --title 装甲悪鬼村正` (the font
is copied into the output as `default.ttf`). It takes
about 8 minutes and **about 6.7 GB of memory**: run it in the
background, one at a time, and say so before starting one.

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
| Story control flow (files, scenes, functions, `if`/`while`, choices, story flags, route ranking, heroine deaths) | ✅ Compiled; 879 of the 883 story files reached from `ma00_000` |
| Text: vertical windows, face windows and name plates, ruby, `<k>` waits, voices | ✅ Converted |
| Backgrounds, sprites, fades, mask wipes, zoom, shake, blur, BGM, sound effects | ✅ Converted (timed actions drawn as cross-fades, see "Leftovers") |
| Choices | ✅ Drawn like the original's `SetChoiceBase` plates |
| Parallel processes (`CreateProcess`: battle HUD, fire flicker, …) | ❌ Not converted; planned through Lua |
| Additive / multiply blends, `Rotate`, `SetFrequency`, camera moves | ❌ Not converted |
| 立方陣 minigame (`function_magiccube.nss`) | ❌ Not converted: blocks 村正END and TRUE END |
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

## How the compiler works

`convert.rs` (read its module doc first) **runs the story at convert
time**: helpers are inlined with their parameters bound by token, the
engine's named objects (textures, colours, sounds, text) are modelled,
and each built-in updates that model. What the player would see becomes
IR ops at the points where the original waits:

- **Pictures:** every visible texture or colour object is a sprite
  layer ordered by draw priority. A blocking `Fade`/`Move`, a `Wait` or
  a text page shows the changes since the last such point as one
  cross-fade over the action's time. Non-blocking actions join the next
  blocking point. `PrintBG` freezes the screen under a cover and
  rebuilds the scene; the cover's fade-out is the cross-fade.
- **Text:** the window comes from the `SetText*` helper's layout
  (`LoadText` size and pitch, `Rotate` 90° = vertical, `Move`). Face
  windows and name plates are `Hideable` textures over the box, drawn
  into a generated window picture (`system/window_<hash>.png`, a new
  `setwindow` when it changes); speaker names aren't printed. A blank
  line ends a page, `<k>` becomes NScripter's `@`, `{code}` runs before
  the page it starts (seen under wine). Vertical text is full-width
  (`tateyoko 1` exits on a half-width character).
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
  (`RANKINGS`), so the ranking and the death of the heroine with the
  most affection happen at run time as in the original.
- **Library files reached as a "next file"** (`function_magiccube.nss`)
  are skipped with a note.

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

Result (2026-10-13): 0 problems. 一条END, 香奈枝END (through the whole
airship exploration), 茶々丸END, and 悪鬼編 up to the 立方陣 with four
bad ends checked on the way. Not done: 村正END and TRUE END (behind the
minigame), state across relaunch and in the web build (step 11.5).

## Status: in progress (2026-10-13)

The whole story compiles from the prologue to the 立方陣, and all four
chart routes reach their endings in the converted control flow. Checked
in the engine: ma01_001 → ma02_003 (29 files, about 500 clicks) and four
forks (a runtime `call_scene`, a heroine death, the ranking, a battle
choice). Scope: extras and movies are out for now; the 立方陣 is
being ported; how closely to follow the battle HUD is open.

Next, roughly in order: the 立方陣, a long engine skip run per route,
the system screens (title, save/load, settings, backlog), then the
largest gaps below.

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
- Engine: a half-width character under `tateyoko 1` exits CONscripter
  with status 255 (`ONScripter_text.cpp`, drawChar's rotation path).
