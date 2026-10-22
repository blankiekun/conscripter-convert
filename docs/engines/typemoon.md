# TYPE-MOON (月姫 -A piece of blue glass moon-, Switch)

TYPE-MOON's engine of the 月姫 remake on the Nintendo Switch (MZP/mrg
archives, as in the 魔法使いの夜 remake). A script is one line of
ASCII commands, `_NAME(arg,arg,…);`; text lives apart in
`script_text.mrg` and the script names a line by number (`$000107`).
The `_Z?` commands carry a five-digit id after the name (`_ZM01602`),
counted here without it (`_ZM`). Pictures sit on 40 numbered layers.
Reader and compiler: `crates/conv-engines/src/typemoon/` (`script.rs`
reads, `convert.rs` lowers). Game document: `docs/games/tsukihime.md`
("Script language").

Sources: `docs/games/tsukihime.findings.tsv` (the Switch program), `docs/games/tsukihime.md` ("Script language"), the converter (`crates/conv-engines/src/typemoon/convert.rs`).

## Commands

| Command | What it does | Arguments | How we convert it |
|---|---|---|---|
| `_ATSV` | Copies the current state into the auto-save. | flags | not converted (CONscripter's own saves) |
| `_CLO2` | Hides the text window at once. | none | `textoff` |
| `_CLO3` | Fades the text window out quickly (4 frames). | none | `windowfade` |
| `_CLO4` | Fades the text window out over a given time. | ms | `windowfade` |
| `_CLO5` | Fades the text window in over a given time. | ms | `windowfade` |
| `_END_` | Ends the script and goes back to the title. | none | return to the title |
| `_FADE` | Flashes or fades the screen in a colour, or sets and clears a screen filter. | type, levels | not converted |
| `_FADS` | Shows the layers set up since the last one, at once, cross-faded or wiped through a rule picture. | type, ms, rule | `print` with a fade or mask |
| `_FANX` | Does nothing: the Switch program has no such command. | - | nothing |
| `_FCLN` | Starts a full-screen effect. | effect number (argument 5) | not converted |
| `_FCSP` | Ends the full-screen effect, at once or over a time. | ms | not converted |
| `_GOTO` | Jumps to a label. | label | `goto` |
| `_GOTS` | Jumps to the label of the option the player chose. | labels | choice jump |
| `_IF__` | Runs on when a variable compares as given with a value, else skips to a local label. | variable, comparison, value, label | `if` |
| `_IFNX` | Does nothing: the Switch program has no such command (never jumps). | - | nothing (falls through) |
| `_IFP4` | Jumps to a label, always. | label | `goto` |
| `_IFSC` | Jumps to a label when a program flag is 0 (inferred). | label | `goto`, always |
| `_IFUR` | Jumps to a label unless every lesson (the Q&A completion flags) has been read. | 1, label | `if` on the lesson flags |
| `_JUMP` | Jumps to another script by number, at a label. | scene number, label | jump |
| `_MFAD` | Fades the music out. | fade options | `bgmfadeout`; a second fade not converted |
| `_MPLY` | Plays a music track by number (or stops it), with a fade-in and volume. | track, options | `bgm`; start point and one flag not read |
| `_MSAD` | Shows a text line, as `_ZM` does. | text line | text |
| `_MSQK` | Shakes the text window. | ms, x, y | not converted |
| `_MVOL` | Changes the music volume over a time. | volume, ms | `chvolfade` |
| `_NOP_` | Does nothing. | none | nothing |
| `_PGST` | Marks the page a loaded save replays the script to. | page | left out: CONscripter's saves restore the state |
| `_RDAS` | With read-skip allowed and the scene already read, shows an indicator and turns read-skip on (inferred). | read flag | not converted |
| `_RDED` | Marks a scene as read when it ends; in a scene replay, ends the replay. | scene | the lessons' flags; the rest not converted |
| `_RDST` | Marks a scene as read. | scene | global flag |
| `_RTM2` | Keeps the music playing through the next `_RTM_`. | none | not converted |
| `_RTM_` | Ends the script: stops every sound and the music and returns to the caller. | none | not converted (the next command runs) |
| `_RTMN` | Keeps the music playing, then ends the script as `_RTM_`. | none | not converted |
| `_RTST` | Records which scene is playing, for `_RDED`. | scene | nothing needed |
| `_SACL` | Makes a layer's picture leave out a position offset when drawn. | layer, on | not converted |
| `_SBAS` | unknown | layer, number | not converted |
| `_SCH2` | Changes a layer's picture, keeping the settings it had. | as `_STCH` | as `_STCH` |
| `_SEFD` | Fades a sound effect channel (or all) out. | channel, fade ms | `chvolfade` |
| `_SELA` | unknown | - | nothing |
| `_SELD` | Opens the choice made of the `_SELR` options. | choice number | the choice; its flowchart number not read |
| `_SELR` | Adds an option to the next choice (may be greyed out). | index, text, greyed | choice option |
| `_SELS` | Opens a yes or no choice. | - | a yes or no screen |
| `_SEPL` | Plays a sound effect on a channel, maybe looping, with fade-in and volume. | channel, loop, sound, options | `dwave` with volume and fade-in; options 18, 68, 70 not read |
| `_SESP` | Stops a sound effect channel (or all) at once. | channel | not converted |
| `_SEVL` | Changes a sound effect channel's volume over a time. | channel, volume, ms | `chvolfade` |
| `_SGMA` | Makes a motion start from where a layer is now. | layer, motion, key | not converted |
| `_SHAF` | Ends the screen shake, at once or dying out. | ms | `screenshake` cleared |
| `_SHK2` | unknown | layer, as `_SQK2` | not converted |
| `_SHOK` | Shakes the screen in a sine wave that dies out. | x, y, period, turns, delay | `screenshake` |
| `_SHS2` | Stops the layer effect `_SHK2` started. | layer | not converted (nothing started) |
| `_SNX2` | unknown | layer, value | not converted |
| `_SNX3` | unknown | layer, value | not converted |
| `_SNX4` | Does nothing: the Switch program has no such command. | - | left out: the Switch program ignores it |
| `_SOXY` | Shifts a layer's picture sideways when drawn (inferred). | layer, values | not converted |
| `_SPFC` | Loads a picture ahead of time so it shows without delay. | picture, scales, colour | nothing |
| `_SPMA` | unknown | on or off | not converted |
| `_SPRC` | Frees the pictures loaded ahead of time. | none | nothing |
| `_SPRD` | Loads a range of pictures ahead of time. | layer, name, ranges | nothing |
| `_SQK2` | Shakes a layer (or every shown one), for a time or for ever. | layer, x, y, ms | not converted |
| `_SQS2` | Stops a layer shake. | layer | not converted (nothing started) |
| `_SSS2` | Stops the layer effect `_SSW2` started. | layer | not converted (nothing started) |
| `_SSW2` | unknown | layer, arguments 0-7 | not converted |
| `_STBR` | Sets how a layer's picture blends with what is below (a shader). | layer, shader | blends and filters; arguments 2, 3 not read |
| `_STCC` | Clears a picture layer (or all of them). | layer | sprite removed |
| `_STCF` | Waits until a layer's fade or motion has finished. | layer, mask | a wait for the fade or motion |
| `_STCG` | Marks a picture as seen for the CG gallery. | picture | global flag |
| `_STCH` | Puts a picture on a layer, with place, zoom, opacity, priority, expression and clip. | layer, picture, x, y, zoom, alpha, priority, … | sprite |
| `_STCL` | Stops a layer's motion and moves the layer over a time. | layer, x0, y0, x1, y1, ms, easing | not converted |
| `_STCP` | Sets a layer's priority (which layer is drawn in front). | layer, priority | sprite order |
| `_STEC` | Puts an effect on a layer (a colour fill, colour filters). | layer, effect | the matching CONscripter effect |
| `_STGS` | Fades a layer's opacity over a time, with easing. | layer, from, to, ms, easing | sprite fade |
| `_STMA` | Starts (or stops) a motion from the program's table on a layer. | layer, motion, start time | `spmotion`; the start time not read |
| `_STNS` | unknown | layer, 0-12 | not converted |
| `_STPV` | unknown | layer, x, y | not converted |
| `_STQK` | Does nothing: the Switch program has no such command. | - | nothing |
| `_STRA` | unknown | layer, value, ms | not converted |
| `_STRR` | Turns a layer's picture to an angle, or back. | layer, angle | not converted |
| `_STRT` | Turns a layer over a time. | layer, from, to, ms, centre | at once |
| `_STTR` | Sets a rule picture on a layer (inferred). | layer, rule, x | not converted |
| `_STZ4` | Zooms a layer over a time. | layer, from, to, ms | jumps to the end zoom |
| `_TOMP` | Plays a movie (prologue, openings, day cards, endings, credits). | movie | movie |
| `_TOMW` | Waits for the movie to end. | none | nothing (our movie blocks) |
| `_TRPY` | Does nothing: the Switch program has no such command (trophies). | - | nothing |
| `_VCWT` | Waits until the voice playing has ended. | none | not converted |
| `_VPL2` | Plays a voice at once, when not skipping and the character's voice is on. | voice, id | not converted |
| `_VPLY` | Sets the voice that plays with the next text line. | voice, id | voice |
| `_WKAD` | Adds a number to a variable. | variable, value | `add` |
| `_WKOR` | Sets bits of a variable (bitwise or). | variable, value | not converted |
| `_WKST` | Sets a variable. | variable, `eq`, value | `mov` |
| `_WNTY` | Sets the text window type (0 or 1); the type changes the lines' offset. | 0 or 1 | page end |
| `_WTSV` | Waits for the auto-save to finish (inferred). | - | not converted |
| `_WTTM` | Waits a given time. | ms | `delay` |
| `_XRSM` | unknown | - | not converted |
| `_XSTP` | unknown | - | not converted |
| `_ZM` | Shows a text line. | text line | text |
| `_ZY` | Marks a local label for `_IF__`. | label | label |
| `_ZZ` | Marks a label. | label | label |

## Unknown

Commands whose effect the sources don't give yet: `_SBAS`, `_SELA`, `_SHK2`, `_SNX2`, `_SNX3`, `_SPMA`, `_SSW2`, `_STNS`, `_STPV`, `_STRA`, `_XRSM`, `_XSTP`.
