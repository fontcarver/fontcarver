# Font comparison backend acceptance handoff

2026-09-30 — implementation complete; final integrated validation belongs to root.

Implemented `font.compare` and `font.compare_glyph`, registered by the cross-document
parent. Comparison matches persistent names, evaluates components/live geometry,
ignores internal IDs, compares Unicode/variation-sequence sets, horizontal/vertical
metrics and selected-layer topology. Family results cover names, UPEM, axis maps,
master names/locations, glyph inventory and global master metrics. Counts cover
the full comparison; sorted pages optionally omit equal glyphs. Overlay results
include exact evaluated SVG paths, per-side bounds and approximation indicators;
optional generation guards reject stale inspected data. Selected merge reuses
the sibling's reviewed `doc.paste_special` transaction.

Independent review identified and prompted fixes for exponential empty-component
DAG traversal, retained geometry memory, CJK work budgets and stale overlays.
Bounds: 100,000 glyphs/document, 100,000 source points or output elements/glyph,
16 million source/output/expanded work units per document, one million expanded
work units/glyph, and 32 geometry evaluation failures. Memoized component-cost
preflight runs before evaluation; SHA-256 signatures avoid retaining all paths.
Source counts and comparison scope remain explicit; this is not unrestricted
all-table binary comparison.

Ten focused tests are written in `crates/fc-api/tests/font_compare.rs`, covering
diffs, paging, filtering, IDs, Unicode, axes, masters, overlays, binary import,
malformed geometry, expansion budgets and revision guards. Narrow rustfmt passed.
Initial `cargo test -p fc-api --test font_compare` stopped before tests because
concurrent fc-formats metrics/vector modules and fc-geom VariableStroke handling
were incomplete. A second queued attempt was canceled at root's request to reduce
host load (exit 143); it did not execute tests. **No test pass is claimed.** Root
must include this suite in final integrated checks.

Task scratch is removed; no owned builds/browser processes remain. No new
dependencies or FontForge source were used. Final artifact checkpoint is recorded
in the delegated completion report.
