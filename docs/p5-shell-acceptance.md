# P5 shell acceptance — 2026-09-30

Implemented document window routing; pinned Overview; numbered Edit selection
(Mod+Alt+1–9), Ctrl+Tab cycling, and Mod+K palette access; tab tear-off buttons
and drag-out; detached sidebar/Inspector/preview hosts and return actions;
per-document remembered layouts and named workspace presets; palette routes
for commands, filters, scripts, and glyph names. Existing edit-settlement and
save/discard/cancel decisions protect view transitions and native closes.

Workspace snapshots remap glyph names after reopen, preserve text settings,
migrate after Save As, merge concurrent window writes, and reject corrupt data.
Session startup preserves the restored active editing tab. GlyphView publication
now depends on text mode/presence, preventing unrelated text-run callback changes
from republishing the editor state indefinitely.

## Verification

Focused commands and actual final outcomes:

```text
vitest windowActions TabBar workspaceTabCommands sessionStore
Test Files 4 passed (4); Tests 24 passed (24)
window transport: 27 passed
window lifecycle: 14 passed
workspace store/dialog/lifecycle/host: 35 passed
palette command/search/routes: 17 passed
TextView.master: 1 passed (proofing subsequently verified its key/callback fixes)
scoped ESLint: exit 0
```

Headless agent-browser against isolated API 8893 and UI 1443 verified:
Overview rendered without a close button; saving “Two dock review”; hiding
Inspector (zero matching dividers), applying the preset (one divider); opening
a separate Inspector popup; returning closed the popup and restored its dock;
palette displayed all five search categories. Screenshot:
[`detached-inspector.png`](acceptance/p5-shell/detached-inspector.png).
The return click produced a CDP timeout because its window closed; the subsequent
tab list and divider count verified the actual outcome.

## Boundaries

Native compilation is delegated to root (`cargo test -p fontcarver --lib
windows::tests`); this task's queued invocation was cancelled before execution.
No native OS input or macOS/Windows execution occurred. Scripts/saved-filter
palette routes have focused tests; the isolated pre-existing API binary lacks
those new handlers. Root owns broad checks and final checklist reconciliation.
All owned browsers/servers stopped and scratch removed; final resource check
passed. No dependencies, commits, or FontForge-derived code were introduced.
