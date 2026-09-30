# P5 outline editor acceptance checkpoint

Date: 2026-09-30. This records the bounded resumed work; it does not declare the
entire outline-editor audit complete. The user subsequently asked to finish
already-running tasks and stop, so remaining scope is listed explicitly.

## Drawing and measurement tools

Added freehand lasso (`Q`), eraser (`Shift+E`) and annotation (`A`) registration.
Lasso uses a polygon, includes its edges, captures Shift-add and Alt-ignore-handle
modifiers at the beginning, and applies the editor's focus/lock selection policy.
Eraser sweeps nodes across each pointer segment, collects them locally and emits
one existing `DeletePoints` command at release. Shift uses `BreakContour`; normal
erasing uses `KeepShape`. Escape discards the uncommitted stroke. Annotation
storage/locking has its own acceptance below.

The measurement HUD now includes horizontal/vertical deltas in an upright
coordinate system using the source's OpenType italic angle. Physical length and
angle retain their original meaning. Shared intersection coordinates are merged
so crossing vertices do not create zero-length spans. `G` during a nonzero live
measurement emits one layer `AddGuide` command while keeping the measurement
active; repeated key events cannot add duplicate undo entries. Tooltips explain
these gestures. Escape removes the measurement. No FontForge source was read or
adapted for these changes, and no dependency was added.

Verification performed by this owner:

```text
measure.test.ts + selectionTools.test.ts + registry.test.ts:
  Test Files 3 passed (3), Tests 15 passed (15)
ToolPalette + useToolShortcuts + drawingTools + registry:
  Test Files 4 passed (4), Tests 37 passed (37)
InfoBox + NumberField: Test Files 2 passed (2), Tests 19 passed (19)
Targeted ESLint for new tools, registry, contracts, tool host and palette: exit 0
```

The new measurement tests first failed five cases (missing delta/key behavior and
intersection duplication); all subsequently passed. Selection tool tests first
failed because the implementations did not exist, then passed. Existing
`AddGuide`/`DeletePoints` kernel commands provide the undo/persistence path; this
checkpoint does not substitute new native-file/export coverage for those commands.

## Parallel implementation reports

- [Snapping preferences](superpowers/plans/tasks/p5-31-snap-preferences.md):
  independent families, background/zones, oblique guides, hit/snap distances,
  grid subdivision and point/handle sizes; 75 focused frontend tests passed.
- [Layer workflows](superpowers/plans/tasks/p5-32-layer-anchor-workflows-report.md):
  custom/duplicate/rename layers, background switching and selected-path copy;
  component locking and mark-cloud UI are coordinated there.
- [Outline audit](p5-outline-audit-acceptance.md): always-on active-layer markers,
  controls, details and fixes, limited to the three documented checks.
- [Mark-cloud preview](p5-34-mark-cloud-acceptance.md): matching conventional
  anchors, independently positioned glyph cards and selected-master behavior.
- [Path operations](p5-path-operations-acceptance.md) and
  [node semantics](p5-outline-node-semantics-acceptance.md) record
  the smoothness/modifier/path-operation verification and remaining Open Corner.
- [Editor metadata](p5-editor-metadata-acceptance.md): glyph/path locks, path
  focus, per-layer annotations and lock-aware tools; 19 frontend tests passed.

## Acceptance boundaries

- Row 20 remains partial: Text is still inert in the outline tool factory and
  Pixel belongs to the separate bitmap editor; lasso/eraser/annotation do not
  establish full combined tool parity.
- Row 21 has nine passing tap/hold tests and four palette tests.
- Row 27 remains partial because true Open Corner geometry was not implemented.
- Row 35 remains partial: open contours and coincident points require manual
  review; automatic fixes currently round fractional coordinates only.
- Row 90 implementation and focused tool tests pass. A completed browser
  measurement/guide/undo acceptance is still required before a stronger claim.

The isolated headless browser did not reach a page: Chrome exited before writing
its DevTools port. Its own failed profile was removed, and owned development
servers on ports 8891/1441 were stopped. No native desktop input was used.
Central API generation, whole-app checks, Rust suites, independent translation
and integrated browser acceptance are owned by the root integration pass.

Final owned cleanup removed 768 KiB of scratch data. Artifact checker: PASS;
build 6.21 GiB / 13,202 files; temporary 0.21 GiB / 6,508 files; free 668.68 GiB.
Owned-file `git diff --check` passed.

After central API generation, completed model fixtures replaced incomplete
inline point/contour/change objects in five outline tests, and selection
predicates/hoisted selection fixtures now retain their real model types. No
production behavior changed and no assertion was weakened. The five corrected
suites passed 18 tests; targeted ESLint and diff checks passed.
