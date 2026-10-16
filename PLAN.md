# CONscripter-convert — Plan

Converts Japanese visual novels (that you own) into games playable on
**[CONscripter](https://github.com/blankiekun/CONscripter)**, our fork of
ONScripter-Yuri, and helps reverse-engineer engines it doesn't support yet —
optionally with AI agents. The converter and the engine are developed
together: when a game feature can't be reproduced with the engine's existing
commands, the feature is added to CONscripter (§4.1).

- **Games:** each converted game has its own document in `docs/games/`
  (formats, opcodes, progression variables, status, leftovers); the list
  is in §6. How to start a new game: `docs/new-game.md`. This file holds
  what is shared by every game: architecture, IR, output stage, engine
  additions and lessons.

- **Language:** Rust (cargo workspace) for the converter; C++ for engine
  features, which go into CONscripter (the `engine/` submodule).
- **Input:** an installed game directory.
- **Output:** `0.txt` / `nscript.dat` + `arc.nsa`/`.sar` archives + converted media,
  + `ons_args`, runnable by CONscripter, plus a conversion report.

## 1. Scope and expectations

- Games that are mostly text, sprites, backgrounds, music, voices and choices
  convert well.
- Custom systems (minigames, battle systems, heavily scripted menus written in
  the engine's own scripting language) do **not** convert automatically. They
  are reported clearly, never silently broken.
- The deterministic pipeline is the source of truth. AI agents only **propose**
  artifacts (profiles, opcode tables, mappings) that the pipeline then
  validates. Agents **only run when explicitly invoked** (`conscripter-convert ai …`);
  `convert`, `lint` etc. never call the API. The tool is fully usable offline.
  (The built-in agent layer is not built, §8.)

## 2. Architecture

```
            ┌──────────┐   ┌──────────┐   ┌────────────┐   ┌──────────┐   ┌────────────┐
 game dir → │ Detect   │ → │ Unpack    │ → │ Decompile  │ → │ IR       │ → │ ONS output │ → 0.txt + arc.nsa
            │ engine   │   │ archives  │   │ scripts    │   │ (common) │   │ + packing  │
            └──────────┘   └──────────┘   └────────────┘   └──────────┘   └────────────┘
                                 │                               ▲
                                 └─→ Asset decode/transcode ─────┘
                 ▲                        ▲                ▲                ▲
                 └──────── AI agent layer (optional): proposes profiles, opcode
                           tables, macro mappings, fixes; validated by the pipeline
```

Each source engine is a Rust trait implementation, compiled in (no dynamic
plugins):

```rust
pub trait Engine {
    fn id(&self) -> &'static str;
    fn detect(&self, dir: &Path) -> Confidence;
    fn open_vfs(&self, dir: &Path, profile: &Profile) -> Result<Box<dyn Vfs>>;
    fn decode_asset(&self, entry: &VfsEntry, data: &[u8]) -> Result<Asset>;   // Image/Audio/Video/Raw
    fn decompile(&self, vfs: &dyn Vfs, profile: &Profile) -> Result<ir::Program>;
}
```

Only the output stage (`conv-cons`) knows about NScripter and CONscripter,
so another output engine could be added later.

## 3. The IR (core design decision)

A small, engine-neutral list of instructions, grouped into labelled blocks,
and serializable (serde → JSON) so tools and agents can read and diff it.

- **Text:** `Say { speaker, text, voice }`, `ClearText`, `TextboxShow/Hide`, `WaitClick`, `Wait(ms)`
- **Graphics:** `Bg { image, transition }`, `SpriteShow { slot, image, x, y, transition }`,
  `SpriteHide { slot }`, `Cg { image }`, `ScreenEffect { kind, params }`
- **Audio:** `BgmPlay/Stop { fade }`, `SePlay { channel, file }`, `Voice { file }`
- **Control flow:** `Label`, `Jump`, `Call`/`Return`, `Choice(Vec<Option{ text, target, cond }>)`, `If { cond, then, else }`
- **Variables:** `Set { var, expr }`. Variables are named and marked `Local`/`Global`.
- **Escape hatches:** `Unsupported { engine, raw, loc }`, `Comment`

Rules:
- Every instruction carries a `SourceLoc` (file and line or byte offset).
- Sprite layer composition (body + face + accessories) happens *before* the
  IR; the IR refers to the merged image.
- The output stage maps named variables to NScripter numbers (`%n`, `$n`),
  putting globals in the `globalon` range.

## 4. Output stage (IR → NScripter)

| IR | NScripter output |
|---|---|
| `Label` / `Jump` / `Call` | `*label`, `goto`, `gosub` / `return` |
| `Say` | `dwave 0,"voice"` + text line with name prefix and `\` / `@` click waits |
| `Bg` | `bg "file",effect#` |
| `SpriteShow` | `lsp n,":a;file",x,y` + `print effect#`. Not `ld`, which only has 3 fixed positions. |
| `Choice` | `select` / `csel` |
| `BgmPlay` / `SePlay` | `bgm`, `bgmstop`, `dwave n` |
| transitions | `effect` definitions; mask wipes converted to mask images |
| `If` / `Set` | `if %n == ...`, `mov` / `add` |
| `Unsupported` | `;` comment + entry in the conversion report |

The output stage also writes the `*define` section (`effect` defs, window and
font setup, menu, `game`). Text is encoded to Shift-JIS with `encoding_rs`.

### 4.1 Target engine: CONscripter

The target is [CONscripter](https://github.com/blankiekun/CONscripter), a
fork of [ONScripter-Yuri](https://github.com/YuriSizuku/OnscripterYuri)
(itself based on ONScripter-Jh). It is checked out as the `engine/`
submodule. ONScripter-Yuri was chosen because it builds for the web
(WebAssembly), Windows, Linux, macOS and Android from one tree, and adds
UTF-8 scripts, Lua and arbitrary resolutions to the NScripter command set.

**Rule for engine features.** Convert with existing NScripter/ONScripter
commands first. When a feature of the original game can't be reproduced
that way (or only with a workaround that visibly differs from the
original), add the feature to CONscripter instead of approximating it:

1. Implement it in `engine/src/cons/` as a new command or option, in the
   style of the surrounding ONScripter code, so it works on every platform
   the fork builds for (the web build especially).
2. Don't make it opt-in (the user, 2026-10-14): a new behaviour applies
   to every script, with no command or flag to turn it on, as bug fixes
   do; a new command takes effect where a script uses it. Check that it
   doesn't change the other games' builds for the worse.
3. List it here in §4.2, with the game feature that needed it.
4. Commit and push in the engine repo, then commit the updated submodule
   pointer here together with the converter change that uses it.

**Startup options.** CONscripter reads command-line options from an
`ons_args` file in the game directory (whitespace-separated, up to 16).
The converter writes one with `--enc:sjis`, because ONScripter-Yuri reads
scripts as GBK by default. This is how every converted game passes
options to the engine, including in the web build.

**Building** (Arch Linux; `sdl2_mixer` from the package or built locally):

```sh
git submodule update --init engine
cmake -B engine/build -S engine -DCMAKE_BUILD_TYPE=RelWithDebInfo
make -C engine/build -j"$(nproc)"     # → engine/build/conscripter
```

`engine/script/cross_web.sh` builds the WebAssembly version (needs emsdk).

### 4.2 CONscripter additions

Engine features added for converted games (details in the engine's
`COMMANDS.md`):

| Command | Game feature that needed it |
|---|---|
| `spfilter` (colour filters on sprites, kept in saves) | 最果てのイマ's effect presets (0x88/0x90): lighting tints on bust-ups, plus negative, greyscale and brightness filters. Replaced ~930 pre-tinted PNGs (about 130 MB of `arc.nsa`) |
| `pushscreen` / `popscreen` (save and restore background, sprites, filters and BGM; kept in saves) | Link scenes, which can change the whole screen before returning to their page. Replaced the converter's file-order guess of what was on screen |
| `delsavegame` (delete a save and its thumbnail) | The save menu's delete button. Replaced a global flag per slot that only hid the save; a slot an older build flagged is deleted when the menu lists it |
| `copysavegame from,to` (copy a save file, or delete `to` when `from` is missing) | 最果てのイマ's link scenes: a save from the menu keeps the levels' slots (900 + level) beside it and a load puts them back, so the levels above can still be gone back to, as the original keeps its scene stack in a save |
| `cons_slide.dll`, `cons_zoom.dll` (effect 99 plugins: slides in 8 directions, four zooms) | 0x76 transitions: Slide in/out (subs 10-25, 7 uses) and Zoom (37-40, 116 uses), decoded from the exe (`transition_style` in the Xuse converter). Scroll (41-44) maps to ONScripter's effects 11-14 and Mosaic (27) to 16 then 17, without engine work |
| `cons_flip.dll` (effect 99 plugin: a picture turning edge-on, either axis, growing or shrinking) | 0x76 RollFlip (subs 32-35, 24 uses): seen in the original under wine (the new picture widens from a centre line, or the old one narrows into it), sizes sin/cos of progress × 90° from the exe (0x446460); also TvPowerSwitch (sub 48, 11 uses) as a 700 ms vertical collapse. CONscripter 0.15.0 |
| `cons_melt.dll` (effect 99 plugin: the old picture melts down off the screen behind a smooth random edge, falling columns stretched) | 0x76 MeltDown (subs 45-47, 3 uses): filmed under wine (sub 45), the edge spline and stretch from the exe (0x445310, 0x4458e0). CONscripter 0.17.0 |
| `cons_wave.dll`, `cons_blur.dll` (effect 99 plugins: a cross-fade with the rows waving, or blurred sideways, most at halfway) | 俺たちに翼はない's InOut_Act 1 (57 uses) and 6 (130 uses), read from the eboot's fragment shaders (Vita3K's recompiler): the blur exactly, the wave's amplitude and length guessed (their constants are uniforms) |
| `windowtrans opacity` | The settings' ウィンドウ透過 row (the original's ウインドウの不透明度 slider): the text window picture drawn at 0-255 over its own alpha. CONscripter 0.16.0 |
| `openurl "url"` | 装甲悪鬼村正's ライナー画像 title button (`System("OPEN:...")`): opens the staff-comment page in the browser, out of full screen first. CONscripter after 0.22.0 |
| WebP in the web build (libwebp linked in; emscripten's SDL_image has no WebP) | Archive size on the web: `--webp` stores images as near-lossless WebP, 218 MB instead of 457 MB of PNG (`arc.nsa` 765 MB instead of 992 MB). Opt-in, because builds whose SDL_image lacks libwebp can't read it |
| `screenfilter` (the `spfilter` filters on the picture behind the text window, 4 slots, kept in saves and by `pushscreen`) and `spfilter`/`screenfilter` kind 5 (monotone: greyscale plus r, g, b offsets) | 0x70 screen filters, 395 uses: darkening (brightness down to -250), greyscale and sepia (monotone palette `pal0001.pal`, grey + (28, -9, -38)) flashbacks, negatives, colour overlays, each faded in by `print` |
| `spfilter`/`screenfilter` kinds 6 (blur: box mean over x, y ranges), 7 (hue and saturation) and 8 (lightness inverted) | 0x70's Smooth (40 uses, mostly a 3 px blur for flashbacks and wind), HueSatura (4, desaturating) and BrightnessInvert (1) |
| `screenshake` and `screenripple` (effects that move by themselves on the picture behind the text window, 8 shared slots, stepped like sprite animations, kept in saves; a shake given an `end` stops by itself, for NSS's timed `Shake`) | 0x8c's Animation-Vibrate (12 uses: shaking while a line is shown) and Animation-Replace (25: a ripple through `mask1001.png` and others, 「視界が歪んだ」). The parameters are the original modules'. 0x70's Filter-Replace (16 uses, wind and heat haze) is a still `screenripple` in slots 5-8 |
| `screenpulse` (a tinted copy of the picture that swells over it and fades, sin(πt); same slots as `screenshake`) | 0x6f "surprise" pulses (87 uses): 10-20 % zoom, opacity 100-200, untinted, light red or white |
| `spfade num,ms[,from]` (a sprite fades in by itself while the script goes on; `spfade num,-ms` fades it out and removes it; at once under `--auto-play`) | 0x8c's static (Animation-RandomNoise), whose opacity rises from 0 over up to 2 s (37 of 88 uses take 1-2 s); the unpinned button bar's 100 ms fade-in; 俺たちに翼はない's `:FadeWait 0` (characters fading in and out while the text goes on) |
| `spz num,over` (a sprite drawn right over another instead of at its own number; kept in saves and by `pushscreen`) | 最果てのイマ's bust-ups at depth 1100-5000 (62 entries), which the original draws over the frame overlay (depth 1000) but under the static and the animations |
| `screenflash opacity,colour` (a colour added over everything on the screen, the text window included) | 0x6e flashes (88 uses), whose object sorts over everything (key `0xffffffff`); an `lsp2add` stays under the text window |
| `!zNNN` text size tag (NNN percent of the window's font size, on the baseline, scaled width) | 0x49 text styles: shouting at 125-175 % and mumbling at 75-95 % in about 380 messages (NScripter text has colour tags but no size). Colours from the same records use `#RRGGBB` |
| `!i1` … `!i0` italic text and `!c` (the rest of the line centred), also read in `` ` `` text | Full Metal Daemon Muramasa's `<I>` (about 2,600) and `<center>` (about 570 rows); NScripter has no italic or centring tag |
| Text hooks in the web build (`onsyuriText`, `onsyuriChoice`, `onsyuriOverlay`) | Not for a converted game: chimimouryou lays each page's text over the game for Yomitan and reading statistics. Was a patch kept in chimimouryou (`scripts/onsyuri/text-hook.patch`) |
| `dwavefadein ms` / `dwavefadeout ch,ms`, now `chvolfade ch,0,ms,1` (CONscripter PR #24; `chvolfade -1,...` is the music, was `bgmvolfade`) (a `dwave` channel's sound fades in, or fades out and stops; SDL_mixer's channel fades) | 最果てのイマ's sound effects: 0x5a fades 41 sounds in (50 ms-3 s, mostly looping ambience) and 0x5b fades 178 out over more than 300 ms; found in the opcode audit |
| `automode_time ms,char_ms` (auto mode waits a time per page plus a time per character) | 最果てのイマ's settings: the auto wait 文章単位 (0-5 s) and 文字単位 (0-100 ms a character) |
| `spzoom` (an `lsp` sprite zoomed about its centre in its place among the sprites, where `lsp2` sprites all lie over them; kept in saves and by `pushscreen`) | 俺たちに翼はない's camera zooms (SpriteMove `:Scale`, 90 uses): impacts zooming in and out on CGs and backgrounds, pans while zoomed |
| `spmotion num,loop,t,x,y,zx,zy,angle,alpha,curve,...` (an `lsp` sprite moving by itself along keys from its place: offset, zoom x/y, angle, alpha, each key reached by a curve: straight, ease in/out/both, a jump, or an `spcurve` table; loop 0 back at its place, 1 for ever, 2 once then kept at the last key, -n n times then kept; stepped by the timer during text and click waits; kept in saves). Replaced `spmotion`'s offsets-only keys, `spmotion2` and `spalpha` (CONscripter PR #27) | 俺たちに翼はない's CharacterAction (2,312 uses: bows, shakes, 72 bobbing loops; `:Scale`, `:Roll`, `:Alpha`, `:EndSet`) and SpriteQuake; NSS `Move`, `Zoom` and `MoveCamera` over time (続・殺戮のジャンゴ 1,820, 装甲悪鬼村正 about 1,900), which go on during cross-fades; 装甲悪鬼村正's title fire glow (`CreateTPO1`) |
| `spcurve id,v0,...,vn` (an easing table for `spmotion` keys, 0-1024 at even time steps; kept in saves) | Easing read from an exe as data, in place of a script loop per eased value (`conv-cons/src/ease.rs`, `zoom.rs`) |
| `spclip num,x,y,w,h` (an `lsp` sprite drawn only inside a screen rectangle wherever it moves; kept through `lsp`, in saves and by `pushscreen`) | 装甲悪鬼村正's `CreateWindow` children: the cockpit's speed and altitude tapes, its roll bar, the split-screen panels (about 4,000 `spclip` lines in the story) |
| `sprot num,degrees` (an `lsp` sprite turned about its centre in its place among the sprites, kept like `spzoom`; a loop-2 `spmotion` leaves its last angle there) | 装甲悪鬼村正's battle HUD: `Rotate` about z (the roll bar `CPCenter`, the tilting sky; about 600 uses), still or over time |
| `spbtn num,no,pressed_cell` (the cell a sprite button shows while the mouse button is held on it) | 装甲悪鬼村正's buttons have a pressed picture (`003`, `MouseClick`) for every button; first used by its yes/no dialogs |
| `selectvoice` outside the define section (an empty name is no sound) | 装甲悪鬼村正's title plays a sound when the pointer comes onto a button, and only there |
| `getkey` (letter keys end the next `btnwait`/`textbtnwait`, -1000 minus the letter's code) | 装甲悪鬼村正's shortcuts (`SetShortcut`): M opens the right-click menu, and H, A, N, R, B, S, L, Q, P, F, C, T run its items at any click wait |
| `vstrsp` (`strsp` with its text in columns from the right, kept in saves and by `pushscreen`) | 装甲悪鬼村正's save screen: the chapter title and the page's text down in columns in its side panel |
| `strspoutline` (`strsp` text's shadow drawn as a 1 px outline of a colour, kept in saves) | 続・殺戮のジャンゴ's text log: black letters with an #AAAA88 rim |
| `getplaytime` / `setplaytime` (the play time counted by the engine, kept in each save as it stands when written, put back by a load) | 装甲悪鬼村正's save screen shows each save's play time (`$PLAY_TIME`) |
| `logrows` / `getlogrow` (the text history as rows: each page's lines cut at a column count or a pixel width, a proportional font's letters at their own width, ruby as its base text, the page's voice on its first row) | 装甲悪鬼村正's backlog, scrolled a row at a time with a speaker icon per voiced page |
| `spovertext num[,on]` (a sprite drawn over the text window and its text, under windowback/humanz's sprites; kept as spz's place: lsp keeps it, csp removes it, saves and pushscreen keep it) | Full Metal Daemon Muramasa's English subtitles over its text boxes |
| `consset "key",value` / `consget` with the keys `mutebgm`, `mutese`, `mutevoice`, `clickvoicestop`, `autovoicewait`, `selectkeep`, `moviestop`, `gamespeed` (were one command each; CONscripter PR #5) (player settings: mute music/SE/voice, a click stops the voice, auto mode waits for the voice, auto and skip go on after a choice, a click stops movies, script times sped up or slowed) | 装甲悪鬼村正's settings screen (its 入/切 pairs and ゲーム全般の処理速度, the exe's speed table) |
| Movies as pictures (`lsp n,":c;movie\a.mp4",x,y`, also `bg`, `ld`, `lsp2`: the movie plays in the sprite's place, under the text window and whatever sprites are over it; `":c/1,0,0;"` loops), `spmoviewait n[,click]`; `mpegplay` draws the text window over the movie when it is shown; a skipped movie stops the `dwave` sounds started with it | 装甲悪鬼村正's movies: the cuts under the text (`CreateMovie` not waited for, drawn as stills until now), a movie over which the text window stays, the title openings' clash sound that played on after a skip. CONscripter after 0.24.0 |
| `mpegplay "file",click,1` (the music goes on through the movie) | 装甲悪鬼村正's battle cuts (`CreateMovie`, `MovieSEStart`): a movie over the scene while its music plays on |
| Movies on Linux and macOS (`mpegplay`, `movie` in the game window through the player's ffmpeg; they were disabled there) | Every game's movies (最果てのイマ's openings and staff rolls, 翼's) on the native Linux and macOS builds, which played none |
| Opus sounds in every build (the engine decodes Ogg Opus with opusfile, compiled in by `cmake/cons_opus.cmake`, and gives SDL_mixer a WAV; no build's SDL_mixer reads Opus) | `--audio-codec opus`: voices and sound effects about half the size (最果てのイマ's 542 MB become 217 MB; 翼's HCA voices 1.7 MB a scene instead of 2.6 MB as Vorbis) |
| `consset "sysevol"` (was `sysevol`) (the volume of `wave`'s channel, where the menu sounds play) and `btnse` (a sound when a `btnwait` or `textbtnwait` ends on a button) | 俺たちに翼はない's SystemSE.AWB in every menu: the cursor's tick (`selectvoice`'s over sound) and the confirm, at システムボリューム (IR `Script::menu_sounds`, `Setting::SystemVolume`) |
| `bgmlater ms,"file"[,loop]` (the music starts ms from now while the script goes on; a music command before then cancels it; kept in a save) | 続・殺戮のジャンゴ's music processes (`CreateProcess`): the 400-vs-3 battle's and the last chapter's themes start their loop a set time after the intro, the insert song five seconds after a line, a two-part jingle (IR `Op::BgmLater`). CONscripter after 0.23.0 |
| `mpeghold "file"[,sp[,ch,"sound",fade_ms]]` (a movie that plays only while the button, a touch, Enter or Space is held down, stopping on its frame when let go; sprite sp drawn over it with its cells going round; a sound paused with it, or restarted looped at each press and faded out when let go; the release after a movie that ended while held is no click) | 続・殺戮のジャンゴ's 400-vs-3 battle (`VS400`): the player holds the trigger down through six movies, the "hold" prompt over them (IR `Op::HoldMovie`). Linux/macOS (ffmpeg) and the web build. CONscripter after 0.23.0 |
| `spblend num,mode` (an `lsp` sprite drawn added onto (1) or multiplied into (2) what is under it, times its alpha, keeping its place among the sprites; kept in saves) and `spmotion`'s loop -n (the keys gone through n times) | 続・殺戮のジャンゴ's muzzle flashes (`GunFire`): a white screen at priority 150 added on, flashing up to 100 times while the text goes on (IR `Op::SpriteFlashing`). CONscripter after 0.25.1. Multiply: 装甲悪鬼村正's MulRender (418 calls; AddRender 2,536) |
| `autooff` (leave auto mode); `btnwait` ends auto mode as it ends skip | 続・殺戮のジャンゴ's choices and ending roll, where the script sets `$SYSTEM_text_auto` false or `$SYSTEM_menu_lock` (IR `Op::StopAutoSkip`, also Muramasa's). CONscripter after 0.24.0 |
| `spcut num,"mask",x,y` (cuts a hole in a sprite's picture, every cell, where the mask is dark) | 続・殺戮のジャンゴ's choices: the bullet hole where the player clicked (IR `ChoiceShot`). CONscripter after 0.24.0 |
| `getbgmpos %v` (the ms the music playing now is through its current pass; 0 with none) | 続・殺戮のジャンゴ's ending credits (`ProcessRoll`): the roll goes over the insert song's time left, the closing song after it (`bgmlater`) (IR `Op::MusicRoll`). CONscripter after 0.26.0 |
| `bitand` / `bitor` / `bitxor %t,%a,%b` (bitwise AND, OR, XOR of two values into `%t`) | 最果てのイマ's three bitwise ops (`Op::Arith` `BitAnd`/`BitOr`). Replaced the `conv_bitand`/`conv_bitor` subroutines, which looped over the bits with div/mod in scratch variables %1-%8. |
| Proportional text (in a proportional font, half-width characters on a horizontal line take their own width plus half the character spacing, one at a time, and lines break at spaces; a monospaced font keeps the grid) and a `!` that starts no tag drawn as text | Full Metal Daemon Muramasa's English text in the release's Droid Serif, one paragraph a line for the engine to wrap (`"Hey! You!"` lost its `!`). CONscripter after 0.23.0 |
| Text past the window goes on on a new page (a character that would be drawn past the window's last line waits for a click, built in, not through `textgosub`, then starts a new page) | Full Metal Daemon Muramasa (English): the Japanese pages hold several paragraphs, which in English outgrow the port's 280×150 box (1,143 pages ran past it) |
| `cons_rule.dll/file,cells` (effect 99: a transition played from a strip of grey masks, white = new picture, mixed between cells) and the trace's `--shot-effects` (each effect filmed at chosen percents of its time, also under `--auto-play`) | Any engine's wipes, dissolves and soft-edged rules as data baked from its exe (see `docs/new-game.md` §10) instead of a plugin each; transitions that move pixels stay plugins (`cons_slide`, `cons_wave`, ...). CONscripter PR #3 |
| Lua state in save files (the script's data globals) and `cons_process(name, "fn", ...)` (a coroutine resumed once a frame, saved by name and restarted from the top on load) | Logic read from an exe (rankings, unlock checks, minigames) written in Lua instead of NScripter arithmetic; 装甲悪鬼村正's CreateProcess (510 calls, parallel processes run each frame) is the first planned user. CONscripter PR #30 |
| `chpan ch,pan[,ms]` (a `dwave` channel's pan, -1000 left .. 1000 right, the other side lowered, ramped in the track effect as `chvolfade`'s volume; stays with the channel, saved, `reset` centres it; not the music) (CONscripter PR #35) | NSS `SetPan`: 続・殺戮のジャンゴ's sounds panned hard left or right (`MusicStart`'s 4th argument, dja001_03's footsteps); 装甲悪鬼村正 pans none off centre |

Planned, from limits hit by converted games (each one is worked around
in the script today), roughly in order of payoff:

| Game feature | Workaround today | Engine feature that would fix it |
|---|---|---|
| Right-click menu / hide window | Everything behind the on-screen button bar | CONscripter has `rgosub *label` now (engine PR #20): a right click at a text wait gosubs and returns to the same wait. Not emitted yet: which menu a right click opens is each game's choice (its command menu, or hiding the window), to be set where the game's menus are written |

Done without engine work (the stock commands are enough; the per-game
details, record layouts and handler addresses are in the game's document):
- **Window caption:** CONscripter's `caption` converts the title from the
  script's encoding to UTF-8, so Japanese titles show in the browser tab
  and the window manager.
- **Movies:** ffmpeg to H.264 + AAC MP4 (loose files in `movie/`),
  played with `bgmstop` + `mpegplay "movie\name.mp4",skippable`. The web
  build plays it in a `<video>` element (muted when the browser blocks
  autoplay with sound); Windows and Android hand it to the system
  player; the Linux and macOS builds skip movies.
- **BGM loop points:** written into each `.ogg` as `LOOPSTART`/`LOOPEND`
  Vorbis comments, which SDL_mixer (2.0.2+) loops on by itself,
  sample-exact, when `bgm` plays music looped (it ignores `LOOPSTART`
  without an end). Being in the file, the loop survives saves and works
  in stock ONScripter. Not used: `bgm "(seconds)file"` loop-back, which
  isn't saved and never fires in ONScripter-Yuri (`playSound` registers
  `Mix_HookMusicFinished` only when `Mix_PlayMusic` fails).
- **Backlog speaker names and voice replay:** every text line starts with
  a `[name|voice]` tag. ONScripter keeps a page's tag in the log, and in
  saves, when there is a `pretextgosub` (ours only runs `saveon`). The
  backlog reads it with `gettaglog`, shows the name above the page and
  replays the voice when the name is clicked.
- **Ruby:** NScripter's `(base/reading)`, added after escaping (which
  turns the text's own parentheses full-width), drawn by `rubyon`.
  `rubyon` puts the ruby in the line gap and draws text a line gap
  lower, so `setwindow` and the backlog's `strsp` start a line gap higher
  to keep the text in place. ONScripter reads the markup bytes singly, so
  single-byte padding must treat them like colour tags. Readings wider
  than their base push the rest of the line right.

## 5. Asset pipeline

- **Images:** decode proprietary formats (TLG5/6, HG-3, CompressedBG, G00,
  PNA, …) into the `image` crate's types and save as PNG, or with
  `--webp` as near-lossless WebP (§13.1). Then sprite
  composition, resize to the target resolution, and alpha conversion according
  to `target_caps`. Run per-image work in parallel with `rayon`.
- **Audio:** keep OGG where possible; transcode the rest with an `ffmpeg`
  subprocess. Loop points (KiriKiri `.sli`, RealLive loop metadata) can't be
  carried over directly: either note them in the report or bake intro + loop
  into the output.
- **Video:** `ffmpeg` to H.264 + AAC MP4, which browsers and the system players play.
- **Packing:** write `arc.nsa` / `.sar` natively (simple formats) and
  optionally encode `nscript.dat`.

## 6. Engine support order

| Order | Engine | Why |
|---|---|---|
| 1 | **NScripter** | Round-trip test (NScripter → IR → NScripter) of the IR and output stage, no reverse engineering involved |
| 2 | **Xuse** (最果てのイマ COMPLETE) | First real target, an undocumented in-house engine reversed from the exe. **Finished 2026-10-14** |
| 3 | **Nitroplus NSS** (装甲悪鬼村正) | In progress (branch `nitroplus`, record in `docs/games/muramasa.md`): NSS compiles to NScripter, Lua as the fallback |
| 4 | **5pb./MAGES TaskList** (俺たちに翼はない, PS Vita) | In progress (branch `oretsuba`, record in `docs/games/oretsuba.md`): text scripts in CRI CPK archives, lowered task by task to the IR |
| 5+ | KiriKiri2/KAG, RealLive/Siglus, CatSystem2, BGI/Ethornell, Yu-ris, Artemis, Majiro, Softpal… | Chosen by which games you want to convert next |

### Per-game documents

| Game | Engine | Branch | Document | Status |
|---|---|---|---|---|
| 最果てのイマ COMPLETE | Xuse | `xuse` | [`docs/games/saihate-no-ima.md`](docs/games/saihate-no-ima.md) | Finished 2026-10-14 |
| 装甲悪鬼村正 | Nitroplus NSS | `nitroplus` | [`docs/games/muramasa.md`](docs/games/muramasa.md) | In progress |
| 俺たちに翼はない (PS Vita) | 5pb./MAGES TaskList | `oretsuba` | [`docs/games/oretsuba.md`](docs/games/oretsuba.md) | In progress |

A game's document holds what is specific to it: the install and build
command, file formats, the opcode or command table with handler
addresses, how its system screens map to CONscripter, the progression
variables (`docs/new-game.md` step 11.2), the status and the optional
leftovers. Start one from step 0.4 of `docs/new-game.md`.

**For every new engine:**
- **Identify it** (M0): file extensions, archive magic bytes, strings in
  the `.exe`, bundled DLLs; compare against GARbro's format list.
- **Existing work to learn from:** GARbro (archive and image formats),
  arc_unpacker, VNTextPatch, rlvm/RLdev, KrkrExtract. Use them as
  references, port the logic into Rust, or (for C++ code) link through
  `cxx`. Check each license before reusing code.
- **Per-game profiles** (`profiles/<game>.toml`, planned) can map custom
  macros to IR, define sprite layer rules, set per-game decryption
  parameters (supplied by the user) and override cases where automatic
  conversion gets it wrong. Decryption keys found go there, not in the
  tool's code. Profiles are also the main thing the AI agents write
  (§8).

## 7. Project layout

The layout as planned at the start. What was built is four crates
(`conv-core`, `conv-engines`, `conv-cons`, `conv-cli`; see `CLAUDE.md`,
"Layout"); `conv-assets`, `conv-walker`, `conv-ai`, `conv-mcp`, the
profiles and the `ai`, `walk` and `mcp` commands don't exist
(2026-10-16).

Cargo workspace:

```
crates/
  conv-core/         ir.rs, vfs.rs, profile.rs, report.rs, source_loc.rs
  conv-engines/      nscripter/, kirikiri/ (xp3, kag, tlg), reallive/, ...
  conv-assets/       image codecs, compositing, ffmpeg wrapper
  conv-cons/         emit.rs, varalloc.rs, effects.rs, nsa.rs, caps.rs, lint.rs
  conv-walker/       IR interpreter: branch exploration, reachability, missing files
  conv-ai/           Claude API client, agent loops, tools, response cache
  conv-mcp/          MCP server exposing the tools to Claude Code (§8.5)
  conv-cli/          `conscripter-convert` binary
engine/              CONscripter (git submodule): the target engine, with our additions
profiles/
tests/               synthetic fixtures + insta snapshot tests
target_caps.toml
```

**Main crates:** `clap`, `serde` / `serde_json` / `toml`, `binrw` (binary
parsing), `memmap2`, `encoding_rs`, `image` / `png`, `flate2`, `rayon`,
`thiserror` / `anyhow`, `tracing`, `insta` + `proptest` (tests), and
`reqwest` + `tokio` (AI layer only, behind the `ai` cargo feature).

**CLI:**

```
conscripter-convert inspect <gamedir>                 # engine, archives, script count, unsupported features
conscripter-convert extract <gamedir> -o out/         # unpack and decode only
conscripter-convert convert <gamedir> -o ons/ [--profile p.toml] [--target ons]
conscripter-convert lint ons/                         # check the generated script
conscripter-convert walk ons/|<gamedir>               # IR walker report
conscripter-convert run ons/                          # launch CONscripter on the result
conscripter-convert ai <task> <gamedir> [...]         # run an agent (see §8)
conscripter-convert mcp                               # serve tools over MCP (stdio)
```

## 8. AI agent layer

Not built (2026-10-16). The games are converted by Claude Code agents
working in the repo, one per game, under the rules in `CLAUDE.md`; the
design below is kept as the plan for a built-in agent layer.

### 8.1 Principles

1. **Proposals, not edits.** Agents write candidate artifacts (profile
   fragments, opcode tables, mapping files) to `.conscripter-convert/proposals/`. Nothing
   reaches the output until it passes validation (round-trip, lint, IR
   walker, smoke run) and you approve it with `conscripter-convert ai accept <id>`.
   Agents never start on their own: `convert` failing only *suggests* the
   matching `conscripter-convert ai …` command in its report.
2. **Validated by tools.** Every agent gets the validators as tools, so it can
   check its own hypotheses before proposing (for example: "decompile all
   scenes with this opcode table — do the instruction lengths line up?").
3. **Sandboxed.** Tools only get read access to the game's VFS and write access
   to a scratch workspace. No arbitrary shell, no network beyond the API.
4. **Opt-in data sharing.** Sending script text or assets to the API is a
   choice set in config (`ai.send_script_text`, `ai.send_images`), defaulting
   to off and asked about on first use.
5. **Reproducible and cheap to re-run.** Every request/response is cached on
   disk, keyed by a hash of the request. Re-running a conversion replays cached
   answers instead of calling the API again. Full transcripts are saved next to
   each proposal.
6. **Bounded.** Each agent has a maximum number of turns, a token budget, and
   a dollar cap from config. `inspect` estimates the cost before running.

### 8.2 Agents

| Agent | Input | Tools | Output (proposal) | Validated by |
|---|---|---|---|---|
| **Format sleuth** | Unknown archive or image file | `hexdump(range)`, `entropy(range)`, `find_magic`, `try_decompress(algo, range)`, `list_known_formats` | A format description (header layout, index structure, compression) + test vectors | Extracting the archive with the generic parser driven by that description; sizes and checksums add up |
| **Opcode inferrer** (bytecode engines) | Script files, partial opcode table | `hexdump`, `disasm(table, file)`, `opcode_stats`, `find_strings(sjis)`, `xref(opcode)` | `opcodes.toml`: opcode → length, argument types, IR meaning | All scripts decompile with no stray bytes; text appears in sensible order; IR walker finds no dangling jumps |
| **Macro mapper** (KAG/TJS etc.) | Macro definitions + example uses | `read_script`, `grep_vfs`, `expand_macro`, `ir_preview` | Profile `[macros]` entries mapping custom macros to IR sequences | Conversion of the scenes that use them, IR diffs, lint |
| **Profile drafter** | `inspect` report | All read-only tools + the agents above as sub-tasks | `profiles/<game>.toml` | Full convert + lint + walker |
| **Repair agent** | Lint / walker / smoke-run failures | `read_report`, `ir_slice(loc)`, `read_source(loc)`, `diff_ir` | Profile overrides that fix the cause upstream. Never patches the generated `0.txt` directly. | Re-run; failure count goes down and none are added |
| **Visual checker** | Screenshots from the headless smoke run + matching original frames, if available | Image input | List of suspected problems (sprite offset, missing alpha, wrong layer order) with source locations | Human review (advisory only) |
| **Translator** (optional) | Extracted `Say` text + glossary | Glossary lookup | Translated text table, kept separate from the IR so the original stays intact | Line-length and encoding checks, name consistency against the glossary |

### 8.3 Provider and API usage

- **Claude API** over HTTPS (`POST /v1/messages`). There is no official Rust
  SDK, so `conv-ai` is a thin typed client (`reqwest` + `serde`) and a
  hand-written tool-use loop (`while stop_reason == "tool_use"`).
  Credentials come from `ANTHROPIC_API_KEY`, never from config files that
  could get committed.
- **Models:** default `claude-opus-5-5` for the reasoning-heavy agents
  (opcode inference, format sleuth, repair). Per-agent override in config, for
  example a cheaper model for bulk translation drafts. Model IDs live only in
  config so they're easy to update.
- **Thinking and effort:** adaptive thinking (default on this model). Depth is
  set per agent with `output_config.effort` — `high` for reverse-engineering
  agents, `low`/`medium` for bulk work.
- **Tool calls:** tools are declared with `strict: true` JSON schemas and
  `tool_choice: auto` (forced tool choice is rejected by current models).
  Every tool input is re-validated in Rust before running.
- **Structured outputs:** final proposals are requested with
  `output_config.format` (a JSON schema generated from the Rust types with
  `schemars`), so they deserialize straight into the profile/opcode structs.
- **Streaming:** used for long agent turns to avoid HTTP timeouts.
- **Refusals:** always check `stop_reason` (including `refusal`) before
  reading content. Server-side refusal fallback is enabled by default
  (`fallbacks: "default"` with the `server-side-fallback-2026-07-01` beta
  header); it can be turned off in config.
- **Prompt caching:** stable prefix first — tool definitions, then a frozen
  system prompt with the IR spec, NScripter reference and the engine's
  current notes — with a `cache_control` breakpoint after it. Per-request data
  (hex dumps, script excerpts) goes after the breakpoint. Track
  `usage.cache_read_input_tokens` in the cost report.
- **Batch API:** translation and per-scene macro mapping are sent as message
  batches (about half the cost, asynchronous), with results matched by
  `custom_id`.
- **Errors:** retry on 429/5xx/connection errors with backoff; fail fast on
  400/404 and show the API's message.

### 8.4 Agent context

Agents get engine knowledge from a `knowledge/` directory (format notes, opcode
tables found so far, NScripter command reference) that grows as games are
converted. Accepted proposals feed back into it, so the next game on the same
engine starts with more knowledge.

### 8.5 Interactive mode: MCP server

`conscripter-convert mcp` exposes the same tools (VFS browsing, hexdump, disasm, IR
preview, convert, lint, walk) over MCP. That lets you do interactive reverse
engineering with Claude Code or another MCP client. The built-in agents cover
the batch, non-interactive cases.

## 9. Testing and validation

1. **Unit and snapshot tests per engine** (`insta`), using small synthetic
   archives and scripts written by hand. Never commit commercial assets.
2. **Property tests** (`proptest`): NSA pack → unpack round-trip,
   Shift-JIS encoding, variable allocation.
3. **Round-trip test** with NScripter: script → IR → script should come out
   equivalent.
4. **Linter:** every jump target exists, variable numbers are in range,
   referenced files exist in the archives, Shift-JIS is valid.
5. **IR walker:** follows every choice branch; reports unreachable code,
   endless loops and missing files.
6. **Smoke test:** CONscripter headless (Xvfb or the SDL dummy driver)
   with auto-advance; checks that it reaches the ending labels without errors
   and captures screenshots for the visual checker.
7. **AI layer tests:** a mock API server replays recorded transcripts, so agent
   loops are tested deterministically without network access or cost. A small
   eval set (known formats and opcode tables with the answers removed) measures
   agent accuracy when prompts or models change.
8. **Conversion report** per run: fraction of instructions converted,
   unsupported instructions with source locations, assets dropped or
   approximated, AI proposals and their validation status, API cost.

## 10. Milestones

- **M0:** identify the first game's engine (最果てのイマ) and archive formats (from the
  installed files); build the target engine, write `target_caps.toml`; set
  up the cargo workspace and IR types. (Done with ogapee's ONScripter; the
  target is now CONscripter, §4.1.)
- **M1:** NScripter output stage, NSA packer, NScripter input engine,
  round-trip tests, linter.
- **M2:** the first game's engine: archive reader, image/audio decoders,
  script decompiler, plus a game profile. First full conversion. (If the
  engine is undocumented, pull the AI layer from M4 forward, since the format
  sleuth and opcode inferrer are most useful exactly here.)
- **M3:** asset pipeline polish (composition, resizing, alpha, audio loops),
  IR walker, conversion report.
- **M4:** AI layer foundation: API client, tool loop, response cache, cost
  limits, mock-server tests. First agents: **format sleuth**, **opcode
  inferrer** and **repair agent**.
- **M5:** **macro mapper** and **profile drafter**; MCP server.
- **M6:** second engine (whichever game comes next), headless smoke tests
  and **visual checker**.
- **M7 (optional):** translator agent via the Batch API.

## 11. Legal and ethical guardrails

- The tool only works on a copy the user has installed; it never downloads
  games or ships assets.
- Decryption parameters live in user-supplied profiles, not in the tool.
- Converted output is for personal use; publishing it would distribute
  copyrighted assets. The README says so.
- AI features send game content to a third-party API only with explicit
  opt-in, and only the parts each agent needs.

## 12. Decisions

- First game: 最果てのイマ (Saihate no Ima); its engine is identified in M0.
- Target: CONscripter (our ONScripter-Yuri fork, `engine/` submodule).
  Until 2026-10-12 the target was ogapee's ONScripter with no engine
  patches; now features the original games need and ONScripter lacks are
  added to the engine (§4.1).
- AI agents run only when explicitly invoked with `conscripter-convert ai …`
  (planned, not built: §8).

## 13. Lessons learned and ways to speed up conversion

General lessons, collected while converting games. Each one is stated
for any engine, with the case that taught it as an example (untagged
ones come from 最果てのイマ, Xuse; its details are in
`docs/games/saihate-no-ima.md`), grouped by topic: add a new lesson to
its topic, not under a heading for its game.
The steps that apply them are in `docs/new-game.md`.

### 13.1 Lessons learned

**Know the game before the bytecode.**
- **Learn the game's mechanics first** (wiki, playthrough video): the
  route choice mechanic, loops, endings. Example: 最果てのイマ's route
  choices are hyperlinks in the text. A no-click trace hit a dead end
  that restarts the chapter, which looked like a control-flow bug until
  the link targets were followed.
- **A loop is not necessarily a bug.** Before "fixing" one, check whether
  the original loops there too (dead ends, retries, back to the title).
- **Ask what to drop.** Debug menus, right-click menus and side rooms
  can each cost a session. The user decides.
- **A 100 % save unlocks the extras at once.** Download one instead of
  playing or reverse-engineering the unlock bits.
- **A route map straight from the scripts** (俺たちに翼はない: every NextScene, condition
  and choice per scene, one line each) is the walkthrough when no
  public one exists, and the route check follows it.

**Reading the exe.**
- **Rank opcodes by use count** and convert from the top: a handful of
  opcodes make most of the visible game.
- **Strings name handlers.** Error messages, logged source-file names
  (`Story.cpp` is bookkeeping, not display) and RTTI class names settle
  an opcode's meaning faster than tracing its logic. Dump them first.
- **MSVC RTTI maps factory ids to classes:** `.?AV<class>@@` string →
  type descriptor → complete object locator → vtable. The constructor
  that stores the vtable, with a static `{type id, factory}` table in
  `.rdata`, gives the class for each id.
- **Jump tables can be two-level:** a byte table indexed by the
  sub-number, then an address table. Reading the address table alone
  gives wrong handlers.
- **Look for divide-by-constant magic numbers.** A division compiles to
  a multiply and shift (`imul 0x14f8b589; sar 13` is ÷ 100000). Searching
  for the literal divisor finds nothing and wrongly suggests a lookup
  table.
- **Follow a field into the code before naming it**, and never name it
  after the values it takes. Examples: a 0/128/255 byte that looked like
  black/grey/white was an acceleration; a field whose values (1000, 500,
  0) looked like fade milliseconds was a sort depth, which the handler
  passed to the object creator (fixed in `6b22f6d`).
- **An operand can refer to state defined elsewhere.** An opcode may
  only name a preset that another opcode defines in a setup script; the
  context around the definitions (evening and night backgrounds next to
  colour presets) can fix their meaning.
- **Object lifetimes depend on mode bits.** Check across the corpus which
  modes keep an object (and what is attached to it) and which recreate
  it, before tracking state.
- **Map an exe from the corpus, not from its functions.** (村正) A coverage
  table (each built-in, system variable, text tag and action: corpus use
  count, what the exe does, what the converter does) turns "map the
  whole exe" into a bounded list of differences actually used; code
  reached only through function pointers has to be turned into functions
  before the decompiled output shows what a built-in does
  (`docs/new-game.md` §2, "Mapping the whole exe").
- **A built-in "converted" can still drop an argument** (村正, 2026-10-16):
  NSS `CreateWindow` was listed as converted because its children were
  placed, but its size, which clips them, was never read; the user saw
  the cockpit's tapes run the whole screen. Coverage is per argument and
  behaviour, and the built-in dispatch notes every argument it doesn't
  read (`docs/new-game.md` step 5).
- **Read the scripts' comments.** (俺たちに翼はない) Ports often keep the original's names
  in comments next to the numbers that replaced them (`:Texture 0 //
  BG_BLACK`, `:Voice 22562 // TAK1004`). Checking every commented number
  against the archives proved the ID scheme in one pass, and the
  comments gave the speaker-name table without the executable.

**Checking decoded data.**
- **Check the whole corpus, not one sample:** every message parses,
  every jump target exists, every checksum verifies. Corpus-wide checks
  catch edge cases that one scene hides.
- **Find out how many index spaces a jump uses** (per-script labels,
  global tables, implicit entries, labels on marker instructions).
  Getting it wrong still produces a script that runs, so check it with
  a trace, not by eye.
- **Cross-check references against asset names.** Name prefixes in the
  archives are a free oracle: checking each sprite reference's character
  against its image name found 623 mismatches in one pass.
- **A prefix check can't catch off-by-one inside a block.** The next
  image is usually the same character, so a 0-based misread still passed
  97 %. Count references to degenerate assets (empty files, 1×1
  placeholders) instead: 1,639 references dropped to 0 after the fix.
- **Count pages as they are displayed, not in file order.** Setup
  scripts and debug menus come first in the file but are never shown.
- **Check the routes against a walkthrough before calling a game done.**
  Simulate each route on both sides (the source with the original's
  run-time rules, the converted `0.txt`) and diff page by page,
  including the wrong-order clicks the walkthrough warns about. Static
  reading and spot runs missed a link that stayed clickable after a
  jump skipped its record (`docs/new-game.md` step 11).
- **Whole-corpus runs find what single scenes don't.** (村正) Two engine-killing
  bugs only showed on the full game: more than 254 transition effects
  (`effect` numbers stop at 255) and a half-width character in vertical
  text (CONscripter exits silently). Run the whole game, at least as a
  dry run, after every feature.
- **Check duplicate definitions for comment blocks** before deciding which
  one is live (Muramasa keeps old helper versions inside `/* */`).
- **Skip runs per ending find what logic checks can't** (村正, 2026-10-14): a
  `?` in text (NScripter reads an array and exits), a 270 s cross-fade,
  heap corruption after 500 files. Patch the choices into the script
  (one `if` per line: a false `if` skips the rest of its line) or write
  them as a `--replay` file, play headless with `--auto-play` and a
  trace (no Xvfb, no held Ctrl; a crash leaves a `crash` record), and
  start deep in a route from the tracer's variables (`--goto` and
  `--vars`, or a patched start) to reproduce in seconds.

**Matching the original.**
- **Read the effect's code, then film only what it leaves open.** The
  exe's kernel gives the exact curve; static reading of the scripts
  alone had got transitions wrong. Wine is the last resort (the user,
  2026-10-16: it takes long and costs many tokens): when a real doubt
  is left, run it silent on its own Xvfb display with a script copy
  patched to slow the effect down, take the fewest shots at fixed
  fractions and fit them to the curve (`CLAUDE.md`, "Rules").
- **Use reference screenshots of the original** for layout: measuring
  pixels settles the text box and name plate in one pass.
- **A wine prefix shares the user's home by default.** Its Documents,
  Desktop etc. are symlinks to the real folders, and games save there.
  Replace them with real folders before the first run.
- **Effects ONScripter can't draw: approximate in the converter first,
  then add them to the engine.** Pre-tinted sprite copies (~930 images,
  130 MB) were replaced by the engine's `spfilter`, which also follows
  the game's control flow instead of a file-order guess.
- **Small effect plugins beat general machinery.** Each transition family
  became a short `cons_*.dll` effect-99 plugin, checkable on Xvfb
  mid-effect. Release them in batches: a CONscripter release is a
  version bump and a tag, and CI builds every platform.
- **Atlases without their layout table:** (俺たちに翼はない) measure the parts from the
  picture's alpha (`research/oretsuba/atlas.py`) and say in the game's
  document that the placement is ours.

**Target engine facts (ONScripter / CONscripter).**
- **Sprite numbers set the stacking order, and saves record them.**
  Lower `lsp` numbers draw on top, so dialogs need the lowest slots.
  Renumber menus and bars freely, but keep the game's own sprites at
  fixed numbers so old saves still load.
- **The font must be fixed-width.** ONScripter draws every character in
  a full-width cell, so a proportional font leaves gaps after narrow
  glyphs. Use IPA Gothic (`ipag.ttf`).
- **The target's image decoder limits the asset formats.** Check a new
  format with one test file in every target build (the web build
  especially) before changing the converter. WebP needed CONscripter's
  own libwebp on the web.
- **Judge lossy image quality on the worst region,** not the average:
  crop the worst block of the difference image and compare it enlarged.
  Lossy WebP's 4:2:0 subsampling blurred small coloured detail at every
  quality; near-lossless was fine at half the PNG size, so lossy is only
  an option (`--webp-quality`).
- **Measure completion per source call, not per note.** One call with
  three unread arguments is one gap; a note list alone ranks a command
  by how many arguments it has. Each engine opens a call on the shared
  `Metrics`, and every note made while it is open is that call's gap
  (`tests/snapshots/<game>.metrics.json`).
- **Engine facts worth knowing early:** (村正) `:a` on a JPEG uses its right
  half as the mask; loose files with Japanese names were found only
  once CONscripter looked for their UTF-8 names (4d94b92; before, pack
  them); `;$Sw,h` sets any screen size, `,valueN` raises the global
  border so more locals are saved per slot; `print 10,ms` works without
  an `effect` definition; `%%n` and `$%n` (the variable whose number is
  in `%n`) work as values and as targets.
- **Button loops:** (村正) `btnwait` forgets its buttons once it returns, so
  every way back into the loop defines them again. Sprite buttons are hit
  by rectangle unless `transbtn` is on (then by alpha; `btndef` turns it
  off); with it, a "transparent" button keeps a little alpha (8/1000) to
  stay clickable.
- **CONscripter's errors go to `stderr.txt`** (村正) in the game folder
  (`USE_FILELOG`), not to the terminal: an exit that looks silent has
  its message there.
- **`setwindow` flushes the backlog** (村正; NScripter's rule): a converter
  that sets the window again for each text block keeps no history. The
  converter emits `setwindow3`, which doesn't. And `btndef` turns
  `btndown` off: a screen that wants presses sets `btndown 1` after each
  `btndef clear`, and in that mode `btnwait` also returns on the release
  and on moves with the button held (tell them apart with `isdown`).
- **`lsp2` sprites are no place for things over the text window** (村正): with
  `windowback` they are drawn under it, and ONScripter doesn't step
  their animation. An animated icon over the window is an `lsp` sprite
  numbered at or below `humanz`.
- **Mind the output stage's variables:** (俺たちに翼はない) pick the game's flag ranges
  against the `%N` that conv-cons already writes (`grep -o '%[0-9]*'`),
  locals below the global border and globals clear of the menus'.

**Getting the files.**
- **Console packages decrypt offline.** A Vita package plus its license
  (`work.bin`) unpacks with pkg2zip; the PFS layer with Vita3K's fork of
  psvpfsparser, which has the F00D step built in (the web service the
  original tool needs is gone). Build both from source.

**Compiling from source scripts** (Nitroplus NSS: 装甲悪鬼村正).
- **With source scripts, compile by running them at convert time.** Story
  scenes are straight runs of helper calls. Inlining the helpers, tracking
  objects and variables, and emitting what the player sees at each wait
  gave a readable build in one session. Only what really depends on the
  player has to become runtime code.
- **Find the runtime state statically, then keep it small.** Story flags
  = variables the story files both set and test (about 190 of thousands).
  Everything else (helper scratch, positions, names) stays convert-time.
  A choice or a flag test forks the run; branches that lead to different
  files dispatch at the end of the file, the rest merge.
- **Pure-logic library code that loops over flags** (the route ranking)
  can't be traced. Port it as a small intrinsic and test it with an IR
  interpreter against a hand-worked example.
- **Merging fork states:** a variable set again after the fork must keep
  that later value. Small synthetic file sets in unit tests caught this.

**Builds.**
- **Measure memory before blaming it on mapped files:** (村正) the kernel's OOM
  line gives anon-rss. The full build's 7 GB was heap (60x zooms scaled
  whole, 8 GB bitmaps), not the mapped archives; cutting big zooms to the
  screen brought it to 1.2 GB. Run builds under `systemd-run --user
  --scope -p MemoryMax=…` so a regression can't take the desktop down.

### 13.2 Ways to speed up conversion

Done, use them: the engine's debug trace with `--auto-play`, `--goto`
`--vars`, `--shot-pages`, `--shot-effects` (replaces a static tracer, the
smoke-test recipe and filming our side's effects on Xvfb;
`engine/DEBUG.md`); `--dry-run` for logic (seconds, script only);
`conscripter-convert lint`; `convert-script` for one scene; `nss route`
for a route map.

Still worth building, in rough order of payoff:
1. **Incremental full builds**: keep extracted assets between runs and
   repack only what changed, or stream archive entries straight into the
   `.nsa`; extract in parallel. A full build is minutes, nearly all of it
   assets.
2. **Corpus tests in `cargo test`**: every message parses, every jump
   target exists, every referenced asset is in the archive.
3. **A frontier report**: compare the files a run reaches with the route
   map's successors and print the condition that blocks each.
4. **Game knowledge up front**: route structure, the choice mechanic,
   known endings, given to an agent before it infers opcodes.
5. **Run exe kernels at convert time**: call a transition's or an
   easing's function from the user's own exe in an x86 emulator (for
   example the `unicorn` crate) and keep its output as data: mask frames
   for `cons_rule.dll`, curve tables for `spcurve`. The original's exact
   pixels without porting its code, and nothing from the exe goes into
   the engine or the repo. Works for self-contained functions; one that
   reaches into the exe's objects or renderer still needs reading.
