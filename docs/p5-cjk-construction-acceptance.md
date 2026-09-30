# CJK construction — resumed acceptance

2026-09-30. Scope: audit row 112; task plan
`superpowers/plans/tasks/p5-112-cjk-construction.md`.

The glyph inspector now contains a collapsed CJK construction section. Its
paged radical library displays outlines already in the current font, including
Kangxi radicals, their normalized ideographs and common variants. IDS input
supports the original twelve Unicode operators, nesting and explicit
`[glyph.name]` leaves. Preview resolves exact glyphs in the selected master,
rejects missing/ambiguous/self/empty/zero-width sources, normalizes source
advances, and appends editable components as one existing atomic Batch command.
Document generation changes invalidate pending previews. The per-component grid
edits all six affine coefficients while preserving reference metadata.

The grid exposed a NumberField rounding defect. Its new optional formatValue
prop allows exact coefficients while retaining default font-unit formatting.

Focused evidence:

```text
vitest run cjkConstruction.test.ts
Test Files 1 passed; Tests 39 passed

vitest run cjkResolve.test.ts CjkConstructionPanel.test.tsx
Test Files 2 passed; Tests 12 passed

vitest run CjkTransformGrid.test.tsx NumberField.test.tsx --maxWorkers=1
Test Files 2 passed; Tests 17 passed
```

Focused ESLint, formatting and diff whitespace checks passed. Five new fc-doc
integration tests cover batch atomicity, missing/cyclic dependencies, complete
undo/redo preservation, transform editing and native save/reopen. Their attempted
Cargo run exited 101 before running tests because concurrent fc-doc integration
did not yet compile (`fcf/project.rs`, `optical/*`, missing NodeLinks match).
They are **not reported as passed**; root owns final Rust/frontend checks.

Visual acceptance is **unverified**. An isolated agent-browser session loaded
the shared UI, but root halted further browser work during integration/resource
pressure. The session was closed and its task-owned temporary files removed.
No review screenshot was produced. Audit row 112 should remain awaiting final
integration and browser acceptance.

Layout templates provide manual starting positions, not optical decomposition.
No external radical outlines or IDS database are bundled. Newer IDS operators
are rejected explicitly. Existing outlines remain; the action appends references.

The first consolidated Rust run found that ordinary `AddComponent` accepted a
new missing base, so the missing-radical rollback regression failed. Authoring
now rejects an absent base before adding it. Existing unresolved imported
references remain untouched; a sixth integration test verifies that a valid
new radical can still be added beside such a reference and undone. The
coordinating agent owns the focused retry after this correction.
