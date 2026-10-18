# Full Metal Daemon Muramasa (English, JAST USA 2021)

The English release of 装甲悪鬼村正 is not a patch of the 2009 PC game
(`docs/games/muramasa.md`) but a port to Nitroplus's newer "Diesel"
engine (`muramasa_en.exe`, DirectX 11, Squirrel scripts, 1024×576).
It has no NPA archives and no NSS scripts.

## Status

Manual: `Manual.pdf` in the install (29 pages, English; screens: title
menu, main menu, quick menu, save/load, backlog, extra, config with
basic/text/sound/display tabs). Use it before asking for screenshots.
The write-ups of finished work are in History ("Status as it stood
before the 2026-10-18 prune" and the sections after it).

**Works** (details in History):

- The English text in the converter: `convert-nss <game> --file
  ma00_000.nss --chain 2000 --english <english game> …` reads the
  English release's `script.npk` into the Japanese NSS (`english.rs`);
  every stretch takes its English, none stay in Japanese; the full build
  plays to the end with the trace (25,430 pages). Pass the release's
  `droidserif-muramasa.ttf` as `--font`. English boxes, pictures
  (`cg.npk`), name plates, telops, subtitles, story captions, choices
  on the port's plates, and the port's `LANG_NAME` text-area lines are in.
- Port screens: title, command menu, choices, line log, save/load
  (compared with the original under wine), settings, questions, click
  icon, quick bar. The extras' menu, CG gallery, backlog, save/load are
  the engine's `screen`s (0 px against main's routines).
- Text box matches the original pixel for pixel in the text area
  (`ma00_000`); the text speed and volume curves are the exe's.

**Decided:** the text font is the release's own; menus follow the port's
redesigned screens; story `CreateText` without an English branch is left
out; a click on an empty slot at the title doesn't save (the original
does); Continue loads the quick save, else the newest; no closer volume
fit.

**Open:**

- `docs/games/muramasa-en.findings.tsv`: `Move`/`Zoom`/`Rotate` keep the
  Japanese curves (the port sends them through the same tempo table as
  the fades, "Eased fades" below); the English pictures taken only for
  the name plates, quick bar and telops (`ENGLISH_FIRST`) are inferred.
- Guesses a frame of the original could settle: the backlog's speaker
  icon at x 102 (no voiced row filmed); the save screen's thumbnails
  (158x90) and its 300 ms page fade; the Q.Save page shows our one quick
  save (the port keeps 20).
- Not yet compared with the original: command menu, subtitles, telops,
  quick bar, click icon, which quick-save notice the game takes
  (`sys_quicksave.nut`), boxes across (H, CH, Noh; 6 lines), backlog
  voice icon and scrolling, settings dragging.
- Not done: the character voices window (`OpenCVSetting`: 4 columns,
  x 118 + 204 i, y 69 + 32 row, the English `cg/sys/config/cv/` names)
  and the text sample; `クイックセーブ中`, `読み込んでいます`, `削除しました`,
  `ロック通知`; faces of `FwB`/`FwCH` keep the Japanese place (the port's
  `FwInfo` has none).
- Metrics: 4 "text window moved inside a page: kept" notes; the
  English-side `Metrics::left_out` entries (menu built-ins the English
  screens replace) are not yet listed.
- ma01_026's document is shown whole before its key sounds; the port
  types it letter by letter (`CreateTextArea` + `FadeCharacter`).
- 22 rows of English placed nowhere (lines the NSS orders or splits
  otherwise: route variants such as mc01's "Ichijo is nowhere to be
  seen" / "My comrades …", ma01_003's `<CLICK>` rows).
- Gaps from the exe map and scripts' coverage ("The exe", "The scripts'
  coverage" below): the larger-type boxes (`SetTextEXC_Large` …, 16 calls
  in 8 scenes); `TransLog` (125) and `_system.nut`'s line-log strings;
  the telops the port rewrote (md05_037vs's 魔剣対決 and 魔王編挿入
  pictures, mc01_026's last 殺意 word) stay at the Japanese places;
  four one-scene English boxes (ma01_009, mc01_003, ma01_002); the
  port's particles (27 scenes) not drawn, as in the Japanese build; the
  port's calls taken in part (the build's `English: … port <function>
  argument <i> ignored` lines, "The port's calls, argument by argument"),
  most noticeable first. To check: `sys_backselect.nut` and `sys_key.nut`'s
  keys.

## Executables

The builds the decompile addresses in this document belong to.

| File | Version | sha256 |
|---|---|---|
| `muramasa_en.exe` | unknown | `5a174f7037f021ec22f228869b3d0e62ca143dcdc051597fe4a378c26ba0b4b9` |

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

## Eased fades

The port's scenes pass a tempo to `Fade(name, time, dst, tempo, wait)`
and `Transition(name, time, src, dst, rate, file, tempo, wait)` as a
global looked up at run time (`GETK 'Dxl1'`). The globals are
`framework/constantvalue.nut`'s constants, which the compiled file
keeps after its main function as a table (u16 length with the NUL, the
name, NUL, `0x05000002`, the int32 value): `Linear` 0, then `Axl1` 1,
`Dxl1` 2, `Axl2` 3, … `Dxl8` 16, `AxlDxl1` 17, `DxlAxl1` 18, …
`DxlAxl8` 32, and the same 32 again with `a`, `b`, `c`, `d` (33-160).
Paired call by call with the NSS (same scene, sprite and order:
`python3 -I research/nitroplus/en_tempos.py <nss dir> <script dir>`,
with the extracted `nss.npa` and `script.npk`'s `media/script`), the
port writes the NSS's tempos as:

| NSS | Port | Id | Curve (share of the way at time share t) |
|---|---|---|---|
| `null` | `null` | 0 | t |
| `Axl1`, `Axl2`, `Axl3` | the same | 1, 3, 5 | t², t³, t⁴ |
| `Dxl1`, `Dxl2`, `Dxl3` | the same | 2, 4, 6 | 1-(1-t)², ³, ⁴ |
| `AxlAuto` | `Axl1d` | 129 | 1 - cos(πt/2) |
| `DxlAuto` | `Dxl1d` | 130 | sin(πt/2) |
| `AxlDxl` | `AxlDxl1` | 17 | (2t)²/2, then 1 - (2-2t)²/2 |
| `DxlAxl` | `DxlAxl1` | 18 | (1 - (1-2t)²)/2, then (1 + (2t-1)²)/2 |

In the exe the natives `sqSprite_fade` (0x4eb0f0) and
`sqSprite_transition` (0x4eb320, through 0x435760) store the time, the
end value and the tempo (0 past 160) in an interpolator; its step
(0x40bca0 for a fade, 0x437cc0 for a transition) calls
`table[tempo](from, to, time, elapsed)` from the 161-entry table at
0x5e40a0 (0x40c860 linear, 0x40c9c0 `Axl1`, 0x40ca00 `Dxl1`, …,
0x40cec0 `AxlDxl1`, 0x40cf50 `DxlAxl1`, 0x412670 `Axl1d`, 0x4126d0
`Dxl1d`; the last two call the CRT's `cosf`/`sinf`). The NSS names
match Muramasa.exe's curves (`tempo` in `convert.rs`) except `AxlDxl`
and `DxlAxl`, where Muramasa.exe has a cosine and an arccosine
(`diesel_tempo` in `muramasa.rs`).

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
| `ShowVOICESEText`/`ClearVOICESEText` | 32/18 calls, 10 scenes (ma04_016vs, ma04_023, ma05_012, mb02_002, mb02_018, mb02_027, mc04_001, md01_018, md02_038, md06_036) | English only: a centred line of text (`serif0`), by its name for voices played as sound effects | converted (Status, "Subtitles") |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/Huge/Big/Middle` | 16 calls, 8 scenes (mc03_010, md01_024, md01_030, md02_021, md03_008, md03_013, md04_061, md06_021) | English only: text boxes in larger type | **not converted** (the NSS's own box) |
| `<WAITVOICE=n>` | 95 | `WaitVoice("voice0", n)` mid-text | CONscripter's `!vn` (waits for the voice) |
| `</voice>` | 68 | ends a voice's range: `Print` (function_textbox.nut) empties `voice_name`/`class`/`file` for the text after it; the voice plays on | nothing to do (seen; no note) |
| `<FW>` | 25 | `SwitchFw`: the face window changes mid-page (24 `img=''`: the face goes and the text goes on in the box without one; md05_033's one picture brings the face back) | a new page in the box without a face (`EN_TEXT`'s pairs) until the next code; a picture: the face the NSS has set |
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
| `lang/en/*.nut` (884) | read for data: `TransText` only (`english.rs`). **Not read:** `TransChoice` 403 rows in 130 files (every choice's English: the build's 153 `select`s stay Japanese), `TransLog` 125 rows in 24 files (the line log's text for lines shown other than as text). `TransVoice` (the `ShowVOICESEText` subtitles) is read |
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
| `ShowVOICESEText`/`ClearVOICESEText` | 32/18 | see "The exe" | converted (Status, "Subtitles") |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/…` | 16 | see "The exe" | **not converted** |
| `SetTextEXC_ma01_009`, `SetFwRR`/`SetFwLL` (ma01_009), `TypeBeginTimeCIY` (mc01_003, the box `C_Yoko`), `DeleteFwAll` (ma01_002) | 5 | English-only boxes and faces for one scene each | **not converted** (the NSS's own box) |
| `CreateTextArea` (ma01_009, ma01_026, mb04_015vs), `FadeCharacter` (ma01_026) | 10 | the NSS's `CreateText` in English (`《Even so―》`); ma01_026's document typed letter by letter | **Japanese text** (the NSS's string); ma01_026 shown whole |
| `SetRippleN2` | 4 | the NSS's `DrawEffect(…, "Ripple")` | from the NSS |
| `VCOpen`, `ToggleSkip`, `QuickMenuCreate`, `StartBoot`, `InitStatus`, … (system, boot and framework: 13) | 122 | voice unlocks (`SYSTEM_CV_OPEN`), skip, boot | from the NSS / CONscripter |

The system scripts' own calls: 252 names that only `system/*.nut` calls
(1,068 calls), all inside the replaced screens above.

**Scenes with English written into the scenario** (`LANG_NAME == "en"`
branches, 15 scenes): their `CreateText` in English (`put_captions`:
mb03_006, mb03_022, md02_009, md02_038, md04_*, md04_048vs), the rest **Japanese in the build**
(the NSS's strings or pictures): ma01_009, ma01_026 (the document), mb03_006, mb03_022
(`CEASE YOUR MINDLESS YAPPING, CUR!!!!!`), mb04_015vs (`CreateTextArea`
lines), mc02_031daidokoroa/aa (the menus: `horse meat`, `Lamb pilaf`,
…), md02_009 and md02_038 (`I AM POWER`), md04_002, md04_006,
md04_015, md04_026vs, md04_030 (dates and times: `November 30, 1940`,
`2:47 AM`), md04_048vs.

**The port's calls, argument by argument.** The converter takes a few
of the port's calls' effects from the port itself (the others come from
the NSS); `english.rs` (`port_reads`) records which arguments it reads
and the build prints one line per argument passed and not read
(`UNREAD_HARMLESS` would list the harmless ones; none yet).

| Call | Argument | Converter |
|---|---|---|
| `ShowVOICESEText(sound, add)` | 0 sound | converted (finds the voice it follows) |
| | 1 add | converted (a line under the one shown) |
| `SetChoice(id, option, …)` | 0 id | converted (`TransChoice["<id>_<n>"]`) |
| | options as strings | converted (Japanese → English) |
| | options as variables (`Route.<name>`) | converted (the values the port puts in them, History) |
| | options as constants (ma05_021vs's debug menu) | harmless: names the port never defines (`UNREAD_HARMLESS`) |
| `Print(number, text)` | 0, 1 | converted (`TransText[number]`, the Japanese to align) |
| `CreateTexture…(name, priority, x, y, picture)` on a telop | 0 name | converted (pairs the sprite) |
| | 1 priority | **partial**: the NSS's |
| | 2 x, 3 y, 4 picture | converted (the port's places, its telop picture) |
| `Move(name, time, x, y, tempo, wait)` on a telop | 0 name | converted |
| | 1 time, 4 tempo, 5 wait | **partial**: the NSS's |
| | 2 x, 3 y | converted |

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
### Status as it stood before the 2026-10-18 prune

Kept whole; the open items are in Status.


Manual: `Manual.pdf` in the install (29 pages, English; screens: title
menu, main menu, quick menu, save/load, backlog, extra, config with
basic/text/sound/display tabs). Use it before asking for screenshots.

#### Exe mapping redone (2026-10-17)

`docs/games/muramasa-en.findings.tsv`: the port's rows (tempo table, text,
choices, pictures, subtitles) and the NSS built-ins it shares with the
Japanese build (`muramasa.findings.tsv`). Open: `Move`/`Zoom`/`Rotate`
keep the Japanese curves. No names carried between muramasa_en.exe and
Muramasa.exe: different engines (Diesel/Squirrel against NSS).

Findings audit (2026-10-18, `docs/new-game.md` step 17): all 10 rows
re-derived from the decompile, the compiled scripts (`nut.py`) and the
archives: 8 confirmed (the tempo table's `AxlDxl1` 17 and `DxlAxl1` 18
at 0x40cec0/0x40cf50, the text fade's 212.5 and 5.3125 at 004ad32f, the
440×50 choice plates), one stale (`ShowVOICESEText` is in
`function.nut`), one unsupported: `CreateTextureEX`/`SP` have no
language test; the English pictures are `cg.npk`'s under the Japanese
names, and ours takes them only for the name plates, quick bar and
telops (`ENGLISH_FIRST`): now inferred.

#### Accuracy (2026-10-17)

The port's hand-typed values (`muramasa.rs`'s English parts,
`screens.rs`'s `*_en`, `subtitles.rs`, `english.rs`) carry a
`// src:`, `// guess:` or `// ours` comment; the places and defaults
are the Squirrel scripts' (`val.nut`, `sys_*.nut`,
`function_textbox.nut`, `function_select.nut`), checked against the
disassembly. The Japanese build's audit (shared values, 一条's mask)
is in its own document. Open guesses a frame of the original could
settle:

- the backlog's speaker icon at x 102 (no voiced row filmed);
- the save screen's thumbnails (158×90) and its 300 ms page fade.

Settled from the exe (2026-10-17): the text speed (`004b4510`: s =
text_speed ÷ 1000, a character fading in at (212.5 − 5.3125)(1 − s)⁴ +
5.3125 alpha per 16 ms frame, the next 50 alpha later; 0 instant, 58 ms
at the default 550, 150.6 ms at 1000; the slider's `TextSpeedCurve`);
`<small>` at 50 % (`CTextEnglish_layout` draws SMALL with the text's
ruby font, `function_textbox.nut`'s fonts size 20, ruby_size 10).

#### Eased fades (2026-10-17)

Open: `Move`, `Zoom` and `Rotate` go through the same tempo table in the
port ("Eased fades", below), so their `AxlDxl` and `DxlAxl` would be the
port's curves too; they keep the Japanese exe's.

#### Engine-drawn screens (2026-10-17)

The extras' menu is the Japanese build's `screen "extras"` (the same
`extras.lua`; "Extras' menu drawn by the engine" in muramasa.md): checked
on this build against main's routine (open, hover, the gallery, back,
the title: 0 pixels). The save screen (popup, locks, tabs, quick page),
the backlog (single-byte rows) and the extras' modes other than the
gallery's pages are still script routines.

The CG gallery's pages are the Japanese build's `screen "gallery"` (the
same `gallery.lua`; "CG gallery drawn by the engine" in muramasa.md),
checked on this build against main's routine: 0 pixels (1 px on an
arrow).

#### Queue (2026-10-16)

- English-only gaps in `metrics.json` (this PR): the 444 `CreateText`
  notes were all `function_select.nss`'s choice labels (the choice is
  the engine's, in English), not story captions; they and the choice
  screen's built-ins (`CreateChoice`, `SetNextFocus`, `Position`,
  `LoadImage`, `CursorPosition`, `MoveCursor`, `Save(9999)`; seen in
  `SetChoiceBase`/`ChoiceFocus`/`SelectSave`) now count as left out
  (`CHOICE_SCREEN`), in both builds and Django's; `</voice>` (68) is
  seen to change nothing visible. Gaps 31,735 → 27,552, `0.txt`
  unchanged. Left: 4 "text window moved inside a page: kept" (the C
  box moved mid-page in function.nss where the English page goes on),
  the larger-type boxes (`SetTextEXC_Large` …, port-only, not in the
  metrics).

- Merged this round: text box (#20), settings (#21), line log (#23,
  engine #17), save/load (#26), title (#29), subtitles over the window
  (#35, engine #21), `Labels` moved out of mages (#38), story
  `CreateText` left out with a note (#42), volume curve already the
  port's (#44), choices from variables take the port's English (#53).
- Merged since: #54 (choice options keep their places past a
  variable), #60 and #66 (shared NSS: choice options set in runtime
  branches, and their line-log entries, picked at run time).
- `―` (SJIS 0x815C) showed the missing-glyph box: the engine maps it
  to U+2500, which the release font lacks (it has U+2015). The engine
  now draws U+2015 when the font has no U+2500 (engine PR #60); the
  converter is unchanged. Other characters absent from the font are
  only the Japanese marks in the Japanese text the build still carries.
- Telop `Move` timing (this PR): of the 252 telop `Move`s paired with
  the port's, 251 have the same time, tempo and wait (155 null tempo,
  96 `Dxl1`; waits equal). The only difference: ma04_021, 250 ms in the
  port for the NSS's 500 (`mov %4,250` in `0.txt`, the one line that
  changes). `Move`'s time and wait now come from the port
  (`english::placing`); a null tempo stays the NSS's.
- Telop priority (stacked on the `Move` PR): of the 214 telop
  `CreateTextureEX`/`SP` calls, 212 pair with the port's; 208 have the
  same priority, md05_037vs's `エフェクト１`/`３`/`４`/`５` are 16000 in
  the port for the NSS's 2000. `english::placing` now takes argument 1
  too: those telops go to sprite 482 for 902, still in front of the
  black and the CG under them.
- The local full build of 2026-10-15 is stale: it lacks the telop
  pictures (`cg/sys/telop/…`) that a fresh build packs (checked with a
  one-scene build of ma04_021). Rebuild before comparing pictures.
- Story captions in English (#158): the port's `LANG_NAME == "en"`
  branches draw their own `CreateText` (dates and times in md04_002 …
  md04_030, `I AM POWER` in md02_009/038 and md04_048vs, `MURDERED` in
  mb03_006, `CEASE YOUR` / `MINDLESS YAPPING, CUR!!!!!` in mb03_022).
  `english::put_captions` gives the NSS's `CreateText` the English of
  the port's call on the same sprite (the k-th one whose Japanese is the
  same): its x, y (Center, Middle), the box `auto`, drawn across in the
  release's font at the NSS's `SetFont` size; the `Rotate` after it is
  undone and the first `Move` takes the port's offset (two-line dates:
  -25/+25). A Japanese line the English branch has no line for deletes
  the sprite (md04_002's second date). 20 calls in the dry run; checked
  on a one-scene build of md04_002 (`2:47 AM` at 450,268, the two lines
  centred 50 px apart). Story `CreateText` elsewhere stays left out
  (Decided). The text blocks' `{…}` code is now walked too (only
  mb03_022's captions are there; telops and choices unchanged).
- Pictures over a frozen screen (#165, shared NSS): a `Plain` cover
  (`CreatePlainSP`) froze the whole screen, so what a scene drew above
  it never showed: mb03_022's captions and black bar (over
  `絵板写` at 30000), the white flash over ma01_002's shaking capture,
  ma00_000's split windows over `絵垣見側`. Now only what lies under the
  top opaque cover stays as it was; pictures above it change (`sync`).
  Both builds change (the Japanese one has 635 more `print`s). Checked
  on a one-scene build of mb03_022 with the trace (`--shot-at` after
  the captions' `print`): `CEASE YOUR` / `MINDLESS YAPPING, CUR!!!!!`
  centred over the bar. Left: the text window is drawn over them (the
  engine's window is above every sprite; in the original it is part of
  the capture under them; fixed since by #173).
- mb03_022's bar (this PR): the black bar behind `CEASE YOUR` /
  `MINDLESS YAPPING, CUR!!!!!` is now the port's: its English branch
  scales `絵色100` to 620×220 (`Scale` in mb03_022.nut) where the NSS
  stands it 500×1000 (`call_en` in `muramasa.rs`, the game's one such
  `Zoom`). In the dry run it is a 635×127 bar at 194,224, centred
  under the captions (the only change).
- Text-area lines (#166): the port's `CreateTextArea` + `SetText`
  lines (mb04_015vs: `《Even so―》`, `《That is justice!》`) pair with the
  NSS's `CreateText` by their Japanese (the port names the sprite
  `add_text01` for the NSS's `text02`), at the area's place and width
  (R box, 630,190, 280 wide). ma01_009's came with its boxes (below).
  Checked on a one-scene build of mb04_015vs with the trace: the lines are placed, but two problems hide them
  (not this PR): the engine's text window is drawn over them, and with
  two boxes up the window takes the first box in `EN_TEXT` (R), not the
  one `SetTextEXL` names (L), so the page's text lands on the caption.
- Pictures over the text window (#173, shared NSS): a picture above
  the window the text is in (`<PRE @box0>`: `CreateWindow("box0",
  20001, …)`) in priority, or as high and made after it, is drawn over
  the text window and its text (CONscripter's `spovertext`), as the exe
  draws by priority: mb03_022's captions and bar, mb04_015vs's text-area
  lines, name plates, flashes and blackouts over the window. Only 3
  pages in the Japanese build show their text under an opaque picture
  that covers the screen (mc04_022vs's `絵演弓聖終` at 29000). Checked
  on a one-scene build of mb03_022 with the trace: the captions over
  the box.
- Two boxes up (this PR): the text goes in the box the last
  `SetTextEX…` names (`SetTextEXL`: `boxLT`/`boxLL`), not the first
  shown one in `EN_TEXT` (`call_en` keeps it in `text_box`): mb04_015vs's
  two pages move from R (630) to L (118), the caption staying in R (the
  only change in the dry run).
- ma01_009's three boxes (this PR): where the NSS calls its only
  `SetFwR`/`SetFwL` the port calls `SetFwRR`/`SetFwLL`: the side boxes
  stand at x 690 and 28 (`FwInfo` RR/LL), beside C at 360, until the
  next `SetTextEXR`/`SetTextEXL` puts them back at 616 and 103
  (`place_side_box` in `muramasa.rs`); a face sits 92 px into its box
  (`en_face`). Its two text-area lines (`"The darkling."`, 43,190 and
  705,190) are now taken (`OWN_BOXES` gone): 24 captions. Checked on a
  one-scene build with the trace: the three boxes side by side, a face
  and the line in each; the next page in R at 616.
- mc02_031's menus (this PR, shared NSS): a page showing a variable
  (`<VALUE name=$料理名前>`; the port's `<VALUE='料理名前'>` too) that a
  runtime flag holds is given once for each text the flag can hold,
  behind `if %flag != code` tests, and once without it (the existing
  `If`/`Jump`/`Label` ops; labels named by file and page, so no other
  label moves). The English values come from the port's slots as the
  choices' do (`slot_strs` reads `=` as well as `<-`), its Japanese
  values beside them left out: the dishes ("Here you are. Sauteed
  pork.", 14) and the ingredient ("stocked with pork."). No variable
  or code moves (the dry runs' `mov`/`if` lines against main: 15
  added, none changed), so saves keep working. The ingredient is
  always the same: its `Random` is picked at convert time (both builds;
  the picks reach text, see muramasa.md "`Random`"), so the
  dish is too unless another path sets the flags. `mc02_031time`'s
  `$mc02_時間` (set in `function_process.nss`) is not a story flag: as
  before.
- Code inside a page (this PR, shared NSS; engine PR #63, merged: CONscripter 7db2f06): a `{…}`
  block between a page's lines, or inside a line, now runs where it
  stands. The text before it shows, the code runs, and the page goes
  on: on the same line when the code is inside a line or after a `<?>`
  (`/`; in the English build `` `…`/ ``, which CONscripter now reads),
  otherwise on the next line. A face changed there swaps only the
  window's picture (`setwindow2`, `Op::WindowImage`; 126 in the
  English dry run); 5 windows that move inside a page stay where the
  page began. In the Japanese builds ma01_026's document is typed one
  letter at a time with its key sounds (checked on a one-scene build
  with the trace). The English build still shows it whole before the
  sounds, because the port's English for it is one stretch (its
  `CreateTextArea` + `FadeCharacter` are not done). The dry runs of
  Muramasa (both builds) and Django change: sounds and waits move into
  the page where they stand; line ends `/` (Muramasa 102, Django 7),
  `` `…`/ `` (English 43); `setwindow2` (Muramasa 11, English 126).
- Open (2026-10-16): The English-side
  `Metrics::left_out` entries (the 444 story `CreateText` left out on
  purpose, menu built-ins the English screens replace) are not yet listed.
- Decided: a click on an empty slot at the title doesn't save (the
  original saves the title screen there): a deliberate difference.
  Continue loads the quick save, else the newest save. Japanese story
  captions stay skipped; no closer volume fit.

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
  become horizontal boxes, 280 wide, line pitch 25, 12 lines, in the
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
  with them (`opening::EN`). As in the original under wine (and its
  script): Continue and Load only while there is a save, Extra once
  英雄編 or 復讐編 is over (`TitleScreen::hide_without_saves`,
  `extras_shown_by`); the button row matches it pixel for pixel at a
  first start and with a save. Continue loads the quick save, else the
  save screen's newest (the port: its newest save of either kind).
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
- **Line log:** the port's (`sys_backlog.nut`, History). Compared with
  the original under wine (the prologue's first five pages): the rows
  break, start and stand where the port's do, to the pixel: rows 570 px
  wide measured letter by letter (CONscripter's `logrows %n,28,570`,
  engine PR #17), 34 px apart from y 22, 14 rows, a paragraph's
  indent kept (each row `strsp`'d after a backquote). Not compared: the
  voice icon's place, scrolling. Drawn by CONscripter's `screen
  "backlog"` since 2026-10-17, as the Japanese build's (`list source =
  "rows"`; engine #107 draws a row of half-width text after a backquote
  and keeps a text's double quotes). Compared headless with main's
  routine (backlog opened after pages 40 and 119, `--shot-effects 0`
  against `--shot-at`): 0 px different, quoted lines and voice icons
  included; scrolling (a row a click, the wheel, the knob) is the
  engine's, checked in the trace (main's routine can't scroll under
  auto-play: it acts on `isdown`).
- **Save/load:** the port's one screen for both (`sys_saveload.nut`,
  `save_grid_en`): the page pictures `ナンバリング001`-`004` with
  `Slots BG`, 15 slots 5×3 under `Slots-Grid`, `未選択時` in the empty
  ones, tabs 1-15 … 46-60 and Q.Save at the top (the current one in its
  pressed picture), Exit at 0,0; a click on a slot opens `menu_window`
  over it with Save, Load, Delete and Lock/Unlock (Save off on a locked
  slot, at the title and on the Q.Save page; Load and Lock off on an empty
  slot; Delete off on an empty or locked one); a lock is a global per
  slot (`%1011`-`%1110`) and shows `locked.png`; after saving,
  `記憶をしました` until a click. Shared `SaveGrid` fields (`tabs`,
  `popup`, `empty`, `quick_page`, `play_time_prefix`). Checked headless
  (`--choose` on `*conv_sg_wait`, `--shot-at`) and against the manual's
  screenshot (page 6 of `Manual.pdf`: same layout and popup), and
  against the original under wine: the screen at rest, the popup on a
  saved slot (its buttons' places), `Save Complete` and the details under
  the pointer match. As there, a click on an empty slot saves at once in
  the game (no popup; `SlotPopup::empty_saves`) and the play time is
  hours and minutes (`PLAY TIME 00:16`, `SaveGrid::play_time_hm`). A
  deliberate difference (decided 2026-10-16): at the title a click on an
  empty slot does nothing (the port saves the title screen there, though
  its popup's Save is off at the title). Left as it is: the Q.Save page shows our one quick save (the port keeps 20,
  oldest overwritten).
  Drawn by CONscripter's `screen` since 2026-10-17 (`save.lua`,
  `load.lua`, the slot's window `save_popup.lua`; `save_grid_screen.rs`,
  engine #105: a widget's `when` for the lock marks, a save from the
  window takes the screen's shot and text): a slot click runs
  `*conv_sg_slot` (saved? locked?), then saves at once or opens the
  window; tabs are page buttons (the current one disabled, its pressed
  cell); the Q.Save page is a one-slot widget of its own (slot 890).
  Compared headless with main's routine (`--choose`, `--shot-effects
  100` / `--shot-at`; save, notice, popup, lock, popup on a locked slot,
  delete asked and refused, tabs, Q.Save, the load screen's popup):
  every frame the same to the pixel, but Exit's hover cell where
  auto-play's pointer rests (853 px). Different from the routine: a
  click outside the window (another slot too) only closes it; after a
  refused delete the window stays open; the play time is read when the
  screen opens.
- **Settings:** the port's (`sys_config.nut`, `settings_en`):
  `ConfigBG` with its check boxes (skip movies, wait for the voice in
  auto mode, the three mutes), ON/OFF pairs (skip unread, skip the voice
  on a click, stop auto and skip at a choice), five lying gauges lit up
  to the knob (auto speed, text speed, music, effects, voices), Restore,
  the voice test and Close, at the places `main` gives them (shared:
  `PictureSlider::horizontal`/`fill`, check boxes as a pair at one
  place). What CONscripter can't set is drawn as a first start leaves it
  (auto save at choices, read text's colour, a click stopping auto,
  syncing text to voices, the effects test, the display options).
  Checked headless (`--shot-at` on `*conv_ps_wait`) against the manual's
  screenshot (page 9: the same states and places), and against the
  original under wine at a first start: the same, the gauges at a first
  start's values (`sys_val.nut`: auto 60, text 550, music 800, effects
  1000, voices 700; the knobs within 4 px). The volumes follow the
  port's curve: its `Mware.dll` (`CSoundXAudio2Player`, the setter at
  0x1006d830: `log10f`, ×33.22, ×0.05, `powf(10, …)`, then the voice's
  `SetVolume`) plays a volume v (0-1) at 10^(33.22·log10 v / 20) =
  v^1.661, each halving 10 dB down: the law conv-cons already
  approximates for the Japanese exe ((p²/50 + p)/3 for p = 0-100:
  within 0.4 points of it above half volume, at most 2.4 below; music
  800 plays at 68 %, the port's 69 %). Not compared: dragging. Not done: the
  character voices window (`OpenCVSetting`: 4 columns, x 118 + 204·i,
  y 69 + 32·row, the English `cg/sys/config/cv/` names), the text sample.
- **Name plates:** the English `cg.npk`'s `cg/fw/nw*.png` (the names
  in English, same names and size as the Japanese) are taken before
  `cg.npa`'s (`ENGLISH_FIRST` in `nitroplus/assets.rs`). Checked on a
  one-scene build (`--file ma01_001.nss --chain 1 --loose`).
- **Subtitles:** the port's subtitles for voices played as sound
  effects (`ShowVOICESEText`, 31 voices in 10 scenes, History) are
  centred lines in the release's font, at the top (y 13) or the bottom
  (y 480) as the port puts them, until the voice ends. Drawn over the
  text window as the port does (`Op::SpriteOverText`, CONscripter's
  `spovertext`, engine PR #21): 37 calls in the build; most lines are
  at the top (y 8-9), above the boxes, so it shows where a bottom line
  meets a box. Checked headless on md02_038 and mb02_002. Not yet compared with
  the original. They change on 250 ms steps; a click that cuts the voice
  short leaves the line up until the voice would have ended.
- **Telops:** the English `cg.npk`'s `cg/sys/telop/` pictures (chapter
  titles, prefaces, 銀星号の唄 captions, names) are taken before
  `cg.npa`'s, at the port's places (`english::place_telops`, History).
  The build prints the calls it left at the Japanese places (7: md05_037vs's
  rewritten effect sprites, mc01_026's `絵文字殺覆` #5). Checked headless
  on ma01_026 (preface, then the title centred). Not yet compared with
  the original.
- **Quick bar** (the fly-out on the right): the English `cg.npk`'s
  `cg/sys/quick/` (Q.SAVE, Q.LOAD, SKIP, HIDE TEXT on its base; the
  buttons transparent pictures with their highlights), at the Japanese
  places, which `boot.nut`'s `QuickMenu` keeps (x 998; y 47, 180, 290,
  390). Not yet compared with the original.
- **Click icon:** at the port's places (History); not yet seen in the
  original.
- **Questions:** the port's (`sys_dialog.nut`, `confirm_en`): the
  question's picture from the English `cg.npk` (`en/cg/sys/dialog/`,
  400×150) centred over black at 800/1000, Yes and No 108,100 and
  230,100 from its corner; it turns open about its middle line over
  150 ms (`Rotate` x -90 → 0, drawn as a vertical stretch:
  `Unroll::y`) and shut as it goes. Checked headless. A quick load asks
  `クイックロード` first (`sys_quickload.nut`); after a quick save
  `クイックセーブ完了` shows over the dimmed screen until a click
  (`sys_quicksave.nut`'s way that waits; its other closes by itself:
  which one the game takes is not yet compared with the original);
  after a save on the save screen, `記憶をしました`. Both checked
  headless. Not done: `クイックセーブ中`, `読み込んでいます` (shown while
  saving and loading; ours are instant), `削除しました`, `ロック通知`.
- **Decided (the user, 2026-10-15):** the text font is the release's own
  `droidserif-muramasa.ttf` (font.npk); the menus follow the English
  port's redesigned screens, not the Japanese PC ones; filming the
  English original under wine is allowed.
- **Decided (2026-10-16, the lead for the user):** story `CreateText`
  (the Japanese build draws it since #17: captions, dates, 殺 害,
  〝我は力〟) is left out in the English build, which would show the
  Japanese (the dry run's `CreateText left out` note: 0 calls now,
  every story caption takes the port's English or goes; the 444 calls
  it counted were the choices' labels, below); the
  title's empty-slot save is not copied (Save/load, above); Continue
  loads the quick save, else the newest.
- **Text box** (compared with the original under wine, History): the
  first pages of `ma00_000` match it pixel for pixel in the text area
  (line pitch 25, the text 2 px under `BoxInfo`'s y, no letter
  spacing). The boxes across (H, CH, Noh) take the same pitch (6 lines
  in their 150 px field), not yet seen in the original.
- **Not yet:** the
  faces of `FwB`, `FwCH` keep the Japanese place (the port's `FwInfo`
  has none for them).
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
     the click icon, the save/load and settings screens (above).
     Then film them against the
     English original under wine (also the lines per box).
  2. 22 rows of English are placed nowhere (lines the NSS orders or
     splits otherwise, route variants such as mc01's "Ichijo is
     nowhere to be seen" / "My comrades …", ma01_003's `<CLICK>`
     rows); ma01_026's document is shown whole, then its typing
     sounds play; the port types it letter by letter (`FadeCharacter`).
  3. Gaps from the exe map ("The exe", coverage table): its
     larger-type text boxes (`SetTextEXC_Large` …, 16
     calls in 8 scenes).
  4. Gaps from the scripts' coverage ("The scripts' coverage"):
     `TransLog` (125) and
     `_system.nut`'s line-log strings not read; 15 scenes with English
     written into the scenario (`LANG_NAME` branches: captions, dates,
     ma01_026's document, mc02_031's menus) show the NSS's Japanese,
     or, for story `CreateText`, nothing (Decided, above); the
     branches' `CreateText` are drawn in English (Queue, above), the
     `CreateTextArea` ones (ma01_009, mb04_015vs, ma01_026) not yet;
     the telops the port rewrote (md05_037vs's 魔剣対決 and 魔王編挿入
     pictures, mc01_026's last 殺意 word: see "Telops") stay at the
     Japanese places;
     four one-scene English boxes (ma01_009, mc01_003, ma01_002); the
     port's particles (rain, snow, sparks, 27 scenes) not drawn, as in
     the Japanese build. To check: `sys_backselect.nut` and
     `sys_key.nut`'s keys.
  5. The port's calls taken in part (the build's `English: … port
     <function> argument <i> ignored` lines; "The port's calls,
     argument by argument"), most noticeable first (the telops' calls are now read
     whole: `Move` and `CreateTextureEX`/`SP`, above).


### Boot logos (2026-10-18)

`sys_titlelogo.nut` `main` runs `LogoSet` while `TitleFirstLogo` isn't
set (the first title of each run; `boot.nut` itself shows nothing).
`LogoSet` (`nut.py`; the disassembler's opcodes from `BITW` up read 4
higher): black at once; `warning.png` (English `cg.npk`, full screen) in
300, `WaitKey(8000)`, `FadeDelete` 300; the black held 300 more;
`nitroplus-logo.png` in 1000, `WaitKey(4000)`, `FadeDelete` 1000;
`Wait(100)`. No JAST logo, no movie. `MURAMASA_EN`'s `boot_logos` is
this, written in NSS with the library's `CreateColorEX`,
`CreateTextureEX`, `FadeDelete` (`muramasa.rs`). `--real-time` trace:
15,036 ms to the title against the script's 15,000.


### The port's DrawTransition edge (2026-10-18)

The port draws `DrawTransition`'s rule through Mware's
`psUpdateTransitionRule` shader (DXBC decoded; cbuffer Now, Dst, Src,
Rate): a mask level u (grey ÷ 255) shows at sat(k(now − u) + u·now),
k = 2 ÷ rate. The scripts' values reach it ÷ 1000 (`CCommandAction.transition`
`tofloat() / 1000.0`; `sqSprite_transition` → `FUN_00435760`: rate at
+0x4f4, now the timeline at +0x4a8 going from src to dst along the tempo),
so k = 2000 ÷ softness. Ours had Muramasa.exe's table, whose band widens
with the threshold. Now `Game::diesel_transition` gives `effectsoft` form 1
(engine #146) and the port's table for transitions stopped partway
(`transition::diesel_table`, which depends on the end alone). Checked
headless by numbers (engine PR); no wine.

### Shake as the port's (2026-10-18)

The port's `Shake` is a Diesel `CEffectShake`: its scenes pass the NSS's n
as a frequency of (1000 / n) * 20 ms (Squirrel integers: 20 for n 501-1000,
40 for 500) and the NSS tempo as the swings' (`beattempo`), the size going
straight (00484990, 00484c60). Ours had the old 16 ms shake (pictures every
100 ms); now `Game::diesel_shake` turns every frequency ms, the size
straight (`screenshake` mode 2, `spmotion` keys), the swings drawn as
jumps (1-2 frames each). The n and tempo notes are gone (gaps 4,786 →
3,302).

- **Eased fades (2026-10-17):** the port eases `Fade` and `Transition`
  (the NSS's `DrawTransition`) by their tempo as the Japanese exe does
  ("Eased fades"), with its own `AxlDxl` and `DxlAxl` (`diesel_tempo`,
  `Game::fade_tempo`): the 588 eased transitions of the Japanese build
  are eased here too (`effectcurve` before their print), the 3 with
  `AxlDxl` along t² to the middle and back out (`effectcurve 51`; the
  Japanese exe's is a cosine, 72). `DxlAxl` has its curve (`MoveEase`'s
  ease 8) but no fade of the story's uses it.
- **Choice options at run time (2026-10-16, shared NSS code):** a
  choice's options set in runtime branches were taken at build time, so
  every player saw one set: mc02_031innsyoku listed all five items
  whatever they held, and md05_004 never offered "Leave your
  companion". Now the variables a choice reads (its options' texts and
  the conditions that pick how many options it has, `choice_flags` in
  `convert.rs`) stay at run time after nested runtime ifs too, and the
  choice takes such an option's text from a string variable set from the
  flag (`Op::RuntimeChoice`, `$3632` up; on the port's plates the x that
  centres it in `%3632` up). Checked headless: mc02_031innsyoku with all
  four items lists five options and with the pestle and the cooking wine
  only lists three, each going to its own file (`--vars`, `--choose`);
  md05_004hazure01 offers 頼む / "Ask for help" without a companion and
  別れる / "Leave your companion" with one.

- **Choices from variables (2026-10-16):** 9 of the port's `SetChoice`
  calls name variables, not text: mc02_031innsyoku's menu of things to
  hand over (ids 60-90, `Route.選択肢名前1`-`5`) and md05_004's third
  option in four scenes (id 20, `Route.md05_hazure` …, 頼む/別れる). The
  port writes the English straight into the variables (`Route.選択肢名前1
  <- "The pestle"`), so `english.rs` reads each such variable's values
  (`Function::slot_strs`) and gives the NSS's values for it the port's in
  order of first appearance (the port merged some branches: md05's three
  assignments are two there), in the assignments and in the
  comparisons that pick the branch (`if($選択肢名前1=="擂粉木を渡す")`):
  27 values. The tenth, ma05_021vs's debug menu (`$GameDebugSelect`),
  names constants the port never defines and stays Japanese. Checked
  headless on a one-scene build (`--file mc02_031innsyoku.nss --chain 1
  --loose`, `--goto if17_end`): `select "　The pestle",…,"　Nothing"`,
  the branches unchanged.

- **`<FW>` (2026-10-16):** the port's `TAG_FW` calls `SwitchFw(img)`
  (`function_textbox.nut`): with `img=''` and a box with a face window
  (CT, TH, …) it deletes the faces (`DeleteFw`), fades `box*` and
  `text*` out over 150 ms and goes on in the same box without a face
  (CC, H, …, `BoxInfo`'s text place). The translation adds it before
  narration it wrote into a speaker's stretch (ma01_014: "......", then
  "She glances over her shoulder."). The converter ends the page there
  and draws the next pages in the box without a face, the `Fw…`/`Nw…`
  pictures left out, until the next code (where the NSS sets the next
  face). Checked headless on ma01_014 (one-scene build, `--shot-pages`
  on the three pages: Ichijo's face, the plain box, Yuuhi's face).

- **Subtitles (2026-10-16):** the port calls `ShowVOICESEText(sound
  [, add])` (`function.nut`) after starting a voice whose
  `TransVoice[file]` (the scene's lang file) holds a line, or a list of
  `{time, text, x, y, add}`; it writes the line in `serif0`
  (`function_textbox.nut`: the screen's width at y 480, priority 20005,
  the `字幕` style: Droid Serif Muramasa 20, white, dark edge) and clears
  it as the voice ends (the list's process, or `ClearVOICESEText` after
  waiting for the voice). `english.rs` puts `ShowVOICESEText(sound, add,
  y, file, ms, text, …)` after the matching `MusicStart`/`SoundPlay` in
  the NSS; `muramasa/subtitles.rs` draws the lines (`font.npk`'s font,
  `mages::menus::Labels`) into a strip of 250 ms cells up to the voice's
  length, the last one empty, shown as an animated sprite (`serif0`, or
  `serif0+` a line under it for `add`). ma04_023's first Wolf line has
  an empty text: no subtitle, as in the port.

- **Text box against the original (2026-10-16):** filmed under wine
  (the launcher's Start Game, windowed 1024×576; first start, the
  prologue's pages): the port's lines are 25 px apart (`rowSpace` 1 in
  every `SetTextBoxVal` under the font's height of 24, not the 21 the
  build had), start 2 px lower than CONscripter's from the same y, and
  are set by the font alone (the build's `setwindow3` spacing 1 added
  half a pixel to each letter in CONscripter's proportional text, 5 %
  wider lines). Now: `horizontal_box` pitch 25 (12 lines), also for the
  `Game::box_text` boxes across; `text_drop` 2; spacing 0 for
  `Script::single_byte_text` windows. Page 5 (five lines) and page 1
  then match the original with no pixel differing in the text area
  (`compare -fuzz 10%`).
- **Telops at the port's places (2026-10-16):** 47 of the English
  telops have the Japanese names but other sizes (chapter titles 528×368
  lying, not 34×576 standing; 銀星号の唄 800 wide, not 460; prefaces a
  full screen at 70 %), and the port's scenes place them anew (centred
  titles, the song's lines 40 px apart from the middle, `outline/`
  versions of the captions). The converter reads each scene's calls
  (`squirrel::Function::calls`: literal arguments, `@n`, `-n`) and gives
  the k-th `CreateTexture…` (x, y, a telop picture) or `Move` (x, y) on
  a telop sprite the port's k-th one's arguments, when both make it as
  often (otherwise a picture the port makes once on the sprite takes its
  place); a `FadeDelete` the port adds on a telop sprite goes before its
  next placing call (ma01_026 clears its preface before the title).
  `cg/sys/telop/` joined `ENGLISH_FIRST`. 465 calls placed, 7 left
  (Status).

- **Faces in their frames (2026-10-15, reported by the user):** the
  face and name plate pictures (`FwC`/`NwC`, `FwR`, `FwL`, `FwH`/`NwH`
  of `function_stand.nss`) sat at the Japanese places (C 451,4 above
  the box; H 228,450 over the text) instead of in the frames the port's
  `TextWindow_Fw`/`THTextWindow_Fw` pictures have: the English table of
  `FwInfo` in `val.nut` puts them at C 452,55; R 708,55; L 195,55;
  H 87,439 (name plate 89,439). `en_face` in `muramasa.rs` moves them
  there in every scene with a face. Checked headless (ma01_003).

- **Voices (2026-10-15, reported by the user):** no line of the English
  build had its voice (2 `dwave 0` against the Japanese build's 36,055):
  the English rows write their tags with single quotes (`<voice
  name='雄飛' class='雄飛' src='voice/ma01/…'>`, also `<RUBY text='…'>`)
  and the NSS parser read only double-quoted values, so `src` kept its
  quotes and named no file. It takes either quote now: 35,902 voices
  (the rest are rows the translation left out or merged).

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
