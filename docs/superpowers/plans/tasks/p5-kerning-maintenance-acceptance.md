# Kerning maintenance acceptance — 2026-09-30

Implemented the bounded backend plan in `p5-kerning-maintenance.md`.

- `Command::KerningMaintenance` supports cleanup, compression, and validated
  replace/merge paste for one master and orientation.
- Recursive generation preflight rejects stale bulk edits before any batch
  mutation. Existing source ownership, undo, no-op detection, and native model
  persistence paths are reused.
- Compression keeps class roots, empty classes, meaningful explicit zero,
  invalid imports and invalid fallbacks. Lower-priority exceptions are processed
  first so masked exceptions settle in one pass.
- Twelve focused tests cover values/class precedence, invalid cleanup, both
  orientations, master isolation, no-ops, paste validation, stale/nested batches,
  undo/redo and native persistence. Independent read-only review identified the
  compression ordering issue; the fix and regression are included.

Verification completed:

```text
rustfmt --edition 2021 --config skip_children=true --check
  crates/fc-doc/src/kerning/maintenance.rs
  crates/fc-doc/tests/kerning_maintenance.rs
  crates/fc-doc/src/command/kerning.rs
exit 0
git diff --check -- <owned/shared touched files>
exit 0
```

The serialized focused command
`cargo test -p fc-doc --test kerning_maintenance --no-default-features`
acquired the shared lock and exited 101 before tests ran:

```text
fcf/project.rs:87: unstable str_as_str
optical/prepare.rs:228,326: unknown field Font.units_per_em
optical/profile.rs:35: ambiguous bounding_box; :51: type inference
command/apply.rs:58: non-exhaustive NodeLinks pattern
error: could not compile `fc-doc` (lib) due to 6 previous errors
```

These concurrent-integration errors were reported to owners. No test pass is
claimed. On the user's stop-after-current-work steering, the parent owns the
consolidated rerun, generated `KerningMaintenanceOperation.ts` import repair,
frontend integration, broad checks and feature-audit acceptance. No new
dependencies, model/schema changes, FontForge adaptations or commits.

Task-owned scratch was empty and removed; the Cargo process had exited. Artifact
checks passed throughout (start: 5.28 GiB build, 0.01 GiB temporary); final
resource checkpoint is recorded in the delegated completion report.
