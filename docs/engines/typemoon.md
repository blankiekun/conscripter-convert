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

Columns: **Command** is the name as the converter counts it (the keys of
`tests/snapshots/<game>.metrics.json`'s `kinds`); **What it does** is the
original's behaviour in one sentence, or `unknown` where the sources
don't say; **Arguments** in short; **How we convert it** names the
CONscripter side (`engine/COMMANDS.md`), "left out: <reason>" or "not
converted"; **Uses** counts the calls in the dry run's metrics when this
file was written (the snapshots hold the current counts).

Sources: `docs/games/tsukihime.findings.tsv` (the Switch program), `docs/games/tsukihime.md` ("Script language"), the converter (`crates/conv-engines/src/typemoon/convert.rs`).

## Commands

| Command | What it does | Arguments | How we convert it | Uses (Tsukihime) | Uses (Tsukihime EN) |
|---|---|---|---|---|---|
| `_ATSV` | Copies the current state into the auto-save. | flags | not converted (CONscripter's own saves) | 398 | 398 |
| `_CLO2` | Hides the text window at once. | none | `textoff` | 5 | 5 |
| `_CLO3` | Fades the text window out quickly (4 frames). | none | `windowfade` | 12,208 | 12,208 |
| `_CLO4` | Fades the text window out over a given time. | ms | `windowfade` | 978 | 978 |
| `_CLO5` | Fades the text window in over a given time. | ms | `windowfade` | 596 | 596 |
| `_END_` | Ends the script and goes back to the title. | none | return to the title | 519 | 519 |
| `_FADE` | Flashes or fades the screen in a colour, or sets and clears a screen filter. | type, levels | not converted | 61 | 61 |
| `_FADS` | Shows the layers set up since the last one, at once, cross-faded or wiped through a rule picture. | type, ms, rule | `print` with a fade or mask | 34,029 | 34,029 |
| `_FANX` | Does nothing: the Switch program has no such command. | - | nothing | 2,741 | 2,741 |
| `_FCLN` | Starts a full-screen effect. | effect number (argument 5) | not converted | 26 | 26 |
| `_FCSP` | Ends the full-screen effect, at once or over a time. | ms | not converted | 31 | 31 |
| `_GOTO` | Jumps to a label. | label | `goto` | 101 | 101 |
| `_GOTS` | Jumps to the label of the option the player chose. | labels | choice jump | 170 | 170 |
| `_IF__` | Runs on when a variable compares as given with a value, else skips to a local label. | variable, comparison, value, label | `if` | 1,194 | 1,194 |
| `_IFNX` | Does nothing: the Switch program has no such command (never jumps). | - | nothing (falls through) | 234 | 234 |
| `_IFP4` | Jumps to a label, always. | label | `goto` | 204 | 204 |
| `_IFSC` | Jumps to a label when a program flag is 0 (inferred). | label | `goto`, always | 2 | 2 |
| `_IFUR` | Jumps to a label unless every lesson (the Q&A completion flags) has been read. | 1, label | `if` on the lesson flags | 1 | 1 |
| `_JUMP` | Jumps to another script by number, at a label. | scene number, label | jump | 1,741 | 1,741 |
| `_MFAD` | Fades the music out. | fade options | `bgmfadeout`; a second fade not converted | 724 | 724 |
| `_MPLY` | Plays a music track by number (or stops it), with a fade-in and volume. | track, options | `bgm`; start point and one flag not read | 1,171 | 1,171 |
| `_MSAD` | Shows a text line, as `_ZM` does. | text line | text | 678 | 678 |
| `_MSQK` | Shakes the text window. | ms, x, y | not converted | 30 | 30 |
| `_MVOL` | Changes the music volume over a time. | volume, ms | `chvolfade` | 780 | 780 |
| `_NOP_` | Does nothing. | none | nothing | 1 | 1 |
| `_PGST` | Marks the page a loaded save replays the script to. | page | left out: CONscripter's saves restore the state | 17,202 | 17,202 |
| `_RDAS` | With read-skip allowed and the scene already read, shows an indicator and turns read-skip on (inferred). | read flag | not converted | 530 | 530 |
| `_RDED` | Marks a scene as read when it ends; in a scene replay, ends the replay. | scene | the lessons' flags; the rest not converted | 550 | 550 |
| `_RDST` | Marks a scene as read. | scene | global flag | 530 | 530 |
| `_RTM2` | Keeps the music playing through the next `_RTM_`. | none | not converted | 1 | 1 |
| `_RTM_` | Ends the script: stops every sound and the music and returns to the caller. | none | not converted (the next command runs) | 544 | 544 |
| `_RTMN` | Keeps the music playing, then ends the script as `_RTM_`. | none | not converted | 1 | 1 |
| `_RTST` | Records which scene is playing, for `_RDED`. | scene | nothing needed | 527 | 527 |
| `_SACL` | Makes a layer's picture leave out a position offset when drawn. | layer, on | not converted | 450 | 450 |
| `_SBAS` | unknown | layer, number | not converted | 943 | 943 |
| `_SCH2` | Changes a layer's picture, keeping the settings it had. | as `_STCH` | as `_STCH` | 4,674 | 4,674 |
| `_SEFD` | Fades a sound effect channel (or all) out. | channel, fade ms | `chvolfade` | 3,516 | 3,516 |
| `_SELA` | unknown | - | nothing | 26 | 26 |
| `_SELD` | Opens the choice made of the `_SELR` options. | choice number | the choice; its flowchart number not read | 170 | 170 |
| `_SELR` | Adds an option to the next choice (may be greyed out). | index, text, greyed | choice option | 823 | 823 |
| `_SELS` | Opens a yes or no choice. | - | a yes or no screen | 1 | 1 |
| `_SEPL` | Plays a sound effect on a channel, maybe looping, with fade-in and volume. | channel, loop, sound, options | `dwave` with volume and fade-in; options 18, 68, 70 not read | 14,543 | 14,543 |
| `_SESP` | Stops a sound effect channel (or all) at once. | channel | not converted | 18 | 18 |
| `_SEVL` | Changes a sound effect channel's volume over a time. | channel, volume, ms | `chvolfade` | 1,285 | 1,285 |
| `_SGMA` | Makes a motion start from where a layer is now. | layer, motion, key | not converted | 109 | 109 |
| `_SHAF` | Ends the screen shake, at once or dying out. | ms | `screenshake` cleared | 397 | 397 |
| `_SHK2` | unknown | layer, as `_SQK2` | not converted | 125 | 125 |
| `_SHOK` | Shakes the screen in a sine wave that dies out. | x, y, period, turns, delay | `screenshake` | 870 | 870 |
| `_SHS2` | Stops the layer effect `_SHK2` started. | layer | not converted (nothing started) | 400 | 400 |
| `_SNX2` | unknown | layer, value | not converted | 1,678 | 1,678 |
| `_SNX3` | unknown | layer, value | not converted | 845 | 845 |
| `_SNX4` | Does nothing: the Switch program has no such command. | - | left out: the Switch program ignores it | 8,762 | 8,762 |
| `_SOXY` | Shifts a layer's picture sideways when drawn (inferred). | layer, values | not converted | 208 | 208 |
| `_SPFC` | Loads a picture ahead of time so it shows without delay. | picture, scales, colour | nothing | 487 | 487 |
| `_SPMA` | unknown | on or off | not converted | 199 | 199 |
| `_SPRC` | Frees the pictures loaded ahead of time. | none | nothing | 85 | 85 |
| `_SPRD` | Loads a range of pictures ahead of time. | layer, name, ranges | nothing | 36 | 36 |
| `_SQK2` | Shakes a layer (or every shown one), for a time or for ever. | layer, x, y, ms | not converted | 1,236 | 1,236 |
| `_SQS2` | Stops a layer shake. | layer | not converted (nothing started) | 532 | 532 |
| `_SSS2` | Stops the layer effect `_SSW2` started. | layer | not converted (nothing started) | 398 | 398 |
| `_SSW2` | unknown | layer, arguments 0-7 | not converted | 200 | 200 |
| `_STBR` | Sets how a layer's picture blends with what is below (a shader). | layer, shader | blends and filters; arguments 2, 3 not read | 14,946 | 14,946 |
| `_STCC` | Clears a picture layer (or all of them). | layer | sprite removed | 63,260 | 63,260 |
| `_STCF` | Waits until a layer's fade or motion has finished. | layer, mask | a wait for the fade or motion | 5,179 | 5,179 |
| `_STCG` | Marks a picture as seen for the CG gallery. | picture | global flag | 39 | 39 |
| `_STCH` | Puts a picture on a layer, with place, zoom, opacity, priority, expression and clip. | layer, picture, x, y, zoom, alpha, priority, … | sprite | 54,503 | 54,503 |
| `_STCL` | Stops a layer's motion and moves the layer over a time. | layer, x0, y0, x1, y1, ms, easing | not converted | 9 | 9 |
| `_STCP` | Sets a layer's priority (which layer is drawn in front). | layer, priority | sprite order | 51,568 | 51,568 |
| `_STEC` | Puts an effect on a layer (a colour fill, colour filters). | layer, effect | the matching CONscripter effect | 12,906 | 12,906 |
| `_STGS` | Fades a layer's opacity over a time, with easing. | layer, from, to, ms, easing | sprite fade | 8,114 | 8,114 |
| `_STMA` | Starts (or stops) a motion from the program's table on a layer. | layer, motion, start time | `spmotion`; the start time not read | 24,333 | 24,333 |
| `_STNS` | unknown | layer, 0-12 | not converted | 196 | 196 |
| `_STPV` | unknown | layer, x, y | not converted | 734 | 734 |
| `_STQK` | Does nothing: the Switch program has no such command. | - | nothing | 3 | 3 |
| `_STRA` | unknown | layer, value, ms | not converted | 569 | 569 |
| `_STRR` | Turns a layer's picture to an angle, or back. | layer, angle | not converted | 448 | 448 |
| `_STRT` | Turns a layer over a time. | layer, from, to, ms, centre | at once | 21,691 | 21,691 |
| `_STTR` | Sets a rule picture on a layer (inferred). | layer, rule, x | not converted | 110 | 110 |
| `_STZ4` | Zooms a layer over a time. | layer, from, to, ms | jumps to the end zoom | 32,319 | 32,319 |
| `_TOMP` | Plays a movie (prologue, openings, day cards, endings, credits). | movie | movie | 48 | 48 |
| `_TOMW` | Waits for the movie to end. | none | nothing (our movie blocks) | 48 | 48 |
| `_TRPY` | Does nothing: the Switch program has no such command (trophies). | - | nothing | 2 | 2 |
| `_VCWT` | Waits until the voice playing has ended. | none | not converted | 75 | 75 |
| `_VPL2` | Plays a voice at once, when not skipping and the character's voice is on. | voice, id | not converted | 16 | 16 |
| `_VPLY` | Sets the voice that plays with the next text line. | voice, id | voice | 16,690 | 16,690 |
| `_WKAD` | Adds a number to a variable. | variable, value | `add` | 45,691 | 45,691 |
| `_WKOR` | Sets bits of a variable (bitwise or). | variable, value | not converted | 21 | 21 |
| `_WKST` | Sets a variable. | variable, `eq`, value | `mov` | 487,136 | 487,136 |
| `_WNTY` | Sets the text window type (0 or 1); the type changes the lines' offset. | 0 or 1 | page end | 58,576 | 58,576 |
| `_WTSV` | Waits for the auto-save to finish (inferred). | - | not converted | 28 | 28 |
| `_WTTM` | Waits a given time. | ms | `delay` | 14,828 | 14,828 |
| `_XRSM` | unknown | - | not converted | 2 | 2 |
| `_XSTP` | unknown | - | not converted | 2 | 2 |
| `_ZM` | Shows a text line. | text line | text | 42,601 | 42,601 |
| `_ZY` | Marks a local label for `_IF__`. | label | label | 1,635 | 1,635 |
| `_ZZ` | Marks a label. | label | label | 2,569 | 2,569 |

## Unknown

Commands whose effect the sources don't give yet: `_SBAS`, `_SELA`, `_SHK2`, `_SNX2`, `_SNX3`, `_SPMA`, `_SSW2`, `_STNS`, `_STPV`, `_STRA`, `_XRSM`, `_XSTP`.
