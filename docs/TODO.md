# Remaining work

Branch: `p5-color-authoring`. Waves 1 and 2 of the [P5 completion
plan](superpowers/plans/2026-09-30-p5-completion.md) are integrated and committed
locally; nothing is pushed and no merge to `main` has happened. Native/global
desktop input automation remains prohibited.

The [feature audit](p5-feature-audit.md) is the completion gate. After wave 2 it
reads **110 Complete, 2 Partial, 0 Pending verification** of 112 rows (after
wave 1: 75 / 5 / 32; before wave 1: 3 / 3 / 106). This file records what those
numbers leave open. It is documented backlog, not an instruction to start the
next wave.

## Wave 3: what is left of P5

| Item                            | What remains                                                                                                                                                                            |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rows 9 and 62                   | The two Partial rows below: native macOS and Windows execution, and the AppImage payload licensing.                                                                                     |
| AppImage copyleft payload       | 73 copyleft-licensed shared objects with notices for 10; LGPL relinking and source-offer duties unmet. Until it is cleared the deb is the only package safe to hand out.                |
| Release rebuild and publication | The only real packages were built from the pre-wave code, so the deb and AppImage must be rebuilt from the integrated tree, re-inspected, `apt install`-verified, signed and published. |
| English wording                 | The list from the wave 2 translation reports, below, then a small zh-CN delta pass by a translation agent.                                                                              |
| Final row-by-row gate           | One pass over all 112 rows against the integrated tree, with the full command set in the plan's exit criteria, before P5 is declared finished.                                          |
| Review follow-ups               | The documented limits and the one code-motion item below; none of them blocks a row.                                                                                                    |

Rows 84, 88 and 104 keep their earlier acceptance records.

## Partial rows and the exact missing piece

| Row | Requirement                                 | Missing                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 9   | Cross-platform parity (macOS/Windows/Linux) | No macOS or Windows binary of this code exists. The keymap abstraction and the whole shell are exercised on Linux only; native execution evidence is still needed.                                                                                                 |
| 62  | Release packaging and installation          | The AppImage bundles 73 copyleft-licensed packages with notices for 10; LGPL relinking and source-offer duties are unmet, so only the deb is safe to hand out. The deb's install is verified by extraction, not `apt install`. No macOS or Windows package exists. |

## English source strings to correct (then a zh-CN delta pass)

The wave 1 list was applied in full by task `m`
([acceptance](p5f-m-help-acceptance.md)), and the `construction.magnetHud` /
`magnetSnapHud` plurals were fixed in task `j`'s review round. These are what the
wave 2 passes ([1](p5f-localization-wave2-pass1.md),
[2](p5f-localization-wave2-pass2.md)) flagged and did not change; `en.json` must
not be edited without a zh-CN delta pass afterwards, by a translation agent.

- `proofPreview.sizesHint` says "steps" where its sibling keys say "sizes", and
  `proofPreview.sizesError.range` mixes "size" and "step".
- `bitmapParity.transferHint` says "Nearest" while the option
  (`bitmapParity.filters.Nearest`) reads "Nearest pixel".
- `contextHelp.topics.workspace.third` says "Manage Workspaces"; the command it
  names reads "Workspaces…".
- Straight and curly apostrophes are mixed (24 straight against 5 curly among
  the strings wave 2 added).
- `cid.unmapped` has both a bare key and a `_one`/`_other` pair while the call
  passes `count`; the bare key is dead weight.

## Follow-ups from the wave 1 and 2 reviews

- `scale_metrics_key` still lives in `crates/fc-api/src/handlers/variation.rs`
  because `crates/fc-doc/src/metrics.rs` was owned by another task during wave 1;
  move it beside `parse_key` (task `f`'s own follow-up, not an integration
  change).
- Documented limits that are not defects, but are the next increments if a row is
  reopened: Fix All needs a second press when one contour holds two structural
  defects of different kinds; the anchor cascade is bounded to four rounds;
  sorting in the glyph grid is ascending-only; `include` statements in imported
  FEA are preserved verbatim; colour strike export takes the whole font and a
  font holding both EBDT and CBDT is served from the colour tables; pixel-font
  detection stays conservative; autotrace reads one brightness threshold, so
  photographs and shaded scans are out of scope, and its tolerance is a target
  rather than a guarantee; optical spacing is a sampled approximation; virtual
  masters are axis-only positions; authored TrueType glyph programs still need a
  static export, and CFF2 carries no stem, mask or flex operators; there is no
  bundled ideographic-description database, so an IDS is typed or assembled;
  vertical writing mode is an Edit View canvas mode, not vertical text layout;
  the help article is modal and its contrast audit covers the seventeen surfaces
  the fixture could reach.
- `ttfautohint` itself is absent from this machine, so every claim about an
  installed external hinter rests on the fake-executable tests and a skipped
  real-binary test. Running it once on a machine that has it would close that.
- Headless Chrome in this checkout needs the shell sandbox disabled, so
  Playwright and agent-browser runs are environment-dependent; record that
  precisely rather than as a product failure.

## Product boundaries that outlive the audit rows

| Area           | Remaining scope                                                                                                                                                                       |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cross-platform | macOS and Windows are reviewed in CI definitions only; no native run, installer test or platform input verification exists for any phase.                                             |
| Release        | AppImage payload licensing, `apt install` verification, signing, publication and the website's first download links.                                                                  |
| Colour         | Static palette-zero previews and proofs only; axis-dependent paints and colour proof sheets remain unsupported. The proofing rows (73–76, 95, 109, 111) are accepted around that gap. |
| Formats/export | Supported static interchange subsets and raw-table TTX, not general semantic TTX XML; Type 42 and bare CFF remain design-required and unbuilt.                                        |
| Automation     | Built-in plugin index only; Python needs a local interpreter; an already dispatched synchronous kernel call is not interruptible.                                                     |
| Geometry       | Accepted within the limits above: optical spacing is sampled, autotrace is threshold-based, attachment placement is affine and smart-component handles commit on release.             |

Before a future build, run `python3 scripts/check_artifacts.py`. Shared build
dependencies are subject to the 20 GiB ceiling and 10 GiB warning in
`CONVENTIONS.md`; clean the development profile before a build that would exceed
it. Never delete shared stores, source files, fonts, Git data or another task's
files.

## End-to-end suite constraints

- The suite runs **one worker** (its specs share one kernel document) and seeds
  `autosave_seconds: 0`: the longer serial run otherwise triggered a crash
  recovery snapshot mid-run, and the Recover Documents dialog then swallowed
  every later click. Any spec that leaves an unsaved document must still expect
  the dialog when autosave is on, and a spec that opens its own font must close
  **only what it opened**.
- The Edit tab needs about **950 px of height** for the text bar, the spacing and
  kerning chrome, the 260 px canvas floor and the preview strip, so the default
  Playwright viewport is 1440x1000 like the P2 configurations. A narrower window
  is usable but has not been given acceptance evidence; the Edit tab's view now
  scrolls vertically rather than pushing the canvas off screen.
- The **first** `text.shape` of a session compiles the shaper font and takes
  about four seconds. A spec that types into the Text tab must wait on
  `data-text-shaping`, which is what the intermittent `text.spec.ts:122` failure
  recorded after wave 1 turned out to be — a cold compile, not dropped input
  ([acceptance](p5f-h-proofing-acceptance.md)).
- Type 1 export no longer reports `type1-hinting-loss` for a source without
  TrueType instructions, because supported PostScript stems and private values
  are now written; it reports `type1-unicode-loss` instead.

## Open regressions and intermittents

- **The default end-to-end configuration fails about three of its 94 executing
  specs per run, and not the same three.** Both wave 2 integration runs passed
  91; the failures were `p2b-imported.spec.ts:75` in one and
  `fontview.spec.ts:303` (a grid cell detached during virtualisation),
  `panels.spec.ts:108` (an Edit tab not yet created) and `p2b-source.spec.ts:26`
  (a stored draft read back undefined) in the other, each passing in the run
  where the others failed. Every one of them drives the single kernel document
  that configuration shares. The P2b pair passes 17/17 in `e2e/p2b.config.ts`.
  Either give those specs their own document in the default configuration or
  find the shared state they depend on; until then a single red spec there is
  not evidence of a product defect, and the dedicated configurations are the
  ones to trust.
- `Mod+]` used to apply a 100-unit sidebearing step to the glyph the Edit tab
  was leaving, because the spacing brackets and the previous/next glyph commands
  shared the binding. That was fixed in wave 1, but the ×100 and exact-one-unit
  spacing steps still have no keyboard binding; give them a non-conflicting one
  or leave the numeric fields as the way to make a large step.
