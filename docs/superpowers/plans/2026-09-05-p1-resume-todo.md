# Font Cutter — resume TODO (written 2026-09-05, session paused by the user)

## Current status — P2 paused by the user, 2026-09-07

**Do not start P2 now.** After reviewing the proposed scope, model allocation
and rough estimates, the user explicitly deferred P2 until a later session in
a few days. Wait for a new instruction to resume; elapsed time does not
authorize an automatic restart. No P2 implementation has begun.

The saved [P2 design proposal](../specs/2026-09-07-p2-kerning-design.md) contains
the P2a–P2d scope, first-milestone behavior and acceptance criteria, proposed
Astra/Sol allocation, and tentative estimates. It is a design draft, not an
approved, task-by-task implementation plan.

Resume checklist, to execute only after the user resumes P2:

- [ ] Read the P2 proposal and confirm the first milestone's scope with the user.
- [ ] Write the detailed P2a implementation plan, with exact interfaces and tests.
- [ ] Implement and verify P2a: horizontal kerning for eligible source fonts.
- [ ] Design and plan P2b: editable OpenType layout and imported-font support.
- [ ] Design and plan P2c: static OTF/CFF export.
- [ ] Design and plan P2d: Glyphs/SFD/Type 1 interoperability and problem checks.

P1 remains on `p1-outline-editing`; no merge to `main` or push has occurred.
Performance work, the FontForge comparison and native desktop automation remain
paused. The following continuation and P1 handoff sections are historical.

## Earlier continuation — 2026-09-07

The user has now tried the desktop version, accepted its current usability
subject to possible optimization, and requested the next step. The stop-after-P1
instruction below is historical and superseded. Export performance work and
the FontForge comparison remain deferred at the user's request; native desktop
automation remains paused. P2 design preparation was started; see
`../specs/2026-09-07-p2-kerning-design.md` for the proposed first milestone.
The user trial is not a complete native workflow test or packaging acceptance.

## P1 handoff (2026-09-07)

**This run is finished. Stop after P1; do not start P2.** Implementation,
Chinese localisation, headless acceptance and final whole-branch/scoped review
are complete on `p1-outline-editing`. Production candidate: `2e53ba6`; later
commits record evidence and keep temporary binaries out of ledger snapshots.
The repository is `/home/chen/githubprojects/fontpixel/fontcutter`.

- T7/T8/T9: complete, including shaped text, local glyph editing, preview,
  metrics coalescing and source/master handover.
- Final review's three blockers are fixed and independently approved: save/
  export/close sequencing, ambiguous TTC preservation and unfinished Pen paths
  during toolbar master changes.
- T13: independent Sol translation and separate QA; all 881 keys match.
- Final app checks: TypeScript, ESLint, Prettier; 104 files / 930 tests passed.
- Final headless acceptance: 21 passed, no skips, 40.9 s using current release
  binaries. All three browser gates and actual Source Han grid passed; 300-point
  drag maximum 5.2 ms meets 8 ms.
- Rust: prior integrated 453-test run passed; final nine preservation tests and
  workspace formatting/Clippy passed. Source Han export gate passed narrowly
  at 58.697 s / 60 s; preserve this qualification.
- Final release CLI/server and embedded debug desktop builds passed. Desktop
  artifact retained at `target/p1-acceptance/fontcutter-desktop`.
- **Native/headed input remains paused and unverified** because the user reported
  interference. The final desktop artifact was not launched. Release packaging
  was not tested. No native interaction is authorized by this handoff.
- All 14 clean, merged temporary worktrees were removed after final review;
  source branches and the main working directory remain. No main merge, push,
  branding change, P2 work or historical fixture-history purge was performed.

Model allocation followed the user's request: Sol for bounded implementation,
tests, builds and translation; Astra for complex integration and independent
review. See `docs/p1-acceptance.md` for exact evidence and accepted limitations.
The final ledger is archived in
`docs/superpowers/ledgers/2026-09-03-p1-outline-editing/`; the temporary ignored
SDD workspace is cleaned after its final snapshot. Future work requires a new
user instruction. The sections below are historical records, not active TODOs.

## Historical paused-session record (2026-09-05)

The sections below preserve the original handoff. Paths, model assignments and
unchecked boxes there are historical; the active resume above takes precedence.

## 0. Where things stand

- Branch `p1-outline-editing` (from `main`): Tasks 1, 2 (+ minor round), 3, 4, 5, 6, 7, 9a, 10, 11 merged and verified
  (Rust 438 tests, Vitest 575, e2e 13 passed / 1 skipped).
- Repository now lives at `/home/chen/githubprojects/pixelfonts/fontcutter` (moved 2026-09-05; Claude project data and
  worktrees migrated). Worktree: `.worktrees/p1-t8` (T8 panels/menus, DONE but unreviewed, base 449a390).
- T7 is merged (5bbd934). No agents are running; P1 is paused by the user.
- Dispatch files ready: `task-9b-dispatch.md` (Text tab; fill `BASE_COMMIT`), `task-7-dispatch.md`, `task-8-dispatch.md`.
- Review packages: `scripts/review-package` from the superpowers SDD skill; reviewer prompts follow
  `task-reviewer-prompt.md` / `re-review-prompt.md` (see earlier dispatches in the ledger for the exact shape).

## 0a. Rename (done 2026-09-05, after the pause)

- Product is now **Font Cutter** (`fontcutter`, fontcutter.com): commits f6c5ade (product) and a166c23 (internals: crates
  `fc-*`, CLI `fontcutter`, desktop binary `fontcutter-desktop`, `fc:event`, `FC_API_PORT`/`FC_UI_PORT`/`FC_DEV_PROXY`/
  `FC_CONFIG_DIR`/`VITE_FC_API`, document extension `.fcf`; `.fwf` and the old `"fontworkshop"` format tag are still read).
- **T8 merge implication**: `.worktrees/p1-t8` predates the rename — when merging P1 into p1-t8, apply the same renames to its
  app-side references (`fw:event`, `FW_*`, `.fwf`, "FontWorkshop" strings) before verifying. Remove `.worktrees/p1-t7`.
- Open for the user: desktop binary name; fixture fonts' baked-in "FontWorkshop" family strings (left as is); repo directory
  name still `fontworkshop`; port 1420 is occupied on this machine (use 1428+ for e2e).

## 1. P1 — remaining steps (in order)

- [x] **T7** complete and merged (5bbd934). Deferred T7 minors → do as a small "T7 minor round" (Opus, worktree from the
      P1 head, then Sonnet re-review, merge): (1) `gestureEnded`/`setTool` must repaint; (2) `finishDrag`'s finally must not
      wipe a nudge preview started during a live drag (guard like `closeNudge`); (3) split `curveFit.ts` (538 lines) into
      fit / corners / quadratic conversion; (4) ShapeDialog Enter commits stale field values — mirror the four values in a
      ref; (5) the synchronous fit costs 100–330 ms on pointer-up — cheaper `distanceToCubic` (Newton per sample, fewer
      re-measures) or run it after the first paint; (6) make the fit tolerance zoom-aware (sampling step × factor) until a
      Prefs field exists; also: tool keys should respect `modalCount`; `ttc` with CFF faces inferred quadratic; verify
      Zoom/Hand against §5. Reviewer notes: task-7-rereview1.md.
- [ ] **T8 review** — implementation is DONE (p1-t8 @ 3b1bd4b, 11 commits, Vitest 699, e2e 15/1 skipped; report
      task-8-report.md lists ten kernel-driven ruling adjustments and five interactive-pass fixes). Review package is ready:
      `.superpowers/sdd/2026-09-03-p1-outline-editing/review-449a390..3b1bd4b.diff`. Dispatch an Opus task review (brief +
      task-8-dispatch.md + report + the 7 screenshots `docs/screenshots/p1-panels-*.png`, `p1-edit-sidebar.png`,
      `p1-metrics-handles.png`), fix rounds on the implementer, then merge P1 into p1-t8 (expect an `editorController.ts`
      conflict with T7 — the implementer resolves), verify (Vitest + e2e; note the shared-kernel e2e flake under parallel
      workers: run with `--workers=1` or add one kernel per worker to the Task 11 harness), merge into P1, snapshot.
- [ ] **T9b Text tab + preview strip**: `git worktree add .worktrees/p1-t9b -b p1-t9b <P1 head>`; fill `BASE_COMMIT` in
      task-9b-dispatch.md; ports 8794/1427; Opus implementer → review → fix rounds → merge → snapshot.
- [ ] **T12 acceptance** (Fable-led, measurements by Opus agents where needed): write `docs/p1-acceptance.md` per
      task-12-brief.md — both exit-criterion workflows via e2e + CLI, preservation proof (byte-identical raw tables,
      instructions kept/dropped, shaping fingerprint), export timings (DejaVu < 5 s, Source Han Sans < 60 s release),
      drag budget (≤ 8 ms repaint on a 300-point glyph; `edit_apply` ≤ 4 ms at 20k glyphs release), UFO round-trip corpus,
      desktop smoke. Small defects fixed in their own commits.
- [ ] **T13 zh-CN** (Opus or Sonnet with an explicit `model:`; never Fable): translate every placeholder key listed in
      task-6/7/8/9a/9b/11 reports (~400 keys) with the P0 terminology list; key-parity test; QA pass by a second agent.
- [ ] **Final whole-branch review** (Opus, most capable): `review-package <plan> $(git merge-base main HEAD) HEAD`;
      point it at the ledger's deferred-minor and parked lines; ONE fix wave + one scoped re-review; adjudicate residuals.
- [ ] **Finish branch**: superpowers `finishing-a-development-branch`; merging to `main` is the user's call
      (also decide whether to rewrite history to purge the old GPL fixtures).
- [ ] Remove merged worktrees; `rm -rf .superpowers/sdd/2026-09-03-p1-outline-editing` only after the final review is clean.

## 2. Kernel backlog (schedule as the first P2 kernel round)

- Contour-scoped `PathOp` (additive `contours: Option<Vec<usize>>`) — pencil/knife/menu ops on a selection.
- Apply `remove_overlap` in export (fc-compile calls fc_geom per touched glyph) instead of only reporting.
- Structured `ExportWarning.glyphs` (UI parses message text today); structured `AddGlyphsResult.skipped` reasons.
- `Event::Prefs` on `prefs.set` (UI re-reads prefs on tab mount today); additive `Prefs` fields: hit/snap distances,
  simplify tolerance, tool defaults.
- fc-layout: reverse cmap for `widths_for_text` (full glyph-map scan under the read lock); `catch_unwind` around the
  synchronous first compile; `Wait`er diagnostics (compile-failed vs AbandonedCompile); no rebuild for an older generation.
- fc-doc: `declared_curve_format` for mixed-curve layers; `delete_points`/`inferred_kind` residuals are done.
- Content-hash touched-glyph net when `.fcf` re-links original sources; gasp raw overwrite vs model edits; format-4 cmap
  truncation warning; asymmetric language tables; per-master guide identifiers; CSP only enforced in production builds.
- Playwright: detect a stale `fc-devserver` binary (mtime vs sources) or default to `cargo run` locally.

## 3. UI backlog (final-review triage list)

- Component filled ghost lags markers during a drag (needs per-component preview paths — partly done in T7).
- Live Info box not memoised; `drag/index.ts`/`editorController.ts` near the size caps; `GlyphView`/`GlyphGrid` test flake
  under load (worker cap 50 %).
- Export "Reveal in folder" (needs an opener plugin); Add Glyphs Enter only from the list; `Mod+,` not in §11 (accepted).
- Toast stack cap/width done; global shortcuts under modals done; `reuseExistingServer` may reuse a fixture-less kernel.

## 4. After P1 (each phase: Fable writes the plan with `writing-plans`, then SDD execution)

- **P2 Spacing, kerning, features, CFF**: kerning groups/panel/text-tab kerning; metrics keys UI; FEA editor with
  diagnostics; layout decompiler (GSUB/GPOS → FEA so imported fonts shape with features); automatic features; CFF/OTF
  export (own CFF writer); Glyphs import/export; SFD import/export; Type 1 read/write; Find problems v1; plus the kernel
  backlog above. Exit: professional Latin workflow; Glyphs/UFO/SFD round trips pass corpus tests.
- **P3 Bitmap fonts**: strikes model + editor; BDF/PCF/FNT/FON/PSF/OTB/EBDT/CBDT/sbix read/write; rasterise/trace;
  waterfall. Exit: replaces bitmap-font-viewer; CJK BDF workflow complete.
- **P4 Variable & CJK pro**: axes/masters/instances UI; interpolation preview; compatibility fixer; brace/bracket layers;
  variable export (gvar/CFF2); smart components; vertical metrics/`vert`/`vkrn`; Unihan filters & batch tools; CID export.
- **P5 Professional finish**: hinting (PS autohint, TT editor, ttfautohint), MATH, COLR/CPAL + SVG colour, style effects,
  compare/merge, autotrace, scripting API, plugins, remaining formats, installers (deb/AppImage/dmg/msi), docs.
  Exit: research doc §12 parity checklist green for Must/Should-have.

## 6. Decisions for the user (nothing proceeds on these until you say so)

- [ ] Desktop binary name: the Tauri shell builds `fontcutter-desktop` because the CLI owns `fontcutter`. Keep, or pick another
      name (e.g. CLI `fontcutter`, shell `Font Cutter.app`/`fontcutter-app`)?
- [ ] Test fixture fonts still carry "FontWorkshop" family names inside the binaries (rebuilding changes committed hashes).
      Recommendation: leave them.
- [ ] Git history still contains the three old GPL/AFPL fixture fonts (removed in 3acdc1c). Rewrite history before the repo
      goes public, or accept? (Rewrite is destructive; needs your explicit go.)
- [ ] Merge `p1-outline-editing` into `main` when P1 is finished (the finishing step asks; it is your call).
- [ ] Codex: currently paused (all seats on Claude). Say when to re-enable it (kernel implementation, fix rounds, T13 translation + QA go back to Codex sol xhigh under the original split).
- [ ] Repository URL / hosting for fontcutter.com (Cargo/package `repository` fields are unset).

## 7. Environment notes (this machine)

- Port 1420 is held by another Vite server ("One True PDF"); run the main tree's e2e with `FC_UI_PORT=1428` (or higher) and
  `FC_API_PORT=8787`; worktrees use 8788+/1421+ pairs as recorded in their dispatch files.
- Before running anything in the main tree after a merge: `pnpm --dir app install --frozen-lockfile` and
  `pnpm --dir app gen:api` (generated bindings are gitignored and go stale), and delete any stale `target/release/fc-devserver`
  (Playwright prefers the release binary).
- Resume this Claude session from the new directory: `claude`, then
  `/resume` (fallback: `claude --resume f0187d23-6841-4655-bad5-89d6bc891ebe`).

## 5. Rules to keep

- Product name Font Cutter (`fontcutter`, fontcutter.com); crates `fc-*`, CLI `fontcutter`, `.fcf` documents.
- Docs/comments in English; reply to the user in their language.
- License MIT OR Apache-2.0; only 3-clause-BSD FontForge files may be adapted (attribution in THIRD_PARTY.md); no copyleft deps.
- Every task: written dispatch with rulings, report file, review package, cross-review; ≤ 5 fix rounds then adjudicate.
- Persist state after every merge/milestone (`scripts/sdd-snapshot.sh`); keep the memory resume pointer current.
