# P5-40 kerning modes acceptance

Implemented optional per-tab `kerningMode` (`off` / `on` / `locked`, default
`on`) and `spacingSide` (`lsb` / `rsb` / `both`, default `lsb`). Off overrides
both `kern` and `vkrn` shaping settings without replacing stored feature choices
and prevents kerning mutations. Locked retains kerning while guarding metrics
keyboard commands and metric-handle dragging. Horizontal and vertical inspector
owners integrated the same live guard. Proofing owner integrated TextView mode,
spacing controls and punctuation typing reservation; workspace owner included
both fields in snapshot validation. English keys: `kerningModes.*` and
`spacingControls.*` (independent translation owner notified through parent).

Spacing controls use [ / ] for signed changes to the selected side(s), the
current preference base step, Shift ×10, Mod ×100, Shift+Mod exactly 1. Legacy
Mod+arrow mappings remain. Both-side edits use one existing undo-coalesced run.
See p5-39-spacing-shortcuts-report.md for helper coverage.

Verification:

- `pnpm -C app test src/views/text/useTextMetrics.test.tsx src/views/text/useTextTyping.test.tsx src/views/text/useTextKerningControls.test.tsx src/editor/editorController.test.ts`: 4 files, **94 passed**.
- Earlier helper/control/TextBar suites: **8 passed**; spacing child: **27 passed**.
- Focused ESLint: exit 0. Owned formatting and `git diff --check`: passed.
- Artifact checker: PASS; build 6.09 GiB, temporary 0.13 GiB, free 668.79 GiB.
  Owned scratch empty and removed; no owned running processes.

No broad frontend check or browser acceptance was run by this task; parent and
proofing owner retain those integrated checks. Row 39/40 final acceptance must
retain this boundary. No FontForge code was consulted or adapted. Scope frozen
at the user's stop-after-current-work steering; no new work delegated afterward.
