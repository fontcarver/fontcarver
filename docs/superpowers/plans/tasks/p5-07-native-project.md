# P5-07 — Native directory projects and recovery evidence

## Scope

Add an explicit `.fcfp` native directory alongside compatible `.fcf` / `.fcf.gz`.
The directory contains a readable `project.json` with font-wide state, ordered
glyph index, and pretty JSON files per glyph in `glyphs/`. Glyph filenames derive
from name and serialized content so unchanged glyphs retain their files. Commit
new glyphs before atomically replacing the manifest; failed/interrupted saves
leave the previous manifest readable. Only obsolete files named by the previous
manifest may be removed. Preserve arbitrary user files and Git data.

Integrate native path detection with Session/API, CLI owners and document picker
owner. Preserve existing document save barriers, dirty-state and autosave API.
Independent delegated work audits labeled gesture history and recovery filtering.
No model changes, command registry rewrites, dependencies or status-file edits.

## Acceptance

- Semantic reopen through both fcf and Session, including order and Unicode.
- One glyph edit changes its own file and manifest, keeping other glyph bytes.
- Unsupported versions, missing/corrupt glyphs, duplicate names, mismatched names,
  path traversal and symlinked glyph paths fail without silently losing data.
- Interrupted writes/unreferenced files do not affect the committed snapshot;
  failed validation leaves previous project intact; user files remain untouched.
- Explicit native API save/open and existing `.fcf` remain compatible.
- Existing recovery/gesture suites plus focused new tests; parent runs workspace
  Clippy/regressions after integration. Every Cargo invocation uses shared flock.
