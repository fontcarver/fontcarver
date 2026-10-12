# P5-10 smart corners acceptance

Verified on 2026-09-28 within the boundaries below. The numeric live-corner
workflow in audit row 84 is complete. Remaining construction and interaction
work stays in the P5 backlog. Work pauses after this milestone at the user's
explicit request.

## Delivered behavior

`Point.corner_radius` stores source-node intent with a backward-compatible zero
default. Positive radii create tangent circular fillets across adjacent lines,
quadratics and cubics; negative values cut V-shaped ink traps into the layer's
nonzero fill, including counter boundaries. Source nodes and handles remain
editable and carry their radius through movement, insertion, reversal, deletion,
undo and native persistence. The Info box edits one or many nodes atomically,
marks mixed values, removes corners and links to the existing bake workflow.

Corners evaluate before ordered layer effects and component placement. Preview,
component dependents, local smart sources, raster paths, batch styles and compiled
exports use the shared evaluated geometry. Local smart interpolation blends
signed radii through zero, including non-default and nested sources. Global
interpolation rejects incompatible evaluated topology. Source writers reject
unbaked corners before touching destination files. Export copies and explicit
bakes clear obsolete point-bound hints under the existing layer-effect policy.

The native schema and [user guide](user/smart-corners.md) specify radius and work
budgets, the sub-resolution threshold, transform behavior and failure boundaries.
Invalid or colliding edits leave both the font and undo history unchanged.

## Independent output checks

- [Synthetic report](acceptance/p5-smart-corners/synthetic.json): **24 exports,
  251,008 containment samples** across TTF, OTF, WOFF and WOFF2. Analytic circle and
  polygon expectations cover both contour directions, four rounded corners,
  concave rounding, four ink traps, a counter trap and corner-before-mask order.
  A translated component must match each base result. Source points/radii,
  component references, advances, FCF reopen, undo/redo and bake/undo are checked.
- [Real-font report](acceptance/p5-smart-corners/real-fonts.json): **8 exports,
  118,460 samples** on DejaVu Sans and Noto Sans A/H/N. Positive and negative
  20-unit constructions are compared against FontTools's original fill plus
  independently calculated circular or polygonal cuts. Dense local sampling
  makes the small corner changes observable. Native source coordinates and
  advance widths survive. Edits took 0.001–0.032 seconds on this host.

FontTools 4.63.0 is the independent reader. Samples within 1.5 units of expected
edges are excluded for integer rounding and curve approximation. The real-font
geometric checks disable imported-table preservation: DejaVu's point-index GPOS
anchors cannot be safely retained across changed point topology. This does not
claim preserved shaping in those geometry-only outputs.

Reproduce against an isolated development server, then retain reports and remove
the task's generated fonts:

```sh
python3 scripts/p5_smart_corners_acceptance.py --output .scratch/p5-10/synthetic
python3 scripts/p5_smart_corners_real_fonts.py --output .scratch/p5-10/real-fonts
python3 scripts/check_artifacts.py
```

## Editor acceptance

An isolated headless agent-browser session exercised single-node trap input,
multi-node radius input, mixed-value unification, invalid radius rejection,
remove/undo, save-as, FCF and compressed FCF reopen. Baking four live corners
produced 37 ordinary quadratic points; one undo restored the four original nodes
with radius 80. Chinese compressed-native reopen retained radii 80/80/-60/80.
The live-corner notice in the layer-effect dialog makes baking available even
when the ordered effect stack is empty.

- [English/light](screenshots/p5-smart-corners-en-light.png)
- [Chinese/dark](screenshots/p5-smart-corners-zh-dark.png)

Browser testing found that an unsuccessful radius commit retained the typed
value in the field. A model-reconciliation token now resets that field after a
rejection even when the model value is unchanged; its regression also checks
retrying the same rejected value. The dark-mode on-curve badge had 3.56:1 contrast;
it now uses the normal text token, as does the selected point-type control.

Axe 4.12.1 reports **zero violations** for both final Info-box scopes, with **one
incomplete contrast rule each**. Both include the existing non-text collapse
arrow; the English mixed-selection scan also could not determine the background
of the partially obscured help paragraph. The arrow was visually reviewed; its
computed foreground/background contrast is
approximately 6.6:1 in dark mode and 6.6:1 in light mode. These incomplete checks
are retained in the [English](acceptance/p5-smart-corners/axe-en.json) and
[Chinese](acceptance/p5-smart-corners/axe-zh.json) reports. They are not reported
as automatic passes or whole-application accessibility certification.

The frontend suite passed **1,160 tests in 145 files**. The added rejected-field
regression and final affected checks passed (**19 tests**, **1,161 distinct
frontend tests** across these runs). TypeScript, ESLint, the final production
build and changed-file formatting passed. The existing Vite large-chunk advisory
remains. English/Chinese keys and placeholders match (**1,841 leaves**).
Independent translation review remains outstanding; the implementer performed
the bilingual browser check.

## Rust verification and storage

Eight affected crates completed **1,007 passing tests** before the full run was
interrupted to handle excessive historical build-cache use. The remaining seven
sfnt test binaries subsequently passed **40 tests**. Two additional regressions
(global corner topology and live corners in batch styles/source-format guards)
passed with their focused suites. This yields **1,049 distinct passing tests**,
with two pre-existing ignored release acceptance tests, across the interrupted,
resumed and focused runs. It is not described as one uninterrupted full-suite run.
All eight crates' doc-test checks completed; they contain no runnable doc tests.
The final geometry and document smart-corner tests passed again. Counts and
outcomes are retained in the [check summary](acceptance/p5-smart-corners/checks.json).

Workspace all-target Clippy, including the desktop shell, passed with warnings
denied. Rust formatting and diff checks passed. No native/global desktop input
automation or packaged-platform acceptance was performed.

[Resource maintenance](build-artifacts.md) cleared approximately 491 GiB of
historical development artifacts and introduced storage checkpoints. Rebuilt
artifacts finished at 5.23 GiB (12,405 files), below the 20 GiB ceiling. The whole
workspace occupies approximately 5.8 GiB, with 692.25 GiB free on its filesystem.
Final reports and screenshots are retained here; generated fonts, obsolete
screenshots, temporary logs and the isolated task services are cleaned/stopped
at the pause boundary.

## Explicit boundaries

- Radius editing is numeric; dedicated radius drag handles and construction at
  crossings without an existing node remain in the interaction backlog.
- Oversized radii and neighboring trim collisions fail instead of being clamped.
  Curve joins require a bounded tangent solution; there is no global collision
  solver for remote contours or complex self-intersections.
- Negative magnitude sets the V mouth's arc-length setbacks and its tip depth;
  additional trap profiles are not provided.
- Source-node transforms retain the numeric radius; component transforms act on
  evaluated geometry. Derived paths refresh after a source edit commits.
- Corner/cap/segment/brush components, variable pen points and the rest of P5
  remain separate work. This milestone does not complete P5.
