# P5 incremental layout compilation

## Scope and ownership

Add an incremental shaping compiler in `fc-compile` and connect it to the
`fc-layout` debounced cache. The first compile and non-layout changes use the
existing full compiler. Feature, kerning and anchor-only edits reuse the exact
glyph order and non-layout tables and rebuild layout through the same fontbe
writers used by export. Preserve outlines for fallback mark placement and
point-indexed attachment. Imported layout remains on the verified full path.

Own the new compiler module, its focused tests, the compiler re-export and
dependency declaration, and `fc-layout/src/shaping.rs`. Coordinate shared
compiler source changes with the feature token authoring agent. No UI changes,
document mutations, commits or status-table changes belong to this task.

## Acceptance

Demonstrate feature-only substitution and positioning updates, automatic
kerning and anchor writer equivalence, source deletion, failure recovery, and
full invalidation when glyph identity, cmap, metrics or outlines change. Assert
non-layout table retention and a generous timed incremental compile budget.
Run formatting, focused compiler/layout tests and Clippy; the parent owns final
workspace validation. Serialize Cargo using `.scratch/p5-resume/cargo.lock`.
Check artifact budgets at start, between build batches and completion. Keep
temporary test data under `.scratch/p5-incremental-shaping`, then remove it.
