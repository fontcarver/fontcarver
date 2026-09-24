# T8 bounded review — 2026-09-07

Reviewer: Astra. Reviewed `449a390..3b1bd4b` in `.worktrees/p1-t8`, against the brief, dispatch, implementation report, CONVENTIONS and `docs/design/editor-ui.md`. Source review only; no source edits, commits or merges. Paths below are relative to that worktree.

**Verdict: CHANGES REQUIRED.** The panel architecture, command builders and ordinary-path coverage are substantial, but two P1 correctness failures and four P2 failures need repair before integration. The recorded model limitations and historical `fw` naming are not rejection reasons.

## Correctness findings

### R1 — P1: Selected layer and master context can disagree

Location: `app/src/views/glyph/panels/LayerList.tsx:59`; `app/src/views/glyph/GlyphView.tsx:67` and `:83`.

Reproduction: in a two-master font, leave the toolbar on Regular and select Bold's master layer in either Layers list. The selected layer changes, but `masterId` and the master object still come from the document view. The scene, published panel context and editor source therefore combine Bold's outline with Regular's guides and metrics context. Adding a global guide edits Regular; Copy to background and Swap with background also use Regular's master. After explicitly choosing a layer, changing the toolbar master can leave that layer pinned while the toolbar names another master.

Fix: derive the editing master from the selected layer (`LayerOutline.master` / `layerMasterId`) and keep toolbar/document-view selection and edited-layer overrides consistent. Use that master for guide lists and commands, metrics resolution and underlays. Test selecting another master's layer, then adding a global guide, copying/swapping its background, resolving metrics and changing masters in the toolbar.

### R2 — P1: Closing a metrics gesture does not gate the next document edit

Location: `app/src/editor/asideGestures.ts:189` (also cancellation at `:112`); integration at `app/src/editor/editorController.ts:119`.

`finishMetrics` clears `this.metrics` before awaiting `gesture.finish()`. Its promise never reaches the controller's `closing` gate; `settle()` only installs nudge completion. A subsequent drag or panel command can send `edit_begin` / `doc.apply` before the previous drag's final queued apply. The kernel explicitly finalizes the previous token on either call (`crates/fw-doc/src/document.rs:138`, `:150`).

Reproduction: with a delayed transport response, drag a sidebearing, release, and immediately drag again or commit an inspector field. A deterministic in-memory probe using the real AsideGestures/MetricsDrag/EditSession implementations produced `begin(1), begin(2), apply(2), apply(1)` and a stale-token error for the first drag. Its final change is lost. Cancellation also needs the gate so its later `undo` cannot undo an unrelated new action.

Fix: register the entire finish/cancel promise with the shared document-edit gate and retain the editing state until completion. Make subsequent gestures and single-shot commands await it. Add delayed-response tests for drag → drag, drag → field, and Escape → new command; verify final geometry and separate undo entries.

### R3 — P2: Escape commits the text that it appears to cancel

Location: `app/src/views/glyph/panels/TextRow.tsx:68` and `:84`; blur commit at `:57`.

Reproduction: change the Glyph name from `A` to `A.changed`, then press Escape. `setText(value)` is batched, while `blur()` synchronously invokes the commit callback with the old DOM value. The glyph is renamed despite cancellation. The same issue affects metrics keys, notes and guide names. An in-memory jsdom probe returned `commits: ["A.changed"]` while the displayed value was temporarily `A`.

Fix: suppress the cancellation blur commit using a synchronous ref, or explicitly restore the DOM value before blur. Cover both input and textarea with tests asserting zero `doc.apply` calls on Escape.

### R4 — P2: A previous formula's resolved metrics remain actionable

Location: `app/src/views/glyph/panels/useResolvedMetrics.ts:33`, `:43` and `:85`; Apply uses the result in `MetricsPanel.tsx`.

Reproduction: resolve `=H`, change the formula to `=n`, and delay the new resolution response. The hook returns the previous formula's triple because `resolved` has no identity and is never invalidated when `signature` changes. The new outline's fallback is ignored. If that triple differs from stored metrics, Apply remains enabled and can write the previous formula's values. Cancellation prevents a late request from overwriting state, but does not prevent already-cached state from being reused incorrectly.

An in-memory hook probe changed the formula and fallback from `(10,20,600)` to `(50,60,900)` and observed the old triple while the new response was pending.

Fix: store the resolution with its signature, expose pending/error status, and only enable Apply for a matching current resolution. Test a formula/master identity change with a delayed response and an attempted Apply.

### R5 — P2: LSB drag previews metrics but leaves the glyph stationary

Location: `app/src/editor/asideGestures.ts:145`; `app/src/views/glyph/GlyphCanvas.tsx:152`.

Reproduction: drag A's LSB substantially right. The metric lines and HUD move, but the outline stays at its original coordinates until release and reload. Metrics gestures publish only a metrics triple; no contour/anchor/component preview reaches the renderer. The actual `SetMetrics` command translates contours, component transforms and anchors (`crates/fw-doc/src/command/metrics.rs:28`, `:45`). This violates the required direct visual feedback and makes the final shape jump.

Fix: generate an optimistic geometry preview matching the kernel's LSB translation, including components/flattened rendering and anchors. Keep RSB and advance behavior consistent with their kernel effects. Test a nonzero LSB drag frame against its eventual stored geometry, plus Escape restoring the original frame.

### R6 — P2: Mixed local/global guide lists use incompatible indices

Location: `app/src/views/glyph/drawGuides.ts:54`; `app/src/views/glyph/panels/GuideRows.tsx:50`.

Reproduction: create one master guide and one local guide, then select and mark the local guide as a measurement guide from its panel row. The panel uses `layer:0`, but the renderer enumerates the combined master-then-layer list and looks for `layer:1`. The selected knob and measurement distances do not appear. With multiple local guides, a different local guide can receive the marker. The new single-scope marker tests do not exercise this case.

Fix: carry an explicit index within each scope through scene/render/hit-test representations; the existing hit test also enumerates the combined list and must agree. Test master + local guides together, including selection from the panel and canvas. Session-only measurement state is approved, but its current bare `scope:index` identity also needs document/layer/master ownership so it does not mark unrelated guides after switching documents or layers.

## Completion and visual observations

- Dispatch ruling 8 explicitly requires a custom transform origin. `TransformPanel.tsx:58` derives every pivot from the nine-point picker; no custom coordinates are exposed. Implement the custom origin before claiming full completion, or obtain an explicit scope ruling. This is not listed among the approved report deviations.
- The angle knob is painted only: `drawGuides.ts:76` calls rotation a future interaction. The dispatch's canvas-marker requirement is met visually, but this is not an interactive angle handle. State this limitation accurately; angle editing remains available numerically.
- `TextRow` has no select-all-on-focus handler despite §7. `GuideRows.tsx:86` uses lock emoji despite §2.4's Lucide/no-emoji requirement. These are small conformance repairs, not reasons to redesign the panels.
- All seven required committed screenshots were inspected: Layers, Components, Anchors, Guides, Transform, Edit sidebar and Metrics handles. Panel order, compact sidebar, row expansion, counts and the example guide measurements are visible and generally coherent. The metrics screenshot exercises RSB only and does not establish LSB preview correctness. All seven are light-theme captures; dark-theme contrast and narrow layouts were not verified by this evidence.

## Verification and accepted limits

Executed the focused T8 Vitest subset: **9 files passed, 161 tests passed, 2.65 s** (`EditInspector`, `panelCommands`, `transformMath`, `pathCommands`, `pathOps`, `metricsHandles`, `markers`, `EditSidebar`, `editorController`). Three additional in-memory probes reproduced R2/R3/R4 without changing repository files. The R2 probe substitutes a transport implementing the documented token invalidation contract; it is not a live-kernel e2e result. R1/R5/R6 follow directly from the source and kernel contracts and have not been reproduced in a fresh interactive browser session.

Command emission, undo labels, affine composition, ordinary gesture behavior and the required menu shortcuts have useful existing coverage. Missing adversarial coverage is specifically delayed completion, changed resolution identity, mixed guide scopes and cross-master layer selection. No Rust or dependency changes occur in this diff. Full API generation, typecheck, lint, formatting and e2e were not rerun during this bounded review; the implementation report's earlier results remain historical evidence, not fresh verification.

Accepted report rulings: whole-layer PathOp scope, master-scoped global guides, session-only measurement flags, auto-align as the available component lock proxy, read-only vertical metrics, added overlay flags, and the separate metrics hit probe. P2 mark cloud, G2 nodes, kerning and the acknowledged pre-existing shared-session e2e harness problem are not T8 blockers. Historical `fw` names are expected here; integration must adapt to current `fc` naming.
