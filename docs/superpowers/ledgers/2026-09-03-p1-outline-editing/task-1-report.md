# P1 Task 1 Report — `fw-geom`

**Status:** DONE  
**Commit:** `76c3fbf feat(fw-geom): add outline geometry library`

## Implemented

- Added workspace crate `crates/fw-geom` and workspace dependencies `flo_curves 0.8`, `i_overlay 8`, and test-only `proptest`.
- Implemented the complete brief API: cubic/quadratic conversion, four booleans, overlap removal with FontForge cleanup passes and area oracle, round and convex-nib stroke expansion, simplify policy flags, extrema/inflections, direction/canonicalization/reversal, transforms/grid rounding, nearest/split/intersections/bbox, and layer compatibility/interpolation.
- Quadratics are exactly elevated; cubic-to-quadratic conversion uses `cubics_to_quadratic_splines` and emits explicit `QCurve` endpoints without implied TT indices.
- Round strokes use `kurbo::stroke`; rectangular/calligraphic strokes use the documented convex-nib sweep tracer. Third-party boolean/oracle panics are contained as errors.

## Tests-first evidence

Initial `cargo test -p fw-geom` failed with unresolved imports for the newly specified API. Implementation then made the integration suite green. Coverage includes DejaVu `O` preservation and `g` nearest-hit behavior, overlapping rectangles, self-touching input, all boolean ops, both non-round nibs on lines/curves, simplification/extrema/inflections, nested winding, quadratic and cubic splits, interpolation diagnostics, and the three required proptest invariants.

## Final verification

- `cargo fmt --all --check` — pass (no output)
- `cargo clippy -p fw-geom --all-targets -- -D warnings` — pass
- `cargo test -p fw-geom` — pass, 27 tests + doc-tests
- `cargo test -p fw-geom --release` — pass, 27 tests + doc-tests

## Files / self-review / concerns

Only root `Cargo.toml`/`Cargo.lock` and new `crates/fw-geom/**` were committed. All public items are documented; library code contains no `unwrap`, `expect`, or explicit panic. Untouched contours retain metadata/TT indices, while changed conversion output is unnumbered. Working tree was clean. No open concerns were recorded in the initial round.

## Fix round 1

**Status:** DONE

- Critical / boolean oracle: `crates/fw-geom/src/boolean.rs:34-94,435-498` now skips validation on the genuine unchanged fast path, checks both input and flo output as NonZero polygon unions, and rejects redundant same-winding nested boundaries from the fast path. `tests/boolean.rs:68-147` covers exact areas/topology, same-winding rectangles (outer-only area 10000), correctly wound DejaVu `B` counter preservation, and same-winding DejaVu `B` as a successful solid NonZero union. `boolean.rs:791-897` injects a corrupt flo result under `cfg(test)` and proves that `BooleanMismatch` still fires. Note: the review's simultaneous request for same-winding nested rectangles to become solid and same-winding `B` counters to remain holes is contradictory under NonZero; the implementation consistently follows the explicitly requested NonZero semantics.
- Independent stroke rings: `crates/fw-geom/src/stroke.rs:32-63,130-224` classifies outer/inner rings per source contour and filters the two flags independently for every nib; both true returns warning-free `Ok(vec![])`. Defaults now keep both rings. `tests/stroke.rs:113-170` covers all four combinations for Round, Rect, and Calligraphic nibs.
- Curved convex-nib tracer: `crates/fw-geom/src/stroke.rs:228-643` replaces flattened cell unions with tangent/support-feature intervals, `fit_to_bezpath` cubic refitting, two-sided traces, convex-nib cap sweeps, bevel/miter/round joins, and final overlap removal. `tests/stroke.rs:173-279` checks a compact curved calligraphic cubic, the Rect/circle analytic swept area within 2%, and observable cap/join behavior.
- Closed-stroke/oracle regressions: `tests/stroke.rs:73-110` covers a 200x200 square with Miter/Round/Bevel analytic areas within 1%, a triangle, and two disjoint squares. `boolean.rs:500-532` normalizes only self-intersecting closed-stroke ring scaffolding before the curve-preserving overlap pass.
- Test gaps: `boolean.rs:849-872` asserts tiny-segment removal, endpoint-control repair, and immediate backtrack cancellation. `tests/boolean.rs:68-86` asserts 15000/5000/5000/10000 boolean areas and the four-point 50x100 intersection. `tests/direction.rs:26-36` covers three-level winding. `tests/simplify.rs:7-35` checks sampled maximum deviation <= error as well as point count.
- Segment indexing/docs: `crates/fw-geom/src/edit.rs:49-63,141-197` documents the exact BezPath segment basis, materialised implied quadratics, invalid mapping sentinel, and exports `segment_to_points`; `tests/nearest.rs:36-53` covers a rotated closed quadratic chain.
- Remaining minors: `boolean.rs:97-110` interleaves unchanged open contours back into their original slots; `boolean.rs:282-291` documents and debug-asserts the single-subpath cleanup precondition; `boolean.rs:682-752` uses an x-sorted sweep with bbox rejection (`O(E log E + K)`). `edit.rs:27-28,65-72` documents the nearly-H/V threshold and extrema re-add tolerance. `convert.rs:28-38` and `edit.rs:72-82` remove silent numeric fallback substitution and report `Error::InvalidParameter` through tracing while deterministically rejecting invalid input; their brief-mandated non-`Result` public signatures remain unchanged. `tests/extrema.rs:55-65` and `tests/simplify.rs:66-76` cover rejection.

### Verification output

- `cargo fmt --all --check` — pass (no output)
- `cargo clippy -p fw-geom --all-targets -- -D warnings` — `Finished dev profile [unoptimized + debuginfo]`
- `cargo test -p fw-geom` — `test result: ok` throughout; 42 tests plus doc-tests, 0 failed
- `cargo test -p fw-geom --release` — `test result: ok` throughout; 42 tests plus doc-tests, 0 failed
