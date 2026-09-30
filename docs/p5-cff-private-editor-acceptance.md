# CID CFF private dictionary editor acceptance

Date: 2026-09-30. Scope: P5-83 editor/API; model and CFF codec acceptance is
reported separately by the CFF preservation owner.

Implemented independent FD0/FD1+ selection and private-value editing, sequential
append up to 256 dictionaries, and glyph-layer FD assignment. Changes use the
existing revision-checked atomic hint command path and snapshot impact routing.
Extra dictionaries require CID metadata; assignments reject missing dictionaries
and layers belonging to a different master. The editor retains drafts on errors.
Automatic analysis remains restricted to FD0; no per-FD inference was added.

Verification:

- `pnpm -C app test src/views/hints/PsHintEditor.test.tsx`: **5 passed**. The two
  new cases cover FD1 editing/assignment without overwriting FD0 and append plus
  failure draft retention. Tests were observed failing before implementation.
- Focused `pnpm -C app exec eslint` on the four changed hint components/tests:
  **exit 0**. Narrow Rust formatting and `git diff --check`: **exit 0**.
- `cargo test -p fc-api --test cff_private_editor`: **blocked at compilation**
  by concurrent unrelated AFM `str_as_str`, layout `Validate` import, project
  `str_as_str`, optical preparation/profile and missing NodeLinks impact routing
  errors. The two acceptance tests cover atomic rollback, wrong revision,
  undo/redo, FCF save/load, wrong master and invalid dictionaries. They require
  root integration execution after those errors are repaired.
- Isolated headless agent-browser verification rendered the actual editor with
  an explicit mock transport. FD selection, glyph assignment and numeric editing
  worked; FD1 disabled analysis. This verifies UI appearance, not a running Rust
  transport. [Screenshot](screenshots/p5-cff-private-editor.png).

API types must be regenerated through the root-owned generator; generated files
were not edited manually. Full typecheck, lint and workspace regression checks
are root integration responsibilities. Independent translation owns the six new
English `hints` keys.

Browser and Vite processes were stopped. The task's 15 MiB browser profile,
harness and transient logs were removed after retaining this report and the final
screenshot. Artifact check before cleanup: build 6.09 GiB / 13,193 files,
temporary 0.13 GiB / 5,587 files; PASS. Final checkpoint is recorded below.

Final artifact checkpoint: **PASS** — build 6.09 GiB / 13,176 files, temporary
0.22 GiB / 8,085 files, workspace free 668.67 GiB. Other parallel tasks account for
remaining temporary storage; this task's scratch directory is removed.
