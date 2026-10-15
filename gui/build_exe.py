"""Builds the app into one executable with PyInstaller, bundling the
conscripter-convert binary and the IPA fonts (run fetch_fonts.py first).

    python gui/build_exe.py [--converter <path>]

Output: gui/dist/conscripter-convert-gui (.exe on Windows; on macOS a
.app folder, since one-file windowed apps are not supported there).
"""

import argparse
import os
import sys
from pathlib import Path

import PyInstaller.__main__

HERE = Path(__file__).resolve().parent
EXE = ".exe" if os.name == "nt" else ""


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--converter", type=Path, default=HERE.parent / "target" / "release" / f"conscripter-convert{EXE}")
    a = p.parse_args()
    a.converter = a.converter.resolve()  # PyInstaller resolves relative paths against --specpath
    if not a.converter.is_file():
        sys.exit(f"{a.converter} not found: run cargo build --release first")
    fonts = HERE / "fonts"
    if not (fonts / "ipag.ttf").is_file():
        sys.exit(f"{fonts} has no fonts: run python gui/fetch_fonts.py first")
    sep = os.pathsep
    args = [
        str(HERE / "app.py"),
        "--name", "conscripter-convert-gui",
        "--windowed",
        "--noconfirm",
        "--clean",
        "--onedir" if sys.platform == "darwin" else "--onefile",
        "--paths", str(HERE),
        "--collect-all", "tkinterdnd2",
        "--add-binary", f"{a.converter}{sep}bin",
        "--add-data", f"{fonts}{sep}fonts",
        "--distpath", str(HERE / "dist"),
        "--workpath", str(HERE / "build"),
        "--specpath", str(HERE / "build"),
    ]
    if sys.platform == "darwin":
        args += ["--osx-bundle-identifier", "io.github.blankiekun.conscripter-convert"]
    PyInstaller.__main__.run(args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
