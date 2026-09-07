# Font Cutter — resume TODO (written 2026-09-05, session paused by the user)

## Active resume (2026-09-07)

The user resumed P1 in Codex and requested **Sol for bounded implementation,
verification and translation; Astra for complex architecture and independent
review**. This supersedes the historical paused-Codex and Claude-only assignments
below. The repository now lives at
`/home/chen/githubprojects/fontpixel/fontcutter`; the moved T8 worktree has been
repaired. Current dispatches and evidence are recorded in the ledger's
`resume-2026-09-07.md`.

- T8: independent Astra review in progress; integration follows fixes and review.
- T7 minor round: Sol implementation in `.worktrees/p1-t7-minors`.
- T9b: Sol builds the independent parsing/layout/request foundation in
  `.worktrees/p1-t9b`; UI integration waits for the reviewed T8/T7 base.
- Then: T12 acceptance, T13 translation and independent QA, whole-branch review.

Authoritative state lives in the SDD ledger `.superpowers/sdd/2026-09-03-p1-outline-editing/progress.md`
(snapshot copies in `docs/superpowers/ledgers/2026-09-03-p1-outline-editing/`). This file is the human-readable
checklist for the next session. Process: superpowers `subagent-driven-development` (Fable orchestrates only;
Opus implements/reviews; Sonnet only for small re-reviews; Codex paused until the user re-enables it;
translation never on Fable). Run `scripts/sdd-snapshot.sh` after every merge.

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
- [ ] Codex: currently paused (all seats on Claude). Say when to re-enable it (kernel implementation, fix rounds, T13 translation
      + QA go back to Codex sol xhigh under the original split).
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
