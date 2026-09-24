# P1 Task 2 — minor round report (fw-doc outline command hardening)

Worktree `/home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t2m`, branch `p1-t2m`
(fe095c8 → 1dcf2e1). Nothing pushed. Only `crates/fw-doc` and one additive query in
`crates/fw-model` were touched.

## Commits

| Commit | Subject |
| --- | --- |
| b57a311 | `feat(fw-model): add a declared_curve_format query` |
| aa2462b | `fix(fw-doc): derive the outline curve kind from the font, not the layer` |
| 9b45b90 | `fix(fw-doc): validate only touched contours and restore the whole layer` |
| 7f56a14 | `perf(fw-doc): capture only what an outline command can overwrite` |
| 5dc384c | `test(fw-doc): measure corpus deletion deviation in both directions` |
| 84bfe37 | `docs(fw-doc): record that malformed contours surface as InvalidCommand` |
| 1dcf2e1 | `docs(fw-doc): describe the outline restore capture precisely` |

Each behavioural change was written test-first: the new test was run and observed to fail
before the fix landed (item 3: `assertion left == right failed: one control point is
inserted, left: 6, right: 5`; items 1–2: `a_pre_existing_malformed_contour_…` panicked at the
`expect("an untouched malformed contour must not block the edit")` and
`a_rejected_transform_…` at `assertion failed: components and anchors are restored with the
contours`).

## Item 1 — touched-contour validation

Every outline operation now reports what it wrote. `Touched`
(`crates/fw-doc/src/command/outline.rs:21`, constructors at `:28`) is either a set of contour
indexes *in the resulting contour list* or `Touched::Layer` for an operation that rebuilds the
list. `dispatch` returns it (`outline.rs:224`) and `validate` (`outline.rs:174`) runs
`try_to_bezpath` only over those contours, so a pre-existing malformed contour elsewhere in the
layer no longer blocks unrelated edits.

Per operation: `move_points`/`set_smooth` → the contours of the listed points
(`outline/points.rs:57`, `:103`); `set_position` → one contour (`points.rs:75`);
`transform_selection` → `Touched::Layer` for the empty-selection branch (`points.rs:165`), the
selected contours otherwise (`points.rs:183`); `add_contour` → the appended index
(`outline/segments.rs:17`); `delete_contours` → nothing, removal cannot malform a survivor
(`segments.rs:29`); `reverse_contours` → the listed indexes (`segments.rs:42`);
`insert`/`convert` → the named contour (`segments.rs:57`, `:90`, with the no-op early returns
reporting nothing); `set_start` (`segments.rs:185`) and `join` (`segments.rs:204`) → the one
contour they rewrite; `split` → the contour and, when an open contour is split in two, its new
neighbour (`segments.rs:253`); `path_op` → `Touched::Layer` (`segments.rs:298`).

`delete_points` (`outline/delete.rs:29`) was restructured to rebuild the contour list once:
contours without a deleted point move across untouched (`delete.rs:46`) and each rewritten,
removed or broken-up contour records its resulting index (`delete.rs:60`). `break_at_points`
(`delete.rs:426`) is now a pure `&Contour → Vec<Contour>` function instead of splicing into the
layer, which is what makes the resulting indexes knowable.

Covering tests: `crates/fw-doc/tests/outline_commands.rs:849`
`a_pre_existing_malformed_contour_does_not_block_edits_on_other_contours` — a layer holding a
raw malformed `Contour` literal (`outline_commands.rs:821`) still accepts `MovePoints` on
contour 0 and still rejects `AddContour` of a malformed contour with `InvalidCommand` and an
unchanged glyph. The pre-existing `degenerate_contours_are_rejected_instead_of_panicking` and
`a_malformed_contour_is_rejected_and_leaves_the_layer_unchanged` still pass.

## Item 2 — whole-layer restore on failure

`Restore` (`outline.rs:46`) captures before dispatch and `apply_to` (`outline.rs:104`) puts the
capture back on any failure (`outline.rs:157`). `TransformSelection` therefore restores the
component transforms and anchor positions it moved, not only the contours: with an explicit
selection the listed components and anchors are captured alongside the listed contours
(`outline.rs:79`), and the whole-layer branch captures the whole `Layer` (`outline.rs:87`).

Covering test: `outline_commands.rs:903` `a_rejected_transform_restores_components_and_anchors_too`
— a layer with a malformed contour, one component and one anchor; a selection-based
`TransformSelection` naming the malformed contour, component 0 and anchor 0 returns
`InvalidCommand` and leaves the glyph byte-identical, and so does the whole-layer variant with
all three lists empty.

## Item 3 — `inferred_kind` from the font

New additive query `fw_model::query::declared_curve_format(font, master)`
(`crates/fw-model/src/query/curves.rs:35`, returning `CurveFormat` at `curves.rs:14`). The rule,
documented on the function: the imported source's table list decides first (`glyf` → quadratic,
`CFF `/`CFF2` → cubic); otherwise TrueType master data — cvt, fpgm, prep or any `maxp` maximum
(`curves.rs:53`) — on the named master, or on any master when none is named, means quadratic;
otherwise `None`. The model carries no explicit outline-format flag, so those are the two
signals an import actually preserves.

`fw-doc` consumes it through `layer_curve_kind` (`outline.rs:325`), which resolves the layer's
master from its `LayerKind` (`outline.rs:338`) and falls back to the old contour heuristic
(`inferred_kind`, `outline.rs:350`) only when the font declares nothing. All four former
`inferred_kind` call sites now route through it: `convert` (`segments.rs:98`), `path_op`
(`segments.rs:304`, feeding `RemoveOverlap` and `ExpandStroke`) and `delete_points`
(`delete.rs:36`). `InsertPointOnSegment` needs no change — `fw_geom::split_segment` subdivides
the existing segment and never invents a handle kind.

Covering tests: `outline_commands.rs:751`
`a_quadratic_font_gets_quadratic_handles_on_a_straight_glyph` — a font whose provenance lists
`glyf` over an all-`Line` glyph gets one `OffCurve` at the chord midpoint (50, 0) and a `QCurve`
endpoint from `ConvertSegment { to: Curve }`, with no `PointKind::Curve` anywhere;
`outline_commands.rs:794` `a_font_without_a_declaration_still_infers_the_kind_from_the_outlines`
pins the fallback (two handles, `PointKind::Curve`). Unit tests for the query itself:
`crates/fw-model/src/query/curves.rs:81-135`, four cases (no declaration, table tags both ways,
master TrueType data, per-master resolution).

## Item 4 — cheaper preview path

`Restore::capture` (`outline.rs:65`) no longer clones the layer for commands that only rewrite
existing points in place. `MovePoints`, `SetSmooth`, `SetPointPosition`, `SetStartPoint`,
`InsertPointOnSegment`, `ConvertSegment`, `ReverseContours` and selection-based
`TransformSelection` capture exactly the contours (and, for the transform, the components and
anchors) they name, via `pick` (`outline.rs:121`); everything that adds, removes or rebuilds
contours still captures the whole `Layer`. That removes one layer clone per command, including
the layer's background image bytes, hints and colour layers. It is not the whole cost of an
edit — `Document::apply_command` still snapshots the glyph for undo around every command,
preview frames included — so no claim is made about the asymptotic cost of a preview frame
(corrected in round 2, Minor 2). Semantics are unchanged: the narrow capture is only used where
contour, component and anchor indexes are stable across the operation, which is asserted in the
`Restore::Parts` doc comment (`outline.rs:46`) and covered by the two restore tests above (the
selection-based transform exercises the narrow path, the empty-selection transform the
whole-layer path). Item 4 has no performance test; the dispatch permitted that ("no benchmark
required"). The 1 ms/step budget test (`crates/fw-doc/tests/commands_undo.rs:543`) still passes,
though it drives `ReplaceLayerOutline` and so confirms the absence of a regression elsewhere
rather than exercising the changed capture.

## Item 5 — corpus sweep two-way deviation

`crates/fw-doc/tests/delete_points.rs` now measures both directions and gates on the larger.
`joined_span` (`delete_points.rs:330`) locates the segments the deletion produced between the two
surviving neighbours — both survive unchanged, so they are found by coordinate — and
`one_way_deviation` (`delete_points.rs:352`) samples those and measures to the original span
(`delete_points.rs:468`). The sweep asserts the joined span was located for all 88 deletions.
Thresholds are unchanged (median ≤ 2 %, fitted p90 ≤ 15 %, worst ≤ 50 %).

New numbers (`cargo test -p fw-doc --test delete_points -- --nocapture`):

```
corpus deletions: 88 (75 fitted, 13 straight)
fitted joins: median 0.893 %, p90 7.049 %, max 36.224 % of the joined chord
line-to-line joins: median 31.802 %, max 46.174 % of the joined chord
all joins: median 0.999 %, max 46.174 % of the joined chord
per direction: original samples max 46.174 %, new-span samples max 33.931 %
```

Previous (one-way) numbers were fitted median 0.891 %, p90 7.049 %, max 36.224 %; line-to-line
median 31.802 %, max 46.174 %; overall median 0.998 %, max 46.174 %. The two-way maxima are
identical to the one-way ones — the reverse direction peaks at 33.931 %, below the forward
46.174 % — so the shipped fitter has no bulge the old measurement was missing; only the two
medians moved (0.891 → 0.893 %, 0.998 → 0.999 %).

## Item 6 — `AddContour` error class

`AddContour` keeps reporting a malformed contour as `Error::InvalidCommand` through the shared
validator; the variant now documents it (`crates/fw-doc/src/error.rs:33-41`): "Malformed
contours surface here too … reported as an invalid command rather than as `Error::Model`,
because one shared validator classifies every such result."

## Verification

```
$ cargo fmt --all --check
fmt: clean

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 05s
(no warnings)

$ cargo test -p fw-doc -p fw-model -p fw-geom
124 passed; 0 failed; 0 ignored (20 test binaries, all "test result: ok")

$ cargo test --workspace
353 passed; 0 failed; 0 ignored (67 test binaries, all "test result: ok")
```

## Notes

- `CurveFormat` deliberately carries no `serde`/`ts-rs` derives, unlike `query::Issue` and
  `query::Incompatibility`: it is an internal derivation helper, not part of any document or
  API payload, and exporting it would add an orphan generated TypeScript type to a surface
  other agents own this round.
- fw-doc's public API is unchanged (`Touched` and `Restore` are private); fw-model gained only
  the additive `query::declared_curve_format` / `query::CurveFormat`.


---

# Round 2 — the three Minor items from `task-2-minor-review.md`

Same worktree and branch, 1dcf2e1 → 1a955b0. Each behavioural change was again written
test-first and observed to fail before the fix.

## Commits

| Commit | Subject |
| --- | --- |
| c48b62c | `fix(fw-doc): let uniformly contradicting outlines outrank the declaration` |
| 5df98c3 | `fix(fw-doc): rewrite deleted contours before touching the layer` |
| 1a955b0 | `docs(fw-doc): drop the preview-cost claim from the outline capture` |

## Minor 1 — `delete_points` is exception-safe on its own

`delete_points` (`crates/fw-doc/src/command/outline/delete.rs:30`) now runs in two phases: every
contour that loses points is rewritten first into a `BTreeMap<usize, Vec<Contour>>`
(`delete.rs:47`), so any `Error::InvalidCommand` returns before `target.contours` is touched at
all; only then is the list moved out and rebuilt, and nothing in that second pass can fail
(`delete.rs:62`, with the invariant stated in the comment and in the function's doc comment).
The function no longer depends on `outline::apply`'s whole-layer capture for atomicity.

Covering test: `crates/fw-doc/src/command/outline/delete.rs:651`
`a_failed_deletion_leaves_the_contour_list_untouched`, a unit test that calls `delete_points`
directly — bypassing `outline::apply` and its restore — on a layer holding a good square and a
raw contour whose leading handles have no preceding node (`delete.rs:605`). Deleting one of
those handles returns `InvalidCommand` and the glyph compares equal to its pre-command clone.
Observed failing before the fix: `assertion left == right failed: a rejected deletion must not
consume the contour list`.

## Minor 2 — no preview-cost claim

`outline::apply`'s doc comment (`crates/fw-doc/src/command/outline.rs:139-152`) now says only
that a point-rewriting command "captures exactly the parts it names" and adds the counterweight:
"This capture is not the whole cost of an edit: `Document::apply_command` snapshots the glyph for
undo around every command, preview frames included." Item 4 above was reworded to match.

Also added, per the review's out-of-scope note: a comment in the corpus sweep
(`crates/fw-doc/tests/delete_points.rs:451`) recording that the two sampling directions are
deliberately asymmetric — forward measures the original span against the whole rewritten
contour, backward measures the joined span against only the two original segments it replaced,
which is the stricter of the two — and that the gate takes whichever is larger.

## Minor 3 — the declaration yields to outlines that uniformly contradict it

`layer_curve_kind` (`crates/fw-doc/src/command/outline.rs:341`) now consults
`layer_curves` (`outline.rs:362`, returning `LayerCurves` at `:319`: `Ambiguous`, `Quadratic`,
`Cubic` or `Mixed`, read from the layer's stored point kinds) before trusting the declaration:

- no declaration → the old `inferred_kind` heuristic, unchanged;
- declaration Quadratic + layer uniformly `Cubic` → cubic (the layer wins);
- declaration Cubic + layer uniformly `Quadratic` → quadratic (the layer wins);
- every other combination — ambiguous, agreeing or mixed — follows the declaration.

`fw_model::query::declared_curve_format` is unchanged. This makes the import declaration yield
to a user's `PathOp::ConvertCurves` while still deciding for a layer whose own outlines say
nothing, which is the case the declaration was added for.

Covering tests: `crates/fw-doc/tests/outline_commands.rs:972`
`a_layer_converted_to_cubic_outranks_the_import_declaration` — a `glyf`-declared font whose
quadratic triangle is converted with `PathOp::ConvertCurves(CurveKind::Cubic)`, then a segment
converted to a line and back to a curve, receives two `OffCurve` handles and a `PointKind::Curve`
endpoint with no `QCurve` left (observed failing before the fix on `!contour.is_quadratic()`);
`outline_commands.rs:1036` `cubic_outlines_without_a_declaration_stay_cubic` — a layer holding a
cubic contour in a font that declares nothing still gets two cubic handles; and the round-1
`outline_commands.rs:751` `a_quadratic_font_gets_quadratic_handles_on_a_straight_glyph` still
passes, pinning the all-line (ambiguous) glyph in a `glyf` font to a single `QCurve` control.

## Verification

```
$ cargo fmt --all --check
fmt: clean

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.04s
(no warnings)

$ cargo test -p fw-doc -p fw-model
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.34s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.13s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
(104 passed, 0 failed, 0 ignored)

$ cargo test --workspace          # run as well, since Minor 3 changes behaviour
356 passed; 0 failed; 0 ignored (67 test binaries, all "test result: ok")
```

Corpus numbers are unchanged by round 2 — the sweep builds its font with `test_font()`, which
declares no curve format and so takes the `inferred_kind` fallback:

```
corpus deletions: 88 (75 fitted, 13 straight)
fitted joins: median 0.893 %, p90 7.049 %, max 36.224 % of the joined chord
line-to-line joins: median 31.802 %, max 46.174 % of the joined chord
all joins: median 0.999 %, max 46.174 % of the joined chord
per direction: original samples max 46.174 %, new-span samples max 33.931 %
```

## Notes

- `layer_curves` reads stored `PointKind`s, the same signal as `Contour::is_quadratic`, so a
  `QCurve` or `Curve` endpoint left with no control points (which `repair_kinds` normally
  prevents) counts as a curve of its family. That matches the pre-existing heuristic.
- The residual case the ruling leaves standing: a layer mixing quadratic and cubic curves in an
  imported font follows the declaration, so new handles there match the import rather than the
  neighbouring segment. That is the dispatch's ruling, recorded here rather than re-litigated.
