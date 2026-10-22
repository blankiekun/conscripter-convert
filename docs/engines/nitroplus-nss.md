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
depth: `research/nitroplus/builtins.md`. The English Muramasa (JAST USA's
Diesel port, Squirrel scripts) runs the same commands; its port's own calls
are the `port <name>` rows and `ShowVOICESEText` (`english.rs`, `squirrel.rs`).

Sources: `docs/games/muramasa.findings.tsv`, `docs/games/django.findings.tsv` (the two exes), `docs/games/muramasa-en.findings.tsv` (the port), `research/nitroplus/builtins.md`, the converter (`crates/conv-engines/src/nitroplus/`).

## Commands

| Command | What it does | Arguments | How we convert it |
|---|---|---|---|
| `BezierMove` | Moves a picture along a curved path over a time. | object, time, points, tempo, wait | `spmotion` through the path |
| `Boxframe` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `ClearBacklog` | Empties the text log. | none | not converted |
| `ClearScore` | Clears the variables of a scope (local, global or all). | `LOCAL`, `GLOBAL`, `ALL` | not converted |
| `Conquest` | Tells whether a scene has been read, for skipping. | script, scene, mark | left out: the engine keeps its own read marks per page |
| `CreateCamera` | Makes a camera: pictures under it grow or shrink as it moves nearer or away. | object, priority, x, y, z | camera |
| `CreateChoice` | Makes an object a button the player can click (the choice screen). | object | left out: the engine's choice stands in |
| `CreateColor` | Makes a rectangle of one colour (black or white covers for fades and flashes). | object, priority, x, y, w, h, colour | coloured sprite |
| `CreateColorSaddP` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `CreateEffect` | Puts a filter over what is below (a frozen copy of the screen, sepia, monochrome, negative, rain). | object, priority, x, y, w, h, kind | `spfilter`; Rain, Lens, Drop and a tint over part of the screen noted |
| `CreateMask` | Cuts the pictures inside it to the shape of a picture. | object, priority, x, y, picture | converted |
| `CreateMovie` | Makes a movie object, full screen or over the pictures. | object, priority, x, y, loop, alpha, file | movie |
| `CreateName` | Makes an empty named object that groups others. | object | group |
| `CreateProcess` | Runs a script function alongside the story (loops, repeated effects). | object, priority, x, y, function | the processes the converter knows; others noted |
| `CreateSound` | Loads a sound as music, a sound effect or a voice. | object, kind, file | sound |
| `CreateStencil` | Cuts the pictures inside it to the shape of a picture. | object, priority, … | converted |
| `CreateSurface` | Makes a surface that other objects are drawn into. | object, … | not converted |
| `CreateText` | Makes a free text box (captions, menus). | object, priority, x, y, w, h, text | text drawn as a picture; the choice screen's left out |
| `CreateTexture` | Shows a picture from a file, another object or the screen. | object, priority, x, y, source | sprite; from the screen noted |
| `CreateTextureadd` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `CreateWindow` | Makes a box whose children are cut to its rectangle. | object, priority, x, y, w, h, relative | `spclip` |
| `CursorPosition` | Reads where the mouse is. | x variable, y variable | left out: the engine's choice stands in |
| `Delete` | Deletes objects (names may use wildcards); locked ones stay. | object | sprite removed |
| `DeleteFw` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `Draw` | Draws the screen now. | none | `print`, waiting for running actions |
| `DrawEffect` | Makes a picture ripple in waves. | object, time, kind, from, to, tempo | `spwave`; the split kinds noted |
| `DrawTransition` | Wipes a picture in or out through a grey mask picture. | object, time, from, to, softness, tempo, mask, wait | mask wipe (`effectsoft`) |
| `Fade` | Fades an object to an opacity over a time. | object, time, opacity, tempo, wait | sprite fade |
| `ImageHorizon` | Gives a picture's width. | object | worked out when converting |
| `ImageVertical` | Gives a picture's height. | object | worked out when converting |
| `KaisouReverse` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `LoadColor` | Makes a picture of one colour for other objects to use. | object, w, h, colour | left out: only the choice and system screens |
| `LoadFont` | Prepares a font's characters ahead of time; nothing is drawn. | object, face, size, colours, weight, outline, characters | nothing |
| `LoadImage` | Loads a picture without showing it, for `CreateTexture` to copy. | object, file | kept for `CreateTexture` |
| `LoadText` | Lays out a block of story text in a box. | box, text, w, h, spacing | text |
| `LockVideo` | Holds back screen updates, so several changes show in one frame. | on or off | nothing: changes show at the next wait |
| `md02_023_comic` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `ModuleFileName` | Gives the current script's name. | none | worked out when converting |
| `Move` | Moves an object to a place over a time. | object, time, x, y, tempo, wait | `spmotion` |
| `MoveCamera` | Moves a camera, nearer or away, over a time. | object, time, x, y, z, tempo, wait | camera move; tempo and wait of a camera with no children not read |
| `MoveCursor` | Moves the mouse pointer. | x, y | left out: the engine's choice stands in |
| `MoveFFPstop` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `MoviePlay` | Plays a movie full screen and waits for it; may be skippable. | file, skippable | movie |
| `PassageTime` | Gives how long a sound has played. | sound | the sound's length |
| `port CreateTextureEX` | The English port's call that shows a picture; it takes the English picture of that name where the picture has words. | object, priority, x, y, picture | the English picture for the pictures with words, else the Japanese |
| `port CreateTextureSP` | As `port CreateTextureEX`, for a picture with a second name. | object, priority, x, y, picture | as `port CreateTextureEX` |
| `port Move` | The English port moves its text boxes to its own places. | object, time, x, y | the port's places |
| `port Print` | Shows a page of English text, written across, a character fading in at a time. | number, text | CONscripter's text layout; small text at 50 % |
| `port SetChoice` | Shows the choice with the port's wider option plates. | id, options | the English choice style |
| `port ShowVOICESEText` | Shows a centred English subtitle for a voice played as a sound effect. | sound, add, … | subtitles |
| `Position` | Reads an object's place. | object, x variable, y variable | known places; others noted |
| `Random` | Gives a random number from 0 to n - 1. | n | `rnd`; some picked when converting |
| `RandomMeimetsu` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `RecoIn` | Scene replay: fades the replayed scene in. | none | our scene replay |
| `ReConquest` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `RecoOut` | Scene replay: fades the replayed scene out; in the story no such function exists and the call is skipped. | none | left out: the original does nothing there |
| `RecoReturn` | Scene replay: returns to the replay menu; in the story no such function exists and the call is skipped. | none | left out: the original does nothing there |
| `RemainTime` | Gives how long a sound has left to play. | sound | the sound's length |
| `Request` | Asks an object for an action: play, stop, lock, start a process, a blend mode and others. | object, action | most actions converted; some blend modes and stops noted |
| `Rotate` | Turns an object about its pivot over a time. | object, time, x, y, z degrees, tempo, wait | turns; a turn about x or y as a mirror or zoom; some noted |
| `Save` | Saves the game to a slot. | slot | left out: our own save screens |
| `SetAlias` | Gives an object a second name. | object, alias | kept when converting |
| `SetBacklog` | Adds an entry to the text log by hand, with a voice and speaker. | text, voice, name | log entry |
| `SetBlur` | Leaves a motion trail of copies behind a moving picture. | object, on, passes, alpha, lag, wait | `sptrail` |
| `SetFont` | Sets the font, size and colours of text made after it. | face, size, colour, outline colour, weight, outline | the style of `CreateText` |
| `SetFrequency` | Changes a sound's playback speed over a time. | sound, time, rate, tempo | `chfreq` on sound effects; on music noted |
| `SetLoop` | Turns looping on or off for a sound. | sound, on | loop |
| `SetLoopPoint` | Sets the part of a sound that loops. | sound, start, end | loop points |
| `SetNextFocus` | Sets which button the keyboard moves to next (menus). | from, to, direction | left out: the engine's choice stands in |
| `SetPan` | Moves a sound left or right over a time. | sound, time, pan, tempo | `chpan` |
| `SetShade` | Blurs a picture by a named strength. | object, level | blur |
| `SetSurface` | Draws objects into a surface. | object, … | not converted |
| `SetTone` | Recolours one picture (monochrome, sepia, negative). | object, tone | `spfilter` |
| `SetVertex` | Sets the point an object turns and zooms about. | object, x, y | pivot |
| `SetVolume` | Changes a sound's volume over a time. | sound, time, volume, tempo | `chvolfade` |
| `Shake` | Shakes an object, the swing easing from one size to another. | object, time, sizes, n, tempo, wait | `screenshake` |
| `ShakeOne` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `ShowVOICESEText` | Shows a centred English subtitle for a voice played as a sound effect. | sound, add, y, file, times, texts | subtitles |
| `SL_Rightdown` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `SL_Rightdownfade` | Calls a script function that does not exist; the original skips the call. | - | left out: the original does nothing |
| `Strstr` | Finds where a piece of text is inside another. | text, text | worked out when converting |
| `Time` | Gives the clock time in seconds (for play time). | none | not converted: play time is the engine's |
| `Wait` | Waits a given time; a click doesn't end it, skip does. | ms | `wait` |
| `WaitAction` | Waits until an object's running actions have ended. | object, limit | wait for the actions; the limit noted in places |
| `WaitFade` | Waits until an object's actions have ended. | object, limit | wait for the actions |
| `Waitkey` | Calls a script function that does not exist (names are case-sensitive); the original skips the call. | - | left out: the original does nothing |
| `WaitKey` | Waits for a click, or at most a time. | ms | click wait or `delay` |
| `WaitMove` | Waits until an object's actions have ended (the same as `WaitAction`). | object, limit | wait for the actions |
| `WaitPlay` | Waits until a sound or movie has finished, at most a time. | object, limit | `delay` of the rest of the sound |
| `WaitText` | Waits until the text has finished typing and the player has clicked. | text, limit | dropped |
| `WaitZoom` | Waits until an object's actions have ended. | object, limit | wait for the actions |
| `Zoom` | Zooms an object about its pivot over a time. | object, time, x, y per mille, tempo, wait | `spmotion` zoom |

## Unknown

None: every command's effect is in the sources.
