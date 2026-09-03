### Task 5: `fw-layout` shaping service + `fw-api` editing/export/shaping surface + CLI build

**Files:** `crates/fw-layout/src/{lib.rs, error.rs, shaping.rs}`, `crates/fw-api/src/types/{editing.rs, export.rs, shaping.rs}`, `crates/fw-api/src/handlers/{editing.rs, export.rs, shaping.rs}`, `crates/fw-cli/src/cmd/{build.rs, convert.rs}`, tests.

**Interfaces (API methods added to `METHODS`):**
| method | params | result |
|---|---|---|
| `doc.edit_begin/apply/end` | (exist) | |
| `doc.apply` | now accepts all Task 2 commands | `Change` |
| `glyphs.outline` | `{ doc, gid, layer }` | `LayerOutline { contours: Contour[], components: ComponentView[] /* with flattened path string + bbox */, anchors, guides, width, lsb, rsb, resolved_metrics }` |
| `glyphs.flattened_components` | `{ doc, gid, layer }` | `{ paths: string[], points: [x,y][] }` (for component ghost-node snapping) |
| `glyphs.add` | `{ doc, recipe: String, master?: MasterId }` | `{ added: GlyphCell[], skipped: String[] }` — recipes via fw-glyphdata (`A-Z`, `uni4E00-uni4E1F`, `a.sc`, `f_i`, charset names) |
| `glyphs.ids` | `{ doc, filter, sort }` | `GlyphId[]` (select-all) |
| `doc.metrics_resolve` | `{ doc, gid, master }` | `ResolvedMetrics` |
| `text.shape` | `{ doc, text, master, features: [tag,bool][], script?, language?, direction?, size }` | `ShapedRun { glyphs: [{ gid, cluster, x_advance, y_advance, x_offset, y_offset }], warnings }` |
| `session.export` | `{ doc, path, options: ExportOptions }` | `ExportReport` (bytes omitted) — emits `progress` events `{ doc, stage, percent }` |
| `session.recover_list` / `session.recover` / `session.discard_recovery` | | wired to `fw_doc::autosave` |
| `prefs.get/set` | Prefs gains `grid_size`, `snap: { nodes, guides, metrics, grid, components }`, `nudge: [1,10,100]`, `autosave_seconds` (already) | |
Shaping: `fw_layout::Shaper::for_document(font, master, generation)` compiles via `fw_compile::compile_static` into an in-memory font (cached, recompiled when glyph set / kerning / features generation changes; outline-only edits patch `glyf`? — P1 ruling: recompile lazily on a background thread, debounce 300 ms, serve the previous font meanwhile with `stale: true`).
CLI: `fw build <in> <out.ttf|.woff|.woff2> [--remove-overlap] [--no-preserve]`, `fw convert` gains UFO/designspace/binary targets, `fw shape <font> <text>` prints the shaped run.
- [ ] **Step 1: Tests**: dispatch tests for every new method on DejaVu (outline of `A` has 2 contours; `glyphs.add "A-C"` on an empty font adds 3 encoded glyphs; `text.shape "fi"` returns the ligature gid; `session.export` writes a TTF whose sha256 equals `fw_compile::export` output; recovery list after a simulated autosave), CLI `assert_cmd` tests for `build`/`shape`.
- [ ] **Step 2: Implement**; autosave: `Session` starts the autosave thread at construction with `Prefs.autosave_seconds`.
- [ ] **Step 3:** commit `feat(fw-api): editing, export, shaping and recovery methods; feat(fw-cli): build/shape`.

---

