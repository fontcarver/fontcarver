# FontCarver status

The single entry point for "where is the project". Update this file at every
pause and every milestone commit. It links to the detailed records; it does
not repeat them.

Last updated: 2026-10-15.

**0.1.0-alpha.1 is published.** The first packaged release is on the
[releases page](https://github.com/fontcarver/fontcarver/releases/tag/v0.1.0-alpha.1):
Linux deb, rpm and AppImage, a macOS disk image for Apple silicon, and a
Windows installer and portable build, each built and verified on the system it
targets by [`release.yml`](../.github/workflows/release.yml) and published with
the digest its own inspection recorded. Nothing is signed or notarized, and the
native acceptance checklist in [releasing.md](releasing.md) is not completed;
that is what makes it an alpha. Everything below this line predates the
release and is unchanged.

Previously: 2026-10-04. Branch: `p5-color-authoring`. Waves 1 and 2 and
this repair pass are local work; no push, `main` merge, signing or publication.

**The authorized local closing work and added UI/UX tasks are complete.**
The actual browser walkthrough led to compact-layout, outline/bitmap gesture,
shortcut and focus repairs. The user's one additional small task adds welcome
links to getting-started help, command search and shortcut settings, with live
key bindings and independently reviewed Chinese wording. See the
[experience record](p5ux-experience-acceptance.md) and
[welcome walkthrough](p5ux-start-discovery-browser-acceptance.md).
The later documentation task adds an
[illustrated welcome guide](user/context-help.md#start-from-the-welcome-screen),
reusing those accepted captures with entry labels and a keyboard-search example.
Documentation verification passed: visual inspection of the reused captures,
105 local links/anchors, image alternative text, Prettier, whitespace and resource
checks. Application/tooling inputs are unchanged; no application tests were rerun
for this documentation task.

The earlier bounded follow-up verifies welcome and command palette at actual
200% browser zoom in English/Chinese and light/dark, repairs clipped command
names and adds four passing reflow cases. See [zoom acceptance](p5ux-zoom-acceptance.md).
The subsequent [menu repair](p5ux-menu-zoom-acceptance.md) closes the separate English
Help overflow found there. All menu titles and popups fit at 200%, including
live resizing to a narrower window; English/Chinese × light/dark pass actual
browser checks. Four new menu regressions and the four welcome/palette cases
pass together, with zero retries. The full frontend rerun passes 2,914 tests /
348 files, type/lint/format and build. Its private browser/servers and 28.9 MB
of temporary data are cleaned; resource check passes at 5.33 GiB build,
0.00 GiB temporary and 616.21 GiB free.

The subsequent [settings check](p5ux-settings-zoom-acceptance.md) fixes overflowing
nudge fields in Preferences by letting them wrap. Actual 200% checks pass in
English/Chinese × light/dark, including all three preferences categories,
shortcut-list scrolling, conflict feedback, footer controls, Tab/Escape and
focus return. Four new browser regressions pass at equivalent 200%, narrower
and ordinary CSS widths, with zero retries. The latest full frontend run passes
2,914 tests / 348 files, type/lint/format and build. Its browser/servers and
39.0 MB of task data are cleaned; resource check passes at 5.33 GiB build,
0.00 GiB temporary and 611.91 GiB free. Existing Linux candidates below
predate these layout repairs and remain the earlier package checkpoint.

The subsequent [workspace zoom check](p5ux-workspace-zoom-acceptance.md) extends actual
200% coverage to glyph editing, sidebar/Inspector, Font Info, metadata and Export.
Narrow windows retain the full editor width and open side panels temporarily;
Font Info navigation wraps, status/node-info overflow stays within its own area,
and metadata Apply retains keyboard focus. Eight actual browser combinations
(English/Chinese × light/dark × two window widths) pass, including node dragging,
undo and TTF export. The final targeted browser regression passes **23/23**, with
zero failures, retries or skips. Frontend validation passes **2,917 tests /
348 files**, type/lint/format and build; screenshots and commands are linked in
the acceptance report.
No new translation keys or dependencies were added. The 112-row audit and broad
effort estimate remain unchanged. Owned browser/servers and task scratch were
removed. The resource gate passes at **5.33 GiB build / 12,262 files,
0.00 GiB temporary / 2 files, 533.55 GiB free**. Work stops after this selected task.

The latest [complete E2E follow-up](p5ux-latest-e2e-acceptance.md) covers the current
source, including all 20 added zoom cases: **125/125 pass**, zero failures, skips
or retries in the final run. Two earlier complete runs each exposed one gesture
test setup/synchronization gap; both were corrected and each affected case passed
three focused repetitions before the final run. Application code and performance
limits were unchanged. The 300-point drag gate records **1.4 ms P95, 2.6 ms maximum,
4.5 ms pointer-down**, all below 8 ms; the actual 65,535-glyph CJK case passes.
The report retains all three full-run results, source hashes and performance
samples. Type checking and changed-test lint/format checks pass. The prior
2,917-test frontend/build checkpoint remains current; no Rust or installer build
was needed for this E2E-only change. Resource cleanup is recorded in the report.
The next local delivery step remains rebuilding/verifying Linux installers with
the recent UI fixes. This selected E2E task is complete.

The prior integrated application/tooling source **`f6f6cf9`** passes **2,914 frontend tests /
348 files**, type checking, lint, formatting and production build. The final
[browser regression](p5ux-browser-acceptance.md) passes **105/105**, with zero
failures, skips or retries and the real external CJK font enabled. Its maximum
measured edit time is **6.5 ms**, below the unchanged 8 ms gate. Final Linux
packages from this source passed inspection, actual installation/startup and
independent review. Publication, signing and macOS/Windows execution remain
outside this batch.

The original wave 3 [integration acceptance](p5f-w3-integration-acceptance.md) records **2,093 Rust
passes**, **2,836 frontend passes** and **100/100 default E2E passes**, with no E2E
skips or retries and the real external CJK font enabled. Four explicitly ignored
Rust cases were covered separately by release and real-hinter acceptance. Earlier
failed/interrupted attempts are retained, including the first E2E timing outlier;
the original 8 ms threshold was not changed.

The [TODO](TODO.md) is the authoritative next-task list. The
[feature audit](p5-feature-audit.md) remains **110 Complete / 2 Partial / 0 Pending
verification**, within each acceptance's boundaries. Rows 9 and 62 still lack
native macOS/Windows execution. The broader effort [estimate](progress-estimate.md)
is **about 79%, weighted band 72–86%**; this is separate from the checklist count.

The earlier metrics-key move, independent English/Chinese wording review and
per-test kernel isolation are committed as `c1bd334`, `a4be04a` and `86770ab`.
The parallel repair pass then completed real ttfautohint verification, restored
large/exact spacing shortcuts, added native bitmap PNG saving and vertical pixel
proofing, corrected stale CJK edits and width previews, and made Fix All complete
safe repairs within one token-scoped undo/cancel operation. Independent audit
covered all 112 rows and corrected stale Type 42/bare-CFF and editor-wiring claims.
Separate GPT-6.1 Sol translation reviewed 12 changed/new values; the catalogues
contained **3,943 matching keys** at that checkpoint. Both added UX and welcome
deltas were separately reviewed by GPT-6.1 Sol; the current catalogues have
**3,945 matching keys**.

The final actual candidates were built from **`f6f6cf9`**, including the bundle
marker correction in `29c97d2`. The superseded first `6e35f70` round is
[archived](acceptance/p5f-w3-packaging/round-1-6e35f70/README.md).
The [manifest](acceptance/p5f-w3-packaging/release-manifest.json)
identifies all **2,008 source inputs** and both artifact hashes. Independent review
verified both payloads, 630 dependency packages / 1,126 license files, the runtime
pins and zero bundled shared libraries in the thin image. Real isolated APT
install/reinstall/remove retained user data and left the host installation
unchanged. Both AppImage startup modes pass; its graphical process survives the
15-second isolated-display check, which does not establish interactive desktop
behavior. The sole executable payload difference is the expected three-byte
`DEB` to `APP` marker conversion. See
[packaging](p5f-w3-packaging-acceptance.md),
[runtime](p5f-w3-thin-appimage-acceptance.md) and
[independent review](p5f-w3-review-release.md). These unsigned local candidates
use compatible host libraries; other distributions and native platforms are
not established by this evidence.

Final cleanup stopped owned browser/server/probe processes and removed their
scratch directories, including old candidate copies. Retained final packages
remain under `target/release/bundle/`. The resource gate passes: **5.33 GiB build /
12,262 files, 0.00 GiB temporary / 1 file, 616.36 GiB free**. Subsequent records
commits for that milestone changed documentation only; the package manifest identifies the
application/tooling source above.
The [independent final records review](p5ux-final-records-review.md) accepted
the current status, backlog, audit counts and source/artifact evidence.

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
11/11, `e2e/p2b.config.ts` 17/17, `e2e/p2cd.config.ts` 8/8. These are the
historical wave 2 results; the 2026-10-02 follow-up and its current limits are
recorded in the [isolation acceptance](p5f-e2e-isolation-acceptance.md).

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
states, not exhaustive format coverage. Native desktop input automation remains prohibited. Current Linux
installer acceptance uses the actual integrated candidates; historical packages
are not promoted by the new tooling or by a matching filename.

## P5 checkpoint

The [feature audit](p5-feature-audit.md) remains the exit gate: **110 Complete,
2 Partial, 0 Pending verification**. Rows 84, 88 and 104 retain their earlier
documented scope; the 107 rows accepted across the two waves each carry an
acceptance document, a review and a fix round. The two Partial rows are **9**
(no native macOS or Windows execution) and **62** (installation on those
platforms is unverified). The old AppImage fails the new policy gate; the actual
replacement and deb now have integrated build, inspection, installation/startup
and independent-review evidence. Thirteen historical acceptance candidates are identified in
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

Release configuration, license/helper checks and actual Linux candidate
verification are complete. Native macOS/Windows workflows remain unverified. Native/global
desktop input automation remains prohibited. The bilingual website is implemented;
its deployment and first release download links remain pending.

All local delegated implementations are integrated. Centralized acceptance
supersedes earlier queued/cancelled-test notes only where the final report
explicitly records a pass. Wave 3 local results and the remaining delivery gates
are recorded in [TODO](TODO.md).

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
