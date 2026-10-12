# P5 resumed PostScript analysis and dictionary guards

Implemented on 2026-09-30 within
[`p5-61-ps-analysis.md`](../superpowers/plans/tasks/p5-61-ps-analysis.md).

The stem detector now checks three cross-sections through a candidate and tests
every interval between actual curve intersections for filled ink. This prevents
a crossbar from making a large empty counter outrank the glyph's side stems.
Candidates are ranked before the expensive checks; a 65,536-candidate cap and
the existing work budget bound allocation and winding evaluation. This remains
a geometric heuristic, not complete PS analysis coverage.

New regressions cover both axes and reversed contours, retain the curved-bowl
case, and require single-dictionary analysis to reject glyphs assigned to a
different CFF FD without mutation. Native PFA/PFB output rejects multiple private
dictionaries or nonzero assignments before replacing the destination, including
when CID metadata has been removed.

Verification completed: narrow `rustfmt --check` and `git diff --check` pass.
The initial focused test was blocked by in-flight model integration errors;
the queued rerun was cancelled before execution at the root's request to
centralize builds. No runtime test pass is claimed here. Pending integration:

```sh
cargo test -p fc-hint
cargo test -p fc-formats --test type1_hints
```

Owned scratch directories and queued processes were removed. Final resource
check passed: build 6.11 GiB / 13,180 files, temporary 0.23 GiB / 8,103 files,
workspace free 668.68 GiB. Other tasks own the remaining temporary artifacts.

No dependency or FontForge code/executable was introduced. Full variable
autohinting and more advanced PS analysis remain open.
