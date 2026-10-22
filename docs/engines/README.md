# Original engine commands

What each command or opcode of the original engines does, and how the
converter turns it into CONscripter. One file per engine family. The game's
`metrics.json` names its file in `engine_doc`.

| File | Engine | Games | Script format |
|---|---|---|---|
| [`nitroplus-nss.md`](nitroplus-nss.md) | Nitroplus NSS | 装甲悪鬼村正, 続・殺戮のジャンゴ, Full Metal Daemon Muramasa (English) | NSS text calling built-ins; the English port's Squirrel calls as `port <name>` |
| [`xuse.md`](xuse.md) | Xuse | 最果てのイマ COMPLETE | compiled bytecode opcodes |
| [`typemoon.md`](typemoon.md) | TYPE-MOON | 月姫 -A piece of blue glass moon- (Switch) | `_NAME(args)` commands |
| [`mages.md`](mages.md) | MAGES TaskList | 俺たちに翼はない (PS Vita) | task lists with key/value parameters |

The table format is read by the project site: keep the column names, backticked names and `unknown` stable.

Each file has one table (`Command | What it does | Arguments | How we convert it`):

- **Command**: the name in backticks, exactly as the key in
  `tests/snapshots/<game>.metrics.json`'s `kinds` (whose values are the use counts).
- **What it does**: the original's behaviour in one plain sentence, or
  exactly `unknown` where the sources don't say (listed under "Unknown").
- **Arguments**: in short.
- **How we convert it**: the CONscripter side (`engine/COMMANDS.md`),
  "left out: <reason>", or "not converted".

Sources: the game's `docs/games/<game>.findings.tsv`, its game document and
the converter. A row per command is optional (no lint requires it); a new
engine's file needs its `engine_doc` stem in its `build.rs`.
