# CFF1 authored flex export guard

## Scope

Reject authored `PsHints.flex = true` during CFF1 export because the model only
stores a layer-wide presence flag, without curve bindings or flex depth. Keep
ordinary cubic export working. Inspect and report the existing imported-flex
limitation without adding an interpreter or changing import/model interfaces.

## Owned files and interfaces

- `crates/fc-sfnt/src/write/hints.rs`: private writer guard only.
- `crates/fc-sfnt/tests/cff_flex.rs`: public `write_cff` regression coverage.
- This plan and `docs/acceptance/p5-cff-flex-guard.md`.

No public API changes, model changes, dependencies, or FontForge reference code.

## Acceptance

1. Demonstrate a failing test for authored flex before adding the guard.
2. A CFF1 export of a flex-marked layer fails with a glyph-specific error that
   identifies unsupported flex metadata; ordinary cubic curves still evaluate.
   Include a component-base flex regression when the base is omitted from the
   output order; the parent owns its `write/cff.rs` implementation.
3. Run the focused integration test under the shared Cargo lock and task TMPDIR,
   format only owned Rust files, and check artifact budgets at task boundaries.
4. Parent owns final workspace formatting, Clippy, and broad regression checks,
   as required by `p5-resume-coordination.md`.
