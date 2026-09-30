# P5 layer/anchor workflow acceptance

Implemented named custom-layer add/duplicate/rename, selected full-path/component
copy to the master background, Mod+B foreground/background editing (creating an
undoable background when needed), and component lock controls that preserve base
inspection while disabling geometry edits/decomposition. Source glyph/layer
identity is checked before delayed background navigation. Shared component rows
also retain the separately owned Smart Component controls.

Validation (focused, single-worker Vitest):
- `layerWorkflows.test.ts`: 3 passed.
- `layerCommands.test.ts`: 5 passed, including creation failure and locked glyphs.
- `LayerManagementControls.test.tsx`: 5 passed.
- `ComponentRow.lock.test.tsx`: 1 passed.
- `panelCommands.test.ts`: 15 passed.
- Targeted ESLint over these modules and existing edited panels: exit 0.

The component fixture initially required the new explicit unlocked default; this
was corrected. One interaction test exceeded 5 seconds during heavy parallel
load; its unnecessary asynchronous click was replaced by a synchronous click,
and the final affected suites passed (16/16). Other three suites passed (13/13).

Kernel layer/lock owners have implemented command semantics, undo and FCF tests;
queued Cargo runs were canceled on integration-owner request, so their final
verification belongs to the shared Rust gate. Mark-cloud and audit implementations
and focused evidence are reported by their delegated task reports. No browser
acceptance or broad frontend/typecheck is claimed here; shared integration owns
those gates. Audit scope is three checks, with safe fractional rounding fixes;
open contours and coincident points require manual review.

Resource checkpoint: PASS; build 6.14 GiB / 13,184 files; temporary 0.13 GiB /
4,273 files; free 668.73 GiB. No owned servers or browser sessions. Empty owned
scratch removed. No new dependencies or adapted FontForge sources.
