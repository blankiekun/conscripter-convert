# Handoff: Oretsuba live particles

## Fourth agent (2026-10-23), stopped at the context cap
- Done, pushed: engine pointer at master 27988f5 (#191 merged); IR
  `Op::Particles` (conv-core ir/mod.rs, end of Op); conv-cons
  `particles.rs` (setlayer 1,16,"particle"; handle %conv_particles under
  the animation; $conv_particles_message restarts it after csp -1 and in
  *conv_load_clear); mages `particles.rs` (keys per kind with eboot
  addresses, 0.05-class speed tables), assets.rs strip/PARTICLE_FRAMES/LCG
  deleted. Builds; `cargo test -p conv-engines mages` 15 ok. Dry-run full
  build: 41 starts (lightball 34, 23 with warm=600; paper n=40 x6 vy=1.5;
  snow n=30 x1).
- Deviations (ours, to report): confetti first side birth at -64/1344
  (eboot -32); speed tables in 0.05-class buckets (midpoint), steep near
  snow c=4.5 (c=4.4: table 5.5 vs eboot 4.125); link scenes don't save
  the particles (Oretsuba has none).
- Next: step 3 numeric check. One-scene loose builds exist:
  out/ore-S03C_15A (snow), out/ore-S02_03G (paper), out/ore-S01_04N
  (light). Harness per kind: dir with `system` symlinked to the build's,
  0.txt = `;$S960,544,value200,V8192` / *define / setlayer 1,16,"particle"
  / game / *start / mov $9,"<message from the build's 0.txt>" /
  layermessage 1,$9 / lsp 1,"*1",0,0 / print 1 / loop: delay,
  layermessage 1,"t":getret %2, "p":getret $2; run a renamed copy of
  /home/saya/Projects/conscripter-engine-particles/build/conscripter
  (tree == master) with SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy
  --trace t.jsonl --save-dir <scratch>; trace "var" records hold $2.
  Match particles across dumps by class c; compare dy/tick with 2·v(c),
  x/y ranges and alpha with particles_exe.txt.
- Then steps 4-5 of the brief: source_lint, `cargo test -p conv-engines
  mages`, cargo test + clippy (shared code), tests/snapshot.sh (only
  oretsuba), Status item 7, PR (none yet), delete HANDOFF.md and
  /home/saya/Projects/conscripter-engine-particles/build/ptest.


## Second agent (2026-10-23), stopped at the 100k cap
- Engine: src/cons/builtin_dll/layer_particle.cpp written (engine
  commit f875cfc, pushed on particle-layer): message parser (every key
  below), pool, step/spawn/tick/update/refresh, tinted sheets. NOT built
  yet. Still to do: setlayer hook ("particle" in
  ScriptParser_command.cpp setlayerCommand), both CMake lists, COMMANDS.md,
  desktop + web build, numeric check, PR.
- Colour word SETTLED: 0xAARRGGBB. The draw 0x8108815a unpacks the word
  (effect+0x288 = draw param+0x1f0, lines 335-346 of the decompile) into
  the float4 uniform it sends with SceGxm_65DD0C84 (8 floats from
  &local_104) in the order (w>>16)&ff, (w>>8)&ff, w&ff, w>>24, i.e. RGBA.
  So 0xffffff00 (light balls, 0x810438d2) is yellow: tint=ffff00. The
  confetti bytes +0x50/51/52 go to bits 16/8/0 = R, G, B.
- Lead's course correction (user 2026-10-23): get the values by RUNNING
  the eboot with research/tools/exe_call.py (pattern
  research/oretsuba/quake_exe.py on origin/main): ctor + N updates per
  kind, rand stubbed, dump the particle records; commit the script under
  research/oretsuba/ and compare the engine against those numbers. Not
  started. Imports to stub: rand 0x81131cc4, operator_new 0x81131e14,
  operator_delete 0x81131de4; inner ctor calls 0x8103fde8, 0x81085e66,
  0x81086982 (stub). Pools: snow DAT_81162b10 (ctor 0x810479f4, start
  0x81047ccc, update 0x81047dfa), light DAT_81162718 (ctor 0x81043604,
  start 0x81043902, update 0x81043c8a), confetti DAT_81162588 (start
  0x81042f66 reads count +0x1c, fall +0x20, drift mult +0x24; update
  0x81043464 -> step 0x8112ad40). Records: [3]=x [4]=y [0]=vx [1]=vy
  [2]=ax [5]=active byte [6]=c [7]=x0 [8]=xw [0xb]=alpha (snow/light).
- Eboot run DONE (third agent, commit ea6af4fe):
  research/oretsuba/particles_exe.py, fixture research/oretsuba/particles_exe.txt
  (particle 0 per update; rand per call site: spawn tests 0, rest U).
  Settled by the run (Status item 7 updated):
  * light balls vy = 1.25/(4.5-c) (x0.25 always applies): c=0.3 0.297619,
    1.5 0.416667, 3.7 1.5625. The "5/(4.5-c)" re-read was wrong.
  * confetti spawn y = 1440U-720: <= 0 top (y=-32, x=1280U); > 0 side at
    that y in (0,720): first x=-32, respawn x=-64 if 10U > 5 else 1344
    (0x8112b11c). 1344 >= 1312 respawns at once; -64 plus leftward drift
    < -64 respawns at once. side=0.5:-64:1344:0:720 approximates this.
  * warm: light 600 steps only when :Param 1 != 0 (rand calls 200 vs
    120400; scenes pass 0 or 1); confetti 1200 steps only when :Param 3
    != 0 (scenes always 0).
  * confetti :Param 2 (drift mult) 0 stays 0; :Param 1 (fall) 0 -> 10.
    The scenes' only confetti :Param is 40,1.5,0.5,0: fall 1.5, mult
    0.5 (drift=...:...:...:0.5, vy constant 1.5, not 10).
  * snow c >= 4.5 (U1 >= 0.875 and U2 > 0.7, 3.75%): vy < 0, rises for
    good, never respawns. Snow :Param 30 in its one scene.
  * spawning step does not move; respawn keeps vx/ax; confetti flip
    counts updates (time - last >= 50), not steps; cell column = frame.
  * every class table (DAT_81162568, DAT_81162578) is 32.

## Done
- Step 1: eboot law read; table in docs/games/oretsuba.md Status item 7
  (commit 81de2d53 on oretsuba/particles, pushed). Decompiled C:
  research/oretsuba/local/ghidra/decomp/ (main checkout).
- Step 2 decision: FuruLayer can't reproduce it (fixed sine sway, 3 fixed
  fall multipliers, no per-particle alpha fade, depth end line, tint,
  twinkle, side spawn, flip). New generic layer.
- Engine worktree /home/saya/Projects/conscripter-engine-particles,
  branch particle-layer (pushed, no PR yet): `ParticleLayer` declared in
  src/cons/builtin_layer.h (Params struct = every key below),
  `ONScripter::getGameSpeed()` added.

## Next (engine)
1. src/cons/builtin_dll/layer_particle.cpp (copy FuruLayer's loadImage).
   - message "s|key=v,key=a:b:c,..." resets prm, loads sheet, makes the pool
     (c from cls; wv=0; wa=U*a0; drift=(k0*c+k1+k2*U)*random sign; ramp=1;
     fdir=+1; frame=0; last=0; tint random; inactive), runs `warm` steps,
     running=true, time=0. "x" stops/clears; "n" returns active count.
   - Keys: img cw ch n hz steps warm spawn scale ox oy cls=a:b:q
     (c = U*a + (U>q ? b : 0)) vy=v0:v1:... (table over c in [0,a+b))
     x=x0:xw y=y0:yh side=p:xl:xr:y0:y1 walk=x|y wv=lo:hi
     wa=amax:noise:a0 end=e0:e1:dir fade=step:min box=x0:y0:x1:y1
     drift=k0:k1:k2:speed flip=frames:period:add:mul tint=rrggbb:...
     twinkle=n:step.
   - step(): inactive: U<spawn -> spawn. Active: x += fdir*drift*speed*ramp,
     ramp=(ramp+add)*mul capped 1; walk axis w += wv, wv += wa,
     wa += (U-0.5)*noise, clamp; y += vy[c]; end: past line (dir) ->
     x outside [x0,x0+xw] respawn, else alpha-min<=0 respawn else
     alpha -= step; box: outside -> respawn; flip: time-last>=period ->
     frame ping-pong over `frames`, at the ends fdir=-fdir, ramp=0.
     spawn(): side with prob p (x = xl or xr, y in [y0,y1)), else
     x0+U*xw, y0+U*yh; alpha=255; active.
   - tick(): `steps` x step() on all, then twinkle waves (+-step a tick,
     bounce 0..1; start U, random dir), time++.
   - update(): SDL_GetTicks delta * getGameSpeed()/100 into acc; ticks of
     1000/hz ms, at most 8 an update, drop the backlog.
   - refresh(): per active particle, sheet[tint] (cells re-packed in one row,
     index row*cols+frame, row=int(c)), alpha*twinkle[i%n], at
     (x*scale+ox, y*scale+oy) via blendOnSurface (offsets by current_cell).
     Sheets: tinted copies (RGB multiply), TRANS_ALPHA, setupImageAlpha,
     pos.w=cw pos.h=ch, loop_mode 3.
2. setlayerCommand (ScriptParser_command.cpp): "particle" -> new
   ParticleLayer(screen_width, screen_height, script_h.cBR).
3. CMakeLists.txt:588 and src/cons_libretro/CMakeLists.txt:57: add the file.
4. COMMANDS.md "setlayer": document the keys. CRLF check per file.
5. Check numerically (trace / --dump-frames), PR -R blankiekun/CONscripter.

## Next (converter, after the engine PR merges)
Converter values per kind (eboot units; scale=0.75555557, ox=oy=31 snow /
32 others, hz=60, steps=2, spawn=0.002, cw=ch=32):
- snow: n<=200, cls=4:1:0.7, vy = table of v(c) (5/(4.5-c), >5 /5, >4
  x0.55, >3 x0.75; c in [0,5), 50 entries), x=0:1280, y=-720, walk=x,
  wv=-1.3:1.3, wa=0.2:0.2:1, end=470:50:1, fade=20:25.
- light balls: cls=3:1:0.7, vy = -1.25/(4.5-c) (40 entries), y=1440,
  wv=-1.6:1.6, end=80:-20:-1, warm=600 only when :Param 1 != 0, tint=ffff00 (0xAARRGGBB, settled), twinkle=20:0.032.
- confetti: n<=100, cls=3:1:0.7, walk=y, wv=-0.2:0.6, wa=0.1:0.025:0.4,
  vy=<:Param 1 fall, scenes 1.5>, x=0:1280, y=-32, side=0.5:-64:1344:0:720,
  box=-64:-100000:1312:720, drift=0.6:0.2:0.05:<:Param 2, scenes 0.5>, flip=3:50:0.008:1.2,
  tint=ff0000:ffff00:0000ff:00ff00.
One setlayer per kind (define), configured at start so a loaded save
draws; EffectStart sends "s|..." (restart) and shows sprite "*k"; stop
hides it. Delete assets.rs `particles` strip + PARTICLE_FRAMES.
