# FontWorkshop — Design Specification

Date: 2026-09-03 · Status: approved for implementation (autonomous session; decisions recorded here stand until the user overrides them)

Companion research (read these before implementing anything that touches the same area):

- `docs/research/fontforge-data-model.md` — FontForge's structs and the 15 modelling lessons
- `docs/research/fontforge-format-io.md` — every format FontForge reads/writes, SFD syntax, table preservation
- `docs/research/fontforge-ui-inventory.md` — every FontForge command, plus the gap list vs Glyphs/FontLab
- `docs/research/fontforge-algorithms-and-otlayout.md` — OT layout model and per-module port/crate verdicts
- `docs/research/rust-crate-spike.md` — verified crate APIs (fontc `Source`, write-fonts, skrifa, fea-rs, harfrust, flo_curves)
- `docs/research/competitor-ux.md` — Glyphs / FontLab / FontCreator UX conventions and the tiered feature checklist

---

## 1. Product definition

FontWorkshop is a cross-platform (Linux, macOS, Windows) desktop font editor built with Tauri 2: a Rust kernel and a React/TypeScript UI. It is a modern re-implementation of FontForge's capabilities with the workflow and polish of Glyphs 3 / FontLab 8 / FontCreator.

Non-negotiables:

1. **Vector and bitmap fonts are both first-class.** Outline editing (cubic and quadratic) and bitmap-strike editing live in one document.
2. **Latin and CJK are both first-class from day one.** Glyph grid, search, batch operations and file I/O are designed for 65,535-glyph fonts. Components, vertical metrics and vertical features are core, not add-ons. A complete vertical-layout pipeline (`vhea`/`vmtx`/`VORG`, `vert`/`vrt2`/`vkrn`/`vpal`, vertical kerning and a vertical text preview) is a deliberate v1 differentiator: the competitor research found that neither Glyphs nor FontLab documents one.
3. **Widest practical format support**: TTF, OTF (CFF/CFF2), TTC/OTC, WOFF, WOFF2, variable fonts, UFO 2/3 + designspace, Glyphs 2/3, SFD, Type 1 (PFA/PFB), SVG fonts/glyphs, BDF, PCF, Windows FNT/FON, Linux PSF, EBDT/CBDT/sbix bitmap tables, OTB, FEA, AFM/PFM, TTX (read). Full OpenType layout: GSUB/GPOS/GDEF/BASE/MATH/STAT/COLR/CPAL and every FEA construct fea-rs supports.
4. **User experience is the top priority.** Every editing flow must be as fast and discoverable as Glyphs: single window, tabs, inspector, live preview, command palette, sensible shortcuts, undo everywhere.
5. **Lossless where the user did not touch anything.** Opening a binary font and exporting it must preserve tables we do not model (raw copy), hinting of untouched glyphs, and layout behaviour (shaping identical).

Out of scope for v1: collaborative editing, cloud sync, mobile.

---

## 2. Architecture overview

```
┌──────────────────────────────── Tauri 2 window ─────────────────────────────────┐
│  app/  (React 19 + TypeScript + Vite)                                            │
│  ├─ shell: tabs, panels, command palette, menus, i18n                           │
│  ├─ views: FontView (virtual grid), GlyphEditor (canvas), TextView (edit/kern), │
│  │         BitmapEditor, FontInfo, Features (CodeMirror), Preview, Layers …      │
│  └─ transport: Transport interface → TauriTransport | HttpTransport (dev/test)  │
└───────────────▲──────────────────────────────────────────────────▲──────────────┘
                │ invoke / events (typed by fw-api)                 │ WebSocket (dev)
┌───────────────┴─────────────────┐                  ┌─────────────┴──────────────┐
│ src-tauri  (thin shell)          │                  │ fw-devserver (axum)         │
└───────────────┬─────────────────┘                  └─────────────┬──────────────┘
                └──────────────────────┬──────────────────────────┘
                                       ▼
                     fw-api  (request/response types, session facade)
                                       ▼
   fw-doc   documents, command bus, undo timeline, dirty tracking, autosave, .fwf
      │
      ├─ fw-model     the font document model (pure data + invariants + queries)
      ├─ fw-geom      kurbo geometry, booleans (flo_curves), stroke, simplify, snapping helpers
      ├─ fw-glyphdata glyph naming/categories/decompositions/Unihan subset
      ├─ fw-layout    FEA ⇄ model, layout tables ⇄ model, kerning/anchor feature generation, shaping (harfrust)
      ├─ fw-sfnt      TTF/OTF/TTC/WOFF/WOFF2 read + write, CFF/CFF2 writer, bitmap tables, table preservation
      ├─ fw-formats   UFO, Glyphs, SFD, Type1, SVG, BDF/PCF/FNT/FON/PSF, AFM/PFM, TTX
      ├─ fw-compile   export pipeline: fontc Source impl, patch/preserve merge, post-processing, hinting hooks
      ├─ fw-raster    thumbnails, strike generation, hinted previews (skrifa + tiny-skia)
      └─ fw-hint      PS hints (blues/stems), TT instruction asm/disasm, ttfautohint integration
   fw-cli   headless binary (open/convert/build/inspect/validate) sharing everything above
```

Principles:

- **The kernel owns the document.** The UI holds a view model for the glyph(s) being edited and applies edits optimistically during drags, but every committed change is a kernel command. The kernel is the single source of truth for undo.
- **One model, many formats.** Every reader produces the same `Font` model; every writer consumes it. Format-specific data that the model cannot represent is kept in `Font::extra` (raw tables, opaque lib data) so it survives round trips.
- **Small crates, clean interfaces.** Each crate has a documented public API and its own tests. Implementation can be delegated per crate.
- **Derived data is never stored in the model.** Caches (flattened components, bounding boxes, thumbnails, dependents index) live in `fw-doc` and are invalidated by generation counters.

---

## 3. The document model (`fw-model`)

Design lineage: UFO 3 + designspace semantics for contours/masters/kerning, Glyphs concepts for masters/instances/layers/categories, FontForge extras for bitmaps/hints/unknown tables/CID. Point-list contours (UFO style) are used because they represent cubic and quadratic contours with one type and preserve TrueType implied points.

```rust
pub struct Font {
    pub id: FontId,
    pub info: FontInfo,                 // names (all name IDs, multilingual), version, OS/2, head, hhea, post, gasp, vendor…
    pub axes: Vec<Axis>,                // empty = static font
    pub masters: Vec<Master>,           // ≥1; each has a location in design space
    pub instances: Vec<Instance>,       // named exports (static or variable named instances)
    pub glyphs: IndexMap<GlyphName, Glyph>,   // insertion order = glyph order
    pub kerning_groups: KerningGroups,  // glyph → (side1 group, side2 group); groups → members
    pub features: Features,             // FEA source (prefixes, classes, features), automatic-feature flags
    pub layout_import: Option<LayoutImport>, // decompiled GSUB/GPOS/GDEF/BASE from a binary font (see §6.3)
    pub math: Option<MathTable>,
    pub color: Option<ColorData>,       // CPAL palettes + COLR paint graphs (v0/v1)
    pub bitmap_strikes: Vec<BitmapStrike>,  // per ppem/depth; glyph bitmaps keyed by GlyphName
    pub cid: Option<CidInfo>,           // registry/ordering/supplement, CID→glyph mapping
    pub guides: Vec<Guide>,             // global guides
    pub extra: Extra,                   // raw sfnt tables by tag, opaque lib entries, source provenance
    pub user_data: serde_json::Value,
}

pub struct Master { pub id: MasterId, pub name: String, pub location: DesignLocation,
    pub metrics: MasterMetrics /* ascender, descender, x_height, cap_height, italic_angle, line gaps, vertical metrics */,
    pub alignment_zones: Vec<Zone>, pub stems: Vec<Stem>, pub guides: Vec<Guide>,
    pub kerning_h: Kerning, pub kerning_v: Kerning,   // (KernSide, KernSide) → f64
    pub private_dict: PsPrivate /* BlueValues etc. */, pub tt: TtMasterData /* cvt, fpgm, prep, gasp */,
    pub user_data: serde_json::Value }

pub struct Glyph { pub name: GlyphName, pub unicodes: Vec<u32>, pub production_name: Option<String>,
    pub category: Option<Category>, pub sub_category: Option<String>, pub script: Option<Script>,
    pub export: bool, pub color_label: Option<u8>, pub tags: BTreeSet<String>, pub note: String,
    pub kern_group_side1: Option<String>, pub kern_group_side2: Option<String>,
    pub metrics_keys: MetricsKeys /* "=H", "=|n", "=n+10" style formulas for LSB/RSB/width */,
    pub layers: Vec<Layer>,             // one per master (kind = Master(id)) + extra layers
    pub locked: bool, pub user_data: serde_json::Value }

pub enum LayerKind { Master(MasterId), Brace { master: MasterId, location: DesignLocation },
    Bracket { master: MasterId, condition: AxisRange }, Background(MasterId), Backup { of: MasterId, time: i64 }, Custom(String) }

pub struct Layer { pub id: LayerId, pub kind: LayerKind, pub name: String,
    pub width: f64, pub vertical_width: Option<f64>, pub vertical_origin: Option<f64>,
    pub contours: Vec<Contour>, pub components: Vec<Component>, pub anchors: Vec<Anchor>,
    pub guides: Vec<Guide>, pub background_image: Option<Image>,
    pub hints: PsHints /* h/v/d stems with active ranges, flex, hint masks */,
    pub tt_instructions: Option<Vec<u8>>,   // per-glyph TrueType program (kept verbatim for untouched glyphs)
    pub color_layers: Option<Vec<ColorLayer>>, pub user_data: serde_json::Value }

pub struct Contour { pub closed: bool, pub points: Vec<Point>, pub name: Option<String>, pub id: Option<String> }
pub struct Point { pub x: f64, pub y: f64, pub kind: PointKind /* Move, Line, Curve, QCurve, OffCurve */,
    pub smooth: bool, pub name: Option<String>, pub tt_index: TtIndex /* Unnumbered | Implied | Index(u16) */ }

pub struct Component { pub base: GlyphName, pub transform: Affine, pub anchor: Option<String>,
    pub auto_align: bool, pub use_my_metrics: bool, pub round_to_grid: bool,
    pub smart: Option<SmartComponentValues>, pub point_match: Option<(u16, u16)> }

pub struct Anchor { pub name: String, pub x: f64, pub y: f64, pub kind: AnchorKind /* derived from name: mark/base/lig/entry/exit/caret */ }

pub struct BitmapStrike { pub ppem: u16, pub depth: u8, pub ascent: i16, pub descent: i16, pub properties: BdfProperties,
    pub glyphs: BTreeMap<GlyphName, BitmapGlyph> }
pub struct BitmapGlyph { pub bitmap: Bitmap /* origin, w, h, depth, rows */, pub advance: i16, pub vertical_advance: Option<i16>, pub refs: Vec<BitmapRef> }
```

Invariants enforced by `fw-model` constructors/setters (never by the UI):

- Glyph names are unique, glyph order is stable, every glyph has exactly one layer per master.
- Contour point sequences are well-formed for their curve kind (cubic: at most two off-curves between on-curves; quadratic: any number, implied points allowed).
- Components never form cycles (checked on insert).
- Kerning groups referenced by kerning pairs exist.

Queries (`fw-model::query`): flattened outline for a layer (components resolved), bounding boxes, dependents of a glyph, glyphs by Unicode/name/category/script, interpolation compatibility between masters, metrics-key resolution.

Identity: glyphs are addressed by `GlyphName` externally and by a stable `GlyphId` (u32 handle) inside a session so renames do not invalidate UI state.

---

## 4. Documents, commands and undo (`fw-doc`)

- `Session` holds open `Document`s. A `Document` wraps a `Font`, its file provenance (path, format, original bytes for preservation), a `generation: u64`, per-glyph generations, and derived caches.
- All mutation goes through `Command`s (`enum Command { Glyph(GlyphCommand), Font(FontCommand), Kerning(...), Features(...), Bitmap(...), Batch(Vec<Command>) }`). `Document::apply(cmd) -> Result<Change>` returns what changed so the UI can invalidate precisely.
- **Undo** is a document-level timeline of `UndoEntry { label, before: Snapshot, after: Snapshot, glyphs_touched }`. Snapshots are `Arc<Glyph>`/`Arc<Layer>` clones with structural sharing (a glyph command clones only that glyph). Interactive drags coalesce: the UI sends `begin_edit / update_edit / end_edit`; one undo entry results. Undo depth is unlimited by default, bounded by a memory budget.
- Autosave every N seconds to a sidecar (`<file>.fwf.autosave`), restored on next launch after a crash.
- Change notifications: `DocumentEvent { doc, generation, glyphs: Vec<GlyphId>, scope: bitflags }` are broadcast to the UI.

---

## 5. Native file format `.fwf`

Single UTF-8 JSON file, deterministic key order and glyph order, 2-space indent, one glyph object per line group. This keeps `git diff` readable and avoids a directory package. Schema versioned (`"format": "fontworkshop", "version": 1`). Binary payloads (bitmaps, raw tables, images) are base64. Files above ~50 MB may optionally be written as `.fwf.gz` (transparent on read). A directory-package variant is deferred until measured need.

Every other format is import/export. UFO/designspace and Glyphs exports are "lossless for what those formats can express" and use `lib`/`userData` for FontWorkshop extras.

---

## 6. File I/O

### 6.1 Format registry (`fw-formats`, `fw-sfnt`)

`trait FontReader { fn sniff(bytes, path) -> Confidence; fn read(&self, src, opts) -> Result<Vec<Font>> }` and `trait FontWriter { fn write(&self, font, opts, sink) -> Result<()> }`. Detection uses magic bytes first, extension second (same order as FontForge's `_ReadSplineFont`). Compressed inputs (`.gz`, `.zip` containing one font/UFO) are handled transparently.

| Format | Read | Write | Basis |
|---|---|---|---|
| TTF / OTF (glyf, CFF, CFF2), variable (fvar/gvar/HVAR/MVAR/STAT/avar/cvar) | ✔ | ✔ | read-fonts / skrifa; write via fontc + write-fonts; **our CFF/CFF2 writer** |
| TTC / OTC | ✔ | ✔ | read-fonts; **our TTC assembler** (shared tables dedup by bytes) |
| WOFF / WOFF2 | ✔ | ✔ | flate2/zopfli; WOFF2 via `woofwoof` (vendored C++ brotli/woff2) |
| GSUB/GPOS/GDEF/BASE/MATH/STAT/COLR/CPAL/EBDT/EBLC/CBDT/CBLC/sbix/VORG/vhea/vmtx/kern | ✔ | ✔ | read-fonts + write-fonts; **our MATH parser/writer, our bitmap-table writers** |
| UFO 2 / UFO 3 + designspace | ✔ | ✔ | norad |
| Glyphs 2 / 3 | ✔ | ✔ | glyphs-reader; **our writer** (Glyphs 3 plist) |
| SFD / SFDir (FontForge) | ✔ | ✔ | **ours** (spec in research doc; UTF-7 and ASCII-85 codecs included) |
| Type 1 PFA/PFB, Type 42, bare CFF, AFM/PFM | ✔ | ✔ | **ours** (charstring Type1 decoder/encoder, eexec) |
| SVG font, SVG glyph import/export | ✔ | ✔ | usvg for import; our writer |
| BDF, PCF, Windows FNT/FON, PSF1/2, OTB | ✔ | ✔ | **ours** (port from bitmap-font-viewer TS + FontForge) |
| FEA | ✔ | ✔ | fea-rs parse/compile; **our FEA writer** |
| TTX (fontTools XML) | ✔ | — | ours, subset |
| PDF-embedded fonts, Ikarus, Palm, PK/GF, Mac resource fonts | later | — | low priority |

### 6.2 Opening a binary font

1. Parse with read-fonts. Build `Font` with one master (static) or masters derived from the default outline plus gvar deltas? **No.** Variable binaries open as a single-master font at the default location with `axes` and named instances recorded, plus a `VariableSource` in `extra` holding the parsed variation data so the user can (a) edit at default and export a static font, or (b) run "Extract masters" which materialises a master per unique tuple region (deltas applied) and marks the font as multi-master. This mirrors FontForge's import while keeping the common case fast.
2. Outlines: glyf → quadratic point lists with `tt_index` and implied points preserved; CFF/CFF2 → cubic; composites → `Component`s with `use_my_metrics`/`round_to_grid`/point-match.
3. Metrics, names, OS/2, post names, cmap (all subtables, variation selectors → `unicodes` with VS), vhea/vmtx, gasp, cvt/fpgm/prep, per-glyph instructions, hdmx-like tables → dropped and regenerated.
4. GSUB/GPOS/GDEF/BASE: kept raw in `extra.raw_tables` **and** decompiled into `LayoutImport` (see 6.3). Kerning-only PairPos lookups in the `kern` feature and mark/mkmk anchors are additionally lifted into `Master::kerning_*` and `Anchor`s when the lift is lossless (the lookup contains nothing else); the corresponding lookups are marked "lifted" so they are regenerated on export.
5. Bitmap tables → `BitmapStrike`s. `MATH` → `MathTable`. `COLR/CPAL` → `ColorData`. `SVG ` → per-glyph SVG documents (kept raw, editable as text).
6. Every table not listed above → `extra.raw_tables` (DSIG is dropped deliberately).

### 6.3 Layout tables ⇄ FEA (`fw-layout`)

- `LayoutImport` holds owned write-fonts `Gsub`/`Gpos`/`Gdef`/`Base` tables plus a **decompiled FEA rendering** produced by our decompiler (lookup-by-lookup, with class synthesis for PairPos format 2 and ChainContext format 2, names like `lookup_gsub_3`). The decompiler is validated by recompiling with fea-rs and comparing shaping on a corpus.
- The user sees the font's features in the Features panel as editable FEA. Until they edit, export uses the raw tables (lossless). Once they edit and the FEA compiles, export uses the compiled result (`Features::source_of_truth = Fea`). "Revert to original tables" is one click.
- Kerning and anchors are compiled to `kern`/`mark`/`mkmk`/`curs`/`abvm`/`blwm` automatically by the fontc pipeline (fontbe feature writers), with `# Automatic Code` markers respected — same behaviour as Glyphs.
- Automatic features from glyph names (`.sc`→`smcp`, `.ss01`→`ss01`, `_` ligatures→`liga`/`dlig`, `.case`, `.locl` conventions, `vert`/`vrt2` from `.vert`) are generated into an "Automatic" prefix the user can freeze and edit.
- Shaping preview everywhere uses harfrust on a font compiled from the current document (incremental: recompile only when features/kerning/glyph set change; outline edits update glyph shapes without a re-shape).

### 6.4 Export pipeline (`fw-compile`)

Single pipeline, user chooses targets in one dialog (like Glyphs Export): format (TTF / OTF-CFF / variable TTF / variable OTF-CFF2 / WOFF / WOFF2 / UFO / Glyphs / SFD / Type1 / BDF / PCF / FON / OTB / SVG), instances or masters, options (remove overlap, autohint, keep TT instructions, subroutinize, production names, preserve untouched original tables, table subset).

1. **Compile** via our `fontir::source::Source` implementation (`FwSource`) feeding fontc `generate_font`, producing glyf/loca or (for CFF targets) glyf which we then replace by **our CFF/CFF2 writer** output from the same IR (CFF writer accepts cubic outlines, PS hints, FDArray/FDSelect for CID, optional subroutinization).
2. **Merge** with preserved data: raw unknown tables, original layout tables if features untouched, per-glyph TT instructions + cvt/fpgm/prep if the glyph's outline generation matches import, MATH, bitmap tables, VORG.
3. **Post-process**: autohint (PS blues/stems from `fw-hint`; TrueType via external `ttfautohint` if installed, else skip with a notice), `kern` legacy table (option), `DSIG` dummy (option), TTC assembly, WOFF/WOFF2 wrapping, validation report (our checks + a shaping smoke test with harfrust).

A headless `fw build` CLI command runs the same pipeline for batch/CI use.

---

## 7. Geometry (`fw-geom`)

Built on kurbo. Public operations, each with a documented tolerance and unit tests against FontForge's fixture SFDs (`fontforge/tests/fonts/*Bugs.sfd`, `StrokeTests.sfd`, `AddExtrema*.sfd`):

- Booleans: remove overlap / union / intersect / exclude via `flo_curves` (cubic-preserving); quadratic input is degree-elevated, output re-quantised with `CubicBez::to_quads` for quadratic layers. `i_overlay` (flattened) is used as a validation oracle; a large area mismatch flags the glyph instead of silently corrupting it. Pre-passes ported from FontForge: remove tiny segments, remove stupid control points, remove backtracks.
- Stroke expansion: `kurbo::stroke` for round nibs; ported FontForge convex-nib tracer for calligraphic/polygonal nibs; caps/joins/miter limit; remove internal/external contour.
- Simplify (with FontForge's flag semantics), add extrema, add inflections, round to grid (with TT implied-point awareness), balance/harmonize, correct direction (signed-area nesting), canonical start point and contour order, cubic ⇄ quadratic conversion (`to_quads` with error bound; quad→cubic exact), transform (affine with anchor/guide/metrics propagation), nonlinear transform (expression via `evalexpr` + resample/refit), align/space points.
- Interactive helpers used by the UI through the API: hit testing (points, handles, segments, components, anchors, guides), nearest point on path with `t`, tangent/curvature at `t`, snapping candidates (points, metrics lines, guides, grid, other glyphs, background), measurement (distance/angle along a ruler, intersections with outline).
- Spacing: port of `autowidth2` (profile-based sidebearing and kerning suggestions) with a pluggable separation function.
- Interpolation: point-compatible master interpolation and compatibility diagnostics (point count/type/start/direction per contour, component order).

---

## 8. Rendering and bitmaps (`fw-raster`)

- Thumbnails and text previews: skrifa outline extraction from either the compiled font or directly from model layers (our own pen), rasterised with tiny-skia. Thumbnail cache keyed by (glyph generation, size, master).
- Hinted previews and "grid fit" view: skrifa `HintingInstance` with `Engine::Interpreter` (TT bytecode, PS hints) or `Engine::Auto`. Provides device-space outlines for the editor overlay and pixel bitmaps for the waterfall.
- Bitmap strike generation from outlines at chosen ppem with options (hinted/unhinted, mono/grey, threshold, depth), and bitmap → outline tracing (pixel-square contours with optional corner smoothing; `vtracer` for raster images).
- Bitmap editing ops (ported from bitmap-font-viewer and FontForge `bvedit`): pixel set/clear, lines, rects/ellipses, floating selection move/copy/paste, flip/rotate/shift, scale (nearest / grey), crop, invert.

---

## 9. Glyph data (`fw-glyphdata`)

A compiled database (generated at build time from AGLFN, Unicode UCD via `icu_properties`, Glyphs' GlyphData conventions re-expressed, and a Unihan subset: kRSUnicode radical/stroke, kTotalStrokes, kMandarin pinyin, kIICore/GB2312/Big5/JIS levels) giving: name ↔ Unicode, category/sub-category, script, default anchors, decomposition recipes, sort keys. Used for "Add glyphs" recipes, automatic naming, sidebar filters (Categories / Scripts / Languages / Unicode blocks / CJK charsets / radical / stroke count / pinyin), and composite building.

---

## 10. Hinting (`fw-hint`)

- PS: blue zones and StdHW/StdVW/StemSnap computation (port of FontForge `FindBlues`/`FigureStems`), stem hint detection (port of `stemdb` core, staged), hint masks; hints stored on layers and written into CFF/Type1.
- TT: instruction assembler/disassembler (FontForge opcode table), instruction editor with disassembly view, cvt/fpgm/prep editing; instructions preserved for untouched glyphs; autoinstruct via external `ttfautohint` (auto-detected on PATH; downloadable via a help link). A stepping debugger on skrifa's interpreter is a stretch goal.

---

## 11. API layer (`fw-api`) and transports

All UI ↔ kernel traffic is a typed request/response set defined once in Rust with `serde` and exported to TypeScript (`ts-rs`) so both sides share types. Categories: session (open/save/close/recent), document queries (glyph list pages, glyph detail, previews), commands (mutations, batch), edit sessions (begin/update/end for drags), export, shaping, search, preferences, plus a `Subscribe` stream for `DocumentEvent`s.

Transports: Tauri `invoke` + `emit` (production), and `fw-devserver` (axum HTTP + WebSocket, same JSON payloads) for running the UI in a normal browser during development and for end-to-end tests with agent-browser. Large payloads (thumbnails, flattened paths) are compact JSON arrays; binary channels are added only if measured necessary.

Performance budgets: opening a 65k-glyph CJK OTF < 3 s; font view scroll at 60 fps with 300 visible cells; glyph edit round trip (command → event → repaint) < 16 ms; export of a 65k-glyph static TTF < 20 s.

---

## 12. User interface (`app/`)

### 12.1 Shell

- **Single window per font document, tabs inside**: the first tab is the Font view; each opened glyph or text string opens an **Edit tab** (Glyphs model). Multiple fonts open as separate windows (Tauri multi-window), with drag-and-drop and copy/paste between them.
- **Left sidebar** (Font view): Categories / Scripts / Languages / Unicode blocks / CJK charsets & radicals / Smart filters / Lists / Tags; glyph count badges.
- **Right inspector**: context-sensitive panels — Glyph (name, unicodes, category, export, color, tags, metrics keys, kerning groups), Selection/Transform (x/y/w/h/rotation/scale, align/distribute), Layers (masters, brace/bracket, backgrounds, backups), Components, Anchors, Guides, Hints, Kerning (pairs for current glyph), Bitmap strike, Font (metrics for current master).
- **Bottom preview strip** in Edit tabs (live shaped text at a chosen size, master/instance switcher, dark/light).
- **Command palette** (Ctrl/Cmd+Shift+P) listing every command with shortcut and search; **menu bar** mirrors it (File, Edit, Glyph, Path, Layer, Font, Features, Bitmap, View, Window, Help).
- Design system: CSS variables for spacing/typography/colour; light and dark themes; HiDPI-correct canvases; keyboard-navigable; English default with zh-CN localisation from the start (all strings through i18n).

### 12.2 Font view

Virtualised grid (canvas or DOM cells with canvas thumbnails, tested for 65k glyphs), cell size slider, label modes (name / Unicode / index), colour labels, master switch, status overlays (unexported, incompatible masters, warnings). Search box with syntax (`name:`, `u+`, `cat:`, `script:`, `tag:`, `has:component`, radical/pinyin for CJK). Multi-select with shift/ctrl and rubber band; batch commands apply to selection (transform, add glyphs, rename, set metrics, remove overlap, build composites, generate bitmaps…). "Add Glyphs" dialog accepts recipes (`A-Z`, `uni4E00-uni4E1F`, `a.sc`, `f_i`, `@Lowercase`) and charsets (GB 2312, GB 18030 level 1, Big5, JIS X 0208, KS X 1001, Latin Plus…).

### 12.3 Glyph editor (Edit tab)

Canvas 2D, DPR-aware, infinite pan/zoom, rulers, grid, metrics lines (baseline, x-height, cap height, ascender, descender, vertical origin/advance for CJK), sidebearing handles with live values, info HUD following the cursor. Tools (single-key shortcuts as in Glyphs): Select (V), Pen (P), Pencil/freehand (B), Knife (E), Measure (L), Text (T), Hand (H), Zoom (Z), Primitives rect/ellipse (F), Rotate (R), Scale (S). Node semantics: smooth/corner toggle by double-click or Enter, handle retraction with Alt-click, Shift constrains, Alt-drag duplicates, Ctrl-Alt drag keeps handle length, click on segment adds a point, drag segment reshapes, Tab/Shift-Tab cycles nodes. Snapping to nodes, handles, metrics, guides, grid, other glyph outlines, background layer, with visual feedback; magnetic alignment lines. Path direction arrows, extrema markers, points-at-inflection markers, coordinates on hover, TrueType point indices optional.

Views/overlays: fill preview (space bar), show all masters, show background layer, show hints/blue zones, show anchors with mark cloud, show components (locked/editable, double-click to enter), grid fit preview at chosen ppem, measurement lines (Alt-drag), guides (global and local, angle, snap). Multiple glyphs are laid out with real shaping in the same tab (text tab): type to add glyphs, Ctrl/Cmd+arrows adjust sidebearings, Alt+arrows adjust kerning (with 1/10/100 step modifiers), kerning group vs exception toggle.

Undo/redo everywhere (Ctrl/Cmd+Z / Shift+Z), with labels in the Edit menu.

### 12.4 Spacing & kerning

Kerning is group-based (`@MMK_L_x`/`@MMK_R_x` semantics, side-aware for RTL), with pair exceptions. Kerning panel: pairs list with filters, group manager, "kern in context" text strings (word lists), conflict/overlap report, auto-kern and auto-space suggestions from `fw-geom` spacing, vertical kerning for CJK. Metrics keys (`=H`, `=|n`, `=n+10`) with live resolution and "out of sync" warnings.

### 12.5 Features

CodeMirror 6 editor with FEA syntax highlighting, autocomplete of glyph names/classes/tags, and inline diagnostics from fea-rs (`Diagnostic::span`). Tree of Prefixes / Classes / Features with automatic-feature toggles. "Compile & preview" pushes to the shaping preview. A read-only "Lookups" view lists compiled lookups with their rules rendered as glyphs (for imported fonts).

### 12.6 Font Info

Sections: Font (family, designer, license, version, vendor, all name records with per-language table), Masters (metrics, zones, stems, guides, custom parameters), Axes, Instances/Exports (names, coordinates, style linking, STAT values), Features (link), Other (OS/2 flags, panose, fsType, gasp, PS private, TeX, MATH constants, vertical metrics policy, CID). Validation messages inline.

### 12.7 Variable fonts

Axes/masters/instances editing with a design-space map, interpolation slider in the editor (live preview across masters), master compatibility indicator per glyph with a "fix compatibility" assistant (reorder contours, start points, add points), brace and bracket layers, "Add master from instance", STAT editing.

### 12.8 Bitmap editor

Strike list (ppem, depth), pixel canvas with pencil/eraser/line/rect/ellipse/fill/select, floating selection, per-glyph metrics (advance, bearings, bbox), outline underlay at the strike size, "rasterise from outline" (with hint options), "trace to outline", BDF properties editor, waterfall preview with real strike metrics.

### 12.9 Preview & proofing

Text preview panel with sample texts per script, waterfall, feature toggles, script/language/direction, dark mode, compare masters/instances, print to PDF.

### 12.10 Other

Find & fix problems (all FontForge checks reorganised into categories with "fix" actions), Find/replace outlines, Compare fonts, Merge fonts, Style effects (embolden LCG/CJK, condense/extend, oblique, small caps, sub/superscripts), Build composites/accents, Batch rename, Mass edit metrics, Import image/SVG/EPS to background, Autotrace, Scripting (JavaScript API exposed to a script console; Python later), Preferences.

---

## 13. Testing strategy

- Rust: unit tests per crate; golden round-trip tests over a fixture corpus (`fixtures/`: FontForge test fonts, DejaVu, Noto samples, Source Han Sans OTF/VF from `~/githubprojects/pixelfonts/SourceHanSanOTF`, bitmap fonts from sibling projects); property tests for geometry (`proptest`); shaping-equivalence tests (harfrust before/after) for layout round trips; fuzz targets for parsers (`cargo fuzz`, optional).
- CLI smoke tests: `fw convert`, `fw build`, `fw inspect` on the corpus in CI.
- Frontend: vitest unit tests for stores/utilities; component tests; e2e with agent-browser against `fw-devserver` (scripted: open font, edit, export, assert on file) — this is also how Opus agents verify UI work with screenshots.
- Performance tests with a 65k-glyph font as a gate for P0.

---

## 14. Delivery phases

Each phase ends in a usable application. Detailed task plans are written per phase in `docs/superpowers/plans/`.

| Phase | Scope | Exit criteria |
|---|---|---|
| **P0 Foundation** | workspace + conventions; `fw-model`, `fw-doc`, `fw-api`, `fw-sfnt` read (TTF/OTF/TTC/WOFF/WOFF2, variable metadata, bitmaps raw), `.fwf` save/load, `fw-devserver`, `fw-cli inspect/convert`; app shell with Font view (virtual grid, sidebar filters, search), read-only Glyph view, Font Info (read), i18n scaffold, theming | Source Han Sans OTF opens < 3 s, grid scrolls smoothly, save/reopen `.fwf` round-trips, CLI converts TTF → fwf → TTF (glyph-identical) |
| **P1 Outline editing & export** | glyph editor with full tool set, undo, snapping, guides, components, anchors, layers, metrics editing, transform/path ops (`fw-geom`), fontc-based export (TTF static) with table preservation, UFO/designspace import/export, text-tab with harfrust preview, command palette, preferences | "Open commercial TTF, edit 3 glyphs, export" yields a font whose untouched tables/hinting are preserved; "draw a new Latin font from scratch and export" works end to end |
| **P2 Spacing, kerning, features, CFF** | kerning groups/panel/text-tab kerning, metrics keys, FEA editor with diagnostics, layout decompiler, automatic features, OTF/CFF export (our CFF writer), Glyphs import/export, SFD import/export, Type 1 read/write, Find problems v1 | professional Latin workflow complete; Glyphs/UFO/SFD round trips pass corpus tests |
| **P3 Bitmap fonts** | strikes model + editor, BDF/PCF/FNT/FON/PSF/OTB/EBDT/CBDT/sbix read/write, rasterise/trace, waterfall | replaces bitmap-font-viewer; CJK BDF workflow complete |
| **P4 Variable & CJK pro** | axes/masters/instances UI, interpolation preview, compatibility fixer, brace/bracket layers, variable export (gvar/CFF2), smart components, vertical metrics/`vert`/`vkrn`, Unihan filters & batch tools, CID export | variable Latin + CJK font from masters exports and installs |
| **P5 Professional finish** | hinting (PS autohint, TT editor, ttfautohint), MATH, COLR/CPAL + SVG colour, style effects, compare/merge, autotrace, scripting API, plugins, remaining formats, installers (deb/AppImage/dmg/msi), docs | feature parity checklist (research doc §12) green for Must-have and Should-have |

---

## 15. Engineering conventions (summary; full text in `CONVENTIONS.md`)

- Rust 2021, MSRV = stable at project start, `cargo fmt`, `cargo clippy -D warnings`, `thiserror` errors per crate, no `unwrap` outside tests, `tracing` for logs, docs on every public item, unit tests beside code, integration tests in `tests/`.
- TypeScript strict, ESLint + Prettier, React function components + hooks, Zustand stores per domain, no default exports, all user strings via `t()`, CSS modules + tokens, canvas code in plain TS classes (not React), Vitest.
- Commit style: Conventional Commits; every task lands with tests; CI runs `cargo test`, `pnpm check`, `pnpm test`.
- Delegation: every task is a file under `docs/superpowers/plans/tasks/` with scope, interfaces, acceptance tests; Codex (gpt-5.6-sol, xhigh) implements Rust/kernel tasks; Opus implements UI tasks with screenshot verification; cross-review before merge; max three review rounds.
