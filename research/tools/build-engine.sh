#!/usr/bin/env bash
# Builds CONscripter natively: cmake (RelWithDebInfo) + make.
# usage: build-engine.sh [ENGINE_DIR [BUILD_DIR]]
#   ENGINE_DIR  default: the repo's engine/ submodule
#   BUILD_DIR   default: ENGINE_DIR/build; the binary is BUILD_DIR/conscripter
# SDL2_MIXER_PREFIX: an SDL2_mixer install prefix (include/, lib/) to use
#   when the system has none; it is put on the include, link and rpath.
# JOBS: make -j (default: nproc).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
src=$(cd "${1:-$here/../../engine}" && pwd)
build=${2:-$src/build}
args=(-S "$src" -B "$build" -DCMAKE_BUILD_TYPE=RelWithDebInfo)
if [ -n "${SDL2_MIXER_PREFIX:-}" ]; then
    L=$SDL2_MIXER_PREFIX
    export PKG_CONFIG_PATH="$L/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    args+=("-DCMAKE_C_FLAGS=-I$L/include" "-DCMAKE_CXX_FLAGS=-I$L/include"
           "-DCMAKE_EXE_LINKER_FLAGS=-L$L/lib -Wl,-rpath,$L/lib")
fi
mkdir -p "$build"
cmake "${args[@]}" > "$build/cmake.log" 2>&1 || { tail -n 30 "$build/cmake.log"; exit 1; }
make -C "$build" -j"${JOBS:-$(nproc)}" > "$build/make.log" 2>&1 || { grep -E 'error|Error' "$build/make.log" | tail -n 30; exit 1; }
echo "built $build/conscripter (logs: $build/cmake.log, make.log)"
