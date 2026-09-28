# P5 static COLRv1 acceptance

Verified on 2026-09-28 on `p5-color-authoring`; no push or main merge. This closes
the static paint-graph block, not the remaining P5 checklist.

## Implemented

The existing atomic COLR/CPAL model now carries named v1 roots, shared paint
nodes and clip boxes beside v0 fallback layers. All 18 static paint formats
are read; compact scale/rotate/skew variants normalize to equivalent exact
parameters. Export writes shared forward offsets, preserves stop order and
foreground/alpha semantics, and remaps names to compiled glyph IDs. Variable
paint/clip formats and variation stores remain raw as a whole, including CPAL.

Validation covers palette/glyph/node references, cycles through explicit and
named edges, finite bounded roots, depth 64, stored-item and root-evaluation
budgets, representable numbers and output size. Rename/duplicate/delete and
undo maintain graph references. Missing export dependencies drop complete
roots with the existing color-reference warning. Undo memory accounting includes
paint nodes, stops, clips and names.

The color dialog has a graph source editor and an initial gradient action.
Malformed source stays visible through attempted close/save handovers; corrected
source applies atomically. Palette deletion recognizes all graph color uses
and renumbers higher solid/stop indices. Draft preview builds a bounded isolated
font from current default-master outlines and the selected palette, then uses a
COLRv1-capable browser. A missing renderer produces an error, not a successful
monochrome preview. [Usage and limits](user/colrv1.md).

## Independent verification

`scripts/p5_colrv1_acceptance.py` uses FontTools 4.63.0 to build original geometric
artwork with all static formats, all 28 composite modes, unsorted/coincident
stops, shared subgraphs, named roots, clips, CPAL v1 labels and a v0 fallback.
FontTools independently decoded eight baseline/edited TTF, OTF, WOFF and WOFF2
exports and compared normalized paint semantics, palettes, clips and fallbacks.
Native `.fcf`/`.fcf.gz` and UFO reopened with identical editable data. Undo/redo
restored whole graphs; temporary preview did not mutate state.

The script emits `render-check.js`. Running it in isolated headless Chromium
compared original and independently edited FontTools fonts against each of the
eight exports: **0 differing RGBA channels**, with 39,424 nontransparent pixels
per comparison. This checks the synthetic sample in Chromium, not every renderer
or every possible paint arrangement.

The pixel check initially exposed an existing TrueType origin defect: import
used raw glyf X coordinates while discarding the hmtx side bearing when it
differed from xMin. Import now normalizes the phantom-point origin and compensates
component transforms. Preserved raw outlines retain their original head origin
flag. Regression tests cover unequal side bearings, a transformed component,
empty glyphs and legacy header-only glyphs. No fixture was changed to hide the
rendering discrepancy. The previously failing header-only import test passes
in the complete sfnt rerun.

## Browser and regression checks

Headless browser checks exercised graph source replacement, invalid JSON blocking
Close, correction/Apply, native save/reopen and actual TTF export. FontTools
independently confirmed the browser-authored gradient and named reference.
English/light and Chinese/dark views were inspected:
[English](screenshots/p5-colrv1-en-light.png),
[Chinese](screenshots/p5-colrv1-zh-dark.png). Targeted WCAG 2 A/AA scans of the
whole graph section reported **0 violations, 0 incomplete checks** in both.
Whole-dialog scans had no violations; contrast checks on partially scrolled
palette/layer content were inconclusive. This is not a whole-app audit.

- Regression coverage over fc-model, fc-sfnt, fc-doc, fc-formats, fc-compile and
  fc-api: **935 passed, 0 unresolved failures, 2 pre-existing ignored tests**.
  This combines the five completed crate suites with a complete fc-sfnt rerun
  after fixing the empty-glyph regression. Optional external corpora retain
  their availability behavior; native platform testing is not implied.
- Codec tests include all static families/composites, truncations, malformed
  offsets, cycles, unbounded roots, empty LayerList handling, dependency deletion
  and 20,000 shared roots within a 10-second debug budget.
- Frontend: **143 files / 1,153 tests passed**; TypeScript, lint and production
  build passed. The existing large-bundle warning remains.
- Affected-crate Clippy with `-D warnings`, Rust formatting, changed frontend
  Prettier and diff whitespace checks passed.

No new dependency, third-party artwork or FontForge implementation was added.
Variable paint authoring, native renderer/platform acceptance and color rendering
in the grid/text tab remain outside this block. Chinese strings were added and
catalog parity checked here; independent translation review remains outstanding.
