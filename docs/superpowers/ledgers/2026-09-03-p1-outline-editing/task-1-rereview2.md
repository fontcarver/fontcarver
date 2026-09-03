All probes are complete. Here is the re-review.

## Finding Verdicts

**1. Important — closed-contour strokes were polygonalised; must be resolved curve-preservingly** — **ADDRESSED**

Implementation: `boolean.rs:545-568 normalize_self_intersecting_contours` now works ring-by-ring, splitting only rings that actually self-intersect (`boolean.rs:574-636 split_self_intersecting_ring` via flo `path_intersects_path` + `find_self_intersection_point`, `subsegment` cuts, `directed_piece_loops` at `:670`), and `stroke.rs:148-160` re-fits smooth cubic runs only for closed round strokes. Every acceptance probe reproduces (my numbers, width 20, both flags false):

| probe | result |
|---|---|
| 200×200 square, Round join | 2 contours; outer 16 pts = **8 on-curve (4 `Line` + 4 `Curve`) + 8 off-curve**, area 48314.247; inner 4 `Line` pts, area −32400; band **15914.247** vs analytic 15914.159 (0.0006 %) and independent Minkowski 15914.151; bbox (−10,−10)-(210,210) |
| same, Miter / Bevel | 4+4 pts / area **16000.000** exact; 8+4 pts / **15800.000** exact |
| triangle (0,0)(200,0)(100,180), Round | outer 13 pts (3 curved corners) + inner 3 pts; area **12029.757** vs Minkowski 12030.694 (0.008 %) |
| DejaVu `B`, round nib | source 17 on-curve → 6 contours, **45 on-curve (2.6×, ≤3× ✓), 54 off-curve**; area 180461.0 vs Minkowski 180409.9 (**0.028 %**) |
| DejaVu `A` / `O` | 36 on/28 off (0.001 %) / 32 on/64 off (0.018 %) |
| `Nib::Rect`, closed circle r=100 | 2 contours, 13 on-curve / **16 off-curve**, area 15986.877 vs Minkowski 15986.632 (0.002 %) |
| corpus, round nib | **3250/3583 successful, 3250/3250 curved (100 %)** — report's claim confirmed; test asserts ≥90 % at `tests/stroke.rs:214` |
| corpus sample, convex nibs | `Rect` 138/138 and `Calligraphic` 129/129 successes curved (0 polygonal) |
| accuracy of successes | 812-glyph sample: **0 glyphs >2 %** area deviation from an independent Minkowski reference, 2 >1 % (worst 1.86 %, `uni0307` — *not* caused by the new compaction: uncompacted rings deviate identically) |

Tests assert the criteria at `tests/stroke.rs:114` (4 lines + 4 curves, ≤12 outer on-curve, inner 4), `:155` (B ≤3×, off-curve >0), `:198` (Rect circle curved), `:214` (release-only corpus ≥90 %). The success-rate collateral is under New Breakage.

**2. Minor — oracle measured before `correct_direction`; `uni2727`** — **ADDRESSED** (test now vacuous)

`boolean.rs:92-96` orders it correctly: `postprocess` → `correct_direction` → `validate_area(&clean_paths, &to_bezpaths(&output))`. `uni2727` is `Ok` (2 contours). Caveat: with the new fast path `uni2727` no longer reaches flo at all (probe: `crossing_or_overlap=false, windings_normalized=true`), so `tests/boolean.rs:213` passes without exercising the oracle ordering it was written for. Only 3 corpus glyphs still reach flo, so the ordering fix is exercised solely by those and the `cfg(test)` corruption hook.

**3. `remove_overlap` fast path exact + `remove_overlap_with_fallback`** — **ADDRESSED**

Fast path `boolean.rs:66` = `!has_crossing_or_overlap(&original_paths) && windings_are_normalized(&closed)`; exact segment test at `boolean.rs:929-1050` (`PathSeg::intersect_line`, x-sweep, 1e-3 touch tolerance, `collinear_overlap`), winding check at `:531-543` via `correct_direction`. API at `boolean.rs:101-146` + `752-806`, exported `lib.rs:14-17`, UI-warning contract documented at `boolean.rs:115-118,122-127`.

My independent corpus run (3583 DejaVu glyphs with closed contours): **strict `remove_overlap` 3583 ok / 0 err**, 3580 byte-identical, 3 changed (`shade`, `uniF001`, `uniF003`) with **identical NonZero fill area in vs out**; **`remove_overlap_with_fallback` 3583/3583, 0 `Flattened`**. An independent overlap detector (NonZero vs EvenOdd fill area of the flattened input) finds **0/3583 genuinely overlapping glyphs → 0 fast-path false accepts**. Adversarial probes confirm the path is not too lenient: rect/rect overlap at depth 0.0005 upward → merged; corner-only overlap ≥0.01 → merged (0.0005–0.005 still routed to the boolean); circles overlapping by 0.001 → merged, exactly tangent → unchanged; bowtie → 2 contours/5000; duplicated square → 1 contour/10000; rotated cross → merged. Tests present with release-only corpus assertions (`tests/boolean.rs:245`, plus `:34`, `:51`, `:235`, `boolean.rs:1129`).

Context worth relaying: the 3583/3583 headline mostly means "nothing in DejaVu overlaps, so 3580 take the fast path". The genuine-overlap path is unchanged and still broken — `O` + an overlapping blob gives `Err(BooleanMismatch oracle=784595.8 result=1741314.8)` (the counter is deleted); I verified the identical numbers on f03285d, so it is **pre-existing, not new**.

**4. Contour-order guarantee documented** — **ADDRESSED**: `boolean.rs:38-45` states byte-for-byte + order on the fast path and the open/closed slot rule otherwise; `tests/boolean.rs` `open_contours_retain_their_relative_slots` covers it (probe: `["open","closed","open"]`).

Verification re-run by me on the worktree: `cargo fmt --all --check` clean, `cargo clippy -p fw-geom --all-targets -- -D warnings` clean, debug 50 tests pass, release 52 pass; corpus test output `total=3583, strict=3583, strict_unchanged=3580, changed=["shade","uniF001","uniF003"], fallback=3583, flattened=0` and `total=3583, successful=3250, failed=333, curved=3250`.

## Stroke failure analysis (333/3583 round nib)

**What fails.** All 333 are `BooleanMismatch`, and in **333/333 flo's area is larger than the oracle's** (e.g. `two` 631797 vs 132894; `s` 511440 vs 114556; `uni01E5` 1301263 vs 205781) — flo returns the outer boundary with the band's hole deleted.

**Why — genuine flo failure, by design of the primitive.** `flo_curves 0.8.1` documents `path_remove_interior_points` (`add.rs:76-84`) as: *"This considers only the outermost edges of the path to be the true edges, so if there are sub-paths inside an outer path, they will be removed."* A closed stroke is exactly outer ring + hole ring, so every ring set that reaches flo mismatches by construction. Strict `remove_overlap` on the 333 ring sets: **0/333 ok**.

**Would routing through `remove_overlap_with_fallback` make it 100 %?** Yes for success: **333/333 ok**, 330/333 within 2 % of an independent Minkowski reference (the 3 outliers `uni2603` 6.2 %, `uni2624` 10.2 %, `uni2695` 19.7 % are wrong in the *ring set itself*, not in the fallback). But the geometry is unusable: `two` 107 ring points → **3457 points**, `s` → **6000**, `O`+blob → **3633**, because `fit_polygon_to_cubics` (`boolean.rs:785-806`) calls `fit_to_bezpath` on **each polygon edge separately**, emitting one cubic per flattening edge.

**Better remedy (measured).** Swapping only the stroke-side primitive to flo's hole-preserving `path_remove_overlapped_points` (`add.rs:123`, `set_exterior_by_adding`), keeping the existing NonZero oracle:

- round nib: **318/333 recovered, all curved**, 315 within 2 % of Minkowski, total points 111 869 vs 112 170 ring points (no growth);
- `Nib::Rect`: 1529 failures → 1480 recovered (1333 curved); `Calligraphic`: 1632 → 1569 (1413 curved);
- `remove_overlap_with_fallback` then mops up the rest: 1529/1529 and 1632/1632.
- Caveat, must not become the general `remove_overlap` primitive: it is even-odd for overlapping same-winding subpaths (two overlapping rects → 15000 instead of 17500) — the existing oracle rejects exactly those cases, which is why 15/333 still error.

**Recommendation.** (a) In `round_stroke`/`convex_nib_stroke` (`stroke.rs:162`, `:290`) union the rings with the hole-preserving primitive, validated by the current oracle; (b) surface the fallback additively, mirroring `RemoveOverlapOutcome`:

```rust
pub struct StrokeOutcome { pub contours: Vec<Contour>, pub fallback: FallbackKind }
pub fn expand_stroke_with_fallback(contours: &[Contour], opts: &StrokeOptions, kind: CurveKind)
    -> Result<StrokeOutcome, Error>;   // expand_stroke stays strict
```

(c) Fix `fit_polygon_to_cubics` to fit *runs* of edges (one `ParamCurveFit` over the polyline, corners via `break_cusp`) instead of one cubic per edge, so `FallbackKind::Flattened` output is editable geometry.

## New Breakage in the Fix Diff

1. **Important — convex-nib closed strokes lose ~1000 glyphs of success rate.** Full corpus (3583, width 20, Round cap/join, both flags false), f03285d → 10f1f1f: `Nib::Rect` **3073 → 2054 ok** (510 → 1529 failures; 206 fixed, **1225 newly failing**, including `A`, `B`, `D`, `O`, `.notdef`); `Calligraphic` **3015 → 1951 ok** (curved 6 → 1950). Round nib is net positive (3174 → 3250) but still churns: 290 fixed, **214 newly failing** (`s`, `question`, `infinity`, `trademark`, `hookabovecomb`). Cause: previously `normalize_self_intersecting_contours` flattened rings into an i_overlay-normalized set that sailed through the fast path; curve-preserving rings now reach flo and hit the hole-dropping primitive (`stroke.rs:284`, `:290`). Not disclosed in the report, which cites only the round-nib 409 → 333. The remedy above fixes it.
2. **Important — the new fallback's re-fit is per-edge** (`boolean.rs:785-806`): output is the flattened polygon with one cubic per edge (measured 3457 / 6000 / 3633 points). The API contract ("succeeds + flag") is met, but as the sanctioned remedy for genuine failures it produces unusable font geometry.
3. **Minor — cleanup pre-passes no longer run on the fast path.** Dropping the old `same_path_segments(clean, original)` condition (`boolean.rs:66`) plus the adjacent-edge exemption (`boolean.rs:952`) means a rectangle with a zero-width outward spike or a duplicated point is now returned **unchanged** (probe: spike survives, area 10000, `unchanged=true`); pre-fix it was cleaned by `clean_path`.
4. **Minor — the `uni2727` regression test is vacuous** (`tests/boolean.rs:213`): the glyph now takes the fast path, so it no longer covers the oracle-ordering fix it was added for.

## Verdict

**Findings remain open** — all four review findings are addressed with the demanded probes reproducing, but the fix introduces Important new breakage that should be resolved before sign-off:

- convex-nib closed strokes drop from 3073/3015 to 2054/1951 successes out of 3583 (1225 and ~1250 previously-working glyphs now `Err`), and 214 round-nib glyphs newly fail;
- `remove_overlap_with_fallback`'s per-edge re-fit returns 3400–6000-point contours where ~110–150 suffice.

Both are fixed by the same change: union stroke rings with flo's hole-preserving `path_remove_overlapped_points` under the existing oracle (318/333, 1480/1529, 1569/1632 recovered, curved and compact), route the residue through the fallback, and expose the flag via an additive `expand_stroke_with_fallback` → `StrokeOutcome { contours, fallback }`.