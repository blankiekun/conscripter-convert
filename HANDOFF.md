# Handoff: oretsuba/quake-path (EffectQuake -> screenshake mode 3)

Done (committed): engine pointer at master (#190); IR `ScreenEffect::ShakeList`
(crates/conv-core/src/ir/mod.rs) and its lowering `screenshake slot,0,interval,3,"x,y ...",loop,0,delay`
(crates/conv-cons/src/ops.rs).

To redo: crates/conv-engines/src/mages/convert.rs. Commit c0a48f68 (pushed earlier,
now rewritten away; `git show c0a48f68:crates/conv-engines/src/mages/convert.rs`)
has the working logic, but that file was run through cargo fmt (the repo is NOT
rustfmt-formatted: never run cargo fmt). Copy only these pieces into the current file:
- `use super::quake::Quake;`
- consts `UPDATE_MS = 33` (0x81050620 queues each frame for 2 vblanks, callback
  0x810009b4 = sceDisplayWaitSetFrameBufMulti(2): 30 updates/s), `QUAKE_TAIL_SLOT = 4`,
  fn `quake_period` (lcm of 50 if :Param 0/1 set, time if swing 4/5 > 0)
- Lowering field `quake_tail: bool` (+ `quake_tail: false` in the 3 constructors),
  method `stop_quake_tail` (clears slot 4 if set)
- the `"EffectQuake"` arm: time = :Time*30/60 (0x81032e2a), abs; params scaled
  x*0.75, y*544/720; non-looped: Quake::frames(usize::MAX), Wait n*33;
  looped: frames(first + 2*period); if periodic from 0 one looped list in slot 1,
  else head (first pass, no loop) slot 1 + tail looped slot 4 with delay first*33
- `"EffectQuakeEnd"`: stop_quake_tail then clear slot 1.

Numbers from the dry run (all 494 scenes): 1071 quake lists, 30 looped (periods 50/150,
11 need the tail slot), longest list 150 offsets (no `end` needed). No looped quake is
still running at a scene label: every loop ends with EffectQuakeEnd in its scene.

Next: rebuild, dry-run diff, scratch check with --dump-frames (one-scene build:
`build <game> -o out/oretsuba-cons --scenes S01_02E` works, 34 MB; the engine build in
this worktree failed at cmake pkg-config: see out/engine-build.log, maybe copy the main
checkout's engine/build setup), source_lint, `cargo test -p conv-engines mages`,
tests/snapshot.sh, oretsuba.md Status item 6 drop, PR.
