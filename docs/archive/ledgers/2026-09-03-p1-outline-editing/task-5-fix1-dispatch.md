# P1 Task 5 — fix round 1 (fw-layout shaper, fw-api surface, CLI)

Worktree: /home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t5 (branch p1-t5). HEAD = your Task 5 commits (935af68) + a merge of the P1 branch that brings in Task 2's final fw-doc (new `DeleteMode::Simple` variant, fw-api error mapping reconciled by the controller in `crates/fw-api/src/error.rs`), the relicensed fixtures (DejaVuSansMin.woff2 / SourceSans3Min.otf / SourceSerif4Min.otf replace CantarellMin.woff2 / test1012.otf / NimbusRoman-Regular-o.otf) and the MIT OR Apache-2.0 license change. Requirements: task-5-brief.md and task-5-dispatch.md in this directory (binding). The review that found the defects: task-5-review.md (read it in full; every numbered issue below refers to its numbering).

## Rulings (binding; do not re-litigate)

1. **Important 1 — `text.shape` must never compile or deep-clone the `Font` while holding the session lock.** Restructure `handlers/shaping.rs` + `fw_layout::Shaper` so that under `session.with_doc` you only (a) read the document generation and the compile key, (b) collect the model widths of the glyphs in the requested text (small map, for the width-correction path), and (c) clone the `Font` ONLY when a rebuild is actually going to be scheduled for a generation that has no pending/running build yet. The harfrust shaping and any `compile_static` run outside the session lock. First-ever compile for a (doc, master): still synchronous for the caller (the UI needs a result), but outside the session lock. Never clone on a stale request that already has a pending build for the same or newer generation. Add a test that proves `doc.edit_apply` (or any session write) is not blocked while a shape compile is running (e.g. start a shape on a large fixture on one thread, apply an edit on another, assert the edit completes within a bound well below the compile time).
2. **Important 2 — a panicking rebuild worker must not wedge the entry.** Clear `rebuilding` from a drop guard; run the compile under `std::panic::catch_unwind` and record the failure (message) on the entry so the next `text.shape` surfaces it in `warnings` (code `compile-failed`) and a later edit retries. Test: inject a failure (a compile hook or a deliberately invalid feature source that makes `compile_static` error, plus a unit test of the guard with a panicking closure) and assert the entry recovers on the next generation.
3. **Minor 3 — shutdown.** `forget(doc)` and `Shaper`/`Api` drop set a cancellation flag the worker checks before starting the compile and before publishing; a cancelled worker exits without compiling. No thread join required.
4. **Minor 4 — event bridge lifetime.** Tie the forwarder thread to the `Api` (a shutdown flag checked per message, or a `Weak` upgrade); do not set the `OnceLock` when the spawn fails (retry on the next `subscribe`). Progress/document event ordering (Minor 5) is accepted as-is; document it in a doc comment.
5. **Minor 6 + 13 — real, terminal progress.** Add an additive progress callback to `fw-compile` (`export_with_progress(font, options, &mut dyn FnMut(ExportStage, u8))` or equivalent; `export()` keeps its signature and calls it with a no-op) so `compile`/`preserve`/`write` percents are emitted when the stages actually run. On any failure emit a final `ProgressEvent { stage: "failed", percent: 100, message: Some(err) }` (additive optional `message` field, `#[serde(default, skip_serializing_if = "Option::is_none")]`), so a UI progress bar always terminates. Test the event sequence for both success and a failing export (unknown master or unwritable path).
6. **Minor 7 + 8 — recipe safety.** Cap a single recipe expansion at 65,536 glyphs (a range beyond the cap → `Unknown` with a warning entry `too-large`), and let a bare non-ASCII scalar (`中`) resolve like a one-element range. Unit tests for both.
7. **Minor 10 — recovered marker.** `DocSummary` gains additive `recovered_from: Option<String>` (the sidecar's original path); the recovered document keeps `path` = the original path and stays dirty (overwriting the original on Save is the purpose of recovery; the UI shows the marker). Regenerate TS.
8. **Minor 11 — deterministic English synthesis.** Choose the source record by a total order `(lang_id, platform, encoding, name_id)` — never by `HashMap` iteration. Test with two candidate records.
9. **Minor 12 — coverage.** Add an API-level `session.export` test on an imported binary (DejaVuSansMin.woff2 or SourceSans3Min.otf via `fw_fixtures`) asserting the preservation contract at the API boundary: untouched raw tables byte-identical and no `hinting-dropped` warning when nothing was edited; the `compile-failed` warning test from ruling 2 covers the other gap.
10. Minor 9 (three lock acquisitions in `glyphs.add`) is accepted for P1 — leave it.
11. **Merge integration.** After the merge, `DeleteMode::Simple` exists in fw-doc: regenerate TS (`cargo run -p fw-api --bin gen_ts`), make sure the JSON dispatch test covers `DeletePoints { mode: "Simple" }`, and that `app` still type-checks.

## Constraints
Rust 1.94, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, no unwrap/expect/panic/indexing-that-can-panic in library code, docs on pub items, all API/model changes additive (do not rename or remove existing fields, variants or methods; the app must keep compiling with at most mechanical fixes). Do not modify `fw-doc`, `fw-model`, `fw-geom`, `fw-sfnt`, `fw-formats` (fw-compile only for the additive progress callback and the name-synthesis order). Commits on p1-t5 with Conventional Commits; do not push; no subagents.

## Verification you must run before reporting
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p fw-api --bin gen_ts && (cd app && pnpm check && pnpm test && pnpm lint)
```

## Report
Append a "Fix round 1" section to task-5-report.md (per issue: what changed, file:line, covering test names, the command run and its output summary). Final message < 15 lines: Status, commits, test summary, concerns.
