# P5 project proof library persistence

Scope: project-owned proof text records persisted in existing `FontInfo.user_data`
under `com.fontcarver.proofLibrary`, with no Rust model or API additions.

Owned files: `app/src/views/text/proofLibrary.ts`, `useProofLibrary.ts`, their
focused tests, and delegated `crates/fc-doc/tests/proof_library.rs`. Parent owns
TextView/library UI, translations and audit/status reconciliation.

Public interface: `ProofEntry` carries id, name, group, script, language,
direction (`ltr`/`rtl`/`ttb`) and exact text. Helpers read/write a versioned library,
validate bounded input, filter/group entries and cycle the current selection.
`useProofLibrary(doc)` returns entries/loading/saving/error plus asynchronous
save(entry) and remove(id) actions. Writes use a fresh font-info read inside the
existing settled-document barrier before applying `SetFontInfo`.

Acceptance:
- Tests first for Unicode preservation, namespaced metadata preservation,
  invalid/future data and bounded entry/count rejection, filtering and cycling.
- Hook tests cover fresh metadata, serialized concurrent saves, rejection,
  generation/undo refresh, close/switch races and no optimistic data loss.
- Native tests prove exact multi-script project round trips and existing command
  undo/redo behavior without adding persistence APIs.
- Focused Vitest, local ESLint/format and shared-lock focused Cargo validation;
  parent runs final broad check/lint/test and workspace checks after integration.
- Artifact checker at task boundaries, task-owned scratch cleanup and concise
  acceptance report with actual command output.
