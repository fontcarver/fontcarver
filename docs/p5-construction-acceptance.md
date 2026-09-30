# P5 linked construction acceptance — 2026-09-30

## Delivered behavior

- Reusable corner/cap/segment/brush glyph attachments retain their source glyph,
  source-node identities and numeric parameters. Shared evaluation includes
  dependency recursion, live source changes, effects, preview and export baking.
  The inspector editor previews a draft, guards writes by the reviewed document
  generation, keeps attachments live or bakes them, edits values and removes them.
- Node identities survive source-node reordering. X/Y servant coordinates and
  adjacent handles interpolate between independent boundary nodes. The inspector
  creates/removes either axis relationship on the selected interior node.
- Ordinary layer guides own signed node offsets. Numeric/pointer guide edits
  update attached nodes; two nonparallel guide owners can constrain a point.
  The inspector's magnetic attachment preserves offsets or projects before
  linking. Automatic straight H/V stem detection avoids counters/whitespace,
  previews a count, then creates centerline guides binding both edge node sets.
- .fcf and UFO private metadata preserve construction/constraint source intent.
  Validation prevents dangling/cyclic bindings; source glyph rename/dependencies,
  removal rollback, undo and background swaps include the new metadata.

## Actual local verification

```
TMPDIR=<workspace>/.scratch/p5-construction pnpm --dir app exec vitest run --maxWorkers=1 \
  src/views/glyph/panels/constructionGeometry.test.ts \
  src/views/glyph/panels/constructionCommands.test.ts \
  src/views/glyph/panels/ReusableConstructionEditor.test.tsx
Test Files 3 passed (3)
Tests 13 passed (13)
```

A later command-helper regression verifies that separated stems sharing the same
centerline retain all eight node links. Its focused rerun passed all four helper
tests (14 frontend tests total across the latest files). Scoped ESLint and
`git diff --check` passed. Initial missing-module/fixture-selector failures were
fixed before these passing results.

Rust task owners added focused suites for construction references, geometry,
preview/generation/undo, source serialization, guide ownership, handles and
cycles. Their queued Cargo commands were cancelled before compilation at the
integration coordinator's request; they are **not local passing evidence**.
Final Rust and generated-TypeScript acceptance is owned by the root integration
batch. Targets: `fc-model` construction_references/outline_filter and node_links
unit filter; `fc-geom` constructions/effects; `fc-doc`
constructions/construction_persistence/node_links; `fc-api` constructions/effects;
`fc-formats` construction_sources. See sibling task reports for individual files.

Headless visual acceptance was paused by the coordinator while the shared
backend/types were unavailable and host load was high. The temporary Vite server
was stopped before opening a browser, and its fixture harness was removed. No
native/global desktop input was used. This report does not claim browser or
full-app end-to-end acceptance.

## Explicit limits

Attachments use affine placement: corner/cap rotate to tangents; segments map a
straight source baseline to a node chord; brush placements follow arc length.
There is no curve deformation or automatic corner trimming. Work is bounded to
128 constructions, 1,024 brush stamps and 65,535 evaluated path elements.

Automatic stem candidates use straight axis-aligned edges; curved segments are
sampled only for fill checks. Very complex outlines return no candidates. The
magnet action is an inspector selection workflow; no dedicated drag magnet tool
or visual link-arrow overlay is claimed. Only ordinary layer guides own links.

A node supports at most two nonparallel guides; competing parallel owners reject.
A servant target cannot simultaneously be guide-owned, though guide-owned
boundary nodes can drive servants. Referenced nodes/guides must be unlinked
before deletion. Existing node metadata is resolved and validated atomically.

No new dependencies or FontForge source adaptations were introduced. New English
UI strings use `construction`; translation is independently owned. The user asked
to finish current work and stop, so no additional task scope was started.
