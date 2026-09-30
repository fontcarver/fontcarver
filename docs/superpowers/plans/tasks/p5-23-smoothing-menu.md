# P5-23 — G2 smoothing menu

Scope: register Super-smooth (G2) and Harmonize in the Path menu, using the
current glyph layer and selected on-curve nodes. Enabling or harmonizing
requires cubic-to-cubic joins. Disabling already marked nodes remains possible
after topology changes. Mixed selection enables all targets. The toggle shows
Enabled, Disabled, or Mixed state and every apply uses the existing undo/edit
settling path.

Owned files: `app/src/views/glyph/smoothCommands.ts`, its test file, and a narrow
import/registration addition in `app/src/shell/builtinCommands.ts`. The parent
owns English keys and generated API types; node-semantics owns kernel commands
`SetSuperSmooth { glyph, layer, points, enabled }` and
`HarmonizeNodes { glyph, layer, points }`.

Coordinated additional scope: a narrow outer-ring marker in `drawOutline.ts`
and separate `smoothNodeMarker.test.ts`, preserving configurable node sizing.

Acceptance: regression tests first for selection gating, sorted exact targets,
closed seam and open-end eligibility, persistent enable/disable and mixed
state, one-shot harmonize, and transport failure reporting/reconciliation.
Run focused Vitest and lint; parent owns final global frontend and browser
acceptance after API generation. No FontForge code is read or adapted.

## Verification

2026-09-30: focused Vitest passed 34 tests across `smoothCommands.test.ts`,
`smoothNodeMarker.test.ts`, and `drawOutline.test.ts` with one worker. ESLint
passed for the two new modules/tests and coordinated drawing file. Initial
command import test and the three missing-ring assertions were observed failing
before implementation. Parent retains global frontend/API/browser acceptance.
Final artifact checkpoint: build 6.13 GiB / 13,195 files, temporary 0.14 GiB /
4,849 files, 668.73 GiB free; PASS. No task scratch or running processes remain.
