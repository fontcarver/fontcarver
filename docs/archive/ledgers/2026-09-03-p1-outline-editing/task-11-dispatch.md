# P1 Task 11 — P0 UI backlog + self-contained e2e in CI

Where it fits: a batched maintenance task for FontWorkshop's React UI (Tauri 2 font editor; Rust kernel behind a single `api_call` dispatch + `fw:event` stream). It closes the P0 UI backlog and makes `pnpm e2e` self-contained so CI can run it. It runs in parallel with the Task 6 review (editor core, already on this branch) — stay out of `app/src/editor/**`, `app/src/views/glyph/GlyphCanvas.tsx`, `InfoBox.tsx`, `ToolPalette.tsx`, and `app/src/stores/editorStore.ts` (Task 6/7 own them); the one renderer change you need (smooth-point display) is described below.

Worktree: /home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t11 (branch p1-t11, base 2433b2b = P1 + Task 5 + Task 6). Ports for this worktree: kernel dev server 8790, Vite 1423 (other agents use 8787–8789 / 1420–1422 — never those).

## Read first
1. `.superpowers/sdd/2026-09-03-p1-outline-editing/task-11-brief.md` (absolute: /home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-11-brief.md) — read this first — it is your requirements; every item in it is in scope.
2. `CONVENTIONS.md`; `app/src/views/font/{GlyphGrid.tsx,invalidation.ts,useVirtualGrid.ts,GlyphCell.tsx,cellRenderer.ts}`; `app/src/views/fontinfo/FontInfoPanel.tsx`; `app/src/stores/{documentStore,sessionStore,uiStore}.ts`; `app/src/components/{Toast,ToastStack}.tsx`; `app/src/i18n/{warnings.ts,en.json,zh-CN.json}`; `app/src/api/client.ts` + generated `ApiError.ts`/`Change.ts`; `app/playwright.config.ts`, `app/e2e/*.spec.ts`; `crates/fw-devserver/src/main.rs` (`--port`, `--open`); `.github/workflows/*` (extend the existing CI workflow; create `ci.yml` only if none exists).
3. `docs/design/editor-ui.md` §4.1 item 9 (node glyphs: corner = 5 px square, smooth = 5 px circle) and §10 (toasts) — for the two rendering/feedback items.

## Rulings (binding)
1. **Smooth-point display**: Task 6 already draws nodes per §4.1 from `Point.smooth` in `app/src/views/glyph/drawOutline.ts` / `renderGlyph.ts`. Check whether the Font view's cell renderer and the read-only outline preview still ignore `Point.smooth`; if so, fix them there (not in the editor). If everything already honours `Point.smooth`, say so in the report with file:line — do not add code.
2. **`invalidateFor`**: extend to `change.masters` (em-box/metrics refresh of the grid cells) and `change.font_info` (Font Info live refresh); keep a single invalidation entry point; scroll position survives a full reload (store the first visible row / scroll offset in the view state and restore after the data arrives).
3. **`GlyphGrid.tsx` split** under 400 lines by responsibility (virtualisation hook already exists; extract cell layout / selection handling / context-menu wiring as needed); no behaviour change; existing tests keep passing.
4. **Toasts**: `ApiError.code` → i18n keys (`errors.<code>`) with the raw message under "Details"; unknown codes fall back to the raw message. Add keys to `en.json`; add the SAME keys to `zh-CN.json` with the English text as placeholders (the i18n parity test needs them; Task 13 translates — do not translate anything yourself) and list the new keys in the report.
5. **Self-contained e2e**: `app/playwright.config.ts` `webServer` becomes an array that starts `fw-devserver` (built binary or `cargo run -p fw-devserver -- --port ${FW_API_PORT}` with the fixtures it needs via `--open`) and Vite (`FW_DEV_PROXY`/`VITE_FW_API` so the UI is same-origin with the kernel); ports from `FW_API_PORT` (default 8787) and `FW_UI_PORT` (default 1420); `reuseExistingServer` outside CI. `pnpm e2e` must pass from a clean checkout with only `cargo build -p fw-devserver` + `pnpm install` done. The CJK spec keeps its skip when the large corpus is absent.
6. **CI**: the Linux job runs `pnpm e2e` after the Rust build with Playwright browsers installed (`pnpm exec playwright install --with-deps chromium`), uploads the Playwright report on failure, and caches cargo + pnpm. Keep the job under ~15 minutes: build `fw-devserver` in release only if the debug binary is too slow to open the fixtures (measure and report).
7. Do not change the kernel (`crates/**`, `src-tauri/**`) except, if unavoidable, an additive CLI flag on `fw-devserver` — justify it in the report.

## Verification (all must pass before reporting)
```
pnpm --dir app gen:api
pnpm --dir app check && pnpm --dir app lint && pnpm --dir app test && pnpm --dir app exec prettier --check .
cargo build -p fw-devserver && FW_API_PORT=8790 FW_UI_PORT=1423 pnpm --dir app e2e
```
Paste the e2e summary (spec names, pass/skip counts, wall time) into the report.

## Constraints
TypeScript strict, no `any`; files ≤ 400 lines for anything you create or split, ≤ 800 hard cap; `t()` for all strings; transport-only kernel access; tests for every behaviour item (invalidation on `masters`/`font_info`, scroll restore, toast mapping); Conventional Commits on p1-t11 (the brief's `chore(app): P0 backlog and self-contained e2e` for the batch; more commits fine); do not push; no subagents.

## Report
Write /home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-11-report.md: per brief item what changed (file:line), tests, the verification output summaries, CI job timing estimate, new i18n keys, deviations and concerns. Final message < 15 lines: Status (DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED), commits, one-line test summary, concerns, report path.
