# Cross-document workflows acceptance — 2026-09-30

## Implemented scope

Audit 77: Paste Special selects an open source font, source/target masters and an
ordered source set (explicit names or source-grid selection). Same-name,
selected-destination and as-components placement support all-glyph-data,
selected-master layer, kerning-group membership and metric-key scopes. Preview
runs actual command validation without mutation; apply checks both captured
document generations and writes one undo transaction.

Audit 78: Missing component/construction sources can be recursively copied,
rejected, represented by marked nonexporting placeholders, or decomposed.
Relink Components preserves transforms and validates the resulting graph.
Transfers map compatible masters and semantic CFF private dictionaries; copying
incompatible global TT/color resources rejects explicitly. Whole-layer stable
node/guide references are retained. Existing glyph handles survive replacement.

Audit 96: Compare Fonts returns full-font difference counts and paginated rows;
clicking a row renders exact evaluated outline overlays with optional em
normalization. Both revisions guard a reviewed overlay. Compare Open Family
checks each open member against the target; reference binaries open through the
existing importer. Selected merge reviews and applies the same atomic transfer.

Entry points are Font → Compare Fonts / Family and Edit → Paste Special /
Relink Components. All new English strings use `crossDocument`.

## Verification evidence

Before final review fixes:

```
pnpm --dir app exec vitest run src/views/crossdocument
Test Files  3 passed (3)
Tests       8 passed (8)
```

Focused ESLint passed, and owned frontend files were formatted. Subsequent
review fixes add per-outline viewport bounds, zero-em guarding and two targeted
regression tests; the parent owns the final broad frontend validation.

Backend suites prepared for the parent's serialized final run:

- `cargo test -p fc-api --test cross_paste` (matrix, failure/undo/persistence,
  missing-base policies, mapping/relink and CFF resource semantics).
- `cargo test -p fc-api --test font_compare` (10 tests, including binary reopen,
  topology, pagination, revision guards and hostile component expansion).
- `cargo test -p fc-doc --test replace_glyph` (7 tests, handle/reference retention,
  failed batch rollback, dependent effects, native FCF/FCF.GZ persistence).

Initial Rust attempts hit concurrent incomplete changes; queued reruns were
canceled at the parent's request to let centralized generation/validation run.
They are **not** recorded as passing here. API types are generated centrally.

No headless visual acceptance is claimed: the isolated Vite attempt found its
port already occupied, and the parent explicitly deferred further browser
launches while host load was high. No browser was launched by this owner.
A fresh shared-backend browser pass remains part of integrated acceptance.

## Boundaries

“All glyph data” transfers a glyph record, not unrelated font-wide tables or
kerning pair values. Incompatible master mappings reject rather than guessing.
Appending a missing CFF private dictionary requires an existing CID target;
incompatible non-CID private dictionaries reject. Components plus the decompose
policy is a contradictory combination and rejects. Family comparison uses each
member's first master; detailed comparisons expose explicit master selection.
Paste transactions are bounded to 512 requested glyphs; relink accepts up to
100,000 selected handles. Large-font comparison has explicit geometry budgets.

No FontForge implementation source or new dependencies were used. Parent owns
shared audit/status updates; rows 77, 78 and 96 await combined validation rather
than being promoted on implementation alone.

## Combined-run correction

The first combined run exposed an incorrect rollback fixture: RemoveOverlap
intentionally preserves open contours, so opening a dependent base was legal.
The invalid-source fixture now uses a filled Boolean union, which requires
closed source contours, and asserts exact font/history/generation/cache rollback
after warming the dependent outline cache. A separate regression verifies that
RemoveOverlap accepts open source contours, invalidates its cached outline and
restores the original document on undo. Narrow rustfmt passed; the parent owns
the corrected suite rerun. No production behavior was changed for this fixture.
