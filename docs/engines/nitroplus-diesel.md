# Nitroplus Diesel (Full Metal Daemon Muramasa, English)

JAST USA's English port of 装甲悪鬼村正 (2021) runs on Nitroplus' Diesel
engine: NPK3 archives and Squirrel 2.2 scripts (`nut/<scene>.nut`) that
call the NSS built-ins by name, with the English text in
`lang/en/<scene>.nut`. The converter compiles the Japanese NSS scripts
and takes from the port only what differs: its text, choices, subtitles,
English pictures and text-box places. So this table holds only the
port's own calls (counted as `port <name>`) and the one command the
English scripts add; every other command Muramasa EN uses is in
`docs/engines/nitroplus-nss.md`. Reader and compiler:
`crates/conv-engines/src/nitroplus/` (`english.rs`, `squirrel.rs`). Game
document: `docs/games/muramasa-en.md`.

Columns: **Command** is the name as the converter counts it (the keys of
`tests/snapshots/<game>.metrics.json`'s `kinds`); **What it does** is the
original's behaviour in one sentence, or `unknown` where the sources
don't say; **Arguments** in short; **How we convert it** names the
CONscripter side (`engine/COMMANDS.md`), "left out: <reason>" or "not
converted"; **Uses** counts the calls in the dry run's metrics when this
file was written (the snapshots hold the current counts).

Sources: `docs/games/muramasa-en.findings.tsv` (the port's exe), `docs/games/muramasa-en.md`, the converter (`crates/conv-engines/src/nitroplus/english.rs`).

## Commands

| Command | What it does | Arguments | How we convert it | Uses |
|---|---|---|---|---|
| `port CreateTextureEX` | The English port's call that shows a picture; it takes the English picture of that name where the picture has words. | object, priority, x, y, picture | the English picture for the pictures with words, else the Japanese | 168 |
| `port CreateTextureSP` | As `port CreateTextureEX`, for a picture with a second name. | object, priority, x, y, picture | as `port CreateTextureEX` | 46 |
| `port Move` | The English port moves its text boxes to its own places. | object, time, x, y | the port's places | 252 |
| `port Print` | Shows a page of English text, written across, a character fading in at a time. | number, text | CONscripter's text layout; small text at 50 % | 39,038 |
| `port SetChoice` | Shows the choice with the port's wider option plates. | id, options | the English choice style | 166 |
| `port ShowVOICESEText` | Shows a centred English subtitle for a voice played as a sound effect. | sound, add, … | subtitles | 32 |
| `ShowVOICESEText` | Shows a centred English subtitle for a voice played as a sound effect. | sound, add, y, file, times, texts | subtitles | 31 |

## Unknown

None: every command's effect is in the sources.
