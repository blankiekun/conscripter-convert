# TODO for the next agent

State on 2026-10-13 (updated by a fourth session; start with "Next"
below). Read this first, then `saiima.md` (local handoff
notes, git-ignored) and `PLAN.md` §4.2 and §6 ("Remaining work for
最果てのイマ").

## Rules (from the user)

- One commit per task, pushed separately. Engine (`engine/` submodule,
  github.com/blankiekun/CONscripter, branch `master`) first, then commit
  the submodule pointer together with the converter change (`main`).
- Fetch and rebase before every push; the user pushes to both repos too.
  Push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push`.
- No Co-Authored-By or Claude attribution lines in commits.
- Use existing NScripter commands before adding anything to the engine.
  Engine additions are opt-in, live in `engine/src/onsyuri/` (preferably
  `ONScripter_cons.cpp`), and are documented in the engine's
  `ADDITIONS.md` and in PLAN §4.2.
- Never touch `out/saiima-flow` (the user's saves).
- Test scripts: encode with `iconv -t cp932`, not `-t sjis`.

## Next

Opcodes 0x70 (leftovers), 0x59, 0x82 and 0x69 are done (unsupported
296 → 119); the engine has new filter kinds 6-8 and 8 effect slots
(engine `5516a1a`, released in CONscripter 0.14.0, tag `v0.14.0`,
Android versionCode 17). Slides (0x86/0x87/0x7f, unsupported 119 → 81) are done too, checked
headless in 0010.cd's 沙也加 slide. Pick the next item from "3. Next
work": 0x99 and 0x9d are the biggest left. Unchecked against the original under wine: quakes,
flashes (ours are under the text window; the original's cover it),
pulses (especially presets 6-8: blend 6 is assumed to lay white over
the copy, `tint 2`).

## Done in the third session (2026-10-13)

| Item | Converter | Engine |
|---|---|---|
| Release CONscripter 0.11.0 (all 5 workflows passed) | `9ea83ab` | `548b798` |
| `0x6d` quakes → `screenshake 4,…` + `delay` + clear (198) | `16c1059` | none |
| `0x6e` flashes → `lsp2add` colour rectangle for 100 ms (88) | `d511d99` | none |
| Bars always shown (user's request; hover hotspots removed) | `1231002` | none |
| Links like the original: colours measured under wine, underline, purple once visited, word sprite no longer 12 px low | `dd56369` | none |
| Submodule → 0.12.0 (another agent's release: web hooks `onsyuriBacklog`, `onsyuriSprites`, overlay `hide`/`show`) | `9d120f3` | `da8ad6b` |
| `0x6f` pulses → `screenpulse 3,…` + `delay` + clear (87); release CONscripter 0.13.0 (tag `v0.13.0`, Android versionCode 16) | this commit | `e696995`, `0ee75ce` |

Unsupported instructions: 2,331 → 1,958. `out/saiima-cons` is rebuilt
with all of it (needs CONscripter 0.13.0 for the pulses).

Notes from this session:
- Another agent may be working on the engine at the same time: always
  fetch and rebase before pushing either repo.
- `smoke.py` keeps its click queue in `%1000`+, so the converter's own
  variables must stay clear of 1000-1099 (link visited flags moved to
  `%2000 + target` because of this).
- Web build: the user's own web app keeps old game files per folder name;
  if a change doesn't show there, a renamed folder fixes it. Not a
  CONscripter bug (the user said not to fix it).
- Headless web check of the real game: serve a dir with symlinks to
  `arc.nsa`/`default.ttf`/`ons_args`, a patched `0.txt` (`mov
  %101,1:goto *conv_link1` after `erasetextwindow 0`), the web
  `conscripter.{js,wasm}`, and `conscripter.html` copied to `index.html`
  with a canvas-posting script injected (preserveDrawingBuffer +
  `fetch('/log/shot', …)` every 3 s); index from
  `conscripter_index.py -i . --lazyload --args=--enc:sjis`; then
  `web-harness/runff.sh <dir> 60` → `shot.png`.
- Wine: the old prefix from the second session still works
  (`/tmp/claude-1000/<project>/21025cd6-…/scratchpad`: `case_head.sh`
  starts the game at the first ADV page, `g.sh click/move/shot`). The
  first link (「あの場所」) is about page 170: use `nstrace.py --pages`
  to count pages to a target line; quick clicks drop some pages, so
  check with screenshots and count again.
- Links, not done: the underline keeps its colour under the pointer; a
  visited link types in yellow until its sprite (purple) appears.
- Still unchecked against the original: quakes, flashes (ours are under
  the text window; the original's cover it), pulses.

## Done in the first session

| Item | Converter | Engine |
|---|---|---|
| Text hooks from chimimouryou's patch (`onsyuriText`, `onsyuriChoice`, `onsyuriOverlay`) | `66d514e` | `648d563` |
| WebP: web build links libwebp; `--webp` writes near-lossless WebP (images 457 → 218 MB) | `46a5769` | `0b6b995` |
| Movies: opcode 0x97 → `bgmstop` + `mpegplay "movie\mvNNNN.mp4",skip` (ffmpeg H.264/AAC, loose files) | `b54e8d5` | `e217bc1` (web video plays muted when autoplay is blocked; sized like the canvas) |
| Japanese window caption (ASCII filter removed) | `3d19cfd` | none |
| Ruby (0x44) → `(base/reading)` with `rubyon`; text window and backlog lifted by the line gap | `db6efb5` | none |
| Release CONscripter 0.9.0 (tag `v0.9.0`) | `0f6b7e9` | `838338b` |

Earlier in the same session: transitions (`30b223a`), BGM loop points
(`11f3601`), release 0.8.0.

## Done in the second session (user away)

| Item | Converter | Engine |
|---|---|---|
| `0x74` = the timed wait of `0x72` (same handler 0x44a2f0; only the input mask differs), as `delay` | `9b729cd` | none |
| `0x49` colours as `#RRGGBB` (COLORREF, i.e. BGR): chat handles, per-speaker lines, single characters | `71f03fc` | none |
| `0x49` sizes with a new `!zNNN` text tag (sizes are percent; 0x1000+n = n×25 %; length 30000 = to the end of the message) | `8a30efb` | `a35cc39` (no release yet) |
| `0x4e` hanging indent (chat lines that wrap) as `indent N` | `2d1cd8b` | none |
| `0x70` screen filters as the new `screenfilter` (+ `spfilter`/`screenfilter` kind 5 monotone for the sepia palette) | `0a20d5c` | `b306429` (no release yet) |

Unsupported instructions: 4,094 → 3,099. `out/saiima-cons` is rebuilt
with all of it; the sizes need an engine from `a35cc39` or later (older
engines print `z150` etc.). The web build compiles with it.

## 1. Check first

- [x] **v0.9.0 release builds:** all 5 workflows passed and the release
      has 9 assets. CI built libwebp for the web build without trouble.
      Windows x64 first failed with `SDL2/SDL.h: No such file` (its job
      missed the `thirdparty_build` cache, although x86 found it and the
      job `needs: install_thirdparty_build`) and passed on
      `gh run rerun <id> --failed`. If it happens again, have the job
      download `thirdparty_build.tar.gz` itself when the cache misses.
- [ ] Tell the user that `out/saiima-cons` was
      rebuilt with the caption `最果てのイマ COMPLETE`. Their usual
      command uses `--title "Saihate no Ima COMPLETE"`; either works now.

## 2. Unverified (check when convenient)

- [x] Ruby in the backlog: works (screenshot after `　(戦/そよ)ぐ。`).
      The empty log was the harness: `getlog 0` is the current page,
      which is empty right after a `\`. Open the backlog from inside
      `*conv_wait` instead (set a flag on the page, then
      `if flag mov %101,1:gosub *conv_backlog` before `mov %9,0`).
- [x] Ruby next to a link: none of the 37 ruby lines has a link.
- [x] The 4 px gap style: no ruby uses it (they use gap 12 or 11).
- [x] Movies with lazy loading, in the real `conscripter.html` with an
      index from `conscripter_index.py --lazyload` (it lists
      `movie/*.mp4`): they play. The engine opens the file before
      `playVideoWeb`, which fetches it into the FS. Limit: the whole MP4
      (up to 76 MB) downloads before it starts.
- [x] `0x74`: converted (see above).
- [ ] The flag value of 1 vs 3 in ruby records (no visible difference found).
- [ ] Still open from before: the brightness filter scale (assumed
      -255..255), the meaning of the 0x88 "field" value, and whether
      clicking a backlog name plays the voice (only checked headless,
      without a click).

## 3. Next work (from PLAN §6, roughly by payoff)

- [x] `0x4d` (276): how the window shows messages, found by running the
      original under wine with a patched `0002.cd`: 0 typed, 1 whole at
      once (`textspeed 0`), 2 all lines typed together (typed one after
      another here). Unsupported: 3,099 → 2,823.
      Wine setup (scratch prefix, registry keys, script patching) is in
      `saiima.md` under "Running the original".
- [x] `0x70` leftovers (61 of 62): Smooth (blur), HueSatura and
      BrightnessInvert as new `screenfilter` kinds 6-8, Filter-Replace
      (kind 13, wind scenes) as a still `screenripple` in slot 4 + n
      (engine: 8 effect slots). Kind numbers were off by one before:
      the creator table is (type, creator) pairs from 0x501428. Left:
      Zigzag (12, one use, 0010.cd #9214). Unsupported: 296 → 235.
      Checked headless in the wind scene of 0003.cd. The layer byte (0/2 → 0x10, 3 → 0x20, ...) is ignored:
      everything goes behind the text window. 0x88's unknown "field" is
      probably the same layer | fade-ms word as 0x70's.
- [x] CONscripter 0.14.0 with filter kinds 6-8 and 8 effect slots
      (engine `4371a5a`, tag `v0.14.0`; all 5 workflows passed, 9 assets).
- [ ] `0x49` leftovers: the edge colour (one use), and whether enlarged
      text raises the line in the original (we keep the line pitch;
      175 % touches the line above).
- [x] CONscripter 0.10.0 with `!z` and `screenfilter` (engine `c1247f4`,
      converter `127af1b`; all 5 workflows passed, 9 assets). Save files are now version 3 (screen filters).
- [x] `0x55` (252): a second background object "bg-2" over the
      background, mostly `frameNN` (frame01 letterbox bars, frame05
      speed lines, frame07 hearts). It outlasts background changes, so
      the converter keeps it in `%180`/`$180` and shows it again after
      each `csp -1`; link scenes save it per depth. Character sprites
      moved up one (layer n = sprite 61 + n). Unsupported: 2,823 → 2,571.
      Assumption: the record's u32 1000 is a depth (the bust-ups' is
      10-100), putting the frame over the characters.
- [ ] Check: 0x8e's field we read as fade (10 ms units) is passed as
      the object's depth to 0x4109a0 (the same argument as 0x55's
      1000). Maybe it is both, maybe the fades are wrong. **0x70 too:**
      its "fade ms" (timing >> 8, mostly 1000) goes to 0x4109a0 as well,
      and 0x4126c0 makes it the object's sort key (layer bits | depth),
      exactly like 0x8c's. So filters set outside a transition probably
      appear at once, not with our 1 s `print` fade. Check under wine.
- [x] `0x8c` static (kind 2, 88 uses + their clears): an animated
      sprite of generated static (`system/noise.png`, 4 frames of
      800×600, 100 ms each) at sprite 60, kept across `csp -1` in
      `%116`/`$116` (link scenes save it at %130 + depth). Overlay moved
      to sprite 61, characters to 62 + n. Not done: the opacity rise over
      `time` (100-2000 ms; most uses sit inside a transition, which fades
      it in). Unsupported: 2,571 → 2,404.
- [x] `0x8c` kinds 4 (ripple, 25 uses, e.g. 「ぐらり、と視界が歪んだ」)
      and 5 (shaking while a line shows, 12) as CONscripter's new
      `screenripple` / `screenshake` (engine `75a52c0`, no release yet;
      save files are now version 4). Maps are `system/maskNNNN.png`.
      Unsupported: 2,404 → 2,331 (no 0x8c left). Checked headless in the
      Jean scene of 0003.cd: the picture shakes while the text window and
      name plate stay. Not checked against the original under wine.
- [x] CONscripter 0.11.0 with `screenshake`/`screenripple` (engine
      `548b798`, tag `v0.11.0`).
- [x] `0x6d` (198): a quake (handler 0x442000): speed, x and y range;
      0x8c's 7-point path, ranges shrinking after 4 points, waited for.
      Converted to `screenshake 4,…` + `delay` + `screenshake 4`.
      Unsupported: 2,331 → 2,133. Checked headless that the picture
      moves during the delay; not checked against the original.
- [x] `0x6e` (88): a 100 ms additive flash of a colour, as `lsp2add` of
      a colour rectangle + `delay`. Unsupported: 2,133 → 2,045. Checked
      headless (red added to the picture, text window untouched).
- [x] `0x6f` (87): the "surprise" pulse, as CONscripter 0.13.0's
      `screenpulse 3,…` + `delay` + `screenpulse 3`. Unsupported:
      2,045 → 1,958. Checked headless (red and white copies mid-pulse,
      clean after the clear); not against the original.
- [x] `0x93`/`0x94` (418 each): `0x93 N` (handler 0x478830) sets bit
      N of a 600-bit global set (saved as "RMSN", read by the replay list
      0x451090) = scene N unlocked in the scene replay; converted to the
      global `mov %3000+N,1`. `0x94` (0x4789a0) returns to the replay in
      replay modes 1/2 (0x51ec70) and does nothing in play; emitted as
      `; scene end`, the place a replay must stop. Unsupported: 1,958 →
      1,122.
- [x] `0x37`/`0x38` (251 each). `0x37` (0x475780, on `|chpe`) closes
      the chapter 0x36 opened, marks the label read (SkipText.cpp) and
      jumps to `|chpl`, which is always the next instruction: nothing
      emitted (anything else would be reported). `0x38` (0x475970, on
      `|chpl`) clears the chapter name and redraws the chapter bar
      (ChapterBar.cpp): `mov $150,""` (an empty `strsp` is fine). Both do
      nothing in replay modes 1/2. The chapter history itself is probably
      recorded by 0x36 (0x475660); not looked at. Unsupported: 1,122 → 620.
- [x] `0x7a`/`0x8b`/`0x8a`/`0x7b` (91/85/73/89): full-screen animations
      (fx), one object at layer 0, depth 1000. 0x7a creates it (handler
      0x414590; record: group, rect 0,0,800,600, blend 0 = normal, opacity
      256), 0x8b loads `fxNNNN` from `HD/fx.arc` into it (Aom_Animation_Parts:
      `.004` frames + a text `.ap`: name, offset, loop, "default" frame,
      count, then "picture ms flags" per frame, pictures from 1); 0x8a waits
      until the last frame (busy check 0x418230; looping ones never block;
      a click ends it); 0x7b removes it. Converted as an opaque animated
      sprite (`:c/N,<ms,…>,mode`, mode 1 holds the last frame) from a strip
      of the frames in playing order (`fx/fxNNNN.png`, packed; not WebP:
      strips reach 37,600 px), kept across `csp -1` and link scenes like
      the static (`%117`/`$117`, saved at `%13+depth`). Sprite 59: the UI
      slots moved down one instead (name 49/50, link words 51-58, `humanz
      58`), so the game's sprites and old saves keep their numbers.
      Checked headless: the slash (fx0401) covers the scene and the
      character, then holds its black last frame until 0x7b. `arc.nsa`
      grows by 175 MB. Not converted: the layer-2 0x7a/0x7b records (6/7
      uses, no animation loaded). Unsupported: 620 → 296.
- [x] `0x59` (47): a BGM crossfade (u32 ms, u16 old track, u16 new
      track), as `bgmfadein ms` + `bgm`: the old track stops at once.
- [x] `0x82` (37 of 38): an object's opacity; bust-ups and the overlay
      via `msp` (to 0, then up to the alpha). Assumption: a later `lsp`
      of the layer makes it opaque again (the original may keep the
      object's opacity). Left: layer 1 depth 1 (0006.cd #21, after 0x56).
- [x] `0x69` (32): a click wait with the picture alone, passed by skip
      (checked under wine: patched `s2_l17` #73 in 0002.cd), as
      `ClickWait { skippable: true }`.
- [x] `0x86`/`0x87`/`0x7f` (19/9/12): key-frame motions on objects
      (`CModule_Animation_KeyFrameMotion`); the game slides bust-ups
      (start, wait, remove, final position), always inside an armed
      transition. Converted as `Op::SpriteMove` (`*conv_move`: `amsp`
      steps on the timer); the transition is shown first, then the slide
      (the original does both at once). Not converted: the overlay fade
      (0002.cd #14) and a -90° rotation (0043.cd #440). Record layouts in
      PLAN §6. Checked headless (0010.cd: 沙也加 slides from x 420 to 550).
      Unsupported: 119 → 81.
- [x] `0x84`/`0x7d`/`0x7c` (10/5/6) and `0x7a`/`0x7b` on layer 2 (6/8):
      picture, anchor and visibility of a layer 2 object (= a character's
      bust-up slot); the blog scenes show close-ups and 0135.cd's install
      meter this way. Converted as `Op::Sprites` on the character layer;
      the stray layer 0 `0x7b` (0012.cd #916) as a plain fx removal.
      Checked headless (0135.cd meter at 70 %, 0087.cd close-up).
      Unsupported: 81 → 46.
- [ ] Opcodes still emitted as comments (46 left): `0x99` (8), `0x9d`
      (6), `0x39` (4), `0x56` (3), rarer ones.
- [ ] Transitions still drawn as cross-fades: Wave, Dissolve, RollFlip,
      SoftEdgeWipe, TvPowerSwitch, BoxRolling, SmoothCrossFade, Dot4x4,
      MeltDown (PLAN §6 "Accuracy").
- [x] Bars always shown (user's request, 2026-10-13): the top bars and
      the bottom button bar draw at every click wait; the hover hotspots
      (`getmouseover`, `bar_reveal*`/`bar_hide` images) are gone. The
      original hides them until the pointer nears an edge.
- [x] Links like the original (user's request, buggylink.png): the
      word sprite was 12 px low (rubyon lowers strsp text too) and now
      covers the text exactly; colours measured under wine (#FEE092,
      pointer #FEC95E; visited #DCB8FE / #CB95FE); 1 px underline (menu
      sprite slots 6-21, redrawn at each wait); visited = global
      %(2000 + target). Not done: the underline keeps its colour under
      the pointer; a visited link types in yellow before its sprite
      turns it purple. (smoke.py keeps its click queue at %1000+.)
- [ ] Settings options, in the user's order: text window opacity, wheel
      options, confirmation toggles.
- [ ] Extras after clearing (CG/BGM/chapter/scene replay), chapter
      history, blog bar buttons, title BGM.
- [ ] Native Linux/macOS builds skip movies (`mpegplay` is disabled
      without SMPEG). Only if the user wants it.
- [ ] Right-click: not planned (the user said skip it).

## How to build and test

- Converter: `cargo build --release`, then
  `target/release/conscripter-convert convert-game "/home/saya/Z:/Media/InstalledVNs/最果てのイマ COMPLETE" -o out/saiima-cons --font /usr/share/fonts/OTF/ipag.ttf --title "Saihate no Ima COMPLETE"`
  (about 2-3 minutes; add `--webp` for WebP). Movies already in
  `out/<dir>/movie/` are kept; delete them if the ffmpeg settings change.
  `out/saiima-webp` is a scratch `--webp` build and can be deleted.
- Native engine: `make -C engine/build -j16`. Run headless with
  `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy engine/build/conscripter -r .`
  from inside the game dir (else default.ttf isn't found); add
  `--enc:sjis` for scripts without `ons_args`. Screenshots:
  `print 1:getscreenshot 800,600:savescreenshot "shot.bmp"`
  (`savescreenshot` alone saves nothing, and sprite changes need a `print`).
  Headless game runs: see the recipe in `saiima.md` / memory
  (`textbtnwait` → `mov %9,0`, comment out `click`, jump after
  `erasetextwindow 0`). Link pages still block on `btnwait`; jump past them.
- Web engine: emsdk is installed in `~/emsdk`. Build with
  `cd engine/script && EMSDK_HOME=~/emsdk bash ./cross_web.sh`
  (output in `engine/build_web/`, about 1 minute).
- Headless browser test: `research/xuse/local/web-harness/` (git-ignored).
  Copy `conscripter.{js,wasm}` and a `game/` dir next to `index.html`
  (edit its `files` list), then run `runff.sh <dir> <seconds>`; results go
  to `<dir>/log.txt` (stdout/stderr, text hook, video events) and
  `shot.png`. The page defines the globals the engine expects
  (`g_onsyuri_module`, `scale_full`, `fetch_file`, …), forces
  `preserveDrawingBuffer` so the canvas can be read, and uses a fresh
  Firefox profile each run. `AUTOPLAY=0 runff.sh …` allows autoplay;
  the default blocks it like a real browser.
- X11 window checks: `xvfb-run -a` with `SDL_VIDEODRIVER=x11 WAYLAND_DISPLAY=`.
  Without those, SDL opens the window on the user's real Wayland desktop.

## Gotchas hit last session

- Release CI (engine `b1de735`): the web workflow caches emsdk 6.0.12, its
  generated ports and Lua/libwebp. A tag can only read caches saved on
  `master`, so the workflow also runs on `master` when `build_web.yml`,
  `cross_web.sh` or `_fetch.sh` change; after changing any of them, let
  that run finish before tagging. To move to a newer emsdk, change
  `EMSDK_VERSION`. Linux jobs install only their own cross compiler.
  Web 5:21 → 1:11, Linux 6:10 → 0:52; Android (4:05) is now the slowest.

- `pgrep -f <pattern>` inside a wait loop matches the loop's own shell;
  use `pgrep -x conscripter-con` or wait on a background task instead.
- `timeout firefox …` leaves an orphaned Firefox that holds the profile
  lock; `runff.sh` starts it with `setsid` and kills the process group.
  The user's own Firefox is also running; never kill it.
- `engine/src/onsyuri_web/conscripter.html` uses CRLF line endings; edit it
  with a byte-level script.
- `version.h`'s `ONS_JH_VERSION "0.8.0"` is the upstream Jh version; don't
  bump it with `CONSCRIPTER_VERSION`.

## Later: mod files for game systems ONScripter can't run

Idea from the user (2026-10-13): Eushully-style RPG and SRPG systems
and other gameplay that NScripter script can't express.

- Lua is already built into every CONscripter build: `USE_LUA` is in
  `CONSCRIPTER_DEFINES` (`engine/CMakeLists.txt`), `LUAHandler.cpp` is
  compiled, and `script/cross_web.sh` builds Lua with emcc. So
  `luasub`/`luacall` should work everywhere, web included. Not tested yet.
- Split the work: the converter extracts the data (unit and skill tables,
  maps, graphics) as Lua tables or files in the archive. Hand-written Lua
  holds the rules (battles, AI, menus). `0.txt` keeps the story and
  `luacall`s into the system. Lua draws with ordinary sprite and button
  commands, so rendering, input and saves stay in one engine.
- Saves: ONScripter saves only its own variables. Have Lua serialise its
  whole state into one string variable before any save can happen, and
  parse it back after a load.
- First find out where each engine's gameplay logic lives. If it's mostly
  in the game's own script bytecode, port that VM and reimplement its
  native built-ins (more faithful, reusable across titles). If it's in the
  exe, write the rules by hand. xsystem4 (AliceSoft) is the precedent
  for the VM route, as a standalone engine.
- Feasibility: story plus small minigames is easy. A full SRPG is months
  of work per title.
- Steps: (1) smoke test: one `luacall` that draws a sprite and survives
  save and load through the string variable; (2) one session finding where
  an Eushully title's battle logic lives; then decide.
