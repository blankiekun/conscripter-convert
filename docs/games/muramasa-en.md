# Full Metal Daemon Muramasa (English, JAST USA 2021)

The English 装甲悪鬼村正 is a port, not a patch, of the 2009 PC game
(`docs/games/muramasa.md`) to Nitroplus's newer "Diesel" engine
(`muramasa_en.exe`, DirectX 11, Squirrel scripts, 1024×576; no NPA, no
NSS). The build converts the Japanese NSS and takes only the port's
English text and pictures.

## Status

Works: every stretch of the Japanese NSS gets its English; the full
build traces to the end (25,430 pages). English boxes, pictures
(`cg.npk`), name plates, telops, subtitles, captions, choices on the
port's plates, faces at the port's `FwInfo` places. Port screens:
title, boot logos, command menu, choices, line log, save/load (compared
under wine), settings (with the character voices window), questions,
click icon, quick bar, back to the choice on R; extras, CG gallery,
backlog and save/load are engine `screen`s. The text area matches the
original pixel for pixel (`ma00_000`); text speed and volume curves are
the exe's.

Open:

- `docs/games/muramasa-en.findings.tsv`: `Move`/`Zoom`/`Rotate` keep the
  Japanese curves (the port sends them through the tempo table, "Eased
  fades"); the English pictures for name plates, quick bar and telops
  (`ENGLISH_FIRST`) are inferred.
- A frame could settle: backlog speaker icon x (102), save thumbnails
  (158x90) and 300 ms page fade; Q.Save shows our one quick save (the
  port keeps 20).
- Backlog first row y 22: 20 is the scripts' (`backlogFieldRect.y` 10 +
  `backlogRowSpace` 10, `sys_backlog.nut`), the glyph's 2 px drop in its
  line measured under wine; `CTextEnglish_layout` 0x4a7f60 reads it from
  the font's metrics at run time (Droid Serif ascent 1901/2048 at 20).
- Not compared with the original: command menu, subtitles, telops,
  quick bar, click icon, quick-save notice, boxes across (H, CH, Noh;
  6 lines), backlog voice icon and scrolling, settings dragging.
- Not done: the settings' text sample; `クイックセーブ中`,
  `読み込んでいます`, `削除しました`, `ロック通知`; `FwB`/`FwCH` faces at
  the Japanese place (no `FwInfo` entry); 5-6 option choices use
  `select`, not plates (`choice_style_en`'s `shortcut:`).
- Metrics: 4 "text window moved inside a page: kept" notes; English-side
  `Metrics::left_out` entries unlisted.
- ma01_026's document shows whole before its key sounds; the port types
  it letter by letter.
- 51 `logadd` lines carry Japanese: voice-only lines (ex/vs, ma02, mb04,
  mc02 dishes), date telops, lines the port folds into one row.
- 22 English rows placed nowhere (the NSS orders or splits them
  otherwise: mc01's route variants, ma01_003's `<CLICK>` rows).
- From "Coverage": larger-type boxes, `TransLog`, telops the port
  rewrote (md05_037vs, mc01_026) at the Japanese places, one-scene
  boxes, particles, ignored port arguments.
- Static checks (`docs/new-game.md` steps 6-7) on `muramasa_en.exe` + `Mware.dll`, where the
  port differs; 8 fields, 2 exe-read settings, 4 every-frame shaders read. Gaps:
  - Typing time from the scripts (3 calls): `sqText_setMaxTypingTime` 0x4ffc50 writes `+0x14c`,
    `setMinTypingTime` 0x4ffc20 `+0x150`, read by `setTypingSpeed` `FUN_004b4510`;
    `function_textbox.nut` passes 800, the `sys_config.nut` sample 800 + 1.5…; ours: the
    default curve fixed (`muramasa/screens.rs`), right only while the scripts pass the default.
  - `TextBox.TypeForce`/`TypeForceSpeed` (`function_textbox.nut` ops 177-185) replace the
    player's speed; ours: not handled; whether a scene sets it is unchecked.
  - Shake on text, camera and models: `CEffectShake` is also built by the text path (0x4b4ed0 …
    0x4b8a80; `sqText_randomShake` 0x4ff390), `sqCamera_shake` 0x509080 and 0x474820; ours: the
    sprite path only (`sqSprite_shake` 0x4eb3f0); `randomShake` in 0 scene files, `shake` in 1.
  - Mask transition: `+0x4f4`'s one writer FUN_00435760 is reached from `sqSprite_transition`
    0x4eb320 and `sqSpriteMask_transition` 0x4f40d0; ours: the sprite one; mask path unchecked.
  - `System.disable_gore` (1 write, `val.nut` 0; launcher 0x405f90 via FUN_00405a40):
    `ReplaceGoreFilename`, `GoreFilenameReplaceTable` looks empty; ours: not handled.
  - Not done: writers of the `FwInfo` places, `CTextEnglish_layout` 0x4a7f60 metrics and
    backlog fields; the globals the exe reads from the VM by name (sq_get not identified among
    0055c5f0, 0055bca0, 0055bbe0); the sprite layers' render-target format (DXGI table at
    Mware 0x10189704; RGBA16F would be ±1 level a layer).
  - Fine: `pssprite_blend_normal.cso` (texture × colour, discard alpha < 1/255) and
    `posteffect/pssimple.cso` are identity; swap chain and textures R8G8B8A8_UNORM (Mware
    0x10052880, 0x100658c0), no sRGB format in 8 lists (`FUN_100551a0`); sprite blend
    SRC_ALPHA/INV_SRC_ALPHA (Mware 1000bff0, FUN_10057410); standing-picture tint
    (`StSetColorBase`) only `その他` = 1000/1000/1000 (0 of 1653 `St()` calls pass a tag); cg
    pictures are WebP (882 of 882).

Next: the web play-through (`docs/games/muramasa.md`, Status).

## Scope

The Steam release over the user's Japanese install. Manual:
`Manual.pdf` in the install (29 pages: title, main and quick menus,
save/load, backlog, extra, config). Decided: the release's own text
font; menus follow the port's redesign; story `CreateText` without an
English branch stays Japanese; a click on an empty slot at the title
saves nothing (the original saves); Continue loads the quick save, else the
newest; no closer volume fit.

Story, walkthrough and progression variables: `docs/games/muramasa.md`;
the port's scenes are the NSS's one for one. Choices whose options the
port keeps in variables (`Route.<name>`: mc02_031innsyoku, md05_004)
take them at run time.

## Build

```sh
python3 -I research/nitroplus/npk.py <english game>/font.npk <fontdir>   # droidserif-muramasa.ttf
target/release/conscripter-convert build <english game> --japanese <game> \
  -o out/muramasa-en-cons --font <fontdir>/media/font/droidserif-muramasa.ttf
```

`build` runs `convert-nss <game> --english <english game>` with the
port's title (`docs/cli.md`); `--dry-run` after `-o` writes only
`0.txt`. IPA Gothic goes in as `fallback.ttf` for what the font lacks
(`「」・✓`, the Japanese of a few kept lines).

## Testing

As the Japanese build (`docs/games/muramasa.md`, "Testing");
`tests/snapshot.sh` covers it as `muramasa-en`. `python3 -I
research/tools/cmdcheck.py --counts` counts a build's commands (it
caught back-to-choice missing: `savegame` 2 against 170).

## Executables

| File | Version | sha256 |
|---|---|---|
| `muramasa_en.exe` | unknown | `5a174f7037f021ec22f228869b3d0e62ca143dcdc051597fe4a378c26ba0b4b9` |

Decompile:

    GHIDRA_HEADLESS_MAXMEM=2G research/tools/capped.sh 3 \
      research/tools/ghidra/ghidra_export.sh <game>/muramasa_en.exe \
      research/nitroplus/local/ghidra-en --names research/nitroplus/en-names.tsv --rtti

12,567 functions, named by `research/nitroplus/en-names.tsv` (1,806
rows; Muramasa.exe's are `names.tsv`); registrations read from
objdump's disassembly (`research/tools/ghidra/`).

- **Map:** 0x401960 `app_main` (reads `system.npk`'s `system.dat`,
  opens archives, starts Squirrel); Diesel classes (RTTI
  `Diesel::CText…`, `CTextTokenizer`, …); natives 0x465260 …
  0x52f360; NPK3 and AES 0x54cf40 … 0x555e00; Squirrel 2.2.4, sqstdlib,
  SqPlus 0x555e00 … 0x57fe3f; then the CRT; SEH funclets 0x5c4000 …
  0x5d4000. zstd and vector maths are `Mware.dll` imports. The 17 code
  labels reached only by pointer are `cb_<addr>`.
- **Natives** (`sq<Class>_<method>`, `sqfn_<name>`): a registrar
  `sqreg_<Class>` per class (SqPlus `SQClassDef`); globals in
  `sqreg_globals` (0x46d890) and `sqreg_math`. 26 classes, 1,159 method
  handlers, 48 globals. `framework/nut/cnut*.nut` wraps each native one
  to one, so every method name occurs in the corpus and the scenario
  calls no native directly.
- **NPK3:** `npk_open` 0x54cf40 (key: eight immediates at 0x401c28),
  `aes256_expand_key` 0x554620, `aes256_decrypt_key` 0x554910; AES-NI
  CBC decryption inline in `npk_open` and `npk_read_segment_a/b`
  (0x54e0a0, 0x54e470), then `ZSTD_decompress`.
- **Text:** `function_textbox.nut` defines `WAIT`, `WAITVOICE`, `FW`,
  `NW`, `RED`, `RED2`, `RED3`, `YELLOW`, `VALUE`, `BR` and handles
  `voice`, `I`, `K`; native `SQText` takes `RUBY`, `SMALL`, `CENTER`
  (`text_tokenize_markup` 0x4c3440, `CTextEnglish_layout` 0x4a7f60:
  breaks at spaces and CR/LF, skips `# { }`).

## Files and formats

`<game>/*.npk`: system, script, cg, dx, font, sound, voice, particle,
then `patch/*.npk` mounted over them.

    python3 -I research/nitroplus/npk.py <game>/system.npk          # list
    python3 -I research/nitroplus/npk.py <game>/script.npk OUT [GLOB]

- **NPK3:** `NPK3`, u32 version 1, 16-byte IV, u32 entries, u32 index
  size; then the AES-256-CBC index. The key is eight dword immediates
  in the code (a byte scan misses it); value in `npk.py`. Entry: u8
  flag, u16 name length, UTF-8 name, u32 size, 32-byte hash, u32
  segments, then per segment u64 offset, u32 stored size (16-byte
  aligned), u32 packed size, u32 size. Segment: AES-256-CBC (same key,
  archive IV), then zstd when packed size < size; some zstd frames omit
  their size.
- `script.npk`: 1884 compiled Squirrel files (framework, system,
  function, 884 scenario files, `lang/en/`). `cg.npk`, `particle.npk`:
  lossless WebP under `.png` names. `font.npk`:
  `droidserif-muramasa.ttf` only. `system.npk`: `setting.ini`
  (1024×576, `WINDOW_NAME`, `[Font]`), the DirectX shaders `Mware.dll` loads by
  path (`shader/sprite/psSprite_Blend_Normal.cso`; we need none, Status).
- `sound.npk`, `voice.npk`, `dx.npk`: unused; the Japanese build's
  identical files stand in.

### Eased fades

Scenes pass a tempo to `Fade(name, time, dst, tempo, wait)` and
`Transition(…, tempo, wait)` as a global (`GETK 'Dxl1'`) from
`framework/constantvalue.nut`, kept after the compiled file's main
function as a table (u16 length with the NUL, name, NUL, `0x05000002`,
int32 value): `Linear` 0, `Axl1` 1, `Dxl1` 2, … `Dxl8` 16, `AxlDxl1`
17, `DxlAxl1` 18, … `DxlAxl8` 32, the same 32 with `a`-`d` (33-160).
Paired call by call with the NSS (`python3 -I
research/nitroplus/en_tempos.py <nss dir> <script dir>`):

| NSS | Port | Id | Curve (share of the way at time share t) |
|---|---|---|---|
| `null` | `null` | 0 | t |
| `Axl1`, `Axl2`, `Axl3` | the same | 1, 3, 5 | t², t³, t⁴ |
| `Dxl1`, `Dxl2`, `Dxl3` | the same | 2, 4, 6 | 1-(1-t)², ³, ⁴ |
| `AxlAuto` | `Axl1d` | 129 | 1 - cos(πt/2) |
| `DxlAuto` | `Dxl1d` | 130 | sin(πt/2) |
| `AxlDxl` | `AxlDxl1` | 17 | (2t)²/2, then 1 - (2-2t)²/2 |
| `DxlAxl` | `DxlAxl1` | 18 | (1 - (1-2t)²)/2, then (1 + (2t-1)²)/2 |

`sqSprite_fade` (0x4eb0f0) and `sqSprite_transition` (0x4eb320) store
the tempo (0 past 160) in an interpolator whose step (0x40bca0 fade,
0x437cc0 transition) calls `table[tempo]` from the 161-entry table at
0x5e40a0. The NSS names match Muramasa.exe's curves (`tempo` in
`convert.rs`) except `AxlDxl`/`DxlAxl` (`diesel_tempo` in
`muramasa.rs`, its test holding the exe's numbers;
`research/nitroplus/en_tempo_exe.py <exe> [--all]` runs the exe's
table). No scene uses ids 7-16, 19-96, 131-132.

The port's `DrawTransition` edge, shake and text box metrics differ
from Muramasa.exe's; notes at `psUpdateTransitionRule`, `CEffectShake`
(`nitroplus/convert.rs`) and `rowSpace` (`muramasa.rs`).

## Script language

Squirrel 2.2.4 (the exe's VM, `"Squirrel 2.2.4 stable"`):

    python3 -I research/nitroplus/nut.py FILE.nut [--strings | --tables]

- Files: `SCRP`, u32 header size 0x10, two u32; then a Squirrel 2.2
  closure stream (32-bit SQInteger, 1-byte chars, UTF-8) whose header
  keeps only sizeof(SQChar) and whose functions end in u32 stack size,
  u8 generator, u8 varparams. All 1884 files parse.
- `nut.py` still names 3.1's opcodes: `LINE` to `GETK` (0-9), `LOAD`,
  `LOADINT`, `PREPCALLK`, `CALL`, `NEWSLOT` decode right; 2.2's others
  are 17 `ARITH` (operator character as last operand: 43 `+`, 45 `-`,
  42 `*`), 19 `RETURN`, 20 `LOADNULLS`, 22 `LOADBOOL`, 30 `NEWTABLE`.
  `nut.py`'s `OPS` still awaits 2.2's list.
- `nut/<scene>.nut`: one `scene` function calling NSS builtins by name.
  Each text block is `Print(key, japanese)`, the Japanese an NSS
  `<PRE>` block cut at its inline `{…}` code (39,038 Prints, 14,527
  blocks).
- `lang/en/<scene>.nut`: `TransText[key] = english` (39,010 rows),
  `TransChoice` (403), `TransLog` (125), `TransVoice` (subtitles). Each
  value repeats the Japanese as `//` lines, then the English with
  `<voice …>`, `<center>`, `<I>`, `<K>`, `<br>`, `<RUBY text='…'>`.
- Join (`nitroplus/english.rs`): 38,465 of 38,798 NSS segments (99.1 %)
  equal a Print's text once `//` lines, tags and whitespace go; `align`
  pairs the rest by position between matched rows.

**Coverage.** The converter compiles none of the port's scripts; each
file's status names its stand-in.

    R=<script>/media/script           # script.npk extracted with npk.py
    python3 -I research/nitroplus/nut_refs.py $R tables
    python3 -I research/nitroplus/nut_refs.py $R calls                    # every called name
    { conscripter-convert nss builtins <nss> | grep -oE '^[A-Za-z_0-9]+:' | tr -d :
      grep -ohE '^\s*function\s+[A-Za-z_0-9]+' <nss>/*.nss | awk '{print $2}'; } > nss-names.txt
    python3 -I research/nitroplus/nut_refs.py $R calls --nss nss-names.txt   # port-only names

| Files | Status |
|---|---|
| `nut/*.nut` (884) | replaced by the NSS of the same name (`0_boot_start` is `0_boot.nss`; `md05_025cube` is `function_magiccube.nss`, `magiccube.rs`) |
| `lang/en/*.nut` | read: `TransText`, `TransChoice`, `TransVoice`, `SetBacklog` rows (`english.rs`); **not read:** `TransLog` (125), `_system.nut`'s line-log strings |
| `function/*.nut` (18) | replaced by the NSS's `function*.nss`; `function_textbox.nut` and `val.nut` (`BoxInfo`, `FwInfo`) read by hand into `EN_TEXT` and the box sizes (`muramasa.rs`) |
| `system/boot.nut`, `sys_titlelogo.nut` | `opening::EN` and the boot logos (`MURAMASA_EN`, `muramasa.rs`) |
| `system/sys_menu.nut`, `sys_dialog*.nut`, `sys_close.nut`, `sys_totitle.nut` | `command_menu_en`, `confirm_en` (`screens.rs`); **not done:** `sys_dialog_message.nut` (`クイックセーブ完了`, …) |
| `system/sys_saveload.nut` etc., `sys_backlog.nut`, `sys_config.nut`, `sys_extra*.nut` | `save_grid`, `line_log`, `settings`, `extras` (Japanese pictures; the English `save/`, `backlog/`, `config/`, `extra/` left) |
| `system/sys_quicksave.nut`, `sys_quickload.nut` | CONscripter's quick save; **not done:** the quick-load question |
| `system/sys_backselect.nut` | the back-to-choice question, opened by R (`CommandMenu::keys`) |
| `system/sys_key.nut` | CONscripter's keys; **to check** against its 36 `KEY_CODE_*` |
| `system/sys_tips*.nut`, `sys_dialog_windowsize.nut`, `zdbg_showval.nut` | not used: no script opens them |
| `framework/` (59), `custom/` (4), `main.nut` | not used: the Diesel framework |
| `particle.npk` (`CreateParticle`, 39 calls, 27 scenes) | **not converted**, as in the Japanese build (`CreateEffect Rain`) |

Port-only scenario calls (205 of 635 names; the other 430 are the
NSS's and come from it):

| Call | Converter |
|---|---|
| `Print`, `SetChoice`, `SceneInit`, … (`function_n2`, `function_textbox`, `function_select`) | the NSS's `<PRE>` blocks and `select`s, with the English |
| `ShowVOICESEText`/`ClearVOICESEText` (32/18, 10 scenes) | a subtitle strip (`muramasa/subtitles.rs`) |
| `<WAITVOICE=n>` 95, `<WAIT=n>` 16, `<RED*>`/`<YELLOW>` 28, `<SMALL>` 8, `<I>`, `<center>` | `!vn`, `!dn`, `#RRGGBB`, `!z` (50 %: the ruby font, exe 004a82e7), `!i1`…`!i0`, `!c` |
| `<FW>` 25 (`SwitchFw`) | a new page in the box without a face until the next code |
| `CreateTexture…`/`Move` on a telop | the port's place and picture; priority, time, tempo and wait the NSS's |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/…` (16, 8 scenes) | **not converted** (the NSS's box) |
| `SetTextEXC_ma01_009`, `SetFwRR`/`SetFwLL`, `TypeBeginTimeCIY` (mc01_003), `DeleteFwAll` (ma01_002) | **not converted** |
| `CreateTextArea` (ma01_009, ma01_026, mb04_015vs), `FadeCharacter` (ma01_026) | **Japanese text**; ma01_026's document shown whole |

`english.rs`'s `port_reads` records which port-call arguments the
converter reads; the build prints `English: … port <function> argument
<i> ignored` per argument passed and unread (`UNREAD_HARMLESS` lists
the harmless ones). Scenes with English in the scenario (`LANG_NAME ==
"en"`, 15): mb03_006, mb03_022, md02_009, md02_038, md04_*, md04_048vs
take it (`put_captions`); the rest stay Japanese: ma01_009, ma01_026,
mb04_015vs, mc02_031daidokoroa/aa (menus), md04_002, md04_006,
md04_015, md04_026vs, md04_030 (dates and times).
