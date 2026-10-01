# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-10-01. Branch: `p5-color-authoring` (committed locally; no
push, no `main` merge). Completion waves 1 and 2 of the
[P5 completion plan](superpowers/plans/2026-09-30-p5-completion.md) are
finished, verified, recorded and committed. What remains of P5 is wave 3:
the two Partial rows, the release rebuild and its licensing, native macOS and
Windows execution, and the final row-by-row gate — all recorded in
[TODO](TODO.md).

The [feature audit](p5-feature-audit.md), the P5 exit gate, now reads
**110 Complete, 2 Partial, 0 Pending verification** of 112 rows, from
75 / 5 / 32 after wave 1 and 3 / 3 / 106 before it. Every accepted row links an
acceptance document carrying its commands, browser evidence and limits; no row
is closed by a passing test suite alone. The effort
[estimate](progress-estimate.md) is reassessed at **about 77%, range 70–85%**.

**Wave 1 (2026-09-30)** — shell, glyph inventory, outline editor,
metrics/spacing/kerning, OpenType features, font info/design space/export/CLI,
cross-document/scripting/plugins and release packaging. Eight area agents
implemented their rows, were reviewed ("accept with fixes" in every case),
completed their fix rounds, and a two-pass zh-CN translation followed
([1](p5f-localization-pass1.md), [2](p5f-localization-pass2.md)). Integration
commits `a4b701d` (code) and `976d213` (docs); the resumed batch baseline is
`6539b13` / `040609f`.

**Wave 2 (2026-10-01)** — bitmap fonts, preview/proofing, construction and
images, hinting completion, CJK completion, and help/keymaps/theming/accessibility.
Six area agents (`docs/p5f-{g,h,j,k,l,m}-*-acceptance.md`) implemented rows 19,
61, 64–76, 80, 82–83, 85–87, 92–95, 97–98, 101–103 and 107–112, were reviewed
("accept with fixes" in every case: 6 + 5 + 7 + 7 + 5 + 5 review items, plus a
`VerticalMetricsPanel` stale-override defect task l's own new e2e case found),
completed their fix rounds, and a two-pass zh-CN translation followed
([1](p5f-localization-wave2-pass1.md), [2](p5f-localization-wave2-pass2.md);
the Chinese catalogue is 3,936 keys against the same English set). The
integration commits are `9b72af8` (code) and the `docs(p5)` commit
that follows it, and the wave is summarised in
[the wave 2 checkpoint](p5f-wave2-checkpoint.md).

Integrated verification of wave 2, all run in one pass on this checkout:
`cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `cargo test --workspace --no-fail-fast` **2,086 passed, 0 failed, 2
ignored** over 277 test binaries; `pnpm check` (tsc) and `pnpm lint` (ESLint)
clean; `prettier --check .` clean; Vitest **2,775 tests in 339 files**, all
passing, including the zh-CN parity test (3,936 keys each side); the production
build succeeds.
Playwright: the default configuration holds 95 specs, runs them on one worker
and skips the CJK spec whose corpus lives outside the repository. Two full runs
were made, the second after the fixes above, and each passed **91 of the 94 that
execute** — but on a different three. The first run failed
`app.spec.ts:217` and `p2a-kerning.spec.ts:546` (both fixed here, and both pass
in a targeted rerun) and `p2b-imported.spec.ts:75`; the second failed
`fontview.spec.ts:303` (a grid cell detached during virtualisation),
`panels.spec.ts:108` (an Edit tab not yet created) and `p2b-source.spec.ts:26`
(a draft read back undefined). All three of the latter passed in the other run;
they are shared-document and virtualisation timing failures of that one
configuration, recorded in [TODO](TODO.md), not product defects demonstrated
twice. The three dedicated configurations pass in full: `e2e/p2a.config.ts`
11/11, `e2e/p2b.config.ts` 17/17, `e2e/p2cd.config.ts` 8/8.

Two regressions were found and fixed while integrating, both where a wave 2
change met another task's test: the File menu's disabled rows carry their reason
as `aria-description` and an inline hint now that Quick Help owns hovering, so
`App.test.tsx` asserts that instead of the dropped native `title`; and
`useFontEditorSettings.test.ts` read the hook's value in the same tick as the
refetch it had just triggered, which the hook answers one microtask later, so
the assertion is now awaited. `KerningTable.test.tsx` was updated to task m's
"Group" wording.

The batch that preceded the waves implements variable pen editing, document
comparison/paste/merge, scripting/plugins, interchange/export workflows, shell
workspaces, proofing, glyph/editor/bitmap/CJK workflows, hint preservation and
release definitions. Its own effort estimate was **about 60%, range 50–70%**.

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

The [feature audit](p5-feature-audit.md) remains the exit gate: **110 Complete,
2 Partial, 0 Pending verification**. Rows 84, 88 and 104 retain their earlier
documented scope; the 107 rows accepted across the two waves each carry an
acceptance document, a review and a fix round. The two Partial rows are **9**
(no native macOS or Windows execution) and **62** (the same, plus the AppImage
copyleft payload licensing and an `apt install` that has only been simulated by
extraction). Thirteen historical acceptance candidates are identified in
[the reconciliation](p5-resume-audit.md); new bounded implementations and
browser evidence are linked from [the batch
checkpoint](p5-resume-integration.md).

MATH, static SVG/COLRv1 authoring, styles, stored masks and numeric smart corners
retain their earlier acceptance records. Variable pen parameters now have real
API/browser editing, bake/undo, native save/reopen and independent stroker
evidence in TTF and OTF (row 87). Static color glyphs render in grid/text
previews and in the proofing surfaces; variable paints and color proof sheets
remain open. The limits each wave 2 acceptance document states — one brightness
threshold for autotrace, sampled optical spacing, affine attachment placement,
static-only TrueType glyph programs, no bundled IDS database — are listed in
[TODO](TODO.md).

Release configuration and license/helper checks are implemented. Actual FontCarver
installers and native macOS/Windows workflows remain unverified. Native/global
desktop input automation remains prohibited. The bilingual website is implemented;
its deployment and first release download links remain pending.

The batch used excessive recursive delegation. All delegated implementations are
finished; centralized acceptance supersedes their earlier queued/cancelled-test
notes only where the final report explicitly records a pass. Wave 3 is scoped in
[TODO](TODO.md) and is not started.

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
