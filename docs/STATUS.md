# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-09-26 (MATH acceptance). Branch: `p5-math-acceptance`
(no push, no main merge). This checkpoint completes MATH acceptance on top of
the website checkpoint `99aa3ed`.

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
verification. Currently two rows are `Partial` (61 autohinting, 98 manual PS
hints) and the rest of the 112-row checklist are `Pending verification`. MATH is a
separate P5 design requirement, now verified in the audit’s supplemental section.

| Work block                       | State                                                                                                                         | Detail                                            |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| Hinting (TT/PS editors, Type 1)  | Static TT and PS editing, Type 1 hint round trips and export preprocessing done; variable hinting, flex, per-FD dicts open    | [p5-hinting-checkpoint](p5-hinting-checkpoint.md) |
| MATH authoring                   | Complete within the acceptance boundaries: editor, undo/save/reopen/export, FontTools/HarfBuzz and large-font checks verified | [p5-math-acceptance](p5-math-acceptance.md)       |
| COLR/CPAL, SVG colour fonts      | Not started                                                                                                                   | plan block "OpenType authoring"                   |
| Geometry and construction        | Not started                                                                                                                   | plan block "Geometry and construction"            |
| Compare/merge/paste special      | Not started                                                                                                                   | plan block "Document workflows"                   |
| Scripting and plugins            | Not started                                                                                                                   | plan block "Automation"                           |
| Remaining formats, profiles, CLI | Not started                                                                                                                   | plan block "Formats/export"                       |
| Shell/UX checklist gaps          | Not started                                                                                                                   | plan block "Shell/UX"                             |
| Release packaging and CI         | Not started                                                                                                                   | plan block "Release"                              |
| Final acceptance                 | Not started                                                                                                                   | plan block "Final row-by-row acceptance"          |

Known open defect: the pre-existing Tibetan layout expansion-budget test
failure, tracked in the P5 plan; the workspace test suite is not claimed green
until it is diagnosed.

## Next step

Continue the P5 OpenType authoring block with COLR/CPAL palettes and layers,
then SVG colour documents. MATH acceptance is complete; see
[the user guide](user/math.md) and [verification record](p5-math-acceptance.md).

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
