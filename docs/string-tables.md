# Text tables and asset layers

Text separate from logic: the script names its text by ID, each language
is one table, and English (or a fan translation) is a swapped table, not a
second build. Pictures and other files that differ by edition live in
layers the engine picks at run time. Phase 1 (2026-10-18): the engine
side and a converter prototype (`conscripter-convert strtab`); phase 2
below makes it the default.

## Why: what differs between the two Muramasa builds

Text in the script, as bytes of 0.txt (dry runs, 2026-10-18; text lines
are lines that start no command):

| Game | 0.txt | Text lines | Share |
|---|---|---|---|
| 続・殺戮のジャンゴ | 2.6 MB | 0.9 MB | 36 % |
| 装甲悪鬼村正 | 28.7 MB | 4.9 MB | 17 % |
| Muramasa (English) | 27.0 MB | 3.6 MB | 13 % |
| 俺たちに翼はない | 16.3 MB | 4.6 MB | 28 % |
| 最果てのイマ | 10.9 MB | 1.8 MB | 17 % |
| 月姫 | 35.4 MB | 3.4 MB | 10 % |

Visible strings in commands (choices, `strsp`, `logadd`, names) add
0.02-0.16 MB per game.

Muramasa against its English build, text masked: 109,000 of 585,000
logic lines differ (19 %). 77,000 of them are the text window set per
page (`tateyoko 1:setwindow3 …` and the click icon's place in Japanese,
the port's horizontal box in English): layout that follows the language,
so it goes in the tables too. The other 66,000 are the port's own
timings (eased fades `effectsoft`, `screenshake`, voices bound to its
own page splits): a one-build English plays the Japanese logic with the
English text and boxes.

## The engine (CONscripter)

`engine/COMMANDS.md`: "Text tables", `language`, `assetlayer`.

- **Reference.** A 0.txt line `^ID` alone is that ID's line from the
  table; a string `"^ID"` in a command line (`select "^…",*a`) is too.
  Plain text lines keep working; a script without `^` lines is untouched
  (same pages and pixels as before over 640 pages of Muramasa).
- **Table.** `text.<code>.txt` beside 0.txt, UTF-8; per line the ID, a
  tab, and the line as it would stand in 0.txt: all tags (`@ \ / !w
  #RRGGBB (base/reading)`, the backquote of English) stay in the text.
  One entry is one 0.txt line, so line numbers are the same in every
  language. `;` lines are comments (a translator's notes, the source
  text). A fan translation is one such file dropped beside 0.txt.
- **Filling.** The script is filled in as it is read, from the table of
  its first `language` command; text then works exactly as before
  (waits, textgosub, rgosub, read marks per byte).
- **Switch at run time.** `language "en"` with a `text.en.txt` reads
  0.txt again with that table and moves every pointer into the script to
  the same place (gosub stack, select, a textgosub's place inside a
  text line by its count of waits, a backquote string resumed there).
  The game goes on where it is; the page on the screen and the backlog
  keep the text they had. About 0.2 s and +40 MB at the moment of the
  switch for the whole of Muramasa (native).
- **Saves and read marks by ID.** A filled line's hash for saves and
  `kidoku.dat` is the hash of its `^ID` line, the same in every
  language: a save made in Japanese loads in English at the same page,
  lines read in one language count as read in the other. Old saves and
  `kidoku.dat` from a build without tables still find their lines by
  the hash of the text (user decision: old saves keep their state).
- **Asset layers.** A file the script names is read from
  `layer\<name>\<path>` when an active layer has it, else from `<path>`.
  The text language is a layer (`language "en"` reads `layer\en\…`
  first); `assetlayer "uncensored",1` turns another on, independent of
  the language (Japanese text with uncensored pictures and English text
  with censored ones both work). Layers turned on last come first.
  Pictures on the screen stay until set again. Archives or loose files
  alike.

## Asset variants (any game with editions)

The general mechanism for later JP/EN games: the build holds the base
files (the Japanese release's) and, for each variant, only the files that
differ, under `layer\<variant>\` with the base path. Variants:

- `en` (the language's code): English UI pictures, name plates, telops,
  subtitled movies, the English font's pictures. Follows the text
  language automatically.
- `uncensored`: the English release's CGs where they differ from the
  censored Japanese ones. Its own setting, not tied to the language.

The converter finds the differing files by comparing the two editions'
assets by base path (prototype: 97 of 3,689 shared names differ in three
Muramasa scenes: name plates, telops, quick bar, movies). The script never
names a variant path, so saves, CG gallery flags and replays are the
same for every variant.

## Settings screen

Two rows beside each other (the settings screens are the engine's
`screen`s, `crates/conv-cons/src/switches.rs`):

- **Language: 日本語 / English** — sets a global variable and runs
  `language "ja"`/`language "en"`; at startup the script runs the saved
  choice. Saves and read marks stay valid across a switch.
- **Pictures: censored / uncensored** — a global variable and
  `assetlayer "uncensored",0/1`, run again at startup. Default: follow
  the edition the player owns (uncensored when the build has the
  `uncensored` layer, i.e. the English release was given to the
  converter).

A third axis later (voices, a second translation) is one more layer and
one more row.

## The converter

`conscripter-convert strtab <0.txt> [--map <source map>] [--english
<0.txt> <map>] --scene-prefix <prefix> -o <dir>` splits a built 0.txt
(`crates/conv-cons/src/strtab.rs`).

- **IDs that stay.** 12 hex digits: a hash of the scene (the last label
  starting with the prefix), the source instruction the line came from
  (`--source-map`), the line's kind and its count at that place. Not a
  counter: a rebuild keeps every ID whose source line keeps its 0.txt
  lines, whatever moved around it. Without a map the line's own text
  stands for the source. Clashes (48 bits) are detected and salted.
- **English by source.** The English build's lines of the same scene and
  source instruction go to the same IDs (the k-th Japanese line takes
  the k-th English one; the last takes the rest, the others are left
  empty). For the whole of Muramasa, 281 of 110,519 text lines find no
  English line from the same source and keep the Japanese. The window
  lines of each place go in both tables (76,693).
- **Choices.** `select` options become `"^ID"`. Muramasa's English
  choices are drawn on the port's plates, not `select`: they stay
  Japanese in the prototype (phase 2).

## Prototype result (Muramasa, three scenes)

One build from `convert-nss --chain 3` of both editions with
`--source-map`, split with `strtab`, the English edition's differing
files in `layer\en\`:

- Japanese pages from `text.ja.txt`, English after `language "en"`
  mid-scene, the port's text box with them (`setwindow3` from the table).
- Read marks: a run in Japanese, then a run that switches to English:
  every English page reports `read` (trace) — the marks hold by ID.
- Saves: saved in Japanese, loaded with English as the table: resumes at
  the same line, in English. A save and `kidoku.dat` from the build
  without tables load at the same line, read.
- Switch at a textgosub click wait in the middle of a line: the rest of
  the line comes in the new language (both ways).
- Asset layer: the face window picture of 落人 at the same moment differs
  in 423 pixels (`layer\en` against none); a toy script shows base / en /
  uncensored / both / off in that order of priority.

## Backlog

Backlog pages keep the text as shown. Phase 2: each page keeps the IDs of
its lines, so the backlog shows the current language after a switch and
a translation fix shows in old logs.

## The translation kit

`vn_lines.py` and `0txt-translation-guide.md` (the user's) translate
0.txt in place. With tables the kit reads and writes `text.<code>.txt`:
copy `text.ja.txt` to `text.xx.txt`, translate the text after each tab
(one line in, one line out, tags kept, as the guide says), drop it beside
0.txt, and switch with `language "xx"`. No code can be touched by
mistake: the table holds only lines players read (and the window lines,
which a translator leaves alone).

## Phase 2

1. Converter: conv-cons emits `^ID` itself for every `Op::Text` and
   writes `text.ja.txt`, IDs from the source position the IR carries
   (a new op beside the text, so no line-based heuristics); the English
   pass (`nitroplus/english.rs`, Tsukihime's `language.rs`) fills
   `text.en.txt` by the same IDs. Choices on picture plates, names and
   `logadd` lines through `"^ID"`. Make it the default for every game
   (snapshot.sh then shows each game's 0.txt change once).
2. Muramasa: one build of both editions: `layer\en\` (English pictures,
   name plates, telops, movies, font) and `layer\uncensored\` (the CGs
   that differ), the two settings rows, the English font for English
   text (the font is per language: a table line can set it).
3. Tsukihime: the pages that line up (29,197 of 42,532) share one
   script with two tables; the rest keep their two copies.
4. Engine: backlog by ID; the language chosen kept in the engine's
   global data so startup reads the right table once; the web package
   lists the tables (`cons_index.json`); a save's place looked up across
   the whole script by its ID (merges with the label-stability work:
   per-scene labels, NEXT SLOTS (a)).
5. The kit: `vn_lines.py` on tables.
