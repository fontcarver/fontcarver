# P5 font comparison backend

Owner: delegated compare backend agent, 2026-09-30. Scope is audit row 96.

Implement read-only `font.compare` and `font.compare_glyph` in new fc-api handler
and type files. Compare open documents (including imported reference binaries)
by persistent glyph names and explicit/default masters. Report family, UPEM,
axis/master structure, glyph set, selected-master metrics, glyph outline,
Unicode, metrics and interpolation topology differences. Evaluate components
and live effects for drawing comparisons. Return bounded pages and full-font
counts, with optional differences-only filtering; reject explicit work-budget
overflow. Return SVG paths and approximation indicators for visual overlays.

Parent owns shared module registries, API dispatch/type generation and frontend.
The sibling paste agent owns undoable selected-glyph merging; comparison returns
source glyph handles suitable for its `doc.paste_special` same-name operation.
No duplicate mutation API, model changes, dependencies or translations here.

Acceptance: write focused tests first for independent glyph differences,
component propagation, stable-ID-insensitive comparisons, Unicode set semantics,
pagination/filter/full counts, alternate masters, invalid IDs/bounds, read-only
behavior and overlays. Verify a compiled binary can be opened and compared.
Run focused fc-api tests under shared Cargo flock with task-specific TMPDIR.
Parent owns broad final checks. Run artifact checker at start and completion;
remove task-owned scratch outputs. Report actual results and boundaries.
