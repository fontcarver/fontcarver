# P5 metrics keys and synchronization acceptance

Owner: `optical_metrics/metrics_keys`, 2026-09-30. Scope: audit rows 37–38.

Horizontal formulas now support references, signed offsets, mirrored references,
constants, layer-local `==` overrides, sidebearing measurements at a specified
height, and offsets from automatically aligned component metrics. Arithmetic
multiplication/division and same-glyph mirrored bearings are also supported.
Behavioral syntax reference: [Glyphs Handbook: Spacing](https://handbook.glyphsapp.com/spacing/).
Implementation is original; no FontForge code or dependency was used.

`Layer.metrics_keys` is optional persisted model data. Non-null fields override
the corresponding glyph formula; other fields inherit. Glyph-wide commands
reject local syntax. Local editing is undoable and resolves the selected layer,
including supplemental layers. Invalid or cyclic keys keep outlines accessible
so the inspector can display the error and accept a correction.

`UpdateMetrics` resolves current/all master layers before applying any result.
Unkeyed metrics are derived consistently from outline width and keyed metrics;
conflicting complete constraints, invalid advances and unrealizable empty-layer
bearings fail without mutation. Repeated updates do not accumulate offsets.
Current/all updates are single document commands and support undo.

Verification: focused frontend suites passed (4 files / 19 tests), followed by
the inspector's metrics cases (2 passed, 26 unrelated cases skipped). Scoped
ESLint, Rust formatting checks, and `git diff --check` passed. The Rust command
`cargo test -p fc-doc --test metrics` was queued behind the shared lock and then
interrupted with exit 143 at the integration owner's request; it was not run or
claimed passed. The central integration suite owns its 12 tests, generated API
regeneration, workspace Clippy/regressions and TypeScript checking.
Exact UI commands and launch diagnostics are recorded in
[the UI acceptance report](p5-metrics-ui-acceptance.md).

Isolated browser launch failed before DevTools became available while the shared
machine was heavily loaded; no screenshot or visual pass is claimed. The parent
owns full application browser verification after central integration. Temporary
metrics Rust work was empty and removed; the UI owner cleaned its harness and
browser attempts. Artifact checkpoint passed at 6.11 GiB build / 0.23 GiB shared
temporary / 668.76 GiB free; no new dependency or debug-profile cache was added.

Boundaries: automatic offsets require an upright, positive horizontal scale and
an auto-aligned component-only glyph. Update commands cover master layers;
supplemental-layer local keys are editable/resolvable but their current-master
update button is disabled. Source interchange formula preservation and native
OS testing are outside this slice. Audit rows should retain these boundaries and
require the parent integration evidence before claiming complete acceptance.
