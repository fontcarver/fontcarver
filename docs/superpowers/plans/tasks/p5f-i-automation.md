# p5f-i: Cross-document workflows, scripting and plugins

Area id: `i-automation`. Ports: API 8809, UI 1509. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                           | State at start                                                                                                              |
| --- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| 77  | Paste Special: same-name / selected / as-components × all-data / layer / kerning groups / metric keys | `PasteSpecialDialog`, `cross_paste/plan.rs`; see `docs/p5-cross-document-acceptance.md`; the full matrix to verify.         |
| 78  | Cross-document copy with component relinking and missing-base placeholders                            | `RelinkComponentsDialog`; verify relinking, placeholders, cycles and atomic failure.                                        |
| 79  | TypeScript scripting API with generated `.d.ts`; macro console; scripts folder + reload               | `app/src/automation/**`, `macro.worker.ts`; see `docs/acceptance/p5-automation.md`; generated `.d.ts` and reload to verify. |
| 96  | Compare Fonts / Compare Family diff view; compare against a reference binary                          | `CompareFontsDialog`, `font_compare.rs`; family comparison and conflict-resolved merge to verify/implement.                 |
| 105 | Python scripting via an out-of-process interpreter; plugin manager with a curated index               | `automation/python.rs`, `python_bridge.py`, `PluginManager`; lifecycle and real sample plugins to verify.                   |
| 106 | Reporter plugins drawing on the canvas; filter plugins with a preview contract                        | `app/src/plugins/**`, `drawPluginOverlays.ts`, `filterSession.ts`; contracts to verify with real sample plugins.            |

Existing records: `docs/p5-cross-document-acceptance.md`, `docs/acceptance/p5-automation.md`,
`docs/acceptance/p5-automation-build-blocked.log`, `docs/user/automation.md`,
`docs/superpowers/plans/tasks/p5-automation.md`, `p5-compare-fonts-backend*.md`,
`docs/superpowers/plans/tasks/p5-rust-api-integration.md`.

## Scope notes

- Row 77: test every cell of the 3 × 4 matrix against the real kernel;
  selection mismatches and stale revisions fail atomically with a message;
  each paste is one undo unit; `.fcf` save/reopen after paste.
- Row 78: cross-document copy relinks components by name and, when the base
  is missing, creates a placeholder glyph flagged in the inventory (coordinate
  with task b's badge API by exposing the status from your handler); cycles
  are rejected; browser evidence with two open documents.
- Row 79: a generated `fontcarver.d.ts` (from the `fc-api` types) shipped with
  the app and consumable by the console's type checking/completion; scripts
  folder discovered from the project or preferences, reload on demand and on
  change; scripts run in the worker with document access, one history
  transaction per run, cancellation and error output in the console.
- Row 96: Compare Fonts (two documents), Compare Family (all masters/sources
  of a family, compatible and mismatched sets), compare against a reference
  binary (open a TTF/OTF as read-only reference); differences in inventory,
  metadata, metrics, kerning, outlines (with visual overlay), components and
  layout; navigable results; a conflict-resolved merge that lets the user choose
  per conflict and applies in one transaction with export/reopen verified.
- Row 105: launch a separate `python3`, versioned protocol, typed
  requests/errors, job bounds/reaping, document preserved on script failure;
  plugin manager: curated index (bundled JSON with provenance), install,
  enable, disable, remove, reload, persisted settings; exercise at least two
  real sample plugins end to end (one reporter, one filter) in the browser.
- Row 106: reporter plugins attach/detach canvas drawing with document/view
  context; filter plugins preview without mutation, commit once, cancel
  cleanly, reject stale previews; a failing plugin never corrupts history.

## Owned files

`app/src/views/crossdocument/**`; `app/src/automation/**`; `app/src/plugins/**`;
`crates/fc-api/src/handlers/{automation/**,cross_paste/**,cross_paste.rs,font_compare.rs}`;
`crates/fc-api/src/types/{automation.rs,cross_paste.rs,font_compare.rs}`;
any Python bridge or sample plugin files under `crates/fc-api/src/handlers/automation/`,
`app/public/plugins/**` or a `plugins/` directory you introduce (document the layout);
the `.d.ts` generator (extend `crates/fc-api/src/bin/gen_ts.rs` or add a sibling binary);
`docs/user/automation.md`; `docs/p5f-i-automation-acceptance.md`; user docs for compare/paste.

Shared files (narrow patches only): `crates/fc-doc/src/document/**` (paste/merge
transactions), `crates/fc-doc/src/session.rs`, `crates/fc-model/src/glyph.rs`,
`crates/fc-doc/src/fcf/schema.md`, `app/src/shell/{builtinCommands,commands}.ts`,
`app/src/dialogs/DialogHost.tsx`, `app/src/stores/documentStore.ts`,
`app/src/views/glyph/GlyphCanvas.tsx` (plugin overlay hook only), `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`, `app/package.json` (scripts only).

## Gap analysis

Method: the 2026-09-30 batch left this area implemented but unaccepted. The first
action was therefore to run every backend suite the earlier reports recorded as
**not run**, against the current checkout:

| Command                                     | Result          |
| ------------------------------------------- | --------------- |
| `cargo test -p fc-api --test automation`    | 20 passed       |
| `cargo test -p fc-api --test cross_paste`   | 8 passed        |
| `cargo test -p fc-api --test font_compare`  | 10 passed       |
| `cargo test -p fc-doc --test replace_glyph` | 7 passed        |
| `cargo test -p fc-api --bin gen_ts`         | 3 passed        |
| `cargo test -p fc-devserver --test http`    | 5 passed        |

That closes the "queued/cancelled, not recorded as passing" hole in
`docs/acceptance/p5-automation.md`, `docs/acceptance/p5-automation-build-blocked.log`
and `docs/p5-cross-document-acceptance.md`. The remaining work per row:

### Row 77 — Paste Special matrix

Works: all three destinations × four payloads are reachable in
`PasteSpecialDialog`, validated by `plan.rs`, and covered cell-by-cell by
`destination_and_content_matrix_preserves_unselected_fields` (12 cells, each with
an undo assertion). Stale-revision and selection-count failures reject
atomically; `.fcf`/`.fcf.gz` save-reopen is covered by the replace_glyph suite.

Missing: no browser evidence with two real documents through the real kernel.

Plan: verify in the headless browser (two documents, one paste per destination,
undo, reopen), and keep the backend matrix test as the exhaustive cell evidence.

### Row 78 — Cross-document relinking and placeholders

Works: name relinking with transform preservation, cycle rejection, atomic
failure, recursive base copy, decompose, and placeholder creation
(`export = false`, tag `missing-component-base`, explanatory note).

Missing: (a) the placeholder is **not flagged in the inventory** — the tag is
written but nothing reads it, so the requirement's "flagged in the inventory"
is unmet; (b) no browser evidence.

Plan: expose the placeholder state from this area's own handler rather than
editing task b's `types/glyphs.rs`: report placeholders explicitly in
`PasteSpecialResult` (`placeholders`, with names and reasons) and make them
findable in the existing inventory through a documented smart-filter expression
on the tag. Record the exact `GlyphCell` field task b would need as the
remaining coordination item.

### Row 79 — TypeScript scripting, console, scripts folder

Works: worker execution, Stop, bounds, output/errors, `automation.scripts_*`,
a generated `fontcarver.d.ts`.

Missing:
1. The declaration is generated into `app/src/api/generated/` and never shown to
   the user; the console has no type checking or completion, so "consumable by
   the console's type checking/completion" is unmet.
2. The scripts folder is only the application config directory — neither a
   project folder nor a preference.
3. Reload is on demand only; there is no reload **on change**.
4. **One history transaction per run is absent.** `Document::apply` finalizes any
   active gesture, so every script call is its own undo entry; the shipped user
   doc states this as a boundary. This is a required behaviour, not a limit.

Plan: generate a machine-readable API manifest next to the `.d.ts`; add console
completion plus a pre-run check of `fc.call` literals (unknown method, missing
or unknown parameter keys) and a declaration panel that shows the shipped path
and text; add three script roots (configured / preferences / project) with
root-scoped reads; add a directory fingerprint and reload-on-change while the
console is open; add a real script transaction (`automation.transaction_*`,
backed by a narrow sticky-gesture patch in `fc-doc`) so one run is one undo
entry, used by both JavaScript and Python.

### Row 96 — Compare Fonts / Family / reference binary

Works: two-document comparison by persistent name, evaluated outlines with
overlay, metrics, Unicode/variation sequences, topology, paging, counts,
revision guards, bounded work budgets.

Missing:
1. No **metadata** diff beyond family name and UPEM.
2. No **kerning** diff (groups or pairs).
3. No **layout** diff (features/lookups).
4. No **component** diff distinct from the evaluated outline.
5. "Compare Family" only walks other open documents at their first master; the
   masters of the family itself are never compared, so a mismatched master set
   inside one document is invisible.
6. A reference binary is opened as an ordinary editable document — nothing makes
   it read-only.
7. The merge is `same_name`/`all_data` for checked rows. There is **no
   per-conflict choice** and no export/reopen evidence.

Plan: extend `font.compare` with metadata/kerning/layout sections and a
component flag; add `font.compare_family` covering every (document, master)
member; add `font.open_reference` with a kernel-enforced read-only document; add
`font.merge` in `cross_paste/merge.rs` with per-glyph keep/take choices and
explicit font-level adoption (info, master metrics, kerning) applied as one
transaction. Adopting a reference's OpenType layout stays out of the merge: the
generic command path deliberately rejects feature drafts, which need the
features editor's revision/provenance API (task e owns it). That is recorded as
a limit with a named owner.

### Row 105 — Python RPC and plugin manager

Works: separate interpreter, versioned bounded line JSON RPC, typed errors,
deadline/reaping, captured-document restriction, document preserved on failure —
all covered by the 20 automation tests that now pass.

Missing: the curated index is TypeScript code with no **provenance** data; there
is no **reload**; there are no **persisted per-plugin settings** (only
enabled/version). No browser evidence for the two sample plugins.

Plan: ship `app/public/plugins/index.json` (bundled, with author/license/source
provenance and a checksum-free local-only statement), validate it at load, add
`reload` to the lifecycle store, and persist per-plugin settings (the translate
filter's offsets) so a reopened session restores them.

### Row 106 — Reporter and filter contracts

Works: declarative reporter overlays, attach/detach on enable/disable, filter
preview without mutation, single commit, cancel, stale rejection, plugin failure
isolated from history.

Missing: reporters receive only `bounds` — no **document or view context**, so
the requirement's "document/view context" is unmet. No browser evidence.

Plan: extend `ReporterInput` with document context (doc, glyph name, master,
layer, advance width, upem, master metrics) and view context (scale, visible
font-space rectangle), pass them from the canvas hook, and add a second reporter
that demonstrably needs them.

### Decisions recorded without asking

- The reference binary becomes a genuine read-only document via a narrow
  `fc-doc` patch, rather than a duplicate scratch session. `DocSummary` is owned
  by another task, so the read-only state is returned by this area's own
  `font.open_reference` result and surfaced in the compare dialog; the shell tab
  bar is not badged.
- Reload-on-change uses a bounded directory fingerprint polled only while the
  console is open, instead of adding a filesystem-watch dependency.
- Script type checking means: completion from the generated manifest plus
  rejection of unknown methods and unknown/missing parameter keys in `fc.call`
  literals before execution. A full in-browser TypeScript compiler is not
  bundled; the generated `.d.ts` remains the editor-side contract.
