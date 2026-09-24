All probes complete. Here are my findings.

## Finding Verdicts

**1. Critical — area-oracle fill-rule mismatch + self-compare fast path** — **ADDRESSED**

- Both sides now `FillRule::NonZero` (`crates/fw-geom/src/boolean.rs:439` input, `:454` output). Fast path no longer calls the oracle at all (`boolean.rs:54-66`); the old `validate_area(&clean_paths, &clean_paths)` is gone. Same-winding nested input is kept off the fast path by the new `nonzero_boundaries_are_normalized` guard (`boolean.rs:60`, defined `:480-498`) so it reaches flo and is measured against the NonZero union of the *input*.
- Re-ran the review's failing probes (width 20, `remove_internal/external = false`):
  - Closed 200×200 square, `Nib::Round`: Miter `Ok` area **16000.000** (analytic 16000, 0.000 %); Round `Ok` **15912.656** (analytic 15914.159, **0.009 %**); Bevel `Ok` **15800.000** (0.000 %). Pre-fix all three were `Err`.
  - Closed triangle: `Ok` for all three joins (12236.50 / 12029.18 / 11846.47). Two disjoint squares: `Ok`, 4 contours, 15825.31 vs analytic 15828.32 (**0.019 %**).
  - Same-winding nested rectangles: `Ok`, **1 contour, 4 points, area 10000.000**. Opposite-winding control still yields outer+hole (3600).
  - DejaVu `B` as-is: `Ok`, output byte-identical to input. `B` with counters reversed (fw-sfnt read + `reverse`): **`Ok`, 1 contour, area 1422634.3 = the outer contour alone** — i.e. the counters are *filled*, not "returned correctly wound".
  - Live oracle: `boolean.rs:834 area_oracle_rejects_a_corrupted_flo_result` (test-only hook at `boolean.rs:786-801`) passes, so `BooleanMismatch` still fires on the non-fast path.
- Deviation, and I agree with it: the dispatch asked for *both* "same-winding nested rects → outer only" *and* "flipped `B` counters come back correctly wound". Those are mutually exclusive under one fill rule (fixing the winding first turns the nested rects into 2 contours/area 3600). The report states this at `task-1-report.md` "Fix round 1" and the implementation is consistently NonZero.
- Corpus check (3583 DejaVu glyphs with contours, `remove_overlap` on unmodified outlines): **460 → 347 `BooleanMismatch`** (114 fixed, 1 new — see New Breakage). Forcing every contour to the same winding: **958 → 1**.

**2. Important — independent `remove_internal`/`remove_external` for all nibs** — **ADDRESSED**

`stroke.rs:191-226 filter_stroke_rings` classifies the max-|area| ring as external and filters each flag separately; both-true short-circuits to `Ok(vec![])` (`stroke.rs:133`, `:163`); defaults flipped to `false/false` (`stroke.rs:59-60`), docs rewritten (`stroke.rs:44-48`). Probe on a closed circle (r=100, w=20), all three nibs:

| nib | (f,f) | int only | ext only | (t,t) |
|---|---|---|---|---|
| Round | 2 contours, 12569.89 | 1, 38023.92 | 1, 25454.03 | 0 |
| Rect | 2, 15984.35 | 1, 39806.94 | 1, 23822.59 | 0 |
| Calligraphic | 2, 12789.47 | 1, 38000.59 | 1, 25211.12 | 0 |

Four-combination test at `tests/stroke.rs:113-170`.

**3. Important — convex-nib tracer replaced by offset-both-sides + caps/joins with curved output** — **ADDRESSED for the stated criteria** (with a caveat covered under New Breakage)

Cell-sweep/convex-hull code is gone; `stroke.rs:228-643` is a support-function tracer (`support_intervals` :397, `support_feature` :447, `TranslatedSegment` + `fit_to_bezpath` :355-395, `append_join` :516, `append_cap` :575, `append_nib_sweep` :608).
- Calligraphic 24×8 @0.4 rad over an open S-curve (cubic): **24 points / 12 on-curve** with Round cap, area **3194.35** vs an independent Minkowski-sweep reference (flatten + convex-cell union via i_overlay) **3193.99** → **0.011 %** (well inside 3 %); bbox [-12.610,-31.451 .. 112.610,31.451]; output contains real `Curve` points. With Butt cap: 22 pts / **10 on-curve = 5 per side** (was 132).
- Caps take effect: Butt/Round/Square give areas 3002.35 / 3194.35 / 3510.00 and different bboxes. Joins take effect on an open corner: Bevel 4350.0, Miter 4350.0, Round 4400.0 (Round matches the sweep reference exactly).
- `Nib::Rect` on the closed circle: area **15984.35**, 0.10 % from the test's 16000 and **0.014 %** from my independent sweep reference (15986.63). (The test's "analytic" 16000 is a coincidence — the true Minkowski sweep is 15986.6, not the 16657 a naive erosion-by-√2·10 formula gives; the implementation is right.)

**4. Important — test gaps** — **ADDRESSED.** Closed-contour strokes `tests/stroke.rs:73-110`; pre-passes asserted in-crate `boolean.rs:849`; boolean areas 15000/5000/5000/10000 + the 4-point 50×100 intersect `tests/boolean.rs:71-86`; 3-level nesting `tests/direction.rs:26-36`; simplify max sampled deviation ≤ error `tests/simplify.rs:32-34`. Suite: **42 tests, all pass in debug and release**; `cargo fmt --all --check` and `cargo clippy -p fw-geom --all-targets -- -D warnings` clean (I re-ran all four).

**5. Minors** — **ADDRESSED** (one partial): `PathHit.segment` docs `edit.rs:52-56`; `segment_to_points` `edit.rs:141-197` (I validated it independently on closed-cubic, off-curve-start, open, and TT-quad contours — 1:1 mappings correct, quad chains map both materialised segments to the enclosing on-curve pair, out-of-range → `(len,len)`); contour order preserved `boolean.rs:97-110` + probe `["open@-20","closed(8)","open@200"]`; `nearly_hv_lines` threshold `edit.rs:27`; sweep + bbox pre-filter `boolean.rs:686-750`; `clean_path` precondition doc + `debug_assert` `boolean.rs:282-291`; extremum re-add tolerance `edit.rs:68-72`. **Partial:** the brief forbids `Result` on `from_bezpaths`/`simplify`, so invalid parameters are logged as `Error::InvalidParameter` via `tracing` and rejected deterministically (`convert.rs:28-38` → empty vec; `edit.rs:73-82` → input unchanged) rather than returned; documented and tested. `remove_overlap`'s doc comment (`boolean.rs:31-33`) still doesn't state the new order guarantee even though the behaviour is now correct.

## New Breakage in the Fix Diff

1. **Important — closed-contour strokes now come back fully polygonal.** `stroke.rs:148` and `stroke.rs:171` route every stroked source contour through `boolean.rs:500 normalize_self_intersecting_contours`, which flattens *all* rings at tolerance 0.05 and rebuilds them as line-only contours whenever *any* ring self-intersects. kurbo's stroke of any closed cornered contour always produces a self-intersecting inner ring (I verified: inner subpath `selfint=true` for Miter/Round/Bevel on a 200×200 square), so the trigger is the normal case:
   - 200×200 square, Round join: outer ring **40 points, 0 off-curve** (should be 4 lines + 4 arcs).
   - DejaVu `B`, round nib: 33 source points → 6 contours, **855 points, 0 off-curve**. DejaVu `A`: 126 points, 0 off-curve. DejaVu `O` (no corners) still curved.
   - Corpus: of 3174 successful round-nib strokes, **2789 are fully polygonal**, 385 curved. Pre-fix the same call (both flags false) returned curved output for all 3583.
   - Same for the convex nib on closed input: `Nib::Rect`/`Calligraphic` on a closed circle → `[112, 100]` points, **0 off-curve** — the review's original complaint (`[109, 95]`) is unchanged for closed contours; only open contours got the curved tracer.
2. **Minor — one glyph newly rejected.** `uni2727` `remove_overlap` was `Ok` pre-fix and is now `Err(BooleanMismatch { 297021.62, 514112.51 })`. Root cause: `validate_area` (`boolean.rs:435`) measures flo's **raw** output with NonZero, but flo emits the counter with the same winding as its parent and `remove_overlap` only normalises windings afterwards via `correct_direction` (`boolean.rs:88-90`). NonZero over-counts (514112.5 = outer alone) what EvenOdd — and the final output — reads correctly (297021.6). Net across the corpus is still strongly positive (−114/+1); measuring the output after `correct_direction` would close this class.

## Out-of-Scope Observations

- `expand_stroke` still returns `BooleanMismatch` for **409/3583 (11.4 %)** DejaVu glyphs (e.g. `two`, `at`, `questiondown`). These are genuine flo failures, not oracle artefacts — I reproduced them on both curved and pre-flattened ring sets, and flo returns the outer union while dropping inner rings. Pre-fix the equivalent configuration failed 3558/3583, so this is a large improvement, but the feature is still unusable on ~1 glyph in 9. `remove_overlap` likewise still fails on 347/3583 unmodified glyphs (pre-existing, 460 before).
- `StrokeOptions::default()` changed `remove_internal`/`remove_external` from `true` to `false`. No other crate depends on `fw-geom` yet, so no caller breaks.
- `round_stroke` now runs `remove_overlap` unconditionally (`stroke.rs:154`), so `expand_stroke` can return `Err` even when no ring removal was requested — that is what surfaces the 11.4 % above.
- A flo/i_overlay sort-comparator panic (`user-provided comparison function does not correctly implement a total order`) is still reachable and still correctly contained by `catch_unwind`.

## Verdict

**Findings remain open** — not as unaddressed review items (all five are addressed, with the `B`-counter sub-clause consciously and correctly resolved as contradictory), but the fix introduces one Important regression that should be fixed before sign-off:

- Closed-contour stroke output is fully polygonalised by `normalize_self_intersecting_contours` (`stroke.rs:148`, `stroke.rs:171`, `boolean.rs:500`) — 2789/3174 DejaVu strokes have zero off-curve points; DejaVu `B` goes from 33 source points to 855 line points; the convex-nib closed circle is still `[112, 100]` line points. Normalisation should be applied only to the ring that actually self-intersects, or performed curve-preservingly, not by flattening the whole set.
- Minor follow-up: the oracle's NonZero measurement of flo's pre-`correct_direction` output cost one previously-passing glyph (`uni2727`).