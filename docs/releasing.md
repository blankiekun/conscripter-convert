# Releasing CONscripter

Bump the version in `asset/icon1.rc`, `src/onsyuri/version.h`,
`src/onsyuri_android/app/build.gradle` (also `versionCode` + 1),
`src/onsyuri_android/app/res/values/strings.xml`,
`src/onsyuri_libretro/conscripter_libretro.info`,
`src/onsyuri_libretro/libretro.cpp` and `src/onsyuri_web/conscripter.html`
(CRLF: edit it byte-safely). Leave `ONS_JH_VERSION` alone. Commit
"Version X.Y.0", tag `vX.Y.0`, push `master` and the tag. Five workflows
run on tags (Linux, Windows, macOS, web, Android); the release should
have 9 assets. If `build_web.yml`, `cross_web.sh` or `_fetch.sh`
changed, let the `master` run finish first (tags read its caches).
Then write the release notes, a changelog from `git log vOLD..vNEW`
grouped by what changed for a game or a player:
`gh release edit vX.Y.0 --notes-file notes.md` (the workflows keep the
notes since 0.19.0; 0.17.0 and 0.18.0 have none).
