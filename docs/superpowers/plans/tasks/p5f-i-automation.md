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
