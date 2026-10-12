### Task 1: `fw-geom` — geometry library

**Files:** `crates/fw-geom/src/*` per the structure; tests `crates/fw-geom/tests/{boolean.rs, stroke.rs, simplify.rs, extrema.rs, direction.rs, transform.rs, nearest.rs, interpolate.rs, fixtures.rs}`.

**Interfaces (produced):**
```rust
pub type Tolerance = f64;   // font units; default 0.01 for booleans, 0.1 for simplify
pub fn to_bezpaths(contours: &[Contour]) -> Vec<BezPath>;              // one per contour, quadratic contours degree-elevated to cubic
pub fn from_bezpaths(paths: &[BezPath], kind: CurveKind /* Cubic | Quadratic { max_err: f64 } */) -> Vec<Contour>;  // Quadratic uses kurbo::cubics_to_quadratic_splines
pub enum BooleanOp { Union, Intersect, Exclude /* A − B */, Xor }
pub fn remove_overlap(contours: &[Contour], kind: CurveKind, tol: Tolerance) -> Result<Vec<Contour>, Error>;   // flo_curves path_remove_interior_points after pre-passes; i_overlay area oracle: Err(Error::BooleanMismatch{area_a, area_b}) if |Δarea| > 1% 
pub fn boolean(a: &[Contour], b: &[Contour], op: BooleanOp, kind: CurveKind, tol: Tolerance) -> Result<Vec<Contour>, Error>;
pub struct StrokeOptions { pub width: f64, pub nib: Nib /* Round | Calligraphic { width, height, angle } | Rect */, pub cap: Cap, pub join: Join, pub miter_limit: f64, pub remove_internal: bool, pub remove_external: bool }
pub fn expand_stroke(contours: &[Contour], opts: &StrokeOptions, kind: CurveKind) -> Result<Vec<Contour>, Error>;   // kurbo::stroke for Round; Calligraphic/Rect via flo_curves stroke_path or the FontForge convex-nib tracer (document which)
pub struct SimplifyOptions { pub error: f64, pub ignore_slopes: bool, pub ignore_extrema: bool, pub smooth_curves: bool, pub force_lines: bool, pub nearly_hv_lines: bool, pub start_at_extrema: bool }
pub fn simplify(contours: &[Contour], opts: &SimplifyOptions) -> Vec<Contour>;
pub fn add_extrema(contours: &[Contour]) -> Vec<Contour>;            // only where missing; tolerant to near-extrema (FontForge 'ForceEndPointExtrema' policy)
pub fn add_inflections(contours: &[Contour]) -> Vec<Contour>;
pub fn round_to_grid(contours: &[Contour], step: f64) -> Vec<Contour>;   // keeps TtIndex; round handles too
pub fn correct_direction(contours: &[Contour], outer_clockwise: bool /* TrueType true, PostScript false */) -> Vec<Contour>;   // signed area + containment nesting
pub fn canonical_start_points(contours: &[Contour]) -> Vec<Contour>;   // leftmost-then-lowest on-curve
pub fn canonical_order(contours: &[Contour]) -> Vec<Contour>;
pub fn reverse(contour: &Contour) -> Contour;
pub fn transform(contours: &[Contour], affine: Affine) -> Vec<Contour>;
pub fn nearest_on_path(contours: &[Contour], p: Point, max_dist: f64) -> Option<PathHit { contour: usize, segment: usize, t: f64, point: Point, distance: f64 }>;
pub fn split_segment(contour: &Contour, segment: usize, t: f64) -> Contour;   // inserts an on-curve point at t (de Casteljau; quadratic aware)
pub fn intersections_with_line(contours: &[Contour], line: Line) -> Vec<(Point, usize, usize, f64)>;   // for the measurement tool
pub fn bbox(contours: &[Contour]) -> Option<Rect>;
pub struct Incompatibility { pub contour: Option<usize>, pub reason: String }
pub fn compatible(a: &Layer, b: &Layer) -> Result<(), Vec<Incompatibility>>;
pub fn interpolate(a: &Layer, b: &Layer, t: f64) -> Result<Layer, Error>;     // points, components (same base/order), anchors by name, width
```
- [ ] **Step 1: Tests first** using FontForge fixtures where present (`fw_fixtures::external("fontforge-tests")` → `OverlapBugs.sfd` cannot be read yet; instead construct cases from DejaVu glyphs via fw-sfnt): remove overlap on two overlapping rectangles → 1 contour, 8 points; union of DejaVu `O` outer+inner unchanged; `remove_overlap` on a self-touching contour does not panic and either succeeds or returns `BooleanMismatch`; stroke of a straight line with round cap yields a stadium with 4 on-curve extrema; `simplify` of a 200-point noisy circle < 20 points within error; `add_extrema` inserts exactly 4 points on a rotated ellipse; `correct_direction` flips a counter-clockwise outer contour; `round_to_grid` preserves `TtIndex`; `nearest_on_path` returns the right segment/t on DejaVu `g`; `split_segment` keeps the curve shape (sample distance < 1e-6); `interpolate` at t=0.5 between two compatible layers; `compatible` reports point-count mismatch with the contour index.
- [ ] **Step 2: Implement**; keep flo_curves conversion in one module; expose `Tolerance` defaults as constants.
- [ ] **Step 3:** `cargo test -p fw-geom`, clippy clean, commit `feat(fw-geom): booleans, stroke, simplify, extrema, direction, transforms, nearest, interpolation`.

---

