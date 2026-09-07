# HTTP timeout fix report

Branch: `p1-http-timeouts`  
Base: `0661ce4` (`p1-outline-editing` when dispatched)  
Commit: `727708e fix(app): extend long-running HTTP deadlines`

## Change

`HttpTransport` now selects a 120,000 ms default deadline for `text.shape` and `session.export`, whose compile/export work can exceed the standard request window. All other methods retain the 30,000 ms default. `HttpTransportOptions.timeoutMs` remains a universal override, including for shaping and export.

The transport still sends each request once. Timeout reporting now uses the selected deadline. The network-error comment no longer claims an aborted request could not have reached the kernel.

Tests spy on `AbortSignal.timeout` and assert exact deadlines for shaping, export, a normal request, and an explicit override across all three method classes. Existing timeout and error behavior remains covered.

## Verification

- `pnpm --dir app exec vitest run src/transport/HttpTransport.test.ts` — 10 passed
- `pnpm --dir app check` — passed
- `pnpm --dir app lint` — passed
- `pnpm --dir app exec prettier --check .` — passed
- `git diff --check` — passed

The Source Han compile benchmark was not rerun, per dispatch; its observed 76.779-second response established the deadline mismatch.
