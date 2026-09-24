# T12 browser workflow and paint gate preparation — Sol

Create isolated `p1-acceptance-browser` worktree from root's identified T8 base.
Read CONVENTIONS, task-12-brief, task-12-resume-rulings and docs/p1-acceptance.md.
Own new `app/e2e/acceptance*.spec.ts` and narrowly scoped e2e helpers only; T9b
owns the text UI. Preparation may precede T9b, but final evidence must run after
its integration. Use ports8795/1429, fresh binaries, workers=1 and isolated config.

Implement the two actual UI workflows, independently verifying kernel state:

- Open real licensed DejaVu TTF via the app's browser path prompt, navigate to
  A/B/C and move a real point in each using pointer events; export via the dialog;
  reopen the exported font and verify the three changed coordinates plus CLI
  inspect. Existing kernel tests own raw-table/hinting preservation details.
- New Font through the dialog, add A-C through Add Glyphs, draw simple closed
  contours using Pen in the actual canvas; set meaningful widths; export via
  the dialog and reopen, verifying outlines/widths plus current CLI inspection.

Add the <=8ms pointer-handler-through-repaint gate for a 300-point glyph. API
setup is permitted for this synthetic timing corpus. Use browser-side timing
around synchronous pointer dispatch, including hit testing, preview creation
and synchronous canvas rendering. First prove a pointer-down started an editing
gesture; each measured move changes preview coordinates. Warm20/measure200
or a comparably justified sample. Report median/p95/max; enforce original<=8ms
without weakening it to renderer-only measurements. Record renderer timing
separately if useful. End the gesture, check actual persisted delta and undo
restoration; do not allow a pan/no-op to pass as editing. Exclude Playwright
network latency explicitly. Use dev inspection handles only for viewport/data
observation and timing setup, not replacing the pointer interaction under test.

Test against existing T8 base now only as preliminary evidence. After T9b merge,
adapt selectors as necessary and rerun these gates in the final integrated UI.
Do not claim whole P1 completion or native-dialog coverage from browser tests.
Fix small discovered defects in isolated commits or report owner conflicts.
No new dependencies or P2 features. Commit clean branch, no merge/push.
Report to root ignored ledger task-12-browser-report.md with exact outputs,
measurement boundaries, fixtures, skips and remaining integrated verification.
