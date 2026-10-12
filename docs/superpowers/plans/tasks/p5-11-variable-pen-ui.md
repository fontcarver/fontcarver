# P5-11 variable pen editor

## Boundary

Extend the existing layer-effects dialog with a variable stroke operation. Edit
normalized contour positions and the width, height and angle of 2–64 pen points;
choose linear or smooth progression and the existing nib, cap and join options.
Keep the existing reviewed-revision preview, apply and bake workflow. Source
contours remain editable after applying; document undo and native persistence
use the kernel effect commands. The kernel/model owner supplies `variable_stroke`
and generated API types. No frontend-only persistence or alternate renderer.

## Owned files

- `app/src/views/glyph/panels/LayerEffectFields.tsx`
- `app/src/views/glyph/panels/LayerEffectsEditor.tsx`
- New pen-point editor component and focused component tests in that directory.
- `app/src/views/glyph/panels/layerEffects.module.css`
- English `layerEffects` strings (independent agent owns Chinese translation).

## Acceptance

1. Tests first: editing a pen point or progression produces a draft, invalidates
   review and reaches `effects.apply` only through preview/apply.
2. Add/remove interior points while preserving normalized endpoint positions;
   initialize the editor from stored variable strokes.
3. Bake submits reviewed parameters and generation; invalid/stale previews stay
   unapplied. Verify actual undo and `.fcf` persistence with the kernel.
4. Focused Vitest tests plus TypeScript/ESLint checks; parent runs broad checks.
5. Isolated agent-browser visual review with retained screenshots and a report.
6. Check artifact budgets at task boundaries and remove owned temporary files.
