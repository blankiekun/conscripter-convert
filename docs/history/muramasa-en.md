# Full Metal Daemon Muramasa (English, JAST USA 2021): History

Done items moved out of [the game document](../games/muramasa-en.md)'s Status.

### `DrawTransition` as `spfadeto` (2026-10-18)

As in the Japanese build (muramasa.md, History): `spfadeto` form 1 (the
`psUpdateTransitionRule` band) from `from` to `to`, no veil pictures;
gaps 3,190 → 2,572.

### Status as it stood before the 2026-10-18 prune

Condensed 2026-10-19; open items are in Status. `Manual.pdf` in the
install (29 pages, screens of every menu): use it before asking for
screenshots.

#### Exe mapping redone (2026-10-17)

`docs/games/muramasa-en.findings.tsv`. Open: `Move`/`Zoom`/`Rotate` keep
the Japanese curves. No names carry between muramasa_en.exe and
Muramasa.exe (Diesel/Squirrel against NSS). Audit 2026-10-18: tempo
table `AxlDxl1` 17 / `DxlAxl1` 18 at 0x40cec0/0x40cf50, text fade at
004ad32f, choice plates 440×50 confirmed. The English pictures are
`cg.npk`'s under the Japanese names; ours takes them only for name
plates, quick bar and telops (`ENGLISH_FIRST`): inferred.

#### Accuracy (2026-10-17)

Hand-typed English values carry `// src:`, `// guess:` or `// ours`.
Open guesses: the backlog's speaker icon at x 102; the save screen's
thumbnails (158×90) and 300 ms page fade. Text speed (`004b4510`): s =
text_speed ÷ 1000, a character fades in at (212.5 − 5.3125)(1 − s)⁴ +
5.3125 alpha per 16 ms frame, the next 50 alpha later; 0 instant, 58 ms
at the default 550, 150.6 ms at 1000. `<small>` is 50 % (font 20, ruby 10).

#### Eased fades (2026-10-17)

Open: `Move`, `Zoom`, `Rotate` use the same tempo table in the port, so
their `AxlDxl`/`DxlAxl` would be the port's curves; they keep the
Japanese exe's.

#### Engine-drawn screens (2026-10-17)

Extras' menu and CG gallery pages are the Japanese `screen "extras"` /
`"gallery"`; 0 px against main's routine (1 px on an arrow).

#### Queue (2026-10-16)

- Merged: text box #20, settings #21, line log #23 (engine #17),
  save/load #26, title #29, subtitles over the window #35 (engine #21),
  `Labels` out of mages #38, story `CreateText` left out #42, volume
  curve #44, choices from variables #53, #54, #60, #66, story captions
  #158, frozen-screen covers #165, text-area lines #166, pictures over the
  text window #173, code inside a page (engine #63, 7db2f06).
- `metrics.json` gaps: the 444 `CreateText` notes were choice labels (the
  engine's choice), now left out (`CHOICE_SCREEN`); `</voice>` (68)
  changes nothing; 31,735 → 27,552. Left: 4 "text window moved inside a
  page: kept", larger-type boxes (`SetTextEXC_Large` …, port-only).
- `―` (SJIS 0x815C) drew the missing-glyph box (U+2500 absent; font has
  U+2015): engine draws U+2015 then (engine #60).
- Story captions (#158): the port's `LANG_NAME == "en"` branches draw their
  own `CreateText`. `english::put_captions` gives the NSS's `CreateText`
  the English of the port's k-th call on the same sprite (x, y Center/
  Middle, box `auto`, release font at the NSS's `SetFont` size; `Rotate`
  undone; first `Move` takes the port's offset, two-line dates -25/+25;
  an unmatched Japanese line deletes the sprite). 20 calls. Other story
  `CreateText` stays left out (Decided).
- Frozen screen (#165, shared): a `Plain` cover froze the whole screen;
  now only what lies under the top opaque cover stays as it was
  (`sync`); Japanese build has 635 more `print`s.
- Text-area lines (#166): the port's `CreateTextArea` + `SetText` pair
  with the NSS's `CreateText` by Japanese, at the area's place and width
  (R 630,190, 280 wide).
- Pictures over the text window (#173, shared): a picture above the
  window in priority, or as high and made after it, is drawn over the
  window and its text (`spovertext`), as the exe draws by priority. Only
  3 Japanese pages show text under an opaque full-screen picture
  (mc04_022vs `絵演弓聖終` at 29000).
- mc02_031's menus (shared): a page showing a runtime-flag variable
  (`<VALUE name=$料理名前>`) is given once per value behind `if %flag !=
  code` tests; English values via `slot_strs` (`=` and `<-`). No
  variable moves, saves keep working. The ingredient's `Random` is picked
  at convert time (muramasa.md "`Random`"), so the dish is fixed too.
- Code inside a page (shared): a `{…}` block runs where it stands; a
  changed face swaps only the window picture (`setwindow2`,
  `Op::WindowImage`); 5 windows that move inside a page stay where the
  page began. English still shows ma01_026's document whole (the port
  types it by `CreateTextArea` + `FadeCharacter`, not done).
- Decided: a click on an empty slot at the title doesn't save (the
  original saves the title screen): deliberate difference. Continue loads
  the quick save, else the newest. Japanese story captions stay skipped.

- **Works:** `convert-nss <game> --file ma00_000.nss --chain 2000
  --english <english game> …` puts `script.npk`'s English into the
  Japanese NSS (`english.rs`): 38,967 stretches take their row, 43 the
  rows between neighbours', 187 left out as the translation left them.
  Text is one paragraph a line after a backquote (`single_byte_text`);
  boxes become horizontal, 280 wide, pitch 25, 12 lines, in the port's
  pictures (`cg/sys/adv/en/TextWindow(_Fw).png`, 305×475 at x 360, 616,
  103, y 51; `boxH`/`boxTH` x 72 and `boxCH`/`boxCTH` x 264/224 take
  `en/THTextWindow(_Fw).png` 880×158). Text starts at `BoxInfo`
  (`val.nut`, `EN_TEXT`): C 375,190, R 630,190, L 118,190 (280 wide); H
  94,426 (845), 238,426 with a face (701); CH 280,225 (501) / 360,225
  (420); Noh 280,225 (492) / 351,225 (434). Pictures `cg.npa` lacks come
  from `cg.npk` (`NpaAssets::cg_en`). Font `droidserif-muramasa.ttf`
  (`--font`), proportional, wraps at spaces; ㉑-㉗ become spaces; `<I>`
  italic, `<center>` rows centred.
- **Title:** `sys_titlelogo.nut`; logo full screen 40 px up, buttons at
  y 467 (Start 158, Continue 280, Load 402, Extra 524, Config 646, Quit
  768); Continue/Load only with a save, Extra once 英雄編 or 復讐編 is
  over (`hide_without_saves`, `extras_shown_by`); pixel-equal to the
  original.
- **Command menu:** `sys_menu.nut`, `command_menu_en`: fourteen rows at x
  637 from y 90, no 選択肢に戻る (R key), クイッセーブ/クイックロード for
  栞を差す/開く, no ロード; 300 ms fade. Not compared.
- **Line log:** `sys_backlog.nut`: rows 570 px wide measured letter by
  letter (`logrows %n,28,570`, engine #17), 34 px apart from y 22, 14
  rows, indent kept; pixel-equal to the original. `screen "backlog"`
  since 2026-10-17 (engine #107); 0 px against main's routine. Not
  compared: voice icon place; scrolling is the engine's.
- **Save/load:** `sys_saveload.nut`, `save_grid_en`: 15 slots 5×3, tabs
  1-15 … 46-60 and Q.Save, Exit at 0,0; slot popup (Save off on locked,
  at the title, on Q.Save; Load/Lock off on empty; Delete off on empty or
  locked); lock global per slot (`%1011`-`%1110`); an empty slot saves at
  once in the game (`SlotPopup::empty_saves`), play time hours:minutes
  (`play_time_hm`). Deliberate: at the title an empty slot does nothing.
  Q.Save shows our one quick save (the port keeps 20). `screen` since
  2026-10-17 (`save_grid_screen.rs`, engine #105; Q.Save is slot 890).
  Differs from the routine: a click outside the window only closes it,
  the window stays after a refused delete, play time read at open.
- **Settings:** `sys_config.nut`, `settings_en`. First-start values
  (`sys_val.nut`): auto 60, text 550, music 800, effects 1000, voices 700.
  Volume curve (`Mware.dll` 0x1006d830): v^1.661; conv-cons's (p²/50 +
  p)/3 is within 0.4 points above half, 2.4 below. Not compared:
  dragging. Not done: the text sample.
- **Quick bar/questions/telops/subtitles:** English `cg/sys/quick/` (x 998; y 47, 180, 290, 390), `confirm_en` dialogs (400×150, opens over 150 ms), `place_telops`, `ShowVOICESEText` (y 13 or 480, engine #21); none compared with the original. Not done: `クイックセーブ中`, `読み込んでいます`, `削除しました`, `ロック通知`, the settings text sample. Click icon `icon_pos` C 640,500; R 896,500; L 383,500; H 924,554; CH 776,580; Noh 776,329 (`EN_TEXT`). Text box pitch 25 matches the original; `FwB`/`FwCH` faces keep the Japanese place.
- **Decided (the user, 2026-10-15):** font is the release's own
  `droidserif-muramasa.ttf`; menus follow the English port's redesigned
  screens, not the Japanese PC ones; filming the original under wine is
  allowed. **Decided (2026-10-16, lead for the user):** story
  `CreateText` is left out of the English build (every caption takes the
  port's English or goes); the title's empty-slot save is not copied.

### Boot logos (2026-10-18)

`LogoSet` while `TitleFirstLogo` is unset: black; `warning.png` in 300,
`WaitKey(8000)`, `FadeDelete` 300; black 300; `nitroplus-logo.png` in
1000, `WaitKey(4000)`, `FadeDelete` 1000; `Wait(100)`. No JAST logo, no
movie. `MURAMASA_EN`'s `boot_logos` (`muramasa.rs`). `--real-time`:
15,036 ms to the title against the script's 15,000.

### The port's DrawTransition edge (2026-10-18)

`psUpdateTransitionRule`: mask level u (grey ÷ 255) shows at sat(k(now −
u) + u·now), k = 2 ÷ rate; scripts' values arrive ÷ 1000, so k = 2000 ÷
softness (rate +0x4f4, now +0x4a8, `FUN_00435760`). Muramasa.exe's table
widens its band with the threshold; the port's doesn't.
`Game::diesel_transition` gives `effectsoft` form 1 (engine #146) and
`transition::diesel_table` for transitions stopped partway (depends on
the end alone).

### Shake as the port's (2026-10-18)

`CEffectShake`: frequency (1000 / n) * 20 ms (Squirrel integers: 20 for n
501-1000, 40 for 500), NSS tempo as the swings' (`beattempo`), size
straight (00484990, 00484c60). `Game::diesel_shake` (`screenshake` mode
2, `spmotion`), swings as jumps. Gaps 4,786 → 3,302.

- **Choice options at run time (2026-10-16, shared):** options set in
  runtime branches were frozen at build time (mc02_031innsyoku listed all
  five items; md05_004 never offered "Leave your companion"). Variables a
  choice reads (`choice_flags`) stay at run time; text from a string
  variable (`Op::RuntimeChoice`, `$3632` up).
- **Choices from variables (2026-10-16):** 9 `SetChoice` calls name
  variables (mc02_031innsyoku ids 60-90 `Route.選択肢名前1`-`5`; md05_004
  id 20 `Route.md05_hazure`); `slot_strs` values go in order of first
  appearance (the port merged some branches), in assignments and
  comparisons: 27 values. ma05_021vs's debug menu stays Japanese.
- **`<FW>` (2026-10-16):** `SwitchFw('')` deletes the faces, fades
  `box*`/`text*` over 150 ms and goes on in the faceless box; the
  translation adds it before narration in a speaker's stretch (ma01_014).
  The page ends there and the next draw without face until the next code.
- **Subtitles (2026-10-16):** `ShowVOICESEText(sound[, add])` after a voice
  whose `TransVoice[file]` holds a line or list `{time, text, x, y, add}`;
  `serif0`: screen width at y 480, priority 20005, Droid Serif Muramasa 20,
  white, dark edge. `muramasa/subtitles.rs` draws a strip of 250 ms cells
  (last empty) as an animated sprite (`serif0+` for `add`).
- **Text box against the original (2026-10-16):** port lines are 25 px
  apart (`rowSpace` 1 under font height 24, not 21), 2 px lower than
  CONscripter's, set by the font alone (`setwindow3` spacing 1 made lines
  5 % wider): `horizontal_box` pitch 25, `text_drop` 2, spacing 0 for
  `single_byte_text`.
- **Telops at the port's places (2026-10-16):** 47 telops have Japanese
  names but other sizes (chapter titles 528×368 lying; 銀星号の唄 800 wide;
  prefaces full screen at 70 %). The k-th `CreateTexture…`/`Move` on a
  telop sprite takes the port's k-th call (`Function::calls`) when both
  make it as often; a port-added `FadeDelete` goes before the next
  placing call. 465 placed, 7 left.
- **Faces in their frames (2026-10-15, the user):** English `FwInfo`: C
  452,55; R 708,55; L 195,55; H 87,439 (name plate 89,439); `en_face`.
- **English text tags (2026-10-15):** `<WAIT=n>`, `<WAITVOICE=n>`
  (`!dn`, `!vn`), `<RED>`/`<RED2>`/`<RED3>`/`<YELLOW>` = #FF0000, #CC0000,
  #990000, #FFFF00, `<SMALL>` (75 % guessed, later 50 %). CONscripter
  reads `!w !d !z !v !s` and `#`+six hex in 1-byte text.
- **Choices in English (2026-10-15):** `TransChoice["<id>_<n>"]` into
  `SetChoice02`…`06` by Japanese: all 403 options. Plates 440×50
  (`select/en/select001.png`, `002` under the pointer) centred, 100 px
  apart, group centred, text centred 13 px below the plate top
  (`choice_style_en`).
- **Italics and centred rows (2026-10-15):** 2,618 `<I>`, 566 `<center>`
  (unclosed ones end with the row). Engine `!i1`…`!i0`, `!c`, also in
  backquote text; IR `TextStyle` `italic`, `centred`. Backlog upright.
- **Text with no equal Japanese (2026-10-15):** 278 NSS stretches had no
  Print with the same Japanese (lines folded into a neighbour's row;
  edited port Japanese; ma01_026's typed document). `align` pairs
  stretches between two matched rows with the unused rows between them:
  the first takes their English, the others none; with none left the
  stretch is dropped. ma01_026's 48 one-letter stretches take the
  document.
- **Pages in a choice's window (fixed 2026-10-15, shared):** 23 pages drew in a choice's window at `goto` labels; `compile_run` forgets the window per file.
- **Text past the box (fixed 2026-10-15):** 1,143 pages overran 280×150.
  The user chose a CONscripter change over converter page breaks: text
  past the last line waits for a click and continues a new page
  (`pastPage`; the built-in wait, since `textgosub`'s return would drop
  the rest of the line).

### Character voices window (2026-10-18)

Settings' Individual (557,263) opens `settings_voices` (`VoiceWindow`):
`cv/ボイスセレクトウィンドウ背景` at (0,0), the 55 `SYSTEM_CV` names four to
a row at (118 + 204 (i mod 4), 69 + 32 (i div 4)), shown once their
`#voice_on_` flag is set (`IsVCOpen`; the two "others" always); 200 ms
fade; right-click or C closes. Each voice line is `logvoice` + `if %97x
== 0 dwave`. Ours: the 15×15 button 2 px lower is the whole picture; the
pressed cell 1 px left. `cv/1章_` pictures unused by the port.

### Log lines without the port's source (2026-10-18)

`SetBacklog` took the port row raw (tags, `//` lines, newlines leaked 71
lines into the window); now parsed like story text (`log_text`,
`nitroplus/english.rs`), lines joined by a space.

### Back to the choice by R (2026-10-18)

`cmdcheck.py --counts` showed `savegame` 2 against 170. The English menu
has no 選択肢に戻る row but the port keeps it on R (`sys_key.nut`
`KEY_CODE_R` → `BackToSelect`); `CommandMenu::keys` carries row-less
shortcuts; 169 `savegame` each. The `select` gap (8 against 168) is
intended: up to four options are plates (`choice_plates`, `btnwait`); the
8 left are 5-6 option choices (the `shortcut:` on `choice_style_en`).
