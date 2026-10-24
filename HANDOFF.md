# Handoff: Oretsuba live particles (context cap, second agent)

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
- Reading the decompile again found errors in the values below (to be
  confirmed by the eboot run):
  * light balls: vy = 0.2*25/(4.5-c) = 5/(4.5-c) rising (x0.25 only if
    < 0.6, never), not 1.25/(4.5-c); wa=0.2:0.2:1 and x=0:1280 like snow;
    the 600 warm steps run only when :Param 1 (+0x20) is nonzero.
  * confetti spawn: y = 1440U-720; y <= 0 -> top: y = -32, x = 1280U;
    y > 0 -> side at that y in (0,720): x = -32 on the first spawn, on a
    respawn x = 1344 or -64 (U*10 >= 5 -> right). Engine side= is
    p:xl:xr:y0:y1 with y in [y0,y1): use side=0.5:-64:1344:0:720 (the
    first-spawn left-only x=-32 is not modelled). Box: x >= 32+1280,
    y >= 720, x < -64 -> respawn. Flip period int(U)*50+50 = 50.
    Fall +0x2ec and drift multiplier +0x2f0 come from :Param 1/2
    (default 10 each, capped 10).
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
  wv=-1.6:1.6, end=80:-20:-1, warm=600, tint=ffff00 (0xAARRGGBB, settled), twinkle=20:0.032.
- confetti: n<=100, cls=3:1:0.7, walk=y, wv=-0.2:0.6, wa=0.1:0.025:0.4,
  vy=10, x=0:1280, y=-32, side=0.5:-64:1344:-720:0,
  box=-64:-100000:1312:720, drift=0.6:0.2:0.05:10, flip=3:50:0.008:1.2,
  tint=ff0000:ffff00:0000ff:00ff00.
One setlayer per kind (define), configured at start so a loaded save
draws; EffectStart sends "s|..." (restart) and shows sprite "*k"; stop
hides it. Delete assets.rs `particles` strip + PARTICLE_FRAMES.
