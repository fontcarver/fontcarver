# Task 8 fix round 1

Branch `p1-t8`.

## Commits

- `faf7ae2` — serialize metrics edits and preview LSB geometry
- `c35d0f4` — scope panel edits to the current source
- `0adebc6` — merge reviewed P1/T7/T9b
- `93b0689` — refresh canvas guide ownership
- `29716d5` — capture corrected interactions
- `112ab38` — cover source-switch metrics handover

## Findings closed

- **R1:** Layer selection updates the master; `GlyphView` derives guides, metrics, underlays, queries, commands and context from it. Toolbar changes clear the override. A two-master regression covers both transitions.
- **R2:** Metrics finish/cancel uses the controller's shared gate through completion. Delayed drag→drag, drag→field, Escape→command and Escape→drag tests verify absolute values, undo ordering and restored baselines.
- **R3:** `TextRow` synchronously suppresses cancellation blur and selects all on focus; input and textarea regressions pass.
- **R4:** Resolved metrics carry their request signature and pending/error status; only the current ready result is actionable.
- **R5:** LSB preview translates contours, anchors, component affines and bounds; RSB/width leave geometry fixed.
- **R6:** Scene guides carry scope-local indices. Measurement identities include document and layer/master ownership.
- Custom transform coordinates, Lucide guide locks and display-only angle-marker wording are present. Extracting controller geometry leaves `editorController.ts` at 799 lines.

## Verification

- frozen install; worktree-local `gen:api`; `check`, `lint`, Prettier: pass
- `vitest run --maxWorkers=50%`: **92 files, 856 tests passed**
- fresh `cargo build -p fc-devserver`: pass
- ports 8793/1426, `playwright test --workers=1`: **16 passed, 1 optional CJK skipped**

Agent-browser confirmed dark 960×720 layout, live LSB 16→216 with outline minimum 16→216 and undo restoration, plus one local and one master guide with local index 0 selected/measuring. The fixture has one master, so cross-master behavior is automated rather than interactive. No console/page errors. Servers and browser stopped.

Screenshots: `p1-panels-dark-narrow.png` (full layout), `p1-lsb-preview-fixed-dark.png` (live translated outline), `p1-guides-mixed-fixed-dark.png` (mixed scopes and measurements).
