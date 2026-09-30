# P5 completion wave 1 checkpoint

Written 2026-09-30 by the orchestrator while wave 1 was still running, because
the session's usage budget was about to run out. Everything below is the state
at that moment; the working tree holds the uncommitted wave 1 work of eight
area agents. Nothing is pushed. If this file is the newest record, resume by
reading it, `git status`, and the `docs/p5f-*-acceptance.md` files.

Plan: `superpowers/plans/2026-09-30-p5-completion.md`. Rules every agent
followed: `superpowers/plans/tasks/p5f-protocol.md`. The user asked to finish
the work in hand, update the progress records and stop; wave 2 (tasks g, h, j,
k, l, m) is written but must **not** start until the user resumes.

## Committed baseline

`6539b13` (resumed batch code), `040609f` (its docs), `449e753` (this plan).
Verified before commit: Rust 1,710 passed / 0 failed / 2 ignored; frontend
289 files / 2,233 tests; tsc, eslint, prettier, production build clean.

## Wave 1 state (uncommitted, in the working tree)

| Task | Rows               | Implementation                                     | Review                                                               | Fix round                            |
| ---- | ------------------ | -------------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------ |
| n    | 9, 62, release     | done, `p5f-n-release-acceptance.md`                | accept with fixes (9 items)                                          | done, 37/37 release tests            |
| a    | 1–9                | done, `p5f-a-shell-acceptance.md`                  | accept with fixes (11 items; palette order, torn Inspector blocking) | running when this was written        |
| d    | 37–45              | done, `p5f-d-metrics-acceptance.md`                | accept with fixes (7 items)                                          | done, plus zero-exception regression |
| i    | 77–79, 96, 105–106 | done, `p5f-i-automation-acceptance.md`             | accept with fixes (9 items; read-only and transaction leak)          | done, all nine with tests            |
| e    | 46–51, 72, 99, 100 | done, `p5f-e-features-acceptance.md`               | accept with fixes (8 items; automatic switch data loss)              | running when this was written        |
| c    | 20–36, 81, 89–91   | done, `p5f-c-editor-acceptance.md`; row 33 partial | review running when this was written                                 | not started                          |
| b    | 10–18              | done, `p5f-b-inventory-acceptance.md`              | review running when this was written                                 | not started                          |
| f    | 52–60, 63          | running when this was written                      | not started                                                          | not started                          |

Per-row recommendations live in each acceptance document; the audit table
(`p5-feature-audit.md`) has **not** been updated yet.

## Integration to-do (for the integration pass that closes the wave)

1. Translation: new English keys in `app/src/i18n/en.json` need zh-CN entries
   (namespaces reported: `history`, `shellWindow`, `workspace`, `spacingMatrix`,
   `kerningTable`, `panel.metrics`, `features.*`, `automation`, `plugins`,
   `crossDocument`, `tool`, `editor.audit`, `editor.undo`, `cmd.*`, `panel.layers`,
   `infoBox`, `editorMetadata`, `dialog.prefs.snapTarget*`, `fontView`, `sidebar`,
   `smartFilters`, `inspector.sortMode`, `toast`, `dialog.addGlyphs`,
   `dialog.batchRename`, plus task f's). `src/i18n/i18n.test.ts` fails until then.
   Translation is done by a separate translation agent, never by feature authors.
2. `pnpm gen:api` (`crates/fc-api/src/bin/gen_ts.rs`) wipes
   `app/src/api/generated/` before rewriting; generate into a temporary
   directory and swap atomically so a running Vite never serves an empty dir.
3. Remove the stray untracked file `--full-page` in the repository root (an
   agent-browser screenshot misfire); `app/test-results/` is disposable.
4. `app/playwright.config.ts`: two specs sharing one kernel document time out
   with two workers; consider `workers: 1` / `fullyParallel: false`.
5. Known clippy failures at the time of writing: `crates/fc-api/src/handlers/variation.rs`
   (`manual_flatten`, three sites) and `crates/fc-api/tests/rescale_upm.rs:69`
   (`needless_update`), both task f's; task f was told.
6. Pre-existing prettier drift at HEAD in `crates/fc-doc/src/fcf/schema.md`
   and `THIRD_PARTY.md` tables (not introduced by wave 1).
7. Pen tool Playwright spec: `p` does not select the Pen under Playwright
   (pre-existing); the editor reviewer was asked to diagnose.
8. Then: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `pnpm check`, `pnpm lint`, `prettier --check .`,
   `pnpm test`, `pnpm build`, `pnpm e2e`; fix regressions; update
   `p5-feature-audit.md` rows from the acceptance documents; update
   `STATUS.md`, `TODO.md`, `progress-estimate.md`; commit per area or as one
   integration commit; run `python3 scripts/check_artifacts.py` (the
   orchestrator cleaned the shared cache twice with per-member `cargo clean`).

## Release facts

First real Linux packages of the current code were built into
`/home/chen/.cache/fontcarver-release-target` (deb 23.3 MB, AppImage 95.8 MB);
evidence in `acceptance/p5f-release/`. The AppImage bundles 73 copyleft-licensed
shared objects with notices for only 10; prefer the deb until that is cleared.
macOS and Windows workflows are reviewed but unexecuted.

## Cut-off note

The orchestrating session hit its usage limit right after this checkpoint was
written. At that moment five agents were still running and may have been cut
off mid-edit: task f (export, implementing), the reviews of tasks c and b, and
the fix rounds of tasks a and e. On resume, expect their files to be in an
intermediate state: run `pnpm --dir app check` and `cargo check --workspace`
first, read the newest `docs/p5f-*-acceptance.md` and task-file gap analyses,
and let the integration agent finish or revert only what does not compile.

## Review findings not yet fixed (recorded after the cut-off)

Task e's fix round finished (all eight items, Playwright p2b 17/17). Task b's
review (ACCEPT WITH FIXES) reported these open defects; they are **not fixed**:

1. **Blocking, data loss (row 16):** Update Glyph Info over a selection
   destroys encodings. `crates/fc-doc/src/command/glyph_data.rs:44` writes
   `glyph.unicodes = info.unicodes` unconditionally; `crates/fc-glyphdata/src/database.rs:153`
   clears encodings for any suffixed name; `data/aglfn.txt:210` still maps
   `Omega` to U+2126. Live on DejaVu: 25 PUA glyphs lost their cmap and Omega was
   re-encoded U+03A9→U+2126. Fix: write unicodes only from an explicit record,
   never clear an existing encoding; correct the AGLFN line.
2. **Blocking (row 17):** Add Glyphs dialog is 710 px tall with no scroll
   (`app/src/shell/dialog.module.css:3-19` lacks `max-height`/`overflow`), so
   the confirm button is off-screen on short windows; this, not `gen:api`,
   fails the e2e case.
3. `app/e2e/fontview.spec.ts:316` matches `/^Batch rename/` but the menu item is
   "Batch Rename Glyphs…", so that case can never pass.
4. `crates/fc-doc/src/command/glyphs.rs:227` lists `markClass` in
   `FEA_NAMING_KEYWORDS`, so `markClass acutecomb <anchor…>` is not rewritten on
   rename; remove it.
5. `handlers/glyph_status.rs:36` computes `has_custom_glyph_info` with
   `GlyphDataOverlay::default()` instead of the document overlay.
6. Low: `sidebar.generateMissing` says "missing letters" for every row and
   prints an unformatted count; `GlyphSearch.tsx:131` prints a raw kernel
   message untranslated; the builder-vocabulary drift test is a hand-written
   list; `documentStore.tableColumns`/`moveTableColumn` are dead code.

Task c's review and task a's fix round had not reported when the session ended.

Task c's review (ACCEPT WITH FIXES) reported these open defects; **not fixed**:

1. **Blocking (row 35):** Fix All inserts a node off the extremum.
   `app/src/views/glyph/outlineAudit.ts:397-402` sorts `insertExtremum` fixes by
   segment and `t`, but `InsertPointOnSegment` reparameterises the segment, so
   a second insertion into the same segment uses a stale `t` (measured 3.3 units
   off; the audit re-flags it). Fix: key the "one structural fix per contour"
   grouping by `(contour, segment)` or rescale the remaining `t` after a split.
2. **Blocking (rows 33/89):** `Command::AlignComponents` bypasses the component
   lock (no arm in `crates/fc-doc/src/command/component_locks.rs` preflight,
   falls through at :157, rewrites a locked component's transform at :206);
   `layerCommands.ts:288-294` enables the menu when one component is unlocked
   then aligns all. Add a preflight arm returning `locked_error()` or skip
   locked components.
3. Minor: chained composites align to a stale transform
   (`component_locks.rs:310-330` reads the pre-pass clone); unstable comparator
   at `outlineAudit.ts:398`; `ALL_OUTLINE_AUDIT_TESTS` aliases
   `DEFAULT_OUTLINE_AUDIT` (:40-41); the audit debounce has no max-wait, so
   markers lag during a continuous drag; locked-path drag/nudge is silent
   because selection is refused first; the modifier docs never say Linux Ctrl
   is both ×100 nudge and snap-suspend.
4. Pen Playwright spec: `p` is not broken for users. At 1280×720 the Edit tab's
   spacing/kerning chrome squeezes the canvas to ~140 px and the Info box
   overlays its top-left, so the spec's click lands in an editable field and
   the key is dropped. At 1280×577 the canvas height is 0 (a real layout
   defect). Fix the spec to click the canvas centre; fix the layout.
   Row 35 downgrades to Partial; row 33 stays Partial.

Task a's fix round finished after the cut-off note: all eleven review items
fixed (palette keyboard order; detached Edit-tab panels via
`DetachedGlyphSource`; `aria-keyshortcuts`/pinned exposure; translated history
layer roles; per-glyph COLR/SVG/MATH scopes; all-or-nothing `replay_to`; panic
guard limited to `Builder::build()`); Playwright `app.spec.ts panels.spec.ts`
12 passed, p2a 10/11. The duplicate-Overview report was a P2a case that opens
one file twice on purpose; behaviour pinned by tests. Still open, not task a's:
`app/e2e/p2a-kerning.spec.ts:322` selects the tab by exact name `Regular.ufo`
while an unlabelled Overview renders as `Overview · <file>`; match
`(?:^|· )Regular\.ufo$`. Only task f (export) was still running at the end.

Update: the usage limit reset the same day. The export agent (cut off while
adding a Playwright spec for the profile round trip) was resumed, and the
inventory and editor agents received their review defect lists as fix rounds.
The wave is being finished as planned: export review, fix rounds, zh-CN
translation, integration, records, commits, then stop.

## Closed, 2026-09-30

The wave is finished. Export review and every fix round reported, both
translation passes ran ([1](p5f-localization-pass1.md),
[2](p5f-localization-pass2.md)), and the integration pass verified the tree
(`cargo fmt`, Clippy, `cargo test --workspace` 1,978 passed / 0 failed / 2
ignored, `tsc`, ESLint, Prettier, Vitest 2,517 tests in 314 files, production
build, Playwright per configuration), made the small fixes listed in
[the integration task](superpowers/plans/tasks/p5f-integration.md), updated
[the audit](p5-feature-audit.md) to 75 Complete / 5 Partial / 32 Pending
verification, [STATUS](STATUS.md), [TODO](TODO.md) and
[the estimate](progress-estimate.md), and committed the wave as `a4b701d`
plus the `docs(p5)` commit that follows it. Nothing is pushed. Everything in the
"integration to-do" and "review findings" lists above is either fixed or recorded
in [TODO](TODO.md); this file is now history, and [STATUS](STATUS.md) is the entry
point. Wave 2 remains planned and not started.
