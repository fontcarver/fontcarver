# P5-12 cross-document editing and comparison

Owner: cross_document, with paste_special and compare_merge delegated backends.

## Scope and interfaces

Deliver audit rows 77, 78 and 96 as accessible dialog workflows: source-document
and master selection, same-name/selected/as-components Paste Special with
all-data/layer/kerning-group/metric-key payloads, missing-base policies, explicit
component relinking, paginated font differences and outline overlays, pairwise
family compatibility, reference binaries opened through existing session import,
and reviewed selected-glyph merge using the transfer transaction.

New backend modules are `handlers/{cross_paste,font_compare}.rs` and matching
API types. Paste uses a validated `ReplaceGlyph` command for atomic undo.
Persistent model schema is unchanged. Registries receive narrow additions only.
New frontend files live under `app/src/views/crossdocument/`; shell integration
adds dialog states, host cases and command registrations. English strings use
`crossDocument`; independent localization owns Chinese.

## Acceptance

Write backend behavior tests before implementations; cover malformed references,
cycles, existing-target relinking, missing-base policies, data scope, atomic
failure, stable glyph handles, undo/redo and native save/load. Font comparisons
cover differences, family compatibility, evaluated outline paths and pagination.
Frontend tests exercise preview invalidation, stale-generation guarded apply,
selection mappings, accessible diff/overlay and error recovery. Run focused
Rust/frontend tests, then headless browser verification. Parent owns final broad
workspace gates and shared status documents. All Cargo commands use shared
flock and scratch TMPDIR; artifact checker at start and completion.
