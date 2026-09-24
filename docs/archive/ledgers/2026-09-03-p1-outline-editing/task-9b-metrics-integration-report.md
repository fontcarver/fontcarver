# Text metrics integration — root/Astra

Commit `7504142` on `p1-text-ui` owns only textMetrics.ts, useTextMetrics.ts and
its new hook tests. Independent Astra integrator reviewed the approach while
owning the captured controller preparation method.

Key presses synchronously create or update an EditSession, avoiding an invisible
async startup queue. The session captures previous barriers before its own close
can be published. A fresh glyphs.outline baseline is read after edit_begin and
previous tool work complete; early key deltas wait for that read, so stale React
metrics do not overwrite a just-finished edit. An optional runner refresh leaves
its existing synchronous callers unchanged. Empty outlines remain width-only.

A key arriving during a prior end creates a successor gated behind the captured
old close, including an automatic idle close while preparation remains pending.
Every run stores its document identity. Old completion/preview callbacks cannot
clear a successor's preview or update a replacement document/editor. Baseline
read failure reports once and closes the session without applying stale metrics.

Verification: 15 tests passed (10 helper, 5 hook). Hook tests cover delayed old
work followed by metrics and an ordinary command, delayed end plus another key,
automatic idle close during preparation, baseline read error, and replacement
identity/unmount. TypeScript passed; scoped ESLint/Prettier/diff checks passed.
Astra integrator separately ran 79 controller tests and reviewed this hook.
Actual integrated headless text e2e remains pending; this report is not final P1
acceptance. No native/headed input or new dependency was used.
