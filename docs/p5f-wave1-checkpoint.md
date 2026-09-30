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
