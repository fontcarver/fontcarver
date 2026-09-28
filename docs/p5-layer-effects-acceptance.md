# P5 layer effects acceptance

Verified on 2026-09-28 within the boundaries below. This completes the stored
boolean-mask workflow in audit row 88 and the uniform-pen part of row 87.
Variable pen points and the rest of P5 remain open.

## Delivered behavior

- An ordered native layer stack supports elliptical/rectangular strokes,
  overlap removal and union/intersection/subtraction/XOR with a stored SVG mask.
  Each entry can be disabled, reordered or removed. The actual Layers-panel
  editor exposes parameters, mask capture, source/result preview, apply and bake.
- Preview and application share geometry and a captured document revision.
  Applying retains the original contours, anchors, components and advance;
  baking replaces the drawing and removes the stack in one undo transaction.
  Invalid source edits beneath an enabled effect are rejected without changing
  document state. Local and dependent caches and metrics are invalidated.
- The model stores and validates effects; `fc-geom` evaluates them through an
  explicit recursive model-query filter. Component effects run before their
  placement transform. Editor, grid, text, raster, color-preview and export
  consumers use the evaluated queries. Source nodes stay available for editing.
- FCF, compressed FCF and recovery preserve the original source and stack.
  TTF/OTF/WOFF/WOFF2 bake an export copy and report the operation. Other editable
  source formats reject stacks before writing until they gain an interoperable
  representation. No source format silently drops effects.
- Local smart sources interpolate before their shared stack and require identical
  stack definitions. Global interpolation uses evaluated master outlines and
  requires compatible resulting topology. Authored TT point bindings require
  baking and review; stale point-bound hints are cleared on changed export copies.

## Independent geometry and persistence checks

`scripts/p5_layer_effects_acceptance.py` passed **60 exports and 128,640 sampled
containment comparisons** using FontTools 4.63.0. Fifteen synthetic cases cover
rotated elliptical pens, all cap styles, rotated rectangular pens, all four mask
operations and wide closed strokes with all three join styles. Each case exports
TTF, OTF, WOFF and WOFF2; a transformed component must match the base result.
Independent expectations use analytic pen-coordinate distance, convex hulls and
set operations, rather than the implementation's geometry library.

The same script verifies unchanged native source coordinates, retained component
references, effect parameters/order, native reopen, undo/redo and bake/undo.
Advance widths remain unchanged. Samples within 1.5 units of an expected edge
are excluded to allow integer export rounding and curve approximation.

`scripts/p5_layer_effects_real_fonts.py` passed **8 exports and 29,438 samples**
on committed DejaVu Sans and Noto Sans fixtures. A/B/O/S/g/a/e and a separately
evaluated Aacute receive a 20-unit circular stroke. Expected ink is the 10-unit
neighborhood of independently sampled original outlines. TTF and OTF preserve
advances, and native documents retain source contours, anchors and components.
Per-glyph draft previews took **0.021–0.049 seconds** in this run. These are
observations on this host, not a whole-font interactive performance guarantee.

These real-font checks intentionally disable original-table preservation:
DejaVu includes point-index-bound layout anchors that cannot safely survive
outline expansion. The default export correctly rejects that combination; the
acceptance does not claim preserved shaping for these geometry-only outputs.

The real-font check exposed a triangular hole at a Noto B inside join. Stroke
normalization had classified a doubly covered region as a nested hole. Each
span now resolves its original nonzero fill before orientation normalization
and union. A focused regression covers open and closed corners; the earlier
wide-pen/small-counter regression and all independent cases still pass.

Reproduce against an isolated development server:

```sh
python3 scripts/p5_layer_effects_acceptance.py --output /tmp/effects-synthetic
python3 scripts/p5_layer_effects_real_fonts.py --output /tmp/effects-real
```

Reports from this run are in `/tmp/fontcarver-effects-qa/final/report.json` and
`/tmp/fontcarver-effects-qa/real-fonts-final/real-font-results.json`.

## Browser and frontend checks

An isolated agent-browser session exercised English/light and Chinese/dark
dialogs. It changed pen dimensions/angle/type, captured another glyph as a mask,
previewed/applied/saved, disabled and reordered effects, confirmed preview
invalidation, baked, undid and saved again. Direct inspection of the saved FCF
confirmed two original source points, both original effects and the surviving
dependent component after bake/undo.

- [English/light screenshot](screenshots/p5-layer-effects-en-light.png)
- [Chinese/dark screenshot](screenshots/p5-layer-effects-zh-dark.png)

Axe 3.5.6 reports **zero violations and zero incomplete checks** for each dialog
when the complete scroll body is visible at 1440×1800. Smaller scrollable views
were also inspected visually. That older axe version misclassified clipped
offscreen text against the dimmed page in earlier partial-scroll scans; those
scans are retained rather than counted as clean full-dialog audits.

Frontend regression passed **1,158 tests in 145 files**. After final control
layout and component extraction, the affected **18 tests** passed again, as did
TypeScript, ESLint and the production build. The existing Vite large-chunk
advisory remains. English/Chinese leaf keys and placeholders match (**1,831
keys**). Chinese was checked in the browser by the implementer; independent
translation review remains outstanding and is not claimed.

## Rust and final checks

The final seven affected crates passed **947 tests**, with **two pre-existing
ignored acceptance tests**, in a serial run. The later disabled-effect regression
passed separately, followed by the six geometry and four API effect tests. This
adds one distinct passing regression (948 in total), not a second full-suite run.
The disabled-only export now clears metadata on the copy without claiming a
geometry edit or overriding the existing preservation policy. The regression
compares `glyf`, `loca`, `maxp`, `fpgm`, `prep`, `cvt ` and GPOS byte for byte.

Logs are `/tmp/fontcarver-effects-rust-final.log`,
`/tmp/fontcarver-effects-disabled-regression.log`,
`/tmp/fontcarver-effects-geom-final.log` and
`/tmp/fontcarver-effects-api-final.log`. Workspace all-target Clippy with warnings
denied, Rust formatting and whitespace checks also pass.

An earlier parallel run exceeded a CJK timing threshold under concurrent load;
the isolated CJK run and serial regression passed without relaxing the threshold.
Native/global desktop input automation and packaged-platform acceptance were
not performed.

## Explicit boundaries

- Strokes have one pen size/angle per operation; variable pen points are pending.
- Masks are stored copies, with no live link to the captured glyph or layer.
- Source-node dragging displays the last committed effect result until commit.
- Geometry limits are documented in [Layer effects](user/layer-effects.md).
  Curve-refitting fallback is visible in previews and export reports.
- Editable source-format interoperability and native-platform rendering are not
  established by the compiled-font and headless-browser checks above.
