# P1 Task 6 — fix round 1 (editor core UI)

Worktree: /home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t6 (branch p1-t6, HEAD 2433b2b). Requirements: task-6-brief.md, task-6-dispatch.md, docs/design/editor-ui.md (binding). The review: task-6-review.md — read it in full; numbering below follows it.

## Rulings (binding; do not re-litigate)

### Important (all must be fixed)
1. **I1** — rename the glyph-view command id to `edit.selectAllPoints` (keep `Mod+A`, `when` = glyph tab); the Font view's `edit.selectAll` must work again in a Font tab. Add a registry test that registering both views leaves both ids present, and a shortcut-dispatch test for `Mod+A` in each tab kind.
2. **I2** — nest the six new command titles under the existing `cmd.edit` / `cmd.path` / `cmd.view` objects in `en.json` and `zh-CN.json` (English placeholders in zh-CN, list them for T13). Add a test that every registered command's `title` key resolves through the app's i18next to a string that is not the key itself (this prevents the shadowing class of bug for good).
3. **I3** — the nudge coalescer must tell the controller when a run ends on its own timer (`onEnd` callback or controller-owned timer) so `onEditingChange(false)` + reload run; additionally give nudges an optimistic preview (apply the delta to the local outline as drags do) so the move is visible on the first key press. Controller-level tests: press → visible move; 500 ms later → `edit_end`, `editing` false, reload requested.
4. **I4** — `EditSession.cancel()` must compensate whenever any command was *produced*, not only after the apply round trip resolved: set `applied` synchronously in `flush()` (or await the in-flight chain before deciding). Test: Escape while an apply is in flight → `edit_end` then `doc.undo`.
5. **I5** — every gesture/commit path gets try/finally that always clears `editing` and the preview and requests a reload, and reports the error through the existing reporter (`shell/commands.ts` error path → toast). Tests: a rejecting `edit_apply` during a drag leaves `editing` false, preview null, a reload requested and the reporter called; a rejecting Info box commit reports and keeps the field editable.
6. **I6** — Alt on a *line* segment converts it to a curve (no-op and no command on curves); Alt-drag on a curve segment reshapes keeping handle angles (§4.3); plain drag on a segment reshapes moving both handles. Tests for the three cases (commands emitted, no empty undo entry on a curve).
7. **I7** — HUD snap-target names (`node`, `handle`, `guide` fallback, metric names) go through the editor label bundle (`t()`); no literal user-visible strings remain in `snapping.ts`/`hud.ts`.
8. **I8** — when the glyph source changes, push the empty selection to the store/callbacks unconditionally; test: switching glyphs clears `editorStore.selection` and the Info box shows the "nothing selected" rows.

### Spec gaps (design spec §4.3 wins over the brief; implement now)
9. Ctrl+⌥+drag on a node = nudge: the node moves, its own handles move with it, and the adjacent *unselected* handles of the neighbouring nodes scale proportionally along their segments (Glyphs behaviour: handle length scales by the ratio of new/old segment chord length). One test with a cubic segment.
10. ⇧+click on a segment inserts the node at the nearest extremum or inflection of that segment (compute locally from the segment's Bézier: derivative roots for x/y extrema plus inflection roots; pick the parameter closest to the hit; fall back to the hit parameter when the segment has none); test on a curved segment.
11. §10 status-bar flash of the last undo label for 1.5 s (documentStore already knows the label from the `Change`/undo events, or take it from the command that was applied); test with fake timers.

### Minors to fix in this round
12. M1 use `nodeOfHandle` in `editorFrame.ts`; M2 undo label counts the moved refs when no node is selected ("Move handle"/"Move 3 handles"); M3 wire `editorStore.snapping` (controller consults it; add `view.toggleSnapping` command with title key + shortcut none; status bar readout reflects it); M4 feed the drag preview into the Info box so its numbers are live during a gesture; M5 master-scope guide rows in the Info box; M6 sample Alt for the marquee at pointer-down; M7 delete dead `hand.ts pan()`; M8 split `editorController.ts` (move the keyboard/command surface and the nudge lifecycle into their own modules; target ≤ 450 lines each); M9 find the actual cause of the slow `GlyphView` tests instead of raising timeouts — revert the global 20 s timeout unless you can name the jsdom cost that justifies a per-test override (document it in the test); M10 Info box shadow uses the §2.3 tokens with a dark variant; M11 multi-selection bbox includes selected anchors/components; M12 anchor name field is controlled; M13 memoise the duplicate-node map per outline.
13. Deferred by ruling (do NOT do here): M14 custom SVG cursors → Task 7; metric-diamond and locked-component markers → Task 8; Edit-tab sidebar (Layers + glyph list) → Task 8.

## Constraints
As task-6-dispatch.md: TS strict, `t()` everywhere, transport-only, Zustand only, no Rust changes, Prettier/ESLint clean, files ≤ 800 (aim ≤ 450 for new splits). Conventional Commits on p1-t6; do not push; no subagents.

## Verification
```
pnpm --dir app check && pnpm --dir app test && pnpm --dir app lint && pnpm --dir app exec prettier --check .
```
Plus a short interactive pass with agent-browser (ports 8789/1422) covering: Mod+A in the Font tab, an arrow-key nudge (visible immediately, one undo entry), Escape mid-drag, Alt-click on a line vs a curve, switching glyph tabs; refresh `docs/screenshots/p1-select-drag.png` only if the visible chrome changed. Kill both servers when done.

## Report
Append a "Fix round 1" section to task-6-report.md (per item: change, file:line, covering test, verification output summary, interactive pass notes). Final message < 15 lines: Status, commits, test summary, concerns.
