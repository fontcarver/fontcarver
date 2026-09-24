### Task 7: Drawing tools — Pen, Pencil, Knife, Shapes, Measure, Zoom, Hand, Rotate, Scale; tap/hold switching; snapping

**Files:** `app/src/editor/tools/{pen,pencil,knife,shapes,measure,zoom,hand,rotate,scale}.ts`, `app/src/editor/snapping.ts` (complete), `app/src/views/glyph/ToolPalette.tsx`, tests, e2e `app/e2e/draw.spec.ts`.

**Behaviour:** tool keys V/P/B/E/F(Shift+F ellipse)/L/Z/H(Space)/R/S with **tap = switch, hold = borrow** (FontLab); Pen: click node, drag curve, Alt during drag = outgoing handle only, Space repositions node, click first node closes, click existing node opens the path there, Shift-click a segment inserts at extremum; Pencil: freehand → `fw-geom` simplify to cubic via `AddContour`; Knife: drag a line → `InsertPointOnSegment` at intersections + `SplitContour`; Shapes: rectangle/ellipse (Shift = square/circle, Alt = from centre; click = numeric dialog); Measure: drag shows distance/angle and outline intersections (`intersections_with_line` via `glyphs.outline` data computed locally on flattened paths); Zoom/Hand; Rotate/Scale: click origin then drag, Shift constrains, commits `TransformSelection`. Snapping: to nodes, handles, metrics lines, guides, grid (pref), component ghost nodes (`glyphs.flattened_components`), Ctrl suspends; magnetic alignment lines to other nodes' x/y; snap and hit distances from prefs.
- [ ] **Step 1: Tests** per tool state machine (pointer sequences → emitted commands) and snapping resolution order (node > handle > guide > metric > grid).
- [ ] **Step 2: Implement**; e2e `draw.spec.ts`: new font → pen-draw a triangle → closed contour with 3 points appears in `glyphs.outline`.
- [ ] **Step 3:** commit `feat(app): drawing and transform tools with tap/hold switching and snapping`.

---

