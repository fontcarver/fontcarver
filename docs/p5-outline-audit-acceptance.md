# P5-35 active-layer outline audit acceptance

Implemented on 2026-09-30. Bounded row 35 scope: the active source layer checks
open contours, fractional coordinates and coincident stored points by default.
Canvas markers provide hover details and keyboard selection; the issue panel
provides per-check toggles, individual Fix and Fix All. Fractional repairs round
reviewed points with existing SetPointPosition commands, batched into one undo
entry. Open contours and coincident points explicitly require manual review.

Repairs settle the editor, refetch and compare the source contours, reject stale
results, respect glyph/path locks, report failures and reload. Live gestures
disable repairs. Reports are bounded to 100 issues and 100,000 inspected points;
Fix All only applies to displayed fixable issues. Existing duplicate rings from
drawOutline remain independent of the audit check toggles.

Verification:

```text
pnpm --dir app test src/views/glyph/outlineAudit.test.ts src/views/glyph/OutlineAudit.test.tsx --maxWorkers=1 --minWorkers=1 --testTimeout=15000
Test Files  2 passed (2)
Tests       12 passed (12)
Duration    8.28s

pnpm --dir app exec eslint src/views/glyph/outlineAudit.ts src/views/glyph/outlineAudit.test.ts src/views/glyph/OutlineAudit.tsx src/views/glyph/OutlineAudit.test.tsx
exit 0
```

The earlier default-timeout run had one timeout under concurrent load; it was
rerun successfully. Coverage includes locations, toggles, bounded output, finite
markers, pan positioning, single/batch commands, source immutability, stale
data, refused mutations, gesture and lock states.

Limits: no new browser session was started under the final coordination freeze;
visual review belongs to root's shared acceptance session. Kernel undo/save/reopen
was not rerun for this UI slice. Broad frontend checks are centralized at root.
Additional geometry checks and automatic topology repairs are outside this slice;
row 35 should remain partial pending that acceptance evidence. No FontForge source
or new dependencies were used.
