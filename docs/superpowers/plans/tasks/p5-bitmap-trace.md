# P5 bitmap tracing modes

## Scope and ownership

Own `crates/fc-raster/src/trace.rs`, its exports in `src/lib.rs`, and focused
Rust tracing tests. Implement bounded polygon tracing modes and preserve exact
pixel-font reconstruction through the existing default tracer. No API, model,
frontend, dependencies or FontForge code changes.

## Public interface

- `BitmapTraceMode::{PixelEdges, ChamferedCorners, RoundedCorners, DiagonalSmoothing}`;
  `PixelEdges` is the default.
- `trace_bitmap_with_mode(bitmap, ppem, units_per_em, threshold, mode)`.
- `BitmapTraceOptions { mode, join_diagonals }` and
  `trace_bitmap_with_options(bitmap, ppem, units_per_em, threshold, options)`.
- `trace_bitmap` remains a backwards-compatible exact pixel-edge wrapper.
- All modes preserve source origin, y-up scaling, separately traced diagonal
  components, contour closure, winding and holes. Chamfering cuts one-quarter
  pixel from corner legs. Rounded corners use tangent cubic arcs; bridge corner
  treatments shrink to at most one-quarter the shorter adjacent edge. Diagonal
  smoothing replaces staircase turns with
  half-pixel corner cuts; isolated rectangles retain exact corners.
- Optional diagonal joining adds a quarter-pixel diamond at each diagonal
  contact: foreground becomes eight-connected, background four-connected.
  Pixel centers remain unchanged; closing background passages may create holes.
- Pixel/edge work limits remain bounded; traced edges remain bounded by twice
  the original boundary edge limit and generated points by eight times that
  limit (four points per rounded corner). This is pixel reconstruction, not
  photographic curve fitting or arbitrary image vectorization.

## Acceptance

Write failing tests first. Verify exact-wrapper compatibility, known corner
coordinates, diagonal-step simplification, isolated rectangles, hole winding,
separate diagonal contacts, negative origins, coverage threshold, empty input,
invalid scale and work limits. Exhaustively compare small binary masks for
topology and finite/nondegenerate contours. Run focused `fc-raster` tests and
Clippy under the shared Cargo lock, format only owned Rust files. Parent owns
final workspace checks and API/frontend integration. Record actual final
command results and resource checks in an acceptance report.

## Resource handling

Use `.scratch/p5-bitmap-trace/` for task temporary files and absolute `TMPDIR`.
Use workspace Cargo profiles and target directory. Serialize Cargo through
`.scratch/p5-resume/cargo.lock`; run `scripts/check_artifacts.py` at boundaries.
Remove only owned scratch files after completion. Do not commit.
