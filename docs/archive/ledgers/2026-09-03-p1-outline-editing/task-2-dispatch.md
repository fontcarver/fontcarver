You are implementing **P1 Task 2: `fw-doc` editing commands and edit sessions** for FontWorkshop (Tauri 2 font editor: Rust kernel + React/TS UI). This directory is the git worktree for branch `p1-t2`. `crates/fw-geom` (Task 1) is merged and is your geometry dependency; read its public API in `crates/fw-geom/src/lib.rs` before coding and use it for every path operation (do not re-implement geometry here). Do not modify `fw-geom` or `fw-model` (report NEEDS_CONTEXT with the precise gap if something essential is missing). Another agent is concurrently editing `crates/fw-doc` outside `command*.rs`/`edit.rs` in a separate worktree (P0 backlog: `document.rs` undo budget/begin_edit docs, autosave) — keep your changes to `command.rs` (+ new `command/` submodules), `edit.rs` (new), `metrics.rs` (new), `lib.rs` exports, `tests/`, and `fwf/schema.md`; the controller merges.

## Read first
1. `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-2-brief.md` — requirements; the `Command` variants, `PointRef`, `DeleteMode`, `HandleMode`, `PathOp`, edit-session and metrics-key semantics are normative.
2. `CONVENTIONS.md` (binding); `crates/fw-doc/src/{document.rs, command.rs, undo.rs, cache.rs}` (existing command application, snapshots, `Change`, `EditToken` coalescing, dependents cache — extend, don't fork); `crates/fw-model/src/{contour.rs, layer.rs, glyph.rs, font.rs}` and `query/`.
3. Spec §4 (commands/undo) and §12.3 (what the editor needs): `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/superpowers/specs/2026-09-03-fontworkshop-design.md`.

## Decisions (do not revisit)
- `Command` stays one serde-tagged enum (`#[serde(tag = "type")]` as today; check the existing representation and keep it) so TS gets a discriminated union; new variants are additive. `PointRef { contour: usize, point: usize }` indexes the layer's `contours[c].points[p]` as stored (off-curves included). `TransformSelection` takes explicit index lists; an empty selection means "whole layer".
- `MovePoints { handles: HandleMode::WithHandles }` moves selected on-curves together with their adjacent off-curves that are NOT themselves selected (so a node drag carries its handles); `PointsOnly` moves exactly the listed points.
- `SetSmooth { level_handles: true }` on a node with two handles rotates both onto the average tangent keeping lengths; `false` only flips the flag.
- `DeletePoints` `KeepShape`: remove on-curve points and re-fit the joined segment with `fw_geom` (fit the two neighbouring segments' sampled polyline into one cubic; quadratic layers get a single quad); deleting an off-curve converts the segment to a line. `BreakContour`: removes the point and both adjacent segments, opening the contour.
- `InsertPointOnSegment` uses `fw_geom::split_segment`; `ConvertSegment` Line→Curve inserts handles at 1/3 and 2/3; Curve→Line drops handles.
- `JoinContours` joins two open contours at their ends (a = end of contour A, b = start of contour B; B is reversed if its end is given); `SplitContour` opens a closed contour at `point` or splits an open one.
- `SetMetrics` order: `width` sets the advance; `lsb` translates all contours/components/anchors by (lsb − currentLsb) and keeps RSB (so width changes); `rsb` sets width = bounds.max_x + rsb (bounds from the flattened layer via `fw_model::query::flatten_layer`; empty glyph → LSB/RSB operate on width only). Metrics keys: parse `=name`, `=|name` (mirrored side), `=name+N`/`=name-N`, `=N` literal; `fw_doc::metrics::resolve` walks keys with a visited set → `Error::MetricsKeyCycle`.
- `PathOp` maps 1:1 onto `fw_geom` functions on the layer's contours; `RemoveOverlap`/`ExpandStroke` errors from the oracle surface as `Error::Geometry(String)` and leave the layer unchanged.
- `AddLayer`/`RemoveLayer` (non-master layers only; removing a master layer is an error), `CopyLayer { to: Background(master) }` replaces an existing background layer, `SwapWithBackground` swaps contours/components/anchors between master and background layers.
- `DuplicateGlyph` copies layers/unicodes cleared/tags/kern groups; `AddGlyphsFromRecipe` is NOT in this crate (fw-api expands recipes into `AddGlyph`s) — remove it from the enum if the brief lists it, and say so.
- `Change` gains `metrics: bool`, `layers: bool`, `guides: bool`; every command sets the right flags (tested).
- `Document::preview_edit(token, cmd)` = `apply_in_edit` (alias) — implement as the same call; document that the coalescing already avoids snapshot churn.
- Update `fwf/schema.md` for any new persisted field (metrics keys already exist in the model — verify).
- Commit on branch p1-t2 with Conventional Commits; do not push.

## Report contract
Full report to `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-2-report.md`. Final message under 15 lines: Status, commits, test summary, concerns, report path.

## Verification you must run before reporting
```
cargo fmt --all --check
cargo clippy -p fw-doc --all-targets -- -D warnings
cargo test -p fw-doc
cargo test -p fw-doc --release
```
Do not spawn other agents; do exactly this task's scope.
