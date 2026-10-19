# Nitroplus NSS (装甲悪鬼村正, 続・殺戮のジャンゴ)

Nitroplus' engine of the late 2000s (NitroplusSystem; 続・殺戮のジャンゴ's
2008 `Django.exe` is an older build of 装甲悪鬼村正's `Muramasa.exe`). The
scripts are NSS text (`.nss` in `nss.npa`), a C-like language whose
statements call built-in functions by name (`Fade`, `Move`,
`CreateTexture`, …) on named objects; text comes in `<PRE>` blocks. The
exe matches the names exactly (case-sensitive); a name that is neither a
built-in nor a script function is skipped. Reader and compiler:
`crates/conv-engines/src/nitroplus/` (`nss.rs` reads, `convert/` lowers;
Muramasa's own rules in `convert/muramasa.rs`). Game documents:
`docs/games/muramasa.md`, `docs/games/django.md`; the built-ins in
depth: `research/nitroplus/builtins.md`. The English Muramasa runs the
same commands through its port's own layer: `docs/engines/nitroplus-diesel.md`.

Columns: **Command** is the name as the converter counts it (the keys of
`tests/snapshots/<game>.metrics.json`'s `kinds`); **What it does** is the
original's behaviour in one sentence, or `unknown` where the sources
don't say; **Arguments** in short; **How we convert it** names the
CONscripter side (`engine/COMMANDS.md`), "left out: <reason>" or "not
converted"; **Uses** counts the calls in the dry run's metrics when this
file was written (the snapshots hold the current counts).

Sources: `docs/games/muramasa.findings.tsv`, `docs/games/django.findings.tsv` (the two exes), `research/nitroplus/builtins.md`, the converter (`crates/conv-engines/src/nitroplus/`).

## Commands

| Command | What it does | Arguments | How we convert it | Uses (Muramasa) | Uses (Muramasa EN) | Uses (Django) |
|---|---|---|---|---|---|---|
| `BezierMove` | Moves a picture along a curved path over a time. | object, time, points, tempo, wait | `spmotion` through the path | 335 | 335 | 1 |
| `Boxframe` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 168 | 168 | - |
| `ClearBacklog` | Empties the text log. | none | not converted | - | - | 1 |
| `ClearScore` | Clears the variables of a scope (local, global or all). | `LOCAL`, `GLOBAL`, `ALL` | not converted | - | - | 1 |
| `Conquest` | Tells whether a scene has been read, for skipping. | script, scene, mark | left out: the engine keeps its own read marks per page | 2,478 | 2,478 | - |
| `CreateCamera` | Makes a camera: pictures under it grow or shrink as it moves nearer or away. | object, priority, x, y, z | camera | 40 | 40 | 87 |
| `CreateChoice` | Makes an object a button the player can click (the choice screen). | object | left out: the engine's choice stands in | 444 | 444 | 48 |
| `CreateColor` | Makes a rectangle of one colour (black or white covers for fades and flashes). | object, priority, x, y, w, h, colour | coloured sprite | 6,342 | 6,342 | 440 |
| `CreateColorSaddP` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `CreateEffect` | Puts a filter over what is below (a frozen copy of the screen, sepia, monochrome, negative, rain). | object, priority, x, y, w, h, kind | `spfilter`; Rain, Lens, Drop and a tint over part of the screen noted | 2,477 | 2,477 | 1 |
| `CreateMask` | Cuts the pictures inside it to the shape of a picture. | object, priority, x, y, picture | converted | 28 | 28 | 30 |
| `CreateMovie` | Makes a movie object, full screen or over the pictures. | object, priority, x, y, loop, alpha, file | movie | 61 | 61 | 123 |
| `CreateName` | Makes an empty named object that groups others. | object | group | 579 | 579 | - |
| `CreateProcess` | Runs a script function alongside the story (loops, repeated effects). | object, priority, x, y, function | the processes the converter knows; others noted | 5,582 | 5,582 | 83 |
| `CreateSound` | Loads a sound as music, a sound effect or a voice. | object, kind, file | sound | 9,001 | 9,001 | 1,954 |
| `CreateStencil` | Cuts the pictures inside it to the shape of a picture. | object, priority, … | converted | 57 | 57 | - |
| `CreateSurface` | Makes a surface that other objects are drawn into. | object, … | not converted | 52 | 52 | - |
| `CreateText` | Makes a free text box (captions, menus). | object, priority, x, y, w, h, text | text drawn as a picture; the choice screen's left out | 474 | 468 | 33 |
| `CreateTexture` | Shows a picture from a file, another object or the screen. | object, priority, x, y, source | sprite; from the screen noted | 61,268 | 61,266 | 2,950 |
| `CreateTextureadd` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `CreateWindow` | Makes a box whose children are cut to its rectangle. | object, priority, x, y, w, h, relative | `spclip` | 639 | 639 | 975 |
| `CursorPosition` | Reads where the mouse is. | x variable, y variable | left out: the engine's choice stands in | 168 | 168 | 20 |
| `Delete` | Deletes objects (names may use wildcards); locked ones stay. | object | sprite removed | 56,954 | 56,960 | 2,162 |
| `DeleteFw` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `Draw` | Draws the screen now. | none | `print`, waiting for running actions | 169 | 169 | 102 |
| `DrawEffect` | Makes a picture ripple in waves. | object, time, kind, from, to, tempo | `spwave`; the split kinds noted | 76 | 76 | - |
| `DrawTransition` | Wipes a picture in or out through a grey mask picture. | object, time, from, to, softness, tempo, mask, wait | mask wipe (`effectsoft`) | 2,650 | 2,650 | - |
| `Fade` | Fades an object to an opacity over a time. | object, time, opacity, tempo, wait | sprite fade | 302,639 | 302,637 | 11,938 |
| `ImageHorizon` | Gives a picture's width. | object | worked out when converting | 3,829 | 3,831 | - |
| `ImageVertical` | Gives a picture's height. | object | worked out when converting | 5,474 | 5,476 | - |
| `KaisouReverse` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `LoadColor` | Makes a picture of one colour for other objects to use. | object, w, h, colour | left out: only the choice and system screens | - | - | 10 |
| `LoadFont` | Prepares a font's characters ahead of time; nothing is drawn. | object, face, size, colours, weight, outline, characters | nothing | - | - | 67 |
| `LoadImage` | Loads a picture without showing it, for `CreateTexture` to copy. | object, file | kept for `CreateTexture` | 532 | 532 | 144 |
| `LoadText` | Lays out a block of story text in a box. | box, text, w, h, spacing | text | 15,184 | 15,184 | 740 |
| `LockVideo` | Holds back screen updates, so several changes show in one frame. | on or off | nothing: changes show at the next wait | 50,726 | 50,726 | - |
| `md02_023_comic` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 5 | 5 | - |
| `ModuleFileName` | Gives the current script's name. | none | worked out when converting | 1,141 | 1,141 | - |
| `Move` | Moves an object to a place over a time. | object, time, x, y, tempo, wait | `spmotion` | 73,946 | 73,942 | 3,639 |
| `MoveCamera` | Moves a camera, nearer or away, over a time. | object, time, x, y, z, tempo, wait | camera move; tempo and wait of a camera with no children not read | 105 | 105 | 373 |
| `MoveCursor` | Moves the mouse pointer. | x, y | left out: the engine's choice stands in | 168 | 168 | - |
| `MoveFFPstop` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `MoviePlay` | Plays a movie full screen and waits for it; may be skippable. | file, skippable | movie | 3 | 3 | 2 |
| `PassageTime` | Gives how long a sound has played. | sound | the sound's length | 2,108 | 2,108 | - |
| `Position` | Reads an object's place. | object, x variable, y variable | known places; others noted | 506 | 506 | - |
| `Random` | Gives a random number from 0 to n - 1. | n | `rnd`; some picked when converting | 7,411 | 7,411 | - |
| `RandomMeimetsu` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `RecoIn` | Scene replay: fades the replayed scene in. | none | our scene replay | 10 | 10 | - |
| `ReConquest` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 20 | 20 | - |
| `RecoOut` | Scene replay: fades the replayed scene out; in the story no such function exists and the call is skipped. | none | left out: the original does nothing there | 10 | 10 | - |
| `RecoReturn` | Scene replay: returns to the replay menu; in the story no such function exists and the call is skipped. | none | left out: the original does nothing there | 10 | 10 | - |
| `RemainTime` | Gives how long a sound has left to play. | sound | the sound's length | 160 | 160 | - |
| `Request` | Asks an object for an action: play, stop, lock, start a process, a blend mode and others. | object, action | most actions converted; some blend modes and stops noted | 239,985 | 239,991 | 9,128 |
| `Rotate` | Turns an object about its pivot over a time. | object, time, x, y, z degrees, tempo, wait | turns; a turn about x or y as a mirror or zoom; some noted | 16,922 | 16,922 | 794 |
| `Save` | Saves the game to a slot. | slot | left out: our own save screens | 168 | 168 | 10 |
| `SetAlias` | Gives an object a second name. | object, alias | kept when converting | 84,603 | 84,606 | 4,414 |
| `SetBacklog` | Adds an entry to the text log by hand, with a voice and speaker. | text, voice, name | log entry | 2,192 | 2,192 | 75 |
| `SetBlur` | Leaves a motion trail of copies behind a moving picture. | object, on, passes, alpha, lag, wait | `sptrail` | 985 | 985 | - |
| `SetFont` | Sets the font, size and colours of text made after it. | face, size, colour, outline colour, weight, outline | the style of `CreateText` | 16,705 | 16,705 | 195 |
| `SetFrequency` | Changes a sound's playback speed over a time. | sound, time, rate, tempo | `chfreq` on sound effects; on music noted | 8,778 | 8,778 | 1,308 |
| `SetLoop` | Turns looping on or off for a sound. | sound, on | loop | 8,613 | 8,613 | 1,438 |
| `SetLoopPoint` | Sets the part of a sound that loops. | sound, start, end | loop points | 63 | 63 | - |
| `SetNextFocus` | Sets which button the keyboard moves to next (menus). | from, to, direction | left out: the engine's choice stands in | 1,776 | 1,776 | 160 |
| `SetPan` | Moves a sound left or right over a time. | sound, time, pan, tempo | `chpan` | 6,494 | 6,494 | 1,308 |
| `SetShade` | Blurs a picture by a named strength. | object, level | blur | 163 | 163 | 1 |
| `SetSurface` | Draws objects into a surface. | object, … | not converted | 58 | 58 | - |
| `SetTone` | Recolours one picture (monochrome, sepia, negative). | object, tone | `spfilter` | 228 | 228 | - |
| `SetVertex` | Sets the point an object turns and zooms about. | object, x, y | pivot | 18,103 | 18,103 | 1,411 |
| `SetVolume` | Changes a sound's volume over a time. | sound, time, volume, tempo | `chvolfade` | 23,866 | 23,866 | 3,857 |
| `Shake` | Shakes an object, the swing easing from one size to another. | object, time, sizes, n, tempo, wait | `screenshake` | 1,486 | 1,486 | 58 |
| `ShakeOne` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `SL_Rightdown` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `SL_Rightdownfade` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing | 1 | 1 | - |
| `Strstr` | Finds where a piece of text is inside another. | text, text | worked out when converting | 21,009 | 21,013 | - |
| `Time` | Gives the clock time in seconds (for play time). | none | not converted: play time is the engine's | - | - | 1 |
| `Wait` | Waits a given time; a click doesn't end it, skip does. | ms | `wait` | 56,004 | 56,002 | 285 |
| `WaitAction` | Waits until an object's running actions have ended. | object, limit | wait for the actions; the limit noted in places | 5,437 | 5,437 | 5,940 |
| `WaitFade` | Waits until an object's actions have ended. | object, limit | wait for the actions | 4 | 4 | 7 |
| `Waitkey` | Calls a script function that does not exist (names are case-sensitive); the original skips the call. | - | left out: the original does nothing | 2 | 2 | - |
| `WaitKey` | Waits for a click, or at most a time. | ms | click wait or `delay` | 1,683 | 1,679 | 62 |
| `WaitMove` | Waits until an object's actions have ended (the same as `WaitAction`). | object, limit | wait for the actions | - | - | 33 |
| `WaitPlay` | Waits until a sound or movie has finished, at most a time. | object, limit | `delay` of the rest of the sound | 2,477 | 2,477 | 55 |
| `WaitText` | Waits until the text has finished typing and the player has clicked. | text, limit | dropped | 23,689 | 23,689 | - |
| `WaitZoom` | Waits until an object's actions have ended. | object, limit | wait for the actions | 3 | 3 | - |
| `Zoom` | Zooms an object about its pivot over a time. | object, time, x, y per mille, tempo, wait | `spmotion` zoom | 6,621 | 6,621 | 1,694 |

## Unknown

None: every command's effect is in the sources.
