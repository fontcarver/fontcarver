# Whole-project effort estimate

Original estimate: 2026-09-28. Product baseline: `410cc97`
(`p5-color-authoring`). Latest reassessment: **2026-10-02**, current repair
integration and Linux installer acceptance. The central whole-product estimate
is **about 79%, with a weighted planning band of 72–86%**. This is an effort estimate for the broader professional
product, not the fraction of checklist rows, and not a release-readiness score.
The dated calculations below are historical snapshots.

The completed local batch covers closing work, the added editor UX review and
the user's one additional small welcome-discovery task.
The [future experience and feature backlog](TODO.md#future-experience-and-feature-backlog)
is explicitly optional later scope; listing it does not increase completed effort.
Publication and macOS/Windows
execution remain outside it. See [TODO](TODO.md) and
[current acceptance](p5f-w3-integration-acceptance.md).

## Reassessment on 2026-10-02

The [112-row feature audit](p5-feature-audit.md) has been independently reviewed
against current code and evidence ([1–56](p5f-w3-audit-a.md),
[57–112](p5f-w3-audit-b.md)). It retains **110 Complete / 2 Partial** within the
stated feature boundaries. Newly found ordinary defects in spacing, vertical
metrics, component submission, bitmap saving/proofing, native-open filters and
Fix All/cancellation were repaired and independently reviewed. Real ttfautohint
and the external large CJK font now have direct execution evidence. The original
integrated regression gate passed, including all 100 then-default browser tests.
The final UX/welcome source `f6f6cf9` passes **2,914 frontend tests / 348 files**
and **105/105 browser tests**, with zero skips or retries. Its source-matched
Linux packages have passed actual installation/startup and independent review.

Actual FontCarver deb and thin AppImage candidates were rebuilt from integrated
source **`f6f6cf9`** (2,008 source inputs). The first `6e35f70` round is archived
as superseded after the thin image bundle-marker correction and subsequent UX
changes. Final package inspection and independent review verify source/hash
provenance, dependency notices and the thin runtime's host-library policy. Real
isolated APT installation, same-version reinstallation, removal and user-data
retention pass. These results supersede the earlier synthetic/minimal probes;
[the package acceptance](p5f-w3-packaging-acceptance.md) records the actual
artifacts. They do not establish clean-OS dependency resolution, cross-version
upgrades, native macOS/Windows behavior, signing or publication.

Only the release/documentation/final-audit bucket changes. Its six-point weight
is retained and its completion band moves from **18–32% to 45–65%** for actual
Linux delivery acceptance, the independent row review and the integrated gate.
The other eleven area bands remain unchanged: most repairs restore behavior
already credited, and tests or commits are not additional product features.

| Calculation             | Previous |                              Current |
| ----------------------- | -------: | -----------------------------------: |
| Weighted lower endpoint |   70.67% | `70.67 + 6 × (0.45 − 0.18) = 72.29%` |
| Weighted upper endpoint |   83.78% | `83.78 + 6 × (0.65 − 0.32) = 85.76%` |
| Midpoint                |    77.2% |                                79.0% |

The 72–86% band is an engineering estimate, not a statistical interval or a
release-readiness score. The authorized current-version local closing work is complete;
the broader product retains the explicit geometry, color, format, family and
automation boundaries recorded in its acceptance reports and TODO. The
110/112 checklist count must not be presented as 98% completion of that broader
product.

## Reconciliation on 2026-09-30

The central estimate remains **about 60%, with a 50–70% planning range**. The
[resume evidence review](p5-resume-audit.md) now maps **all 112 rows** to exact
accepted subsets and remaining qualifiers. At resume the authoritative checklist
had 3 Complete, 3 Partial and 106 Pending verification rows. The review identifies
**13 pending rows whose stated scope has strong reusable acceptance evidence**:
3, 21, 22, 25, 28, 29, 45, 51, 59, 64, 67, 102 and 110. These recommendations
need their evidence attached by the integration owner; this documentation pass
does not itself change their acceptance status.

The remaining 96 rows are a mixture of partial implementations, verification
gaps and missing workflows. They are not 96 equally sized unimplemented features.
The accepted MATH/static SVG/static COLRv1/batch-style scopes also remain credited
without being counted again as new checklist completions. Historical P0–P4
working paths were already included in the original weighted estimate, so
reconciling their evidence does not create additional completed effort.

The review replaces vague remaining-work groups with the bounded acceptance
checklist in [the resume audit](p5-resume-audit.md#bounded-acceptance-checklist-for-the-largest-remaining-areas):

| Existing effort bucket              | Weight retained | Acceptance units now separated                                                                                                             |
| ----------------------------------- | --------------: | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Advanced geometry and construction  |               9 | Variable pens; attached component types; linked guides/servant nodes; editor geometry; image import/trace; optical spacing/kerning.        |
| Cross-document and family workflows |               5 | Paste target/payload matrix and relinking; document/reference comparison; family comparison and conflict-resolved merge.                   |
| Scripting and plugins               |               6 | Executable TypeScript console/reload; out-of-process Python RPC; plugin lifecycle/index; reporter and preview/apply filter contracts.      |
| Static formats and export           |               9 | Remaining codecs, including design-required Type 42/bare CFF; shared stored profiles/matrix/preflight; processing and failure-safe output. |
| Shell refinements and proofing      |               5 | Window/workspace lifecycle; preview and saved proof libraries; HTML/PDF and vertical proofing; help/keymap editing and translation review. |
| Release and final verification      |               6 | Local packages and installation; actual macOS/Windows runs; final row evidence, documentation, regression and resource gates.              |

These are subdivisions of existing buckets, not added scope or new effort
weights. Inventory, core outlines, layout, bitmap, variable/CJK and hinting retain
their original separate buckets; acceptance items that cross domains must not
earn duplicate credit. The largest unresolved risks remain advanced geometry,
family merging, automation/plugin behavior and native platform execution.

The shared checkout is receiving extensive parallel implementation. This
revision deliberately gives unaccepted new code no additional completion credit.
After integrated milestones, update each affected completion band from its
actual editing/persistence/output evidence and reassess the effort still needed.
The 50–70% band is a planning range, not a statistical interval or delivery-date
promise. No new application build, test or browser session was run for this
documentation review.

The weighted assessment and sensitivity below preserve the original 2026-09-28
baseline. Statements about code availability there describe that historical
snapshot; the resume audit and later acceptance records govern new work.

## Reassessment on 2026-09-30, after completion wave 1

Completion wave 1 of the [P5 completion
plan](superpowers/plans/2026-09-30-p5-completion.md) is implemented, reviewed,
fix-rounded, integrated and committed. The [feature audit](p5-feature-audit.md)
now reads **75 Complete, 5 Partial, 32 Pending verification** of 112 rows, up
from 3 Complete, 3 Partial and 106 Pending. Each accepted row carries an
acceptance document with the exact commands, browser evidence and limits, and a
review that returned "accept with fixes" whose items were then fixed. Integrated
verification for the wave: Rust 1,978 passed / 0 failed / 2 ignored across 271
test binaries; the frontend suite, `tsc`, ESLint, Prettier and the production
build pass; Playwright runs are recorded per configuration in
[STATUS](STATUS.md).

Only the completion bands of areas that gained accepted rows move, and each moves
conservatively: an accepted row is a bounded workflow with stated limits, not a
finished product area. Weights are unchanged, so the arithmetic is the same
`sum(area_weight * area_completion / 100)`.

| Product area                                                       | Weight | 2026-09-28 band |   Revised band | Why it moved                                                                                         |
| ------------------------------------------------------------------ | -----: | --------------: | -------------: | ---------------------------------------------------------------------------------------------------- |
| Document foundation, persistence, history, API and glyph inventory |      9 |          85–95% |         90–97% | Rows 6–8 and 10–18: workspaces, history panel, badges, labels, search, coverage, glyph data.         |
| Core outline editor                                                |     13 |          75–90% |         85–95% | Rows 20–36, 81, 89–91 accepted with their limits; ordinary editing is now verified end to end.       |
| Spacing, kerning and OpenType layout                               |     12 |          65–80% |         78–90% | Rows 37–45 and 46–51, 72, 99, 100: metrics keys, spacing matrix, kerning table, feature authoring.   |
| Static source/binary formats and export workflows                  |      9 |          60–80% |         70–85% | Rows 58–60, 63: stored profiles, instance × profile matrix, preflight, processing, CLI parity.       |
| Bitmap font workflows                                              |      7 |          70–90% |         70–90% | Unchanged; wave 2 task `g` owns rows 64–71 and 101–103.                                              |
| Variable fonts and CJK                                             |      9 |          65–85% |         72–88% | Rows 54, 56, 57: named configurations, compound condition sets, live interpolated scrubbing.         |
| Hinting, MATH and color authoring                                  |     10 |          65–85% |         65–85% | Unchanged; rows 61 and 98 stay Partial for wave 2 task `k`.                                          |
| Advanced geometry and construction                                 |      9 |          20–40% |         20–40% | Unchanged; rows 82–86, 92–94 and 97 are wave 2 task `j`.                                             |
| Cross-document and family workflows                                |      5 |           0–15% |         45–65% | Rows 77, 78, 96: paste matrix, relinking and placeholders, comparison and conflict-resolved merge.   |
| Scripting and plugins                                              |      6 |           5–15% |         50–70% | Rows 79, 105, 106: console with declarations and reload, Python RPC, plugin lifecycle, reporters.    |
| Shell refinements, proofing and UX parity                          |      5 |          25–45% |         40–55% | Rows 1–5 accepted; proofing (73–76, 95, 109) and help/keymaps/theming (80, 107, 108) remain.         |
| Release, documentation completion and final cross-platform audit   |      6 |          10–25% |         18–32% | Row 62 has real Linux packages, but AppImage licensing, `apt install` and macOS/Windows runs remain. |
| Total                                                              |    100 |      51.8–68.5% | **62.8–78.0%** | Rounded for planning to **60–80%**, central estimate **about 70%**.                                  |

Lower endpoints: `9·0.90 + 13·0.85 + 12·0.78 + 9·0.70 + 7·0.70 + 9·0.72 +
10·0.65 + 9·0.20 + 5·0.45 + 6·0.50 + 5·0.40 + 6·0.18 = 62.82`. Upper endpoints:
`9·0.97 + 13·0.95 + 12·0.90 + 9·0.85 + 7·0.90 + 9·0.88 + 10·0.85 + 9·0.40 +
5·0.65 + 6·0.70 + 5·0.55 + 6·0.32 = 77.97`. Their midpoint is 70.4. The decimals
only make the calculation reproducible; the inputs are engineering judgments.

What this revision deliberately does **not** do: it gives no credit for wave 2's
written task files, none for implementations without accepted rows, and none for
native macOS or Windows execution, which has never been run. The largest
remaining risks are unchanged — advanced geometry and optical algorithms, bitmap
and proofing surfaces, hinting completion, cross-platform packaging and the
defects a final row-by-row gate will find.

## Reassessment on 2026-10-01, after completion wave 2

Completion wave 2 (bitmap, proofing, construction, hinting, CJK, help) is
implemented, reviewed, fix-rounded, integrated and committed. The [feature
audit](p5-feature-audit.md) now reads **110 Complete, 2 Partial, 0 Pending
verification** of 112 rows, up from 75 / 5 / 32 after wave 1 and 3 / 3 / 106
before it. The two Partial rows are 9 and 62: native macOS and Windows
execution, and the AppImage copyleft payload licensing. Integrated verification
for this wave is listed in [STATUS](STATUS.md); as in wave 1, every accepted row
carries an acceptance document with its commands, browser evidence and limits,
and a review whose items were fixed.

Only the completion bands of areas that gained accepted rows move, each moves
conservatively, and the weights are unchanged, so the arithmetic is still
`sum(area_weight * area_completion / 100)`.

| Product area                                                       | Weight | Wave 1 band |   Revised band | Why it moved                                                                                                      |
| ------------------------------------------------------------------ | -----: | ----------: | -------------: | ----------------------------------------------------------------------------------------------------------------- |
| Document foundation, persistence, history, API and glyph inventory |      9 |      90–97% |         90–97% | Unchanged; wave 2 added no row here.                                                                              |
| Core outline editor                                                |     13 |      85–95% |         85–95% | Unchanged; the wave's editor-adjacent rows are counted under advanced geometry.                                   |
| Spacing, kerning and OpenType layout                               |     12 |      78–90% |         78–90% | Unchanged; auto-spacing (94) is credited to geometry and the proof library (95) to proofing.                      |
| Static source/binary formats and export workflows                  |      9 |      70–85% |         70–85% | Unchanged; the bitmap tables (102) and proof export (109) are credited in their own areas.                        |
| Bitmap font workflows                                              |      7 |      70–90% |         85–95% | Rows 64–71 and 101–103: strikes, the editor's nine tools, overlays, rasterisation, tracing, BDF/PNG, EBDT/CBDT.   |
| Variable fonts and CJK                                             |      9 |      72–88% |         82–92% | Rows 19, 110, 111 and 112: vertical metrics, CID mapping and export, vertical mode, radical/IDS construction.     |
| Hinting, MATH and color authoring                                  |     10 |      65–85% |         75–88% | Rows 61 and 98 plus the TrueType scope: analysis quality, flex, dot sections, per-FD data, `cvar`, CFF2 private.  |
| Advanced geometry and construction                                 |      9 |      20–40% |         55–75% | Rows 82, 83, 85, 86, 87, 92, 93, 94, 97 accepted within their limits; the limits keep the band well below 100%.   |
| Cross-document and family workflows                                |      5 |      45–65% |         45–65% | Unchanged.                                                                                                        |
| Scripting and plugins                                              |      6 |      50–70% |         50–70% | Unchanged.                                                                                                        |
| Shell refinements, proofing and UX parity                          |      5 |      40–55% |         75–88% | Rows 73–76, 95, 109 and 80, 107, 108: preview and proofing, proof library, HTML/PDF sheets, help, keymaps, theme. |
| Release, documentation completion and final cross-platform audit   |      6 |      18–32% |         18–32% | Unchanged; no package was rebuilt from this code and no native platform was run.                                  |
| Total                                                              |    100 |  62.8–78.0% | **70.7–83.8%** | Rounded for planning to **70–85%**, central estimate **about 77%**.                                               |

Lower endpoints: `9·0.90 + 13·0.85 + 12·0.78 + 9·0.70 + 7·0.85 + 9·0.82 +
10·0.75 + 9·0.55 + 5·0.45 + 6·0.50 + 5·0.75 + 6·0.18 = 70.67`. Upper endpoints:
`9·0.97 + 13·0.95 + 12·0.90 + 9·0.85 + 7·0.95 + 9·0.92 + 10·0.88 + 9·0.75 +
5·0.65 + 6·0.70 + 5·0.88 + 6·0.32 = 83.78`. Their midpoint is 77.2. The decimals
only make the calculation reproducible; the inputs are engineering judgments.

What this revision deliberately does **not** do: it gives no credit for a
release package built from this code (none exists), none for native macOS or
Windows execution, none for the final row-by-row gate that has not run, and none
for closing the documented limits each acceptance document states. Those, with
the AppImage payload licensing, are what the remaining band covers; they are
also why the upper endpoint stays below 85%.

## Current interpretation

The latest accepted milestone is the 2026-10-02 local integration and Linux
installer verification. Its broader-product estimate is **about 79%**, with a
weighted band rounded to **72–86%**. Earlier 60%, 70% and 77% estimates above
remain historical snapshots. The remaining delivery gates are native
macOS/Windows execution, signing and publication; accepted feature boundaries
also reserve later product work. No public-release or delivery-date claim follows
from this estimate.

## Scope and method

The target is the existing [product design](superpowers/specs/2026-09-03-fontcarver-design.md)
and [P5 exit checklist](p5-feature-audit.md), including remaining professional
editing workflows, formats, automation, installation, documentation and final
verification. New feature requests and ongoing maintenance after delivery are
excluded. The separate marketing website is not used to increase this score.

The table groups work by product area rather than by phase number. Its weights
sum to 100 and represent estimated relative implementation, integration and
verification effort for the complete area. They are engineering judgments, not
measured hours, commit counts, code-line counts or equal-sized checklist items.
Completion bands give partial credit to accepted working paths while reserving
work for known gaps, broader acceptance and integration. Earlier phase completion
remains valid within its documented scope; these broader product-area bands do
not revoke those milestones.

Evidence reviewed for the original estimate: P0–P4 acceptance records, P5 acceptance/checkpoint records, all
112 audit requirement descriptions, the remaining TODO, and representative code
entry points for the shell, layout inspector, API handlers, formats and CLI. No
fresh application tests, builds, browser sessions or native input were run for
that estimate. The 2026-09-30 reconciliation adds a row-by-row evidence map;
neither review claims fresh execution of all 112 requirements.

## Weighted assessment

| Product area                                                       | Effort weight |         Estimated completion | Evidence and remaining work                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------ | ------------: | ---------------------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Document foundation, persistence, history, API and glyph inventory |             9 |                       85–95% | [P0](p0-acceptance.md), [P1](p1-acceptance.md): document model, native save/recovery, history, transport and large glyph grid. Some inventory/metadata behaviors and full-product verification remain.                                                                                                                                                |
| Core outline editor                                                |            13 |                       75–90% | [P1](p1-acceptance.md): drawing, point editing, transforms, guides, components, layers and text editing. Complete modifier/tool/audit behavior still needs checklist closure. Advanced live construction is counted separately below.                                                                                                                 |
| Spacing, kerning and OpenType layout                               |            12 |                       65–80% | [P2a](p2a-acceptance.md), [P2b](p2b-acceptance.md): editable groups, pair rules, FEA, diagnostics and validated conversion. Visual lookup authoring, broader metric/kerning UI and feature-token requirements remain. The existing lookup inspector is read-only.                                                                                     |
| Static source/binary formats and export workflows                  |             9 |                       60–80% | [P1](p1-acceptance.md), [P2c/d](p2cd-acceptance.md): working TTF/OTF/web containers, UFO/designspace, Glyphs, SFD and Type 1 paths. Remaining SVG/EPS/AFM/PFM/TTX/collection work, profiles, processing and preflight remain. Bitmap, variable and color semantics are counted in their own areas.                                                    |
| Bitmap font workflows                                              |             7 |                       70–90% | [P3](p3-acceptance.md): editable strikes, tools, legacy and embedded formats, rasterization, exact pixel tracing and waterfall. Broader pixel-as-component, tracing options, PNG-sheet and checklist parity need verification or implementation.                                                                                                      |
| Variable fonts and CJK                                             |             9 |                       65–85% | [P4](p4-acceptance.md): axes/masters/instances, variable export, conditional sources, local smart components, vertical/CID/Unihan workflows and Linux font installation. Richer live/compatibility UI, virtual masters and CJK construction remain.                                                                                                   |
| Hinting, MATH and color authoring                                  |            10 |                       65–85% | [Hinting](p5-hinting-checkpoint.md), [MATH](p5-math-acceptance.md), [COLR](p5-color-acceptance.md), [SVG](p5-svg-acceptance.md), [COLRv1](p5-colrv1-acceptance.md): substantial independently verified paths. Advanced/variable hinting, per-FD data and the recorded rendering/format boundaries keep this below 100%.                               |
| Advanced geometry and construction                                 |             9 |                       20–40% | [Styles](p5-style-effects-acceptance.md), [layer effects](p5-layer-effects-acceptance.md), [smart corners](p5-smart-corners-acceptance.md) are verified. Variable pen points, corner/cap/segment/brush components, linked guides, servant nodes, image workflows, optical spacing and auto-kerning remain. Ordinary outline tools are credited above. |
| Cross-document and family workflows                                |             5 |                        0–15% | Existing documents and ordinary clipboard behavior provide reusable foundations. Compare/family compare, merge, Paste Special and full cross-document relinking have no accepted complete workflow.                                                                                                                                                   |
| Scripting and plugins                                              |             6 |                        5–15% | Typed API generation and transports provide limited reusable groundwork. Macro console/reload, Python RPC, plugin manager/lifecycle and reporter/filter contracts remain. This is not a claim that a user scripting/plugin workflow is delivered.                                                                                                     |
| Shell refinements, proofing and UX parity                          |             5 |                       25–45% | Existing theme/language controls, panels, shortcuts and preview supply a base. Per-document OS windows/tear-off panels, saved workspaces, full filter DSL, proofing, shortcut editor, help/accessibility and independent translation review remain. Basic shell and editor behavior is credited in the earlier areas.                                 |
| Release, documentation completion and final cross-platform audit   |             6 |                       10–25% | Existing CI, user docs and Linux build evidence provide groundwork. Installers/release workflows, macOS/Windows acceptance and final checklist reconciliation remain. Previously completed feature-specific tests are credited with those features, not a second time here.                                                                           |
| Total                                                              |       **100** | **51.8–68.5% weighted band** | Rounded for planning to **50–70%**, central estimate **about 60%**.                                                                                                                                                                                                                                                                                   |

The arithmetic is `sum(area_weight * area_completion / 100)`. Using every lower
endpoint gives 51.8; every upper endpoint gives 68.5; their midpoint is 60.15.
Those decimals only make the calculation reproducible. The underlying estimates
are much less precise. The planning band is not a statistical confidence interval.

## Why the earlier counts were insufficient

- P0–P4 have accepted deliverables, but phase sizes differ and their completion
  does not imply that five-sixths of the complete product is delivered.
- P5 has six checked work blocks, two partially implemented blocks and six
  unstarted blocks. These blocks also differ in effort; this estimate does not
  convert six out of fourteen directly into a P5 or whole-product percentage.
- The 106 audit rows marked `Pending verification` include existing P1–P4
  functionality. They cannot all be counted as unimplemented, and they cannot
  be marked complete from filenames or historical phase labels alone.
- Examples found in code support the gap assessment: the
  [lookup inspector](../app/src/views/features/LookupInspector.tsx) explicitly
  provides read-only inspection; the
  [format registry](../crates/fc-formats/src/lib.rs) and
  [CLI opener](../crates/fc-cli/src/cmd/mod.rs) show the supported source paths;
  the [desktop configuration](../src-tauri/tauri.conf.json) currently defines a
  single main window. A model field for background images does not establish a
  complete image-editing workflow.

## Sensitivity and next revision

Remaining work is roughly 40 units against 60 units already credited. Holding
completed work fixed illustrates the risk in estimating unfinished work:

| Scenario                       | Implied whole-project completion |
| ------------------------------ | -------------------------------: |
| Current central estimate       |                        About 60% |
| Remaining effort is 25% larger |                        About 55% |
| Remaining effort is 50% larger |                        About 50% |
| Remaining effort doubles       |                        About 43% |

The largest uncertainties are advanced geometry/optical algorithms, scripting
and plugin boundaries, behavior required by broad checklist rows, cross-platform
packaging and defects discovered during final acceptance. The high-complexity
remaining work is why an 80–90% completion claim would be unjustified.

The 2026-09-30 resume reconciliation now maps older acceptance evidence to the
pending checklist and splits the large remaining areas into bounded deliverables.
No new accepted milestone yet justifies changing the original weights/bands.
Update the estimate at integrated milestones or scope changes;
do not increase it automatically for each commit or task number. Calendar time
requires that further breakdown and observed throughput; elapsed calendar days
and test counts are not enough to predict a completion date.

Development resumed on 2026-09-30. This estimate revision adds evidence and
planning documentation only; parallel implementation has separate acceptance.
