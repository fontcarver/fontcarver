# Portable fixture lookup report — 2026-09-07

Implemented in `p1-text-kernel` commit `cc17701 test(fixtures): make Source Han lookup portable`.

Source Han Sans lookup now honors `FC_SOURCE_HAN_SANS`, then defaults to the sibling `SourceHanSanOTF` checkout beside the primary FontCarver repository. Rust resolves linked worktrees from their `.git` pointer without invoking Git, canonicalizing relative gitdir paths before locating the primary repository. The Rust benchmark reuses `fc_fixtures::external`; Playwright and `scripts/perf-open.sh` use the equivalent override/default policy. Missing optional fixtures still skip, while the performance script retains its explicit missing-file error.

Validation:

- `cargo test -p fc-fixtures`: 7 passed, including synthetic linked-worktree, override, sibling-default, unknown-key, and committed-fixture checks.
- `cargo test -p fc-sfnt --test read_cjk -- --nocapture`: executed the real 16.5 MB Source Han fixture, passed, open time 6.315 s.
- `cargo test -p fc-compile source_han_export_meets_release_budget_when_available -- --nocapture`: found the fixture and reached its debug-mode release guard.
- `cargo clippy -p fc-fixtures -p fc-sfnt --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm --dir app check`, `pnpm --dir app lint`, Prettier for `cjk.spec.ts`, and `bash -n scripts/perf-open.sh`: passed.
- API bindings were generated with a freshly compiled binary from the current worktree; generated output remained untracked/ignored.

The full release Source Han export timing remains a T12 follow-up; its 60-second threshold was not changed.
