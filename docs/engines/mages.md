# MAGES TaskList (俺たちに翼はない, PS Vita)

5pb./MAGES' "TaskList" ADV engine on CRI middleware (CPK archives,
ADX/HCA audio, USM movies), as in the PS Vita port of 俺たちに翼はない.
A scene is a plain Shift-JIS text file: a list of tasks (`:Tasktype
"Message"`, `"SpriteBG"`, …), each with `:Key value` parameters and
`[CharaN]` blocks for characters. Times are 60 fps frames; the game runs
at 30 fps. Reader and compiler: `crates/conv-engines/src/mages/`. Game
document: `docs/games/oretsuba.md`.

Columns: **Command** is the name as the converter counts it (the keys of
`tests/snapshots/<game>.metrics.json`'s `kinds`); **What it does** is the
original's behaviour in one sentence, or `unknown` where the sources
don't say; **Arguments** in short; **How we convert it** names the
CONscripter side (`engine/COMMANDS.md`), "left out: <reason>" or "not
converted"; **Uses** counts the calls in the dry run's metrics when this
file was written (the snapshots hold the current counts).

Sources: `docs/games/oretsuba.findings.tsv` (the eboot), `docs/games/oretsuba.md`, the converter (`crates/conv-engines/src/mages/convert.rs`).

## Commands

| Command | What it does | Arguments | How we convert it | Uses |
|---|---|---|---|---|
| `Achievement` | Unlocks a PlayStation trophy. | `:ID` | left out: no trophies outside the console | 11 |
| `BacklogMessage` | Adds a line to the text log without showing it on screen. | `:Message` | `logadd` | 56 |
| `BgmPlay` | Starts a music track, fading it in over a number of frames. | `:BGM` track, `:Div` fade-in frames | `bgm`, with a fade-in when `:Div` is above 1 | 1,557 |
| `BgmStop` | Stops the music, fading it out, and can wait for the fade to end. | `:Div` fade-out frames, `:Wait` | `bgmfadeout` | 1,665 |
| `CharacterAction` | Plays a canned movement (a hop, a sway) on the characters on screen. | `[CharaN]` blocks, `:Action`, `:EndWait` | `spmotion` key frames; `:SpriteMax` not read, as by the original | 2,310 |
| `CharacterActionEnd` | Stops a character's canned movement; moves already going on carry on. | `:Character`, `:Copy`, `:EndWait` | as the original; `:Copy` dropped | 2,212 |
| `CharacterDelete` | Removes characters from the screen, fading them out. | `[CharaN]` blocks, `:Fade`, `:FadeOut`, `:FadeWait` | character fade-out; `:SpriteMax` not read | 1,414 |
| `CharacterMove` | Slides characters to a new place over time, with easing. | `[CharaN]` blocks, `:Time`, `:Position`, `:Wait` | `spmotion`; `:SpriteMax` not read | 836 |
| `CharacterOpen` | Shows up to ten characters (pose, clothes, face), fading them in. | `[CharaN]` blocks (`:Number`, `:Position`, `:Priority`, …), `:SpriteMax`, `:Fade` | one sprite per character, faded in | 16,948 |
| `CharacterOpenBS` | Shows characters like `CharacterOpen`, but never fades them in. | as `CharacterOpen` | one sprite per character, at once | 892 |
| `CharacterPriority` | Changes which character is drawn in front. | `:Priority` | sprite order | 44 |
| `CharacterStop` | Stops every character movement: moves jump to their end, actions back to their place. | none | `spmotion` stop on every moving character | 2 |
| `CharacterWait` | Waits until the characters have finished moving. | none | a wait for the motions | 651 |
| `EffectEnd` | Ends a screen effect started by `EffectStart`, fading it out. | `:Fade`, `:Delete` | fade-out; `:Delete` noted | 136 |
| `EffectQuake` | Shakes the screen, for a time or until `EffectQuakeEnd`. | `:Param` swing, `:Time`, `:Loop`, `:Window` | `screenshake`; the text window does not shake with it | 1,076 |
| `EffectQuakeEnd` | Stops a screen shake, at once or dying out over some frames. | `:Fade` | as the original | 918 |
| `EffectStart` | Starts a screen effect (particles, light) chosen by number. | `:EffectNumber`, `:Param`, `:Fade`, `:Priority` | per effect (the game document); `:Priority` dropped, particle motion our own | 208 |
| `End` | unknown | none | return to the title (where the eboot handles it is not traced) | 469 |
| `EyeCatch` | Shows the scene-change card: shutters with a picture, colour panels or the chapter card. | `:Image`, `:Color`, `:Fade` | shutters and chapter card as the original; colour panels dropped | 847 |
| `EyeCatchEnd` | Fades the scene-change card away. | `:Fade` | fade | 847 |
| `FadeIn` | Fades the screen back in from a colour. | `:Time`, `:Type` (rule) | cover sprite fade; plain fades only | 579 |
| `FadeOut` | Fades the screen out to a colour. | `:Time`, `:Color`, `:Type` (rule), `:SpriteDelete` | cover sprite fade; plain fades only | 585 |
| `FlagOperation` | Sets or changes a story flag (such as a heroine's points). | `:FlagName`, `:Action`, `:CGFlag` | variable arithmetic; `:CGFlag` noted | 11 |
| `FlagSwitch_Tag` | Jumps to a label depending on a story flag's value. | `:FlagName`, `:Operator`, `:Param`, `:NextTag` | `if`; `==` and `<` only (all the scenes use) | 59 |
| `Flash` | Flashes the screen with a colour, in and out. | `:InTime`, `:OutTime`, `:Color`, `:TimeWait` | flash over in and out; `:TimeWait` not read | 461 |
| `GlobalFlagOperation` | Sets or changes a flag kept across all saves (such as an ending seen). | `:FlagName`, `:Action` | global variable arithmetic | 19 |
| `GlobalFlagSwitch_Tag` | Jumps to a label depending on a flag kept across all saves. | `:FlagName`, `:Param`, `:NextTag` | `if` on a global variable | 21 |
| `InOut` | Changes the scene with a wipe through a rule picture, or a plain cross-fade. | `:Type`, `:Time` | `print` with the mask | 969 |
| `InOut_Act` | Changes the scene with an animated transition (squeeze, push, mosaic, blur, warp). | `:Type`, `:Param`, `:Time` | per type (the game document); extra `:Param`s noted | 441 |
| `Jump_Tag` | Jumps to a label in the same scene. | `:NextTag` | `goto` | 104 |
| `Message` | Shows a line of text, with the speaker's name and voice. | `:Message`, `:Name`, `:Voice`, `:FontScale`, `:Color` | text with speaker and voice; `:Color` noted | 65,485 |
| `MessageSwitch_Tag` | Offers a choice; each answer jumps to a label and can add points to flags. | `:AnswerMax`, `:AnswerMessageNumberN`, `:MessageNFlag`, `:Text` | choice with points | 25 |
| `MoviePlay` | Plays a movie, which the player may skip. | `:Movie`, `:Loop`, `:EndWait`, `:Skip` | movie; always skippable, always waited for | 13 |
| `MovieStop` | Stops a movie. | `:EndWait` | nothing: our movies have ended by then | 7 |
| `NextScene` | Goes on to another scene file. | `:NextScene` | jump to the scene | 521 |
| `SceneJumpDialog` | Offers to skip a scene already read, with faces and the date. | `:BG`, `:CharaIcon`, `:Text`, `:NextTag`, `:JumpNextTag`, `:UnreadTag` | button screen; read flag per scene | 447 |
| `SceneJumpDialogEnd` | Closes the skip-scene dialog, fading it. | `:Fade` | the dialog closes on the click; the fade dropped (the user's choice) | 531 |
| `SePlay` | Plays a sound effect, with a fade-in, looping or waiting for its end. | `:SE`, `:Div`, `:Wait`, `:Loop` | `dwave`, `dwavefadein`, a wait of the sound's length | 4,738 |
| `SeStop` | Stops a sound effect (or all), with a fade-out, and can wait for it. | `:SE`, `:Div`, `:Wait` | `chvolfade` and a wait | 2,882 |
| `SetSceneName` | Sets the chapter title shown in saves. | `:SceneName` | chapter title | 24 |
| `Sprite` | Shows a picture (not a character), fading it in, with size and layer. | `:Texture`, `:Position`, `:Priority`, `:Scale`, `:FadeIn`, `:Time`, `:Rule` | `lsp`, fade, zoom | 842 |
| `SpriteBG` | Replaces the background and every picture with a new background, in one fade. | `:Texture`, `:Position`, `:Scale`, `:Rule`, `:FadeIn`, `:FadeWait`, `:Time`, colour filters | one fade; filters as `spfilter`; a fade under the text still waits | 2,840 |
| `SpriteBS` | Like `SpriteBG`, for a background drawn over the characters. | as `SpriteBG` | as `SpriteBG` | 1,410 |
| `SpriteDelete` | Removes a picture, fading it out. | `:SpriteNumber` or `:Texture`, `:FadeOut`, `:Time`, `:Rule`, `:FadeWait` | hide with a fade | 72 |
| `SpriteDeleteBG` | Removes the background, always fading it out. | `:Time` | as the original | 277 |
| `SpriteMove` | Moves, zooms or turns a picture over time, along a straight or bowed path. | `:Texture` or `:SpriteNumber`, `:Time`, `:Position`, `:Curve`, `:Scale`, `:Roll`, `:Wait` | `spmotion` and `spzoom`; `:Roll` dropped | 385 |
| `SpriteMoveEnd` | Settles a picture's move at its end place. | `:Sprite` | as the original | 171 |
| `SpritePriority` | Changes which picture is drawn in front. | `:Priority` | sprite order | 59 |
| `SpriteQuake` | Shakes one picture. | `:Param`, `:Time`, `:Fade`, `:Wait` | as the original | 87 |
| `Tag` | Marks a label other commands jump to. | `:Name` | label | 1,194 |
| `TimeWait` | Waits a given time. | `:Time` frames | `wait` in ms | 405 |
| `Window` | Hides the text window (fading it) or shows it again. | `:Visible`, `:Time`, `:WindowNumber` | `windowfade`; a timed fade-in dropped (shown at the next line) | 1,560 |

## Unknown

Commands whose effect the sources don't give yet: `End`.
