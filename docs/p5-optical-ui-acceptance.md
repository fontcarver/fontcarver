# P5 optical spacing and kerning UI acceptance

## Implemented

The glyph-grid toolbar opens the optical dialog for 1–128 selected glyphs. The
dialog captures document, master and glyph handles at opening. It settles pending
editor drafts before previewing or applying `optical.batch`.

Controls select optical spacing or automatic kerning, density from 0.25 to 4,
one/all masters and optional replacement of existing kerning. Every option edit
invalidates the reviewed preview. Apply requires a preview with actual changes
and supplies its document generation; a rejected apply clears that preview.

The review shows before/after LSB, RSB and width, or effective kerning. Spacing
examples also compare translated outlines at the same scale. Examples are capped
at sixteen, with total change counts and skipped-reason totals. All labels use
`opticalSpacing` translations; Chinese translation is owned by the independent
localization pass.

## Verified

The behavior tests were written first. Their initial run failed because the
dialog module did not yet exist. After implementation:

```text
pnpm exec vitest run src/views/font/OpticalSpacingDialog.test.tsx
Test Files  1 passed
Tests       11 passed
```

Coverage includes preview gating, actual review values, skip totals, option
invalidation, fixed request parameters after rerender, stale rejection, a
zero-change preview, draft-flush rejection and invalid density values.

Focused ESLint passed for the dialog, review component and tests. Focused
Prettier checks passed. The task's resource checks passed throughout.

## Verification boundaries

Generated API bindings, workspace TypeScript checks, full frontend tests and
real-backend integration are owned by the root coordinator. Generated files were
not edited manually for this UI.

An isolated Vite harness was prepared for a browser review using a mock
transport, but Chromium exited before creating `DevToolsActivePort` on both
launch attempts, including `--no-sandbox`. The offline browser diagnostic passed.
The coordinator then stopped new browser launches because host load exceeded
100. No screenshot or browser acceptance is claimed. The task browser session
and Vite process were stopped, and its harness/profile scratch was removed.
