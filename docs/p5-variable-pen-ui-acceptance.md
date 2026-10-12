# P5-11 variable pen editor acceptance

## Delivered editor behavior

The existing Layers → Layer effects dialog offers **Add variable stroke**.
Each profile stores 2–64 pen points with normalized source-contour position,
width, height and angle. The first and last positions stay at 0 and 1; interior
points can be inserted, edited and removed. Insertion splits the largest gap and
interpolates dimensions and the shortest angular interval. Linear and Smooth
control progression between points; the existing nib, cap, join and miter
controls also apply to variable strokes.

Edits remain local until Preview changes. Any edit, insertion, deletion or
progression change invalidates the reviewed result. Apply keeps the editable
source and profile; Bake result uses the reviewed document generation and the
existing one-step kernel transaction. Failed previews leave the draft editable.
Existing profiles initialize directly from the native layer effect stack.

## Focused frontend checks

On 2026-09-30, the existing layer-effects suite and the new variable-pen suite
passed together:

```text
pnpm --dir app exec vitest run src/views/glyph/panels/LayerEffectsEditor.test.tsx src/views/glyph/panels/VariablePenEditor.test.tsx --maxWorkers=1
Test Files  2 passed (2)
Tests       8 passed (8)
Duration    5.01s
```

The six new tests cover local parameter editing and preview/application,
anchored endpoints and interior insertion/removal, stored-profile initialization
and baking, effect removal, progression invalidation, and rejection/correction
of unordered positions. The original two tests retain mask, reorder and stale
revision coverage. ESLint and Prettier checks pass for the affected files.
Implementation landed concurrently with the tests, so no initial failing run
is claimed.

## Real kernel and browser acceptance

An isolated agent-browser session used the freshly built `fc-devserver` through
Vite on 2026-09-30. An original synthetic font supplied an open four-point
centerline, a 700-unit advance and a second glyph referencing the source.

- Authored a three-point profile at positions 0, 0.45 and 1, changing width,
  height, angle, progression and join style through the dialog.
- An unordered position was rejected by the real kernel. Correcting it enabled
  preview. Editing a reviewed width disabled Apply until another preview.
- Applied and saved through the UI. Direct native-file and API inspection
  confirmed the exact profile, all four source points, anchors and advance.
- Reopened the dialog from stored parameters. Baking produced one closed
  contour with 883 points and cleared the effect. One UI Undo restored the exact
  original layer and profile; UI Save preserved that restoration.
- Closed the document and reopened the saved FCF into a fresh document handle.
  The complete API layer matched the pre-bake layer. The dependent component's
  reference, transform and existing metadata also remained unchanged.

The visual review found that the source preview filled open centerlines as if
they were closed regions. The source-only preview now draws an unfilled outline;
the result and ordinary batch-style previews retain their existing rendering.

- [English/light screenshot](screenshots/p5-variable-pen-en-light.png)
- [English/dark screenshot](screenshots/p5-variable-pen-en-dark.png)
- [Native checks and accessibility results](acceptance/p5-variable-pen-ui.json)

Both complete dialogs were inspected at 1440×1800. Axe 4.12.1 reported zero
violations and zero incomplete checks in each theme. The screenshots show the
source/result comparison and the explicit approximation warning.

## Boundaries and cleanup

This report establishes the editor workflow and native persistence on the
synthetic source above. It does not independently establish geometric error
bounds, all cap/join combinations, compiled-font exports, or native desktop
platform behavior. Those remain the kernel/integration owner's checks.
No FontForge source was used and no dependency was added.

Only the acceptance report, compact JSON evidence and two review screenshots
are retained. Task-owned browser, API and Vite processes and disposable test
fonts/profiles were removed at the completion boundary (104 MiB reclaimed). Final broad frontend
and workspace checks belong to the integration owner.
