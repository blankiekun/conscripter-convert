# Text tables and asset layers

The script names its text by ID; each language is one table, and English
(or a fan translation) is a swapped table, not a second build. Files that
differ by edition live in layers the engine picks at run time. Every build
writes its text in tables.

## The engine (CONscripter)

Reference: `engine/COMMANDS.md`, "Text tables", `language`, `assetlayer`.

- **Reference.** A 0.txt line `^ID` alone is that ID's line from the
  table; a string `"^ID"` in a command line (`select "^…",*a`) is too.
  Plain text lines keep working; a script without `^` lines is untouched.
- **Table.** `text.<code>.txt` beside 0.txt, UTF-8; per line the ID, a
  tab, and the line as it would stand in 0.txt: all tags (`@ \ / !w
  #RRGGBB (base/reading)`, the backquote of English) stay in the text.
  One entry is one 0.txt line, so line numbers are the same in every
  language. `;` lines are comments (a translator's notes, the source
  text). A fan translation is one such file dropped beside 0.txt.
- **Filling.** The script is filled in as it is read, from the table of
  its first `language` command; text then works exactly as before
  (waits, textgosub, rgosub, read marks).
- **Switch at run time.** `language "en"` with a `text.en.txt` reads
  0.txt again with that table and moves every pointer into the script to
  the same place (gosub stack, select, a textgosub's place inside a text
  line). The page on the screen and the backlog keep the text they had.
  About 0.2 s and +40 MB at the switch for the whole of Muramasa.
  The engine remembers the language (`language.txt` in the save folder)
  and reads the script once with its table at a start; a language's own
  font is `default.<code>.ttf`.
- **Saves and read marks by ID.** A filled line's hash for saves and
  `kidoku.dat` is the hash of its `^ID` line, the same in every
  language: a save made in Japanese loads in English at the same page,
  and lines read in one language count as read in the other. Saves and
  `kidoku.dat` from a build without tables still find their lines by
  the hash of the text.
- **Asset layers.** A file the script names is read from
  `layer\<name>\<path>` when an active layer has it, else from `<path>`.
  The text language is a layer (`language "en"` reads `layer\en\…`
  first); `assetlayer "uncensored",1` turns another on, independent of
  the language. Layers turned on last come first. Pictures on the
  screen stay until set again. Archives and loose files alike.

## Asset variants (any game with editions)

The build holds the base files (the Japanese release's) and, for each
variant, only the files that differ, under `layer\<variant>\` with the
base path:

- `en` (the language's code): English UI pictures, name plates, telops,
  subtitled movies. Follows the text language.
- `uncensored`: the English release's CGs where they differ from the
  censored Japanese ones. Its own setting, not tied to the language.

The converter finds the differing files by comparing the two editions'
assets by base path. The script never names a variant path, so saves,
CG gallery flags and replays are the same for every variant. A further
axis (voices, a second translation) is one more layer and one more
settings row.

## Settings screen

Rows in the settings screens (`crates/conv-cons/src/switches.rs`):

- **Language: 日本語 / English**: a global variable; the define section
  runs `language` from it (`picture_settings::define_language`). With
  two editions' code in one script (Muramasa), picking another language
  restarts the game (`definereset`): only a line both editions share may
  switch. A save remembers its language (`%conv_save_lang`) and a load
  goes on in it. The English text is drawn in the English release's
  font, written as `default.en.ttf`.
- **Pictures: censored / uncensored**: a global variable and
  `assetlayer "uncensored",0/1`, run again at startup. Default: follow
  the edition the player owns.

## The converter

Every build writes its text lines as `^ID` and the strings players read
(choice options, name plates, backlog lines) as `"^ID"`, the text in
`text.<code>.txt` of the script's language (`crates/conv-cons/src/strtab.rs`,
from the emitter). A choice plate's text line, placed by its width, goes
in whole. `conscripter-convert fill <dir>` prints the script filled in,
as the engine reads it; the lint and `tests/snapshot.sh` read that.

- **IDs that stay.** 12 hex digits: a hash of the source instruction the
  line came from (`Script::source_map`), the line's kind and its count
  at that place. A rebuild keeps every ID whose source line keeps its
  0.txt lines, whatever moved around it. Without a source map, the last
  label and the line's own text stand for the source. Clashes (48 bits)
  are detected and salted.
- **Keyed by place, not by the Japanese text.** The same Japanese line in
  two places (「……」, 「はい」) gets two IDs, so each can take its own
  translation; the text hash is only the fallback for old saves.
- **Two editions in one build** (Muramasa's `--english`, which compiles
  the Japanese too, `Built::translation`): each run of lines from one
  source instruction whose code differs, or that one edition lacks,
  becomes per-language table lines (`strtab::merge`), so each language
  plays exactly like its original. The English edition's own pictures
  and menus go in `layer\en\`; the build starts in English. Text lines
  are paired by place (the k-th line of a kind takes the other's k-th);
  a place the other edition lacks keeps its own text. The text window
  lines (`setwindow3` per page) go in both tables.

## The translation kit

Copy `text.ja.txt` to `text.xx.txt`, translate the text after each tab
(one line in, one line out, tags kept), drop it beside 0.txt, and switch
with `language "xx"`. The table holds only lines players read (and the
window lines, which a translator leaves alone), so code stays out of reach.

## Planned

1. Muramasa: `layer\uncensored\` (the CGs that differ) and the language
   and pictures rows in its settings.
2. Backlog by ID: the converter writes `log.<code>.txt` per language
   (text ID → the text as a backlog page holds it), the engine records
   each page's IDs, and after a switch shows the page from the new
   language's log table; a translation fix then shows in old logs.
3. A rebuild re-matches a table line whose ID it dropped by the hash of
   its Japanese text, when that text is unique in its scene (in the kit).
4. A save's place looked up across the whole script by its ID.
5. 月姫 (paused): the pages that line up share one script with two
   tables; it keeps its two copies, both in one table, until then.
