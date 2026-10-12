# P5-46 atomic glyph replacement

## Scope

Add `fc_doc::Command::ReplaceGlyph { glyph: Glyph }` for an existing named glyph.
Replace persistent glyph data while preserving the name, glyph order, session
handle, references from other font data, and the existing two kerning membership
fields. Callers change group membership separately with `AssignKerningGroup`.
Validate source outlines and insert through the model's effect/component checks.
Invalidate layers, metrics, and component dependents. Reuse document batch,
history, Unicode indexing, and persistence infrastructure.

Owned files: `crates/fc-doc/src/command.rs` (one variant),
`crates/fc-doc/src/command/apply.rs` (impact and implementation),
`crates/fc-doc/tests/replace_glyph.rs`, and this plan/report. No model schema or
fc-api changes. Coordinate narrow registry edits with the parent.

## Acceptance

- Write focused tests first: replacement preserves references, order and handles,
  updates Unicode index, invalidates dependent outlines, and undoes/redoes.
- Reject missing targets, invalid outlines/effects and component cycles without
  document mutation; failed batches restore earlier replacements.
- Preserve kerning membership despite incoming fields and permit a subsequent
  explicit group assignment in the same batch.
- Round-trip replaced glyph data through plain and compressed `.fcf`.
- Run the focused fc-doc test under the shared Cargo flock and task TMPDIR.
  Parent owns final broad checks, per `p5-resume-coordination.md`.
- Check artifact budgets at start and completion; remove owned scratch files.

## Implementation and acceptance handoff — 2026-09-30

Implemented the command and six focused integration tests. Replacement retains
existing kerning memberships, references, glyph position and stable handles;
changes layer data and Unicode indexing; invalidates layers, metrics, guides and
component dependents. Validation rejects malformed/nonfinite geometry,
nonfinite metrics/transforms, invalid effects, component cycles and malformed
glyph/layer metrics formulas. Valid unresolved metrics references remain editable.
Insertion failure restores the exact original glyph without revalidating imported
source data. History, explicit kerning reassignment and plain/gzip FCF round trips
are covered by tests.

Verification performed:

```text
rustfmt --edition 2021 crates/fc-doc/tests/replace_glyph.rs
exit 0
git diff --check -- crates/fc-doc/src/command.rs crates/fc-doc/src/command/apply.rs crates/fc-doc/tests/replace_glyph.rs
exit 0
```

The first `cargo test -p fc-doc --test replace_glyph` attempt was blocked in
concurrent dependencies (missing TTX module and the VariableStroke effect arm).
A later run, serialized through the shared Cargo flock with task TMPDIR, reached:

```text
error[E0063]: missing field `entity_resolver` in initializer of `ParsingOptions<'_>`
  --> crates/fc-sfnt/src/ttx.rs:61:9
error: could not compile `fc-sfnt` (lib) due to 1 previous error
exit 101
```

The no-default-features retry was canceled while queued, before spawning Cargo,
at the root coordinator's request to centralize integration validation. Its exit
143 is **interrupted**, not passed. No test pass is claimed here. Root must run
`cargo test -p fc-doc --test replace_glyph`; it owns broad formatting, Clippy and
regressions. Independent read-only review found and prompted the nonfinite
metrics fix, and found no other identity/rollback/cache/membership defects.

No dependencies or model schema changes; no FontForge code was consulted.

Owned queued processes were stopped and `.scratch/p5-replace-glyph` removed.
Final artifact checkpoint passed: build 6.09 GiB / 13,176 files; temporary
0.22 GiB / 8,051 files across the shared workspace; free 668.67 GiB. Owned paths
were checked with `git status`; the command registry also contains other agents'
independent changes, which were preserved.
