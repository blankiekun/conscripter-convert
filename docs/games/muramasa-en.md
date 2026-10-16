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
- **Title:** the port's (`sys_titlelogo.nut`): its pictures from the
  English `cg.npk` (`en/cg/sys/title/…` in the converter), the logo a
  full screen 40 px up, the six buttons in a row at y 467 (Start 158,
  Continue 280, Load 402, Extra 524, Config 646, Quit 768), the notes
  button leading to `secret.muramasa.jastusa.com`; the openings drawn
  with them (`opening::EN`). Not yet compared with the original.
- **Command menu:** the port's (`sys_menu.nut`, `command_menu_en`):
  fourteen rows from the English `cg.npk` at x 637 from y 90, no 選択肢に
  戻る, クイッセーブ/クイックロード in place of 栞を差す/栞を開く (no
  ロード: セーブ opens the save screen); rows and backdrop fade in and out
  together over 300 ms (the rows' wipe `メニューエフェクト` drawn as a
  fade); the keys printed on the rows (S quick save, L quick load, D the
  save screen, …). Checked headless, not yet against the original.
- **Choices:** in English on the port's plates (History). Choices of
  5 or 6 options (8 selects) are the plates in the window with the
  texts at their left (conv-cons has sprites for 4 picture-button
  options).
- **Line log:** the port's (`sys_backlog.nut`, History). Checked
  headless, not yet against the original.
- **Click icon:** at the port's places (History); not yet seen in the
  original.
- **Questions:** the port's (`sys_dialog.nut`, `confirm_en`): the
  question's picture from the English `cg.npk` (`en/cg/sys/dialog/`,
  400×150) centred over black at 800/1000, Yes and No 108,100 and
  230,100 from its corner; it turns open about its middle line over
  150 ms (`Rotate` x -90 → 0, drawn as a vertical stretch:
  `Unroll::y`) and shut as it goes. Checked headless. Not done: the
  port's quick-load question (`クイックロード`) and its messages
  (`クイックセーブ完了`, `記憶をしました`, …).
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
  1. The English pictures: the redesigned menus in `cg/sys`. The port's
     system scripts (`media/script/system/sys_*.nut`) give each picture's
     English place after a `LANG_NAME == "en"` test. Done: the title,
     the command menu, the questions, the choices, the line log and
     the click icon (above). Left: new
     screens: save/load (`save/`), settings (`config/`). Then film them against the
     English original under wine (also the lines per box).
  2. 22 rows of English are placed nowhere (lines the NSS orders or
     splits otherwise, route variants such as mc01's "Ichijo is
     nowhere to be seen" / "My comrades …", ma01_003's `<CLICK>`
     rows); ma01_026's document is shown whole after its typing
     sounds, not typed letter by letter as the port does.
  3. Gaps from the exe map ("The exe", coverage table): `<FW>` (25,
     the face window changing mid-page) is still dropped; `<SMALL>`'s
     size (75 %) is a guess; the port's centred subtitles (`ShowVOICESEText`, 32 calls in 10
     scenes); its larger-type text boxes (`SetTextEXC_Large` …, 16
     calls in 8 scenes).
  4. Gaps from the scripts' coverage ("The scripts' coverage"):
     `TransLog` (125), `TransVoice` (24) and
     `_system.nut`'s line-log strings not read; 15 scenes with English
     written into the scenario (`LANG_NAME` branches: captions, dates,
     ma01_026's document, mc02_031's menus) show the NSS's Japanese;
     four one-scene English boxes (ma01_009, mc01_003, ma01_002); the
     port's particles (rain, snow, sparks, 27 scenes) not drawn, as in
     the Japanese build. To check: `sys_backselect.nut` and
     `sys_key.nut`'s keys.

## Archives (NPK3)

`<game>/*.npk`: system, script, cg, dx, font, sound, voice, particle,
plus `patch/*.npk` mounted after them (opener 0x401960).

    python3 -I research/nitroplus/npk.py <game>/system.npk          # list
    python3 -I research/nitroplus/npk.py <game>/script.npk OUT [GLOB]

- Header: `NPK3`, u32 version 1, 16-byte IV, u32 entries, u32 index
  size; the index follows, AES-256-CBC.
- Key: eight dword immediates written at 0x401c28 and passed to the
  archive opener 0x54cf40 (AES-NI key schedules 0x554620, 0x554910); it is not in
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

## Scripts (Squirrel 2.2)

    python3 -I research/nitroplus/nut.py FILE.nut [--strings | --tables]

- Files: `SCRP`, u32 header size 0x10, two u32; then a Squirrel 2.2
  closure stream (32-bit SQInteger, 1-byte chars, UTF-8) whose header
  keeps only sizeof(SQChar), and whose functions end in u32 stack size,
  u8 generator, u8 varparams. All 1884 files parse.
- Opcodes are Squirrel 2.2.4's (the exe's VM; "The exe", below), which
  `nut.py` still names after 3.1: `LINE` to `GETK` (0-9),
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

## The exe (muramasa_en.exe)

    GHIDRA_HEADLESS_MAXMEM=2G systemd-run --user --scope -p MemoryMax=3G \
      research/tools/ghidra/ghidra_export.sh <game>/muramasa_en.exe \
      research/nitroplus/local/ghidra-en --names research/nitroplus/en-names.tsv --rtti

12,567 functions after the names (`research/nitroplus/en-names.tsv`,
1,806 rows; Muramasa.exe's are in `names.tsv`). The registrations were
read from objdump's disassembly, not by browsing (`docs/new-game.md` §2,
"Mapping the whole exe").

- **Map:** 0x401960 `app_main` (reads `system.npk`'s `system.dat`,
  opens the archives, starts Squirrel); the Diesel classes (RTTI names
  `Diesel::CText…`, `CTextTokenizer`, …); the natives 0x465260 …
  0x52f360; NPK3 and AES 0x54cf40 … 0x555e00; **Squirrel 2.2.4**
  (`"Squirrel 2.2.4 stable"`), sqstdlib (blob, regexp, io) and SqPlus
  0x555e00 … 0x57fe3f (`sqlib_*`, named in bulk; the API functions the
  registrations call have their names: `sq_newslot`, `sq_newclosure`,
  …); the CRT after it (`operator_delete`, `malloc_`, … named); SEH
  funclets 0x5c4000 … 0x5d4000. zstd and the vector maths are imports
  from `Mware.dll`. No `_ftol2`: the float conversions are inline SSE.
  The 17 code labels reached only through a pointer are `cb_<addr>`.
- **The VM is Squirrel 2.2.4, not 3.1:** the opcode puzzles in
  "Scripts" above are 2.2's numbering (17 `ARITH` with the operator as
  last operand, 19 `RETURN`, 20 `LOADNULLS`, 22 `LOADBOOL`, 30
  `NEWTABLE`). `nut.py` still names 3.1's opcodes; switching its `OPS`
  to 2.2's list is a research-tool fix, not done yet.
- **Natives** (`sq<Class>_<method>`, `sqfn_<name>`): each class has a
  registrar `sqreg_<Class>` (SqPlus `SQClassDef`) that calls a
  per-signature template as `reg(handler, "name", …)`; the globals are
  registered by `sqreg_globals` (0x46d890: `mov handler,%edx; push
  "name"`) and `sqreg_math` (`SQMATH_*`, vectors and quaternions). 26
  classes, 1,159 method handlers (SQSprite 211, SQSpriteMask 204,
  SQSpriteRenderTarget 197, SQModelCharacter 116, SQText 113, SQApp
  101, SQModel 99, SQScene 94, SQParticle 55, SQSprite3D 54, …, SQSound
  34) and 48 globals. Every method name occurs in the corpus, because
  `framework/nut/cnut*.nut` wraps each native one to one (`CNutSprite`
  holds an `SQSprite` in `m_Sprite`); the scenario never calls a native
  directly.
- **NPK3:** `npk_open` 0x54cf40 (key: eight immediates at 0x401c28),
  `aes256_expand_key` 0x554620 (aeskeygenassist), `aes256_decrypt_key`
  0x554910 (aesimc); the AES-NI CBC decryption is inline in `npk_open`
  and `npk_read_segment_a/b` (0x54e0a0, 0x54e470), then
  `ZSTD_decompress`.
- **Text:** the tags are split between Squirrel and the exe.
  `function_textbox.nut` defines `WAIT`, `WAITVOICE`, `FW`, `NW`
  (`DefineSingleTag`), `RED`, `RED2`, `RED3`, `YELLOW`
  (`DefineAreaTag`: `SetCharacterColor`/`SetRubyColor` on the range)
  and `VALUE`, `BR`, and handles `voice`, `I`, `K`. The native `SQText`
  gets `RUBY`, `SMALL`, `CENTER`: `text_tokenize_markup` 0x4c3440
  builds `<RUBY text='…'>`, `CTextEnglish_layout` 0x4a7f60 (horizontal
  English; breaks at spaces and CR/LF, skips `# { }`) takes `RUBY`,
  `SMALL`, `CENTER`, `CTextVertical_layout` and `CText_layout` only
  `RUBY` and `CENTER`. `AddNGCode_HeadOfLine`/`EndOfLine`/`ResetNGCode`
  (line-break rules) are registered but never called.

Coverage. The converter converts the Japanese NSS and takes only the
English text and pictures, so a native matters only where the port does
something the NSS doesn't:

| Feature | Corpus | The port | Converter |
|---|---|---|---|
| Scenario calls (`nut/*.nut`) | 635 names | Squirrel functions in `function/*.nut`; 430 carry the NSS's names, most of the other 205 wrap NSS constructs (`Print`/`PreStart`/`PreEnd` a `<PRE>` block, `SceneInit`, `CallSceneN2`, `RegisterCGvar`, `GetCheckReadFile`) | from the NSS |
| `ShowVOICESEText`/`ClearVOICESEText` | 32/18 calls, 10 scenes (ma04_016vs, ma04_023, ma05_012, mb02_002, mb02_018, mb02_027, mc04_001, md01_018, md02_038, md06_036) | English only: a centred line of text (`serif0`, faded in and out), by its name for voices played as sound effects | **not converted** |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/Huge/Big/Middle` | 16 calls, 8 scenes (mc03_010, md01_024, md01_030, md02_021, md03_008, md03_013, md04_061, md06_021) | English only: text boxes in larger type | **not converted** (the NSS's own box) |
| `<WAITVOICE=n>` | 95 | `WaitVoice("voice0", n)` mid-text | CONscripter's `!vn` (waits for the voice) |
| `</voice>` | 68 | ends a voice's range | dropped |
| `<FW>` | 25 | `SwitchFw`: the face window changes mid-page | **dropped** |
| `<WAIT=n>` | 16 | `WaitKey(n)` mid-text | `!dn` |
| `<RED2>` 14, `<RED>` 8, `<YELLOW>` 6 | 28 | the range's colour (`SetCharacterColor`) | `#RRGGBB` |
| `<SMALL>` | 8 | native: smaller glyphs (`CTextEnglish_layout`) | `!z075` (size a guess) |
| `RandomI` | 1,416 | native random integer | NSS `Random` |
| `CountTextLength` 18, `TokenizeText` 1 | | text box sizing, line log | CONscripter's layout |
| `FileDelete` 8, `GetSavePathRoot` 4, `Serialize`/`DeserializeRootTable` | | save screens, save data | CONscripter's saves |
| `SystemCall` | 4 | the title's web link | the converter's link |
| `SQMATH_*` | 22 | camera and 3D maths | from the NSS |

The full dry run's notes that the Japanese build lacks are the text tags
above and 75 more "code inside a page runs before it" (232 against 157).

## The scripts' coverage

The port's own scripts, counted statically (`docs/new-game.md` §2,
"Covering the scripts too"); the converter compiles none of them, so
each file's status says what stands in for it. The NSS that the build
compiles is the Japanese game's (its coverage belongs in
`docs/games/muramasa.md`).

    R=<script>/media/script           # script.npk extracted with npk.py
    python3 -I research/nitroplus/nut_refs.py $R tables
    python3 -I research/nitroplus/nut_refs.py $R calls                    # every called name
    { conscripter-convert nss builtins <nss> | grep -oE '^[A-Za-z_0-9]+:' | tr -d :
      grep -ohE '^\s*function\s+[A-Za-z_0-9]+' <nss>/*.nss | awk '{print $2}'; } > nss-names.txt
    python3 -I research/nitroplus/nut_refs.py $R calls --nss nss-names.txt   # port-only names

**Files** (script.npk 1884, system.npk, the other archives):

| Files | Status |
|---|---|
| `nut/*.nut` (884) | replaced by the NSS, line for line: 882 have an NSS file of the same name; `0_boot_start` is `0_boot.nss`; `md05_025cube` (the cube puzzle as a scene, touch input) is `function_magiccube.nss` (`magiccube.rs`). The NSS's `sys_*`, `extra_*`, `function_*` have no `nut/` file (the port moved them to `system/`, `function/`) |
| `lang/en/*.nut` (884) | read for data: `TransText` only (`english.rs`). **Not read:** `TransChoice` 403 rows in 130 files (every choice's English: the build's 153 `select`s stay Japanese), `TransLog` 125 rows in 24 files (the line log's text for lines shown other than as text), `TransVoice` 24 rows in 3 files (the `ShowVOICESEText` subtitles) |
| `lang/en/_system.nut` | **not read:** the line log's `SKIP_MESSAGE` (`——- SKIPPED ——-`) and choice lines (`SEL_10` … `SEL_40`: `- %s`, `[%s]`) |
| `function/*.nut` (18) | replaced by the NSS's `function*.nss`, except: `function_textbox.nut` (text tags, `CreateAllTextBox`) and `val.nut` (`BoxInfo`, `FwInfo`) read by hand into `EN_TEXT` and the box sizes (`muramasa.rs`); `val_cg.nut` (CG registration) from the NSS (`extras.rs`) |
| `system/boot.nut`, `sys_titlelogo.nut` | replaced: `opening::EN` (`opening.rs`) |
| `system/sys_menu.nut` | replaced: `command_menu_en` (`screens.rs`) |
| `system/sys_dialog.nut`, `sys_dialog_yes_no.nut`, `sys_close.nut`, `sys_totitle.nut` | replaced: `confirm_en` (`screens.rs`) |
| `system/sys_dialog_message.nut` | **not done:** the save screen's messages (`クイックセーブ完了`, …) |
| `system/sys_quicksave.nut`, `sys_quickload.nut` | CONscripter's quick save; **not done:** the quick-load question |
| `system/sys_saveload.nut`, `sys_writeload.nut`, `sys_after_load.nut`, `sys_load_error.nut` | replaced: `save_grid` (Japanese pictures; the English `save/` left) and CONscripter's saves |
| `system/sys_backlog.nut` | replaced: `line_log` (Japanese pictures; the English `backlog/` left) |
| `system/sys_config.nut` | replaced: `settings` (Japanese pictures; the English `config/` left) |
| `system/sys_extra*.nut` (6: main, cg, media, rec, turugi) | replaced: `extras` (`extras.rs`, Japanese layout); the English `extra/` left |
| `system/sys_backselect.nut` | **to check:** the port's back-to-choice question (`選択肢まで`), opened by a key in `sys_key.nut`; the English command menu has no row for it |
| `system/sys_hmode.nut` | from the NSS (`sys_hmode.nss`), opened by a key in `sys_key.nut` |
| `system/sys_key.nut` | CONscripter's keys; **to check** against its 36 `KEY_CODE_*` (the command menu's keys are done) |
| `system/sys_skip.nut`, `sys_cursor.nut`, `sys_function.nut`, `sys_val.nut`, `sys_val_product.nut` | engine state (skip, cursor, voice and skip toggles, globals): CONscripter's |
| `system/sys_tips.nut`, `sys_tips_val.nut` | not used: a glossary screen no script opens, its `cg/sys/glossary/` pictures aren't in cg.npk |
| `system/sys_dialog_windowsize.nut` | not used: no script opens it (window size: the PC window) |
| `system/zdbg_showval.nut` | not used: debug, no script opens it |
| `framework/` (59), `custom/` (4), `main.nut` | not used: the Diesel framework wrapping the natives ("The exe") and loading the rest; the converter has its own lowering of the NSS built-ins |
| `system.npk` `setting.ini` | read for data: `SCREEN_WIDTH`/`HEIGHT` 1024×576 (the NSS's too), `WINDOW_NAME` (`--title`), `[Font]` (`--font`); not used: `DOCKING`, `SOUND_TYPE`, `CODE_PAGE`, `VERSION`, `SAVE_PATH` (CONscripter's saves) |
| `system.npk` `shader/` (350 `.cso`, 4 `.bat`, a log) | not used: DirectX shaders (blend modes, post effects) |
| `cg.npk` | read: the English pictures where the converter asks for them (`NpaAssets::cg_en`) |
| `font.npk` | read: the text font (`--font`) |
| `particle.npk` (7 particles) | not used: the port's rain, snow and sparks (below) |
| `sound.npk`, `voice.npk`, `dx.npk` | not used: the Japanese build's sound, voices and movies stand in (same files) |

**Names the scenario calls** (`nut/`, 635 names, 221,476 calls): 430
carry the NSS's names (from the NSS); the other 205, by kind:

| Names | Calls | The port | Converter |
|---|---|---|---|
| `Print`, `PreStart`, `PreEnd`, `WaitKeyN2`, `SetOwnerN2`, `CallSceneN2`, `SceneInit`/`End`, `MainInit`/`End`, `GetCheckReadFile`, `RegisterCGvar`, `SetChoice`/`WaitChoice`, … (`function_n2`, `function`, `function_textbox`, `function_select`: 42 names, the English-only rows below among them) | 81,397 | the NSS's `<PRE>` blocks, scenes, read marks, CG flags and choices rewritten as functions | from the NSS |
| `St`, `FadeSt`, `DeleteSt`, `DeleteAllSt`, `FadeAllSt`, `Bg`, `InitBg`, … (`function_stand`, `function_bg`: 9) | 5,343 | the NSS's sprite helpers (`CreateTextureSP` …) | from the NSS |
| `framework/command` (53: `Scale` 4,346, `Transition` 1,317, `SetParameter` 314, `SetShadingPower` 162, `SetRasterScroll` 132, `SetBlendMode` 80, `SetMonochromePower` 78, `SetPriority` 73, `CreateSprite` 40, `CreateParticle` 39, …) | 7,126 | the natives' wrappers standing for NSS built-ins (`Zoom`, `DrawTransition`, `SetShade`, `CreateEffect`, …) | from the NSS |
| `*_begin` (82: `FlyMoving_begin` 48, `TurboBlur_begin` 24, …; 70 defined in their scene) | 276 | the NSS's `CreateProcess` functions started | from the NSS |
| `tointeger`, `tostring`, `find`, `append`, `slice`, `Import` | 54 | Squirrel's own | — |
| `CreateParticle` (27 scenes: mb01_006 …, md06_004) | 39 | particles from `particle.npk` (`雨_強`, `雪_前`, `火の粉01後`) where the NSS has `CreateEffect(…, "Rain")` | **not converted** (as in the Japanese build: a `CreateEffect Rain` note) |
| `ShowVOICESEText`/`ClearVOICESEText` | 32/18 | see "The exe" | **not converted** |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/…` | 16 | see "The exe" | **not converted** |
| `SetTextEXC_ma01_009`, `SetFwRR`/`SetFwLL` (ma01_009), `TypeBeginTimeCIY` (mc01_003, the box `C_Yoko`), `DeleteFwAll` (ma01_002) | 5 | English-only boxes and faces for one scene each | **not converted** (the NSS's own box) |
| `CreateTextArea` (ma01_009, ma01_026, mb04_015vs), `FadeCharacter` (ma01_026) | 10 | the NSS's `CreateText` in English (`《Even so―》`); ma01_026's document typed letter by letter | **Japanese text** (the NSS's string); ma01_026 shown whole |
| `SetRippleN2` | 4 | the NSS's `DrawEffect(…, "Ripple")` | from the NSS |
| `VCOpen`, `ToggleSkip`, `QuickMenuCreate`, `StartBoot`, `InitStatus`, … (system, boot and framework: 13) | 122 | voice unlocks (`SYSTEM_CV_OPEN`), skip, boot | from the NSS / CONscripter |

The system scripts' own calls: 252 names that only `system/*.nut` calls
(1,068 calls), all inside the replaced screens above.

**Scenes with English written into the scenario** (`LANG_NAME == "en"`
branches, 15 scenes), all **Japanese in the build** (the NSS's strings
or pictures): ma01_009, ma01_026 (the document), mb03_006, mb03_022
(`CEASE YOUR MINDLESS YAPPING, CUR!!!!!`), mb04_015vs (`CreateTextArea`
lines), mc02_031daidokoroa/aa (the menus: `horse meat`, `Lamb pilaf`,
…), md02_009 and md02_038 (`I AM POWER`), md04_002, md04_006,
md04_015, md04_026vs, md04_030 (dates and times: `November 30, 1940`,
`2:47 AM`), md04_048vs.

**Other vocabularies:**

| Vocabulary | Corpus | Converter |
|---|---|---|
| `SYSTEM_*` constants (24: BGV and voice fades, `SYSTEM_CV_*` voice unlocks, `SYSTEM_STATE*`) | 2-8 uses each | the NSS's `$SYSTEM_…` stand in (from the NSS) |
| `Request` actions | same names as the NSS's (`Smoothing`, `Start`, `Stop`, `Disused`, `Lock`, `UnLock`, `Clear`, …) | from the NSS |
| Text tags | "The exe" (coverage table) | there |
| `BLEND_MODE_*` (6), `TEXT_TYPING_TYPE_*` (`JPN`, `NOT_JPN`, `VERTICAL`) | framework constants the exe matches | the NSS's blend arguments; horizontal text in English |
| `KEY_CODE_*` (36) | `sys_key.nut` | the command menu's keys; the rest **to check** |
| `setting.ini` keys | files table | there |

## History

- **English text tags (2026-10-15):** `<WAIT=n>` becomes a timed wait,
  `<WAITVOICE=n>` a wait until the page's voice has played n ms
  (`TIMED_WAIT_MARK`/`VOICE_WAIT_MARK` in the IR; CONscripter's `!dn`
  and its new `!vn`), `<RED>`/`<RED2>`/`<RED3>`/`<YELLOW>` the port's
  colours (#FF0000, #CC0000, #990000, #FFFF00: `TAG_RED` … in
  `function_textbox.nut`), `<SMALL>` 75 % type (a guess). CONscripter
  now reads `!w`, `!d`, `!z`, `!v`, `!s` before a number and `#` before
  six hex digits in 1-byte text too (other `!` and `#` stay text), so
  the single-byte text carries colours and sizes. Checked headless (a
  test script; the trace of ma01_003 waits 5 s for "elaborate...").

- **Click icon (2026-10-15):** the port's `cg/sys/adv/改行改頁アイコン.png`
  is the Japanese 32 frames in one sheet (pixel for pixel), so only its
  place changes: `BoxInfo`'s English `icon_pos` (C 640,500; R 896,500;
  L 383,500; H 924,554; CH 776,580; Noh 776,329; the Japanese table
  follows it in `val.nut`), kept with each box in `EN_TEXT`
  (`Game::box_text`). Windows outside `EN_TEXT` keep the Japanese place.

- **Line log (2026-10-15):** `sys_backlog.nut` has no English branch:
  the port takes its own pictures of the same names (`cg/sys/backlog/`
  from the English `cg.npk`, `en/cg/sys/backlog` in the converter) at
  the Japanese places but 退出 at (906,443), and the rows from x 150,
  570 wide (`TextBox.backlogFieldRect`), Droid Serif 20, 10 px apart
  (`backlogRowSpace`): 28 full-width columns (`line_log(a, en)` in
  `screens.rs`). CONscripter's `logrows` now cuts a row of English at
  the space before the word that doesn't fit (for every script).

- **Choices in English (2026-10-15):** `english.rs` reads each scene's
  `SetChoice(id, japanese…)` and its lang file's `TransChoice["<id>_<n>"]`
  (n from 1, `function_select.nut`) and puts the English into the NSS's
  `SetChoice02` … `SetChoice06` by the option's Japanese: all 403
  options. They are drawn as the port's `LANG_NAME == "en"` branch
  draws them (`choice_style_en` in `muramasa.rs`): 440×50 plates
  (`cg/sys/select/en/select001.png`, `002` under the pointer) centred,
  100 px apart, the group centred on the screen, the text centred 13 px
  below a plate's top in black. Checked headless (ma02_003's choice);
  not yet against the original.

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
