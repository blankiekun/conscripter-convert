# Full Metal Daemon Muramasa (English, JAST USA 2021)

The English release of 装甲悪鬼村正 is not a patch of the 2009 PC game
(`docs/games/muramasa.md`) but a port to Nitroplus's newer "Diesel"
engine (`muramasa_en.exe`, DirectX 11, Squirrel scripts, 1024×576).
It has no NPA archives and no NSS scripts.

## Status

- **Works:** reading every archive (`npk.rs`; `research/nitroplus/npk.py`,
  below); the English text in the converter: `convert-nss <game> --file
  ma00_000.nss --chain 2000 --english <english game> …` reads the
  English release's `script.npk` and puts its text into the Japanese
  NSS (`english.rs`): 38,967 stretches of text take their row's
  English, 43 more the rows between their neighbours' (the port split
  or edited the Japanese there), 187 are left out as the translation
  left them out, none stay in Japanese (History, below). With
  `--english` the text is written as it is, one paragraph a line after a
  backquote (`Script::single_byte_text`), and the vertical text boxes
  become horizontal boxes, 280 wide, line pitch 21, 14 lines, in the
  port's own window pictures: `function.nss`'s `boxCC` … `boxLT` are made
  as the port's `CreateAllTextBox` makes them (`cg/sys/adv/en/
  TextWindow(_Fw).png`, 305×475 at x 360, 616, 103, y 51; the NSS's
  `Move`s after them are dropped); the horizontal `boxH`/`boxTH`
  (x 72) and `boxCH`/`boxCTH` (x 264/224) take the port's
  `en/THTextWindow(_Fw).png` (880×158) at the Japanese y. The text
  starts where the port's `BoxInfo` in `val.nut` puts it, as wide as
  its field: C 375,190, R 630,190, L 118,190 (280 wide); H 94,426 (845),
  238,426 with a face (701); CH 280,225 (501) / 360,225 (420); Noh
  280,225 (492) / 351,225 (434) (`EN_TEXT` in `muramasa.rs`). Pictures `cg.npa` lacks are taken
  from the English `cg.npk` (lossless WebP, written as PNG;
  `NpaAssets::cg_en`).
  CONscripter draws it proportionally in the release's
  `droidserif-muramasa.ttf` (pass it as `--font`) and wraps at spaces;
  checked headless on the first pages (`ma00_000`). The port's ㉑-㉗
  (spaces of set widths around italics) become spaces. `<I>` is drawn
  in italics and a `<center>` row centred in the box (CONscripter's
  `!i1` … `!i0` and `!c`, History). A page whose
  English outgrows the box goes on on a new page after a click
  (CONscripter, for every script). The full build plays to the end
  with the trace (25,430 pages, none past the window); its other
  findings are the Japanese build's.
- **Decided (the user, 2026-10-15):** the text font is the release's own
  `droidserif-muramasa.ttf` (font.npk); the menus follow the English
  port's redesigned screens, not the Japanese PC ones; filming the
  English original under wine is allowed.
- **Not yet:** the lines
  per box (14 vertical, 5 at a 28 px pitch horizontal, the Japanese
  pitch) are read from the boxes, not yet seen in the original; in
  `boxTH` the face stays at the Japanese place (x ~230, under the
  text's third line), not in the empty frame the port's
  `THTextWindow_Fw.png` has at its left (x ~80-210; `FwInfo` in
  `val.nut` says H `fw_pos` 228,450: to check in the original).
- **Plan (from the scripts, 2026-10-15):** the port's scenario scripts
  (`nut/*.nut`) are a line-by-line transcription of the Japanese NSS
  (same sprite names, coordinates, files with `.png` for `.jpg`; screen
  1024×576 in both). So the English build reuses the existing Muramasa
  converter on the Japanese NSS and swaps in the English text, joined
  by the Japanese text (below), and takes the English pictures where
  they differ (the redesigned menus in `cg/sys`, pictures with words).
- **Next:**
  1. The English pictures: the redesigned menus in `cg/sys`, filmed
     against the English original under wine (also to check the lines
     per box).
  2. 22 rows of English are placed nowhere (lines the NSS orders or
     splits otherwise, route variants such as mc01's "Ichijo is
     nowhere to be seen" / "My comrades …", ma01_003's `<CLICK>`
     rows); ma01_026's document is shown whole after its typing
     sounds, not typed letter by letter as the port does.

## Archives (NPK3)

`<game>/*.npk`: system, script, cg, dx, font, sound, voice, particle,
plus `patch/*.npk` mounted after them (opener 0x401960).

    python3 -I research/nitroplus/npk.py <game>/system.npk          # list
    python3 -I research/nitroplus/npk.py <game>/script.npk OUT [GLOB]

- Header: `NPK3`, u32 version 1, 16-byte IV, u32 entries, u32 index
  size; the index follows, AES-256-CBC.
- Key: eight dword immediates written at 0x401c28 and passed to the
  archive opener 0x54cf40 (AES-NI key expansion 0x554910); it is not in
  the exe as plain bytes, so a byte scan doesn't find it. Value in
  `npk.py`.
- Entry: u8 flag, u16 name length, UTF-8 name, u32 size, 32-byte hash,
  u32 segments, then per segment u64 offset, u32 stored size (16-byte
  aligned), u32 packed size, u32 size.
- Segment: AES-256-CBC (same key, the archive's IV), then zstd when
  packed size < size; otherwise stored as is. Some zstd frames don't
  record their size.

## Contents

- `script.npk`: 1884 compiled Squirrel files: `media/script/` framework,
  system, function, and 884 scenario files in `lang/en/`
  (`ma00_000.nut` …).
- `cg.npk`, `particle.npk`: the `.png` names hold lossless WebP.
- `font.npk`: `media/font/droidserif-muramasa.ttf` only.
- `system.npk`: `setting.ini` (`SCREEN_WIDTH=1024`, `SCREEN_HEIGHT=576`),
  shaders.

## Scripts (Squirrel 3)

    python3 -I research/nitroplus/nut.py FILE.nut [--strings | --tables]

- Files: `SCRP`, u32 header size 0x10, two u32; then a Squirrel 3
  closure stream (32-bit SQInteger, 1-byte chars, UTF-8) whose header
  keeps only sizeof(SQChar), and whose functions end in u32 stack size,
  u8 generator, u8 varparams. All 1884 files parse.
- Opcodes are renumbered against stock 3.1: `LINE` to `GETK` (0-9),
  `LOAD`, `LOADINT`, `PREPCALLK`, `CALL`, `NEWSLOT` decode right; others
  don't (`NEWOBJ` sits at 30, table-literal slots at 61, booleans and
  nulls come out as `DIV`/`BITW`). The op shown as `ADD a b c op` is the
  arithmetic one: its last operand is the operator's character (43 `+`,
  45 `-`, 42 `*`), so `575 150 ADD…45` is 575 - 150. The text join needs none of them; the
  VM's dispatch in the exe would give the full map.
- `nut/<scene>.nut`: the scenario, one `scene` function calling the
  NSS builtins by name (`CreateTextureSP`, `FadeDelete`, `St`, …). Each
  text block is `Print(key, japanese)`, where the Japanese is an NSS
  `<PRE>` block cut at its inline `{…}` code, so one NSS block gives
  one or more Prints (39,038 Prints for 14,527 blocks).
- `lang/en/<scene>.nut`: `TransText[key] = english` (39,010 rows, plus
  120 `TransLog`). Each value repeats the Japanese as `//` lines, then
  the English, with `<voice …>` tags, `<center>`, `<I>`, `<K>`, `<br>`,
  `<RUBY text='…'>`.
- Join: 38,465 of 38,798 NSS segments (99.1 %) equal a Print's text once
  `//` lines, tags and whitespace are removed; the rest are paired by
  position (History: "Text with no equal Japanese").

## History

- **Italics and centred rows (2026-10-15):** the English has 2,618
  `<I>` … `</I>` and 566 `<center>` rows (most not closed: the row's end
  closes them). CONscripter gained `!i1` … `!i0` (the font opened in
  SDL_ttf's italic style, kept apart from the upright one) and `!c` (the
  rest of the line centred in the room left on the window's line), both
  also read in backquote text, where the other tags are text; the IR's
  `TextStyle` has `italic` and `centred`, which conv-cons writes as those
  tags. The Japanese NSS's two `<I>` (`「鋼の人形は…」`) are italic too.
  The backlog shows the text upright and not centred.

- **Text with no equal Japanese (2026-10-15):** 278 stretches of the
  NSS had no Print with the same Japanese. The translation folded some
  lines into a neighbour's row (ma01_003: Konatsu's line and the
  narration after it are one row, and the narration's Print is gone),
  the port's Japanese is edited or cut elsewhere (この/その, a block
  split at other `{…}` code), and ma01_026's typed document is the
  port's own `SetText` lines, its Japanese Prints having no English.
  `align` (`english.rs`) now pairs the stretches between two matched
  rows with the rows left unused between them: the first takes their
  English, the others none, and with none left the stretch is dropped.
  Prints with no English take the scene's `SetText` lines (the first)
  or nothing, so ma01_026's 48 one-letter stretches take the document.

- **Pages in a choice's window (fixed 2026-10-15, shared with the
  Japanese build):** 23 pages after a vertical choice were drawn in the
  choice's window (`setwindow3 557,108,…,18`) at file labels reached by
  `goto` (`*f_mb02_008`): each file is compiled once, from the first
  path that reaches it, so the window tracking followed that path. The
  compiler now forgets the window at each file's start, and its first
  page sets its own (`compile_run`); `conscripter-convert lint` on the
  dry run lists no window problems.
- **Text past the box (fixed 2026-10-15):** each Japanese page holds
  several paragraphs, and 1,143 pages of English ran past the 280×150
  box. The user chose a CONscripter change over page breaks in the
  converter: text that would be drawn past the window's last line waits
  for a click and goes on on a new page (`pastPage`; the built-in wait,
  since `textgosub`'s return goes on after the line's next click wait
  and would drop the rest of the line).
