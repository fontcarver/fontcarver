# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-09-30. Branch: `p5-color-authoring` (committed locally; no
push, no `main` merge). The user requested finishing the work in hand, updating
the progress records and stopping. Completion wave 1 is finished, verified,
recorded and committed; wave 2 is planned and **not started**.

The batch implements variable pen editing, document comparison/paste/merge,
scripting/plugins, interchange/export workflows, shell workspaces, proofing,
glyph/editor/bitmap/CJK workflows, hint preservation and release definitions.
The full frontend regression passes **2,233 tests in 289 files**. TypeScript,
ESLint, Prettier and the production build pass. Rust has 1,710 distinct passes across the full run and
repair rerun, with two existing ignored gates; final Clippy passes. This is substantial implemented
work, not acceptance of every P5 requirement or native platform.

That batch's own effort estimate was **about 60%, range 50–70%**; after wave 1 the
[estimate](progress-estimate.md) is reassessed at **about 70%, range 60–80%**, and
[TODO](TODO.md) records the remaining scope.

**2026-09-30, later:** the P5 completion drive started
([plan](superpowers/plans/2026-09-30-p5-completion.md)) and its **wave 1 is
integrated**. Eight area agents (shell, glyph inventory, outline editor,
metrics/spacing/kerning, OpenType features, font info/design space/export/CLI,
cross-document/scripting/plugins, release packaging) implemented their rows, were
reviewed ("accept with fixes" in every case), completed their fix rounds, and a
zh-CN translation pass followed ([pass 1](p5f-localization-pass1.md),
[pass 2](p5f-localization-pass2.md)). The integration commits are
`a4b701d` (code) and the `docs(p5)` commit that follows it; the resumed
batch baseline is `6539b13` / `040609f`.

Integrated verification, all run in one pass on this checkout:
`cargo fmt --all` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `cargo test --workspace --no-fail-fast` **1,978 passed, 0 failed, 2
ignored** over 271 test binaries; `pnpm check` (tsc) and `pnpm lint` (ESLint)
clean; `prettier --check .` clean; Vitest **2,517 tests in 314 files**, all
passing, including the zh-CN parity test (3,524 keys each side); the production
build succeeds. Playwright: the default configuration runs its 81 specs on one worker and skips the CJK spec, whose corpus lives outside the repository. The last two full runs passed 79 and 78 of the 80 that execute; the cases that come and go are `text.spec.ts:122` and two P2b specs, all three recorded in [TODO](TODO.md). The three dedicated configurations pass in full: `e2e/p2a.config.ts` 11/11, `e2e/p2b.config.ts` 17/17, `e2e/p2cd.config.ts` 8/8.

The [feature audit](p5-feature-audit.md) now reads **75 Complete, 5 Partial, 32
Pending verification** of 112 rows, from 3/3/106 before the wave; every accepted
row links its acceptance document. The effort estimate is reassessed at **about
70%, range 60–80%** ([estimate](progress-estimate.md)). Per-area detail lives in
`docs/p5f-{a,b,c,d,e,f,i,n}-*-acceptance.md`, the state of the wave while it ran
in [the wave 1 checkpoint](p5f-wave1-checkpoint.md), and the remaining scope in
[TODO](TODO.md). Wave 2 (bitmap, proofing, construction, hinting, CJK, help) is
planned and not started.

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

The [feature audit](p5-feature-audit.md) remains the exit gate: **75 Complete, 5
Partial, 32 Pending verification**. Rows 84, 88 and 104 retain their earlier
documented scope; the 72 rows accepted in wave 1 each carry an acceptance
document, a review and a fix round, and no row is closed by a passing test suite
alone. The five Partial rows are 9 and 62 (no native macOS or Windows execution;
AppImage payload licensing unresolved), 61 and 98 (hinting, wave 2 task `k`) and
87 (non-destructive strokes, wave 2 task `j`). Thirteen historical acceptance
candidates are identified in [the reconciliation](p5-resume-audit.md); new bounded
implementations and browser evidence are linked from [the batch
checkpoint](p5-resume-integration.md).

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
