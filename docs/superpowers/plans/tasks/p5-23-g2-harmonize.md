# P5-23 — G2 harmonization geometry

## Scope and ownership

Implement an atomic geometry projection for selected cubic-to-cubic joins.
The helper aligns adjacent handles and adjusts their lengths so the incoming
and outgoing signed curvatures agree, while fixing on-curve positions and each
selected node's combined handle length. Preserve unrelated metadata.

Owned files: `crates/fc-geom/src/harmonize.rs` and its test module,
`crates/fc-doc/src/command/outline/smooth.rs` and `tests/super_smooth.rs`,
narrow command/export/dispatch/outline-validation additions, and this plan.
`Point.super_smooth` is owned by the linked-nodes model agent. UI integration
and frontend preview parity are owned by node-semantics. Parent extended this
task to backend commands and persistent reapplication on 2026-09-30.

Public interface:
`harmonize_nodes(&mut Contour, &[usize]) -> Result<(), HarmonizeError>`.
Duplicate nodes are harmless. Any invalid, non-cubic, degenerate, non-finite,
incompatible-curvature, or non-convergent request leaves the contour unchanged.
Selected joins become smooth; the persistent super-smooth flag is not changed.

## Acceptance

Write regression tests first for unequal cubic curvature, tangent alignment,
closed-contour seam handling, simultaneous neighboring joins, metadata and
on-curve preservation, duplicates, unsupported/degenerate/opposite-sign joins,
and atomic failure. Run the focused test target under the shared Cargo lock;
parent performs final workspace Clippy and regression checks. Audit row 22
against existing backend keep-outline behavior and report any missing coverage.
No FontForge implementation is read or adapted; curvature is derived from the
cubic Bézier endpoint derivatives.

## Implementation and acceptance handoff — 2026-09-30

Implemented `SetSuperSmooth` and one-shot `HarmonizeNodes` through the normal
atomic outline command path. Every touched contour and replacement outline
reapplies its retained G2 constraints. Ordinary `SetSmooth` clears the G2 marker;
making a corner no longer rotates handles even if `level_handles` is true.
Already accepted G2 geometry remains bit-for-bit stable on reapplication.

Five geometry cases cover curvature, tangent alignment, closed seams,
neighboring selected nodes, exact idempotency, metadata, unsupported inputs,
atomic failure, and straight joins. Four document cases cover one-shot
undo/redo, retained constraints after drag/absolute/replacement edits, FCF
round-trip, invalid-command atomicity, and ordinary outline-preserving toggles.

Individually formatted owned Rust modules/tests with `rustfmt --edition 2021`.
The first `cargo test -p fc-geom --test harmonize` exited 101 because concurrent
sfnt work temporarily lacked `cff_interchange`, `read/cff/private`, and
`write/subroutinize`, and a roxmltree initializer lacked `entity_resolver`.
The G2 helper itself compiled before those unrelated errors. No test pass is
claimed. The queued `cargo test -p fc-doc --test super_smooth` was interrupted
(exit 143) before execution at the root's instruction to consolidate all Cargo
work under `/root/rust_integration`. That owner received both test targets and
must record the final results. No Cargo process or task scratch data is retained.

Acceptance boundary: only ordinary cubic-to-cubic nodes with nonzero adjacent
handles can retain G2. Opposite signed curvature, open endpoints, quadratic
joins, cusps, and non-convergent coupled selections return an atomic error.
No external-format retention guarantee is added; native FCF retention is tested.
