# HANDOFF: Muramasa web play-through, JP half (branch muramasa/web-playthrough, PR #641)

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
- --at mb01_001 with paths/1-ichijou.txt: FAILS in mkstart (engrun): 0.txt has `goto *choice207_end` twice
  (lines 71866, 71874, in *f_ma02_014) but no `*choice207_end` label and no select between the
  `*if205_end` (71829) and the first branch: the converter output lost the choice (a build bug, or engrun's
  expectations are stale). engrun then ends in ma02_014 with 16 problems. Needs a decision/fix before the
  route starts.

## Next
1. Resolve the missing *choice207_end (check origin/main `git log -- crates/conv-cons`: #649..#664 changed
   0.txt output); then rerun the --at smoke test.
2. Prologue from the title (s0), 1800 s real time.
3. Route starts via mkstart --at: mb01_001 (paths/1-ichijou), mc01_001 (2-kanae),
   md01_001 (3-chachamaru), md05_001 (4-muramasa); 1800 s each.
4. Per trace: `python3 engine/tools/cons_trace_check.py out/play/<n>/trace.jsonl --script
   out/play/<scratch>/0.txt --config research/nitroplus/cons_trace_check.json`; triage; shots only where
   the trace points.
5. Bugs: one frame each under out/play-bugs/; list in docs/games/muramasa.md Status; push; update PR #641.
