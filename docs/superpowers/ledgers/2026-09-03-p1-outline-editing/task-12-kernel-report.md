# T12 kernel acceptance report — 2026-09-07

Implemented the bounded kernel and CLI acceptance work on `p1-acceptance-kernel`, based on reviewed P1 commit `3df91c9`.

## Added gates and workflows

- `fc-api/tests/edit_performance.rs` adds an ignored, release-only `doc.edit_apply` gate. Its corpus has exactly 20,000 populated glyphs: one 300-point target outline and 19,999 master layers with a width and four-point closed outline. It warms 20 calls and records 200 calls.
- The timed boundary starts immediately before `Api::call("doc.edit_apply", params)` and ends after the response returns. It includes API dispatch, request JSON decoding, session/document locking, the real 300-point `MovePoints` mutation, coalescing, and response construction. Request JSON construction happens before the timer; frontend/Tauri/network transport and rendering are outside this kernel gate.
- The conservative assertion is p95 <= 4 ms. The semantic tail checks the cumulative delta after all 220 real calls (`+27.5, +55.0`), exactly one retained/coalesced history entry, one API undo, exact coordinate restoration, no remaining undo label, and the expected redo label. `undo_len()` intentionally remains one after undo because it counts the retained redo entry.
- `fc-api/tests/acceptance_workflow.rs` opens the licensed DejaVu Sans fixture through the API, moves points in A, B, and C, exports TTF, reparses it, and checks the edited coordinates. Component dependents expanded those three direct edits to 79 invalidated glyphs in this fixture; the test verifies each direct glyph generation and independently matches the export report to the document's full invalidation count.
- The DejaVu workflow checks byte-identical GSUB/GPOS/GDEF tables where present and an identical shaping fingerprint for `Hello fi AVWA`. It enforces the ruled edited-font behavior: a `hinting-dropped` warning names A/B/C and global TrueType hint tables are absent.
- `fc-cli/tests/acceptance_workflow.rs` creates an empty `Scratch Latin` font through the API, adds and draws A-C with widths 600/620/640, exports TTF, then uses the current worktree's Cargo-built `fontcarver` binary to inspect, list, and shape it. A direct SFNT reopen verifies A's 600-unit width and three-point outline; CLI shaping returns A/B/C.

Commits:

- `d0c2df1 test(api): add edit apply performance gate`
- `c2fe856 test(acceptance): cover kernel and CLI workflows`

## Release measurements

The final run occurred in the shared development environment while other P1 worktrees were active, so it was not an isolated machine benchmark. Both enforced budgets still had substantial headroom.

`cargo test --release -p fc-api --test edit_performance --test acceptance_workflow -- --ignored --nocapture`

- DejaVu API edit/export/reopen workflow: passed (test process reported 1.82 s).
- 20,000-glyph, 300-point `edit_apply`, 200 measured samples after 20 warm calls: median 556.456 us, p95 602.373 us, max 1.410845 ms. The enforced p95 <= 4 ms gate passed.

`cargo test --release -p fc-compile --test export_static exports_dejavu_within_release_budget -- --nocapture`

- DejaVu Sans export: 581.987386 ms; existing release budget <= 5 s passed.

## Verification

- `cargo fmt --all -- --check`: passed.
- `cargo clippy -p fc-api -p fc-cli --all-targets -- -D warnings`: passed.
- `cargo test -p fc-api -p fc-cli`: 118 passed, 2 ignored release-only tests, 0 failed. This includes the new scratch-font/current-CLI workflow.
- `cargo test -p fc-compile --test preserve -- --nocapture`: 8 passed. In particular, untouched DejaVu preserves `glyf`, `loca`, and `maxp` verbatim, keeps global and glyph TrueType instructions, and shapes identically; any outline edit drops all global and per-glyph TrueType instructions and warns.
- `cargo test -p fc-compile --test roundtrip_shaping noto_sans_shapes_identically_after_export -- --nocapture`: 1 passed.
- `cargo test -p fc-formats --test ufo_write -- --nocapture`: 8 passed, including semantic, incremental, and SFNT/UFO round trips.

## Remaining P1 acceptance work

- The external Source Han release/export and compiled identity evidence is owned by the separate repair/integration task and was not run or changed here.
- Browser pointer-to-paint coverage and the final desktop new/open/edit/export/reopen workflows wait for the reviewed T8/T9 UI integration.
- The timing gate measures the Rust API boundary described above; it does not establish frontend paint latency or desktop transport latency.
- This report is kernel/CLI evidence only and does not claim final whole-P1 acceptance.
