# Full Metal Daemon Muramasa (English, JAST USA 2021)

The English release of 装甲悪鬼村正 is not a patch of the 2009 PC game
(`docs/games/muramasa.md`) but a port to Nitroplus's newer "Diesel"
engine (`muramasa_en.exe`, DirectX 11, Squirrel scripts, 1024×576).
It has no NPA archives and no NSS scripts.

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-15 13:05 (reuses the NSS reader) | - |
| First dry run | 2026-10-15 13:17 | - |
| Story plays | - | - |
| Title, save, settings | 2026-10-15 20:18 to 10-16 00:25 | - |
| Extras | - | - |
| Progression verified | - | - |
| Finished | - | - |

## Status

Manual: `Manual.pdf` in the install (29 pages, English; screens: title
menu, main menu, quick menu, save/load, backlog, extra, config with
basic/text/sound/display tabs). Use it before asking for screenshots.
The write-ups of finished work are in History ("Status as it stood
before the 2026-10-18 prune" and the sections after it).

**Works** (details in History):

- The English text in the converter: `build <english game> --japanese <game>`
  (`convert-nss <game> … --english <english game>`, its title) reads the
  English release's `script.npk` into the Japanese NSS (`english.rs`);
  every stretch takes its English, none stay in Japanese; the full build
  plays to the end with the trace (25,430 pages). Pass the release's
  `droidserif-muramasa.ttf` as `--font`; the converter adds IPA Gothic
  as `fallback.ttf`, from which the engine draws what the font lacks
  (`「」・✓`, the Japanese a few kept lines carry). English boxes, pictures
  (`cg.npk`), name plates, telops, subtitles, story captions, choices
  on the port's plates, and the port's `LANG_NAME` text-area lines are in.
- Port screens: title, command menu, choices, line log, save/load
  (compared with the original under wine), settings, questions, click
  icon, quick bar, back to the choice on R (no menu row, as the port). The extras' menu, CG gallery, backlog, save/load are
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
- Settings' "click stops auto" pair (y 269) works (2026-10-18:
  `consset "clickautostop"`; off, the bar's AUTO button ends auto).
- Not done: the settings' text sample; `クイックセーブ中`, `読み込んでいます`, `削除しました`,
  `ロック通知`; faces of `FwB`/`FwCH` keep the Japanese place (the port's
  `FwInfo` has none).
- Metrics: 4 "text window moved inside a page: kept" notes; the
  English-side `Metrics::left_out` entries (menu built-ins the English
  screens replace) are not yet listed.
- ma01_026's document is shown whole before its key sounds; the port
  types it letter by letter (`CreateTextArea` + `FadeCharacter`).
- Backlog (`SetBacklog`) lines take the English of the row with the same
  Japanese; 51 `logadd` lines still carry Japanese: voice-only lines
  (ex/vs, ma02, mb04, mc02 dishes), the date telops, and lines the port
  folds into one row (ma01_017's three-line speech).
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
  most noticeable first. To check: `sys_key.nut`'s other keys.

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

Run in the exe (`research/tools/exe_call.py`;
`research/nitroplus/en_tempo_exe.py <exe>` prints them), the eleven
curves above give our shares at t = 0, 1/16, …, 1 to within 2e-6 (a test
in `muramasa.rs` holds the exe's numbers). The rest of the table, from
`--all`: ids 7-16 are t⁵…t⁹ in and out, 19-32 the in-out pairs up to
t⁹, 33-96 overshoot below 0 or past 1, and 132 (`DxlAxl1d`) gives the
same as 131 (`AxlDxl1d`, ½ - ½cos πt); no scene uses these.

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

    GHIDRA_HEADLESS_MAXMEM=2G research/tools/capped.sh 3 \
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
| `system/sys_backselect.nut` | in: the port's back-to-choice question (`選択肢まで`), opened by R (`sys_key.nut` BackToSelect); the English command menu has no row for it ("Back to the choice by R") |
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

Done items: [docs/history/muramasa-en.md](../history/muramasa-en.md).
