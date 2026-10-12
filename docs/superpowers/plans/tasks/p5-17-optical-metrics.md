# P5-17 — Optical spacing and kerning

Scope: audit row 94, with rows 37–43 delegated to independent metrics-key and
kerning workflow agents. Implement original outline-profile sampling, density
control, captured-selection preview/apply, one/all masters, atomic undo and FCF
persistence. Reuse P4 metrics commands and P2a kerning eligibility/precedence.
No FontForge implementation is read or adapted. No new dependencies.

Owned implementation: `fc-doc/src/optical/`, `fc-api/src/{types,handlers}/optical.rs`,
`fc-api/tests/optical.rs`, isolated OpticalSpacingDialog and tests. Only additive
registry/export patches; coordinate GridToolbar integration with inventory owner.

Algorithm: sample resolved curve silhouettes at bounded horizontal scanlines;
set sidebearings from mean edge recession and a global UPM-scaled density target.
Auto-kerning measures the mean inter-outline gap with a sampled clearance floor.
Concrete pair exceptions preserve class membership; existing rules are protected
unless explicitly enabled. Empty/zero-width, locked and metrics-linked spacing
sources are reported as skipped. Work caps bound glyphs, layers, segments and
pairs; imported/source-owned kerning remains read-only.

Preparation runs against a private snapshot. Spacing processes dependencies
first and applies each proposed metrics edit to the snapshot, so selected base
components and composites yield the same preview and final result. Apply checks
the captured document generation and submits one Command::Batch.

Acceptance: rectangle/curved/slanted outline comparisons and density monotonicity;
preview purity, stale/rejected atomicity, components in both selection orders,
manual/group rules protection, one/all-master isolation, undo/redo and FCF;
bounded Noto Sans/Source Sans fixture runs; UI preview invalidation and accessible
controls; isolated headless browser inspection when integrated builds permit.
Focused Cargo commands use shared cargo.lock, normal profiles and task TMPDIR.
Final workspace checks are owned by the root coordinator.
