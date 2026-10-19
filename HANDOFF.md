# Handoff: docs/new-game.md trim (branch docs/new-game-trim)

Done: sections 0-1 and 2 rewritten (commit 1); file was 96290 bytes, now see `wc -c`.
Sections 2 is still ~40 KB; target is under 55 KB TOTAL, so section 2 needs to reach ~25 KB:
cut the "interpreter from a dump" prototype paragraph and the FadeFade worked example to one line each,
shorten Reference traces (Muramasa H-mode patch detail), Trace speeds prose, Picture diff example, Full-route paragraph.
Then trim sections 3-13 and the three trailing sections ("Checking a build with the debug trace", "Test saves", "Film list")
(~45 KB untouched so far). Keep step numbers/headings cited elsewhere (grep -rn 'new-game.md' ...).
Do NOT edit the History sentence in step 0.4. Fallback order everywhere: code -> oracle -> wine last.
Checks: python3 -I research/tools/docpaths.py ; python3 -I research/tools/doclint.py. Delete HANDOFF.md before the final commit.
Drafts of the rewritten parts: /tmp/ng/p0.md p1.md (already spliced in).
