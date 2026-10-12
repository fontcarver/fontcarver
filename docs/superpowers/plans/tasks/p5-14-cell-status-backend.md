# P5-14 Glyph cell status backend

## Scope and files

Add defaulted, TypeScript-optional boolean status fields to `GlyphCell` and
populate both normal and missing-master cells. Own
`fc-api/src/handlers/glyph_status.rs`, the `GlyphCell` portion of
`types/glyphs.rs`, narrow `build_cell` integration, and module registration.
Update the dry-run `preview_cell` initializer in `handlers/editing.rs` with false
defaults to preserve its existing prospective-glyph behavior.
Frontend badges and smart-filter predicates remain with their delegated owners.

## Interface and semantics

- `metrics_out_of_sync`: selected-master horizontal formula resolution differs
  from stored outline bounds/width, or the formula cannot resolve.
- `has_annotations`: nonblank glyph note or nonempty canonical per-layer editor
  annotation array.
- `has_custom_glyph_info`: explicit production/classification overrides or
  meaningful glyph user data.
- `incompatible_masters`: missing required master layer or geometry topology,
  components, or anchor names incompatible under `fc_geom::compatible`.
- `locked`: persistent glyph lock.

## Acceptance

Add synthetic tests first for default/overridden metadata, canonical annotations,
notes, multiple master topology/anchors/missing masters, metrics equality,
mismatch and resolution failures, and backward-compatible deserialization.
Reuse already computed outline bounds, skip metrics resolution without formulas,
and add a timed 4,096-cell-equivalent status calculation budget test.
Run focused `fc-api` tests under the shared Cargo lock. Root owns final broad
format/lint/regression and API type generation. Check disk budgets at boundaries.

## Acceptance report — 2026-09-30

Implemented all five defaulted optional status flags, both normal and
missing-master cell paths, and prospective glyph preview defaults. Metrics use
the central resolver's bounded formula traversal and local-layer overrides,
skip formula-free resolution, reuse outline bounds before bitmap fallback, and
tolerate differences up to 0.000001 font units. Compatibility checks stop at the
first missing or incompatible master. Notes and canonical editor annotations
aggregate across all glyph layers. Custom information excludes tags and kerning
groups; explicit classification/production fields and nonempty glyph user data
qualify.

Nine synthetic unit tests cover metadata, annotations, formulas, layer overrides,
missing masters, anchors, topology, backward-compatible payloads, and a timed
4,096-iteration default-status path. Tests were written before implementation;
both queued Cargo attempts were cancelled before compilation because central
Rust integration owns verification after the user's stop-after-current-work
steering. Therefore no Cargo test pass is claimed here. Central command:
`cargo test -p fc-api --lib glyph_status` (also included by all fc-api lib tests).

`rustfmt --check --edition 2021 crates/fc-api/src/handlers/glyph_status.rs` and
`git diff --check` for touched feature files passed. Independent read-only review
caught the prospective glyph initializer omission, now fixed. No remaining
semantic issue was identified by that review. Shared broad Clippy/regression and
API generation remain with root; no source outside this feature was cleaned.

Final `python3 scripts/check_artifacts.py`: build 6.07 GiB / 13,157 files,
temporary 0.11 GiB / 6,582 files, 668.97 GiB free, PASS. The task's empty scratch
directory was removed; no owned process remains. Git status was inspected.
