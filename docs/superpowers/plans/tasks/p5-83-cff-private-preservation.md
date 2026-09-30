# P5-83 CFF private dictionary preservation

## Scope

Preserve representable static CFF1 CID private dictionaries and each layer's
Font DICT selection through model serialization and static CFF output. Keep the
existing master private dictionary as FD 0; additional dictionaries are retained
without claiming a new per-FD editor. Normalize supported private values into
model units when matrices permit it, and disclose unsupported transforms.

The separate flex guard subtask rejects an authored flex preference that the
current CFF writer cannot represent; imported flex operator binding remains open.

## Files and interfaces

- `fc-model` master/layer additive serde-default fields, after owner coordination.
- `fc-doc/src/fcf/schema.md`: field persistence documentation.
- `fc-sfnt/src/read/cff.rs`, a private-reading helper module, and read integration.
- `fc-sfnt/src/write/cff.rs` and focused CFF tests.

Do not edit compiler export/preprocessing, API/editor code, audit/status files,
or unrelated formats. Cross-document and generated-instance handling must either
retain assignments coherently or reject unresolved selections at export.

## Acceptance

Write regressions before behavior: distinct CID private dictionaries survive
read/write and JSON serialization with glyph assignments, invalid dictionary
indices fail, legacy default data remain supported, and supported transformed
private values are normalized. Validate static CFF parsing using fontations.
Run focused `fc-model` and `fc-sfnt` tests and crate Clippy under the shared Cargo
lock; parent owns final workspace checks. Record exact commands and actual
boundaries. Check artifact budget at task boundaries and clean owned scratch.

No FontForge source or executable is used. No new dependency is planned.
