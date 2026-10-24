# checks/oretsuba handoff (context cap)
Done: check 2 (global flags the eboot reads: 0-6, 15-17; story menu matches; album card gate missing);
check 1 partly (+0x560 only scene_load Sprite/SpriteBS; per-task records are per-type, so no cross-writers;
sprite +0x50..0x7c written by 0x81086982 init, 0x8108665a / 0x81086a0c moves, scene_load).
Next: does convert.rs map :Grayscale/:Gamma/:Floor/:Ceil/:ColorMode (grep shows 2 mentions each);
decode GXP 0x81160e58 maths (gxp2glsl needs a Vita3K build); YUV GXP 0x811608c8 matrix vs our ffmpeg MP4;
0x8115e000/0x8115e150 (final pass in main loop 0x8104ffe4); render-target format (sceGxm colour format).
Scratch: /tmp/ore/all.txt (whole script corpus, UTF-8, file-prefixed), /tmp/ore/tasknum.txt (task numbers).
