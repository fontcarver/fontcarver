# P5 kerning table maintenance

Owner: delegated backend kerning maintenance task, 2026-09-30.

## Scope and files

Implement undoable horizontal/vertical table cleanup, compression, and clipboard
paste into one chosen master. Changes live in `fc-doc/src/kerning/maintenance.rs`
and focused `fc-doc/tests/kerning_maintenance.rs`, with narrow registry/dispatch
additions in `kerning.rs`, `command.rs`, `command/kerning.rs`,
`command/apply.rs`, and a document generation validation hook. Existing pair
queries provide clipboard records. No model/schema change, metrics edits,
translations, status edits, commits, or broad builds.

## Public contract

`Command::KerningMaintenance { master, vertical, expected_generation, operation }`
uses exported `fc_doc::kerning::KerningMaintenanceOperation`:
`CleanupInvalid`, `Compress`, or `Paste { records: Vec<KernPair>, replace: bool }`.
Paste replaces the target table when `replace` is true, otherwise incoming keys
overwrite matching keys and retain other existing records. Incoming records must
be valid in the destination font and contain no duplicate keys. Stored invalid
records are only removed by explicit cleanup or replacement.

Bulk operations reject a stale document generation, including nested batches,
before any mutation. Source ownership checks apply to the chosen orientation.
Compression removes exceptions only where fallback supplies the same resolved
value for every affected concrete glyph pair; group/group roots and zero
exceptions that override nonzero class kerning remain. Cleanup removes records
whose values, glyph references, groups, or group membership are invalid.

## Acceptance

Write tests first covering class precedence, zero exceptions, both orientations,
master isolation, invalid/dangling records, destination paste validation,
merge/replace, stale generations including nested batches, no-ops, undo/redo,
native persistence, and source ownership. Run focused tests with the shared
Cargo flock and task-owned absolute TMPDIR. Parent performs final broad checks.
Run artifact checks at start and completion; retain only the acceptance report.
