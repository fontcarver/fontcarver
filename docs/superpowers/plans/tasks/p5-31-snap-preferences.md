# P5-31 snapping families and distances

Scope: independent node, handle, anchor, guide, metric, component, alignment-zone,
background-point and grid snapping preferences; persisted CSS-pixel snap/hit
distances; preferences controls; active master/background source wiring. Extend
guide snapping to project onto arbitrary-angle guides for construction magnets.
Parent-approved integration also covers grid subdivisions (row 53) and node/handle
display diameters (row 81), persisted and wired to canvas rendering.

Own snapping/editorSnapping helpers, narrow editorActions/editorTypes/controller
and geometry changes, preference Rust/API/store/dialog fields, translated-label
inputs and additive English keys. Coordinate GlyphView source fields with image
owner and API generation with root. No source is adapted from FontForge.

Acceptance: failing-first focused Vitest snap-family, zoom/distance, oblique-guide,
hit-test and persisted preference UI tests; Rust preference compatibility,
validation and persistence tests. Parent coordinates broad frontend/Rust checks,
independent translation and browser review. Use shared Cargo lock and task-local
TMPDIR; artifact checker at start/end and before compilation.

## Acceptance (2026-09-30)

Implemented independent families, oblique guide projection, zone/background and
advance-boundary snapping; persisted hit/snap distances, subdivision count and
marker diameters; active master/background wiring and preferences UI. No
FontForge source was used.

Initial snapping tests: 5 new failures, 14 passed before implementation.
Final focused Vitest results with `--maxWorkers=1 --minWorkers=1`:

```text
src/editor/snapping.test.ts                  19 passed
src/editor/hitTest.test.ts                   12 passed
src/stores/prefsStore.test.ts                18 passed
src/views/glyph/drawOutline.test.ts          17 passed
src/dialogs/PreferencesDialog.test.tsx        9 passed
```

The dialog persistence test spans nine control changes and uses a 30-second
timeout; it passed in 17.4 seconds under heavy shared-machine load. An earlier
five-second timeout was a failed run, not counted as passing. Targeted ESLint,
owned-file Prettier and Rust `rustfmt --check` passed. Seven prefs handler tests
(five new) await the parent's centralized Rust verification:
`cargo test -p fc-api --lib handlers::prefs::tests -- --nocapture`.
The child Cargo invocation was interrupted before it acquired the shared lock;
it produced no test result. Parent owns generated API refresh, integrated
typechecking/lint, independent translation and browser verification. No own
server or browser process was started; own empty scratch directory was removed.
