# P1 Task 2 — minor round (fw-doc outline command hardening)

Worktree: /home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t2m (branch p1-t2m from p1-outline-editing fe095c8). Scope: `crates/fw-doc` (and one additive helper in `crates/fw-model` if needed). Do not touch fw-api, fw-layout, fw-compile, fw-cli or the app — other agents own them right now.

Background: Task 2 (fw-doc editing commands) is merged; its re-review (`task-2-rereview1.md`, sections "New Breakage in the Fix Diff" and "Out-of-Scope Observations") left these Minor findings, which the controller ruled to fix in one batched round. Read that file and `task-2-fix1-dispatch.md` (rulings 2–4 describe the intended semantics) first.

## Changes (all binding)
1. **Touched-contour validation.** `command::outline::apply` currently validates every contour of the layer after a command. Change it to validate only the contours the command touched (the dispatch's ruling 4 said "each touched contour"): each outline operation reports the set of touched contour indices (or the whole layer for operations that rebuild it, e.g. PathOp/TransformSelection over everything), and only those are checked with `try_to_bezpath`. A pre-existing malformed contour elsewhere in the layer must not block edits on other contours. Test: a layer with one malformed contour (built via a raw `Contour` literal) still accepts `MovePoints` on another contour, and still rejects a command that malforms the touched contour.
2. **Whole-layer restore on failure.** On validation failure restore the entire `Layer` (contours, components, anchors, guides — take a clone of the layer before dispatch, or restore from the undo snapshot), so `TransformSelection` cannot leave component transforms or anchors moved when it reports `InvalidCommand`. Test: TransformSelection on a layer whose selected contour is malformed → `InvalidCommand` and components/anchors unchanged.
3. **`inferred_kind` from the font, not the layer.** A layer with only straight contours in a TrueType (quadratic) font must not receive cubic handles from `DeletePoints`/`ConvertSegment`/`InsertPointOnSegment`. Derive the curve kind from the master's declared outline kind if the model has one (check `fw_model` — `Master`/`Font` may carry a TrueType vs PostScript flag or `TtMasterData`), falling back to the existing contour-based inference only when the font does not say. If fw-model lacks such a field, add an additive `Font`/`Master` query (`preferred_curve_kind()` or similar) that derives it from existing data (e.g. presence of TrueType master data / imported `glyf` provenance) — document the rule. Test: a quadratic font's all-line glyph gets a `QCurve` midpoint control from `ConvertSegment { to: Curve }`, not cubic handles.
4. **Cheaper preview path.** `outline::apply` clones all contours on every command (each `preview_edit` frame). Clone only the touched contours (consistent with item 1/2: keep a full-layer clone only when the operation touches the whole layer). Keep semantics identical; add a note in the doc comment. No benchmark required, but the existing 1 ms/step budget test in fw-doc (if present) must still pass.
5. **Corpus sweep two-way deviation.** In `crates/fw-doc/tests/delete_points.rs` measure deviation both ways (original samples → new contour AND new-segment samples → original contour) and gate on the max of the two; keep the current thresholds (median ≤ 2 %, fitted p90 ≤ 15 %, worst ≤ 50 %). Report the new numbers.
6. Leave `AddContour`'s error class as `InvalidCommand` (consistent with the validator); add a doc-comment line on `Error::InvalidCommand` saying malformed contours surface here.

## Constraints
Rust 1.94; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; no unwrap/expect/panic/indexing-that-can-panic in library code; docs on pub items; public API additive only (no renamed/removed variants or fields); every command apply/undo exact. Commits on p1-t2m with Conventional Commits; do not push; no subagents.

## Verification
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p fw-doc -p fw-model -p fw-geom
cargo test --workspace
```

## Report
Write `task-2-minor-report.md` in this directory (per item: what changed, file:line, covering tests, command + output summary, corpus numbers). Final message < 15 lines: Status, commits, test summary, concerns.
