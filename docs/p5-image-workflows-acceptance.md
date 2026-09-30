# P5 source images and raster tracing acceptance

Implementation prepared 2026-09-30 for audit rows 92 and 93. Final consolidated
Rust/frontend verification is owned by the parent resumed-work integration task.
Do not treat pending checks below as passes.

## Implemented behavior

- Per-layer source PNG import from browser upload or a local filesystem path.
  Embedded bytes and document-relative external links are distinguished in the
  inspector. Missing/corrupt sources produce a repairable status, leaving the
  stored placement intact. Refresh reloads external bytes.
- Six-coefficient affine placement, image locking, and undoable unlock/remove.
  The editor paints the source at 50% opacity in the same y-up coordinates used
  for tracing. Glyph locks reject image changes too. A corrupt embedded source
  can be unlocked and removed without needing to decode it first.
- Native Save As rebases relative links in both the active font and history.
  Failed writes retain font/path/history unchanged. UFO export maps embedded
  sources to safe, distinct filenames and retains image locking; source snapshots
  remain unchanged. Export workflow hydrates native links in a detached copy.
- Trace preview, threshold and conservative simplification controls, append or
  replace, atomic apply/undo/redo, and retained source PNG. Apply requires the
  reviewed document generation and SHA-256 of source bytes so an externally
  changed file cannot silently replace the reviewed trace.

## Deliberate boundaries

The accepted raster format is a static PNG. Limits are 16 MiB input, 4096 pixels
per dimension, one million total pixels, and the existing tracing edge budget.
APNG and other formats fail explicitly. Tracing produces dark-ink polygons;
photographic/color segmentation and Bezier curve fitting are not implemented.
Simplification conservatively falls back to exact edges when topology cannot be
retained. Placement is edited through an affine matrix, without on-canvas image
transform handles. Source images remain source data and do not enter binary fonts.

## Evidence recorded before consolidated verification

- `rustfmt --edition 2021 --check` on owned Rust API, raster, path and test files:
  passed; scoped `git diff --check`: passed.
- Focused image panel tests: 12 passed; painter/load hook tests: 11 passed.
  Focused image ESLint passed (frontend agent report).
- Added 12 raster tests, 6 API workflow tests, 4 native path/history tests and
  2 UFO source-image tests. Initial Cargo attempts encountered concurrent
  model/sfnt compilation errors. Queued retries were intentionally canceled at
  root instruction to prioritize one final generator/workspace run (exit 143).
  These Rust tests are **pending consolidated execution**.
- Interactive agent-browser verification is pending a freshly rebuilt shared
  backend. No native desktop input was used.
- Disk checker passed at task boundaries. Last pre-handoff check: build 6.17 GiB,
  temporary outputs 0.21 GiB, workspace free 668.65 GiB. Owned empty scratch
  directory and queued Cargo processes were removed/stopped before handoff.

Recommended audit status: implementation available; keep rows 92/93 pending final
integration acceptance until the consolidated checks and browser smoke complete.
