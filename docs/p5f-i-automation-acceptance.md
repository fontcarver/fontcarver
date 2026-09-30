# p5f-i acceptance — cross-document workflows, scripting and plugins

Date: 2026-09-30. Area `i-automation`; feature-audit rows **77, 78, 79, 96, 105
and 106**. Task file: [`p5f-i-automation`](superpowers/plans/tasks/p5f-i-automation.md)
(its `## Gap analysis` records what was missing before this work and every
decision taken without asking). Protocol:
[`p5f-protocol`](superpowers/plans/tasks/p5f-protocol.md).

Shared checkout with seven other tasks. Ports: API 8809, UI 1509.

The browser session ran against
`target/debug/fc-devserver --port 8809 --open fixtures/fonts/SourceSans3Min.otf --open fixtures/fonts/SourceSerif4Min.otf`
with `FC_CONFIG_DIR` pointing at a task-local configuration directory, and Vite
on 1509 proxying to it. Because the other seven tasks were editing `app/src`
throughout, the first attempts were interrupted every couple of minutes by
hot-module reloads; the session was therefore re-run with a task-local Vite
config that serves the same application with HMR and the file watcher disabled.
That config lived in task scratch and is not part of the repository.

## What this report claims

Every command below was run in this checkout and its summary line is quoted
verbatim. Browser evidence comes from one headless session driven against
`fc-devserver` and Vite on the assigned ports, with the two committed CFF
fixtures plus `NotoSans-Regular.ttf` and `DejaVuSansMin.woff2` as reference
binaries. Screenshots cited here are in
[`screenshots/p5f-i-automation/`](screenshots/p5f-i-automation/):

| File                                 | Shows                                                                 |
| ------------------------------------ | ---------------------------------------------------------------------- |
| `01-script-folders.png`              | The three searched script roots and the extra-folder preference.       |
| `02-plugin-index-provenance.png`     | The curated index with author, licence and shipped path.               |
| `03-plugin-settings.png`             | Per-plugin persisted settings and the lifecycle buttons.               |
| `04-reporter-overlays.png`           | Bounds and metric overlays drawn on the glyph canvas.                  |
| `05-compare-fonts.png`               | Counts, setup differences and the per-glyph resolution column.         |
| `06-merge-review.png`                | Kerning/layout sections and the reviewed merge.                        |
| `06-metadata-differences.png`        | The metadata table wrapping long values instead of clipping them.      |
| `07-compare-family.png`              | The family matrix with the reference member and a mismatched member.   |
| `08-reference-binary.png`            | `NotoSans-Regular.ttf` opened as a read-only reference.                |
| `09-python-run.png`                  | A Python run and its single recorded undo entry.                       |
| `10-paste-special-placeholder.png`   | Paste Special preview naming the placeholder and its filter.           |
| `11-placeholder-in-inventory.png`    | The created placeholder in the target's glyph list.                    |
| `12-api-reference.png`               | The shipped declarations and the method search.                        |
| `13-script-check.png`                | An unknown method blocking Run before execution.                       |

## Starting point: the suites that had never run

`docs/acceptance/p5-automation.md`, its retained build log and
`docs/p5-cross-document-acceptance.md` recorded their backend suites as queued
and cancelled, never passing. The first action of this task was to run them
unchanged against the current checkout:

| Command                                     | Result    |
| ------------------------------------------- | --------- |
| `cargo test -p fc-api --test automation`    | 20 passed |
| `cargo test -p fc-api --test cross_paste`   | 8 passed  |
| `cargo test -p fc-api --test font_compare`  | 10 passed |
| `cargo test -p fc-doc --test replace_glyph` | 7 passed  |
| `cargo test -p fc-api --bin gen_ts`         | 3 passed  |
| `cargo test -p fc-devserver --test http`    | 5 passed  |

`docs/acceptance/p5-automation-build-blocked.log` describes a compile failure
that no longer exists; it is superseded by the runs above and by this report.

## Row 77 — Paste Special matrix

**Implemented before this task**, verified here.

All three destinations (same name, selected target glyphs, as components) and
all four payloads (all glyph data, selected master layer, kerning group
membership, metric keys) are reachable in **Edit → Paste Special…** and are
covered cell by cell by `destination_and_content_matrix_preserves_unselected_fields`.
The source glyph is given a distinct drawing and advance, so each cell asserts
what actually moved: a drawing payload must reproduce the source contours and
advance in the destination and repoint the component at the copied base; a
metadata-only payload must leave the destination's contours, advance and
components untouched; fields outside the payload must keep their target values;
and one undo must restore the document exactly. A no-op paste fails every
drawing cell. `preview_copy_undo_redo_and_native_persistence_are_atomic`
covers `.fcf`/`.fcf.gz` save and reopen after a paste;
`stale_source_and_late_mapping_failure_never_modify_destination` covers atomic
failure on a stale revision or a bad mapping.

Browser: with `SourceSans3Min.otf` and `SourceSerif4Min.otf` open, pasting
`obreve` same-name/all-data from the second into the first previewed
`1 glyphs to paste; 1 additional bases`, applied as the single undo entry
`Paste special`, and raised the target from 5 to 7 glyphs
(`10-paste-special-placeholder.png`).

## Row 78 — Cross-document relinking and placeholders

**Gap closed**: the placeholder a transfer creates was tagged but nothing read
the tag, so it was not "flagged in the inventory".

- `PasteSpecialResult` now carries `placeholders: [{name, source_base, tag}]`
  and the warning names the tag. The dialog lists every placeholder it will
  create and prints the smart filter that finds them again.
- `PLACEHOLDER_TAG` (`missing-component-base`) is now a written contract,
  documented in `crates/fc-doc/src/fcf/schema.md`.
- `placeholders_are_reported_with_the_tag_that_finds_them_in_the_inventory`
  parses that filter through `glyph.smart_filters.parse` and then runs
  `glyphs.list` with it, asserting the placeholder is the only match and that
  it does not export.

Browser: the placeholder `brevecomb` appears in the target's glyph list drawn as
a non-exporting cell (`11-placeholder-in-inventory.png`), and the live filter
`tags~"missing-component-base"` returned exactly `[('brevecomb', False)]`.
**Edit → Relink Components…** then repointed `brevecomb` to `V`; the component
bases became `['o', 'V']` with transforms intact, in one undo entry
`Relink components`, and a relink that would make `obreve` reference itself was
refused with `component cycle at glyph 'obreve'`.

Coordination note for task b: the inventory marks the placeholder only because
it does not export. A dedicated badge would need one boolean on `GlyphCell`
(for example `placeholder`) set from the same tag; that type is task b's and was
not touched.

## Row 79 — TypeScript scripting, console, scripts folder

Four gaps closed.

**One history transaction per run.** `Document::apply` used to finalize any live
gesture, so every kernel call a script made was its own undo entry — the shipped
user guide stated this as a boundary. `fc-doc` now has a *script transaction*
(`begin_transaction` / `end_transaction`): while one is open, plain `apply`
calls join it, so a run of any length collapses into one entry. The console
opens one around every JavaScript run that captured a document and always closes
it in a `finally`; `automation.python_run` does the same server-side with an
RAII guard. History navigation inside a run is rejected rather than silently
splitting history.

**A leaked transaction is bounded two ways.** A transaction carries a deadline
(the console asks for 60 s, the Python bridge for its own deadline plus 30 s,
both capped at 120 s). Every later apply, gesture, undo, redo and save reaps an
expired one first, so a reloaded page or a dropped transport costs at most that
one entry and never blocks history afterwards. Independently, `fc-api` records
the transactions it opened: starting another run supersedes them
(`Document::finalize_transaction`), and listing scripts reaps only the expired
ones (`finalize_expired_transaction`), which is safe to call while a run is
live. The console's `transaction_end` is itself wrapped, so a failing close
reports a warning instead of masking the run's result. Eleven `fc-doc` tests and
three `fc-api` tests cover coalescing, rejection, gesture supersession, saving
mid-run, an empty transaction, expiry, host supersession, lifetime validation
and the read-only guards.

Browser: running `widen.js` (three `doc.apply` calls) reported
`Recorded one undo entry: Script: widen.js`; `doc.summary` returned that exact
undo label, and one `doc.undo` restored all three widths
(`undo: None redo: Script: widen.js dirty: False`). The Python run `compose.py`
(four kernel calls) likewise recorded `Script: compose.py`.

**Declarations shipped and consumable.** `gen_ts` now also writes
`app/src/api/generated/scriptingManifest.ts`: the exact text of
`fontcarver.d.ts` plus, for every kernel method in `fc_api::METHODS`, the
parameter and result type names, whether the method is available to scripts,
and each accepted parameter key read back from the generated `ts-rs` type. A
generator test asserts that no method is missing from the manifest, so a method
another task adds appears there as soon as `gen:api` runs. The console's **API
reference** tab shows the declaration path and text, copies it, searches methods
by name or parameter key and inserts a ready-to-run call
(`12-api-reference.png`).

**Type checking in the console.** Before a JavaScript run, every literal
`fc.call` is checked against that manifest. An unknown or withheld method is an
error and blocks Run; an unknown parameter key is a warning. A *missing* key is
deliberately not reported: many kernel parameters carry `#[serde(default)]`,
which the generated TypeScript cannot distinguish from a required field, so the
check would reject valid calls such as `glyphs.list` without `filter`. The scan
skips strings and comments, understands shorthand properties, and refuses to
guess past a computed method name, a spread or a computed key. Browser: a draft
calling `doc.rename_everything` reported
`Line 3: doc.rename_everything is not a FontCarver API method.` with Run
disabled (`13-script-check.png`).

**Folders and reload on change.** Scripts are now discovered from three roots —
a configured preference folder, the application configuration folder, and a
`scripts` folder beside the captured document's project — each listed with its
path and whether it exists, with root-scoped reads. The preference is stored in
`automation.json` in the configuration directory and validated (absolute,
existing, not a symlink). `automation.scripts_list` returns a digest of the
searched roots, and the console re-reads them every two seconds while it is
open, refreshing the list when the digest changes. Browser: all three roots were
listed, a script dropped into the configured folder appeared under it
(`01-script-folders.png`), and a script written to disk during the session
appeared after the automatic re-read. The screenshot predates the review fix
that renamed the folder labels to **Configured folder** and **Preferences
folder**, matching their keys; the paths and behaviour it shows are unchanged.

## Row 96 — Compare Fonts / Family / reference binary

Seven gaps closed.

- **Metadata, kerning, layout and components.** `font.compare` now also reports
  every differing font-info field with both values, kerning-group and
  kerning-record differences for the selected masters with a bounded sample of
  differing pairs, editable OpenType layout differences (prefixes, features,
  classes, authority and assembled source), and a per-glyph `components_changed`
  flag with its own count, separate from the evaluated outline. The
  compatibility flags gained `metadata`, `kerning` and `layout`.
  `metadata_kerning_and_layout_differences_are_reported_with_flags` and
  `component_differences_are_reported_separately_from_the_evaluated_outline`
  cover these; the second changes only a component's anchor, so the evaluated
  path is byte-identical and only the component flag moves.
- **Family across masters.** `font.compare_family` compares every
  (document, master) member of the family against one reference member and says
  whether the whole family matches; a member that cannot be read is reported in
  its row instead of abandoning the report.
  `family_comparison_covers_every_master_and_marks_mismatched_members` covers a
  second master that really differs and a second document with a different
  master set.
- **Read-only reference binaries.** `font.open_reference` opens a font through
  the existing importer and marks the document read-only in the kernel.
  `a_reference_binary_opens_read_only_and_the_kernel_rejects_every_edit` asserts
  the refusals and that the font is unchanged afterwards.

  The review found one unguarded path, now fixed: `commit_prepared_features`
  writes the font and pushes an undo entry without going through `apply`, and is
  reachable on a reference through `features.apply`, `layout.apply_conversion`
  and `layout.revert_original`. It now calls `check_writable()` first.
  `a_read_only_reference_refuses_prepared_feature_commits` drives the reachable
  endpoint end to end — read-only inspection (`features.get`,
  `features.validate`) still works, `features.apply` is refused with the
  read-only message, and neither the font nor its history changes — and
  `a_read_only_document_rejects_the_prepared_feature_commit` covers the kernel
  entry point directly.

  Every `pub fn` on `Document` that takes `&mut self`, across
  `crates/fc-doc/src/document.rs`, `crates/fc-doc/src/document/features.rs` and
  `crates/fc-doc/src/edit.rs`, was then audited:

  | Function                                    | Guard                                                        |
  | ------------------------------------------- | ------------------------------------------------------------- |
  | `apply`                                     | `check_writable()`                                            |
  | `apply_in_edit`, `preview_edit` (an alias)  | `check_writable()`                                            |
  | `begin_transaction`                         | `check_writable()`                                            |
  | `undo`, `redo`                              | `check_writable()`                                            |
  | `save`                                      | `check_writable()`                                            |
  | `commit_prepared_features`                  | `check_writable()` — added by this fix                        |
  | `begin_edit`, `end_edit`, `end_transaction` | none needed: they open or close a token; every write inside them goes through `apply_in_edit` |
  | `finalize_transaction`, `finalize_expired_transaction` | none needed: they only close a transaction |
  | `set_undo_budget`                           | none needed: changes the history budget, never the font        |
  | `set_read_only`                             | none needed: it is the switch itself                           |
- **Conflict-resolved merge.** `font.merge` takes a `target`/`source` decision
  per glyph plus explicit font-level adoptions (metadata, master metrics,
  kerning), validates the whole plan in a scratch session and applies it as one
  `Command::Batch` under the label `Merge fonts`.

Browser (`05-compare-fonts.png`, `06-merge-review.png`, `07-compare-family.png`,
`08-reference-binary.png`):

- Comparing the two fixtures reported
  `0 equal · 2 outlines · 2 metrics · 0 Unicode · 0 components · 3 missing`,
  `Differences in font setup: Family name, Glyph inventory, Master metrics, Metadata`,
  a metadata table (copyright, PostScript names, PANOSE, versions…) and
  `Kerning groups and records match.` / `Editable layout source matches.`
- Choosing *Take the reference* for `.notdef`, *Keep this font* for `o` and
  ticking *Take reference metadata* previewed
  `1 glyphs replaced or added, 1 kept, 0 extra bases. History entry: Merge fonts.`
  Applying reported `Merged 1 glyphs as one undo entry (Merge fonts).`; the
  family name became the reference's, and one `doc.undo` restored the original
  family with `redo: Merge fonts`.
- **Compare open family** listed the reference member and the mismatched member
  with its flags and difference counts.
- `NotoSans-Regular.ttf` opened as a read-only reference (3743 missing glyphs
  against the small fixture) and `doc.undo` on it returned
  `this document is opened as a read-only reference and cannot be modified`.

**Export and reopen, verified independently.** `a_merged_document_saves_reopens_and_exports_with_the_merged_glyphs`
saves the merged document to `.fcf`, reopens it, checks the glyph order and the
adopted family name, exports a TTF, opens that binary as a read-only reference
and compares it back. The same flow was run through the development server and
the exported font checked with three independent libraries:

```
fontTools: glyphs = 5 ['.notdef', 'space', 'A', 'V', 'o']
fontTools: family = FontWorkshop CFF Serif Fixture | upem = 1000
fontTools: advance o = 549 | A = 544
fontTools: cmap size = 4
FreeType: family = FontWorkshop CFF Serif Fixture | glyphs = 5 | o bitmap = 22 x 25
HarfBuzz: shaped = [(4, 549), (2, 530), (3, 515)]
```

`o` carries the reference's advance because it was merged; `A` keeps the
target's because it was not; the shaped advances differ from `hmtx` because the
target's own GPOS kerning still applies — which is the documented boundary that
a merge does not adopt layout.

## Row 105 — Python RPC and the plugin manager

The Python side needed no new behaviour: the 20 previously unrun tests pass and
now also assert the run-level transaction. The plugin manager had three gaps.

- **Curated index with provenance.** The index is now the bundled JSON file
  `app/src/plugins/index.json`, validated at load: an entry is dropped unless a
  shipped implementation exists with the same id, kind and version and the
  entry carries an author, a licence, a repository path, `origin: "bundled"` and
  well-formed settings. `registry.test.ts` covers the shipped index and twelve
  malformed variants.
- **Reload.** `usePluginStore.reload()` re-reads the curated index and the
  persisted installation record without fetching anything, and the manager
  exposes it.
- **Persisted settings.** An entry declares numeric or toggle settings; values
  are clamped to the declared range, step and kind, persisted next to the
  installation, and repaired on load.

Browser (`02-plugin-index-provenance.png`, `03-plugin-settings.png`): the
manager shows `Author FontCarver project · Licence MIT OR Apache-2.0 · Shipped
from app/src/plugins/reporters.ts` per entry; installing, enabling and toggling
`Also mark the advance width` produced exactly
`{"fontcarver.bounds":{"enabled":true,"version":"1.0.0","settings":{"advance":1}}, …}`
in browser storage. Three sample plugins were exercised end to end — two
reporters drawing on the canvas and the Translate filter previewing and
applying.

## Row 106 — Reporter and filter contracts

**Gap closed**: reporters received only the outline bounds, so the required
"document/view context" was absent. `ReporterInput` now carries the document
context (document, glyph, layer, master and the resolved layer metrics), the
view context (zoom and the visible font-space rectangle) and the plugin's
persisted settings, and overlays gained a font-space `line`. A second bundled
reporter, **Metrics reporter**, exists precisely because it needs both: its line
heights come from the document and their extent from the view, so they span
exactly what is visible at any zoom. `drawPluginOverlays.test.ts` (21 tests)
covers the computed view rectangle, both reporters, a setting that suppresses
part of the drawing, a metric line outside the visible region, and every
existing attach/detach and preview case.

The filter contract needed no change and is covered by the existing 20
`filterSession` and 7 `PluginManager` tests. Browser: with glyph `A` selected,
the Translate filter previewed
`Preview only — 1 master layer at revision 8. The document has not changed.`
(the document's undo label stayed `Relink components`), applying wrote the
single entry `Translate filter`, and one undo restored the glyph with no error
shown. Reporter overlays are visible on the glyph canvas in
`04-reporter-overlays.png`.

## Commands run

Rust (each under the shared `flock`, with a task-local `TMPDIR`):

```
cargo test -p fc-doc                      330 passed; 0 failed   (after the fixes)
cargo test -p fc-api                      308 passed; 2 failed   (see below)
cargo test -p fc-devserver --test http      5 passed; 0 failed
```

Re-run after the review fixes, as the final state of the work:

```
cargo test -p fc-api --bin gen_ts               5 passed; 0 failed
cargo test -p fc-api --test automation         28 passed; 0 failed
cargo test -p fc-api --test cross_paste         9 passed; 0 failed
cargo test -p fc-api --test font_compare       15 passed; 0 failed
cargo test -p fc-api --test font_merge          3 passed; 0 failed
cargo test -p fc-doc --test script_transaction 11 passed; 0 failed
cargo test -p fc-doc --test feature_drafts     19 passed; 0 failed
cargo test -p fc-doc --test replace_glyph       7 passed; 0 failed
```

The two `fc-api` failures are `dispatch.rs::damaged_preferences_fall_back_and_do_not_block_opening`
and `dispatch.rs::dispatches_session_open_and_reports_boundary_errors`. Both
compare a whole `Prefs` value that has gained a new `snap.distances` field; the
file, the type and the change belong to another task in this wave and neither
test touches this area's code.

Clippy:

```
cargo clippy -p fc-doc --all-targets -- -D warnings     clean (rerun after the fixes)
cargo clippy -p fc-api --all-targets -- -D warnings     3 errors, all in
                                                        crates/fc-api/src/handlers/variation.rs
```

Those three are `clippy::manual_flatten` findings in another task's file. They
abort the crate before the lint reaches this area's test targets; no finding was
reported in any file this task owns or patched.

Frontend:

```
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/automation src/plugins src/views/crossdocument
  Test Files  14 passed (14)
       Tests  153 passed (153)

pnpm --dir app check          no diagnostic in src/automation, src/plugins or src/views/crossdocument
pnpm --dir app exec eslint src/automation src/plugins src/views/crossdocument   clean
pnpm --dir app exec prettier --write <owned files>                              clean
pnpm --dir app gen:api                                                          regenerated
```

`pnpm --dir app check` still reports diagnostics in `src/dialogs/batchRename.test.ts`,
`src/views/features/**` and `src/views/glyph/panels/GlyphVariationPanel.tsx`.
They are other tasks' files and predate this work.

## Review fixes

An independent review returned *accept with fixes*; all nine are applied, each
with a test.

| # | Fix                                                                                                                                    |
| - | --------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | `commit_prepared_features` now calls `check_writable()`; every mutating `Document` entry point audited (table above).                    |
| 2 | Script transactions carry a deadline and are superseded by the next run; history and save reap an expired one; the console's close is wrapped. |
| 3 | The paste matrix now asserts the transferred contours, advance and component base, and that a metadata-only payload leaves the drawing untouched. |
| 4 | The plugin manager no longer renders the index file's raw English notice; the translated `plugins.localOnly` string carries the statement, and `curatedNotice` stays as provenance metadata for reviewers and tests. |
| 5 | The missing-parameter warning class is gone: `#[serde(default)]` fields made it report valid calls. Unknown keys are still reported.     |
| 6 | A throwing reporter is caught per plugin, logged once per session, and no longer breaks the canvas paint.                                |
| 7 | The metadata table uses a fixed layout, wraps long values and scrolls, so a reference value is never clipped mid-word.                   |
| 8 | `automation.folderKind.configured` / `.preferences` now read *Configured folder* / *Preferences folder*, matching their keys; user doc updated. |
| 9 | `normalizeSetting` re-clamps after rounding to the step, so a stepped value can no longer leave the declared range.                      |

## Remaining limits

1. **The console's check is not a TypeScript compiler.** It validates literal
   `fc.call` method names and rejects parameter keys the method does not accept.
   It does not report a *missing* key, because the generated TypeScript does not
   record which Rust fields have a serde default. Type-level checking of
   argument *values* needs the shipped `fontcarver.d.ts` in an editor; bundling
   `typescript` into the application was rejected as disproportionate.
2. **Reload on change is a bounded poll, not a filesystem watch.** The console
   re-reads its folders every two seconds while it is open. This avoids a new
   dependency and a background watcher; a change made while the console is
   closed is picked up when it opens.
3. **A merge does not adopt OpenType layout.** The generic command path
   deliberately rejects feature-draft commands, which need the features editor's
   revision and provenance API (task e). Bitmap strikes, colour palettes and
   global hint programs are likewise out of scope and stated in the merge
   warnings and the user guide.
4. **A reference document is read-only in the kernel, not badged in the tab
   bar.** `DocSummary` belongs to another task; the read-only state is returned
   by `font.open_reference` and shown in the compare dialog, and any edit
   attempt is refused by the kernel with a clear message.
5. **Plugins remain bundled code.** The index ships with the application;
   "install" records a local preference and "reload" re-reads that record. There
   is no download, no remote index and no third-party loader, by design.
6. **Reporter overlays are suppressed while a glyph tab shows a text run**
   rather than an editable glyph. That is the existing canvas hook's behaviour;
   it is now stated in the user guide.
7. **Family comparison is bounded** to 64 (document, master) members, kerning
   comparison to 200,000 records per master, metadata to 128 differing fields,
   a merge to 4,096 glyph decisions and 50,000 kerning record changes. The
   pre-existing geometry budgets are unchanged.
8. **No on/off pixel diff of the reporter overlays.** Seven tasks were editing
   `app/src` throughout this session, and each save reloaded the page; the
   controlled comparison could not be completed before a reload. The overlays
   are visible in `04-reporter-overlays.png` and are covered by 22 unit tests,
   including the failure-isolation case added by the review.
9. **The transaction reaper is not exercised in the browser.** A reloaded page
   dropping a live transaction, and the next run superseding it, are covered by
   `an_abandoned_script_transaction_expires_and_a_new_run_supersedes_it` and
   `listing_scripts_reaps_only_expired_transactions` rather than by a screenshot,
   because the failure is a history state rather than a visible one.

## Files, dependencies and shared edits

New dependencies: **none**, in Rust or npm. No FontForge source was read or
adapted.

Owned files changed: `app/src/automation/**`, `app/src/plugins/**`,
`app/src/views/crossdocument/**`,
`crates/fc-api/src/handlers/{automation/**,cross_paste.rs,cross_paste/**,font_compare.rs}`,
`crates/fc-api/src/types/{automation.rs,cross_paste.rs,font_compare.rs}`,
`crates/fc-api/src/bin/gen_ts.rs`,
`crates/fc-api/tests/{automation,cross_paste,font_compare,font_merge}.rs`,
`docs/user/automation.md`, `docs/user/cross-document.md` and this report.
New files: `app/src/automation/scriptCheck.ts` (+ test),
`app/src/automation/ScriptDeclarations.tsx`, `app/src/plugins/index.json`,
`app/src/plugins/reporters.ts`, `app/src/plugins/PluginSettings.tsx`,
`app/src/plugins/registry.test.ts`,
`app/src/views/crossdocument/{ComparisonSections,FamilyResults,MergeControls}.tsx`,
`app/src/views/crossdocument/useFontMerge.ts`,
`crates/fc-api/src/handlers/cross_paste/merge.rs`,
`crates/fc-api/tests/font_merge.rs`,
`crates/fc-doc/tests/script_transaction.rs`.

Narrow patches to shared files:

| File                                | Change                                                                                                      |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `crates/fc-doc/src/document.rs`      | `read_only` flag with guards on apply/undo/redo/save; script transaction with a deadline (`begin_transaction`, `end_transaction`, `finalize_transaction`, `finalize_expired_transaction`). |
| `crates/fc-doc/src/document/features.rs` | `check_writable()` at the head of `commit_prepared_features`.                                          |
| `crates/fc-doc/src/fcf/schema.md`    | Documents the placeholder tag and that read-only/transaction state is not serialized.                        |
| `crates/fc-api/src/api.rs`           | Registers seven new methods and their types.                                                                 |
| `app/src/i18n/en.json`               | Appends keys under `automation`, `plugins` and `crossDocument` (see below).                                 |

No other task's files were edited. `docs/p5-feature-audit.md`, `docs/STATUS.md`,
`docs/TODO.md` and `docs/progress-estimate.md` were not touched.

New i18n keys, all English, under three existing namespaces: `automation`
(`api`, `folders`, `folderKind.*`, `folderMissing`, `configuredFolder`,
`saveFolder`, `reloadedOnChange`, `transactionHelp`, `historyLabel`,
`historyEntry`, `check.*`, `declarations*`, `copyDeclarations`, `copied`,
`methodSearch`, `method`, `parameters`, `insert`, `insertNamed`, `noParameters`,
`noMethods`), `plugins` (`reloadIndex`, `provenance`, `settingsTitle`,
`settings.*`, `metricsName`, `metricsDescription`) and `crossDocument`
(`familyCompatible`, `familyMismatched`, `member`, `referenceMember`,
`allMasters`, `compatibility.{metadata,kerning,layout}`, `resolution`,
`resolveGlyph`, `resolutions.*`, `components`, `mergeTitle`, `mergeHelp`,
`adopt.*`, `mergeReplaced`, `mergeAdopted`, `metadata*`, `field`, `targetValue`,
`sourceValue`, `kerning*`, `groups*`, `pair`, `vertical`, `layout*`, `classes*`,
`referenceHelp`, `referenceOpened`, `readOnlySource`, `placeholders`,
`placeholderFilter`). `zh-CN.json` was not touched; `src/i18n/i18n.test.ts` is
expected to fail until the translation pass runs.

## Recommended status

| Row | Recommendation | Basis                                                                                                                              |
| --- | -------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 77  | **Complete**   | Twelve-cell matrix test with undo per cell, atomic failure, `.fcf` round trip, and a browser paste between two documents.           |
| 78  | **Complete**   | Relinking, cycles and placeholders verified in the browser; placeholders reported and findable in the inventory by a tested filter. |
| 79  | **Complete**   | One undo entry per run (kernel-enforced), shipped declarations plus in-console checking and completion, three script roots, reload on change. Limits 1–2. |
| 96  | **Complete**   | Metadata/kerning/layout/component differences, family across masters, kernel-enforced read-only references, per-conflict merge in one transaction, export reopened and checked with fontTools, FreeType and HarfBuzz. Limits 3–4, 7. |
| 105 | **Complete**   | Python RPC suite passing, curated JSON index with provenance, install/enable/disable/remove/reload, persisted settings, three sample plugins exercised in the browser. Limit 5.  |
| 106 | **Complete**   | Reporters receive document and view context and their settings; filter preview/commit/cancel/stale behaviour unchanged and reverified. Limits 6, 8. |
