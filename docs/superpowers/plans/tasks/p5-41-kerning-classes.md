# P5 row 41: automatic class scope and held exceptions

Owned files: `useTextKerning.ts`, `kerningModel.ts`, `kerningEdits.ts`, a small
scope-state hook if needed, and focused tests in `app/src/views/text/`.
The parent owns panel strings and the modes agent owns TextView/control wiring.

New pair selection uses both eligible groups, or the available single group.
An explicit scope selection persists for that pair across generation reloads.
Comma/period keyboard edits are opt-in for locked kerning mode. Ctrl drops the
left class, Alt drops the right, both target the two glyphs, and Shift means ten
units. Existing Mod+Alt arrow shortcuts and their 1/10/100 steps remain intact.
Held exceptions capture their own rule identity without changing the panel's
persistent scope. Membership refresh, edit barriers, and explicit zero/remove
semantics remain required.

Acceptance: focused tests first for defaults, explicit scope persistence, pair
changes, temporary exceptions, coalescing across scope changes, modifier steps,
input/modal guards, and existing captured-run regressions. Parent performs broad
frontend checks and integrated browser acceptance after the mode wiring lands.
No Rust, generated API, shared JSON, or TextView edits in this task.

## Acceptance

Implemented eligible class defaults, per-pair explicit choices, and transient
modifier scopes with separately captured edit sessions. Automatic class choices
survive query reloads, preserving repeats while generation updates. Baseline
refresh/membership checks were extracted to keep the main hook under 400 lines.
No dependencies or FontForge source were used.

The initial new test run failed six intended behavior assertions. Final focused
results cover 34 tests: existing hook (9), runner (7), group impact (2), panel
(6), and new class behavior (10). One combined run could not collect the new
class suite because shared `en.json` was read while another task was writing it;
JSON validation passed afterward and that suite passed on its focused rerun.

```text
pnpm exec vitest run src/views/text/useTextKerning.classes.test.tsx
Test Files  1 passed (1)
Tests       10 passed (10)

Other four focused suites in the preceding run:
Tests       24 passed (24)

pnpm exec eslint <six owned TypeScript files>
exit 0
pnpm exec prettier --check <six owned TypeScript files>
All matched files use Prettier code style!
```

Modes integration and visible shortcut hint were handed to the parent/modes
agent; broad `pnpm check`, lint, full regression and browser acceptance belong
to the integration owner under resumed coordination. Artifact checker passed:
5.98 GiB build, 0.04 GiB shared temporary output, 669.35 GiB free. This task's
scratch directory was empty and removed; no owned processes remain.
