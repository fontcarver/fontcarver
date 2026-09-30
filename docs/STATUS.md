# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-09-30. Branch: `p5-color-authoring` (uncommitted local
changes; no push or main merge). The user requested finishing the active batch,
then stopping. The active batch is finished and stopped; final integration results and remaining
limits are recorded in [the batch checkpoint](p5-resume-integration.md).

The batch implements variable pen editing, document comparison/paste/merge,
scripting/plugins, interchange/export workflows, shell workspaces, proofing,
glyph/editor/bitmap/CJK workflows, hint preservation and release definitions.
The full frontend regression passes **2,233 tests in 289 files**. TypeScript,
ESLint, Prettier and the production build pass. Rust has 1,710 distinct passes across the full run and
repair rerun, with two existing ignored gates; final Clippy passes. This is substantial implemented
work, not acceptance of every P5 requirement or native platform.

The historical effort estimate remains **about 60%, range 50–70%**, pending a
new weighted assessment; it does not include credit for this batch. See
[estimate](progress-estimate.md) and [evidence reconciliation](p5-resume-audit.md).
[TODO](TODO.md) records the remaining scope for a future explicit resume.

**2026-09-30, later:** the P5 completion drive started
([plan](superpowers/plans/2026-09-30-p5-completion.md)). The resumed batch is
committed (`6539b13`, `040609f`). Wave 1 (shell, inventory, editor, metrics,
features, export, automation, release) is implemented in the working tree but
**not yet integrated or committed**; the user asked to finish it, update the
records and stop. The exact state and the integration to-do list are in
[the wave 1 checkpoint](p5f-wave1-checkpoint.md). Wave 2 is planned, not started.

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

## P5 checkpoint

The [feature audit](p5-feature-audit.md) remains the exit gate. Its three accepted
rows (84, 88, 104) retain their documented scope; other rows are not automatically
closed by a passing test suite. Thirteen historical acceptance candidates are
identified in [the reconciliation](p5-resume-audit.md). New bounded implementations
and direct browser evidence are linked from [the batch checkpoint](p5-resume-integration.md).

MATH, static SVG/COLRv1 authoring, styles, stored masks and numeric smart corners
retain their earlier acceptance records. Variable pen parameters now have real
API/browser editing, bake/undo and native save/reopen evidence. Broader geometry
and independent export acceptance limits remain explicit. Static color glyphs
now render in grid/text previews; variable paints and color proof export remain open.

Release configuration and license/helper checks are implemented. Actual FontCarver
installers and native macOS/Windows workflows remain unverified. Native/global
desktop input automation remains prohibited. The bilingual website is implemented;
its deployment and first release download links remain pending.

The batch used excessive recursive delegation. All delegated implementations are
finished; centralized acceptance supersedes their earlier queued/cancelled-test
notes only where the final report explicitly records a pass. No additional
feature batch is authorized by this stopping checkpoint.

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
