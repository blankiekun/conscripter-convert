# Full Metal Daemon Muramasa (English, JAST USA 2021)

The English release of 装甲悪鬼村正 is not a patch of the 2009 PC game
(`docs/games/muramasa.md`) but a port to Nitroplus's newer "Diesel"
engine (`muramasa_en.exe`, DirectX 11, Squirrel scripts, 1024×576),
with no NPA archives and no NSS scripts. It is part of the Muramasa
work: the build converts the Japanese game's NSS and takes only the
English text and pictures from the port.

## Scope

The Steam release (English), on top of the user's Japanese 装甲悪鬼村正
install. Manual: `Manual.pdf` in the install (29 pages: title, main and
quick menus, save/load, backlog, extra, config); read it before asking
for screenshots. Decided: the text font is the release's own; menus
follow the port's redesigned screens; story `CreateText` without an
English branch stays Japanese; a click on an empty slot at the title
doesn't save (the original does); Continue loads the quick save, else
the newest; no closer volume fit.

## Build

```sh
python3 -I research/nitroplus/npk.py <english game>/font.npk <fontdir>   # droidserif-muramasa.ttf
target/release/conscripter-convert build <english game> --japanese <game> \
  -o out/muramasa-en-cons --font <fontdir>/media/font/droidserif-muramasa.ttf
```

`build` runs `convert-nss <game> --english <english game>` with the
port's title (`docs/cli.md`); `--dry-run` after `-o` writes `0.txt`
only. The converter adds IPA Gothic as `fallback.ttf`, from which the
engine draws what the font lacks (`「」・✓`, the Japanese a few kept
lines carry). `--original-archives` doesn't work with `--english` yet.

## Executables

| File | Version | sha256 |
|---|---|---|
| `muramasa_en.exe` | unknown | `5a174f7037f021ec22f228869b3d0e62ca143dcdc051597fe4a378c26ba0b4b9` |

Decompile:

    GHIDRA_HEADLESS_MAXMEM=2G research/tools/capped.sh 3 \
      research/tools/ghidra/ghidra_export.sh <game>/muramasa_en.exe \
      research/nitroplus/local/ghidra-en --names research/nitroplus/en-names.tsv --rtti

12,567 functions; `research/nitroplus/en-names.tsv` (1,806 rows) names
them (Muramasa.exe's are in `names.tsv`). The registrations were read
from objdump's disassembly (`docs/new-game.md` §2, "Mapping the whole
exe").

- **Map:** 0x401960 `app_main` (reads `system.npk`'s `system.dat`,
  opens the archives, starts Squirrel); the Diesel classes (RTTI
  `Diesel::CText…`, `CTextTokenizer`, …); the natives 0x465260 …
  0x52f360; NPK3 and AES 0x54cf40 … 0x555e00; Squirrel 2.2.4,
  sqstdlib and SqPlus 0x555e00 … 0x57fe3f; the CRT after it; SEH
  funclets 0x5c4000 … 0x5d4000. zstd and the vector maths are imports
  from `Mware.dll`. The 17 code labels reached only through a pointer
  are `cb_<addr>`.
- **Natives** (`sq<Class>_<method>`, `sqfn_<name>`): each class has a
  registrar `sqreg_<Class>` (SqPlus `SQClassDef`); the globals are
  registered by `sqreg_globals` (0x46d890) and `sqreg_math`. 26
  classes, 1,159 method handlers, 48 globals. Every method name occurs
  in the corpus, because `framework/nut/cnut*.nut` wraps each native
  one to one; the scenario never calls a native directly.
- **NPK3:** `npk_open` 0x54cf40 (key: eight immediates at 0x401c28),
  `aes256_expand_key` 0x554620, `aes256_decrypt_key` 0x554910; the
  AES-NI CBC decryption is inline in `npk_open` and
  `npk_read_segment_a/b` (0x54e0a0, 0x54e470), then `ZSTD_decompress`.
- **Text:** `function_textbox.nut` defines `WAIT`, `WAITVOICE`, `FW`,
  `NW`, `RED`, `RED2`, `RED3`, `YELLOW`, `VALUE`, `BR` and handles
  `voice`, `I`, `K`; the native `SQText` takes `RUBY`, `SMALL`,
  `CENTER` (`text_tokenize_markup` 0x4c3440, `CTextEnglish_layout`
  0x4a7f60: breaks at spaces and CR/LF, skips `# { }`).

## Files and formats

`<game>/*.npk`: system, script, cg, dx, font, sound, voice, particle,
plus `patch/*.npk` mounted after them.

    python3 -I research/nitroplus/npk.py <game>/system.npk          # list
    python3 -I research/nitroplus/npk.py <game>/script.npk OUT [GLOB]

- **NPK3:** `NPK3`, u32 version 1, 16-byte IV, u32 entries, u32 index
  size; the index follows, AES-256-CBC. The key is eight dword
  immediates in the code (not plain bytes, so a byte scan misses it);
  value in `npk.py`. Entry: u8 flag, u16 name length, UTF-8 name, u32
  size, 32-byte hash, u32 segments, then per segment u64 offset, u32
  stored size (16-byte aligned), u32 packed size, u32 size. Segment:
  AES-256-CBC (same key, the archive's IV), then zstd when packed size
  < size. Some zstd frames don't record their size.
- `script.npk`: 1884 compiled Squirrel files (framework, system,
  function, 884 scenario files, `lang/en/`). `cg.npk`, `particle.npk`:
  the `.png` names hold lossless WebP. `font.npk`:
  `droidserif-muramasa.ttf` only. `system.npk`: `setting.ini`
  (1024×576, `WINDOW_NAME`, `[Font]`), DirectX shaders (not used).
- `sound.npk`, `voice.npk`, `dx.npk`: not used; the Japanese build's
  files stand in (the same files).

### Eased fades

The port's scenes pass a tempo to `Fade(name, time, dst, tempo, wait)`
and `Transition(…, tempo, wait)` as a global (`GETK 'Dxl1'`) from
`framework/constantvalue.nut`, which the compiled file keeps after its
main function as a table (u16 length with the NUL, the name, NUL,
`0x05000002`, the int32 value): `Linear` 0, `Axl1` 1, `Dxl1` 2, …
`Dxl8` 16, `AxlDxl1` 17, `DxlAxl1` 18, … `DxlAxl8` 32, and the same 32
with `a`-`d` (33-160). Paired call by call with the NSS
(`python3 -I research/nitroplus/en_tempos.py <nss dir> <script dir>`):

| NSS | Port | Id | Curve (share of the way at time share t) |
|---|---|---|---|
| `null` | `null` | 0 | t |
| `Axl1`, `Axl2`, `Axl3` | the same | 1, 3, 5 | t², t³, t⁴ |
| `Dxl1`, `Dxl2`, `Dxl3` | the same | 2, 4, 6 | 1-(1-t)², ³, ⁴ |
| `AxlAuto` | `Axl1d` | 129 | 1 - cos(πt/2) |
| `DxlAuto` | `Dxl1d` | 130 | sin(πt/2) |
| `AxlDxl` | `AxlDxl1` | 17 | (2t)²/2, then 1 - (2-2t)²/2 |
| `DxlAxl` | `DxlAxl1` | 18 | (1 - (1-2t)²)/2, then (1 + (2t-1)²)/2 |

In the exe `sqSprite_fade` (0x4eb0f0) and `sqSprite_transition`
(0x4eb320) store the tempo (0 past 160) in an interpolator whose step
(0x40bca0 fade, 0x437cc0 transition) calls `table[tempo]` from the
161-entry table at 0x5e40a0. The NSS names match Muramasa.exe's curves
(`tempo` in `convert.rs`) except `AxlDxl`/`DxlAxl` (`diesel_tempo` in
`muramasa.rs`, whose test holds the exe's numbers;
`research/nitroplus/en_tempo_exe.py <exe> [--all]` runs the table in the
exe). Ids 7-16, 19-96 and 131-132 are used by no scene.

The port's `DrawTransition` edge, shake and text box metrics differ
from Muramasa.exe's; the converter's notes on them are at
`psUpdateTransitionRule`, `CEffectShake` (`nitroplus/convert.rs`) and
`rowSpace` (`muramasa.rs`).

## Script language

Squirrel 2.2.4 (the exe's VM, `"Squirrel 2.2.4 stable"`):

    python3 -I research/nitroplus/nut.py FILE.nut [--strings | --tables]

- Files: `SCRP`, u32 header size 0x10, two u32; then a Squirrel 2.2
  closure stream (32-bit SQInteger, 1-byte chars, UTF-8) whose header
  keeps only sizeof(SQChar), and whose functions end in u32 stack size,
  u8 generator, u8 varparams. All 1884 files parse.
- `nut.py` still names 3.1's opcodes: `LINE` to `GETK` (0-9), `LOAD`,
  `LOADINT`, `PREPCALLK`, `CALL`, `NEWSLOT` decode right; 2.2's others
  are 17 `ARITH` (operator character as last operand: 43 `+`, 45 `-`,
  42 `*`), 19 `RETURN`, 20 `LOADNULLS`, 22 `LOADBOOL`, 30 `NEWTABLE`.
  Switching `nut.py`'s `OPS` to 2.2's list is not done.
- `nut/<scene>.nut`: one `scene` function calling the NSS builtins by
  name. Each text block is `Print(key, japanese)`, the Japanese being an
  NSS `<PRE>` block cut at its inline `{…}` code (39,038 Prints for
  14,527 blocks).
- `lang/en/<scene>.nut`: `TransText[key] = english` (39,010 rows),
  `TransChoice` (403), `TransLog` (125), `TransVoice` (subtitles). Each
  value repeats the Japanese as `//` lines, then the English with
  `<voice …>`, `<center>`, `<I>`, `<K>`, `<br>`, `<RUBY text='…'>`.
- Join (`nitroplus/english.rs`): 38,465 of 38,798 NSS segments (99.1 %)
  equal a Print's text once `//` lines, tags and whitespace are removed;
  `align` pairs the rest by position between matched rows.

**Coverage.** The converter compiles none of the port's scripts; each
file's status says what stands in for it.

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

Port-only scenario calls (205 of 635 names; the other 430 carry the
NSS's names and come from the NSS):

| Call | Converter |
|---|---|
| `Print`, `SetChoice`, `SceneInit`, … (`function_n2`, `function_textbox`, `function_select`) | the NSS's `<PRE>` blocks and `select`s, with the English |
| `ShowVOICESEText`/`ClearVOICESEText` (32/18, 10 scenes) | a subtitle strip (`muramasa/subtitles.rs`) |
| `<WAITVOICE=n>` 95, `<WAIT=n>` 16, `<RED*>`/`<YELLOW>` 28, `<SMALL>` 8, `<I>`, `<center>` | `!vn`, `!dn`, `#RRGGBB`, `!z` (size a guess), `!i1`…`!i0`, `!c` |
| `<FW>` 25 (`SwitchFw`) | a new page in the box without a face until the next code |
| `CreateTexture…`/`Move` on a telop | the port's place and picture; priority, time, tempo and wait the NSS's |
| `SetTextEXC_Large/Huge/Big/Middle`, `TypeBegin…_Large/…` (16, 8 scenes) | **not converted** (the NSS's box) |
| `SetTextEXC_ma01_009`, `SetFwRR`/`SetFwLL`, `TypeBeginTimeCIY` (mc01_003), `DeleteFwAll` (ma01_002) | **not converted** |
| `CreateTextArea` (ma01_009, ma01_026, mb04_015vs), `FadeCharacter` (ma01_026) | **Japanese text**; ma01_026's document shown whole |

`english.rs`'s `port_reads` records which arguments of the port's calls
the converter reads; the build prints one `English: … port <function>
argument <i> ignored` line per argument passed and not read
(`UNREAD_HARMLESS` lists the harmless ones). Scenes with English written
into the scenario (`LANG_NAME == "en"`, 15 scenes): mb03_006, mb03_022,
md02_009, md02_038, md04_*, md04_048vs take it (`put_captions`); the
rest stay Japanese: ma01_009, ma01_026, mb04_015vs,
mc02_031daidokoroa/aa (the menus), md04_002, md04_006, md04_015,
md04_026vs, md04_030 (dates and times).

## Story structure and walkthrough

As the Japanese game (`docs/games/muramasa.md`): the port's scenes are
the NSS's, one for one.

## Progression variables

As the Japanese game. The choices whose options the port puts in
variables (`Route.<name>`: mc02_031innsyoku, md05_004) take their values
at run time.

## Testing

As the Japanese build (`docs/games/muramasa.md`, "Testing");
`tests/snapshot.sh` covers it as `muramasa-en`.
`python3 -I research/tools/cmdcheck.py --counts` counts the commands a
build uses (it found back-to-choice missing: `savegame` 2 against 170).

## Milestones

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

**Works:** every stretch of the Japanese NSS takes its English; the full
build plays to the end with the trace (25,430 pages). English boxes,
pictures (`cg.npk`), name plates, telops, subtitles, captions, choices
on the port's plates, faces at the port's `FwInfo` places. Port screens:
title, boot logos, command menu, choices, line log, save/load (compared
with the original under wine), settings (with the character voices
window), questions, click icon, quick bar, back to the choice on R. The
extras, CG gallery, backlog and save/load are the engine's `screen`s.
The text area matches the original pixel for pixel (`ma00_000`); text
speed and volume curves are the exe's.

**Open:**

- `docs/games/muramasa-en.findings.tsv`: `Move`/`Zoom`/`Rotate` keep the
  Japanese curves (the port sends them through the tempo table, "Eased
  fades"); the English pictures taken only for the name plates, quick
  bar and telops (`ENGLISH_FIRST`) are inferred.
- Guesses a frame could settle: the backlog's speaker icon x (102), the
  save screen's thumbnails (158x90) and 300 ms page fade; the Q.Save
  page shows our one quick save (the port keeps 20).
- Not yet compared with the original: command menu, subtitles, telops,
  quick bar, click icon, the quick-save notice, boxes across (H, CH,
  Noh; 6 lines), backlog voice icon and scrolling, settings dragging.
- Not done: the settings' text sample; `クイックセーブ中`,
  `読み込んでいます`, `削除しました`, `ロック通知`; faces of `FwB`/`FwCH`
  keep the Japanese place (the port's `FwInfo` has none); choices of 5-6
  options use `select`, not plates (`choice_style_en`'s `shortcut:`).
- Metrics: 4 "text window moved inside a page: kept" notes; the
  English-side `Metrics::left_out` entries are not listed.
- ma01_026's document is shown whole before its key sounds; the port
  types it letter by letter.
- 51 `logadd` lines carry Japanese: voice-only lines (ex/vs, ma02, mb04,
  mc02 dishes), the date telops, lines the port folds into one row.
- 22 rows of English placed nowhere (lines the NSS orders or splits
  otherwise: mc01's route variants, ma01_003's `<CLICK>` rows).
- From the coverage above: the larger-type boxes, `TransLog`, the
  telops the port rewrote (md05_037vs, mc01_026) at the Japanese
  places, the one-scene boxes, particles, the ignored port arguments.

### Next

1. Play the web build through and fix what is seen (Muramasa's Status).
