# Remaining work

Branch: `p5-color-authoring`. Waves 1 and 2 of the [P5 completion
plan](superpowers/plans/2026-09-30-p5-completion.md) are integrated and committed
locally; nothing is pushed and no merge to `main` has happened. Native/global
desktop input automation remains prohibited.

The [feature audit](p5-feature-audit.md) is the completion gate. After wave 2 it
reads **110 Complete, 2 Partial, 0 Pending verification** of 112 rows (after
wave 1: 75 / 5 / 32; before wave 1: 3 / 3 / 106). This file records what those
numbers leave open. The user authorized wave 3 and parallel agents on
2026-10-02. Online publication is deferred by the user; signing credentials are
not configured. Complete verifiable builds and installation first.

## Wave 3: what is left of P5

Work is active under `docs/superpowers/plans/tasks/p5f-w3-*.md`. The earlier
metrics-key move, isolated E2E sessions and wording/translation follow-ups are
committed as `c1bd334`, `86770ab` and `a4be04a`, respectively. The tasks below
retain their pre-wave evidence status until their new acceptance checks pass.

| Item                            | What remains                                                                                                                                                                            |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rows 9 and 62                   | The two Partial rows below: native macOS and Windows execution, and the AppImage payload licensing.                                                                                     |
| AppImage copyleft payload       | 73 copyleft-licensed shared objects with notices for 10; LGPL relinking and source-offer duties unmet. Until it is cleared the deb is the only package safe to hand out.                |
| Release rebuild and publication | The only real packages were built from the pre-wave code, so the deb and AppImage must be rebuilt from the integrated tree, re-inspected, `apt install`-verified, signed and published. |
| English wording                 | Completed 2026-10-02: unified wording and independent GPT-6.1 Sol Chinese delta review; acceptance below.                                                                              |
| Final row-by-row gate           | One pass over all 112 rows against the integrated tree, with the full command set in the plan's exit criteria, before P5 is declared finished.                                          |
| Review follow-ups               | The documented limits below; none of them blocks a row. The metrics-key code move was completed on 2026-10-02.                                                                        |

Rows 84, 88 and 104 keep their earlier acceptance records.

## Partial rows and the exact missing piece

| Row | Requirement                                 | Missing                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 9   | Cross-platform parity (macOS/Windows/Linux) | No macOS or Windows binary of this code exists. The keymap abstraction and the whole shell are exercised on Linux only; native execution evidence is still needed.                                                                                                 |
| 62  | Release packaging and installation          | The AppImage bundles 73 copyleft-licensed packages with notices for 10; LGPL relinking and source-offer duties are unmet, so only the deb is safe to hand out. The deb's install is verified by extraction, not `apt install`. No macOS or Windows package exists. |

## English wording and Chinese delta — completed 2026-10-02

The wording follow-up from the two wave-2 translation reports is complete:

- Waterfall entries consistently refer to sizes; range spacing is an increment.
- Bitmap help names the actual Nearest pixel / Area average options.
- Workspace help names the actual Workspaces… / 工作区… command.
- English prose uses curly apostrophes consistently.
- The unused bare `cid.unmapped` key was removed from both catalogues; counted
  plural forms remain and resolve correctly in both languages.

As requested, an independent **GPT-6.1 Sol sub-agent** reviewed all 43 changed
English values against Chinese. One Chinese value needed revision; the other
42 were already accurate. Both catalogues now have **3,935 matching keys**.
See the [translation review](p5f-wording-translation-pass.md) and
[integrated acceptance](p5f-wording-translation-acceptance.md). This closes only
this wording task; the remaining P5 work below is unchanged.

## Follow-ups from the wave 1 and 2 reviews

- Completed 2026-10-02: `scale_metrics_key` and its existing unit tests now live
  beside `parse_key` in `crates/fc-doc/src/metrics.rs`; the API calls the shared
  helper without changing scaling behavior. This closes task `f`'s code-motion
  follow-up ([acceptance](p5f-metrics-key-move-acceptance.md)).
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

- Each test owns a **fresh kernel process and preference directory**. Default
  workflows get a private DejaVu copy; source workflows start empty. Teardown
  closes browser contexts and stops the kernel, including unfinished imports.
  `autosave_seconds: 0` is written before startup. The suite still uses **one
  worker** because Vite proxies one kernel port; use separate `FC_API_PORT` and
  `FC_UI_PORT` values for independent runs. Existing servers are never reused.
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

- Completed 2026-10-02: tests now own separate kernel processes, profiles and
  documents, including cleanup of unfinished imports. Save/conversion/history
  checks wait for the corresponding responses, and grid clicks tolerate
  virtualisation. See the [isolation acceptance](p5f-e2e-isolation-acceptance.md)
  for the original failures, regression and repeated verification.
- The current bundled headless Chromium cancels PNG, HTML and PDF downloads.
  A blank-page Blob download reproduces `canceled` without FontCarver. These
  three browser-download cases remain an environment verification gap; the
  complete default suite has **not** been established green. The external CJK
  corpus test also remains skipped on this machine.
- `Mod+]` used to apply a 100-unit sidebearing step to the glyph the Edit tab
  was leaving, because the spacing brackets and the previous/next glyph commands
  shared the binding. That was fixed in wave 1, but the ×100 and exact-one-unit
  spacing steps still have no keyboard binding; give them a non-conflicting one
  or leave the numeric fields as the way to make a large step.
