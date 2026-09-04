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

---

## Fix round 1

Branch `p1-t5`, on top of the merge commit `ab056b9`, not pushed. Seven commits, one
per ruling group, each test-first.

| Commit | Ruling | Contents |
|---|---|---|
| `7e2430a` | 8 | `fix(fw-compile)`: deterministic English name synthesis |
| `25361de` | 5a | `feat(fw-compile)`: additive `export_with_progress` |
| `020af9a` | 1, 2, 3 | `fix(fw-layout,fw-api)`: shape outside the session lock; panic-safe, cancellable rebuild worker |
| `1892b3f` | 5, 13 | `feat(fw-api)`: real, terminal export progress with a failure message |
| `6bc6ae3` | 6, 7 (review numbering) | `fix(fw-api)`: recipe expansion cap and bare literal characters |
| `11efe83` | 4, 10 | `feat(fw-api)`: `recovered_from` marker and an `Api`-scoped event forwarder |
| `32c449a` | 11, 12 | `test(fw-api)`: API-level preservation contract and `DeleteMode::Simple` |

### Ruling 1 — `text.shape` no longer compiles or copies the font under the session lock

`fw_layout::Shaper` gained a two-phase API. `Shaper::plan(key, generation)`
(`crates/fw-layout/src/shaping.rs:191`) only reads the cache and returns a `ShapePlan`;
`ShapePlan::needs_font()` (`shaping.rs:131`) is true only for the two cases that actually
need the model — the first-ever compile and a rebuild for a generation nothing is already
building. `Shaper::shape_planned` (`shaping.rs:226`) does the compiling and the harfrust
shaping and is called outside the lock. Width correction no longer reads the `Font`: the
handler collects a small `ModelWidths` map with `widths_for_text` (`shaping.rs:369`), which
covers exactly the glyphs the requested text encodes.

`crates/fw-api/src/handlers/shaping.rs:31` (`prepare`) is the only code that runs under
`session.with_doc`: it resolves the master, reads `document.generation`, plans, collects the
widths, and copies the font *only* when `needs_font()` says so. `handlers/shaping.rs:22`
then compiles and shapes with the lock released. The first compile is still synchronous for
the caller, as ruled.

"Never clone on a stale request that already has a pending build for the same or newer
generation" is enforced by `EntryState::scheduled_for` (`shaping.rs:414`), which tracks both
the debounced `pending` build and the generation the worker is compiling right now.

`Shaper::shape(&Font, …)` is kept as the single-shot form for callers that already own the
font outside a lock (the CLI and the crate tests); it is implemented on top of the two-phase
API, so no existing caller broke.

Covering tests:
* `crates/fw-api/tests/shaping.rs` `a_shaping_compile_does_not_block_document_edits` — opens
  DejaVu Sans, starts `text.shape` on one thread and applies `SetWidth` on another; it
  asserts the edit takes under 500 ms and under a tenth of the compile. Before the fix it
  failed with `edit 4.734314715s, compile 4.985060547s`; after it, the edit is in the
  millisecond range.
* `crates/fw-layout/tests/shaping.rs` `a_scheduled_rebuild_is_not_copied_again_for_the_same_generation`
  and `a_cached_generation_never_copies_the_font` — the copy policy itself.

### Ruling 2 — a panicking rebuild worker no longer wedges the entry

`WorkerGuard` (`crates/fw-layout/src/shaping.rs:445`) clears `worker`/`building` from `Drop`,
so a worker that dies for any reason is replaced by the next scheduling call. The compile
itself runs under `catch_unwind(AssertUnwindSafe(…))` (`shaping.rs:510`); a panic and a
compile error both record a `BuildFailure { generation, message }` on the entry. The next
`text.shape` surfaces it as a `compile-failed: <detail>` entry in `ShapedRun::warnings`
(`shaping.rs:253`, code constant `fw_layout::COMPILE_FAILED`). `EntryState::failed_for`
(`shaping.rs:427`) stops the shaper retrying the same broken generation on every keystroke;
the next edit bumps the generation and retries, and a successful build clears the failure.

`ShapedRun::warnings` stayed `Vec<String>` (changing it to a code/message struct would have
been a breaking TS change), so the stable code is the string prefix; it is documented on the
field at `shaping.rs:101`.

Covering tests:
* `crates/fw-layout/src/shaping.rs` `the_worker_guard_clears_the_worker_flag_when_the_worker_panics`
  — the guard with a panicking closure.
* `crates/fw-layout/tests/shaping.rs` `a_failed_rebuild_is_reported_and_retried_on_the_next_generation`
  — an invalid feature source makes `compile_static` error; the warning appears, and
  repairing the source plus a new generation recovers the entry.
* `crates/fw-api/tests/shaping.rs` `a_failed_background_rebuild_is_reported_and_retried`
  — the same round trip through the JSON dispatch boundary.

### Ruling 3 — shutdown

`Entry` gained a `cancelled` flag (`shaping.rs:394`). `Shaper::forget` cancels the entries it
drops and `impl Drop for Shaper` (`shaping.rs:159`) cancels every entry, so a worker that is
still in its debounce exits without compiling. The worker checks the flag before taking the
pending build, before compiling and before publishing. No joins.

Covering tests: `forgetting_a_document_cancels_its_entries`, `dropping_the_shaper_cancels_every_entry`
and `a_cancelled_worker_exits_without_compiling` (all in `crates/fw-layout/src/shaping.rs`).

### Ruling 4 — event bridge lifetime

`Api` gained `shutdown: Arc<AtomicBool>` and an `impl Drop` that sets it
(`crates/fw-api/src/api.rs:94`); the forwarder checks it before blocking and again before
republishing (`api.rs:221`), so it stops with the `Api` rather than with the `Arc<Session>`.
`event_bridge` became a `Mutex<bool>` set only when the spawn succeeded, through the
`start_once` helper (`api.rs:274`), so a failed spawn is retried by the next `subscribe()`.
The accepted progress/document ordering caveat (Minor 5) is documented on `Event`
(`crates/fw-api/src/types/events.rs:25`).

Covering tests: `a_failed_forwarder_spawn_is_retried_by_the_next_subscription` and
`dropping_the_api_stops_the_event_forwarder` in `crates/fw-api/src/api.rs`.

### Rulings 5 and 13 — real, terminal progress

`fw-compile` gained `ExportStage` (`crates/fw-compile/src/export.rs:36`) and
`export_with_progress` (`export.rs:159`), which reports `compile 0` before `compile_static`,
`preserve 60` before `preserve_merge` and `write 90` before the container is encoded;
`export()` keeps its signature and passes a no-op callback. `ProgressEvent` gained an
additive `message: Option<String>` (`crates/fw-api/src/types/events.rs:22`,
`#[serde(default, skip_serializing_if = "Option::is_none")]`, `#[ts(optional)]`), and
`Api::emit_failure` (`api.rs:199`) publishes `failed 100` with the error text. The export
handler now wraps its whole body so that *every* failure emits that terminal event
(`crates/fw-api/src/handlers/export.rs:27`).

Covering tests:
* `crates/fw-compile/tests/export_static.rs` `progress_reports_every_stage_that_actually_runs`
  and `a_failing_export_stops_reporting_at_the_stage_that_failed`.
* `crates/fw-api/tests/export.rs` `export_emits_compile_and_write_progress` (now
  `compile 0 → preserve 60 → write 90 → write 100`) and
  `a_failing_export_always_ends_with_a_terminal_progress_event` (unknown master →
  `failed 100` alone; an unwritable path → the three stages then `failed 100`).

### Ruling 6 — recipe safety

`EXPANSION_LIMIT = 65_536` (`crates/fw-api/src/handlers/recipe.rs:35`); a wider range becomes
`RecipeItem::TooLarge` and is reported in `AddGlyphsResult::skipped` as
`too-large: <token>` (`crates/fw-api/src/handlers/editing.rs:139`). A bare literal character
now resolves exactly like a one-element range through the shared `one_character` helper
(`recipe.rs:157`), so `中` and `中-中` agree.

Covering tests: `a_range_beyond_the_cap_is_reported_as_too_large` and
`a_bare_non_ascii_character_resolves_like_a_one_element_range` (unit, `recipe.rs`), plus
`an_oversized_range_is_skipped_instead_of_expanded` (`crates/fw-api/tests/editing.rs`).

### Ruling 7 — recovered marker

`DocSummary` gained `recovered_from: Option<String>` (`crates/fw-api/src/types/session.rs:138`,
additive, `#[ts(optional)]` so the app's existing summary literals still type-check). The
marker is kept in `Api::recovered` (`api.rs:84`), written by `session.recover`
(`crates/fw-api/src/handlers/session.rs:145`) and cleared by `session.close`. The recovered
document keeps the original `path` and stays dirty, as ruled. TypeScript regenerated.

Covering test: `crates/fw-api/tests/export.rs` `recovery_lists_opens_and_discards_an_autosave`
now asserts the marker on the recovered summary and its absence on a normally opened one.

### Ruling 8 — deterministic English synthesis

`insert_english_default` picks the source record by the total order
`(lang_id, platform_id, encoding_id, name_id)` (`crates/fw-compile/src/metadata.rs:256`).
Covering test: `english_synthesis_breaks_ties_deterministically` (`metadata.rs`), which builds
two German family-name records that share a language but not an encoding (one holds a
non-BMP character) and runs the synthesis 64 times. It failed before the fix
(`left: Some("Alpha 𝐀") right: Some("Alpha")`).

### Rulings 9 and 12 — coverage

`crates/fw-api/tests/export.rs` `exporting_an_untouched_import_preserves_its_raw_tables`
opens `DejaVuSansMin.woff2` through `session.open` and exports it through `session.export`:
`glyf`, `loca`, `maxp`, `fpgm`, `prep` and `cvt ` and every `extra.raw_tables` entry come out
byte-identical to the imported sfnt, `touched_glyphs` is 0 and no `hinting-dropped` warning
is reported. The other gap (a shaping compile failure surfaced sensibly) is covered by the
ruling 2 tests.

### Ruling 11 — merge integration

`cargo run -p fw-api --bin gen_ts` regenerated the bindings: `DeleteMode.ts` now reads
`"KeepShape" | "BreakContour" | "Simple"`, `DocSummary.ts` has `recovered_from?: string` and
`ProgressEvent.ts` has `message?: string`. `crates/fw-api/tests/editing.rs`
`apply_accepts_outline_commands_through_json_dispatch` now also drives
`DeletePoints { mode: "Simple" }` through raw JSON and checks the point count. The app
type-checks, tests and lints clean without any source change.

### Ruling 10

Minor 9 (three lock acquisitions in `glyphs.add`) left as accepted.

### Verification

```
cargo fmt --all --check                                  # clean
cargo clippy --workspace --all-targets -- -D warnings    # clean
cargo test --workspace                                   # 422 passed, 0 failed
cargo run -p fw-api --bin gen_ts && (cd app && pnpm check && pnpm test && pnpm lint)
                                                         # tsc clean; 38 files, 307 tests passed; eslint clean
```

The workspace total moved from 378 to 422: 44 new tests, no test removed. One earlier `pnpm test`
run flaked in `GlyphGrid.test.tsx` (`replaces on click…`, a Testing Library timeout on a cold,
heavily loaded machine); it passed on the two runs after that and the file is untouched by this
round.

### Notes for the reviewer

1. **`compile-failed` is a string prefix, not a structured code.** `ShapedRun::warnings` is
   `Vec<String>` and is already exported to TypeScript, so turning it into a `{code, message}`
   list would not have been additive. The failure is reported as `compile-failed: <detail>`,
   the prefix is the exported constant `fw_layout::COMPILE_FAILED`, and the convention is
   documented on the field. Say the word if you want the structured shape instead.
2. **A failed rebuild is not retried at the same generation.** Without that, a font whose
   feature source is broken would recompile on every keystroke forever. The next edit retries,
   which is what ruling 2 asks for; the cost is that a rebuild that failed for a transient
   reason waits for the next edit.
3. **Width correction now covers only the glyphs the text encodes.** That is the ruling's
   "small map", and it is what makes the lock-free path possible, but it means an advance edit
   to a glyph that only GSUB can reach (a ligature such as `f_i`) is no longer corrected while
   the rebuild is in flight; it lands with the rebuild instead. Nothing regressed for the
   glyphs a user types.
4. **The concurrency test is a timing test.** `a_shaping_compile_does_not_block_document_edits`
   asserts the edit finishes in under 500 ms *and* in under a tenth of the compile, and it
   fails loudly if the compile itself was too fast to prove anything. It passed in the full
   parallel workspace run, but it is the one test in this round that a very slow CI machine
   could make noisy.
5. **`Shaper::shape(&Font, …)` was kept** rather than replaced, so the CLI and the crate tests
   keep a one-call form. It is a thin wrapper over `plan`/`shape_planned` and copies the font
   only when the plan needs it, so it is safe as long as the caller is not holding a lock —
   which is documented on the method.
