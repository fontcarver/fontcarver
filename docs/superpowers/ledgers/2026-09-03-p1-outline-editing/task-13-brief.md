### Task 13: zh-CN localisation for P1 strings (Codex)

Translate all new keys (editor tools, panels, dialogs, warnings) with the P0 terminology list; key-parity test; QA pass by a second Codex run; commit `feat(i18n): zh-CN for P1 editor and dialogs`.

---

## Dependency graph and delegation plan

```
T1 fw-geom ─┐
T2 fw-doc commands (needs T1) ─┐
T3 fw-compile ─────────────────┼─ T5 fw-layout + fw-api + CLI (needs T2, T3, T4)
T4 fw-formats UFO ─────────────┘        │
T10 kernel backlog (parallel with T1–T4, disjoint files; merge before T5)
                                        ├─ T6 editor core (needs T5 API) ─ T7 tools ─ T8 panels ─ T9 text tab + dialogs ─ T12 acceptance ─ T13 i18n
                                        └─ T11 UI backlog (parallel with T6 in a worktree; merge before T8)
```
Kernel tasks T1, T3, T4, T10 run in parallel Codex worktrees (disjoint crates); T2 starts after T1 merges; T5 after T2/T3/T4. UI tasks are Opus (T6, T7, T9) or Codex when mechanical (T8, T11); reviews cross-vendor as in P0. Ports: each worktree gets its own dev-server/Vite port pair.

## P0 rulings carried into P1
- `.fwf` layout: pretty at font level, one compact glyph per line (spec §5 amended).
- Autosave sidecar path: `<datadir>/<sha1(path)|docid>.autosave.fwf` (spec §4 amended); recovery UI is Task 9.
- Binary export was never in P0 (`fw convert` to TTF arrives in Task 5).
- Warnings cross the API as `{code, message}`; the UI maps codes to i18n.

## Self-review notes
- Spec coverage for §14 row P1: glyph editor tools ✔ (T6–T7), undo ✔ (T2/T6), snapping/guides ✔ (T7/T8), components/anchors/layers ✔ (T8), metrics editing ✔ (T8/T9), transform/path ops ✔ (T1/T8), fontc export with preservation ✔ (T3/T5), UFO/designspace ✔ (T4), text tab with harfrust ✔ (T5/T9), command palette (P0) ✔, preferences ✔ (T9). Autosave wiring ✔ (T5/T9). Exit criteria proven in T12.
- Names consistent across tasks: `PointRef`, `MovePoints`, `SetMetrics`, `PathOp`, `glyphs.outline`, `text.shape`, `session.export`, `ExportOptions`, `FwSource`, `compile_static`, `preserve_merge`, `invalidateFor`.
