# Whole-project effort estimate

Original estimate: 2026-09-28. Product baseline: `410cc97`
(`p5-color-authoring`). Evidence reconciliation: **2026-09-30**, at historical
checkout `91db6fa`. Development resumed at the user's explicit request. This
document remains a planning estimate, not a feature-acceptance record.

The user's later instruction freezes the active batch: complete the work already
underway, then stop and rest. The remaining acceptance checklist records future
work without starting new tasks. Final active-batch acceptance is recorded
separately by the integration owner.

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

## Conclusion

The current central estimate is **about 60% of the work toward the approved
complete product**, with a practical planning range of **50–70%**. The engineering
judgment is that the project is probably beyond halfway, with substantial work
remaining. The evidence does not support a guarantee that it is beyond halfway,
a claim that P5 is almost finished, or a delivery date.

This replaces the previous absence of a whole-project estimate. It does not
change any accepted scope or close any pending audit row.

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
