# Task 5 review — `fw-layout` shaping, `fw-api` editing/export/shaping surface, CLI build

### Spec Compliance

- ✅ Spec compliant. Every method in the brief's table exists, is dispatched, is in `METHODS`, and is ts-rs exported.
  - `METHODS` entries + dispatch arms for all ten new methods: `crates/fw-api/src/api.rs:49-70` and `:257-266`; parity test `crates/fw-api/src/api.rs` `dispatch_and_typescript_method_tables_match` (unchanged, still covers the new rows).
  - `glyphs.outline` returns contours/components/anchors/guides/width/lsb/rsb/resolved_metrics (`crates/fw-api/src/handlers/editing.rs:16-45`, `crates/fw-api/src/types/editing.rs:30-70`); `glyphs.flattened_components` returns `{paths, points}` (`handlers/editing.rs:47-65`); `glyphs.ids`, `doc.metrics_resolve`, `glyphs.add`, `text.shape`, `session.export`, `session.recover_list/recover/discard_recovery` all present.
  - Prefs gained `grid_size`, `snap{nodes,guides,metrics,grid,components}`, `nudge[3]`, all `#[serde(default)]` (`crates/fw-api/src/types/session.rs:143-224`); serde tolerance proven by `crates/fw-api/tests/export.rs` `autosave_starts_from_preferences`, which sets prefs with the old four fields only.
  - CLI `fw build`/`fw shape`/widened `fw convert` (`crates/fw-cli/src/main.rs:19-45`, `cmd/build.rs`, `cmd/shape.rs`, `cmd/convert.rs`), exit code 2 for unsupported output and unknown master (`crates/fw-cli/src/error.rs:47-62`), covered by `crates/fw-cli/tests/cli.rs:74-170`.
  - "Also fix" (a) done and tested: `crates/fw-compile/src/metadata.rs:220-257` synthesises a 0x0409 record for name IDs 1/2/4/6 from any other language; `crates/fw-compile/tests/export_static.rs:158-181` asserts both the English default and the surviving German record.
  - "Also fix" (b) honoured: outside fw-api/fw-cli/fw-layout/app the diff touches only `fw-compile/src/metadata.rs`; fw-doc, fw-model, fw-geom, fw-sfnt, fw-formats are untouched.
  - Event union built additively as `#[serde(tag="kind")] Event { Document(DocumentEvent), Progress(ProgressEvent) }` (`crates/fw-api/src/types/events.rs:21-31`); generated `Event.ts` is `{kind:"Document"} & DocumentEvent | {kind:"Progress"} & ProgressEvent` and `DocumentEvent` keeps its TS name, as the dispatch required.
  - Controller rulings 1–4 match the code: no content hash (export passes real per-glyph generations only, `handlers/export.rs:90-107`); `remove_overlap` is forwarded to fw-compile and only surfaced as a warning (`handlers/export.rs:112-135`); nothing decompiles imported GSUB/GPOS; every edit bumps the layout generation.
- ⚠️ Cannot verify from diff:
  - Test/lint results (`cargo test --workspace` 378 passed, `pnpm check`/`pnpm test`) are the implementer's claims; I did not re-run them. The report's evidence block is present and not truncated. I did verify the generated bindings on disk are consistent with the app's new code (`app/src/api/generated/Event.ts`, `ShapedRun.ts`, `methods.ts:68-77`, `Command.ts` with the Task 2 imports).
  - Performance budgets (DejaVu TTF export < 5 s, drag commit ≤ 4 ms) — no measurement anywhere in the diff or report; see Important #1 for a structural risk against the second one.
  - Preservation contract end-to-end through `session.export` on an imported binary (untouched raw tables byte-identical, instruction retention/drop warning): tested at the fw-compile layer in Task 3, but no API-level test exercises it. `snapshot()` copies the `Font`, and `extra.original_bytes` is `Option<Arc<Vec<u8>>>` (`crates/fw-model/src/extra.rs:32`), so the copy keeps preservation working — checked because "export must still preserve after the font is copied out of the session" was a concrete risk.

### Strengths

- No `unwrap`/`expect`/`panic!`/`unreachable!` anywhere in the new library code (checked all thirteen new/changed non-test source files); mutex poisoning is handled by design (`crates/fw-layout/src/shaping.rs:434-436` `lock()` uses `PoisonError::into_inner`).
- Plan file structure followed exactly (`fw-layout/src/{lib,error,shaping}.rs`, `fw-api/src/types/{editing,export,shaping}.rs`, `fw-api/src/handlers/{editing,export,shaping}.rs`, `fw-cli/src/cmd/{build,convert}.rs`), all files well under 800 lines (largest new file: `shaping.rs` 436).
- The rebuild handshake is genuinely race-free: `pending` is taken and `rebuilding` cleared under the same lock (`shaping.rs:263-283`), so there is no lost wake-up and never two workers per entry; the compile itself runs outside the entry lock, so a fontc panic cannot poison it.
- Tests exercise real behaviour on real fonts through the JSON dispatch boundary — DejaVu outline/ids (6 253), sha256 equality with `fw_compile::export`, a real autosave thread for the recovery round trip, stale-serve + background rebuild with an `is_current` poll, `fw shape` kerning on/off via the CLI binary (520/620 vs 600/620).
- Export deliberately copies the font out of the session before a multi-second compile, with the reason written down (`handlers/export.rs:86-89`) — exactly the right instinct.
- Recovery refuses snapshot paths that are not in the recovery directory (`handlers/session.rs:152-163`), so `discard_recovery` cannot be turned into an arbitrary file delete.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

1. **`text.shape` runs the synchronous first compile — and a full `Font` deep clone on every stale request — while holding the session lock.**
   `crates/fw-api/src/handlers/shaping.rs:19-35` wraps the whole `Shaper::shape` call in `session.with_doc(...)`, which is a read lock on the session's `RwLock<HashMap<DocId, Document>>` (`crates/fw-doc/src/session.rs:195-206`). Inside it, `crates/fw-layout/src/shaping.rs:197-199` runs `compile_static` synchronously the first time a (doc, master) is shaped, and `shaping.rs:185-188` does `Box::new(font.clone())` on *every* stale request.
   Why it matters: `doc.edit_apply` needs the write lock on the same map (`session.rs:212-225`), so the first preview shape of a document stalls every mutation — of every open document — for the length of a full static compile (the plan budgets that at up to 5 s for DejaVu and 60 s for Source Han Sans SC, against a 4 ms drag-commit budget), and each subsequent keystroke during the debounce window copies the entire model under the same lock. The comment at `handlers/shaping.rs:16-18` ("copying a large font on every keystroke would cost far more than the shaping itself") is contradicted by `shaping.rs:187`, which copies it anyway on the stale path.
   Fix: do what the export handler already does — clone the font (cheap relative to a compile; `original_bytes` is an `Arc`) inside `with_doc`, release the lock, then compile/shape outside it. That also removes the double copy, since the snapshot can be handed straight to `PendingBuild`.

2. **A panic in the background rebuild wedges that (doc, master) into permanent staleness.**
   `crates/fw-layout/src/shaping.rs:205-217` spawns a detached worker and sets `state.rebuilding = true`; `rebuilding` is only ever cleared by the worker itself at `shaping.rs:271`. If `CompiledFont::build` panics (fontc/fea-rs on unusual input — the workspace rule is that *our* code must not panic, but this calls into third-party compile code), the thread dies before that line, `rebuilding` stays `true` forever, no further worker is ever spawned for that entry, and every later `text.shape` returns the old font with `stale: true` for the rest of the session, with `pending` holding a leaked `Font` clone. Compile *errors* are handled correctly (logged, loop continues) — only the panic path wedges.
   Fix: clear `rebuilding` from a drop guard on the worker (or re-check liveness when scheduling), so a dead worker is replaced rather than blocking all future rebuilds.

#### Minor (Nice to Have)

3. `crates/fw-layout/src/shaping.rs:161-163` — `forget(doc)` drops the map entry but the running worker holds an `Arc<Entry>` and will still perform one full `compile_static` for a document that has just been closed, then exit. There is no shutdown signal and no join anywhere; a worker outlives the `Api` by up to `debounce` + one compile. Bounded, but a `Weak`/cancel flag would stop closing a big CJK document from burning a core for a minute.
4. `crates/fw-api/src/api.rs:176-195` — the event forwarder is keyed to *session* lifetime (`RecvError::Closed`), not `Api` lifetime, so an `Api` dropped while its `Arc<Session>` is still held elsewhere leaks a thread forwarding into a dead channel. Also, `event_bridge` is a `OnceLock<()>` set by `get_or_init` even when `thread::Builder::spawn` fails (`api.rs:190-193`): after a failed spawn every later `subscribe()` silently returns a receiver that will only ever see progress events.
5. `crates/fw-api/src/api.rs:168-174` vs `:176-195` — progress events are sent straight onto the API channel while document events cross the forwarder thread, so a progress event can be delivered before a document event that happened first. The report acknowledges this; it is invisible to the current UI but is an ordering guarantee nobody can rely on later.
6. `crates/fw-api/src/handlers/export.rs:38-73` — on a failed compile the handler returns early after emitting `compile 0`, so no terminal progress event is ever sent and a UI progress bar stays stuck. Emit a final stage (or a failure event) on the error path.
7. `crates/fw-api/src/handlers/recipe.rs:68-81` — ranges are unbounded: `U+0000-U+10FFFF` expands to ~1.1 M `RecipeItem`s and then a single `Command::Batch` of ~1.1 M `AddGlyph`s. No panic (surrogates are filtered by `char::from_u32`), but a typo in a text field can hang or OOM the app. A cap (with the item reported as skipped) would be cheap; `@charset:GB2312` needs ~6.7 k, so a limit of a few tens of thousands is safe.
8. `crates/fw-api/src/handlers/recipe.rs:44-66` — a bare non-ASCII character (`中`) is `Unknown`, while the same character as a range endpoint (`中-中`) resolves fine (`scalar()` at `:94-104` accepts any single char). Accepting single literal characters in `single()` would make the grammar consistent.
9. `crates/fw-api/src/handlers/editing.rs:67-99` — `glyphs.add` takes the session lock three separate times (plan, apply, read-back); a concurrent add of the same name between plan and apply makes the whole batch fail rather than skipping. Single-writer UI makes this unlikely, but planning and applying inside one `with_doc_mut` would remove the window.
10. `crates/fw-api/src/handlers/session.rs:125-138` — `recover` opens the snapshot with the *original* document path as the document path and only marks it dirty; `DocSummary` has no "recovered from" marker, so the UI cannot distinguish a recovered document from a normally opened one and a plain `session.save` overwrites the original file. The dispatch asked for the original path as provenance; consider an explicit flag on `DocSummary`.
11. `crates/fw-compile/src/metadata.rs:250-256` — `min_by_key(|(key, _)| key.lang_id)` over a `HashMap` picks an arbitrary record when two entries share the lowest `lang_id` (same language, different platform, different text), making the synthesized English text non-deterministic across runs. Break the tie on the text (or on platform id) to keep exports reproducible.
12. Coverage gaps: no API-level export test on an imported binary font asserting the preservation contract (untouched tables byte-identical / instruction-drop warning listing the edited glyphs) — `crates/fw-api/tests/export.rs` only uses model-born fonts; and no test that a `text.shape` compile failure is surfaced sensibly (today it stays `stale: true` forever with no diagnostic).
13. `crates/fw-api/src/handlers/export.rs:43` — `preserve 60` is emitted *after* `fw_compile::export` has already done both compile and preserve, so the two intermediate stages are cosmetic rather than real progress. Fine for P1 given the ruling on debounced recompiles, but the stage names promise more than they deliver.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** The surface is complete and faithful to the brief and the controller's rulings, the concurrency handshake in the shaper is correct, and the tests are real; but `text.shape` performs a multi-second compile and a whole-model clone while holding the session lock (Important #1, directly against the plan's interactive budget and contradicted by its own comment), and a panicking rebuild worker permanently wedges shaping for that document (Important #2).
