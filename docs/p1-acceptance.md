# P1 acceptance

Status: **in progress**, 2026-09-07. This record covers the `p1-outline-editing`
branch. P1 is not complete until the pending integration, browser, localisation
and review items below are resolved. P2 is outside this session's scope.

## Kernel and CLI workflows

`crates/fc-api/tests/acceptance_workflow.rs` opens the licensed DejaVu Sans
fixture, edits a point in each of A, B and C, exports TTF, reparses the result
and checks the changed coordinates. It verifies byte-identical GSUB/GPOS/GDEF
tables where present and an unchanged shaping fingerprint for `Hello fi AVWA`.
The three directly edited glyphs invalidate component dependents too; the
export report is checked against the actual document invalidation count.
DejaVu is the redistributable representative real font used for this workflow.

`crates/fc-cli/tests/acceptance_workflow.rs` creates Scratch Latin through the
API, adds A–C, draws outlines, sets widths to 600/620/640, exports TTF and invokes
the current Cargo-built CLI to inspect, list and shape it. Reopening the SFNT
verifies A's width and outline. Both workflows passed in the implementer's
isolated worktree. Pointer-driven counterparts remain pending UI integration.

## Preservation and shaping

Eight `fc-compile` preservation tests passed. Entirely untouched fonts with
verified original glyph set/order retain `glyf`, `loca`, `maxp`, global hint
programs and glyph instructions verbatim. **Any outline edit drops all
TrueType hinting and emits a warning**, including instructions in otherwise
untouched glyphs. This is the explicit Task 3 fix-1 C1 ruling, a limitation of
P1 relative to the original broader preservation wording. Edited-font tests
verify both global and per-glyph instruction removal.

The Noto shaping round trip passed. Compiled model glyph names are retained
before large-font `post` format 3 removes font-file names; a focused regression
shapes `fi` into an unencoded `f_i` and verifies its model identity. Real Source
Han import also passed the Adobe/Identity CID and `uni4E00` identity regression.

## Performance evidence

Measurements below were taken on this development machine while other worktrees
were active, rather than in an isolated benchmark environment.

| Gate | Observed | Budget | Status |
| --- | --- | --- | --- |
| Rust API `doc.edit_apply`, release | median 0.556 ms; p95 0.602 ms; max 1.411 ms | p95 ≤ 4 ms | Passed |
| DejaVu Sans TTF export, release | 0.582 s | ≤ 5 s | Passed |
| Source Han Sans open, release | 0.900 s | < 3 s | Passed |
| Source Han Sans export, release | 58.697 s | ≤ 60 s | Passed |
| Pointer handler through synchronous repaint, 300 points | Pending browser measurement | ≤ 8 ms | Pending |

The API gate uses exactly 20,000 populated glyphs: one 300-point target and
19,999 four-point outlines with widths and master layers. It performs 20 warmup
and 200 measured calls. Timing includes API dispatch, request decoding, document
locking, actual mutation, coalescing and response construction; it excludes
caller JSON construction, frontend transport and rendering. The test checks
cumulative movement and a single undo restoring the original coordinates.
Run it explicitly with `cargo test --release -p fc-api --test edit_performance
-- --ignored --nocapture`; a debug run cannot satisfy this release-only gate.

The existing canvas `__fwPaint` instrumentation measures renderer duration
only. It must not be presented as the complete pointer-to-paint measurement.

Source Han used the actual 16,529,832-byte external OTF, resolved through the
portable fixture lookup in a fresh linked worktree. The export gate verified
65,535 glyphs and the U+4E00 cmap entry. Neither measurement early-returned.
Export passed with only 1.303 seconds of headroom; this is a narrow margin on
this machine, not a guarantee for every development environment.

## UFO, UI and desktop

Eight `fc-formats` UFO write tests passed, including semantic, incremental and
SFNT/UFO round trips. Full integrated UI acceptance, text editing and preview
checks, Chinese translation with independent QA, and desktop build/launch
smoke are pending. Native dialog interaction will be reported separately from
automated browser or build evidence.

## Evidence and remaining completion steps

Detailed commands and outputs are recorded in the tracked P1 ledger snapshots:
`task-12-kernel-report.md`, `task-12-kernel-review.md`,
`task-9b-shaping-identity-report.md`, `task-9b-kernel-review.md` and
`fixture-path-resume-report.md` and `task-12-cjk-report.md`.

- Finish T8 scoped review and T9b UI implementation/review.
- Execute integrated browser/desktop acceptance.
- Finish T13 translation and a separate QA pass.
- Run whole-branch review, resolve its P1 findings and record final checks.
- Stop after P1 and hand off the branch; merging to `main` is a separate user decision.
