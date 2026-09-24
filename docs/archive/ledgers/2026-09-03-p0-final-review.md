Review complete. Working tree untouched (`git status` clean, HEAD still `cc2068b`).

### Strengths

- **The seams hold.** The `Change`/`DocumentEvent` contract is consumed coherently end to end: kernel bumps per-glyph generations → `sessionStore` (`stores/sessionStore.ts:96-118`) updates dirty/undo labels and calls `noteGeneration` → `documentStore.sidebarGeneration` drives a debounced sidebar refresh (`views/font/SidebarFilters.tsx:43-57`) → `GlyphPager` rejects answers stamped with an older generation (`views/font/glyphPager.ts:139-146, 234-243`) → `pathCache` is keyed by `doc:gid:generation`. Every layer of the invalidation chain has a distinct, documented reason to exist, and each is unit-tested.
- **The hand-resolved merge points are genuinely resolved, not just compiling.** `FontInfoView` carries `tables` *and* `instances` and both are rendered (`handlers/doc.rs:19-27`, `views/fontinfo/FontInfoPanel.tsx:96-110`); `forgetFontMetrics` correctly delegates to the single owner `forgetFontInfo` (`views/font/useFontMetrics.ts:37-39`); `glyph.open` uses focus-then-selection targeting with the cached name (`shell/builtinCommands.ts:37-45, 182-193`); the duplicate `tempfile` workspace dep is gone.
- **Robustness on untrusted fonts is real, not claimed.** Zero `unwrap`/`expect`/`panic!` in non-test library code across all eight crates; `glyf` contour slicing is bounds-checked (`read/glyf.rs:105-116`), `cmap` format-2 reads go through a fallible `read_u16`, and `tests/robustness.rs` feeds 10 truncation points per fixture. Write paths are narrow: `session.save` rejects non-`.fwf` targets *and* refuses to infer a path for an imported font (`handlers/session.rs:65-91`), `fw convert` validates its output extension, and autosave only ever writes `<datadir>/<sha1>.autosave.fwf`.
- **Verified green at HEAD:** `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test -p fw-api -p fw-doc` green, `pnpm test` 302/302 in 37 files, `pnpm lint` clean. i18n is complete and correct: 407 leaves in both catalogues, no missing/extra keys, no placeholder mismatches, and the 20 identical values are all legitimately identical (proper nouns, `OS/2`, `PANOSE`).
- Test hygiene is better than the ledger suggests: every timing assertion is gated behind `cfg!(not(debug_assertions))` and CI runs debug only; the `FW_CONFIG_DIR` mutations in `fw-api/tests/dispatch.rs` are serialised by a `CONFIG_ENV_LOCK` guard, and the only other site lives in a separate test binary. Every test file contains at least one assertion.

### Issues

#### Critical

**1. CI cannot pass on a clean checkout — the generated API bindings are gitignored and never generated.** `.gitignore:11` ignores `/app/src/api/generated/*` (only `.gitkeep` is committed — confirmed with `git archive HEAD`), `app/src/api/index.ts:1` does `export * from './generated'`, and `.github/workflows/ci.yml:54-69` runs `pnpm check`/`lint`/`test`/`build` with no `cargo run -p fw-api --bin gen_ts` step. Verified empirically by type-checking a `git archive HEAD` copy of `app/`:

```
src/api/index.ts(1,15): error TS2307: Cannot find module './generated' or its corresponding type declarations.
src/stores/documentStore.ts(5,8): error TS2305: Module '"../api"' has no exported member 'DocId'.
… ~40 more; tsc exit 2
```

Why it matters: the merge gate for main is red for the whole frontend half of the pipeline, and every future PR inherits it. `app/README.md` already documents `pnpm gen:api` as a prerequisite, so this is a pure CI omission. Fix: add a step before "Install frontend dependencies" running `cargo run -p fw-api --bin gen_ts` (it only needs the Rust toolchain, already installed at that point).

#### Important

**2. `.fwf` is written compact, silently dropping spec §5's readable-diff property, with no ledger ruling.** `crates/fw-doc/src/fwf/write.rs:36,42` use `serde_json::to_writer` (single line). Spec §5 requires "deterministic key order and glyph order, 2-space indent, one glyph object per line group. This keeps `git diff` readable" — the stated justification for choosing a single JSON file over a directory package — and plan Task 5 says "written with `serde_json` pretty printer, 2-space indent". Commit `4f2d91c` ("perf(fw-doc): compact native document JSON") changed it with no rationale in the message, no ledger entry, and `fwf/schema.md` documents only the envelope, not the layout. Fix: either restore pretty-printing and re-run the save+load gate (currently 4.2 s of a 6 s budget — pretty may not fit), or record an explicit ruling amending spec §5 and state the layout in `schema.md`. This must be a decision, not silence, because P1's UFO/Glyphs work and any "diffable sources" claim depend on it.

**3. Spec §14's P0 exit criterion "CLI converts TTF → fwf → TTF (glyph-identical)" is unmet and unruled.** Plan Task 8 deliberately inverts it ("`fw convert x.fwf y.ttf` must fail with a clear 'not yet supported' message"), `crates/fw-cli/src/cmd/convert.rs:31-46` enforces that, and `docs/p0-acceptance.md` quietly restates the criterion as "CLI convert TTF → FWF → inspect preserves glyph count". Binary export is genuinely P1 (`fw-compile`/fontc), so the code is right and the *record* is wrong. Fix: add a ledger ruling amending the P0 exit criterion, and mark it as deviated (not met) in `docs/p0-acceptance.md` rather than restated.

**4. sfnt reader warnings never reach the UI.** `crates/fw-api/src/handlers/session.rs:198-204` logs `outcome.warnings` via `tracing` and returns only the TTC face-count string (`:181-190`), so `DocSummary.warnings` — which the UI surfaces as toasts (`stores/sessionStore.ts:78-81`) and counts in the status bar (`shell/StatusBar.tsx:51-55`) — is always empty for the two codes the reader actually emits (`cff-hints-not-imported`, `glyph-names-synthesized`). The CLI reports them (`docs/p0-acceptance.md` font-inspection matrix); the app is strictly worse, and "glyph names were synthesised" is exactly the kind of thing a user must know before editing. Fix: extend the returned vector with `outcome.warnings`. Note the coupling to the Minor below — those strings land in toasts untranslated, so prefer passing `code` and mapping known codes to i18n keys with a raw-message fallback.

**5. Autosave is implemented but never started, and nothing recovers it.** Spec §4 requires "Autosave every N seconds to a sidecar, restored on next launch after a crash". `fw_doc::autosave::spawn` and `list_recoverable` have no callers outside `crates/fw-doc` (grepped across `crates/*/src` and `src-tauri/src`), and `Prefs.autosave_seconds` is exposed but unwired. Impact today is nil — P0 has no editing UI, so no document can become dirty (the app calls only `doc.undo`/`doc.redo`/`session.save`; `doc.apply` and the edit-token trio are unused) — which is precisely why it slipped through 13 task reviews. Fix now: record the ruling (deferred to P1 with the editor) and add it to the P1 plan; wiring it in P0 would be untestable.

**6. The order-position cache is only invalidated on `glyph_order`, while the pager's staleness rule is broader.** `app/src/views/font/GlyphGrid.tsx:177-193`: `forgetOrder(doc)` sits inside the `if (event.change.glyph_order)` branch, but `requiresFullReload` (`views/font/glyphPager.ts:255-263`) also fires for any non-`Order` sort or any active filter. So after e.g. a `SetUnicodes` edit under Unicode sort, `glyphOrder.ts`'s gid→index map keeps pre-edit positions and the glyph tab's next/previous (`views/glyph/GlyphView.tsx:118-125` → `neighbourGid`) steps to the wrong glyph. This is exactly the seam that commit `7111fe1` hand-merged (Task 10's generation-tagged reset + Task 11's `forgetOrder`), and it is latent only because P0 cannot edit. One-line fix: also call `forgetOrder(doc)` in the `requiresFullReload` branch.

#### Minor

- `crates/fw-sfnt/src/read/mod.rs:32` — `ReadOptions.decode_bitmaps` is never read by anything, yet all three production call sites pass `true` (`fw-api/handlers/session.rs:195`, `fw-doc/session.rs:44`, `fw-cli/cmd/mod.rs:50`), implying decoding happens. Either honour it or document it as reserved for P3 and pass `false`.
- `crates/fw-model/src/ids.rs:15,72,98` — `ts(type = "String")` generates the *boxed* TS type (`export type MasterId = String`), which is not assignable to `string`; the app pays for it with 32 `.toString()` calls. Changing three attributes to `"string"` is strictly relaxing (no new type errors) and stops the wart spreading through P1's editor code. Best value-for-effort item in the whole deferred list.
- Kernel-produced strings bypass `t()`: `ApiError.message` and `DocSummary.warnings` go straight into toasts (`stores/sessionStore.ts:29-33,78-81`; `shell/AppShell.tsx:45-50`), e.g. "collection has 2 faces; opened face 0". CONVENTIONS requires every user-visible string through `t()`; the kernel already carries stable `code` fields to map from.
- Prettier is not clean: `npx prettier --check src e2e` reports 83 files, CONVENTIONS requires it, and CI has no `--check` step (so the debt is invisible). Fix in one pass plus a CI step, or drop the requirement from CONVENTIONS.
- `crates/fw-devserver/src/server.rs:44-62` mutates `FW_CONFIG_DIR` with `std::env::set_var` inside library `serve()` for a test-only knob — process-global and unsafe from Rust 2024. Prefer threading the config dir into `Api`.
- `crates/fw-sfnt/src/read/metrics.rs:108-135` derives x-height/cap-height from glyphs literally *named* `x` and `H`; CID fonts (`cidNNNNN`) and renamed sources get no fallback. Prefer a cmap lookup of U+0078/U+0048.
- `app/src/views/fontinfo/FontInfoPanel.tsx:44` calls `useFontInfo(summary.doc)` with no reload token, so the Font Info tab never refreshes on `change.font_info` although the mechanism exists and `GlyphView.tsx:47` uses it.
- After an edit-triggered full reload, the grid loses its scroll position: `pager.reset()` nulls `total` → `contentHeight` 0 → the browser clamps `scrollTop` to 0 (`GlyphGrid.tsx:189-191`).
- `crates/fw-sfnt/tests/robustness.rs:5-12` omits `NimbusRoman-Regular-o.otf` from the truncation corpus.
- e2e is not self-contained and not in CI: `app/playwright.config.ts` starts only Vite; `fw-devserver --port 8787 --open fixtures/fonts/DejaVuSans.ttf` must be started by hand (documented in the acceptance record, but `pnpm e2e` alone fails).
- `#![warn(missing_docs)]` is present in fw-model/fw-doc/fw-api but absent from fw-sfnt, fw-glyphdata, fw-devserver, fw-cli, fw-fixtures — the "docs on every pub item" rule is enforced for 3 of 8 crates.
- `src-tauri/tauri.conf.json:24` sets `"csp": null` (no content-security policy for the webview).
- `app/src/views/font/GlyphGrid.tsx` is 425 lines against the "≤ ~400" convention (the input handlers are the natural extraction).
- `Document::begin_edit` (`crates/fw-doc/src/document.rs:143-155`) silently discards a live edit token, and `apply()` does the same; harmless with one window, worth a doc note before P1 adds multiple editors.

### Deferred-minor triage

| Item (ledger) | Verdict | Reason |
|---|---|---|
| T9 boxed `MasterId`/`GlyphName`/`LayerId` TS type | **must-fix-now** | 3 attributes in `ids.rs`; strictly relaxing; otherwise `.toString()` noise spreads through all P1 editor code |
| T1 `commands.rs` placeholder | already resolved | `src-tauri/src/commands.rs` is the real `api_call` |
| T6 `set_var` in tests | already mitigated | `CONFIG_ENV_LOCK` guard serialises them; other site is a separate test binary |
| T1 redundant `.gitignore` lines | defer to P1 | cosmetic (`/target/` + `**/target/`) |
| T2 dead version guard, dependents O(k²), validate misses kern-group fields, duplicated `MAX_COMPONENT_DEPTH` | defer to P1 | no P0 path stresses them; validate coverage belongs with P1 "Find problems" |
| T3 unused scaffold `Error` enum in fw-glyphdata | defer to P1 | dead code, zero risk |
| T4 platform 2/4 name records, `head.created` format, cmap streaming, CID dup-name suffix, hint second pass | defer to P1 | needs model changes (P1 UFO task) or is a perf micro-opt; import fidelity is P1/P2 territory |
| T4/T11 CID fonts show `cidNNNNN` names | defer to P1 | cosmetic until editing; fix in `fw-sfnt/names.rs` with cmap-derived `uniXXXX` |
| T5 unreachable undo-budget test, cache eviction, poison policy, save-event docs, rename-mid-batch test | defer to P1 | budget/eviction unreachable at P0 scale; batch rollback *is* implemented (`command.rs:216-231`), only the test is missing |
| T6 candidate-list cache, cheap metadata page, sidebar cache "leak" | defer to P1 | page gate measured 22 ms of 30 ms; `session.close` already clears both caches (`handlers/session.rs:44-53`) |
| T6 `SetGlyphMeta` `Option<Option<u8>>` JSON ambiguity | defer to P1 | already routed to the P1 fw-doc follow-up; no P0 caller |
| T7 `window` guard under node env | defer to P1 | tests inject via `setTransport`; only affects a hypothetical node consumer |
| T8 collection-warning helper duplicated in 3 crates | defer to P1 | fold into `fw-sfnt` when warnings are surfaced (Important #4) |
| T9 kernel prefs wiring (localStorage), hard-coded version/upem in New Font, per-document Tauri windows | defer to P1 | plan puts preferences in P1; `prefs.get/set` are implemented and tested kernel-side |
| T10 kernel `glyphs.ids` for Mod+A, `encoded_only` flag, devserver CORS for extra ports | defer to P1 | Mod+A currently costs 33 requests on 65k glyphs — works, just wasteful |
| T10 repo-wide Prettier pass | defer to P1 | 83 files; pair it with a CI `--check` step so it stays fixed |
| T11 smooth-point inference, narrow toolbar, component label overlap | defer to P1 | display polish; smooth points need the P1 editor anyway |

### Recommendations for P1

- Add the `gen_ts` step to CI first, then consider committing the generated bindings (or a `pretest`/`prebuild` hook) so no future contributor hits the same wall.
- Decide the `.fwf` layout question deliberately: compact for huge fonts, pretty for human-scale, or pretty + `.fwf.gz` above a size threshold — and write the decision into `fwf/schema.md`.
- Give warnings a typed identity across the boundary (`{code, message}` instead of `Vec<String>`), then map codes to i18n keys in the UI; the same mechanism should carry `ApiError.code` to translated toasts.
- Wire autosave and crash recovery together with the editor, and reconcile the sidecar naming with spec §4 (the current data-dir + sha1 scheme is safer — amend the spec rather than the code).
- Make invalidation one function: today `pager.reset`, `forgetOrder`, `forgetGlyphNames` and `pathCache` are invalidated by three different predicates in one effect. A single `invalidateFor(change, query)` in `views/font/` would have prevented Important #6.
- The grid ignores `change.masters` entirely (no glyph generations bump, no em-box refresh) — metrics editing in P1 needs that path.
- Add `glyphs.ids` (handles only, no cells) for select-all, and a `changed_only`/`encoded_only` fast path; the candidate list is rebuilt over all 65k glyphs per page request, which will not survive live editing.
- Keep `Command` additive: adding `Kerning`/`Features`/`Bitmap` variants is non-breaking for the TS union, but `SetGlyphMeta`'s `Option<Option<u8>>` should be fixed before more optional-clearing fields appear.
- Put e2e in CI with the kernel server started by Playwright's `webServer` array (two entries), so the CJK/grid regressions stay caught.
- Enforce `#![warn(missing_docs)]` in the remaining five crates and add `prettier --check` once the one-time pass lands.

### Assessment

**Ready to merge? With fixes.** One Critical (CI is red for the entire frontend pipeline on a clean checkout) plus two one-line code fixes (#4 warnings, #6 `forgetOrder`) and three record-keeping fixes (#2 `.fwf` layout ruling, #3 P0 exit-criterion deviation, #5 autosave ruling) are all that stand between this branch and main — the engineering itself is in good shape: clean lints, 302 + 188 green tests, all three performance gates inside budget, coherent cross-task seams, and no panic or write-path hazard on untrusted fonts.