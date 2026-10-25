"""Compare JPEG decodes with the exe's (jpeg_exe.py PPMs): per decoder, the max
and mean channel difference and the pixels differing.

    python3 -I research/nitroplus/jpeg_parity.py EXE_PPM_DIR DJPEG9 a.jpg ...

Decoders: `sdl` the native engine's (SDL2_image's IMG_Load through ctypes:
the system SDL2_image and its libjpeg), `pil` (Pillow), `ijg9` (IJG
libjpeg 9f's djpeg, the library emscripten's sdl2_image port links: the web
engine's), `image-rs` (the converter's own decode, through
`conscripter-convert`'s image crate: not reachable from Python, skipped).
"""
import ctypes
import io
import os
import subprocess
import sys

import numpy as np
from PIL import Image


def ppm(data):
    return np.asarray(Image.open(io.BytesIO(data)).convert("RGB"), dtype=np.int16)


class Surface(ctypes.Structure):
    _fields_ = [("flags", ctypes.c_uint32), ("format", ctypes.c_void_p), ("w", ctypes.c_int), ("h", ctypes.c_int),
                ("pitch", ctypes.c_int), ("pixels", ctypes.c_void_p)]


class Format(ctypes.Structure):
    _fields_ = [("format", ctypes.c_uint32), ("palette", ctypes.c_void_p), ("bits", ctypes.c_uint8), ("bytes", ctypes.c_uint8)]


img = ctypes.CDLL("libSDL2_image-2.0.so.0")
img.IMG_Load.restype = ctypes.POINTER(Surface)
img.IMG_Load.argtypes = [ctypes.c_char_p]


def sdl(path):
    s = img.IMG_Load(path.encode()).contents
    f = ctypes.cast(s.format, ctypes.POINTER(Format)).contents
    assert f.bytes == 3, f.bytes  # RGB24 (SDL_PIXELFORMAT_RGB24 on little endian: R first)
    raw = ctypes.string_at(s.pixels, s.pitch * s.h)
    a = np.frombuffer(raw, np.uint8).reshape(s.h, s.pitch)[:, :s.w * 3].reshape(s.h, s.w, 3)
    return a.astype(np.int16)


def main():
    ref_dir, djpeg = sys.argv[1:3]
    print("file\tdecoder\tmax\tmean\tpixels_differing\tpixels_over_1")
    for f in sys.argv[3:]:
        ref = ppm(open(os.path.join(ref_dir, os.path.splitext(os.path.basename(f))[0] + ".ppm"), "rb").read())
        outs = {
            "sdl": sdl(f),
            "pil": np.asarray(Image.open(f).convert("RGB"), dtype=np.int16),
            "ijg9": ppm(subprocess.run([djpeg, "-dct", "int", "-ppm", f], capture_output=True, check=True).stdout),
        }
        for name, a in outs.items():
            d = np.abs(a - ref)
            px = d.max(axis=2)
            print(f"{os.path.basename(f)}\t{name}\t{d.max()}\t{d.mean():.4f}\t{(px > 0).sum()}/{px.size}\t{(px > 1).sum()}")


if __name__ == "__main__":
    main()
