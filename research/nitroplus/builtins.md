# Nitroplus engine built-ins (装甲悪鬼村正)

What the ~115 functions that NSS calls but never defines do, in the
order we need them: first everything one story scene uses (text,
backgrounds, fades, sound), then the rest by use count.

Sources, strongest first:

1. **The corpus.** `conscripter-convert nss builtins <nss dir> [NAME…]`
   prints, for each built-in, its call count, argument counts and the
   most common values in each argument position. Counts below come from it.
2. **The NSS helpers that wrap the built-ins** (`function.nss`,
   `function_stand.nss`): their parameter names say what an argument is
   (`CreateTextureSP("ナット名", 描画優先度, Ｘ座標, Ｙ座標, $イメージデータ)`).
3. **Two open reimplementations**, read for semantics only (no code taken):
   FGRE/libnpengine (LGPL-3.0), in the Muramasa-specific fork
   `maosasagawa/muramasa-android-port`, which runs this game's scripts
   up to the story; and CommitteeOfZero/nitrosharp (MIT, written for
   Chaos;Head and Steins;Gate, same script language).
4. **Muramasa.exe itself**, decompiled (see "In the exe" below).
   Strongest for whatever it settles; rows checked there say
   "**exe:**".

**Unverified** marks a row that rests on a guess or on one
reimplementation only. Check those in the exe or under wine before
relying on them (`docs/games/muramasa.md`, "Running the original under
wine").

## In the exe

Muramasa.exe reads the `.nss` text directly. Names below are from
`research/nitroplus/names.tsv`; the Ghidra export command is in
`docs/new-game.md` §2.

- **Statements** go to `dispatch_builtin` (`00405250`). It compares the
  statement text byte by byte with 91 built-in names, each followed by
  `(`, a space, a tab, a full-width space or a line break, and calls
  one handler per name (`builtin_SetVolume` = `00411030`, …). The
  comparison is **case-sensitive**. `research/nitroplus/dispatch.py`
  decodes the table: `python3 -I research/nitroplus/dispatch.py
  <export>/decomp/00405250_*.c` prints name, handler address and name.
- **Values** (built-ins used inside expressions) are matched the same
  way in `eval_primary` (`0043a780`): 31 names, from `Random`,
  `String`, `Count`, `Conquest`, `RemainTime` and `ImageHorizon` to
  `LockVideo` (`research/nitroplus/evalnames.py`).
- **Any other name** goes to `call_nss_function` (`004130a0`), which
  looks for an NSS function with exactly that name. If there is none,
  the statement is skipped: no error, no effect. The scripts call 17
  such names: `XBOX360_LockVideo` and `DialogChangeState` (18 each,
  Xbox 360 leftovers in `sys_*.nss`), `Waitkey` (2, see `WaitKey`),
  and one-off typos or removed helpers (`ShakeOne`, `SceneFadeBG`,
  `CreateTextureadd`, `SL_Rightdown`, `Boxframe`, …). The converter
  matches names exactly too and skips these calls (the build's list
  counts them as `skipped: no function <name>`).
- Built-ins the exe has but the scripts never call: `CreateAnimation`,
  `CreateClipTexture`, `CreateDifferenceTexture`, `CreateFrame`,
  `Disable`, `LoadClipImage`, `LoadFile`, `LoadFont`, `LoadNPT`,
  `MoveCube`, `SetBaseTextInterval`, `SetChip`, `SetComposite`,
  `SetImage`, `UnloadFile`, `WaitMove`, `WaitRotate`, `WaitShake`;
  values `AvailableFile`, `BacklogLine`, `BacklogRow`, `MilliTime`,
  `MountSavedata`, `SoundAmplitude`, `Strlen`.

### Django.exe (続・殺戮のジャンゴ, NitroplusSystem2.0)

The older engine (2008), mapped from its 1.01 exe: names in
`research/nitroplus/django-names.tsv`. The 1.01 update's `Django.exe`
is packed (all sections encrypted, Ghidra finds no function); the
addresses are those of the unpacked 1.01 exe (the no-CD one used under
wine, `docs/games/django.md`). What differs from Muramasa.exe:

- **Statements**: `dispatch_builtin` (`00404320`) compares 75 names the
  same way (case-sensitive, then `(`, space, tab, full-width space or
  line break; `dispatch.py` decodes it). Muramasa's that Django lacks:
  `CreateCube`, `CreateDifferenceTexture`, `CreateStencil`,
  `CreateSurface`, `Escape`, `LoadFile`, `UnloadFile`, `MoveCube`,
  `MoveCursor`, `Position`, `SetFov`, `SetLoopPoint`, `SetScrollSpeed`,
  `SetShortcut`, `SetSurface`, `WaitText`. Every built-in Django's
  scripts call is in the table or the evaluator.
- **Values**: `eval_primary` (`0042b150`) knows 15: `Random`,
  `RemainTime`, `DurationTime`, `PassageTime`, `ImageHorizon`,
  `ImageVertical`, `EnableBacklog`, `ExistSave`, `ReadFile`,
  `BacklogRow`, `BacklogLine`, `ScrollbarValue`, `Time`, `Integer`,
  `EnableDirect3DHAL`. No `String`, `Count`, `Array`, `Conquest`,
  `MilliTime`, `Strlen`.
- **Constants** (`NULL`, `TRUE`, `FALSE`, `AUTO`, the font weights,
  `LOCAL`/`GLOBAL`) are compared letter by letter in either case
  (`== 'N' || == 'n'`), not by the sum of their characters.
- **`$SYSTEM_` variables**: `set_system_variable` (`0045dac0`) takes
  the name after its 8-character prefix (`$SYSTEM_` or `#SYSTEM_`
  alike) and compares it exactly with 35 names, one switch case each:
  `play_speed` (the text speed level, 1-12, default 3), `sound_bgm`,
  `sound_se`, `sound_voice`, `sound_volume_bgm`/`se`/`voice` (0-1000),
  `window_full`, `menu_lock` (turns skip off and shows the text window),
  `backlog_lock` (nothing), `text_auto`, `text_speed`, `text_erase`
  (hides the window), `spt_name` (nothing), `skip`, `keep_skip`,
  `skip_absolute`, `voice_enable_list`, `voice_enable_another`,
  `product_code` (must be `ZokuSatsurikunoJyango` or the game quits),
  `text_interval`, `current_path`, `auto_wait_per_character`,
  `auto_text_break_voice`, `click_break_voice`, `save_last_select`,
  `last_text` (fed character by character to the text log),
  `show_cursor` (falls through into `break_play_movie`: setting it
  sets both), `break_play_movie`, `low_thread_priority` (thread
  priority only), `text_erase_lock` (shows the window), `skip_lock`
  (turns skip off), `video_aspect_fixed`, `save_lock`, `load_lock`.
  The others the scripts set (`menu_enable`, `menu_save_enable`,
  `menu_load_enable`, `menu_config_enable`, `menu_close_enable`,
  `backlog_enable`, `backselect_lock`) are plain variables, read by
  name where they matter: the right-click menu (`0045f5a0`) and the
  input handler (`004503b0`). `backselect_enable` is read nowhere.
- **Stack strings** come out of Ghidra as `builtin_strncpy(local_40,
  "$SYSTEM_",8)` plus array stores; `stackstrings.py` joins both.
- **`Request`** actions are compared exactly (case-sensitive) by a chain:
  the object's own parser first, then `request_render`, then
  `request_lock`. Process: `Open`, `Start`, `Pause`, `Resume`. Sound and
  movie: `Play`, `Pause`, `Stop`, `Resume`, `CompulsorySuspend`,
  `EntrustSuspend`. Scrollbar: `Increment`, `Decrement`, `Next`,
  `Prev`. Every object: `Erase`, `Enter` (starts showing; a text types
  out), `Smoothing`/`UnSmoothing` (texture filtering on/off),
  `NormalRender`/`AddRender`/`SubRender` (blend mode normal, additive,
  subtractive), then `Disused`, `Lock`, `UnLock`. **No `NoLog`,
  `PushText`, `NoIcon`, `Hideable` or `Passive`**: the scripts' 18
  `NoLog`/`PushText`/`NoIcon` requests do nothing in Django.
- **Text tags** (`parse_text_tag`, `00463700`), names in either case:
  `FONT` (`INCOLOR=`, `OUTCOLOR=`, `FACE=`, `SIZE=`), `I`, `U`, `S`,
  `K`, `CENTER`, `MIDDLE`, `PRE`, `WAIT`, `INTERVAL`, `RUBY` (`text=`),
  `VOICE` (`NAME=`, `CLASS=`, `MODE=`, `SRC=`). No `BR` (used once, in
  `$SYSTEM_last_text`) and no `continuation` (only in `//` comments). An
  unknown tag matches no branch and is read up to its `>`
  (**unverified**).
- `_ftol2` is `004b4dcc`; `CreateEffect` kinds are parsed in
  `00422110` (`NegaPosi`, `Monochrome`, `Sepia`, `KitanoBlue`, `Drop`,
  `FloatBubble`, `Rain`, `LookupRain`, `Mosaic`, the waves, the splits,
  `TurnOver`, `Ripple`).

## In the scripts: coverage

What the converter does with every script file and every name the
scripts use (`docs/new-game.md` §2, "Covering the scripts too"; 2026-10-15).
Sources: `nss route`, `nss refs`, `nss builtins` over the 913 files of
`nss.npa`, and the whole-game dry run's notes (`convert-nss … --dry-run`,
the command in the game document). Counts in the "notes" column are
compiled occurrences: a helper's body counts once per call. The gaps
are in `docs/games/muramasa.md`, Status.

**Files** (`nss.npa` 913, `system.npa` 2):

| Files | Status |
|---|---|
| 883 story files (`ma`, `mb`, `mc`, `md`, `mz`) | compiled; `nss route`: all reachable, 9 forks with no `else` (Status) |
| `0_boot_開始スクリプト.nss`, `function*.nss` (8) | compiled as the library, each function where the story calls it; `function_magiccube.nss` ported (`muramasa/magiccube.rs`); `function_select.nss`'s choice plates by `vertical_choice_style`; `function_quick.nss` → `screens.rs` `quick_bar`; process bodies not run (`CreateProcess`, below) |
| `0_boot.nss` | replaced: title and openings (`screens.rs` `title`, `muramasa/opening.rs`); its 13 `SetShortcut` letters → `command_menu` |
| `sys_menu` | `command_menu` |
| `sys_save`, `sys_load` | `save_grid` |
| `sys_config` | `settings` (its defaults, `ConfigFormat`, read for the sliders) |
| `sys_backlog` | `line_log` |
| `sys_close`, `sys_reset` | `confirm` |
| `sys_backselect` | 選択肢に戻る (`BarAction::BackToChoice`) |
| `sys_auto`, `sys_skip`, `sys_screen`, `sys_quicksave`, `sys_quickload` | menu actions and letters A, N, F, Q, P (`command_menu`); `sys_skip` is 超速スキップ, drawn as a plain skip (Status) |
| `sys_hmode` | compiled (its ranking functions, `muramasa.rs`); the favour screen ported (`FAVOUR`) |
| `extra_menu`, `extra_gallery`, `extra_sound`, `extra_recollection`, `extra_dimension` | `muramasa/extras.rs` |
| `sys_edit.nss` | not used: debug editor, its shortcut D is commented out in `0_boot.nss` |
| `prutestselect.nss` | not used: a choice-screen test nothing calls |
| `system.ini` | read for data by hand (keys below) |
| `vssver2.scc` | not used: Visual SourceSafe leftover |

**Functions the scripts define** (1,736 definitions): 585 called from
the story or the library (compiled inline), 273 only run as a
`CreateProcess` body (not converted), 143 called only from the system
and extra scripts (replaced with them), 18 only named by `Conquest` (a
read-mark, not a call), 717 never called by a literal name (dead code:
`function_cockpit.nss` alone has dozens of unused `CP*` helpers). The
16 names nothing defines are skipped, as by the exe ("In the exe").

**Built-ins** (114 names; `story`/`lib`/`sys` = static calls in the story
files / the library / the system and extra scripts; converter as of the
dry run):

| Built-in | story / lib / sys | Converter |
|---|---|---|
| `Delete`, `Wait`, `ModuleFileName`, `SetAlias`, `WaitKey`, `CreateColor`, `RemainTime`, `Draw`, `ImageHorizon`/`Vertical`, `CreateSound`, `CreateName`, `SetLoop`, `Integer`, `Strstr`, `PassageTime`, `DurationTime` | most story | converted: every argument read |
| `Fade`, `Move`, `Zoom`, `Rotate`, `SetVolume`, `DrawTransition`, `Shake`, `SetBlur`, `Random`, `Conquest`, `LoadText`, `WaitAction`, `WaitPlay`, `WaitFade`, `WaitZoom`, `CreateWindow`, `CreateEffect`, `CreateMovie`, `MoviePlay`, `SetBacklog` | most story | **partial**: arguments not read, below (non-blocking `Move`/`Zoom` as cross-fades, Status) |
| `LockVideo`, `WaitText`, `SetFrequency`, `Reset` | most story | dropped (`SetFrequency` noted when not 1000) |
| `Request` | 1,643 / 1,748 / 469 | actions below |
| `DrawTransition`, `Shake`, `Rotate`, `CreateTexture`, `CreateEffect`, `CreateMovie`, `MoviePlay` | story | converted except the noted cases (`Shake` of a small picture 145, `CreateTexture` from an object or the screen 1,812, `CreateEffect` Rain/Lens/Drop 41 and one KitanoBlue box, 3 transition masks missing) |
| `CreateProcess`, `SetFrequency` | 287 / 193 / 30 … | not converted (Status) |
| `SetBacklog` | 83 / 6 / 0 | not converted: 1,958 noted; the hand-made backlog lines (pictures, ASCII art) are missing from our log |
| `CreateWindow` | story | partial: its children are drawn only inside it (`spclip`), not with `Game::relative_children` (続・殺戮のジャンゴ, whose moved windows aren't checked); argument 6 (true/false) not read |
| `CreateText` | 53 / 104 / 40 | story: drawn as pictures in the last `SetFont`'s style (`convert/text_object.rs`), clipped at the box's height (argument 5) as the exe's surface of the box's size; choices and markup left out; sys: replaced |
| `SetVertex` | most story | converted: the point a `Zoom` or `Rotate` keeps in place |
| `SetFont` | most story | converted: its style is `CreateText`'s (the story's text window keeps ours) |
| `LoadImage` | 30 / 16 / 101 | converted: the sources of `CreateTexture` from an object (`builtin_CreateTexture` looks the source up by name, FUN_004491a0, and copies its picture, FUN_00488670, before loading a file, FUN_00488130); the choice screen's (`function_select.nss`) are left out |
| `DrawEffect` | 79 / 0 / 2 | waves converted (`spwave`, Status); Ripple and the Splits noted |
| `CreateStencil`, `CreateMask`, `SetSurface`, `CreateSurface` | 59 / 2 / 0 …; 30 / 1 / 1; 6 / 4 / 0; 0 / 3 / 0 | `CreateStencil` and `CreateMask` converted (children cut by the picture; `docs/games/muramasa.md`, "History"); `SetSurface`, `CreateSurface` not converted |
| `SetLoopPoint` | 0 / 3 / 0 | not converted: the BGM helpers' loop points (33 noted) |
| `String`, `VariableValue`, `Array`, `AssocArray`, `Count` | 2 / 52 / 64 …; mostly lib and sys | converted when the value is known at convert time; else noted (1,658, 1,584, 689, 165): `SetRoll`'s ending tables (Status), `sys_hmode` |
| `CreateChoice`, `SetNextFocus`, `CursorPosition`, `MoveCursor`, `Position`, `Save` | 0 story | lib and sys only: replaced by our choices and screens (`function_select.nss`'s body gives the 153 notes each); `Position` in the battle HUD (`function_cockpit.nss`) goes with `CreateProcess` |
| `ExistSave`, `DeleteSaveFile`, `Load`, `Time`, `ReadFile`, `WriteFile`, `ScrollbarValue`, `CreateScrollbar`, `SetScrollbar`, `SetScrollSpeed`, `SetShortcut`, `LoadColor`, `ClearScore`, `System`, `ClearBacklog`, `CreateBacklog`, `EnableBacklog`, `Message`, `Platform`, `SetStream`, `Exit`, `Escape`, `DateTime`, `EnableDirect3DHAL` | sys only | replaced by our screens and the engine's own save, log and settings |
| `DialogChangeState`, `DialogChangeState2`, `DialogCautionSignIn`, `DialogConfigStorage`, `XBOX360_LockVideo`, `XBOX360_IsSignin`, `SceneFadeBG` | sys only | not built-ins: skipped by the exe too |
| `CreateCube`, `SetFov`, `TextureRoll00H` | lib, 1 each | in process bodies only |

**Arguments not read** (2026-10-16). `Compiler::builtin` notes every
argument a call passes and the converter doesn't read
(`<Builtin> argument <i> ignored`), whatever its value: an argument
not read means our own behaviour applies, so its value may differ from
what we do. Arguments known to change nothing are in `UNREAD_HARMLESS`
(`convert/builtin.rs`), each with the value it covers and why (a `null`
tempo is linear, as ours; `Move` and `Zoom` read theirs; a process's priority and place). Notes from the
whole-game dry run (`ma00_000.nss --chain 2000`), and 続・殺戮のジャンゴ's
(`convert-nss <game> --dry-run`), compiled occurrences:

| Built-in | Argument (meaning) | Muramasa | Django |
|---|---|---|---|
| `LockVideo` | 0 (on/off) | 50,696 | 0 |
| `WaitText` | 0, 1 | 23,689 | 0 |
| `LoadText` | 0, 1 of the 7-argument form, 0 of the 6 | 15,184 | 740 |
| `SetFrequency` | 0, 1, 3 (2 is checked) | 8,724 | 1,283 |
| `Random` | read: picked at run time (`rnd`, "`Random`" in `docs/games/muramasa.md`); 771 calls still picked when converting | 7,411 | 0 |
| `Fade` | 3 tempo (read for a fade over time: its transition's curve; a 0 ms one sets at once, `cb_Fade` 0044f020; not for the text window's); 5 (a sixth argument, 1) | 0 | 0 |
| `WaitAction`, `WaitPlay`, `WaitFade`, `WaitZoom`, `WaitMove` | 1 (a time limit?) | 3,371, 2,477, 4, 3, 0 | 5,852, 51, 1, 0, 2 |
| `DrawTransition` | none: 4 softness read (`effectsoft` for a full sweep; a partial one noted), 2 from read (another opacity than shown noted), 5 tempo read; a missing mask: no transition | 0 | 0 |
| `Conquest` | 0-2 (we return 0) | 2,478 | 0 |
| `CreateEffect` | none: 2-5 (place and size) are read; a tone or `Plain` covering the screen is ours, over part of it noted | 2,477 | 1 |
| `SetBacklog` | 2 (the speaker) | 1,958 | 75 |
| `Shake` | 6 `n`, 7 tempo (read: its step and curve); 8 wait (read for a 0 ms stop; left: a whole-screen shake growing from 0 px) | 0; 7 | 0; 2 |
| `SetBlur` | 4 strength; 2 passes (when off) | 983; 14 | 0 |
| `CreateWindow` | 6 (`false`: children placed from the screen in Django; Muramasa.exe keeps it only in the creation text, 0040b3a0: read) | 0 | 0 |
| `Rotate` | 5 tempo (read: a turn's curve) | 0 | 0 |
| `MoveCamera` | 1-6 | 105 | 373 |
| `CreateMovie` | 2-5 (a missing movie) | 61 | 0 |
| `SetVolume` | 3 tempo (a fade's steps, 00478570; read at 0 ms); 4 | 9; 1 | 0 |
| `MoviePlay` | 1 (a missing movie) | 3 | 0 |
| `Delete` | 1, 2 | 2 | 0 |

**`Request` actions** (static count; converter):

| Action | Uses | Converter |
|---|---|---|
| `Smoothing`, `Passive`, `Erase` | 1,055, 31, 120 | ignored |
| `Disused`, `Lock`/`UnLock`, `Hideable`, `Play`, `Enter`, `Start` (process) | 606, 244/137, 108, 122, 116, 527 | converted |
| `Stop`, `Pause` | 431, 2 | sounds and movies; on other objects noted (16, 2) |
| `AddRender`, `MulRender`, `SubRender`, `OverlayRender`, `NormalRender` | 80, 46, 20, 9, 1 | blend modes: noted (2,555, 420, 60, 111, 1); `SubRender`/`AddRender` on a movie converted |
| `NoLog`, `PushText` | 64, 42 | not converted: the ASCII-art pages (Status) |
| `NoIcon` | 4 | dropped without a note (`@text…` objects the converter doesn't keep): the page's click icon still shows |
| `CompulsorySuspend`, `EntrustSuspend` | 7, 1 | only in `SetRoll`'s and `ProcessRoll01`'s BGM handling (the staff roll, Status) |
| `Clickable`, `UnClickable`, `Prev`, `Next`, `Increment`, `Decrement` | 18, 33, 1 each | system and extra scripts only: replaced |
| `Smooting` | 2 | a typo: matched exactly by the exe, so it does nothing; noted, harmless |

A `Request` on an object the converter has no record of returns without
a note (`request` in `convert.rs`), so the notes undercount: `NoIcon` and
the story `PushText`/`NoLog` on dropped `CreateText` objects show up
only in this table.

**Text tags** (static; the exe's set is under "Text" below): `<voice>`
35,105, `<PRE>` 29,634, `<RUBY>` 13,849, `<k>`/`<K>` 444 converted;
`<BR>` 16 (10 story files, 7 in `ma04_016vs`) converted as a line
break, though the exe's tag list has no `BR` (check under wine whether
it prints or breaks); `<FONT>` 27 in 9 files dropped (Status);
`<VALUE>` 3 (`mc02_031`), converted when known at convert time; `<I>` 1
(`ma04_017`, closed by `</FONT></I>`) converted; `<SPAN value=11>` 2,
`sys_config`'s version page only (replaced).

**`$SYSTEM_` variables** (117 names): the converter keeps them out of
the story's state; read, they have their start value (false, 0). The
story reads `$SYSTEM_skip`, `#SYSTEM_skip_express`,
`#SYSTEM_skip_absolute` (the 超速スキップ `SkipOut` test, 774 files; never
true here, so the scene always plays), `$SYSTEM_present_process` (882
files, the text box name), `$SYSTEM_text_auto` and the
`mousewheel`/`keydown` flags (131 files, the choice loops, replaced by
our choices), `$SYSTEM_effect_rain_*` (17 files) and
`effect_drop_*`/`effect_lens_*` (2 each: parameters of the
`CreateEffect` Rain, Drop and Lens kinds, not converted),
`#SYSTEM_play_speed` (1, the game-speed setting: engine) and
`#SYSTEM_break_play_movie` (3, the 動画 setting: converted). The other
~100 are read only in the system scripts (window, keys, locks, Xbox
360): replaced.

**`system.ini`** keys: `[スクリプト]` `start`/`close`/`menu` → the files
above; `[ウィンドウ] title` → `--title`; `[フォント]` face ＭＳ ゴシック
(a TTF given with `--font`; the scripts' `SetFont` sets the text
boxes' size and colours); `[テキスト] bad_end` (line-end kinsoku:
opening brackets and digits) and `ruby_rate 50` not passed on
(CONscripter's defaults); `[サウンド]` volumes 600/800/800 superseded by
`sys_config`'s `ConfigFormat`; `[アイコン] icon_line`/`icon_page` →
`wait_icon`, `icon_auto` (自動送りアイコン, 15 frames) not used: auto
mode shows the normal icon; `[バックログ]` script and `icon_voice` →
`line_log`, `word_in_row 26` superseded by its layout; `[ボイス]
all_name` → the voice switches (`VOICES`; Status: grouped by `name`,
the exe by `class`); `[プロダクト]`, `[セーブ]`: nothing to do.

**Names the exe matches** (static; converter): `SetTone` Monochrome
98, Sepia 38, NegaPosi 5, KitanoBlue 1: converted as `CreateEffect`'s
tones, on the object itself; `SetShade` levels: converted as blurs (below). `CreateEffect` Rain 37, Plain 33, Sepia 17,
Monochrome 16, KitanoBlue 4, NegaPosi 4, Lens 2, Drop 1: Plain, and
Sepia, Monochrome, NegaPosi and KitanoBlue (the pictures below tinted,
`spfilter`) converted, except a tint over part of the screen (one
KitanoBlue box, noted). `DrawEffect` SuperWave 21, LowWave 25, HighWave 11,
MiddleWave 10: converted (`spwave`); HardSplit 5, Ripple 4, SoftSplit,
Split: not converted. 13 are written with a trailing space
(`"LowWave "`), which the exe accepts (`FUN_0042f770`: a space, tab or
end after the name). Tempo names: Status ("Tempo is
never read").

## Conventions shared by most built-ins

**Objects ("nut", ナット) are named by strings.** A creator
(`CreateTexture`, `CreateColor`, `CreateSound`, `CreateWindow`,
`CreateProcess`, …) stores an object under its name. Creating an object
under a name that already exists replaces it.
- `a/b`: object `b` inside object `a` (windows and choices hold children:
  `"MSGWND/MSG_yes"`, `"絵演窓上/絵演背景"`). The text of a `<PRE @box0>`
  block is the object `box0/text0010`.
- `@name`: look `name` up in the alias table first (`SetAlias(obj, alias)`;
  the scripts nearly always alias an object to its own name). `@@x` = `@x`.
- A trailing `*` is a prefix wildcard: `"絵背景*"` matches every object
  whose name starts with `絵背景`, `"@*"` every alias, `"*"` every object.
  Muramasa uses it constantly (`Delete("*")`, `SetVolume("@mbgm*", …)`).
  **Children** (Muramasa.exe, 2026-10-16): a wildcard is matched against
  the objects of one level; for each match the call goes on to all its
  children (`FUN_0044d2a0` calls itself with `"*"` on the object's child
  list, `+0x12d8`) only when the object's flag at `+0x9d4` is set. The
  object setup (`FUN_0044c8c0`) sets it to 1; `CreateWindow`,
  `CreateName`, `CreateChoice`, `CreateMask` and `CreateCamera` set it to
  0. So a wildcard that matches a picture reaches its children, one that
  matches a window or a name doesn't. Checked under wine from a patched
  `md04_006` (shots in `research/nitroplus/local/shots/star*/`):
  `Fade("@試験親A*", 0, 300)` on a `CreateColor` faded its child too;
  `Fade("@試験窓A*", …)` (a `CreateWindow`) and `Fade("@試験名A*", …)` (a
  `CreateName`) left their children at full opacity. 続・殺戮のジャンゴ's exe
  differs: there a wildcard reaches no child, pictures' included
  (`Game::family`, compared under wine). The converter follows
  Muramasa's rule (`Object::container`, set by `CreateWindow`,
  `CreateName`, `CreateCamera`; `Game::family` keeps Django's).
  `Delete` is the exception: under wine `Delete("*")` takes the
  standing pictures in the locked name `遠３` (`遠３/StL01`) with it, so a
  wildcard `Delete` that matches a container also deletes its unlocked
  children, and deleting a container deletes its children. Also seen in the same test: a picture's child is
  placed from its parent's corner (a `CreateColor` child at (100, 150) of
  a parent at (50, 100) drew at (150, 250)), a window's child at its own
  place; the converter does the same (`Compiler::parent_offset`).
- A string argument may name a variable: `"$SYSTEM_present_text"` means the
  object whose name is in that variable.
- `Request(obj, Lock)` protects an object from `Delete` (including
  wildcard deletes); `UnLock` lifts it. The boot script locks the text
  windows, so `Delete("*")` in `PrintBG` keeps them.

**Coordinates** are pixels on the 1024×576 screen. Each object has an
origin ("vertex", set by `SetVertex`). `CreateTexture` and `CreateColor`
put it at the object's centre, but positions given to them and to `Move` place the
object's top-left corner. Named positions use the object's width `w` (or
height `h`), case-insensitive:

| Name | x | Name | y |
|---|---|---|---|
| `Center` | (1024 − w) / 2 | `Middle` | (576 − h) / 2 |
| `InLeft` / `Left` | 0 | `InTop` / `Top` | 0 |
| `InRight` | 1024 − w | `InBottom` | 576 − h |
| `OutLeft` | −w | `OutTop` | −h |
| `OutRight` | 1024 | `OutBottom` | 576 |
| `OnLeft` | −w/2 | `OnTop` | −h/2 |
| `OnRight` | 1024 − w/2 | `OnBottom` | 576 − h/2 |

`@n` is relative to the current value (`Move(x, 0, @0, @+4, …)` moves down
4 px; `Zoom(x, t, @0, …)` keeps the scale). A bare `@` (a typo that occurs) presumably means `@0`.

**Units.** Times in ms. Opacity 0–1000 (1000 = opaque). Zoom 1000 = 100 %.
Volume 0–1000. Frequency 1000 = normal speed. Angles in degrees. Priority
(描画優先度): higher draws on top (story backgrounds use 10000, the text windows
20000, the text 20001–20002, full-screen covers 24000–30000).

**Tempo** (easing), the argument before `wait` in every timed call:

| Constant | Easing |
|---|---|
| `null` | linear |
| `Axl1` / `Axl2` / `Axl3` | ease-in, quadratic / cubic / quartic |
| `Dxl1` / `Dxl2` / `Dxl3` | ease-out, quadratic / cubic / quartic |
| `AxlAuto` / `DxlAuto` | sine ease-in / ease-out |
| `AxlDxl` / `DxlAxl` | sine ease-in-out / out-in |

**exe** (`apply_tempo`, `00449f40`): `Axl1/2/3` = t², t³, t⁴; `Dxl1/2/3` =
1−(1−t)², ³, ⁴; `AxlAuto` = 1−cos(πt/2); `DxlAuto` = sin(πt/2); `AxlDxl` =
½−½cos(πt); `DxlAxl` = acos(1−2t)/π; anything else linear. Constants such
as tempos and shade levels are matched by the **sum of their bytes**,
case-sensitively (`NULL` = 315 = `off`); a wrong-case spelling falls to
the default. `Request` actions are compared letter by letter.

**Wait**, the last argument of every timed call: `true` (or `1`) blocks
for the call's duration, `false`/`null` returns at once and the action
keeps running. Any other number blocks that many ms (`Fade(x, 2000, 0,
null, 500)` starts a 2 s fade and goes on after 0.5 s; 44 calls do this).
This is what makes overlapping animations: several `false` calls,
then a `Wait`/`WaitAction`.

**Colours**: `#rrggbb`, a bare hex number (`FFFFFF`), or a name
(`"Black"`, `BLACK`, `"White"`, …; case-insensitive).

**Assets**: paths in the scripts (`cg/bg/bg001_空a_01.jpg`,
`sound/se/se…`) are stored in the NPAs without the archive's own folder
and **lower-cased** (`cg/sys/adv/TextWindow.png` is `sys\adv\textwindow.png`
in `cg.npa`), so match them case-insensitively. Sound names have no extension
(`.ogg` is implied). BGM is referred to by objects `InitBGM` creates and
aliases (`@mbgm29`).

## Tier 1: one readable scene

`ma01_001.nss`, a typical opening, uses only these (via the NSS helpers
named in parentheses).

### Text

A text block is a statement:

```
<PRE @box0>
[text0010]
　彼は破壊を求めたわけではない。
<voice name="景明" class="景明" src="voice/ma01/0050030a00">
「……」
</PRE>
	SetTextEXC();
	TypeBeginCIO();
```

Executing the `<PRE>` block (no built-in name; libnpengine's `ParseText`):
deletes the previous text object (the one named in `$SYSTEM_present_text`),
builds a text object `box0/text0010` from the block's markup, and sets
`$SYSTEM_present_preprocess` = `@box0` and `$SYSTEM_present_text` =
`@box0/text0010`. Nothing is shown yet. `box0` is a `CreateWindow` made
once by the boot helpers (`function.nss` line 115: full screen, priority
20001, locked).

Then the helpers (12,098 `SetTextEXC`, 1,788 `…EXR`, 640 `…EXL`, 187 `…EXH`;
C/R/L = window centre/right/left, H = horizontal) lay it out, and a
`TypeBegin*` helper shows it. The `TypeBegin` name says
which window (C/R/L/H/NOH) and what happens after the click:
`I` = keep the box, `IO` = fade the box out afterwards, `IFO` = also remove
the face window, `Time…` = auto-advance variants.

| Built-in | Uses | Meaning |
|---|---|---|
| `SetFont(face, size, colour, outline colour, weight, outline)` | 71 | Default font for text created after it. `SetTextEXC`: `"@ＭＳ 明朝", 20, #FFFFFF, #222222, 500, RIGHTDOWN` (the `@` face is the vertical-writing variant; `RIGHTDOWN` = shadow/outline offset to the lower right; `NONE` none). Weight 500 or `HEAVY`/`MEDIUM`. |
| `LoadText(script, box, text, width, height, letter spacing, line spacing)` | 12 | Lays out the block in `text` (named by variable: `"$SYSTEM_present_text"`) as a box `width`×`height` px. `SetTextEXC`: 550×150, spacing 0, 29 px between lines. Horizontal layout; the vertical look comes from rotating it (next rows). |
| `SetVertex(obj, x, y)` | 370 | Sets the object's origin for `Rotate`/`Zoom` (`center`, `bottom`, px). |
| `Rotate(obj, time, x°, y°, z°, tempo, wait)` | 1,332 | Rotation about the vertex. `SetTextEXC` turns the text 90° about its top-left corner, so a 550×150 horizontal box becomes a 150×550 vertical one: **Muramasa's text is vertical** (tategaki), each written line one column, read right to left. |
| `Move(obj, time, x, y, tempo, wait)` | 5,793 | Moves the top-left corner to `x, y`. The text goes to (575, 55), or (575, 134) under a face window. |
| `Request(obj, action)` | 3,644 | See the list after this table. |
| `WaitText(text, timeout)` | 75 | Waits until the text has finished typing **and** the player has clicked (auto mode/skip advance on their own). `null` = no timeout. `TypeBeginCI` also calls it once before `Enter`, presumably a no-op. |
| `<voice name= class= src=>` (tag in the block) | 35,116 | Plays `src` (+ `.ogg`, from `voice.npa`) when typing reaches the tag; `name` is the speaker shown in the backlog, `class` the per-character voice volume/mute setting (`#ConfigCV_…`). |
| `<RUBY text="…">…</RUBY>`, `<BR>`, `<FONT …>`, `<k>` (tags) | 6,929 RUBY | Ruby, line break, font change, `<k>` = click wait without clearing the page (seen under wine). **exe** (`parse_text_tags`, `00485760`), the full set: `FONT` (`incolor`, `outcolor`, `face`, `size`), `I`, `U`, `S` (strike), `K`, `CENTER`, `MIDDLE`, `PRE`, `SPAN value=`, `WAIT time=` (pause at that character), `INTERVAL` (speed), `RUBY text=`, `VALUE name=` (prints a variable), `VOICE name class src mode` (`mode` defaults to on; `off` unverified). `BR` isn't in it. |
| `{ code }` inside the block | | Runs when typing reaches it (sound effects between lines in `ma01_001` text0040). |

`Request` actions (31 constants; by use):
`Smoothing` (1,029: bilinear filtering on scaled/rotated images, ignore),
`Disused` (593: delete the object once it's idle — sounds when they finish,
textures now), `Start`/`Stop` (511/418: a `CreateProcess` process, or a movie),
`Lock`/`UnLock` (protect from `Delete`), `Play` (sound/movie),
`Erase` (hide; libnpengine: disable), `Enter` (text: start typing it;
others: show), `Hideable` (hidden along with the text window when the
player hides it with right-click), `AddRender`/`SubRender`/`MulRender`
(blend mode), `PushText` (show the whole text at once), `Pause`/`Resume`,
`Passive` (`PrintBG`'s cover; probably "ignores input", **unverified**).
From their use, **unverified**: `NoLog` (64, text kept out of the
backlog, with `PushText` on the ASCII-art pages), `NoIcon` (4, a text
box without the click icon), `Clickable`/`UnClickable` (18/33, a
`CreateChoice` button on or off), `CompulsorySuspend`/`EntrustSuspend`
(7/1, BGM in the staff roll: stopped at once / left to finish),
`Prev`/`Next`/`Increment`/`Decrement` (1 each, `sys_backlog`'s
scrolling), `OverlayRender`/`NormalRender` (9/1, more blend modes).
`Smooting` (2) is a typo the exe's exact match ignores.

**Pages (seen under wine, 2026-10-13, `ma00_000` text0010–text0050):**
a blank line inside a `<PRE>` block ends a page. Each paragraph types
into the empty box on its own, the click-wait icon (a flame, at
`$SYSTEM_position_{x,y}_text_icon` = 452, 555) waits for a click, then
the box clears for the next paragraph. So text0020's 15 lines are 5 pages,
not one overflowing page. Single line breaks inside a paragraph are
column breaks (the authors hard-wrap at ~25 characters). `<k>` waits for a click
in mid-page: the text stays, the `{…}` code after it runs (text0050:
a soldier sprite appears), and the next line types into the same page.
Each `<voice>` paragraph is its own page with the name plate (`SetNwC`)
above it. The window holds 5 columns of 29 px (`textwindow.png`,
134×540 at x = 447, y = 40), and a paragraph never needs more.

For CONscripter: the onsyuri base already has vertical text (`tateyoko`).
Map `SetTextEX{C,R,L}` + `TypeBegin*` to a `setwindow` for that box
(x 447–581, columns from 575 leftwards), with the text emitted as
NScripter text, the `<voice>` tag as `dwave 0` and `[name|voice]` backlog tags as in
saiima. `TypeBegin*IO` → the box fades out with the
text (`erasetextwindow`-style `print`).

### Images

| Built-in | Uses | Meaning |
|---|---|---|
| `CreateTexture(obj, priority, x, y, source)` | 1,077 | Image object from a file (`cg/…`), from another texture/`LoadImage` object, or `SCREEN` (a screenshot, for save thumbnails and transitions). Starts opaque. Helpers: `CreateTextureSP` (+ alias, 3,383), `CreateTextureEX` (2,539: creates at opacity 0 and positions by `Center`/`InRight`/`InBottom`/number itself, because backgrounds larger than the screen are centred), `PrintBG`. |
| `CreateColor(obj, priority, x, y, w, h, colour)` | 149 | Solid rectangle. `CreateColorSP` (2,242) = full screen + alias: the black/white covers for fades and flashes. |
| `CreateEffect(obj, priority, x, y, w, h, effect)` | 114 | Full-screen filter over what's below: `"Plain"` (33: copy of the screen, i.e. a frozen frame), `"Rain"`, `"Sepia"`, `"Monochrome"`, `"NegaPosi"`, `"KitanoBlue"`. `PrintBG(name, prio)` (1,180) freezes the screen in a `Plain` effect, deletes everything else (`Delete("*")`, locked objects survive), and `FadeDelete` then fades the frozen frame out: a crossfade into the new scene. |
| `Fade(obj, time, opacity, tempo, wait)` | 11,975 | Fades to `opacity`. 117 calls have 4 arguments (no tempo); NitroSharp also accepts tempo and wait swapped. **exe**: the arguments are read by position, so a 4th is the tempo and the wait is missing, which reads as "don't wait" (unverified under wine). Helpers: `FadeDelete(obj, time, wait)` (3,584) = fade to 0, then `UnLock` + `Disused`. |
| `Delete(obj)` | 4,982 | Deletes objects (wildcards; skips locked ones). Deleting a process stops it. |
| `Zoom(obj, time, x‰, y‰, tempo, wait)` | 4,679 | Scale about the vertex. |
| `ImageHorizon(obj)` / `ImageVertical(obj)` | 85 / 80 | Width / height in px. `CreateTextureEX` uses them; the converter can read image sizes at convert time. |
| `SetAlias(obj, alias)` | 1,702 | Adds `alias` for `@alias` lookups. Nearly always `SetAlias(x, x)`, which lets `"@x*"` wildcards find it. |
| `DrawTransition(obj, time, from, to, softness, tempo, mask, wait)` | 1,344 | Wipes `obj` in or out through a grey mask (`cg/data/slide_01_01_0.png`, `blind_…`, `circle_…`): opacity from `from` to `to` (0–1000), `softness` the edge width (100 typical). = NScripter `print 18` with the mask, if the mask is converted. |

For CONscripter: object → sprite number from a compile-time table (names
are mostly literals); priority → sprite order (NScripter draws lower sprite
numbers on top, so number = descending priority). `Fade(…, true)` on one
object → set the sprite's alpha, then `print 10,time`; several `false` fades
followed by a wait → one `print`. Different durations, or a fade the script keeps
running while text shows (`false` with no wait after it), need the
Lua fallback.

### Sound

| Built-in | Uses | Meaning |
|---|---|---|
| `CreateSound(obj, kind, file)` | 48 | Loads a sound object; `kind` = `SE`, `BGM`, `VOICE` (picks the volume setting). Helpers: `CreateSE(obj, file)` (3,904; `sound/se/` + file, volume 0, alias), `OnSE(file, volume)` (2,607; one-shot SE on a rotating `OnSE01…10` object). |
| `Request(snd, Play)` | 122 | Starts playback. |
| `SetVolume(snd, time, volume, tempo)` | 2,556 | Volume ramp; `SetVolume("@mbgm*", 2000, 0, null)` fades the BGM out. Wildcards common. **exe** (`sound_set_volume`, `00475f50`): `volume` is 0–1000 (`"@n"` = current + n), clamped; the ramp is linear in that number over `time` ms (time 0: at once). The sound card gets `max(−10000, 3322·log10(volume/1000))` hundredths of a dB (volume times a per-sound factor, probably the volume setting), i.e. −10 dB per halving: 500 = −10 dB, 100 = −33 dB, **1 = −99.7 dB, silent but still playing**. Reaching 0 stops a playing sound. As a linear amplitude (`chvol`, `bgmvol`): 100·(volume/1000)^1.661 %, so 500 → 32 %, 100 → 2 %. |
| `SetLoop(snd, bool)` | 19 | Loop on/off. |
| `SetLoopPoint(snd, start ms, end ms)` | 3 | Loop region. |
| `SetFrequency(snd, time, rate, tempo)` | 182 | Playback speed, 1000 = normal. |
| `SetPan(snd, time, pan, tempo)` | 2 | Pans (exe `0040d140`, as `SetVolume`: an action over time). Converter: `chpan ch,pan[,ms]` on a sound effect (before its `dwave` when it starts with it); a music pan is noted (dja055's chorus parts are BGM objects, one plays) |
| `WaitPlay(snd, timeout)` | 150 | Waits until it stops, at most `timeout` ms (`null`/`true`: no limit; exe `0040b8a0`). Converter: `delay` of the rest of the sound (its length less the time of the ops since its start: waits, fades, an hour for each page, click or choice), at most the limit. A disused one-shot stays until its end, as in the exe (`FUN_00436040` deletes a disused object only once its sound has stopped), so `WaitPlay`/`Fade`/`SetVolume` on `OnSE*` still find it |
| `PassageTime(snd)` / `RemainTime(snd)` / `DurationTime(snd)` | 2 / 123 / 4 | ms played / left / total. |

Helpers: `MusicStart(obj, time, vol, pan, rate, tempo, loop)` (3,824) and
`SoundPlay(obj, time, vol, loop)` (1,643, BGM) = `Play` + volume ramp + loop + `Disused`.
For CONscripter: BGM → `bgm` (+ `bgmfadeout`), one-shot SE → `dwave`
on a channel picked per object name, looping SE (`MusicStart(…, true)`) →
`dwaveloop`; volume ramps → `chvol` steps or Lua.

### Timing and flow

| Built-in | Uses | Meaning |
|---|---|---|
| `Wait(ms)` | 3,939 | Sleep. NScripter `wait` (or `delay`, which a click skips). |
| `WaitKey([ms])` | 1,639 | Wait for a click, or at most `ms` (**exe:** no number = no time limit). `Waitkey` (2, `md04_004`, `md06_035`) is **not** the same: names are case-sensitive and no NSS function has that name, so the exe skips it (see "In the exe"). NScripter `click`, or `waittimer`/`btnwait` with timeout. |
| `WaitAction(obj, timeout)` | 552 | Wait until `obj`'s running animations end. |
| `WaitFade` / `WaitZoom` | 4 / 3 | Same, for one kind of animation only. |
| `ModuleFileName()` | 2,005 | Name of the current script (`ma01_001`), used to form `$GameName`. Compile-time constant. |
| `Reset()` | 886 | In `chapter main` when a story file is started directly: restart through the boot script. Only used for debugging, so ignore it. |
| `Conquest(script, scene, bool)` | 312 | "Already read" query/mark for skip (`$AllRead = Conquest(…)`). Map to NScripter's read-text tracking (`skip` honours it) or a global flag. |
| `Random(n)` | 1,412 | 0 … n−1 (**exe:** generator value % n, seeded from the clock). `rnd`. |

## Tier 2: everything else, by use count

| Built-in | Uses | Meaning | CONscripter |
|---|---|---|---|
| `Shake(obj, time, x0, y0, x1, y1, n, tempo, wait)` | 1,343 | **exe** (`queue_shake` `004462d0`, `shake_frame` `004578e0`): the amplitude goes from (x0, y0) to (x1, y1) over `time` along `tempo`, the offset flipping sign each step; nearly every call ends at 0, a jolt that eases out. `n` above 1000 makes the call do nothing. Django.exe (`FUN_00431de0`, `FUN_0043f570`): the sign flips once 1000 ÷ (`n` × 0.001 × fps) ms have passed, fps 60 at the default game speed, so `n` 1000 is a flip a frame and `n` 200 every 83 ms (docs/games/django.md, History); Muramasa.exe the same (`queue_shake` 00446381-004463c3, fps the double at 005dcb40 + 8 × 4, set to 60.0 at 0048c5d0) | `screenshake` when `obj` is the whole screen (mode 2, to and fro, eased by the tempo); else a one-shot `spmotion` of the picture (`convert/shake.rs`), from the next sync if not shown yet |
| `SetBlur(obj, on, passes, alpha, lag, wait)` | 914 | **exe**: a motion trail (docs/games/muramasa.md, History, "`SetBlur`"); nearly always `true, 1–3, 500, 30–100, false`. `wait` `true` draws the copies before the picture (under it), anything else after it (over it; 00454ca4) | `sptrail` (`,1`: over) |
| `Array(…)` / `AssocArray(…)` / `Count(a)` | 719 / 27 / 57 | Array literal, keyed array, length | converter: NScripter arrays (`dim`) or compile-time tables |
| `CreateProcess(obj, prio, x, y, "Func")` | 493 | Runs NSS function `Func` as a parallel thread; `Request(obj, Start/Stop)`, `Delete(obj)` stops it | Lua coroutine (`docs/games/muramasa.md`, "Approach") |
| `BezierMove(obj, time, curve, tempo, wait)` | 471 | Move along a Bézier path (`(x,y){cx,cy}…` points) | converted: an `spmotion` through the path (Status) |
| `SetNextFocus(a, b, dir)` | 283 | Menu keyboard navigation: from `a`, key `dir` goes to `b` | own menus |
| `CreateWindow(obj, prio, x, y, w, h, clip?)` | 249 | Container object (children `obj/…`) clipped to the rect | sprite group / `lsp` positions relative |
| `CursorPosition($x, $y)` / `MoveCursor(x, y)` | 167 / 62 | Read / warp the mouse | `getmousepos` / menus |
| `SetShade(obj, level)` | 162 | Blur preset; **exe** (`shade_level`, `00465b10`): `EXTRALIGHT` 1, `LIGHT` 2, `MEDIUM` 4, `SEMIHEAVY` 8, `HEAVY` 16, `EXTRAHEAVY` 24, `NOMORE` 32, `NULL` restores the picture; names are matched by the sum of their upper-cased letters, any other sum does nothing. The unit: one pass sets each pixel (all four channels, in place) to the mean of the 3×3 about it; passes add up over calls (`cb_00451f30` keeps the original for `NULL`) | `spfilter` box blur, r(r+1) = 2 × passes |
| `SetTone(obj, tone)` | 143 | `Monochrome` (98), `Sepia` (38), `NegaPosi`, `KitanoBlue`, `null` (off); **exe** (`FUN_00447a00` → `FUN_00452000` per matching object): the object's pixels rewritten in the tone | `spfilter` on the object (a `Plain` cover: on the pictures it froze) |
| `LoadImage(obj, file)` | 136 | Image without display, as a source for `CreateTexture` | |
| `MoveCamera` / `CreateCamera` / `SetFov` / `CreateCube` | 128 / 40 / 1 / 1 | 3D camera over textures (battle scenes) | cameras converted (`Kind::Camera`); `SetFov`, `CreateCube` in process bodies only |
| `String(fmt, …)` | 118 | `sprintf` (`"@noise1%04d"`) | `sprintf`-like via `itoa` + concatenation |
| `CreateChoice(obj)` | 112 | Makes `obj` a clickable button; `select`/`case` test it | `btndef`/`btn` + `btnwait` |
| `VariableValue(kind, name[, value])` | 103 | Read/write a variable by computed name (`$`/`#`) | runtime name table |
| `Draw()` | 97 | Force a redraw now | `print 1` |
| `CreateText(obj, prio, x, y, w, h, markup)` | 91 | Free text object (captions `字幕01`, menus) | `strsp` |
| `SetBacklog(text, voice, name)` | 85 | Adds a backlog entry by hand (lines shown as images) | `[name|voice]` tag / skip |
| `DrawEffect(obj, time, kind, from, to, tempo)` | 79 | Wave distortion (`SuperWave`, `LowWave`, …); **exe** reads 6 arguments (no wait): rows shifted by sin × amp, amp `from` → `to` px over `time`, then held | `spwave` |
| `CreateStencil` / `CreateMask` / `SetSurface` / `CreateSurface` | 59 / 31 / 10 / 3 | Masks and render targets for effects | case by case |
| `LockVideo(bool)` / `XBOX360_LockVideo` | 50 / 18 | Batch screen updates (draw nothing until unlocked) | maps well: `print` only at unlock |
| `Position(obj, $x, $y)` | 47 | Read an object's position | compile-time table |
| `CreateMovie(obj, prio, x, y, loop, alpha, file)` / `MoviePlay(file, skippable)` | 46 / 8 | `.ngs` = MPEG-1 movies; overlays and full-screen | `mpegplay`/`movie`, after converting the files |
| `ExistSave` / `Save` / `Load` / `DeleteSaveFile` | 45 / 18 / 8 / 13 | Save slots (9998/9999 quick saves) | own menus |
| `CreateName(obj)` | 23 | Empty named object (a group) | nothing |
| `DialogChangeState` … `DialogConfigStorage`, `Message(title, text, buttons, icon)` | 18 / 4 | OS dialogs | own menus |
| `Integer` / `Time` / `DateTime` / `Strstr` | 17 / 17 / 1 / 14 | int cast, ms clock, date parts, substring search | NScripter equivalents |
| `ReadFile` / `WriteFile` | 16 / 4 | Save comment files | not needed |
| `ScrollbarValue` / `CreateScrollbar` / `SetScrollbar` / `SetScrollSpeed` | 14 / 7 / 1 / 6 | Config and backlog scrollbars | own menus |
| `SetShortcut(key, script)` | 13 | Hotkeys to system scripts | own menus |
| `LoadColor` | 6 | Colour-filled image source | |
| `ClearScore(LOCAL)` | 5 | Clear local variables | |
| `System("OPEN:url")` | 5 | Open a browser | skip |
| `ClearBacklog` / `CreateBacklog` / `EnableBacklog` | 4 / 1 / 1 | Backlog control | NScripter backlog |
| `Platform` / `EnableDirect3DHAL` / `XBOX360_IsSignin` / `DialogCautionSignIn` | 4 / 3 / 1 / 1 | Platform checks, stubs | constants |
| `SetStream(obj, ms)` | 4 | Streaming buffer for title BGM | skip |
| `Exit()` / `Escape(n)` | 3 / 2 | Quit / break out of n levels (skip mode `BoxOut`) | `end` / converter |
| `Boxframe`, `CreateColorSaddP`, `CreateTextureadd`, `DeleteFw`, `MoveFFPstop`, `SL_Rightdown`, `SL_Rightdownfade`, `SceneFadeBG`, `ShakeOne`, `TextureRoll00H` | 1 each | Typos or removed helpers; the engine presumably ignores unknown calls. Check each call site for dead code | drop |
