# Remaining work

Branch: `p5-color-authoring`. Wave 1 of the [P5 completion
plan](superpowers/plans/2026-09-30-p5-completion.md) is integrated and committed
locally; nothing is pushed and no merge to `main` has happened. Native/global
desktop input automation remains prohibited.

The [feature audit](p5-feature-audit.md) is the completion gate. After wave 1 it
reads **75 Complete, 5 Partial, 32 Pending verification** of 112 rows (before the
wave: 3 Complete, 3 Partial, 106 Pending). This file records what those numbers
leave open. It is documented backlog, not an instruction to start the next wave.

## Wave 2 and 3, planned and not started

| Task file                                                             | Area                                                   | Rows                               |
| --------------------------------------------------------------------- | ------------------------------------------------------ | ---------------------------------- |
| [`p5f-g-bitmap`](superpowers/plans/tasks/p5f-g-bitmap.md)             | Bitmap strikes, editor, formats, waterfall             | 64–71, 101–103                     |
| [`p5f-h-proofing`](superpowers/plans/tasks/p5f-h-proofing.md)         | Preview, proofing, waterfall, sheet export             | 73–76, 95, 109, 111                |
| [`p5f-j-construction`](superpowers/plans/tasks/p5f-j-construction.md) | Construction, images, optical, virtual                 | 82, 83, 85, 86, 87, 92, 93, 94, 97 |
| [`p5f-k-hinting`](superpowers/plans/tasks/p5f-k-hinting.md)           | Hinting completion                                     | 61, 98                             |
| [`p5f-l-cjk`](superpowers/plans/tasks/p5f-l-cjk.md)                   | CJK completion                                         | 19, 110, 111 (Edit View), 112      |
| [`p5f-m-help`](superpowers/plans/tasks/p5f-m-help.md)                 | Help, keymaps, theming, accessibility                  | 80, 107, 108                       |
| wave 3                                                                | Remaining Partial rows, final row-by-row gate, release | as reported                        |

Rows 84, 88 and 104 keep their earlier acceptance records.

## Partial rows and the exact missing piece

| Row | Requirement                                 | Missing                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 9   | Cross-platform parity (macOS/Windows/Linux) | No macOS or Windows binary of this code exists. The keymap abstraction and the whole shell are exercised on Linux only; native execution evidence is still needed.                                                                                                 |
| 61  | Autohinting (ttfautohint, PS autohint)      | Authored variable-font hints, imported flex binding, dotsection and general counter semantics; CFF2 private variation data. Wave 2 task `k`.                                                                                                                       |
| 62  | Release packaging and installation          | The AppImage bundles 73 copyleft-licensed packages with notices for 10; LGPL relinking and source-offer duties are unmet, so only the deb is safe to hand out. The deb's install is verified by extraction, not `apt install`. No macOS or Windows package exists. |
| 87  | Non-destructive strokes with pen points     | Corner/cap/segment/brush components, radius drag and crossing construction beyond the current bounded pen work. Wave 2 task `j`.                                                                                                                                   |
| 98  | Manual PostScript hint editing              | Hint replacement across the full grammar and per-FD private data. Wave 2 task `k`.                                                                                                                                                                                 |

## Follow-ups from wave 1 reviews

- `scale_metrics_key` lives in `crates/fc-api/src/handlers/variation.rs` because
  `crates/fc-doc/src/metrics.rs` was owned by another task during the wave; move
  it beside `parse_key` (task `f`'s own follow-up, not an integration change).
- Documented limits that are not defects but are the next increments if a row is
  ever reopened: Fix All needs a second press when one contour holds two
  structural defects of different kinds; the anchor cascade is bounded to four
  rounds; the measurement-guide switch is view state; interchange formats carry
  no guide anchoring; sorting in the glyph grid is ascending-only; `include`
  statements in imported FEA are preserved verbatim rather than resolved.
- Headless Chrome in this checkout needs the shell sandbox disabled, so
  Playwright and agent-browser runs are environment-dependent; record that
  precisely rather than as a product failure.

## English source strings to correct (then a zh-CN delta pass)

Recorded from the two localization passes
([1](p5f-localization-pass1.md), [2](p5f-localization-pass2.md)). Every change
here needs a small zh-CN delta pass afterwards, by a translation agent.

- Unused keys to remove: `exportProfiles.hint` (replaced by `projectHint`),
  `history.font` and `history.current` (the panel uses `history.currentMarker`);
  no `t()` reference exists in `app/src` or `app/e2e` for any of the three.
- Normalise "Licence" (1 use) to "License" (6) and "Colour" (3) to "Color" (13).
- `exportMatrix.profiles` still reads "Compiled-font profiles"; `exportMatrix.loading`,
  `changed`, `noInstances` and `error.invalid-instance` still describe named
  instances only, although variable-font configurations are rows too.
- Prefer "node" over "point" and "group" over "class" where the same concept
  appears both ways and the string is not a FEA/OpenType term:
  `editor.audit.detail.coincident` ("nodes") versus `coincidentManual` ("points");
  `kerningTable.classRule` and `kerningTable.compressHint` ("class") versus
  `kinds.groups` ("group").
- `fontSettings.upmHint` says a hint is "resolved" where the kernel says "bake or
  remove"; `history.layerRole.smart` says "Smart layer" where
  `glyphInspector.layerKind.smart` says "Smart source".

## Product boundaries that outlive the audit rows

| Area           | Remaining scope                                                                                                                                |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Cross-platform | macOS and Windows are reviewed in CI definitions only; no native run, installer test or platform input verification exists for any phase.      |
| Release        | AppImage payload licensing, `apt install` verification, signing, publication and the website's first download links.                           |
| Color/proofing | Static palette-zero previews only; axis-dependent paints and color proof export remain unsupported (rows 73–76, 95, 109 in wave 2).            |
| Formats/export | Supported static interchange subsets and raw-table TTX, not general semantic TTX XML; Type 42 and bare CFF remain design-required and unbuilt. |
| Automation     | Built-in plugin index only; Python needs a local interpreter; an already dispatched synchronous kernel call is not interruptible.              |
| Geometry       | Optical spacing/auto-kerning, autotrace curve fitting, node-linked guides and servant nodes are wave 2 task `j`.                               |

Before a future build, run `python3 scripts/check_artifacts.py`. Shared build
dependencies are subject to the 20 GiB ceiling and 10 GiB warning in
`CONVENTIONS.md`; clean the development profile before a build that would exceed
it. Never delete shared stores, source files, fonts, Git data or another task's
files.

## End-to-end suite constraints found during integration

- The suite runs **one worker** (its specs share one kernel document) and seeds
  `autosave_seconds: 0`: the longer serial run otherwise triggered a crash
  recovery snapshot mid-run, and the Recover Documents dialog then swallowed
  every later click. Any spec that leaves an unsaved document must still expect
  the dialog when autosave is on.
- A suite that opens its own fonts must close **only what it opened**. The P2a
  and P2b fixtures closed every document, which removed the shared DejaVu
  fixture the later specs need.
- The Edit tab needs about **950 px of height** for the text bar, the spacing
  and kerning chrome, the 260 px canvas floor and the preview strip. Below that
  it scrolls, and a spec that computes canvas coordinates drags off screen, so
  the default Playwright viewport is now 1440x1000 like the P2 configurations.
  A narrower window is usable but has not been given acceptance evidence.
- Type 1 export no longer reports `type1-hinting-loss` for a source without
  TrueType instructions, because supported PostScript stems and private values
  are now written; it reports `type1-unicode-loss` instead. The P2c/d spec was
  updated to the current disclosure.

## Open regressions found during integration

- **Typed text is sometimes dropped in the Text tab (intermittent, blocking a
  P1 workflow).** After `t` opens the Text tab and a string is typed into the
  text field, the shaped run sometimes stays empty (`data-run-count="0"`), and
  the click that should enter local edit mode on a shaped glyph then does
  nothing. `app/e2e/text.spec.ts:122` failed three runs out of five on its own,
  twice on the run count and once on the mode switch, and passed in the suite.
  `TextView.transition()` drops its `patch()` when the active tab or the editor
  instance changes between its awaits, which is the most likely cause; the
  typed text is then lost without a message. Owner: the editor/metrics area.
- **Two P2b specs are intermittent in the default end-to-end configuration**,
  where every spec shares one kernel document: `p2b-imported.spec.ts:75` (the
  feature authority does not always revert to `ImportedTables` after the
  original is restored) and `p2b-source.spec.ts:26` (the stored draft comes back
  undefined). Both pass in `e2e/p2b.config.ts`, which gives them their own
  kernel, 17 of 17. Either give those specs their own document in the default
  configuration or find the shared state they depend on.
- `Mod+]` used to apply a 100-unit sidebearing step to the glyph the Edit tab
  was leaving, because the spacing brackets and the previous/next glyph commands
  shared the binding. Fixed during integration (the modified brackets no longer
  space), but the ×100 and exact-one-unit spacing steps now have no keyboard
  binding; give them a non-conflicting one or leave the numeric fields as the
  way to make a large step.
