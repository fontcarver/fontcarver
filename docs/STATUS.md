# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-09-28 (paused by user after P5-10). Branch: `p5-color-authoring`
(no push, no main merge). Static SVG and COLRv1 authoring are implemented within
their acceptance boundaries. Batch styles and non-destructive layer effects are verified.
Frontend checks passed: 1,160 tests in 145 files, followed by 19 affected tests
including one added regression (1,161 distinct tests across the runs).
TypeScript, lint, production build and formatting passed.
The latest user instruction is to finish P5-10, save, update TODO and stop; resume only when requested.

P5-10 numeric live smart corners passed independent checks on 24 synthetic and
eight real-font exports (369,468 containment samples), bilingual browser editing,
bake/undo and compressed-native reopen. Rust coverage combines 1,007 tests before
the disk-cleanup interruption, 40 resumed tests and two additional regressions
(1,049 distinct passes; two pre-existing ignored). The interrupted run is not
claimed as one full pass. See [acceptance](p5-smart-corners-acceptance.md).
[Resource maintenance](build-artifacts.md) released approximately 491 GiB and
introduced build/temp budgets. Current resume instructions are in [TODO](TODO.md).

## Phases

| Phase | Scope                                                    | Status      | Done       | Record                                                                                                                           |
| ----- | -------------------------------------------------------- | ----------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------- |
| P0    | Tauri shell, lossless sfnt round trip, virtualised grid  | Complete    | 2026-09-03 | [p0-acceptance](p0-acceptance.md)                                                                                                |
| P1    | Outline editing, tools, panels, text tab, export         | Complete    | 2026-09-07 | [p1-acceptance](p1-acceptance.md)                                                                                                |
| P2a   | Horizontal kerning authoring                             | Complete    | 2026-09-16 | [p2a-acceptance](p2a-acceptance.md)                                                                                              |
| P2b   | Editable OpenType layout (FEA drafts, conversion)        | Complete    | 2026-09-17 | [p2b-acceptance](p2b-acceptance.md)                                                                                              |
| P2c/d | Static OTF/CFF; Glyphs, SFD, Type 1; Find problems       | Complete    | 2026-09-19 | [p2cd-acceptance](p2cd-acceptance.md)                                                                                            |
| P3    | Bitmap fonts: strikes, editor, BDF/PCF/PSF/FNT, EBDT     | Complete    | 2026-09-22 | [p3-acceptance](p3-acceptance.md)                                                                                                |
| P4    | Variable fonts, design space, CJK construction, vertical | Complete    | 2026-09-22 | [p4-acceptance](p4-acceptance.md)                                                                                                |
| P5    | Professional finish (112-row exit checklist)             | Paused      | —          | [plan](superpowers/plans/2026-09-22-p5-professional-finish.md) · [feature audit](p5-feature-audit.md) (the row-by-row exit gate) |

"Complete" means verified within the boundaries each acceptance report
states, not exhaustive format coverage. Native desktop input automation and
release packaging are unverified for every phase.

## P5 work in progress

The [feature audit](p5-feature-audit.md) is the exit gate: a row is marked
complete only after a working editing path, persistence/export and direct
verification. Rows 84 (numeric smart corners), 88 (stored boolean layer masks) and 104
(static COLR v0/CPAL) are complete within their recorded boundaries. Three rows are `Partial` (61
autohinting, 87 uniform strokes with variable pen points pending, 98 manual PS
hints); the other 106 rows are `Pending verification`. Pending verification
does not mean no implementation exists; earlier phase evidence must be checked. MATH is a
separate P5 design requirement, now verified in the audit’s supplemental section.

| Work block                       | State                                                                                                                         | Detail                                                                                               |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Hinting (TT/PS editors, Type 1)  | Static TT and PS editing, Type 1 hint round trips and export preprocessing done; variable hinting, flex, per-FD dicts open    | [p5-hinting-checkpoint](p5-hinting-checkpoint.md)                                                    |
| MATH authoring                   | Complete within the acceptance boundaries: editor, undo/save/reopen/export, FontTools/HarfBuzz and large-font checks verified | [p5-math-acceptance](p5-math-acceptance.md)                                                          |
| COLR/CPAL authoring              | Complete within static COLR v0 / CPAL v0/v1 boundaries; synthetic/real-font exports, browser editing and persistence verified | [p5-color-acceptance](p5-color-acceptance.md)                                                        |
| SVG colour documents             | Static subset verified; independent export/browser and regression checks passed                                               | [p5-svg-acceptance](p5-svg-acceptance.md)                                                            |
| COLRv1 paint graphs              | Static graph editor, shared codecs, palettes, persistence and independent binary/pixel checks verified                        | [p5-colrv1-acceptance](p5-colrv1-acceptance.md)                                                      |
| Geometry and construction        | Batch styles, non-destructive layer effects and numeric smart corners verified                                              | [style acceptance](p5-style-effects-acceptance.md) · [layer effects](p5-layer-effects-acceptance.md) · [smart corners](p5-smart-corners-acceptance.md) |
| Compare/merge/paste special      | Not started                                                                                                                   | plan block "Document workflows"                                                                      |
| Scripting and plugins            | Not started                                                                                                                   | plan block "Automation"                                                                              |
| Remaining formats, profiles, CLI | Not started                                                                                                                   | plan block "Formats/export"                                                                          |
| Shell/UX checklist gaps          | Not started                                                                                                                   | plan block "Shell/UX"                                                                                |
| Release packaging and CI         | Not started                                                                                                                   | plan block "Release"                                                                                 |
| Final acceptance                 | Not started                                                                                                                   | plan block "Final row-by-row acceptance"                                                             |

The Tibetan layout expansion-budget defect is fixed: conversion eligibility
now checks the compiler's work budget while retaining the imported lookup
inventory and original tables. Affected Rust regression checks passed (1,032
tests; two pre-existing ignored acceptance tests). This was not a full-workspace
native-platform run.

## Paused / next session

The user requested stopping after the current task. P5-10 is saved and verified
within its acceptance boundaries; temporary task outputs and owned services are
cleaned/stopped. See [TODO](TODO.md) for the remaining work. On an explicit resume,
start by scoping P5-11 variable pen points and running the resource-budget check.
No P5-11 implementation has started. Native/global desktop input remains prohibited.

The bilingual website is also complete, with deployment still pending. Its
[completion record](superpowers/plans/2026-09-24-website.md) covers the website
checks; release download links await the first packaged release.

## Where things are written

| Question                                    | File                                                                                                      |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| What is the product and architecture?       | [design spec](superpowers/specs/2026-09-03-fontcarver-design.md), [editor UI design](design/editor-ui.md) |
| What are the rules for contributors/agents? | [`CONVENTIONS.md`](../CONVENTIONS.md), [licensing](licensing.md)                                          |
| What did phase N deliver and how verified?  | `docs/pN-acceptance.md`                                                                                   |
| What is planned for a phase?                | `docs/superpowers/plans/<date>-<phase>.md`; per-task scope in `plans/tasks/`                              |
| Design decisions for a phase                | `docs/superpowers/specs/`                                                                                 |
| Detailed progress of an unfinished P5 block | `docs/p5-*-checkpoint.md`                                                                                 |
| How do I use feature X?                     | `docs/user/`                                                                                              |
| Background research (FontForge, crates, UX) | `docs/research/`                                                                                          |
| Agent work logs from P0/P1 (historical)     | `docs/archive/ledgers/` (unreferenced; kept as the review audit trail)                                    |
