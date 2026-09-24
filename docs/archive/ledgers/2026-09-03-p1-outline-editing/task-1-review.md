## Spec Compliance — ❌

**✅ API completeness.** Every item in the brief exists with the exact signature: `Tolerance`, `to_bezpaths`/`from_bezpaths`/`CurveKind`, `BooleanOp`, `remove_overlap`, `boolean`, `StrokeOptions`/`Nib`/`expand_stroke`, `SimplifyOptions`/`simplify`, `add_extrema`, `add_inflections`, `round_to_grid`, `correct_direction`, `canonical_start_points`, `canonical_order`, `reverse`, `transform`, `nearest_on_path`/`PathHit`, `split_segment`, `intersections_with_line`, `bbox`, `Incompatibility`, `compatible`, `interpolate`. Both tolerance defaults are exposed as documented constants (`lib.rs:31,34`). `#![warn(missing_docs)]`, no `unwrap`/`expect`/`panic!` anywhere in `src/`; `cargo fmt --all --check`, `cargo clippy -p fw-geom --all-targets -- -D warnings` and `cargo test -p fw-geom` (27 tests + 3 proptests) all pass as reported. Commit scope is exactly `Cargo.toml`/`Cargo.lock` + the new crate.

**✅ Quadratic path.** Exact `QuadBez::raise()` elevation; `cubics_to_quadratic_splines` with a per-path fallback; every quad gets an explicit `QCurve` endpoint (implied-point-free). Measured `max_err` honour on a cubic source: dev = 0.00075 / 0.047 / 0.64 / 3.33 for `max_err` = 0.001 / 0.05 / 1 / 10. Quadratic round-trip on DejaVu `o` is exact (0.0 deviation), point count 12→16 as implied points materialise. `TtIndex` is `Unnumbered` on generated points, preserved on untouched contours (`restore_unchanged_contours`, boolean.rs:483) and on coordinate-matching points (edit.rs:498).

**❌ Boolean robustness — the area oracle is self-contradictory** and rejects ordinary inputs (details below).

**⚠️ `expand_stroke`.** Round → `kurbo::stroke`, caps/joins honoured (Butt gives bbox 0..100, Round/Square −15..115). Rect/Calligraphic path *is* documented (stroke.rs:62-66) but is a flatten-and-union cell sweep, not the dispatch's "offset both sides by the nib half-vector + caps"; `cap`/`join` are silently ignored there and `remove_internal`/`remove_external` are not independent.

**✅ simplify / extrema / direction / numerics.** Noisy 200-point circle → 19 points with max deviation 0.514 at error 1.0; extrema survive; `force_lines` threshold documented. 3-level nesting parity verified (outer cw, hole ccw, island cw); `canonical_start_points` picks (0,0) on my probe; `canonical_order` stable under input permutation. `nearest`/`split`/`intersections` use one consistent segment basis and round-trip (`nearest → split_segment` works on a contour that starts with an off-curve point); `t` always in range.

**✅ compatible/interpolate.** Kind sequence, component count/base, anchors by name with duplicate guarding, width; no panics on wildly mismatched layers; extrapolation documented.

## Strengths
- Third-party panics from flo_curves and i_overlay are contained by `catch_unwind` and mapped to typed errors — good instinct for untrusted input.
- Genuinely accurate numerics where it counts: round stroke of a circle is within 0.01 % of the analytic annulus area; `split_segment` preserves the curve to 1e-8; `nearest_on_path` is exact on a real DejaVu `g` segment; bowtie and self-touching contours resolve without panic (`[3,3]`, `[3,4]`).
- `reverse` correctly rotates point kinds for both open and closed contours (round-trips exactly with mixed line/curve segments).
- Degenerate input handled uniformly: empty, single-point, zero-area, open-only, bad tolerance — all no-panic with sensible results.

## Issues

#### Critical

**1. The area oracle compares two different fill rules, producing false `BooleanMismatch` on mainstream inputs** — `crates/fw-geom/src/boolean.rs:397-440` (`FillRule::NonZero` for the input at :402 vs `FillRule::EvenOdd` for the output at :416), aggravated by the fast path calling `validate_area(&clean_paths, &clean_paths)` (`boolean.rs:57`), which compares a path set *against itself* and can therefore only ever measure the disagreement between the two rules.

Verified consequences through the public API:

- `expand_stroke` with `Nib::Round` **fails on essentially every closed contour with corners**:
  - closed 200×200 square, join Round/Miter/Bevel → `Err(boolean area mismatch: oracle=23804.66, result=52254.66)`
  - closed triangle → `Err` (all three joins); two disjoint squares → `Err`
  - only smooth shapes pass (circle, ellipse → `Ok`)
  The stroke rings are correctly wound (outer +52707.1, inner −28450.0, no boundary intersections), so the fast path fires and the oracle reports 23804.66 (NonZero, the correct annulus) against 52254.66 (EvenOdd of *the same two paths* = annulus + inner square). Open contours are unaffected (L-polyline, zigzag, crossing X all `Ok`), which is why the suite is green — `tests/stroke.rs` only strokes open contours.
- `remove_overlap` fails on nested contours that share a winding — exactly the input Correct Direction exists to repair: two nested rectangles wound the same way → `Err(BooleanMismatch { area_a: 10000.0, area_b: 6400.0 })`, and real DejaVu `B` with its counters flipped → `Err(... oracle=1177849.5, result=853936.8)`. Running `correct_direction` first makes it `Ok`, but callers can't be required to do that: overlap removal on a mis-wound outline is a core use case.

Both sides must use the same fill rule (NonZero for both is the semantics the model implies), and the fast path should skip the oracle entirely rather than compare a set with itself.

#### Important

**2. `remove_internal` / `remove_external` are not independent and are partly ignored** — `stroke.rs:143-147`. Both flags collapse into a single "run `remove_overlap`" decision, so `{internal:true, external:false}` and `{internal:false, external:true}` return byte-identical results (verified: both `[15, 15]`). For `Nib::Rect`/`Calligraphic` the flags are never read at all — `convex_nib_stroke` always unions (`stroke.rs:184`). The field docs promise two separate behaviours.

**3. The convex-nib tracer flattens the outline and unions one cell per flattened edge** — `stroke.rs:150-185`. Output is fully polygonal (a single stroked cubic yields a 132-point line contour; a calligraphic closed circle yields `[109, 95]`), `cap`/`join` are ignored, the boolean engine is fed O(flattened-edges) input paths, and it is fragile: `Nib::Rect` on a closed circle returns `Err(boolean area mismatch: oracle=23950.15, result=20347.77)` — here the flo result really is wrong, but the feature is still unusable for that input. The dispatch's fallback ("offset both sides by the nib half-vector + caps", convex nib) keeps curves and cap/join semantics.

**4. Test gaps that let #1 through.** No test strokes a closed contour; no test exercises the pre-passes (tiny segments, stupid control points, backtracks — all three work, I verified them by probe, but nothing in `tests/` covers them); nothing ever triggers or asserts `BooleanMismatch` or the 1 % threshold; no 3-level nesting test. Two tests assert nearly nothing: `tests/boolean.rs:65` checks only `is_ok()` for all four ops (`Intersect` of the two fixtures should be a 4-point 50×100 rect — I verified areas 15000/5000/5000/10000 are all correct, so the assertions are cheap to add), and `tests/stroke.rs:33` asserts only `!is_empty()` for both non-round nibs. `tests/simplify.rs:31` checks the point count but not the "within error" half of the brief's criterion.

#### Minor

- `PathHit.segment` (`edit.rs:53`) indexes `Contour::to_bezpath().segments()`, which for a closed contour starts at the first *on-curve* point and materialises implied quadratic midpoints — self-consistent across `nearest_on_path`/`split_segment`/`intersections_with_line`, but a UI caller cannot map a hit back to a point index from the docs alone. Worth one sentence.
- `remove_overlap` appends open contours after the closed ones (`boolean.rs:86`), so input order is not preserved; the doc says only that they are "returned unchanged".
- Silent parameter fallbacks: `from_bezpaths` substitutes 0.01 for a non-finite/non-positive `max_err` (`convert.rs:153`) and `simplify` substitutes the default error (`edit.rs:64`), while every other invalid parameter yields `Error::InvalidParameter`.
- `nearly_hv_lines`' threshold (it is `opts.error`) is undocumented on the field, unlike `force_lines`.
- `has_boundary_intersections` (`boolean.rs:571`) is O(E²) over the flattened edges of the whole glyph on the fast path of every `remove_overlap` call.
- `clean_path` (`boolean.rs:259`) folds a multi-subpath `BezPath` into one subpath; safe today because all callers pass single-subpath paths, but it is an easy trap.
- `simplify` implements "never remove extrema" by re-adding extrema after fitting with an 8×`error` endpoint tolerance (`edit.rs:110`) rather than protecting the originals, so extremum positions can move by up to `error`. Acceptable, but undocumented.

## Assessment
**Task quality: Needs fixes.** The API, docs, quadratic handling and numerics are solid, but the oracle's NonZero-vs-EvenOdd asymmetry makes `expand_stroke(Nib::Round)` fail on any closed cornered outline and `remove_overlap` fail on same-winding nested contours — two headline P1 features broken on ordinary input, invisible to the current suite because no test strokes a closed path or removes overlap from a mis-wound outline.