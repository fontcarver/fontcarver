# P1 Task 9b — Text tab (shaped multi-glyph editing), sample texts, preview strip

Where it fits: the second half of Task 9 for FontWorkshop (Tauri 2 font editor: Rust kernel + React 19/TypeScript/Zustand UI). Task 9a delivered the dialogs; Tasks 6/7/8 the single-glyph editor (tools, panels, Info box, gesture protocol). This task turns the Edit tab into a Text tab per the Glyphs model: type text → `text.shape` → glyph runs laid out with real advances/kerning → click a glyph to edit it in place with the same tools.

Worktree: /home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t9b (branch p1-t9b, base BASE_COMMIT). Ports: kernel dev server 8794, Vite 1427.

## Read first
1. /home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-9-brief.md — the Text tab / preview strip / `text.spec.ts` half is your requirements (dialogs are done).
2. `docs/design/editor-ui.md` §3 (tab titles: first 24 characters of the text; status bar), §4.3 last bullet (Escape: deselect; second Escape exits to text mode), §5 (T = Text tool), §8 (Text tab: text bar, glyph runs, `Mod+←/→` LSB/RSB, Escape toggles text/edit mode, preview strip at 12/24/48 px with stale indicator and dark/light toggle), §10 (busy states: recompiling indicator), §11, §12.
3. Reports: `task-5-report.md` (the `text.shape` contract: `ShapedRun { glyphs[], names, stale, units_per_em, size, warnings }`, width correction, `compile-failed` warning prefix, 300 ms debounce), `task-6-report.md` (controller/store, `activeEditor`, gesture protocol), `task-7-report.md` (tools), `task-8-report.md` (panels, metrics handles), `task-9a-report.md` (dialogs, prefsStore, progress line). Code: `app/src/editor/**`, `app/src/views/glyph/**`, `app/src/shell/{TabBar,TabContent,StatusBar,tabActions}.tsx/.ts`, `app/src/stores/{uiStore,documentStore,editorStore}.ts`, `CONVENTIONS.md`.
4. Bindings: `ShapeParams.ts`, `ShapedRun.ts`, `ShapedGlyph.ts`, `Command.ts` (`SetMetrics`), `Event.ts`.

## Rulings (binding)
1. **Tab model**: an Edit tab holds `{ text: string, activeIndex: number | null, mode: 'text' | 'edit', master, size, features }`; a single-glyph tab is the degenerate case (text = that glyph's character or `/glyphname`). Tab title = first 24 chars of the text (§3); dirty dot from the document. Existing single-glyph tabs keep working unchanged (same store shape, migrated in place).
2. **Text bar** (§8, 32 px): text input (typing while in text mode edits the string; `/name` glyph-name escapes like Glyphs; paste), sample-text menu (Latin pangrams, Chinese sample sentences, "All encoded glyphs" from `glyphs.ids`, "Kerning pairs" disabled until P2), size field (px), master switcher, feature toggles (`kern`, `liga`, `calt` when present in the run's warnings/features — read what `text.shape` exposes; else fixed three), direction LTR/RTL (vertical disabled until P4).
3. **Layout**: `text.shape` once per text/master/size/features change (debounced 150 ms while typing); glyph runs positioned by the shaped advances + offsets in font units; the active glyph draws with full editing chrome and the same controller/tools (the canvas transforms by the run offset — implement as a per-glyph origin offset in the editor's viewport transform, NOT a second controller); other glyphs draw outline-only at 100 % `--cv-ink` with metrics boxes hidden unless "show metrics for all" is on. Clicking a glyph makes it active (`edit` mode); Escape toggles `edit` ↔ `text` (in edit mode a first Escape deselects, a second exits to text mode per §4.3).
4. **Stale**: when `ShapedRun.stale` is true, keep the previous layout, show the §8 dot + "recompiling…" indicator, and re-shape when the next document event arrives (debounced); a `compile-failed:` warning shows as a toast once per generation and the indicator turns into a warning badge.
5. **`Mod+←/→`** adjust LSB/RSB of the active glyph by 1/10/100 (Shift/Ctrl) as `SetMetrics` through the nudge-style coalescing (one undo entry per run, reuse the Task 6 coalescer protocol); `Mod+⌥+←/→` kerning is P2 — register the command disabled with a reason.
6. **Preview strip** (bottom, 96 px, collapsible, `Mod+⇧+P`? — no: §11 says `Mod+⇧+P` is the palette; use a View-menu toggle without a shortcut): the current text at 12/24/48 px rendered from the kernel-compiled font: request `text.shape` at those sizes and draw the outlines from `glyphs.outline`/flattened paths scaled — or, if `text.shape` returns per-glyph outlines/paths, use those (read the binding; do not add kernel work unless a small additive field is unavoidable — say so). Stale indicator shared with the layout; dark/light toggle for the strip only; feature toggles and master switch mirror the text bar.
7. **Status bar** in a Text tab: `glyph name · U+XXXX · width · LSB/RSB · zoom % · pointer x,y · snapping` for the active glyph; text-mode shows the text length and the run count.
8. **Tests**: text layout from a shaped run (pure function: run → glyph origins); `/name` escape parsing; LSB/RSB keyboard editing emits `SetMetrics` with coalescing; stale handling; e2e `app/e2e/text.spec.ts`: open DejaVu → open an Edit tab → type "Hamburgefonstiv" → runs laid out with advances (assert the second glyph's x offset equals the first advance from `text.shape`) → click the third glyph → edit chrome visible → Escape twice → back in text mode.
9. i18n keys in `en.json`, same keys in `zh-CN.json` as English placeholders, listed for Task 13. Files ≤ 450 lines; no Rust changes (unless ruling 6's small additive field — then `cargo test -p fw-api`, `gen_ts`, and say so); transport-only; Zustand only.

## Verification (all must pass before reporting)
```
pnpm --dir app gen:api
pnpm --dir app check && pnpm --dir app test && pnpm --dir app lint && pnpm --dir app exec prettier --check .
FW_API_PORT=8794 FW_UI_PORT=1427 pnpm --dir app e2e
```
Interactive pass with agent-browser (kernel `cargo run -p fw-devserver -- --port 8794 --open fixtures/fonts/DejaVuSans.ttf`; UI `cd app && FW_DEV_PROXY=http://localhost:8794 VITE_FW_API=http://localhost:1427 pnpm exec vite --port 1427 --strictPort`): screenshots `docs/screenshots/p1-text-{runs,edit-in-place,preview-strip,stale}.png` with captions. Kill both servers when done.

## Constraints
TypeScript strict, no `any`; `t()` for all strings; Conventional Commits on p1-t9b (`feat(app): text tab with shaped runs, in-place editing and preview strip`); commit often; do not push; no subagents.

## Report
Write /home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-9b-report.md (what was built with file:line, the `text.shape` usage, tests, e2e output, interactive notes + screenshots, new i18n keys, deviations, concerns). Final message < 15 lines: Status, commits, test summary, concerns, report path.
