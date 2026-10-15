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
  NSS (`english.rs`): 38,919 stretches of text replaced, 278 left in
  Japanese (mostly the 《…》 incantations; not looked at yet). The text
  still comes out full-width (`Ｓｕｚｕｋａｗａ`) and `<i>` is dropped.
- **Decided (the user, 2026-10-15):** the text font is the release's own
  `droidserif-muramasa.ttf` (font.npk); the menus follow the English
  port's redesigned screens, not the Japanese PC ones; filming the
  English original under wine is allowed.
- **Engine needs for English text:** a Latin font next to the Japanese
  `default.ttf` (or in its place), proportional character widths instead
  of the fixed grid, line wrapping at spaces, ASCII punctuation kept
  as is (not turned into full-width forms).
- **Plan (from the scripts, 2026-10-15):** the port's scenario scripts
  (`nut/*.nut`) are a line-by-line transcription of the Japanese NSS
  (same sprite names, coordinates, files with `.png` for `.jpg`; screen
  1024×576 in both). So the English build reuses the existing Muramasa
  converter on the Japanese NSS and swaps in the English text, joined
  by the Japanese text (below), and takes the English pictures where
  they differ (the redesigned menus in `cg/sys`, pictures with words).
- **Next:**
  1. The engine items above, then the converter's text emitter for
     English (no full-width conversion; `<i>`, `<center>`, `<K>`,
     `<br>`), with the release's `droidserif-muramasa.ttf` (font.npk).
  2. The 278 stretches left in Japanese.
  3. The English menus from `cg/sys` (1876 pictures), filmed against
     the English original under wine.

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
  nulls come out as `DIV`/`BITW`). The text join needs none of them; the
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
  `//` lines, tags and whitespace are removed. The misses are the 《…》
  incantation blocks in ma00_000 and similar, still to look at.
