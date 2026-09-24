# P1 Task 2 review (Opus) — base b6f8a96 → head ac465c0

## Spec Compliance — ❌ (with substantial ✅ areas)

✅ Command surface: every variant in the brief exists with the specified field names and types (`crates/fw-doc/src/command.rs:258-742`), including `PointRef`, `HandleMode`, `DeleteMode`, `SegmentKind`, `PathOp` (all 10 ops), `SimplifyOptions`, `StrokeOptions`/`Nib`/`Cap`/`Join`, `CurveKind`. `AddGlyphsFromRecipe` correctly absent. Serde representation matches the pre-existing enum (externally tagged).
✅ Apply/undo/redo exactness probed on DejaVu `B` for MovePoints, SetSmooth, ReverseContours, SetStartPoint, TransformSelection, PathOp::{RemoveOverlap, AddExtrema, RoundToGrid, ExpandStroke}, ConvertSegment, Join, Split, BreakContour, SetMetrics, layer commands, DecomposeComponents, guides (3 scopes), DuplicateGlyph, SetGlyphOrder, SetMaster, SetFontGuides — undo restores `Font` equality, redo re-applies.
✅ Coalescing: 50 apply_in_edit → 1 entry; preview_edit identical; begin_edit while live finalises; stale token → InvalidEditToken; apply finalises.
✅ Metrics: lsb preserves rsb; rsb from flattened bounds; width+lsb+rsb order; keys `=H`, `=|n`, `=n+10`, `=N`, `=42`; cycle → MetricsKeyCycle; visited set is a proper recursion stack.
✅ Change flags/events: metrics/layers/guides threaded through Impact, DocumentEvent, Change::merge, Snapshot::FontGuides; guide scope routing verified.
✅ Errors/atomicity: ExpandStroke(width 0) → Error::Geometry with layer unchanged; out-of-range → InvalidCommand; master-layer removal rejected; fwf/schema.md note accurate; fmt/clippy clean; 44 tests.
❌ Failing: DeletePoints KeepShape semantics, MovePoints handle semantics, panic-freedom, workspace compilation.
⚠️ Cannot verify: TS discriminated-union ergonomics (no bindings generated yet).

## Strengths
- Clean module split with a single impact/apply_inner dispatch spine; geometry delegated to fw-geom.
- PathOp write-back after the fallible call → structurally atomic.
- replace_layer_outline preserves glyph order and restores on failure.
- Metrics-key parser handles `=A-B` hyphen ambiguity.
- two_mut for the background swap avoids a clone without unsafe.

## Issues

### Critical
1. **MovePoints cannot move any off-curve point** — `crates/fw-doc/src/command/outline.rs:126-131` rejects the command if any PointRef is off-curve (message copy-pasted from SetSmooth). `MovePoints{points:[handle], handles:PointsOnly}` on DejaVu B → Err(InvalidCommand). The `continue` at outline.rs:132-135 is dead code. No test.
2. **DeletePoints KeepShape does not work on real outlines.** Sweeping every on-curve node of DejaVu B O S a e g: 64 of 85 deletions returned `Error::Geometry("fw-geom could not fit adjacent segments into one segment")`; of the 21 successes, deviation up to 417 units (2048 upm) (g[1] p23: 301.6; a[1] p6: 275.4). `fit_joined_span` (outline.rs:331-385) doubles `fw_geom::simplify` error from 1 to 4096 until the span collapses to one segment — unbounded quality — and gives up otherwise. Deleting a node between two collinear lines adds two off-curve points (6-point square → 7 points). Shipped test covers only insert-then-delete.
3. **Multi-point KeepShape deletion deletes the wrong points and can panic.** `delete_points` (outline.rs:280-296) loops indices in reverse calling `delete_keep_shape`, but each call rewrites the contour (`rotate_left` at outline.rs:322 plus a splice) so later PointRefs address different points. On a 6-point closed contour deleting points 1 and 2 removes only point 2. Deleting all points of a closed quadratic contour panics at outline.rs:306 (`contour.points[index]`, index out of bounds).
4. **ConvertSegment panics on a degenerate closed contour** — outline.rs:601 `contour.points.drain(start + 1..end)` with end == 0 after rotate/remap (1-point closed contour, to: Curve). Several outline commands leave contours that fail `try_to_bezpath` (ConvertSegment→Curve on an all-off-curve closed contour; KeepShape delete of a terminal node on open/closed cubic contours) whereas AddContour/ReplaceLayerOutline validate — outline commands should validate their output.

### Important
5. **Unrelated commands rotate a closed contour's start point**: convert_segment (outline.rs:583) and delete_keep_shape (outline.rs:322) both rotate_left; ConvertSegment(segment 3) on B moves the start from (403,713) to (727,166). Start point is interpolation/hinting significant; SetStartPoint exists for deliberate changes.
6. **Workspace no longer compiles**: fw-api's exhaustive `fw_doc::Error` match misses InvalidCommand, Geometry, MetricsKeyCycle (E0004). Disclosed in the report; branch not mergeable as-is.
7. **SetMetrics lsb/rsb uses the wrong layer's bounds for non-master layers**: `crates/fw-doc/src/command/metrics.rs:21` flattens the master layer regardless of the command's `layer`. fw-model exposes no per-layer flatten.
8. **ConvertSegment { to: Curve } inserts cubic handles into quadratic layers** (B: LLQooQooQ → QooCooQLL). Should infer CurveKind::Quadratic as DeletePoints does.

### Minor
9. `command/outline.rs` is 919 lines (> 800 threshold).
10. `fit_joined_span`'s `target` parameter unused (`let _ = target;` outline.rs:383).
11. `Change.metrics` false for every outline command (moving points changes derived LSB/RSB); RemoveGlyphs sets neither layers nor metrics (command/apply.rs:54-61).
12. `tests/edit_sessions.rs` has one test; begin_edit-while-live, stale token, apply-finalises-live, apply_in_edit/preview_edit equivalence, redo re-applies are untested (verified working by probe); no test for MovePoints with an off-curve point.
13. DuplicateGlyph copies the source's LayerIds verbatim (command/glyphs.rs:303-316); use fresh LayerId::new().
14. Commit subject wording differs from the brief (fine).

## Assessment
Task quality: Needs fixes. Command surface, undo/redo exactness, coalescing, metrics resolution and Change plumbing are solid, but DeletePoints KeepShape is unusable on real outlines, MovePoints cannot move a handle, two reachable panics exist in library code, and the workspace does not compile.
