# Nitroplus engine built-ins (装甲悪鬼村正)

What the ~115 functions that NSS calls but never defines do, in the
order we need them: first everything one story scene uses (text,
backgrounds, fades, sound), then the rest by use count.

Sources, strongest first:

1. **The corpus.** `conscripter-convert nss builtins <nss dir> [NAME…]`
   prints, for each built-in, its call count, argument counts and the
   most common values in each argument position. Counts below come from it.
2. **The NSS helpers that wrap the built-ins** (`function.nss`,
   `function_stand.nss`): their parameter names say what an argument is
   (`CreateTextureSP("ナット名", 描画優先度, Ｘ座標, Ｙ座標, $イメージデータ)`).
3. **Two open reimplementations**, read for semantics only (no code taken):
   FGRE/libnpengine (LGPL-3.0), in the Muramasa-specific fork
   `maosasagawa/muramasa-android-port`, which runs this game's scripts
   up to the story; and CommitteeOfZero/nitrosharp (MIT, written for
   Chaos;Head and Steins;Gate, same script language).

**Unverified** marks a row that rests on a guess or on one
reimplementation only. Check those under wine before relying on them
(saiima.md "Running the original").

## Conventions shared by most built-ins

**Objects ("nut", ナット) are named by strings.** A creator
(`CreateTexture`, `CreateColor`, `CreateSound`, `CreateWindow`,
`CreateProcess`, …) stores an object under its name. Creating an object
under a name that already exists replaces it.
- `a/b`: object `b` inside object `a` (windows and choices hold children:
  `"MSGWND/MSG_yes"`, `"絵演窓上/絵演背景"`). The text of a `<PRE @box0>`
  block is the object `box0/text0010`.
- `@name`: look `name` up in the alias table first (`SetAlias(obj, alias)`;
  the scripts nearly always alias an object to its own name). `@@x` = `@x`.
- A trailing `*` is a prefix wildcard: `"絵背景*"` matches every object
  whose name starts with `絵背景`, `"@*"` every alias, `"*"` every object.
  Muramasa uses it constantly (`Delete("*")`, `SetVolume("@mbgm*", …)`).
- A string argument may name a variable: `"$SYSTEM_present_text"` means the
  object whose name is in that variable.
- `Request(obj, Lock)` protects an object from `Delete` (including
  wildcard deletes); `UnLock` lifts it. The boot script locks the text
  windows, so `Delete("*")` in `PrintBG` keeps them.

**Coordinates** are pixels on the 1024×576 screen. Each object has an
origin ("vertex", set by `SetVertex`). `CreateTexture` and `CreateColor`
put it at the object's centre, but positions given to them and to `Move` place the
object's top-left corner. Named positions use the object's width `w` (or
height `h`), case-insensitive:

| Name | x | Name | y |
|---|---|---|---|
| `Center` | (1024 − w) / 2 | `Middle` | (576 − h) / 2 |
| `InLeft` / `Left` | 0 | `InTop` / `Top` | 0 |
| `InRight` | 1024 − w | `InBottom` | 576 − h |
| `OutLeft` | −w | `OutTop` | −h |
| `OutRight` | 1024 | `OutBottom` | 576 |
| `OnLeft` | −w/2 | `OnTop` | −h/2 |
| `OnRight` | 1024 − w/2 | `OnBottom` | 576 − h/2 |

`@n` is relative to the current value (`Move(x, 0, @0, @+4, …)` moves down
4 px; `Zoom(x, t, @0, …)` keeps the scale). A bare `@` (a typo that occurs) presumably means `@0`.

**Units.** Times in ms. Opacity 0–1000 (1000 = opaque). Zoom 1000 = 100 %.
Volume 0–1000. Frequency 1000 = normal speed. Angles in degrees. Priority
(描画優先度): higher draws on top (story backgrounds use 10000, the text windows
20000, the text 20001–20002, full-screen covers 24000–30000).

**Tempo** (easing), the argument before `wait` in every timed call:

| Constant | Easing |
|---|---|
| `null` | linear |
| `Axl1` / `Axl2` / `Axl3` | ease-in, quadratic / cubic / quartic |
| `Dxl1` / `Dxl2` / `Dxl3` | ease-out, quadratic / cubic / quartic |
| `AxlAuto` / `DxlAuto` | sine ease-in / ease-out |
| `AxlDxl` / `DxlAxl` | sine ease-in-out / out-in |

**Wait**, the last argument of every timed call: `true` (or `1`) blocks
for the call's duration, `false`/`null` returns at once and the action
keeps running. Any other number blocks that many ms (`Fade(x, 2000, 0,
null, 500)` starts a 2 s fade and goes on after 0.5 s; 44 calls do this).
This is what makes overlapping animations: several `false` calls,
then a `Wait`/`WaitAction`.

**Colours**: `#rrggbb`, a bare hex number (`FFFFFF`), or a name
(`"Black"`, `BLACK`, `"White"`, …; case-insensitive).

**Assets**: paths in the scripts (`cg/bg/bg001_空a_01.jpg`,
`sound/se/se…`) are stored in the NPAs without the archive's own folder
and **lower-cased** (`cg/sys/adv/TextWindow.png` is `sys\adv\textwindow.png`
in `cg.npa`), so match them case-insensitively. Sound names have no extension
(`.ogg` is implied). BGM is referred to by objects `InitBGM` creates and
aliases (`@mbgm29`).

## Tier 1: one readable scene

`ma01_001.nss`, a typical opening, uses only these (via the NSS helpers
named in parentheses).

### Text

A text block is a statement:

```
<PRE @box0>
[text0010]
　彼は破壊を求めたわけではない。
<voice name="景明" class="景明" src="voice/ma01/0050030a00">
「……」
</PRE>
	SetTextEXC();
	TypeBeginCIO();
```

Executing the `<PRE>` block (no built-in name; libnpengine's `ParseText`):
deletes the previous text object (the one named in `$SYSTEM_present_text`),
builds a text object `box0/text0010` from the block's markup, and sets
`$SYSTEM_present_preprocess` = `@box0` and `$SYSTEM_present_text` =
`@box0/text0010`. Nothing is shown yet. `box0` is a `CreateWindow` made
once by the boot helpers (`function.nss` line 115: full screen, priority
20001, locked).

Then the helpers (12,098 `SetTextEXC`, 1,788 `…EXR`, 640 `…EXL`, 187 `…EXH`;
C/R/L = window centre/right/left, H = horizontal) lay it out, and a
`TypeBegin*` helper shows it. The `TypeBegin` name says
which window (C/R/L/H/NOH) and what happens after the click:
`I` = keep the box, `IO` = fade the box out afterwards, `IFO` = also remove
the face window, `Time…` = auto-advance variants.

| Built-in | Uses | Meaning |
|---|---|---|
| `SetFont(face, size, colour, outline colour, weight, outline)` | 71 | Default font for text created after it. `SetTextEXC`: `"@ＭＳ 明朝", 20, #FFFFFF, #222222, 500, RIGHTDOWN` (the `@` face is the vertical-writing variant; `RIGHTDOWN` = shadow/outline offset to the lower right; `NONE` none). Weight 500 or `HEAVY`/`MEDIUM`. |
| `LoadText(script, box, text, width, height, letter spacing, line spacing)` | 12 | Lays out the block in `text` (named by variable: `"$SYSTEM_present_text"`) as a box `width`×`height` px. `SetTextEXC`: 550×150, spacing 0, 29 px between lines. Horizontal layout; the vertical look comes from rotating it (next rows). |
| `SetVertex(obj, x, y)` | 370 | Sets the object's origin for `Rotate`/`Zoom` (`center`, `bottom`, px). |
| `Rotate(obj, time, x°, y°, z°, tempo, wait)` | 1,332 | Rotation about the vertex. `SetTextEXC` turns the text 90° about its top-left corner, so a 550×150 horizontal box becomes a 150×550 vertical one: **Muramasa's text is vertical** (tategaki), each written line one column, read right to left. |
| `Move(obj, time, x, y, tempo, wait)` | 5,793 | Moves the top-left corner to `x, y`. The text goes to (575, 55), or (575, 134) under a face window. |
| `Request(obj, action)` | 3,644 | See the list after this table. |
| `WaitText(text, timeout)` | 75 | Waits until the text has finished typing **and** the player has clicked (auto mode/skip advance on their own). `null` = no timeout. `TypeBeginCI` also calls it once before `Enter`, presumably a no-op. |
| `<voice name= class= src=>` (tag in the block) | 35,116 | Plays `src` (+ `.ogg`, from `voice.npa`) when typing reaches the tag; `name` is the speaker shown in the backlog, `class` the per-character voice volume/mute setting (`#ConfigCV_…`). |
| `<RUBY text="…">…</RUBY>`, `<BR>`, `<FONT …>`, `<k>` (tags) | 6,929 RUBY | Ruby, line break, font change, and `<k>` probably a click wait inside the block (**unverified**). |
| `{ code }` inside the block | | Runs when typing reaches it (sound effects between lines in `ma01_001` text0040). |

`Request` actions (31 constants; by use):
`Smoothing` (1,029: bilinear filtering on scaled/rotated images, ignore),
`Disused` (593: delete the object once it's idle — sounds when they finish,
textures now), `Start`/`Stop` (511/418: a `CreateProcess` process, or a movie),
`Lock`/`UnLock` (protect from `Delete`), `Play` (sound/movie),
`Erase` (hide; libnpengine: disable), `Enter` (text: start typing it;
others: show), `Hideable` (hidden along with the text window when the
player hides it with right-click), `AddRender`/`SubRender`/`MulRender`
(blend mode), `PushText` (show the whole text at once), `Pause`/`Resume`,
`Passive` (`PrintBG`'s cover; probably "ignores input", **unverified**).

**Open (check under wine before building the text path):** 2,513 of the
~15,000 blocks have 12 or more written lines, but the window
(`cg/sys/adv/textwindow.png`, 134×540 px at x = 447, y = 40) holds 5
columns of 29 px. So the engine must page or scroll inside one block, at
blank lines or when the box fills. libnpengine's port shows one line per
click, which is likely wrong for narration. Take screenshots of
`ma00_000` text0131 (10 lines, a blank line after 4) to settle it.

For CONscripter: the onsyuri base already has vertical text (`tateyoko`).
Map `SetTextEX{C,R,L}` + `TypeBegin*` to a `setwindow` for that box
(x 447–581, columns from 575 leftwards), with the text emitted as
NScripter text, the `<voice>` tag as `dwave 0` and `[name|voice]` backlog tags as in
saiima. `TypeBegin*IO` → the box fades out with the
text (`erasetextwindow`-style `print`).

### Images

| Built-in | Uses | Meaning |
|---|---|---|
| `CreateTexture(obj, priority, x, y, source)` | 1,077 | Image object from a file (`cg/…`), from another texture/`LoadImage` object, or `SCREEN` (a screenshot, for save thumbnails and transitions). Starts opaque. Helpers: `CreateTextureSP` (+ alias, 3,383), `CreateTextureEX` (2,539: creates at opacity 0 and positions by `Center`/`InRight`/`InBottom`/number itself, because backgrounds larger than the screen are centred), `PrintBG`. |
| `CreateColor(obj, priority, x, y, w, h, colour)` | 149 | Solid rectangle. `CreateColorSP` (2,242) = full screen + alias: the black/white covers for fades and flashes. |
| `CreateEffect(obj, priority, x, y, w, h, effect)` | 114 | Full-screen filter over what's below: `"Plain"` (33: copy of the screen, i.e. a frozen frame), `"Rain"`, `"Sepia"`, `"Monochrome"`, `"NegaPosi"`, `"KitanoBlue"`. `PrintBG(name, prio)` (1,180) freezes the screen in a `Plain` effect, deletes everything else (`Delete("*")`, locked objects survive), and `FadeDelete` then fades the frozen frame out: a crossfade into the new scene. |
| `Fade(obj, time, opacity, tempo, wait)` | 11,975 | Fades to `opacity`. 117 calls have 4 arguments (no tempo); NitroSharp also accepts tempo and wait swapped. Helpers: `FadeDelete(obj, time, wait)` (3,584) = fade to 0, then `UnLock` + `Disused`. |
| `Delete(obj)` | 4,982 | Deletes objects (wildcards; skips locked ones). Deleting a process stops it. |
| `Zoom(obj, time, x‰, y‰, tempo, wait)` | 4,679 | Scale about the vertex. |
| `ImageHorizon(obj)` / `ImageVertical(obj)` | 85 / 80 | Width / height in px. `CreateTextureEX` uses them; the converter can read image sizes at convert time. |
| `SetAlias(obj, alias)` | 1,702 | Adds `alias` for `@alias` lookups. Nearly always `SetAlias(x, x)`, which lets `"@x*"` wildcards find it. |
| `DrawTransition(obj, time, from, to, softness, tempo, mask, wait)` | 1,344 | Wipes `obj` in or out through a grey mask (`cg/data/slide_01_01_0.png`, `blind_…`, `circle_…`): opacity from `from` to `to` (0–1000), `softness` the edge width (100 typical). = NScripter `print 18` with the mask, if the mask is converted. |

For CONscripter: object → sprite number from a compile-time table (names
are mostly literals); priority → sprite order (NScripter draws lower sprite
numbers on top, so number = descending priority). `Fade(…, true)` on one
object → set the sprite's alpha, then `print 10,time`; several `false` fades
followed by a wait → one `print`. Different durations, or a fade the script keeps
running while text shows (`false` with no wait after it), need the
Lua fallback.

### Sound

| Built-in | Uses | Meaning |
|---|---|---|
| `CreateSound(obj, kind, file)` | 48 | Loads a sound object; `kind` = `SE`, `BGM`, `VOICE` (picks the volume setting). Helpers: `CreateSE(obj, file)` (3,904; `sound/se/` + file, volume 0, alias), `OnSE(file, volume)` (2,607; one-shot SE on a rotating `OnSE01…10` object). |
| `Request(snd, Play)` | 122 | Starts playback. |
| `SetVolume(snd, time, volume, tempo)` | 2,556 | Volume ramp; `SetVolume("@mbgm*", 2000, 0, null)` fades the BGM out. Wildcards common. |
| `SetLoop(snd, bool)` | 19 | Loop on/off. |
| `SetLoopPoint(snd, start ms, end ms)` | 3 | Loop region. |
| `SetFrequency(snd, time, rate, tempo)` | 182 | Playback speed, 1000 = normal. |
| `SetPan(snd, time, pan, tempo)` | 2 | Pan. |
| `WaitPlay(snd, timeout)` | 150 | Waits until it stops. |
| `PassageTime(snd)` / `RemainTime(snd)` / `DurationTime(snd)` | 2 / 123 / 4 | ms played / left / total. |

Helpers: `MusicStart(obj, time, vol, pan, rate, tempo, loop)` (3,824) and
`SoundPlay(obj, time, vol, loop)` (1,643, BGM) = `Play` + volume ramp + loop + `Disused`.
For CONscripter: BGM → `bgm` (+ `bgmfadeout`), one-shot SE → `dwave`
on a channel picked per object name, looping SE (`MusicStart(…, true)`) →
`dwaveloop`; volume ramps → `chvol` steps or Lua.

### Timing and flow

| Built-in | Uses | Meaning |
|---|---|---|
| `Wait(ms)` | 3,939 | Sleep. NScripter `wait` (or `delay`, which a click skips). |
| `WaitKey([ms])` | 1,639 | Wait for a click, or at most `ms`. `Waitkey` (2) is presumably the same (case-insensitive names, **unverified**). NScripter `click`, or `waittimer`/`btnwait` with timeout. |
| `WaitAction(obj, timeout)` | 552 | Wait until `obj`'s running animations end. |
| `WaitFade` / `WaitZoom` | 4 / 3 | Same, for one kind of animation only. |
| `ModuleFileName()` | 2,005 | Name of the current script (`ma01_001`), used to form `$GameName`. Compile-time constant. |
| `Reset()` | 886 | In `chapter main` when a story file is started directly: restart through the boot script. Only used for debugging, so ignore it. |
| `Conquest(script, scene, bool)` | 312 | "Already read" query/mark for skip (`$AllRead = Conquest(…)`). Map to NScripter's read-text tracking (`skip` honours it) or a global flag. |
| `Random(n)` | 1,412 | 0 … n−1 (**unverified** bound). `rnd`. |

## Tier 2: everything else, by use count

| Built-in | Uses | Meaning | CONscripter |
|---|---|---|---|
| `Shake(obj, time, x amp, y amp, x2, y2, opacity?, tempo, wait)` | 1,343 | Shakes for `time` ms. Args 5–7 mostly 0/0/1000 (**unverified**) | `quake`/`quakex/y` when `obj` is the whole screen; else Lua |
| `SetBlur(obj, on, passes, time?, strength, wait)` | 914 | Gaussian blur; nearly always `true, 1–3, 500, 30–100, false` | engine addition (blur) or skip |
| `Array(…)` / `AssocArray(…)` / `Count(a)` | 719 / 27 / 57 | Array literal, keyed array, length | converter: NScripter arrays (`dim`) or compile-time tables |
| `CreateProcess(obj, prio, x, y, "Func")` | 493 | Runs NSS function `Func` as a parallel thread; `Request(obj, Start/Stop)`, `Delete(obj)` stops it | Lua coroutine (muramasa.md "Approach") |
| `BezierMove(obj, time, curve, tempo, wait)` | 471 | Move along a Bézier path (`(x,y){cx,cy}…` points) | Lua |
| `SetNextFocus(a, b, dir)` | 283 | Menu keyboard navigation: from `a`, key `dir` goes to `b` | own menus |
| `CreateWindow(obj, prio, x, y, w, h, clip?)` | 249 | Container object (children `obj/…`) clipped to the rect | sprite group / `lsp` positions relative |
| `CursorPosition($x, $y)` / `MoveCursor(x, y)` | 167 / 62 | Read / warp the mouse | `getmousepos` / menus |
| `SetShade(obj, level)` | 162 | Blur preset `LIGHT`…`EXTRAHEAVY` | as `SetBlur` |
| `SetTone(obj, tone)` | 143 | `Monochrome` (98), `Sepia` (38), `NegaPosi`, `KitanoBlue` | `lsp` with `:c`-style effect or pre-rendered image |
| `LoadImage(obj, file)` | 136 | Image without display, as a source for `CreateTexture` | |
| `MoveCamera` / `CreateCamera` / `SetFov` / `CreateCube` | 128 / 40 / 1 / 1 | 3D camera over textures (battle scenes) | Lua or pre-render |
| `String(fmt, …)` | 118 | `sprintf` (`"@noise1%04d"`) | `sprintf`-like via `itoa` + concatenation |
| `CreateChoice(obj)` | 112 | Makes `obj` a clickable button; `select`/`case` test it | `btndef`/`btn` + `btnwait` |
| `VariableValue(kind, name[, value])` | 103 | Read/write a variable by computed name (`$`/`#`) | runtime name table |
| `Draw()` | 97 | Force a redraw now | `print 1` |
| `CreateText(obj, prio, x, y, w, h, markup)` | 91 | Free text object (captions `字幕01`, menus) | `strsp` |
| `SetBacklog(text, voice, name)` | 85 | Adds a backlog entry by hand (lines shown as images) | `[name|voice]` tag / skip |
| `DrawEffect(obj, time, kind, …, tempo, wait)` | 79 | Wave distortion (`SuperWave`, `LowWave`, …) | Lua/skip |
| `CreateStencil` / `CreateMask` / `SetSurface` / `CreateSurface` | 59 / 31 / 10 / 3 | Masks and render targets for effects | case by case |
| `LockVideo(bool)` / `XBOX360_LockVideo` | 50 / 18 | Batch screen updates (draw nothing until unlocked) | maps well: `print` only at unlock |
| `Position(obj, $x, $y)` | 47 | Read an object's position | compile-time table |
| `CreateMovie(obj, prio, x, y, loop, alpha, file)` / `MoviePlay(file, skippable)` | 46 / 8 | `.ngs` = MPEG-1 movies; overlays and full-screen | `mpegplay`/`movie`, after converting the files |
| `ExistSave` / `Save` / `Load` / `DeleteSaveFile` | 45 / 18 / 8 / 13 | Save slots (9998/9999 quick saves) | own menus |
| `CreateName(obj)` | 23 | Empty named object (a group) | nothing |
| `DialogChangeState` … `DialogConfigStorage`, `Message(title, text, buttons, icon)` | 18 / 4 | OS dialogs | own menus |
| `Integer` / `Time` / `DateTime` / `Strstr` | 17 / 17 / 1 / 14 | int cast, ms clock, date parts, substring search | NScripter equivalents |
| `ReadFile` / `WriteFile` | 16 / 4 | Save comment files | not needed |
| `ScrollbarValue` / `CreateScrollbar` / `SetScrollbar` / `SetScrollSpeed` | 14 / 7 / 1 / 6 | Config and backlog scrollbars | own menus |
| `SetShortcut(key, script)` | 13 | Hotkeys to system scripts | own menus |
| `LoadColor` | 6 | Colour-filled image source | |
| `ClearScore(LOCAL)` | 5 | Clear local variables | |
| `System("OPEN:url")` | 5 | Open a browser | skip |
| `ClearBacklog` / `CreateBacklog` / `EnableBacklog` | 4 / 1 / 1 | Backlog control | NScripter backlog |
| `Platform` / `EnableDirect3DHAL` / `XBOX360_IsSignin` / `DialogCautionSignIn` | 4 / 3 / 1 / 1 | Platform checks, stubs | constants |
| `SetStream(obj, ms)` | 4 | Streaming buffer for title BGM | skip |
| `Exit()` / `Escape(n)` | 3 / 2 | Quit / break out of n levels (skip mode `BoxOut`) | `end` / converter |
| `Boxframe`, `CreateColorSaddP`, `CreateTextureadd`, `DeleteFw`, `MoveFFPstop`, `SL_Rightdown`, `SL_Rightdownfade`, `SceneFadeBG`, `ShakeOne`, `TextureRoll00H` | 1 each | Typos or removed helpers; the engine presumably ignores unknown calls. Check each call site for dead code | drop |
