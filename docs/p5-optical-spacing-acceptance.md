# P5 optical spacing and automatic kerning acceptance

Scope: [P5-17 task](superpowers/plans/tasks/p5-17-optical-metrics.md), audit row94.
Implementation is original; no FontForge source was read or adapted and no new
dependencies were added. [User workflow](user/optical-spacing.md).

Implemented: resolved curve silhouette sampling (256 rows), UPM-scaled global
density, explicit selection and current/all-master scope, numeric/outline review,
preview invalidation, revision-checked one-step apply/undo, component dependency
ordering, concrete auto-kerning exceptions, protection for existing class rules
and explicit zero, preservation/skips for locks, whitespace, marks and metric
links. Work limits:128 glyphs,512 master layers,16384 pairs,4096 resolved segments
per source,16million aggregate segment samples. Kerning applies through one
validated table merge per master rather than repeated feature-source parsing.

The algorithm uses a sampled optical clearance approximation; it is not continuous
collision detection or a script-aware typographic optimizer. Automatic kerning
requires editable model-owned layout and records concrete exceptions, leaving
class membership intact.

## Validation status at integration handoff

- Added 3 profile unit tests for curved versus rectangular spacing, density,
  slanted pair geometry/clearance, nonfinite input and work budget rejection.
- Added 6 API tests for preview purity, components in both selection orders,
  one/all masters, stale/invalid/late-failure atomicity, metrics links, class/zero
  protection, undo/redo/FCF and Noto Sans + Source Sans CFF fixtures. Real-font
  cases have an explicit 15second batch budget.
- Own queued Cargo run was deliberately interrupted before execution to allow
  the root integration owner to run consolidated tests. Another agent's build
  exposed two optical type errors; both were fixed. Rust semantic tests remain
  pending the root integration run and are not claimed passed here.
- Optical UI owner initially passed11 tests; the final composite-preview fix
  passes12/12 focused Vitest tests (`--maxWorkers=1`), scoped ESLint and Prettier.
  Detailed UI evidence is in [UI acceptance](p5-optical-ui-acceptance.md); browser inspection is pending
  the shared integration environment per the root's host-load instruction.
- No optical background process or extra Cargo target directory remains.

Audit row94 is ready for integration validation; do not mark it fully accepted
from the implementation or UI-unit-test evidence alone. The root coordinator
records actual final workspace and browser results after integration.

Final focused frontend output (2026-09-30):

```text
pnpm --dir app exec vitest run src/views/font/OpticalSpacingDialog.test.tsx --maxWorkers=1
Test Files  1 passed (1)
Tests       12 passed (12)
Duration    14.12s
pnpm --dir app exec eslint src/views/font/OpticalMetricsPreview.tsx src/views/font/OpticalSpacingDialog.test.tsx
exit 0
```

The final preview returns the actual post-edit resolved outline and bounds. This
covers composites whose selected bases move by different amounts; the view does
not approximate such changes by translating the original compound silhouette.
