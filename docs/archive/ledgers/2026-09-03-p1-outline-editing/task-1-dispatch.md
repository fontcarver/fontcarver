You are implementing **P1 Task 1: `fw-geom` — geometry library** for FontWorkshop (Tauri 2 font editor: Rust kernel + React/TS UI). This directory is the git worktree for branch `p1-t1`. Create the new crate `crates/fw-geom` (add it to the workspace members) — do not modify other crates except `Cargo.toml`/`Cargo.lock` at the workspace root.

## Read first
1. `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-1-brief.md` — requirements; the interface block is normative (names, signatures, semantics).
2. `CONVENTIONS.md` (binding). The model types you consume: `crates/fw-model/src/{contour.rs, layer.rs, component.rs, anchor.rs}` (`Contour`, `Point`, `PointKind`, `TtIndex`, `Layer`, `Component`, `Anchor`; `Contour::to_bezpath/try_to_bezpath/from_bezpath` already exist — reuse them).
3. `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/research/rust-crate-spike.md` §2.h (kurbo 0.13 caveats: no public `CubicBez::lower`, use `cubics_to_quadratic_splines`; `offset_cubic` is the only offset API; `Affine::svd` private; no curve/curve intersection in kurbo) and §2.k (flo_curves: `path_remove_interior_points` is Remove Overlap; `SimpleBezierPath` shim ~40 lines; degree-elevate quads first; keep i_overlay as an area oracle). Example code: `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/research/spike-examples/boolops.rs`.
4. `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/research/fontforge-algorithms-and-otlayout.md` Part 2 — port the *policy* pieces named there (remove-tiny / stupid-control-point / backtrack pre-passes before booleans; simplify flag semantics; `ForceEndPointExtrema` tolerance; correct-direction by signed area + containment nesting). Do not port FontForge's overlap or stroke engines.

## Decisions (do not revisit)
- Dependencies: `kurbo` (workspace), `flo_curves = "0.8"`, `i_overlay = "8"` (oracle only), `thiserror`, `tracing`; add them to `[workspace.dependencies]`. No skia, no lyon.
- Quadratic handling: all operations work on cubic `BezPath`s; `to_bezpaths` elevates quadratic contours exactly; `from_bezpaths(.., CurveKind::Quadratic{max_err})` converts with `kurbo::cubics_to_quadratic_splines` and re-derives implied-point-free `QCurve/OffCurve` point lists (drop `TtIndex` to `Unnumbered` for changed contours; keep it for untouched contours where the API allows).
- `remove_overlap`: pre-passes → flo_curves `path_remove_interior_points(accuracy = tol)` → post-pass (drop zero-length segments, merge collinear line runs) → area oracle with `i_overlay` union of the flattened input (tolerance 0.05): `Err(Error::BooleanMismatch{..})` when |Δarea| > 1 % of max(area).
- `expand_stroke`: `Nib::Round` → `kurbo::stroke` (`StrokeOpts` default) then `remove_overlap`; `Nib::Rect` and `Nib::Calligraphic` → flo_curves `stroke_path` if it supports the nib shape, otherwise implement the "offset both sides by the nib half-vector + caps" tracer described in the FontForge doc (convex nib); document which path is taken and test both nibs on a straight segment and on a curve.
- `simplify`: kurbo `simplify_bezpath` for the point-reduction core, wrapped by the FontForge flag policy (`force_lines`, `nearly_hv_lines`, `start_at_extrema`, `smooth_curves`); never remove points that are extrema unless `ignore_extrema`.
- `correct_direction`: outer contours clockwise for `outer_clockwise = true` (TrueType) else counter-clockwise; nesting parity by point-in-polygon of a sample point against other contours' flattened polygons.
- `nearest_on_path`: kurbo `ParamCurveNearest` per segment; return the closest within `max_dist` (font units).
- `interpolate`/`compatible`: same contour count, per-contour same point count and `PointKind` sequence (smooth flags may differ), same component count/base order, anchors matched by name (unmatched → incompatibility), width interpolated; component transforms interpolated element-wise.
- Public API is exactly the brief's; all `pub` items documented; `#![warn(missing_docs)]`; no `unwrap`/`expect` in library code; property-style tests with `proptest` (add as dev-dep) for: transform(inverse(transform)) ≈ identity, split_segment preserves the curve, round_to_grid idempotent.
- Commit on branch p1-t1 with Conventional Commits; do not push.

## Report contract
Full report to `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-1-report.md` (implemented, tests-first evidence, commands with final output, files, self-review, concerns). Final message under 15 lines: Status (DONE | DONE_WITH_CONCERNS | BLOCKED), commits, one-line test summary, concerns, report path.

## Verification you must run before reporting
```
cargo fmt --all --check
cargo clippy -p fw-geom --all-targets -- -D warnings
cargo test -p fw-geom
cargo test -p fw-geom --release
```
Do not spawn other agents; do exactly this task's scope.
