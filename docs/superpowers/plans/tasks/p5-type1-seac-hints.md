# P5 Type 1 seac hint inheritance

Scope: retain representable hint metadata while the existing Type 1 reader
expands StandardEncoding base/accent outlines. Own `type1/hints_seac.rs`, a narrow
hook in `type1/hints.rs`, and `tests/type1_seac_hints.rs`. No writer/model changes.

Implement original bounded composition from Adobe Type 1 section 6.4 and the
sidebearing correction in Type 1 Supplement 5015 section 6. The source outline
evaluator supplies the component programs and orders base before accent. Scan
each independently, translate accent stems by `(parent_sbx + adx - asb, ady)`,
and remap their existing point-bound replacement masks into the combined stem
ordering and contour sequence. Ghost widths remain marker values. Reject nested
seac, drawing/stems before seac, mismatched accent sidebearing, scoped counter
groups, invalid component programs, excessive stems, and mismatched contour
counts through the existing persistent missing-hints diagnostic.

Tests first: nonzero sidebearings, translated/nonuniform FontMatrix, ghost
stems, overlapping component stem sets with independent active masks, empty
component active sets, replacement events and closing boundaries, unsupported
counter/malformed seac cases, PFA/PFB semantic round trips, plus independent
FontTools operand decoding and FreeType source/output outline inspection.

Use `.scratch/p5-type1-seac/` for temporary tests and serialize Cargo using the
resume lock. Run focused Rust tests and formatting; parent runs integrated
Clippy/regression suites. Artifact budget checks apply at task boundaries. No
FontForge sources, executables, new dependencies, commits, or status edits.

## Acceptance checkpoint

Implemented the bounded scanner composition and five integration tests covering
both output containers, sidebearings/matrices/ghosts, unhinted component masks,
replacement closing edges, persistent rejection diagnostics, and independent
FontTools/FreeType checks. Updated the Type 1 user documentation with the exact
supported boundary. Component-scoped counter groups and nonbinding/degenerate
component contours remain explicitly rejected.

`rustfmt --check` on the three changed Rust files and the narrow
`git diff --check` pass. The initial tests-first Cargo command was waiting for
the shared Cargo lock when root requested cancellation to reserve resources for
integration; it ended with exit 143 and never ran. Runtime verification is
**pending**, not passed. Parent integration must run:

```sh
cargo test -p fc-formats --test type1_seac_hints -- --test-threads=1
```

Python FontTools 4.63.0 and FreeType 2.13.2 are available for the independent test.
The task's empty scratch directory and queued Cargo process were removed. Final
artifact check passes: build 6.09 GiB / 13,190 files; temporary 0.19 GiB / 7,215
files; workspace free 668.75 GiB. No full Type 1 hinting completion is claimed.

The first centralized run passed the four native regressions. The independent
test stopped while FontTools parsed the synthetic **input**: its `T1Font.parse`
requires nonnegative `lenIV`, whereas the fixture used legal unencrypted
`lenIV = -1`. The fixture now uses standard four-prefix-byte charstring encryption
with seed 4330. The independent source/output operand, outline and rendering
checks are unchanged; no exporter workaround or skipped assertion was added.
Formatting and whitespace checks pass; the corrected independent run awaits the
root's next focused integration run. The follow-up artifact check passes at
17.54 GiB build / 1,392 temporary files / 657.40 GiB free, with the build-size
warning delegated to the root that owns the shared build cache.
