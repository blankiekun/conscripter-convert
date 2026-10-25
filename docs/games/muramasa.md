# 装甲悪鬼村正 (Nitroplus NSS)

Full Metal Daemon Muramasa (Nitroplus, 2009), the reference game for
Nitroplus NSS (`docs/engines/nitroplus-nss.md`): scripts are **plain
source text** in an encrypted NPA, so the converter is an NSS compiler.
The English port joins the same build (`docs/games/muramasa-en.md`).

- **Game files:** install the retail DVD (InstallShield 10.5) or
  `unshield` its `product/data1.hdr`: data in `ゲームデータ/` (`<game>`,
  on a local disk: a full build reads 3 GB), movies in `ムービー/`.
- **Code:** `crates/conv-engines/src/nitroplus/`, this game's part in
  `convert/muramasa.rs` and `convert/muramasa/`; `research/nitroplus/`.

## Status

Active (Japanese and English in one build).

| Area | State |
|---|---|
| Archives, pictures, sounds, voices, movies, script parsing | ✅ |
| Whole story (883 files, every ending; route check and 10 route runs) | ✅ in the engine, headless |
| Text, choices, pictures, motion, processes, blends, sounds | ✅ (approximations parked) |
| System screens, extras, 立方陣 | ✅ each checked beside the original |
| English text and pictures, censored/uncensored layer | ✅ (`docs/games/muramasa-en.md`) |
| Web build | 🟡 JP (4 routes) and EN (1) played 1800 s each; known bugs |
| Dry-run `score` (`metrics.json`) | 99.90 (picture 99.67) |

Known bugs:

- Trace diff: pages the original shows and ours lacks (`ma03_015`…;
  lead's lane R).
- Scene-start check (640 of 865 scenes clean, before
  `scene_objects.py` read containers' children: rerun it):
  `mc01_010vs`'s lock-on mark is not a bug (`CP_LockOn` is
  top-level, not under `CPW`, so `Cockpit_AllFade0` leaves it; it
  sits behind the full-screen `ev946`, as in the original).
- A file reached by a path other than the first compiled one gets
  that path's music: `SoundPlay` of the BGM already playing is
  left out (md05_027a's `@mbgm15`; pictures are placed anew).
- `--original-archives` (`docs/original-archives.md`) fails with
  `--english`.
- `layer\uncensored\` unchecked on a censored CG.
- Left out: `$SYSTEM_text_interval` 128 and `$SYSTEM_text_waitkey` false: the
  scripts set them only around the 6 staff rolls, pictures with no text page.
- A load restarts looping `spmotion`s from their start (engine
  `readMotion`: `motion_start` = now): mc01_020vs's cockpit sway (5
  sprites: frame, bg201, gauges) differs on the whole screen, loaded vs
  played 6,059-16,316 px. The list lens restores once engine PR
  blankiekun/CONscripter#203 is merged and pointed at.
- Web runs (1800 s real time, auto-play, random choices; `cons_trace_check` "missing"
  findings are the checker's loose-build lookup, not the game): JP prologue 370 pages;
  JP mb01 3166; mc01 2757; md01 3226; md05 1062 (reached the end); EN mc01 (Kanae route
  `--at mc01_001`) 382 pages (script waits up to 46 s; English text fits the window, no
  Japanese in any page). A shot's `pgN` is the last page begun: after a
  `TypeBegin…IO`/`…CIO` page the window is closed (`windowfade 0,150`) as there; the
  vertical boxes (`TextWindow*.png`) are black, so unseen over black.

Static checks (`docs/new-game.md` steps 6-7; 20 fields, 62 `set_system_variable` cases, 6
every-frame passes); the script locks and roll speed are done, what is left:

- Present: the 1024x576 A8R8G8B8 frame is drawn to the desktop-sized back buffer as a quad with
  no -0.5 texel offset, LINEAR, CLAMP (0048af60, quad 0048c6e0): a 0.5/r px shift and blur, up
  to ~128 levels at r=1 on hard edges (flat areas 0); then `Present` 48b308 stretches it to the
  window (driver filter); ours: one linear scale, identity at 1024x576.
- Unconfirmed: no write found to `+0xa0` (tempo) in 00446010 or `+0x100` (alias) in 004475b0,
  where builtin.rs cites them (pointer forms; needs Ghidra xrefs). `CreateStencil` clears `+0x9d4`
  in 00409fa0, not 00443800; `CreateChoice` clears it too (00442a70, 00442bb0; only in 0_boot and
  prutestselect, which we replace). JPEG (libjpeg 6b) decode options not read.
- Fine: no gamma ramp, no pixel shaders, software composition into the locked texture, no
  blending at present; `+0x9d4` (8 writers), `+0x11f4` (4), speed index 005dcaf0 (1).
- PNG gamma: libpng reads `gAMA` (0049b9d0, 004a4730) but no 2.2 screen gamma is in the decomp;
  the 353 PNGs in a `cg` sample all carry 0.45454 (identity); the rest of the PNGs unchecked.


Next: (1) the known bugs above; (2) release.

## Scope

Owned: the retail PC DVD (1.00), the English release (JAST USA 2021).
Converted: story, system screens, extras, 立方陣, movies, both languages
in one build (`docs/string-tables.md`; `layer\en\` pictures,
`layer\uncensored\` event CGs, Language and Pictures settings; a
language change restarts). Manual: print only; the English
`Manual.pdf` shows the same screens in the port's look.

## Build

`conscripter-convert build <game> -o <output> [convert options]` runs
`convert-nss --file ma00_000.nss --chain 2000 --title 装甲悪鬼村正`
(README). The **dry run** (10 s, 1.2 GB) answers logic questions:

```sh
target/release/conscripter-convert convert-nss <game> \
  --file ma00_000.nss --chain 2000 -o out/muramasa-dry --dry-run
```

The **full build**: `--dry-run` replaced by `--font <Japanese TTF,
e.g. ipam.ttf> --title 装甲悪鬼村正`; 3 min, 1.2 GB, 3.5 GB written, one
at a time. English: `build <english game> --japanese <game>`.

- `--webp [--webp-quality 80]` for the web (`--chain 2`: 66.5 MB PNG,
  13.3 MB at 80).
  The web build needs `--loose` too: it fetches each file whole, so a
  1.9 GB `arc.nsa` never finishes loading. JP+EN (`build <english game>
  --japanese <game> --webp --loose`): 10 min, 6.0 GB peak (a 4.5 GiB
  cap stops it after the text tables).
- Web play tools (scratch start, route start with `--at`, screenshots every N s):
  `research/nitroplus/web_play/` (`mkstart.py`, `play.sh`).
- The prologue `ma00_000.nss` also unlocks 魔王編. `--file X.nss` alone:
  one scene; full build with `--chain 6`: a few files' assets, seconds.
- Movies: `ムービー/`, `movie/` or `movies/` in or beside `<game>`, or
  `--movies <dir>`; encoded once to `movie/` (H.264). Without them:
  `--no-movies`.
- `NSS_TRACE=1`: each file compiled, a `flag %N name` per story flag.
  Builds rank what is "not converted or approximated", lint, and are
  deterministic.

## Testing

Recipes: `docs/tools.md`, `docs/new-game-reference.md`.

- **Route check** ("Route simulation", dry run): `nstrace.py` walks
  `0.txt` along each committed path against the route map, to every
  ending and five bad ends. `PUZZLE 1`/`0` in a path solves or fails the
  立方陣; `--flags <file> --watch '$Muramasa_Flag,RC_N['` prints flag
  changes.

  ```sh
  conscripter-convert npa extract <game>/Muramasa.exe <game>/nss.npa -o out/muramasa-nss
  conscripter-convert nss route out/muramasa-nss > out/muramasa-route.txt
  python3 -I research/nitroplus/nstrace.py out/muramasa-dry/0.txt \
    research/nitroplus/paths/{1-ichijou,2-kanae,3-chachamaru,4-muramasa}.txt \
    --route out/muramasa-route.txt --quiet
  ```
- **Engine runs** (full build): the debug trace with `--trace-ignore
  999` (the marker sprite); heed `long wait` and `large image` warnings.
- **Route runs:** `research/nitroplus/engrun.py` (nstrace's options)
  plays each ending, every page drawn; it passes on reaching the end
  through the same files. `--all` adds bad ends; `--at <file>` starts
  there with nstrace's variables; `--wrap 'valgrind -q'` or an ASan
  `--engine`:

  ```sh
  P=$(ls research/nitroplus/paths/*.txt)
  for n in 1 2 3 9 10; do
    python3 -I research/nitroplus/engrun.py out/muramasa-cons $P \
      --engine <conscripter> --only $n > out/engrun-$n.log &
  done
  ```

### Running the original under wine

Rules and recipe: CLAUDE.md, `docs/tools.md` ("Matching the
original"); 続・殺戮のジャンゴ runs the same way.

- Without `ja_JP.UTF-8` the save folder's name is garbled and a new
  game falls back to the title. `HKCU\Software\Wine\Drivers`
  `Graphics=x11`.
- **Audio** (needs DirectSound): `Drivers\Audio=alsa`,
  `WINEDLLOVERRIDES="winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"`,
  `PULSE_SERVER=unix:/nonexistent`, `ALSA_CONFIG_PATH=<file>` holding
  only `pcm.!default { type null }` (+ `ctl.!default`, `pcm.null`).
- `wine Muramasa.exe` in `C:\muramasa\` (symlinks to `<game>`).
  Fullscreen 1024×576 sits at (128, 125) of 1280×800: `import -display
  :N -window root -crop 1024x576+128+125`; `ffmpeg -f x11grab
  -video_size 1024x576 -i :N+128,125`. Stop: `wineserver -k`, Xvfb by
  PID.
- Wine's ＭＳ 明朝 stand-in steps ~18 px a character at size 20: compare
  column starts, not steps.
- **Patch first:** unpack `nss.npa` (`npa.py`), edit (cp932), `--pack`
  a copy linked as `nss.npa`. `0_boot.nss`: uncomment `//$Logo =
  true;` to skip logos; `Load(3);` after `#play_speed_plus =
  #SYSTEM_play_speed;` loads slot 3, a save written to any scene ("Save
  format").
- **Trace DLL:** `GAME=<game> research/nitroplus/tracedll/ntrace.sh -o
  orig.jsonl --pages 60` plays from `ma00_000` unclicked; `th` 0 is the
  story; a `page` record comes when the block loads, the page shows at
  its `Request($textnumber, Enter)`. `--frames`: per-frame state (as
  `docs/games/django.md`). Hooks: `ntrace.c`.
- **Reference-trace patches** (`reftrace.sh muramasa`): `mura-patch/`
  turns the first H-mode `select{` (`sys_hmode.nss`) into `if(0){` and
  the 立方陣's (`function_magiccube.nss` line 261) into
  `$立方陣正解=true;$Loop_end=false;$Answer=true;if(0){`, each on its
  own line.
- **Trace diff** (`--source-map` build):

  ```sh
  target/release/conscripter-convert convert-nss <game> --file ma00_000.nss \
    --chain 3 -o out/mura3 --font ipam.ttf --title 装甲悪鬼村正 --source-map out/mura3.map
  research/tools/scratch.sh out/mura3 <scratch>
  <scratch>/run.sh --trace ours.jsonl --auto-play 60 --real-time --trace-ignore 999
  research/tools/trace_diff.py orig.jsonl ours.jsonl --map out/mura3.map \
    --pages 50 --ms --fps 60 --thread 0
  ```

  Noise: the reader's wait after `Request(…, Enter)`; `extra` from a
  page's own code; `procAutoIcon`; `missing` calls that draw nothing;
  the text box's own calls; calls folded into one of ours.

### Per-scene snapshots

The original's memory at each story scene's start, from saves:

```sh
python3 -I research/nitroplus/npa.py <game>/Muramasa.exe <game>/nss.npa <nss>
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> > manifest.tsv
GAME=<game> research/nitroplus/tracedll/ntrace.sh -o scenes.jsonl --patch <patch> --saves <saves> \
    --scene-dumps research/nitroplus/local/<dir> --clicks 500 --pages 1 --seconds 5400
```

Slot i goes to scene i and each `_MAIN` is patched to `Load(i+1)`: one
wine process dumps all 881 (2.5 s each, 41 min). `dumppack.py` packs,
`muramasa-scene-dumps.tsv` indexes, `exe_call.py`'s `load_dump`
rebuilds. A loaded save lacks moves in flight, sound positions,
`CreateProcess` threads, timers, backlog, and route flags beyond the
template's.

#### Scene-start check

The original's objects at each scene's first text block against our
sprites:

```sh
python3 -I research/nitroplus/scene_saves.py <game>/Muramasa.exe <template slot> <nss> <saves> <patch> --at page > manifest.tsv
research/tools/headless-run.sh env GAME=<game> research/nitroplus/scene_chain.py manifest.tsv <patch> research/nitroplus/local/<dir> \
    --stall 40 -- --patch <patch> --saves <saves> --scene-at page --clicks 300
research/tools/.venv/bin/python -I research/tools/scene_check.py research/nitroplus/scene_objects.py <game>/Muramasa.exe \
    research/nitroplus/local/<dir>/index.tsv <start dumps> research/nitroplus/muramasa-scene-dumps.tsv <scratch> --jobs 3 --out check.jsonl
```

`scene_objects.py` reads a dump's objects, creating calls and render
records (x, y, alpha; zoom, rotation not compared yet), with the
children of names and windows (the battle HUD is under `CPW`; window
children's x, y not checked against a dump yet).

## Save format

Written by `builtin_Save` (`00411ea0` → `0046eb70`), read by
`0046fa80`/`00471b40`; `research/tools/nitroplus_save.py` `dump`,
`write`, `selftest` (checked under wine).

- `AppData\Roaming\Nitroplus\装甲悪鬼村正\1.00\`: `0001` = slot 1…,
  `common` (`val.npf` `#` variables, `cqst.npf` read marks), `images\`
  thumbnails.
- `.npf`: `NPSF`, u32 plain size, u32 1, u32 1, then zlib whose first
  0x1000 bytes are `table[b + key]` (`FUN_00401650`, `npa.py`'s table),
  key per file name (`val` 100, `script` 164, `frames` 138, `bklg` 248,
  `cmt` 93, `cpt` 72, `date` 8, `tm` 52, `thum` 8, `cqst` 32). Binary
  files start with a tag (`NPSL`, `NPSG`, `NPSS`, `NPSF`, `NPSB`,
  `NPSI`), `c5 57 f2 f1`, u32 size; `cmt`, `cpt`, `date`, `tm` are
  cp932 text.
- `val.npf`: u32 count, then 1024 bytes per variable: u8 type (0 null,
  1 string, 2 number), name, 4 NULs, f64, string, 4 NULs, array key,
  4 NULs. The page is `$SYSTEM_present_text`, the file `$GameName`.
- `script.npf` (`NPSS`, `FUN_00471580`): per `call_chapter` level, the
  **full text of every loaded script**, then how far each function,
  scene and chapter was compiled; a load recompiles these, not
  `nss.npa`. `frames.npf`: objects as the calls recreating them;
  `bklg.npf` backlog; `thum.npf` 160×90 BGRA.

So `write` with `"scene": "<file>.nss"` cuts the scene's text after the
current page and `_MAIN` after `call_scene $MainGameName`, and sets
`$GameName` to the target; the template stands on a story page, not a
choice.

```
python3 -I research/tools/nitroplus_save.py dump <game>/Muramasa.exe <saves>/0001
echo '{"scene": "md05_025.nss", "vars": {"$md05_２週目": true}}' > edit.json
python3 -I research/tools/nitroplus_save.py write <game>/Muramasa.exe <saves>/0001 edit.json -o <saves>/0003
```

## Approach

NSS to NScripter: functions as `gosub` labels, variables as variables,
object names as sprite numbers; Lua (`luasub`/`luacall`, every target)
only where NScripter falls short.

## How the compiler works

`convert.rs` (module doc first) is the NSS compiler;
`convert/muramasa.rs` this game's part (`MURAMASA`, a `Game`, picked
by `Muramasa.exe`). It **runs the story at convert time**: helpers
inlined (parameters bound by token), engine objects modelled, and the
changes since the last wait (blocking action, `Wait`, text page)
emitted as IR ops, one cross-fade each. Zooms: scaled copies
(`system/zoom/`), cut to the screen when large. Text windows follow
`SetText*`; faces and name plates are drawn into a generated window
picture. Each file compiles once, from the first state reaching it.
**Story flags**: variables story files both set and test; a condition
on one becomes a run-time comparison or a fork. `Random` is `rnd` where
the result stays in the script; 771 calls are picked at convert time.
Ranking and death (`RouteChicker*`, `SatugaiSystem`): NScripter
comparisons (`RANKINGS`).

**The 立方陣** (`function_magiccube.nss`): a 5×5×5 magic cube, 18 cells
of layers 4-5 filled with numbered tiles. `convert/muramasa/magiccube.rs`
→ `Op::TilePuzzle` → `conv-cons/src/puzzle.rs` (`btnwait` over
`transbtn` sprites) → `$立方陣正解`; `_MAIN` forks to `md05_025end` or
`md05_025gameover`. Failing offers a retry, then easy mode
(`$立方陣簡易フラグ`).

**Variables in `0.txt`:**

| Range | Use |
|---|---|
| %40-%69 | fork variables (which branch of a choice or runtime `if`) |
| %70 | leaf variable (which way through the last fork) |
| %100-%299 | the backend and its menus |
| %300-%799 | story flags (`$`, saved with each slot), in first-use order |
| %800 up | `#` flags, `globalon` (header `value800`); past %969 at %5200 |

### System screens

Each filmed in the original: title, right-click
menu, quick bar, confirmations (`cg/sys/dialog`), save/load, backlog,
settings, favour screen, click-wait icon; `convert/muramasa/screens.rs`,
engine screens where they exist (`SaveGrid`,
`conv-cons/src/save_grid_screen.rs`). Keys: right-click at a text wait
opens the menu and closes any screen, wheel-up the backlog, Esc asks
ゲームを終了しますか.

### Extras

特別付加目録 (`extra_*.nss`; values in
`convert/muramasa/extras.rs`, menu `conv-cons/src/picture_extras.rs`):
画像映写機 (CG gallery, 805 CGs, each a `#<cg name>` flag), 回想映写機
(ten H scenes), 電磁蓄音機/動画映写機 (BGM, songs, movies), 劒冑回転目録
(46 armour movies), the ライナー画像 button. 画像, 回想, 音楽 unlock with
`#ClearG` (any end roll); the sound room and 3D viewer need the movies.
Check: `tests/extras_smoke.py <build>`.

## Story structure and walkthrough

Walkthrough: rodemass, "ゲーム攻略メモ～装甲悪鬼 村正～"
(http://www5d.biglobe.ne.jp/~rodemass/game_k/gk_muramasa.html). The
route map (`conscripter-convert nss route`, `route.rs`) reaches all 883
story files from `ma00_000` (new game) and `ma02_001` (title option:
part 2); 84 hold choices.

| Route | Heroine | Files | Ending | How it opens |
|---|---|---|---|---|
| common | — | `ma00_000` prologue, `ma01`-`ma05` | — | new game |
| 英雄編 | 綾弥一条 | `mb01`-`mb04` | `mb04_026` 一条END | affection ranking at `ma05_023` |
| 復讐編 | 大鳥香奈枝 | `mc01`-`mc04` (airship exploration `mc02_031*`) | `mc04_052` 香奈枝END | affection ranking at `ma05_023` |
| 魔王編 | 足利茶々丸 | `md01`-`md04` | `md04_045aa` 茶々丸END | both endings above, then a new game (`#魔王編解放`) |
| 悪鬼編 | 村正 | `md05`-`md06` (village exploration `md05_004*`, 立方陣 at `md05_025`) | `md06_033a` 村正END, `md06_036` TRUE END | the other option at `md04_045` |
| game over | — | `mz00_000`, `mz01_*`, `*gameover` | — | battles, failed explorations |

Order: 一条, 香奈枝 (either first), 茶々丸, 村正 (from the save before
`md04_045`). `research/nitroplus/paths/*.txt` hold every chart choice
(file, option number); 茶々丸 options need `#復讐編終了` / `#魔王編解放`;
where the chart lists two, take the one keeping `$Muramasa_Flag` ahead.
Explorations: each room puts its destination in `$Next_GameName` and
passes a clock file (`mc02_031time`, `md05_004time`); past the limit,
game over. The most-loved heroine dies at the end of part 3
(`SatugaiSystem`).

## Progression variables

`research/nitroplus/progress.py` lists the 207 variables story files
set and read; the route gates (`%N` per `NSS_TRACE=1`, moves with the
compiler):

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

## Executables

| File | Version | sha256 |
|---|---|---|
| `Muramasa.exe` (1.2 MB) | 1.00 | `1d67e4771a7229224daf8b8508e74313fadd950f70ea17f6ce18d089882722b5` |

Addresses here, in `research/nitroplus/names.tsv` and
`docs/games/muramasa.findings.tsv` are this exe's; decompile with
`research/tools/ghidra/ghidra_export.sh` and `names.tsv`.

## Files and formats

`ゲームデータ/`: NPA v1 archives (zlib, encrypted) of standard-format
assets. **Screen 1024×576**.

| File | Size | Contents |
|---|---|---|
| `Muramasa.exe` | 1.2 MB | The engine; holds the NPA substitution table |
| `nss.npa` | 4.1 MB | 913 `.nss` scripts, Shift-JIS, CRLF (19 MB unpacked) |
| `system.npa` | 1.5 KB | `system.ini`: start script, font (ＭＳ ゴシック 18 px, black outline), volumes, icons, voice names |
| `cg.npa` | 1.3 GB | 5,469 images (3,167 PNG, 2,293 JPEG); folders `sys/extra`, `fw` (face windows), `ev`, `bg`, `st`, `sys/magiccube`, `sys/config`, `sys/telop`, `data` (transition masks), `anime` |
| `sound.npa` | 213 MB | 760 Ogg Vorbis: 56 BGM (`bgm/mbgmNN.ogg`), 704 sound effects |
| `voice.npa` | 1.5 GB | 35,530 Ogg Vorbis, `voice/<script>/<id>` |
| `../ムービー/*.ngs` | 353 MB | 78 MPEG-1 program streams ffmpeg reads as is; 46 are extras (`3d/ex_NNN`) |

**NPA** (`npa.rs`; `conscripter-convert npa list|extract`):
- Names: `byte[x] += 0xFC·x − (bytes of key1·key2) − (bytes of the entry
  index)`, the published Nitroplus scheme.
- Data: only the first `0x1000 + len(name)` bytes are encrypted:
  `b = inv[b] − k − i`, `inv` the inverse of a 256-byte permutation at
  exe file offset `0x134830`, `k` = low byte of
  `((0x87654321 − Σname) · len(name) + key1·key2) · size`.
- `research/nitroplus/npa.py` decodes; `--pack DIR NEW.npa` writes a
  copy with `DIR`'s files swapped in.

## Script language (NSS)

C-like source, interpreted. The parser (`nss.rs`, module doc) takes
all of it; `conscripter-convert nss check|builtins|refs|route` inspect
the corpus.

**Files** (913): story `ma` 159, `mb` 115, `mc` 251, `md` 353, `mz` 4
(`NN_NNN` part and scene; `a`/`b`/`c` branches, `vs` battles); library
`function*.nss` (8), menus `sys_*` (15), extras `extra_*` (5), boot
`0_boot*.nss`, a debug select.

**Constructs:** `chapter`, `scene`, `function`, `call_scene`,
`call_chapter`, `#include`, `if`/`else`, `while`, `select`/`case @label`
(choices), `return`, `break`, labels; `$name` in the save, `#name` in
system data; arrays (`$a[0]`, `$Name[word]`, `Array()`/`AssocArray()`),
string `+`. **Text**: `<PRE @box0> … </PRE>` with `<voice name= class=
src=>`, `<RUBY>`, `<k>` (mid-line click wait), `<FONT>`, `<BR>`,
inline code `{ … }`.

Traps:
- **Parameters bind by token**, like macros: `function
  FadeM("ナット名", 描画時間, t, $x)` uses `"ナット名"` and `描画時間` as
  written; missing trailing arguments are empty, extras ignored.
- **Strings name variables**: `Fade("$現ナット名", …)`.
- **Lookup is per file**: current file, then library; later definitions
  win (65 names defined twice or more).
- **Block comments nest** (`function.nss` hides old
  `TypeBegin*`/`SetText*` in `/*=====*/` banners).
- Tolerated typos: `$a++:`, missing `;` or comma, bare `@`, `$$name`,
  `*/=====`, `ture`.

**Flow:** `0_boot_開始スクリプト.nss` loops `while($GameCircle)
call_scene "nss/"+$GameName+"->"+$GameName+"_MAIN"` (`$GameCircle = 0`
ends). Each story file: `chapter main`, `scene X.nss_MAIN` (skip check,
CG flags, the content, `$GameName` = next file), `scene X.nss` (content).

**Built-ins:** ~1,700 NSS functions (`FwC`, `SetTextEXC`,
`TypeBeginCI*`, `CreateSE`, `MusicStart`, `PrintBG`, …) over ~120
engine built-ins (`Fade`, `Move`, `Delete`, `Zoom`, `Wait`, `Request`,
`SetVolume`, …; `research/nitroplus/builtins.md`). Objects ("nuts") are
named by strings, the number after the name their draw priority. Most
timed calls end in a "wait" argument: `false` lets animations run on
until `Wait`/`WaitAction`.
