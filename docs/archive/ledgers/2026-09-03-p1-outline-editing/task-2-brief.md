### Task 2: `fw-doc` editing commands and edit sessions

**Files:** `crates/fw-doc/src/command.rs` (+ new modules `command/{outline.rs, structure.rs, metrics.rs, layers.rs, glyphs.rs}` if `command.rs` would exceed 800 lines), `crates/fw-doc/src/edit.rs`, tests `crates/fw-doc/tests/{outline_commands.rs, structure_commands.rs, layers.rs, edit_sessions.rs}`.

**Interfaces:**
```rust
pub struct PointRef { pub contour: usize, pub point: usize }
pub enum Command { …P0 variants…,
    // outline (all take glyph + layer)
    MovePoints { glyph, layer, points: Vec<PointRef>, delta: (f64, f64), handles: HandleMode /* WithHandles | PointsOnly */ },
    SetPointPosition { glyph, layer, point: PointRef, x: f64, y: f64 },
    SetSmooth { glyph, layer, points: Vec<PointRef>, smooth: bool, level_handles: bool },
    InsertPointOnSegment { glyph, layer, contour: usize, segment: usize, t: f64 },
    DeletePoints { glyph, layer, points: Vec<PointRef>, mode: DeleteMode /* KeepShape (reconstruct segment) | BreakContour */ },
    ConvertSegment { glyph, layer, contour: usize, segment: usize, to: SegmentKind /* Line | Curve */ },
    AddContour { glyph, layer, contour: Contour }, DeleteContours { glyph, layer, contours: Vec<usize> },
    ReverseContours { glyph, layer, contours: Vec<usize> }, SetStartPoint { glyph, layer, point: PointRef },
    JoinContours { glyph, layer, a: PointRef, b: PointRef }, SplitContour { glyph, layer, point: PointRef },
    TransformSelection { glyph, layer, points: Vec<PointRef>, components: Vec<usize>, anchors: Vec<usize>, affine: [f64; 6] },
    PathOp { glyph, layer, op: PathOp /* RemoveOverlap | CorrectDirection | AddExtrema | AddInflections | Simplify(SimplifyOptions) | RoundToGrid(f64) | CanonicalStart | CanonicalOrder | ExpandStroke(StrokeOptions) | ConvertCurves(CurveKind) */ },
    // structure
    AddComponent { glyph, layer, component: Component }, SetComponent { glyph, layer, index: usize, component: Component }, RemoveComponents { glyph, layer, indices: Vec<usize> }, DecomposeComponents { glyph, layer, indices: Vec<usize> },
    AddAnchor { glyph, layer, anchor: Anchor }, SetAnchor { glyph, layer, index: usize, anchor: Anchor }, RemoveAnchors { glyph, layer, indices: Vec<usize> },
    AddGuide { glyph: Option<GlyphName>, layer: Option<LayerId>, master: Option<MasterId>, guide: Guide }, SetGuide {..}, RemoveGuide {..},   // glyph-local, master-global, or font-global
    // metrics
    SetMetrics { glyph, layer, width: Option<f64>, lsb: Option<f64>, rsb: Option<f64> /* lsb/rsb move the outline / adjust width; applied in that order */ },
    SetMetricsKeys { glyph, keys: MetricsKeys },
    // layers
    AddLayer { glyph, layer: Layer }, RemoveLayer { glyph, layer: LayerId }, CopyLayer { glyph, from: LayerId, to: LayerKind /* Background(master) | Backup */ }, SwapWithBackground { glyph, master },
    // glyphs
    DuplicateGlyph { source: GlyphName, name: GlyphName }, AddGlyphsFromRecipe { recipe: String /* handled in fw-api via fw-glyphdata; here AddGlyph list */ },
    SetGlyphOrder { order: Vec<GlyphName> },
    SetMaster { master: Master }, SetFontGuides { guides: Vec<Guide> },
}
```
Edit sessions: `Document::begin_edit(label)` returns `EditToken`; `apply_in_edit` coalesces; **new** `Document::preview_edit(token, cmd)` applies without snapshot churn (used only inside a token; identical result). `Change` gains `metrics: bool`, `layers: bool`, `guides: bool`. Each command's inverse is via snapshots (unchanged design). Metrics keys are stored; resolution (`=H`, `=|n`, `=n+10`) is a query `fw_doc::metrics::resolve(font, glyph, master) -> ResolvedMetrics { lsb, rsb, width, source: Key|Explicit }` with cycle detection.
- [ ] **Step 1: Tests**: every command applies and undoes to the exact prior glyph (property-style helper); `DeletePoints` KeepShape on a curve keeps the outline within 1 unit sampled; `InsertPointOnSegment` then `DeletePoints` round-trips; `SetMetrics { lsb }` shifts all contours/components/anchors and adjusts width; `DecomposeComponents` flattens with transform and removes the component; `CopyLayer` to Background creates the layer kind; coalesced `MovePoints` ×50 = one undo entry; metrics-key resolution `=|n` mirrors n's RSB and reports cycles.
- [ ] **Step 2: Implement** (commands call `fw-geom` for PathOp/transform/split).
- [ ] **Step 3:** `cargo test -p fw-doc`, clippy, commit `feat(fw-doc): outline, structure, metrics and layer commands with edit sessions`.

---

