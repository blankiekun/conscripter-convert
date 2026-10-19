# 俺たちに翼はない (PS Vita): History

Done items moved out of [the game document](../games/oretsuba.md)'s Status.

- Skip dialogs of S01_12B/S01_13B shared their `*<scene>_offerN` label;
  counted first now, every label in 0.txt defined once (2026-10-18).
- SpriteMove `:Scale -2` (S01_05B): the eboot's sprite quad
  (0x8103d53c) negates both corner offsets (a 180° turn at 2×); SpriteZoom's
  `to` is signed, `spzoom`/`spmotion` flip on a negative percent. Not filmed.
- Reference trace findings (2026-10-18): `\n` in a record is two characters
  and starts a new line (`vpatch.py log` turns it into a newline);
  CharacterActionEnd `:EndWait 1` after a message waits with
  `spmotionwait` (3 of 154 name a character with no action: no-op in the
  eboot too).

### Memory dumps through Vita3K's GDB stub (2026-10-19)

`gdbstub: true` in `<emu>/home/config/Vita3K/config.yml`; port 2159 once the
log says `GDB Server is listening`; **one** connection only (wait on the log
line). `research/tools/gdbdump.py 127.0.0.1:2159 <prefix> --scan
10000:fffe0000:10000` probes every 64 KB (`EAA` outside allocated memory) and
dumps the readable stretches. 50 s after boot: 0x80000000 (4.6 MB, system
modules) and 0x81000000 (5.6 MB, the eboot with data); 5 s. Read with
`dumppack.Memory` / `exe_call.load_dump`. Object pages not looked at yet.

### Status as it stood before the 2026-10-18 prune

#### Queue (2026-10-16)

- Merged: InOut_Act 3 (flip) and 1 (wave) as warp maps (`cons_warp.dll`,
  #227, engine #67/#69); 6 (blur) stays `cons_blur`; the plugins retired
  (engine #72: builds from before #227 play them as cross-fades). Chapter
  card names (#48). Open then: motions and fades as engine motions (#13, #18).
- Dropped arguments picked by the user (2026-10-17): SE fades and waits,
  CharacterDelete `:FadeOut 0`, window fade-out, BacklogMessage `:Message`
  (`logadd`), the shakes' swing (`:Param` 2-5), SpriteQuake `:Wait`: done.
  Left: SceneJumpDialogEnd `:Fade`, SpriteMove `:Roll`. CharacterWait waiting
  for fades needs nothing (no CharacterWait follows a `:FadeWait 0` fade
  without a message between). CharacterActionEnd leaves moves running, as the
  eboot does (0x81127a00 -> 0x811277c0 ends only the action, +0x2e8).
- Vita3K timings stay rough; decompile and our traces are the main checks. `spgrab` (engine #32) could let SpriteBG/BS `:FadeWait 0` (890) cross-fade; not used.

#### Accuracy (value audit, 2026-10-17)

Every hand-typed value in `crates/conv-engines/src/mages/` carries a `// src:`
(eboot address or measured SystemData picture), `// ours` or `// guess:`.
Guesses open: a missing `:Position` (Sprite/SpriteBG/SpriteBS settled: the
picture takes the place of the one it replaces, flag at Sprite +0x560 /
SpriteBG +0x94, `task_run` 0x81021070; with none to replace Sprite gets
(640, 560), SpriteBG's corner (640 - w/2, 360 - h/2); ours centres it; the
characters' flag +0x44 has no reader traced; EffectStart's missing `:Param`
are 0 in the eboot); the text window's place, text origin, font size 23, line
gap, skip dialog font 21; the particles' motion; CONFIG's lit cells (eboot
0x8107145c: 90/180-wide from x 289; ours lose the glow's last 1-3 px).

Tools: `crates/conv-engines/src/mages/` (`cpk.rs`, `tasks.rs`, `convert.rs`,
`assets.rs`; sounds to Ogg via ffmpeg, BGM ADX loop as `LOOPSTART`/`LOOPEND`);
`conscripter-convert mages list|extract|check|convert <game> -o
out/oretsuba-cons [--scenes ..] [--webp] [--dry-run]`. `mages check`: 463
scenes, 65,485 messages. WebP of S01_01A: 3.9 MB PNG vs 1.7 MB.

Output 960x544; script coordinates (1280x720) scaled 3/4 across, 544/720 down;
CG/CHR and SystemData pictures already at that scale. Text window =
`meswin.png`'s left 960 px above the button bar, four lines of 30 at 23 px in
x 120-840, y 40-144 (longest message 109 characters; no paging).

Feature table (condensed; eboot addresses in code comments):
- Sprites: `SpriteBG`/`SpriteBS` replace every picture (characters too unless
  `:SpriteDelete 0`); a scene's first picture removes the previous scene's
  (eboot loads textures anew). `Sprite` adds. S04_16B's `<ST_ASU02H>>`
  resolved by picture name (Asuka 0,0,2,8,0).
- Characters: `:Wait 0` CharacterMove (668) runs under the text, `:Wait 1`
  waits; CharacterWait (651) waits for moves and actions (task_run 0x1e,
  0x23). At y 0 a character stands on the bottom edge (1209 of 1333 pictures,
  630 px tall, are cut along the bottom row); the eboot scales `OffsetY`
  (-68) by 544/720, centred that leaves 8 px above the edge: the anchor is
  unfound. Shorter pictures (960x540 cut-ins) are centred plus `OffsetY`.
  `CharacterDelete` without a list (962 of 1414) removes all;
  `CharacterOpenBS` removes the characters it doesn't name.
- Tints: dusk 255,236,220 (3,892 pictures), night 220,222,255 (2,105),
  `:Grayscale` 479, sepia (`:Gamma`) 188, `:Floor`/`:Ceil` negatives 42 as
  `spfilter`; sepia = monotone, each grey channel to 1/gamma as offset at
  mid-grey (+75, +28, -47).
- InOut_Act (`:Type`, `:Param` 0 = direction 0-3 right, down, left, up):
  0 (21) squeeze and 7 (89) push as scrolls; 5 (9) through white/black; 2 (66)
  mosaic to `:Param` 1 px of 720 halfway; 3 (47) card flip `cons_flip`; 1 (57)
  wave (size a guess: `:Param` 1 x 100 per mille, `:Param` 2 waves); 6 (130)
  blur by `:Param` 1 px of 1280; 4 (2) plain fade.
- SePlay `:Div` n > 1 fades in over n frames of 60/s (`dwavefadein`);
  `:Wait 1` waits the SE length (ADX header). Window `:Visible 0` fades over
  2 x `:Time` frames (500 ms none; step 1/15 per update).
- SceneJumpDialog: global read flag per scene (`%1200` + index in name order);
  asked only when シーンスキップ機能 is on; the line at (81, 99) left empty.
- Shakes: EffectQuake `:Time` 60 frames default, dying with `:Fade 1`,
  `:Loop 1`/negative `:Time` until EffectQuakeEnd; swing by `:Param` 2, 3
  with 4, 5 half-waves (0x81008a3c; SpriteQuake 0x81087114). `:Curve` (12)
  bows a move (0x8103cf20), one `spmotion` key a frame (IR `MoveArc`);
  `:Scale` (90) zooms about the centre (`spzoom`), staying for later moves.
  Easing (ease_value 0x8103cc18: type 1 in, 2 out, 3 in/out, t^power; 65
  uses, IR `MoveEase`): place only; the zoom goes straight with time
  (0x81086a0c), so an eased move that zooms (37) is one key a frame.
- CharacterAction (`chract`, 661 keyframed moves; 72 loop): `spmotion`;
  fields `:X :Y :Roll :Scale :Alpha :Priority` are tracks keyed where given
  (0x81008008), sampled straight at every key (`:Priority` unused); `:Roll`
  radians clockwise (guess, 5 uses); `:Scale` (60) / `:Alpha` (60) =
  `spmotion2`; `:EndSet 1` (475 uses, only when not looping) leaves the
  character at the last key (loop mode 2); `:EndWait 1` 64 uses.
- EyeCatch `:Image 0,N` = CG.cpk ID N + 457 (0x81048978; `EC01`-`EC19`; no
  `EC20`/`EC30`, so 20 = `EC21` 京's after-story, 27 = `EC28` title logo):
  halves (960x272) close like shutters (500 ms, guess) and open over `:Fade`
  (`system/shutters_N_in/out.png`, 10 frames); `:Image 0,-1` (7 uses) shows
  nothing. `:Image 1,card,place,date` (114): card `ec1_bg`..`ec6_bg` (ID 67 +
  card; 0-3 protagonists, 4-5 Karura, 6 奈落の底), place ID 74 + place
  (`ec_bgNN`, A/B/C times), date (0 = 11月21日, -1 none); drawn where the 40 px
  pan ends.
- Movies: USM (MPEG-1 960x544 + ADX) to H.264/AAC MP4 in `movie/`;
  `:Movie` N = OP00, OP01, OP02, seven `ED_*` (alphabetical), Poem, OP03;
  `:Skip -1` skippable once seen (0x81044b74), global flag `%1680` + N.
- EffectStart 57 (50 uses, class 0x81040714): `screenzoom`, scale 1 to
  `:Param` 2, centre to (x, y) over half of `:Param` 3 frames, eased as above.
  53 static, 42 blur. 52/54/51 lightball/paper/snow: 16-frame loops from
  ETC.cpk, snow and paper fall a screen per 2 s (real motion unknown).
  40 (starry wave, `:Param 400,3000`, shader 0x8115f304): row shift 1.5 % of
  width x sin(2pi(600y/(height x P0) + 2t/P1 ms)); `screenripple` at 45°
  (`system/waveN.png`); EffectEnd stops it without fade. Achievement: nothing.
- Title buttons (0x81083fd8): 173x36 cells centred x 786, rows every 41.25 px
  from y 258 (QUICK LOAD, LOAD, START, GALLERY; no quick save, slot empty);
  menu pictures placed by centres. `SystemSE.AWB`: 0 tick on pointer
  (`selectvoice`), 1 confirm (0x8105e7a6, `btnse`) at システムボリューム
  (`Script::menu_sounds`).
- START's story menu (0x8105e69c, 0x81083086, 0x81084c26): opens once 明日香's,
  京's, 亜衣's or 鳴's ending is seen or part 1/2 cleared (global flags 0-5);
  heroine after-stories P03/P01/P02 then open, 狩男's (P04) needs part 1, 2 or
  3 cleared (？？？？？ before). Cells 242x60 at x 788: 始めから y 225, header
  305, after-stories 365/395/425/455.
- CONFIG (Vita look, user 2026-10-15): `bg_config_01/02.png`, `mages/config.rs`,
  IR `PagedSettings`, engine `screen "settings"`; page 1 シーンスキップ,
  音声スキップ (`clickvoicestop`), スキップモード, 選択後のスキップ
  (`selectkeep`), オート速度 (ours, on the 選んだ選択肢の色 row), メッセージ速度
  (4, 7, 9, 10 of 10), 不透明度; page 2 four volumes, BGM lowered for voices
  (to 50 %), キャラクター音声; third page ours. Defaults from 0x81070e70, steps
  of 10: Voice 85, BGM 55, SE 55, System 55, 不透明度 70 %, speed 普通.
- Music room: 67 songs, 3 columns of 23 at 3/5 size; every song open (no lock
  check, 0x81065f02); player bar (0x8106592a, 0x810655a4): ◁◁ ▷▷ ▷, volume
  knob at x 605 + volume x 203/100, 0x810664ca restores on leave; repeat on /
  play-on off set only by the constructor (0x8106551c), so never plays on. The
  second 8-song table (0x81066198) and the large song title are left out.
Full build: ~4 min, 1.7 GB `arc.nsa` + 12 MP4s; lints clean (one false
positive: the save thumbnail name); all eight endings reached.

**Checking a build with the debug trace**: recipe in `docs/new-game.md`; config `research/oretsuba/cons_trace_check.json` (ignore 34-48 = button bar, keep title buttons 6-8).

**Web build.** Loose files (`mages convert --loose`, index with `--lazyload`; 2.2 GB, 27,472 files). Headless Chromium test (2026-10-15, 0.21.0) passed; five bugs fixed (GBK text -> `--enc:sjis`, settings lost on tab close, bar click in a choice, `%3` modulo, slot names).

**Steered route runs (2026-10-15).** `research/oretsuba/routes.py` plays the
eight paths in one save folder (`--auto-choice untaken`, `--taken` marks every
option but the path's `avoid`; `--only 9,10,11,12` = after-stories). Run with a
renamed engine copy (another agent's `pkill -x conscripter` ends the trace
without a reason, exit 0). Each run stops at auto-play's loop guard ("3000
pages without a new label", exit 1): the ending scene matters. Endings: 明日香
S01_16A, 京 S01_18A, 美咲 M09_01, 日和子 S02_16A, 亜衣 S03C_20A, bad end
S04_06L, 鳴 S03_22A, 小鳩 S04_28B; after-stories end P03_01D, P01_01D,
P02_01E, P04_02B. Engine bug found (6837994): `bgmfadeout 16:bgmstop` during
`bgmfadein` left the music playing. EC21 (540 px) centred, 2 px each edge.

**Coverage against the eboot (2026-10-15).** Whole eboot mapped (`docs/new-game.md`
§2): `scene_load` (0x8102840a) keys (`TaskList/Task%d/<key>`, `cfg_find`/
`cfg_number`) against those the scenes use. `task_type_number`: 111 task
types, scenes use 52, all converted. Messages hold no markup but `\n`.
SpriteBG/SpriteBS `:FadeWait 0` (890) still wait (a sprite fade cannot
cross-fade under the text); fades are `spmotion` (loop 0 in, 3 out); a hop
started while the character fades in takes the fade into its keys.

**Arguments not read (2026-10-16).** `Block::read` marks read arguments; each
unread one is a dry-run note "<Task> [<sub-block>] :<Key> ignored";
`convert.rs`'s `NOT_NEEDED` lists the harmless ones with reasons (a row
naming a value covers only that value). `:SpriteMax` limits only
CharacterOpen/OpenBS (scene_load 0x8102840a case 0x1b: Chara0-9 in order, those
with a `:Number`, up to `:SpriteMax`, 0 if missing/empty = none); Delete, Move,
Action (0x1c, 0x1e, 0x20) read all ten. Traced (scene_load 0x8102840a, start 0x8100da30, task_run 0x81021070): shakes (0x81008a3c; SpriteQuake 0x81120894 modes 7 full, 8 `:Fade 1`, 9 loop; EffectQuakeEnd `:Fade` > 0 runs it out); FadeOut/FadeIn `:Type` = rule (0x81049292); MessageSwitch_Tag `:Frame` moves the cell down (0x810160e2); EffectStart `:EffectNumber` arg 1 = variant code (0x8112a860), always 0.

**Exe mapping redone (2026-10-17).** `docs/games/oretsuba.findings.tsv`: 88
rows (71 seen, 10 inferred, 6 unknown). The game updates at 30 fps (0x810009b4
waits two vblanks), script times count 60 fps frames. Fixed: MoviePlay `:Skip -1`
(#359), CharacterOpen `:SpriteMax` (#360). `argcensus.py` oddities explained:
`chvolfade -1` (65,497: music ducking), `spmotion` 20000 ms (S03C_02B pan).
Open: a key with no value (6) is 0 if the parser (0x8108cc48) keeps the key,
ours the default; voice ducking level and ramp (50 %, 200/400 ms) untraced;
`Tag`/`End` are not in task_type_number (0x81035094); only scene tasks checked
(not `chract` per-key `:Priority`/easing nor the `.txt` tables).
Dry-run notes: 2,783 of 80 kinds. Still dropped, by uses: SceneJumpDialogEnd
`:Fade` (531; the eboot keeps the dialog up after the click, task 0x85 fades
over `:Fade`, 18 default, 0x8112a490; on the skip path it stays through
NextScene, 382 of 894 paths; before 22 choices no End comes: untraced);
SpriteMove `:Roll` (12; radians, kept after the move, S02_03F/S03_02A turn a
background; the GPU turn 0x8103d53c direction/pivot untraced, `spzoom`'s angle
could carry it); EffectStart `:Priority`/`:Param` beyond those read (208);
InOut_Act `:Param` 1 for types reading only `:Param` 0 (72); Window `:Time`
with `:Visible 1` (2: fade-in); EyeCatch/MoviePlay `:Color :Fade :EndWait`
(7); SpriteMove/SpriteQuake naming a picture not up (8, 16); CharacterActionEnd
`:Copy` (5). Character anchor: the quad puts the
`:Center` point (default 0,-1, top edge middle) at the position; `OffsetY`
role untraced. Open in the eboot: SpriteQuake's `:Param` layout, whether shake
`:Param` 0, 1 are pixels.

**The boot (2026-10-18; `Script::boot`, #425).** Scene 7 then 1 (0x8103a7b0):
logos `media/img/logo/` makerlogo_WH_vita, brandlogo_5pb_vita, Navel_logo,
criware960x544sw, each on white (0x810815bc), alpha +6 % per 30 Hz update (567
ms), held 4000 ms, circle ends it after 3000 ms, faded out the same
(0x81081c16); then OP00.usm every boot (circle/bit 0x10 stops it, 0x8108205c),
30 updates of black (0x8108237a), the title. Logos 5.13-5.22 s apart in Vita3K,
ours 5.21 s. Not converted: the save-data check between logos and movie (scene
0xf), OP00 replayed from the title (likely idle timeout, unread). The title
fades in from black over 833 ms (0x81085a4c, fade object 0x8104a056, 4 % per
update, 25 updates of 2 frames).

**CharacterStop (2026-10-18)** (case 0x24) stops actions and settles moves: `spmotion N` per character (2 uses). FlagSwitch_Tag with no/unknown `:Operator` is operator 6, a three-way branch on `:NextTag` (<, ==, >); unused.

**SE fades and waits, window fade-out (2026-10-17).** scene_load cases 0x37,
0x38 read SePlay/SeStop `:Div`, `:Wait` (0 default); fades over `:Div` frames of
60/s (0x810370d4, 0x81037794); `:Wait 1` waits while the SE plays (32) or until
the fade-out ends (4). SePlay `:Div` > 1 (31) = `dwavefadein`, SeStop `:Div` > 0
(279) = `chvolfade` to 0 + stop, waits = `delay` (ADX length). CharacterDelete
`[CharaN]` `:FadeOut 0` (85) removes at once (case 0x1c reads `:Fade`, then
`:FadeOut`, then the task's). Window `:Visible 0` (case 0x51, 0x8112a19c): fade
1/`:Time` per update, waited: `windowfade 0,ms` + `delay` (500 ms default, 1,508).
Effect on M02_08: 383 pages wait 500 ms more.

**Backlog answers, swing (2026-10-17).** BacklogMessage `:Message` (56) = `logadd`. Swing: `:Param` 2, 3 sizes, 4, 5 half-wave counts over `:Time` (only shakes without 0, 1 use it: 19 EffectQuakes, 4 SpriteQuakes): sine `screenshake` (speed 32768 x count / ms), SpriteQuake `spmotion` four keys a half-wave; SpriteQuake `:Wait` on a stop (40) waits for nothing.

**Zooms and moves (2026-10-16).** The zoom steps by raw frames, only the place is eased (0x81086a0c): 37 eased zooming SpriteMoves are keyed every frame (`MoveArc { bow: 0 }`). `:Wait` default 1; 0 goes on at once (cases 0x1e, 0x27): 866 moves drop `spmotionwait` (`MoveNoWait`); CharacterWait (0x23) waits for moves, acts and fades, SpriteMoveEnd (0x29) for its `:Sprite`.

**SpriteDeleteBG always fades (2026-10-16).** Case 0x26 reads `:Time`, `:Rule`,
`:FadeWait`, never `:FadeOut`: deletes at once if `:Time` < 1, else fades over
`:Time` (30 default, 0x81129ccc). Four deletes that vanished at once now fade
(S03_02A `spfade`; S03_06G1, G2, S03_07A1 `print` rule039, 500 ms).

**Shakes `:Fade 0`, `:Loop 1` (2026-10-16).** Case 99 (defaults 0x8112a26c:
`:Fade` 0, `:Wait` 1, `:Time` 30, `:Loop` 0): negative `:Time` or `:Loop 1` =
loop, `:Fade`/`:Wait` unread; `:Time` 0 without loop stops the shake
(0x810089a4). `:Fade 0` (1,054 EffectQuakes, 85 SpriteQuakes) shakes at full
size for `:Time`; 30 loops run until EffectQuakeEnd, never wait; 3 `:Time 0`
stops (P04_01B, 01C, 02B); no `:Time` = 1 s; no `:Wait` waits.

**Reading decisions (2026-10-16).** CharacterOpen's own `:Color` (17) is unread
by the eboot (characters open solid). Sprite/SpriteBG/SpriteBS `:Scale` (49;
case 9, 10; stored +0xa0 -> sprite +0x48) zooms about `:Center` (centre for
these) which `:Position` places (`spzoom` in the same print). CharacterMove/
Delete/Action look characters up by the whole key only with `:NumberDirect`
(never used: all 22,904 are `:Number`), so `:Number` args 1-4 are `NOT_NEEDED`
(the whole-key rule would have skipped 52 blocks). `:Chara%d/Copy` (51): number
+ Copy x 1000 (`c1300`, `c2300`).
- SpriteMove `:Curve c` (12, IR `MoveArc`; 0x8103cf20): with eased t and
  d = (dx, dy): position = start + t d + h (dy, -dx)/|d|, h = c sin(pi t)
  |d|/2; c > 0 bows left of the direction; zoom stays straight.
- CharacterOpen `:Chara%d/Center cx,cy` (4 uses, all `0,1`; default 0,-1;
  0x8103d53c): quad x from -w/2 (1+cx) to w/2 (1-cx), y likewise: 0,1 puts the
  bottom edge middle at the position; y counts from the centre raised by cy x
  half height; cx unread.
- EffectEnd `:Delete 1` (7): only frees the slot (0x8103f59c); nothing to do.
- Window `:WindowNumber` -1 (1,506) = the text window; n >= 0 only sets the
  shown flag of an extra box (0x81120abc): nothing.

**Coverage gaps closed (2026-10-15).**
- CharacterOpen/Delete fades (0x8112963c, 0x81129758): 1/9 per update default,
  `:Fade F` steps 2/F (F in 60 fps frames), 0 at once, OpenBS never fades; ours
  18 frames (300 ms) default. Delete's per-character `:Fade` unread.
- `:FadeWait 0` (16,428): CharacterOpen/Delete go on at once when the record's
  +0x10 is 0 (else wait 0x811274c4); Message waits only for its text objects.
  Ours: IR `Op::SpritesFading` -> `spfade` (out `spfade num,-ms`, engine
  535e460), then `print 1`; an expression change (12,276 of 14,146) uses
  another slot of its priority, both linear (the eboot's mix unchecked);
  characters with motion/zoom/look and SpriteBG/BS (890) still wait;
  `--auto-play` spfade is at once (engine 08a782a).
- `:Rule` N: texture 200000 + table[N] (0x811635fc), ETC.cpk file N-1, ID k =
  `RULE<k>`, so `RULE<N-1>`, 0 plain; InOut `:Type` the same (0x81162e38),
  `:Type 0` (422) plain (was `RULE000`).
- Message `:FontScale` (435; +0x8fc) = `!z<percent>` (150 for 1.5); line
  breaks there not compared with the Vita.

**Found by the debug trace (2026-10-14).** Fixed: 347 scenes ended under a black `Sprite` cover (a scene's first picture now clears everything); SpriteBG/BS remove characters unless `:SpriteDelete 0`; `SpriteMove :Scale` now zooms.
**Findings audit (2026-10-18, seed 20261018):** 20 rows re-derived: 17
confirmed, 2 stale, 1 unsupported (CharacterStop, see above), none wrong.
CharacterOpen `[CharaN]` `:FadeWait` and CharacterMove/Action `[CharaN]`
`:Priority` are never read by the eboot.
