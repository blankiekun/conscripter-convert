# HANDOFF: nitroplus/jpeg-decode

Done (committed): Django exe loader 0044bea0 = libjpeg 6b defaults (ISLOW, fancy, RGB; struct 0x1b0,
default_decompress_parms 00474cc0). research/nitroplus/jpeg_exe.py decodes with the exe's own libjpeg
(Unicorn); jpeg_parity.py compares SDL2_image (native), Pillow, IJG 9f djpeg (web: emsdk 6.0.12 port
sdl2_image 2.6.0 + libjpeg 9f, no FAST_JPEG). Results jpeg_parity.tsv: native 0 diff, web max 1 / 4 px.
All sampled CGs are 4:4:4 q99 (upsampling moot). Sources pass through byte-identical.

Next: re-encode loss of converter copies (assets.rs: make_scaled q90, mirrored q90, overlaid q92;
Muramasa build: 982 zoom, 80 mirror, 1 overlay; Django build: 691 zoom). Extracted samples in
research/nitroplus/local/jpeg/{mu-build,dj-build} (git-ignored). Plan: decode copy (SDL) vs
flip(exe decode) for mirrors; vs image-rs Triangle resize (unencoded) for zooms. If > 1 level visible,
smallest fix = write the copies as PNG (or quality 100) in assets.rs. Helpers: /tmp/saya-nsaget.py,
djpeg 9f in /tmp/saya-jpeg9/jpeg-9f/djpeg (rebuild: tar of ~/emsdk/upstream/emscripten/cache/ports/libjpeg.v9f.tar.gz).
