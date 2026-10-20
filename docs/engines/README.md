# Original engine commands

What each command or opcode of the original engines does, and how the
converter turns it into CONscripter. One file per engine family; every
command a game's dry run counts (the `kinds` of
`tests/snapshots/<game>.metrics.json`) has a row. The game's
`metrics.json` names its file in `engine_doc`.

| File | Engine | Games | Script format |
|---|---|---|---|
| [`nitroplus-nss.md`](nitroplus-nss.md) | Nitroplus NSS | 装甲悪鬼村正, 続・殺戮のジャンゴ, Full Metal Daemon Muramasa (English) | NSS text calling built-ins; the English port's Squirrel calls as `port <name>` |
| [`xuse.md`](xuse.md) | Xuse | 最果てのイマ COMPLETE | compiled bytecode opcodes |
| [`typemoon.md`](typemoon.md) | TYPE-MOON | 月姫 -A piece of blue glass moon- (Switch) | `_NAME(args)` commands |
| [`mages.md`](mages.md) | MAGES TaskList | 俺たちに翼はない (PS Vita) | task lists with key/value parameters |

The table format is read by the project site: keep the column names, backticked names and `unknown` stable.

Each file has one table. Its **Command** cell is the name in backticks,
exactly as the metrics' `kinds` key; **What it does** says what the
original does in one plain sentence, or exactly `unknown` when the
sources don't say (those are listed under "Unknown"). The sources are
the game's `docs/games/<game>.findings.tsv`, its game document, and the
converter; for our side, `engine/COMMANDS.md`. A new kind (a new game or
a new command counted) needs its row before CI passes; a new engine
needs its own file here and its `engine_doc` stem in its `build.rs`.
