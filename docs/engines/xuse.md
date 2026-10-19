# Xuse (最果てのイマ COMPLETE)

Xuse's engine of 最果てのイマ COMPLETE (`Farthest2015.exe`). The scripts
are compiled bytecode (`*.cd`): a u16 opcode (0 to `0x9d`, one handler
each in the exe's dispatch table), a `sub` and an `arg`, most opcodes
pointing at an operand record in a pool. Messages are pre-wrapped lines;
pictures are layered objects addressed by layer and depth. Reader and
compiler: `crates/conv-engines/src/xuse/` (`cd.rs` reads, `convert.rs`
lowers). Game document: `docs/games/saihate-no-ima.md` ("Script
bytecode" has the opcode table with every record's layout).

Columns: **Command** is the name as the converter counts it (the keys of
`tests/snapshots/<game>.metrics.json`'s `kinds`); **What it does** is the
original's behaviour in one sentence, or `unknown` where the sources
don't say; **Arguments** in short; **How we convert it** names the
CONscripter side (`engine/COMMANDS.md`), "left out: <reason>" or "not
converted"; **Uses** counts the calls in the dry run's metrics when this
file was written (the snapshots hold the current counts).

Sources: `docs/games/saihate-no-ima.findings.tsv` (the exe), `docs/games/saihate-no-ima.md` ("Script bytecode"), the converter (`crates/conv-engines/src/xuse/convert.rs`).

## Commands

| Command | What it does | Arguments | How we convert it | Uses |
|---|---|---|---|---|
| `opcode 0x00` | Does nothing for one frame (shares the handler of `0x67`). | none | nothing | 1 |
| `opcode 0x01` | Sets a variable to a number. | variable, value | `mov` | 49 |
| `opcode 0x02` | Adds a number to a variable. | variable, value | `add` | 3 |
| `opcode 0x08` | Sets a variable to another variable's value. | variable, variable | `mov` | 1 |
| `opcode 0x0d` | Keeps only the bits of a variable that another variable also has (bitwise and). | variable, variable | bitwise and | 3 |
| `opcode 0x10` | Jumps to a label in the current script; aimed at its own label it only marks the label. | label number | `goto`, or a label | 895 |
| `opcode 0x15` | Jumps to a label from the label table. | label-table index | `goto` | 442 |
| `opcode 0x16` | Jumps to a label when a variable equals a value. | variable, value, label | `if` | 82 |
| `opcode 0x17` | Jumps to a label when a variable differs from a value. | variable, value, label | `if` | 1 |
| `opcode 0x1a` | Jumps to a label when a variable is at most a value. | variable, value, label | `if` | 1 |
| `opcode 0x22` | Calls a subroutine at a label (at most 20 deep). | label-table index | `gosub` | 3,094 |
| `opcode 0x23` | Returns from a subroutine. | none | `return` | 46 |
| `opcode 0x30` | Part of the debug menu, which players cannot reach. | - | left out: the unreachable debug menu | 1 |
| `opcode 0x36` | Starts a chapter: sets the chapter title shown in the chapter bar and saves. | own label, end label | chapter title variable | 251 |
| `opcode 0x37` | Closes a chapter and jumps past its end. | own label | jump | 251 |
| `opcode 0x38` | Clears the chapter title. | own label | clears the title variable | 251 |
| `opcode 0x39` | Clears the chapter history. | none | clears the static; no chapter history yet | 4 |
| `opcode 0x3b` | Stores an entry name ("debug entry" or "release entry") that only an unused opcode reads. | name | nothing | 2 |
| `opcode 0x40` | Shows a message (pre-wrapped lines, maybe a speaker's name first). | record: lines | text | 30,958 |
| `opcode 0x41` | Shows a voiced message: a message that also plays the character's voice clip. | record: voice id, lines | text with voice | 15,620 |
| `opcode 0x43` | Hides the text window. | 0 or 1 (same frame or next) | `textoff` | 578 |
| `opcode 0x44` | Puts ruby (a small reading) over words of the next message. | line, column, length, reading | ruby | 37 |
| `opcode 0x46` | Makes words of the next message a link that opens another scene (route choices, encyclopedia). | row, column, length, variable, label | buttons over the words; at most 7 links deep (the original: no limit) | 214 |
| `opcode 0x47` | Returns from a link scene. | none | return through the link page | 160 |
| `opcode 0x48` | Leaves every open link scene, back to the main story. | none | `return` through each link page | 2 |
| `opcode 0x49` | Changes the size and colour of part of the next message (shouting, mumbling). | line, column, length, size, colour, edge | text size and colour tags | 565 |
| `opcode 0x4a` | Sets the size and colour of the following messages. | size, colour, edge | a whole-message style | 2 |
| `opcode 0x4c` | Part of a text window preset: the window mode. | mode | `setwindow` per preset | 9 |
| `opcode 0x4d` | Sets how messages appear: typed one character at a time, all at once, or every line typed together (two people speaking). | 0, 1, 2 | typed or shown at once; mode 2 types the lines one after another | 276 |
| `opcode 0x4e` | Indents the next message under its first characters (chat logs). | indent | `indent` | 24 |
| `opcode 0x4f` | Part of a text window preset: the window's look. | style | `setwindow` per preset | 9 |
| `opcode 0x50` | Part of a text window preset: the window's place. | position | `setwindow` per preset | 9 |
| `opcode 0x52` | Waits for a click, then the text goes on below. | window, read flag | click wait (`\` style) | 1,117 |
| `opcode 0x53` | Waits for a click, then starts a new page. | window, read flag | page wait | 501 |
| `opcode 0x54` | Shows a background picture (or none), removing the old background and overlays. | background number, frame | `bg` | 2,195 |
| `opcode 0x55` | Shows a second background picture over the first (frames, overlays). | picture, layer | an overlay sprite | 252 |
| `opcode 0x56` | Fills a background slot with a colour. | slot, colour | a colour picture | 3 |
| `opcode 0x57` | Plays a music track, fading it in. | track, fade-in ms | `bgm` with a fade | 542 |
| `opcode 0x58` | Stops the music, fading it out. | track or 0 (all), fade-out ms | `bgmfadeout` / `bgmstop` | 579 |
| `opcode 0x59` | Cross-fades from one music track to another. | ms, track to stop, track to play | `bgm "file",ms` | 47 |
| `opcode 0x5a` | Plays a sound effect, maybe looping or fading in; not while skipping. | fade-in, loop, sound | `dwave`, `dwavefadein` | 2,633 |
| `opcode 0x5b` | Stops a sound effect (or all), fading it out. | sound or 0 (all), fade ms | `dwavefadeout` | 890 |
| `opcode 0x62` | Part of the debug menu, which players cannot reach. | - | left out: the unreachable debug menu | 1 |
| `opcode 0x66` | Waits a given time; a click or skip may end it, or nothing may. | ms, clickable | `delay`, or `wait` when not clickable | 205 |
| `opcode 0x67` | Does nothing for one frame. | none | nothing | 2 |
| `opcode 0x68` | Waits for a click (one form first turns skip and auto mode off). | 0 or other | click wait | 84 |
| `opcode 0x69` | Waits for a click with the picture alone on screen (text window hidden). | none | click wait, skippable | 32 |
| `opcode 0x6d` | Shakes the screen, the swing shrinking until it stops; a click ends it. | speed, x and y range | `screenshake` | 198 |
| `opcode 0x6e` | Flashes the screen with a colour for a moment (lightning, blows). | colour | `screenflash` | 88 |
| `opcode 0x6f` | Makes the screen pulse (a quick zoom and glow, maybe tinted). | time, preset | `screenpulse` | 87 |
| `opcode 0x70` | Puts a filter (colour change, ripple) over the screen, or clears it. | slot, layer, kind, data | `screenfilter` / `screenripple` | 457 |
| `opcode 0x71` | Waits for the transition set up before and reveals the changes made since. | none | `print` | 3,908 |
| `opcode 0x72` | Fades the screen to black. | ms | a cover faded with `print` | 217 |
| `opcode 0x74` | Fades the screen to white. | ms | a cover faded with `print` | 11 |
| `opcode 0x76` | Sets up a transition (wave, wipe, slide, mosaic and others) for the next reveal. | kind, ms, flags, layer | `print` with the matching effect | 3,670 |
| `opcode 0x77` | Sets up a wipe through a mask picture for the next reveal. | mask, softness, ms | mask wipe | 234 |
| `opcode 0x7a` | Creates a hidden object (a full-screen animation or a picture). | layer, depth, size | a sprite | 91 |
| `opcode 0x7b` | Removes such an object. | layer, depth | sprite removed | 89 |
| `opcode 0x7c` | Shows or hides an object. | layer, depth, visible | sprite shown or hidden | 6 |
| `opcode 0x7d` | Sets an object's anchor (which corner or the centre its place means). | layer, depth, anchor | sprite place | 5 |
| `opcode 0x7f` | Places an object's centre at a point. | layer, depth, x, y | sprite place | 12 |
| `opcode 0x80` | Scales an object. | layer, depth, x and y percent | a pre-scaled copy | 1 |
| `opcode 0x82` | Sets an object's opacity. | layer, depth, opacity | `msp` | 38 |
| `opcode 0x83` | Sets how an object blends with what is below. | layer, depth, blend | dissolve drawn | 1 |
| `opcode 0x84` | Loads a picture into an object. | layer, depth, kind, number | sprite picture | 10 |
| `opcode 0x86` | Starts a key-frame motion (move, fade, turn) on an object, or removes one. | layer, depth, slot, kind, frames | `spmotion`-style move | 19 |
| `opcode 0x87` | Waits for an object's motion to end. | layer, depth | nothing: ours runs to its end at `0x86` | 9 |
| `opcode 0x88` | Defines part of an effect preset (colour change, blur). | preset, slot, kind, blend, opacity, data | preset variables | 45 |
| `opcode 0x8a` | Waits for a full-screen animation to end. | object | a wait | 73 |
| `opcode 0x8b` | Loads a full-screen animation into an object. | object, animation | sprite 59 | 85 |
| `opcode 0x8c` | Starts a screen-wide effect (noise, ripple, shake) in a slot, or clears it. | slot, layer, kind, data | noise sprite, `screenripple`, `screenshake` | 240 |
| `opcode 0x8d` | Waits for the screen-wide effect in a slot to end. | slot | `delay` of the effect's time | 1 |
| `opcode 0x8e` | Shows, moves or removes character pictures (bust-ups), with a short cross-fade. | entries: character, picture, depth, x, y | sprites | 10,271 |
| `opcode 0x8f` | Removes every character picture. | -1 | `csp -1`, the rest put back | 414 |
| `opcode 0x90` | Applies an effect preset to a character picture, or removes it. | character, preset | `spfilter` | 651 |
| `opcode 0x91` | Plays the staff roll. | 0 or 1 | a rendered movie, `mpegplay` | 2 |
| `opcode 0x93` | Unlocks a scene in the scene replay. | scene | global flag | 418 |
| `opcode 0x94` | Ends a replayed scene. | none | `if` on the replay variable | 418 |
| `opcode 0x96` | Stops the sounds and goes back to the title. | none | `reset` | 1 |
| `opcode 0x97` | Plays a movie, maybe skippable; turns skip and auto mode off first. | movie, skippable | `mpegplay` | 5 |
| `opcode 0x99` | Records story progress (a route finished) in the system data. | bit | global flag | 8 |
| `opcode 0x9d` | Marks a CG as seen for the gallery without showing it. | CG number | global flag | 6 |

## Unknown

None: every command's effect is in the sources.
