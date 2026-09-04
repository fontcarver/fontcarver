# P1 Task 5 report — `fw-layout` shaping, `fw-api` editing/export/shaping surface, CLI build

Branch `p1-t5` in `/home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t5`, not pushed.

## Commits

| Commit | Contents |
|---|---|
| `97a516b` | `fix(fw-api)`: map the three new `fw_doc::Error` variants (build was broken at `680dc23`) |
| `fa70ccc` | `fix(fw-compile)`: always emit English default name records (the "Also fix" item) |
| `753ee83` | `feat(fw-layout)`: new crate — shaping service with a cached in-memory compiled font |
| `6a5a2f7` | `feat(fw-api)`: editing, export, shaping and recovery methods; event union; prefs |
| `1d3245e` | `feat(fw-cli)`: `fw build`, `fw shape`, widened `fw convert` |
| `7938bb8` | `feat(app)`: start autosave in the Tauri shell and the dev server |
| `935af68` | `test(fw-api)`: outline commands through the JSON dispatch boundary |

## What was built

### `crates/fw-layout` (new crate, in the workspace)

`Shaper` caches one compiled font per `FontKey { doc, master }`, built with
`fw_compile::compile_static` (no preservation; features, kerning and anchors come from the model
through fontc's feature writers). `shape(font, key, generation, request)`:

* generation matches the cache → shape, `stale: false`;
* nothing cached → compile synchronously (first use of a document/master), `stale: false`;
* generation is newer → serve the previous font with `stale: true` and schedule a debounced
  (`DEFAULT_DEBOUNCE`, 300 ms; `Shaper::with_debounce` for tests) background recompile on a named
  worker thread that coalesces bursts and exits when nothing is pending.

The **layout generation is `Document::generation`**. The dispatch listed "glyph_order || kerning ||
font_info || masters, or glyphs touched", which in practice is every change, and the document's own
counter needs no side table that could drift when commands are applied outside the API (CLI, tests).
Outline-only edits therefore also schedule a (debounced, background) recompile; this is the
dispatch's "simpler ruling" branch.

Advances: harfrust positions are corrected per glyph by `model width − advance recorded at compile
time` (a delta, so kerning already applied by the shaper survives), which makes width editing feel
instant while the rebuild runs. Glyph names come from the compiled font's `post` table and are
returned alongside the run (`ShapedRun::names`), because compiled gids are meaningless to the
document and the CLI needs the name column. Positions are exact `f64` font units; when `size > 0` is
requested they are scaled by `size / upem` (harfrust's own integer `scale` would quantise them).
`Shaper::forget(doc)` drops a document's fonts and is called by `session.close`.

### `fw-api`

New methods, exactly the brief's table (`METHODS` and dispatch stay in parity — the existing test
enforces it):

| method | notes |
|---|---|
| `glyphs.outline` | `{doc, gid, layer?}` → contours, `ComponentView[]` (flattened rounded SVG path + exact bbox), anchors, guides, width, lsb/rsb measured from the flattened bounds, layer bbox, `resolved_metrics`. `layer` absent = the default master layer; the master is derived from the layer's kind. |
| `glyphs.flattened_components` | `{paths, points}`; points are every path point (on-curve and handles) of every transformed component, for ghost-node snapping. |
| `glyphs.add` | recipe parser in `handlers/recipe.rs`: `A-Z`, `uni4E00-uni4E1F`, `U+0041-U+005A`, plain names (Unicode from AGLFN), `a.sc` / `f_i` (never encoded), `@charset:GB2312` (also `@charset:Big5`; the not-yet-compiled repertoires report the item as skipped). Applies one `Command::Batch` labelled "Add glyphs"; returns `{added: GlyphCell[], skipped: String[]}` where skipped = already present, duplicated in the recipe, or not understood. |
| `glyphs.ids` | `{doc, master?, filter, sort}` → `GlyphId[]` reusing the grid's filter/sort helpers (select-all). |
| `doc.metrics_resolve` | `{doc, gid, master?}` → `fw_doc::ResolvedMetrics`. |
| `text.shape` | `{doc, text, master?, features:[tag,bool][], script?, language?, direction?, size?}` → `ShapedRun`. Shapes inside `with_doc` (no font clone per keystroke). |
| `session.export` | `{doc, path, options}` → `ExportReport` (no bytes: path, format, bytes written, warnings, table list, touched-glyph count, timing). Formats `Ttf | Woff | Woff2 | Ufo | Designspace | Fwf`. Emits `Progress` events `compile 0 → preserve 60 → write 100` (source/native formats emit `write 0 → 100`). The font and its generations are copied out of the session first so a multi-second compile does not hold the session lock. |
| `session.recover_list / recover / discard_recovery` | over `fw_doc::autosave`; `recover` opens the sidecar as a new document keeping the original path as its provenance ("recovered from") and marks it dirty; `discard_recovery` refuses paths that are not listed in the recovery directory. |
| `prefs.get/set` | `Prefs` gains `grid_size`, `snap {nodes, guides, metrics, grid, components}`, `nudge [1,10,100]` (all `#[serde(default)]`, so older `prefs.json` files still load). |

`session.open` now also accepts `.ufo` directories and `.designspace` files. `session.save` is
unchanged (fwf only). `doc.apply` already took `fw_doc::Command`, so all Task 2 commands work; the
regenerated TS union has all 43 variants and a new test drives `MovePoints` and `PathOp` through raw
JSON.

Events: `Event` is `#[serde(tag = "kind")] { Document(DocumentEvent), Progress(ProgressEvent) }`;
`DocumentEvent` keeps its TS name and shape. `Api::subscribe()` now yields `Event` and lazily starts
a named forwarder thread that republishes session events (it ends when the session is dropped).
`Api::start_autosave()` starts `fw_doc::autosave::spawn` with `Prefs.autosave_seconds` (0 disables)
into `<config dir>/autosave`; the Tauri shell and the dev server call it at start-up. Tests that
create an `Api` never start threads unless they subscribe or export.

Error mapping: the three new `fw_doc` variants (`InvalidCommand`, `MetricsKeyCycle` →
`invalid_params`, `Geometry` → new `geometry` code) plus `From` impls for `fw_compile`, `fw_formats`
and `fw_layout` errors.

### `fw-cli`

* `fw build <in> <out.(ttf|woff|woff2)> [--remove-overlap] [--no-preserve] [--master NAME]` — goes
  through `session.export`; prints export warnings on stderr and `wrote <path> (<n> bytes, <ms> ms)`.
* `fw shape <font|fwf|ufo> <text> [--features kern,-liga] [--master NAME] [--script/--language/--direction/--size]` —
  prints tab-separated `gid name x_adv y_adv x_off y_off cluster` (tabs match `fw glyphs`).
* `fw convert` reads UFO/designspace and writes `fwf`, `ttf`, `woff`, `woff2`, `ufo`, `designspace`.
  `Error::UnsupportedOutput` now names the supported set, and `UnknownMaster` was added (both exit 2).
* The CLI opens UFO/designspace directly (new `fw-formats` dependency) instead of going through
  `session.open`, so it still does not touch the user's recent-files config.

### App (mechanical only)

`Transport.onEvent` carries `Event`; `onDocumentEvent` filters `kind === 'Document'` and a new
`onProgressEvent` filters `Progress`; the mock transport wraps bare document events. Four test files
gained the `metrics/layers/guides` fields Task 2 added to `Change`, and `gen_ts` learned the imports
Task 2's new commands need (`Guide`, `Layer`, `LayerKind`, `Master`, `MetricsKeys`) — without that
the generated `Command.ts` did not type-check. No UI work.

## Deviations from the dispatch (please confirm)

1. **No content-hash safety net for touched glyphs.** Export passes the real per-glyph generations
   (`glyph_generation(gid) > 0`, i.e. the import generation is 0 by construction). I did not add
   `Document::glyph_content_hash` captured at open, because: (a) `fw_compile::preserve_merge` only
   preserves outlines/instructions when `touched.is_empty()` **and**
   `SourceProvenance::original_bytes` is present, and those bytes are `#[serde(skip)]` — so
   preservation can only happen in the importing session, where generations are exact; (b) the union
   rule ("touched if either signal says so") means the hash can never narrow the set, so it can only
   catch an fw-doc impact bug; (c) hashing every glyph at open costs real time against the 3 s
   65k-glyph open budget; (d) it would mean adding a field and capture step to `fw-doc` while that
   crate is under review. Consequence: `fw_compile::touched_since_import` was left untouched — the
   optional hash comparison the dispatch mentions is expressible through the existing closure
   (return `u64::MAX` for a differing hash) if the controller wants it later.
2. **`remove_overlap` is still only reported, not applied.** The dispatch's "Also fix" says
   "(b) nothing else in other crates", and wiring fw-geom into the pipeline means decomposing
   components and unioning per glyph inside `fw-compile` (not in this task's file list). The API and
   CLI accept the flag and surface fw-compile's `remove-overlap-unavailable` warning
   (`fw build --remove-overlap` prints it on stderr).
3. **Layout generation = `Document::generation`** (see above), rather than a separate counter fed
   from `Change` flags.
4. `ShapedRun` carries `names`, `stale`, `units_per_em` and `size` in addition to the brief's
   `glyphs`/`warnings`; `LayerOutline` carries `layer`, `master` and `bbox` in addition to the
   brief's fields. Both are additive and needed by the CLI and the editor.
5. The old CLI test `convert_rejects_binary_output_with_exit_code_two` had to change: binary output
   is now supported. It became `convert_rejects_an_unknown_output_format_with_exit_code_two`.
6. `fw-api` gained `fw-compile`, `fw-formats` and `fw-layout` dependencies; `fw-cli` gained
   `fw-formats`; `fw-layout` (new) depends on `harfrust`, `fw-compile`, `fw-model`, `read-fonts`.

## Nothing in fw-doc was changed

No additive change to `fw-doc` was needed in the end: `autosave::list_recoverable` already exposes
the original document path, `fwf::read`/`fwf::write` are public, and import generation is
structurally 0.

## How it was verified

```
cargo fmt --all --check                                  # clean
cargo clippy --workspace --all-targets -- -D warnings    # clean
cargo test --workspace                                   # 378 passed, 0 failed
cargo run -p fw-api --bin gen_ts && cd app && pnpm check && pnpm test
                                                         # tsc clean; 38 files, 307 tests passed
pnpm --dir app lint                                      # clean
```

New tests (TDD: the fw-compile fix and every API/CLI method were written test-first):

* `crates/fw-compile/tests/export_static.rs` — `german_only_names_still_get_english_defaults`
  (failed before the fix with only 1031-language records).
* `crates/fw-layout/tests/shaping.rs` (7) — ligature from model FEA, feature switch-off, size
  scaling, stale-serve + background rebuild + width correction, DejaVu advances, invalid
  script/feature warnings, `forget`.
* `crates/fw-api/tests/editing.rs` (7) — DejaVu `A` outline has 2 contours with resolved metrics,
  component views and flattened points, `glyphs.add "A-C"` adds 3 encoded glyphs, skip/unknown
  reporting, `glyphs.ids` select-all (6253), metrics-key resolution, outline commands over JSON.
* `crates/fw-api/tests/export.rs` (6) — exported TTF sha256 equals `fw_compile::export` bytes,
  progress event sequence, touched-glyph count after an edit, WOFF2/UFO/FWF exports, recovery
  list→recover→discard, autosave started from preferences.
* `crates/fw-api/tests/shaping.rs` (4) — ligature through dispatch, feature/size options, unknown
  master → `not_found`, width edit visible while stale.
* `crates/fw-cli/tests/cli.rs` (+6) — `fw build` from a UFO produces a TTF, rejects source formats
  and unknown masters with exit 2, `fw convert` writes a UFO package, `fw shape` prints 7 columns
  with kerning applied and honours `--features -kern` (520/620 vs 600/620).
* `crates/fw-api/src/handlers/recipe.rs` — 5 unit tests for the recipe grammar.

## Open concerns for the reviewer

1. **Imported fonts shape without their layout features.** `fw-sfnt` imports GSUB/GPOS as raw tables
   (`FeaturesSource::ImportedTables`) and the shaping font is compiled without preservation, so a
   preview of an imported TTF has no ligatures or kerning until the FEA decompiler lands. Model-born
   documents (UFO, `.fwf`, new fonts) shape correctly — the tests use those.
2. **Recompile cost.** Because any edit bumps the layout generation, editing a 6k+ glyph font
   schedules a full background `compile_static` every 300 ms of activity. Correct and non-blocking,
   but it will need incremental compilation (or a coarser generation) before large-CJK editing feels
   good; measurement and tuning were out of scope here.
3. **Event delivery is now asynchronous** (one forwarder thread per `Api` that subscribes). The
   existing dispatch test had to poll for its event instead of `try_recv`ing immediately; UI clients
   are unaffected. A progress event can in principle overtake a document event because it is sent
   directly on the API channel while document events cross the bridge.
4. **`written_bytes` for UFO/designspace** walks the directory after the write; it is a report field
   only.
5. `Api::start_autosave` reads preferences at start-up only — changing `autosave_seconds` in the UI
   needs a restart to take effect (no stop/restart API in this task).
