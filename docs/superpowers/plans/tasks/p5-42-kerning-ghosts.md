# P5-42 kerning member ghosts and sign feedback

## Scope

Add an opt-in, keyboard-accessible outline comparison for the selected kerning
pair's group members. Use the existing bounded membership preview, fetch at most
four alternate members per side, require exact glyph-name matches and the active
master, and stop queued work/discard results when selection or generation changes.
Ghosts compare member outlines at the selected pair origins; they do not claim to
shape every alternative. Color the selected gap from its resolved stored value,
with a signed text legend so color is not the only feedback.

## Owned files and interfaces

New `app/src/views/text/kerningGhosts.ts`, `useKerningGhosts.ts`,
`KerningGhostControls.tsx`, CSS and focused tests. Narrow changes to
`textCanvas.ts`, `textCanvasBridge.ts`, `drawTextRun.ts`, `drawTextGaps.ts`, and the
English `kerningGhosts` namespace. TextView integration is owned by proofing agent;
provide its exact hook, component, and optional run fields. Do not modify the
kerning model/edit hook, generated API, status/audit files, or translations.

## Acceptance

Tests cover bounded exact-name/master loading, aborted queued work, suppression
of stale hook output, unavailable/memberless pairs, toggle semantics, origin/cull
behavior, and negative/positive/neutral drawing colors. Run focused Vitest and
ESLint/format checks only; parent owns broad integrated frontend verification and
browser acceptance. Check artifact budgets at start and completion.

## Acceptance report — 2026-09-30

Implemented opt-in outline ghosts (four alternatives per group side), exact-name
and active-master loading with two workers, abort checks, synchronous stale-result
suppression, viewport culling, local-origin drawing and vertical member origins.
The selected gap uses its resolved kerning sign for color; visible signed text
provides equivalent non-color feedback. Native checkbox keyboard interaction is
covered. Same-named groups remain distinct across side1 and side2.

Proofing integrated `useKerningGhosts` in `TextView` and
`KerningGhostControls` in `TextWorkspaceControls`. Shared rendering edits preserve
concurrent color-glyph support. English strings use `kerningGhosts`; independent
localization owns translations.

Verification (all commands used the task-specific absolute TMPDIR for tests):

- `pnpm --dir app exec vitest run src/views/text/kerningGhosts.test.ts src/views/text/useKerningGhosts.test.tsx src/views/text/ghostRendering.test.ts src/views/text/textCanvasBridge.test.ts src/views/text/drawTextRun.test.ts`
  completed: `Test Files 5 passed (5); Tests 20 passed (20)`.
- After the same-name side fix,
  `pnpm --dir app exec vitest run src/views/text/kerningGhosts.test.ts src/views/text/useKerningGhosts.test.tsx`
  completed: `Test Files 2 passed (2); Tests 10 passed (10)`.
- Focused ESLint over all ten touched TypeScript feature/render/test files exited
  0 with no output. Prettier completed for owned files and narrow render patches.
- Artifact checks passed: initial build 5.51 GiB, final build 6.10 GiB; task scratch
  empty and removed; no owned servers or background processes remain.

Not claimed: broad frontend check/lint/regression completion or browser visual
acceptance (root owns integrated checks and shared-browser acceptance). Ghosts
compare member outlines at the selected pair origins; alternatives are not
individually shaped. User freeze was honored after the existing fix and checks.
