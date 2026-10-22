# Releasing CONscripter

The converter's versions follow the same numbering.

## Which number

- **Patch (X.Y.Z+1)**: bug fixes only: no new command or behaviour, no save block version bump; games built for
  X.Y.0 keep working and their saves load. Ship it as soon as it is merged.
- **Minor (X.Y+1.0)**: any new command or behaviour, a save block version bump, or anything that needs games
  rebuilt with a matching converter. While the version is 0.x, breaking changes go here too.
- **Major**: reserved for the stable engine (1.0).

## Steps

1. **Bump the version** in `asset/icon1.rc`, `src/cons/version.h`, `src/cons_android/app/build.gradle` (also
   `versionCode` + 1), `src/cons_android/app/res/values/strings.xml`,
   `src/cons_libretro/conscripter_libretro.info`, `src/cons_libretro/libretro.cpp` and
   `src/cons_web/conscripter.html` (CRLF: edit it byte-safely). `ONS_JH_VERSION` stays. Done: all seven files
   show the new number.
2. **Merge** the commit "Version X.Y.Z" through a PR (`master` is protected). Done: it is on `master`.
3. **Tag** the merged commit `vX.Y.Z` on `master` and push the tag. If `build_web.yml`, `cross_web.sh` or
   `_fetch.sh` changed, let the `master` run finish first (tags read its caches).
4. **Check the release**: five workflows run on tags (Linux, Windows, macOS, web, Android). Done: the release has
   16 assets and `build_web`'s "web smoke test" step (`engine/tests/web/smoke.py`: the page hooks in headless
   Chrome) is green. macOS ships `universal_darwin` and the same file as `aarch64_darwin` for older GUIs; the
   libretro builds add `.3dsx`, `_libnx.nro`, `_miyoo.so`, `_vita.self` and `.rpx`.
5. **Write the notes**: a changelog from `git log vOLD..vNEW`, grouped by what changed for a game or a player;
   `gh release edit vX.Y.Z --notes-file notes.md`. Done: the release shows the notes (the workflows keep them
   since 0.19.0; 0.17.0 and 0.18.0 have none).
