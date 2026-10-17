# Releasing CONscripter

Bump the version in `asset/icon1.rc`, `src/cons/version.h`,
`src/cons_android/app/build.gradle` (also `versionCode` + 1),
`src/cons_android/app/res/values/strings.xml`,
`src/cons_libretro/conscripter_libretro.info`,
`src/cons_libretro/libretro.cpp` and `src/cons_web/conscripter.html`
(CRLF: edit it byte-safely). Leave `ONS_JH_VERSION` alone. Commit
"Version X.Y.0" on a branch and merge it through a PR (`master` is
protected); then tag the merged commit `vX.Y.0` on `master` and push the
tag. Five workflows
run on tags (Linux, Windows, macOS, web, Android); the release should
have 16 assets (macOS: `universal_darwin`, and the same file as `aarch64_darwin` for older GUIs; the libretro builds add `.3dsx`, `_libnx.nro`, `_miyoo.so`, `_vita.self` and `.rpx`), and `build_web`'s "web smoke test" step must be green
(`engine/tests/web/smoke.py`: the page hooks in headless Chrome). If `build_web.yml`, `cross_web.sh` or `_fetch.sh`
changed, let the `master` run finish first (tags read its caches).
Then write the release notes, a changelog from `git log vOLD..vNEW`
grouped by what changed for a game or a player:
`gh release edit vX.Y.0 --notes-file notes.md` (the workflows keep the
notes since 0.19.0; 0.17.0 and 0.18.0 have none).
