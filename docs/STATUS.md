# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-09-28 (remaining P5 work resumed). Branch: `p5-color-authoring`
(no push, no main merge). Static SVG and COLRv1 authoring are implemented within
their acceptance boundaries. Batch style effects are now verified too.
Frontend checks passed: 144 test files, 1,156 tests;
TypeScript, lint, production build and formatting are clean.
The user requested continuing all remaining tasks without confirmation.

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
| P5    | Professional finish (112-row exit checklist)             | In progress | —          | [plan](superpowers/plans/2026-09-22-p5-professional-finish.md) · [feature audit](p5-feature-audit.md) (the row-by-row exit gate) |

"Complete" means verified within the boundaries each acceptance report
states, not exhaustive format coverage. Native desktop input automation and
release packaging are unverified for every phase.

## P5 work in progress

The [feature audit](p5-feature-audit.md) is the exit gate: a row is marked
complete only after a working editing path, persistence/export and direct
verification. Row 104 is complete within static COLR v0/CPAL boundaries; two
rows are `Partial` (61 autohinting, 98 manual PS hints), and the other 109 rows
are `Pending verification`. MATH is a
separate P5 design requirement, now verified in the audit’s supplemental section.

| Work block                       | State                                                                                                                         | Detail                                            |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| Hinting (TT/PS editors, Type 1)  | Static TT and PS editing, Type 1 hint round trips and export preprocessing done; variable hinting, flex, per-FD dicts open    | [p5-hinting-checkpoint](p5-hinting-checkpoint.md) |
| MATH authoring                   | Complete within the acceptance boundaries: editor, undo/save/reopen/export, FontTools/HarfBuzz and large-font checks verified | [p5-math-acceptance](p5-math-acceptance.md)       |
| COLR/CPAL authoring              | Complete within static COLR v0 / CPAL v0/v1 boundaries; synthetic/real-font exports, browser editing and persistence verified | [p5-color-acceptance](p5-color-acceptance.md) |
| SVG colour documents            | Static subset verified; independent export/browser and regression checks passed | [p5-svg-acceptance](p5-svg-acceptance.md) |
| COLRv1 paint graphs              | Static graph editor, shared codecs, palettes, persistence and independent binary/pixel checks verified | [p5-colrv1-acceptance](p5-colrv1-acceptance.md) |
| Geometry and construction        | Batch style effects verified; non-destructive effects and remaining smart geometry next | [p5-style-effects-acceptance](p5-style-effects-acceptance.md) |
| Compare/merge/paste special      | Not started                                                                                                                   | plan block "Document workflows"                   |
| Scripting and plugins            | Not started                                                                                                                   | plan block "Automation"                           |
| Remaining formats, profiles, CLI | Not started                                                                                                                   | plan block "Formats/export"                       |
| Shell/UX checklist gaps          | Not started                                                                                                                   | plan block "Shell/UX"                             |
| Release packaging and CI         | Not started                                                                                                                   | plan block "Release"                              |
| Final acceptance                 | Not started                                                                                                                   | plan block "Final row-by-row acceptance"          |

The Tibetan layout expansion-budget defect is fixed: conversion eligibility
now checks the compiler's work budget while retaining the imported lookup
inventory and original tables. Affected Rust regression checks passed (1,032
tests; two pre-existing ignored acceptance tests). This was not a full-workspace
native-platform run.

## Next step

Continue with non-destructive layer effects, smart geometry and the remaining
P5 work sequence. Batch styles passed 658 Rust tests, 32 independent synthetic
exports, 19,212 real-font sampling checks and bilingual browser verification. Do not repeat the earlier task-boundary stop: the
2026-09-28 instruction supersedes it. See [COLRv1 acceptance](p5-colrv1-acceptance.md), [SVG usage](user/svg.md),
[acceptance](p5-svg-acceptance.md) and the [P5 plan](superpowers/plans/2026-09-22-p5-professional-finish.md).

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
