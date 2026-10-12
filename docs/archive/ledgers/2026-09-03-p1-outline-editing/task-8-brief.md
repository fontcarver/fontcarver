### Task 8: Panels — Layers, Components, Anchors, Guides, Transform; metrics editing; Path menu commands

**Files:** `app/src/views/glyph/panels/*.tsx`, `app/src/views/glyph/metricsHandles.ts`, `app/src/shell/builtinCommands.ts` (Path/Glyph/Layer menus filled), tests.

**Behaviour:** Layers panel (masters list, add background/backup, copy to background, swap with background, show background toggle, "show all masters" overlay); Components panel (add via glyph picker with search, offset/scale fields, auto-align toggle, decompose, double-click enters the base glyph in a new tab, "use my metrics"); Anchors panel (add with name suggestions from fw-glyphdata defaults, drag on canvas, mark-cloud preview later); Guides panel (local/global, angle, name, lock; Alt+drag from ruler creates a guide); Transform panel (numeric move/scale/rotate/skew with origin picker, align/distribute selection, flip); metrics editing: LSB/RSB/width fields in the inspector + draggable sidebearing handles on canvas (`SetMetrics`), metrics keys field with live resolution and "out of sync" badge; Path menu: Remove Overlap (`Mod+Shift+O`), Correct Direction (`Mod+Shift+R`), Add Extremes, Add Inflections, Simplify…, Round to Grid, Reverse, Set Start Point, Tidy (canonical start/order), Expand Stroke…, Convert to Cubic/Quadratic; Glyph menu: Duplicate, Add Glyphs… (Task 9 dialog), Decompose all; all commands undoable with labels.
- [ ] **Step 1: Tests** for each panel's command emission and for Path menu wiring through `doc.apply(PathOp)`.
- [ ] **Step 2: Implement**; screenshots `docs/screenshots/p1-panels-{layers,components,anchors,guides,transform}.png`.
- [ ] **Step 3:** commit `feat(app): editor panels, metrics editing and path operations`.

---

