# P5-11 variable pen kernel acceptance

Implemented 2026-09-30. Scope: normalized pen-point model, geometry, native
history/persistence and shared effect/export evaluation. UI acceptance is owned
separately. No new dependency or FontForge source was used.

## Behavior

`VariableStroke` supports ellipse/rectangle nibs, butt/round/square caps,
bevel/round/limited-miter joins, and linear/smoothstep progression. Each contour
uses its own normalized arc length. Dimensions and rotation interpolate between
2–64 ordered points; rotation follows the shortest angular interval. Closed
contours require matching endpoint nibs. Source edits resample preserved points.
Native sources and stacks remain reversible; baking/export use the existing
shared evaluator. Undo memory accounting includes pen vectors.

Geometry is sampled and polygon-refitted, and always reports approximation.
Limits are 8192 centerline/interpolation samples per contour, 200000 patch
vertices per effect, and existing 4096-input/65535-output path limits. Invalid
closed seams, unrepresentable point spacing, excessive work and font-grid
overflow fail explicitly. Global variable export still requires compatible
evaluated master topology. Other editable source formats require explicit bake.

## Verification

- `cargo test -p fc-model --test variable_pen`: **PASS**, 2 tests, 0 failed.
- Narrow `rustfmt` on the new implementation/tests and model effects: **PASS**.
- `git diff --check` on changed existing kernel/schema/doc files: **PASS**.
- Added nine geometry tests and six document/compiler tests. They cover tapers,
  progression, rotated nibs, caps/joins, closed counters, thick fills, reversals,
  degenerate spacing, source/history/native preservation and TTF/OTF geometry.
- Independent review identified reversal joins, rectangle square-cap drift and
  zero-length generated spans; each was fixed and received a regression.
- Initial focused geometry and document/compiler commands were **interrupted**
  at the root's request to release the shared Cargo queue. The later root suite
  (`.scratch/p5-resume/rust-tests.log`) passed all 2 model, 4 document and 2
  compiler variable-pen tests. Geometry passed 8 of 9; its round-cap bounds test
  exposed severe overshoot in the polygon-to-cubic fitter.
- Necessary regression fix: compare every fitted contour against its source
  polygon using bounds (0.05 units) and filled symmetric-difference area
  (0.05 × perimeter). Keep the sampled polygon boundary if the fit distorts it.
  Analytic cap/join assertions remain unchanged. Focused rerun:
  `cargo test -p fc-geom --test variable_pen`: **PASS**, 9 passed, 0 failed,
  finished in 0.33 seconds.

The initial geometry attempt also encountered temporary concurrent model edits
(`features/preprocess.rs` and `Layer.metrics_keys`); those owners completed them.
Initial artifact checkpoint: PASS, build 6.10 GiB, free 668.74 GiB. After the
shared full regression build, artifacts were 17.54 GiB (warning, below the
20 GiB ceiling), with 657.40 GiB free. Shared-cache cleanup belongs to root.
Own scratch was removed after recording focused results; no commits were made.
