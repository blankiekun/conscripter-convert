# HANDOFF: Muramasa web play-through, JP half (branch muramasa/web-playthrough, PR #665)

Stopped at the 80k context cap before any play. No bug found yet in play.

## Done
- Worktree + own target/; built: target/release/conscripter-convert, engine/build/conscripter
  (native), engine/build_web/conscripter.{html,js,wasm} (web).
- Finding (committed, PR #641): the web build needs `--loose` (packed arc.nsa 1.9 GB:
  the page sits at "Loading game resources" forever); JP+EN build: 10 min, 6.0 GB peak RSS
  (capped.sh 3 and 4.5 kill it after the text tables; use capped.sh 8).
- web_run.py launches Chromium without --mute-audio (rule breach): worked around with
  CHROME=research/nitroplus/local/chrome-wrap.sh (adds --mute-audio --lang=ja-JP and a CDP
  port). Worth a one-line engine fix later (engine PR).

## Build (finished after the handoff)
- Loose rebuild done: out/muramasa-web (3.3 GB, 0.txt + cg/ sound/ voice/ movie/ screens/), 10:28, 5.9 GB peak.

## Build (2026-10-24, rebased on origin/main, engine at its pointer e1b9ef07)
- Rebuilt: target/release/conscripter-convert (own target/), engine/build_web (cross_web.sh), out/muramasa-web
  (loose JP+EN, `--webp --loose`, command in out/build-game.sh): 7:35 wall, 6.18 GB peak RSS under capped.sh 8
  (it needs 10 GiB MemAvailable to start; retry loop out/build-retry.sh).

## Tools (tracked, research/nitroplus/web_play/)
- mkstart.py <build> <scratch> [--at mb01_001 --path research/nitroplus/paths/1-ichijou.txt]
  [--args '--real-time']: scratch game for the web build (symlinks, patched 0.txt, cons_index.json).
- play.sh <scratch> <name> <seconds> [query]: web_run.py + webshots.py into out/play/<name>/ (NOT
  out/play/<scratch>/); CHROME=chrome-wrap.sh (muted, CDP port). EVERY=10 for a shot every 10 s.
- webshots.py, chrome-wrap.sh: used by play.sh.
- nstrace.py now starts after `game` when 0.txt has no `*start` label (the new 0.txt has none).

## Smoke test
- s0: `mkstart.py out/muramasa-web out/play/s0 --args '--real-time'`, `EVERY=10 capped.sh 4 play.sh out/play/s0 test 90`:
  OK. 760 trace records, pages 2..9 in 90 s, story text in the English box on bg003 (shot
  out/play/test/shots/00080_pg9.jpg). The run exits 1 only because no end record in 90 s.
- --at route starts (2026-10-24): nstrace now reads the current 0.txt shape: `^<key>` lines put back
  from text.ja.txt beside 0.txt (they hold commands too, e.g. a choice's `mov %312,...`), defvar names
  mapped to numbers from 20000 (engrun writes them back as names), choice plates (`btnwait %9` +
  `*conv_cplateN_c`) taken from the path like `select`, texts from their `vstrsp` lines; engrun patches a
  plate's `btnwait %9` to `mov %9,%1098`. mkstart --path takes several paths (comma-separated), played in
  one session: routes 3 and 4 need the earlier endings' flags.
- Scratch build on the fixed 0.txt: out/mw-fixed = out/muramasa-web media + 0.txt, version.txt (copies),
  text.*.txt and screens/ (links) from ~/Projects/conscripter-convert-muramasa-playable/out/mjen-dry.
  All 4 pass with 0 problems:
  `P=research/nitroplus/paths; mkstart.py out/mw-fixed out/play/at-mb01_001 --at mb01_001 --path $P/1-ichijou.txt --args '--real-time'`;
  mc01_001: $P/2-kanae.txt; md01_001: $P/1-ichijou.txt,$P/2-kanae.txt,$P/3-chachamaru.txt;
  md05_001: the same plus $P/4-muramasa.txt.
- Smoke at mb01_001 (90 s, --real-time): f_mb01_001, pages 0..8, village bg with English text
  (out/play/smoke-mb01/shots/00030_pg3.jpg). Missing images, all because out/muramasa-web predates the
  converter fix: system/strip/*.png (plates, quick bar), system/window_*.png, cg/sys/adv/en/textwindow.png
  (no text window drawn), cg/sys/quick/クイックメニュー台座.png. Without --real-time the auto-play run
  stopped "stuck: 20000 waits without a new page" at pg 117 (mb01_002, 0.txt line 233372): recheck on the
  rebuilt game.

## Next
1. Rebuild out/muramasa-web once muramasa/playable (the converter fix) is merged, so images match 0.txt;
   rebuild the scratches with mkstart as above.
2. Prologue from the title (s0), 1800 s real time.
3. The 30-minute runs: route starts via mkstart --at (paths as above), `EVERY=60 capped.sh 4 play.sh
   <scratch> <name> 1800` each.
4. Per trace: `python3 engine/tools/cons_trace_check.py out/play/<n>/trace.jsonl --script
   out/play/<scratch>/0.txt --config research/nitroplus/cons_trace_check.json`; triage; shots only where
   the trace points.
5. Bugs: one frame each under out/play-bugs/; list in docs/games/muramasa.md Status; push; update PR #665.
