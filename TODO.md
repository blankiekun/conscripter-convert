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

Work happens on branch `xuse` (pushed; based on `origin/main`). The
Nitroplus agent works in `~/Projects/conscripter-convert-nitroplus` on
branch `nitroplus` and may commit to local `main` without pushing: base
on `origin/main`, not `main`.

Every opcode the story uses is converted (unsupported 81 → 2: the start
point's debug menu, which the user doesn't want). The fourth session
(2026-10-13, later) did the CG gallery (`cf127f3`); the fifth the scene
replay ("chapter") and the BGM room (items under "Extras" below).
**Next: WSS**, the last extra, unless the user says otherwise:

1. **WSS** (`VoiceMode.004`, `HD/wss.bin`): Windows system-sound voices
   per character, with play / stop / save / settings; unlocked by
   progress bit 4 + route (table 0x4fbc00). Probably only "play" makes
   sense here.

It is still an `ExtrasAction::Unavailable` item in
`extras_menu()` (convert.rs); give it its own action, IR and routine
like the gallery (`extras.rs`), the replay (`replay.rs`) and the BGM
room (`music.rs`). Sprite slots 100-499 and %120-%129 are the
extras' (title only, cleared on exit); the replay uses %152-%160 (kept
while a scene plays) and sprites 200-412 for its list; the BGM room's
song playing is %161.

How to see the original's extras (details in `saiima.md`, "Extras"):
run it under wine with the 100% save the user downloaded
(`~/Z:/Media/Visual Novels/farthest2015_muwokoerumono.zip`, from
sagaoz.net) as `Documents\Xuse\最果てのイマ COMPLETE\Farthest2015_000.bin`
**inside a prefix whose Documents is a real folder** (the old prefix's
Documents was a symlink to the user's `~/Documents`; see saiima.md).

Other approximations worth checking or improving, roughly by
visibility: the Zigzag's Multi blend (needs an engine blend option for
`screenripple`), MeltDown (3 uses, still a cross-fade), quakes, flashes
and pulses never compared with the original. Wine harness: audio is
muted; scripts that FIRST START runs are in 0004.cd (0003.cd is the
epilogue).

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
- [x] `0x99` (8): progress bits the extras read (title, voice mode),
      as global flags `%3600 + bit`. Unsupported: 46 → 38.
- [x] `0x9d` (6): a CG counts as seen in the CG gallery, as global flags
      `%2200 + n`. A gallery will also need 0x54's own seen marks (its
      record's byte; not tracked yet). Unsupported: 38 → 32.
- [x] `0x56`/`0x83`/`0x67` (3/1/2): colour fills of the background or
      the overlay (generated `system/fill_rrggbb.png`); the impact routine
      `s2_l2` (9 calls) now ends in white fading out over 1 s (the
      original dissolves it); 0006.cd's red "Server Error" screen. Checked
      headless. Unsupported: 32 → 24.
- [x] `0x39` (4): clears the chapter history; nothing emitted until the
      chapter history screen exists (then clear it here). 24 → 20.
- [x] `0x3b` (2): the boot script's entry name ("debug entry" /
      "release entry"), read only by the unused 0x3c; nothing emitted.
- [x] `0x91` (2): the staff roll, rendered by the converter
      (`xuse/staffroll.rs`, tables from the exe) into
      `movie/staffroll{0,1}.mp4` with its song (BGM 23 / 22) and played
      with `mpegplay …,0` (the original can't be skipped). Roll 1 (223 s)
      opens the epilogue, after the end of 0008.cd; roll 0 (63 s) plays
      from `*s0_l8` (route ends, when flags %210-%213 allow). Compared with the original under wine: frames
      match (mean pixel difference 2-5, 10 while the panel widens).
      Web build (0.15.0, headless Firefox harness): plays, 223 s, time
      advancing in real time (muted only because the harness blocks
      autoplay sound).
- [x] `0x48` (2): return to the root scene. Routes 10 and 11 are entered
      from links and start with it; without it each entry left a
      `pushscreen` entry (a full stack of 32 exits the engine), the link
      depth and `%181` raised. Now the link pages unwind (`%118` counts open
      link scenes, `%119` says where to go on; `*conv_unwind`). Checked
      headless: 『たまに早く出る』 → route 10 at depth 0.
- [x] `0x4a` (2): the style of the messages that follow (0x49's value
      encoding): 0011.cd's 「こ」「わ」「せ」「た」 in orange (#ff8c00) at
      125 %, then the defaults. The edge colour isn't converted.
- [x] `0x36` without a chapter name (2): `chapterさやか]１?ビジョン` has a
      part number before the `?`; the title is the text after it.
- [x] `0x17`-`0x1b` (1 + 1 use): the other comparisons of 0x16's `if`
      (!=, <, >, <=, >=; handlers by their `set*`), as `Op::If`.
- [x] `0x96` (1): back to the title at the end of the epilogue, as
      `reset`; `0x00` (1): a one-frame yield like 0x67.
- [x] `0x8d` (1): waits for an environment slot's module (0007.cd: a 3 s
      ripple), as a `delay` of the module's time.
- [x] `0x80` (1): an object's scale (percent); 0013.cd's character 12
      at 80 %, as a pre-scaled copy (`sprite/<name>_s80x80.png`) centred
      on the same point, its 0x82 opacity put back.
- [x] The 0x86 rotation (1): 0043.cd's 章二 falls over (-90°) while
      sliding off screen, as `Op::SpriteTurn` (an `lsp2` copy stepped with
      `amsp2`, then removed). Direction checked under wine: Xuse turns
      clockwise for positive angles (0x81 +45° on a bust-up); ONScripter's
      `rot` turns counter-clockwise, hence `rot` = -angle.
- [x] Zigzag (0x70 kind 12, 1 use, 0010.cd #9214): the kernel (0x43f0b0)
      shifts each pixel by distance × sin(2π·d·freq/100/1024) along the
      direction (d = distance from the centre); converted as a still
      `screenripple` over a generated ring map (`system/zigzag_*.png`).
      With freq 10 that is a shift of a few pixels. Its Multi blend
      (100/256) darkens the picture: `screenripple`'s new optional blend 1
      (CONscripter 0.15.0) multiplies by the shifted copy; saves keep
      their format (the parameter count is stored). The phase (begin
      angle) and reduction are left out.
- [ ] Opcodes still emitted as comments (2 left): the start point's debug menu (`0x62` choice + `0x30` jump table
      in 0001.cd, unreachable: nothing jumps to `s1_l0`). **Not doing the
      debug menu** (user, 2026-10-13).
- [x] Dissolve (69 uses) and Dot4x4 (3): filmed in the original under
      wine (0004.cd #13 patched to each sub, 8 s; shots at 2/4/6 s).
      Dissolve passes through white (the old picture brightens to white,
      the new one comes out of it): two half-time fades with a white
      `lsp2` in between. Dot4x4 is the dotted cross-fade (kernel 0x4430a0,
      a 4×4 pattern): effect 15 over a tiled Bayer mask. (The first
      Dissolve commit wrongly gave it the dither.)
- [x] Wave (39 uses): Trans_Wave (setup 0x449020, draw functions from
      0x51801c) is a curtain: the new picture behind an edge whose last
      16 px bands open in shrinking strips. Converted to ONScripter's
      curtains: sub 0 top down → 8, 1 right to left → 7, 2 bottom up → 9,
      3 left to right → 6. Not compared frame by frame with the original.
- [x] SoftEdgeWipe (10 uses, subs 5-8): directions seen under wine (0004.cd
      #13 patched to each sub, 8 s): 5 up from the bottom, 6 from the
      left, 7 down from the top, 8 from the right; soft edge about 64 px.
      Converted as a hard wipe (effect 15) through a linear gradient
      (`system/wipe_*.png`): ONScripter's soft mask fade (18) can't keep a
      narrow edge across the screen.
- [x] RollFlip (24): a picture turning edge-on about the centre (size
      sin/cos of progress × 90°, 0x446460; flag +0x90 the axis, +0x91
      whether the new one grows); filmed 32 (new grows horizontally) and
      34 (old shrinks horizontally). New engine plugin `cons_flip.dll`
      (CONscripter 0.15.0; older engines draw a cross-fade). Checked on Xvfb mid-effect (band widths match sin/cos).
- [ ] Also filmed: BoxRolling 28 = the new picture rises from the bottom
      while the old is squeezed up (a cube roll); RollFlip 32 = the new
      picture widens from a centre strip over the old; MeltDown = the old
      picture melts down with a wavy edge; SmoothCrossFade = an eased
      cross-fade (the plain fade is close). TvPowerSwitch had finished by
      2 s (unknown). Used subs: BoxRolling 29-31, RollFlip 32-35,
      MeltDown 46/47, TvPowerSwitch 48.
- [x] BoxRolling (8 uses): filmed 28 (from the bottom), 29 (from the
      left), 30 (from the top); 31 assumed from the right. Converted as a
      push of both pictures (ONScripter scrolls 11-14); the original
      squeezes them like cube faces.
- [x] TvPowerSwitch (11): runs its own 700 ms key frames (setup 0x4485d7)
      whatever the record says; a burst capture under wine showed the
      picture spreading from a bright line across the middle. Most uses
      cut to black (a television switching off): converted as a vertical
      `cons_flip` collapse of the old picture over 700 ms.
- [x] CONscripter 0.15.0 (engine `9f7ef7e`, tag `v0.15.0`, Android
      versionCode 18): `cons_flip.dll`, `screenripple` blend. All 5
      workflows passed, 9 assets.
- [ ] Transitions still drawn as cross-fades: MeltDown (3); SmoothCrossFade
      (8) is an eased cross-fade, close enough.
      SmoothCrossFade (8), Dot4x4 (3), MeltDown (3) (PLAN §6 "Accuracy").
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
- [x] Text window opacity (user's first settings item): a sixth level row
      ウィンドウ透過 (%910, 0 = the window as drawn, so older saves keep it),
      applied with CONscripter 0.16.0's new `windowtrans` (255 - level ×
      25). Rows now draw label and level as one text sprite (labels at x
      150, rows 60 px apart) to make room. The original's options, seen
      under wine: テキスト (window opacity slider), マウス (wheel advances
      text, wheel closes the backlog, wheel sensitivity), 確認メッセージ
      (save, load, delete, autosave load, chapter history overwrite,
      settings OK/cancel, unread skip warning, title, quit; all on).
- [x] Wheel options and confirmation toggles (user's 2nd and 3rd items):
      a third settings page マウス・確認 with the original's labels:
      wheel reads on, wheel closes the backlog, save / load / delete /
      title / quit confirmations. Global off flags %3640-%3646 (0 = on,
      the original's defaults; older saves get them). The dialog routine
      answers yes at once for a switched-off kind (%125). Not converted:
      wheel sensitivity, the autosave / chapter history / settings / unread
      skip confirmations (no such dialogs here). Checked headless.
- [x] CG gallery (filmed under wine with a 100% save from sagaoz.net):
      the title's option opens the extras menu (`SubModeCommon.004` #19;
      cg / bgm / chapter / wss / exit, as in the original even before
      clearing; cg enabled once a gallery CG is seen). The original has
      no settings on the title, so the menu's "option" heading opens ours.
      Gallery table from the exe (`xuse/gallery.rs`: 5 tabs, 113 slots,
      thumbnails from `SubModeThumb-TT-P.png`), 5 x 4 grid, locked
      tiles, pointer frame, page numbers; viewer with previous / next
      (skips unseen variants), clear (a click brings the panel back),
      back. 0x54 now marks gallery CGs seen (and 0x55 with its mark
      byte), as `%2200 + number`. `crates/conv-cons/src/extras.rs`.
      Checked headless: layout matches the original's page (mean pixel
      difference 0.2), variants, locked slots, pages, exits. Not done:
      the panel's slide-away when idle (ours stays up), the voice button
      (`cgv.bin`) and a check box of unknown use.
- [x] Scene replay ("chapter" in the extras; filmed under wine with
      the 100% save): a menu of 11 lists (プロローグ … BLOG-3), each a
      window of up to 3 × 23 titles over the darkened menu
      (`SceneList.004`; the band subtracts 80 per channel, pre-rendered
      as `system/replay_back.png`). Only unlocked scenes are listed,
      closing up the gaps; "new" until replayed (`%3650 + scene`, set
      when the replay starts); H scenes in pink. Table from the exe
      (`xuse/replay.rs`, 0x4fe200: 418 entries of scene bit, H flag,
      `rm…` label in 0014-0018.cd). A title clears the playthrough's
      variables, sets `%152` and jumps to the `rm…` label (which calls
      the replay set-up, 0019.cd, and jumps into the story); `0x94` now
      emits `if %152 == 1 goto *conv_replay_end`, which stops the sound
      and shows the list again. During a replay the bar has no save or
      load and the settings screen's "タイトルに戻る" becomes "シーン選択に
      戻る" (asks "シーン選択に戻りますか？", as the original's right-click
      menu does). `crates/conv-cons/src/replay.rs`. Checked headless: list
      layout against the original, a scene played to its end, leaving
      from the settings, "new" cleared. Not done: the selected row's
      pressed state, the window's slight blur.
- [x] BGM room (filmed under wine): 22 song titles (`BgmMode.004`, a
      2 × 4 grid of cells: normal / pointer / pressed, right column while
      playing) at (120, 71 + 20 i) under the "bgm" heading; table from
      the exe (`xuse/music.rs`: 0x5001d0 places the songs, 0x500598 maps
      tracks to songs; the last song, track 22, only once progress bit 0
      is set). A click plays the song looping (clicking it again changes
      nothing); it goes on after leaving the room and stays marked
      (`%161`). `crates/conv-cons/src/music.rs`. Checked headless: page
      matches the original (mean pixel difference 0.25), playing mark,
      mark kept after leaving. Sound not heard (headless runs are muted).
- [ ] Extras: WSS (system voices); chapter history, blog bar buttons,
      title BGM.
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
