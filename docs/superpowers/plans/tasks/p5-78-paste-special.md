# Paste Special and reference relinking

Scope: typed `doc.paste_special` and `doc.relink_components` requests, explicit
source/target handles, same-name/selected/component destinations and all glyph
data/active layer/kerning membership/metric-key content. All-data preserves target
names and transfers compatible master layers; incompatible master mappings are
rejected. Source snapshots and target mutations carry generation guards. Preview
validates the same batch without modifying either document.

Missing bases have explicit reject, recursively copy, visible nonexporting
placeholder, or decompose policies. Existing target bases relink by name; mapped
selected glyph names relink together. Relinking preserves component transforms
and rejects missing targets and cycles. Commands apply in one undo transaction.

Owned files: `fc-api/src/{types,handlers}/cross_paste.rs`, handler helper directory,
`fc-api/tests/cross_paste.rs`; delegated `ReplaceGlyph` command implementation.
Parent integrates API/export registries and frontend. No persistent model change
or new dependency. Font-wide layout, bitmap strikes, palettes and global hint
programs remain separate document resources; all-data means all glyph records.

Acceptance: tests first for preview purity, selected-name/reference preservation,
all four missing-base policies, partial-content preservation, atomic failure,
generation guards, undo/redo, relink cycle rejection and native persistence.
Focused Cargo tests use shared flock and task TMPDIR. Parent owns final checks.

## Implementation and acceptance handoff

Implemented both typed APIs and the 12 destination/content combinations. Requests
validate handles and target locks, cap paste mappings at 512 and dependency copies
at 4096, and bound component depth at 64. Relink accepts up to 100000 explicit
handles, ignores unaffected locked glyphs, and covers construction sources and
color component references as well as outline components. Paste preview executes
the actual command validators in a separate session outside session callbacks;
commit rechecks the captured target generation and creates one undo entry.

Cross-document metadata integration preserves layer-local node/guide links,
remaps construction sources and layer metric keys, maps equal CFF dictionaries,
and atomically appends missing CFF dictionaries for existing CID destinations.
Target-only master layers are preserved. Hinted glyphs with incompatible TT
globals/UPEM, color layers with different palettes, incompatible source master
mapping, and unmatched CFF dictionaries in non-CID destinations reject explicitly.
Decomposition of live constructions bakes geometry and clears obsolete links,
effects and hints. As-components/decompose is rejected because it cannot retain
component references. The placeholder policy creates empty nonexport glyphs
tagged `missing-component-base` and returns visible warning text to the UI.

Eight focused API tests were added, including a 12-case content/destination
matrix, preview purity, four missing-base policies, undo/redo/native persistence,
stale source and late-mapping failure, selected-name relinking, component name
collisions, relink cycle rejection and success, and CFF dictionary remapping.
The delegated ReplaceGlyph command has six separate tests.

Validation on 2026-09-30:

- Narrow `rustfmt --edition 2021 --check` on all four owned Rust files: PASS.
- `git diff --check`: PASS.
- First `cargo test -p fc-api --test cross_paste`: exit 101 before this crate,
  while the parallel Layer.metrics_keys model field insertion was in progress.
- Retry was cancelled while waiting for the shared lock (owned flock PID
  2336375, no Cargo child) at root's request to consolidate builds. This is
  interrupted validation, not a passing test run.
- Root must run `cargo test -p fc-api --test cross_paste` and
  `cargo test -p fc-doc --test replace_glyph` with final integration checks.

No new dependency, model schema addition, or FontForge implementation was used
by this task. Parent owns API registries, generated types, frontend, and final
audit status. Full-row acceptance is pending the consolidated tests and visual
review; font-wide layout/bitmaps/palettes/TT globals are outside glyph transfer.
