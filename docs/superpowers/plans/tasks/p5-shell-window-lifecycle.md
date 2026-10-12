# P5 shell window lifecycle

Scope: new `app/src/shell/windowLifecycle.ts` and focused lifecycle tests only.
Integrate the delegated window transport with the existing session and UI stores;
parent owns AppShell integration and final broad frontend checks.

Public interface: `installWindowLifecycle(): () => void` installs routing,
rejoin, browser/native close protection and focus-time shared-session refresh.
Target windows restore their document Overview plus a transferred tab. Desktop
main windows route additional document tabs to dedicated document windows.
Browser main windows retain the existing multi-document workflow.

Acceptance: focused tests cover document scoping and delayed session load,
transferred text tab state, desktop/browser opening, routing failure, rejoin,
clean cached documents with pending drafts, native Cancel and Save flows, and
cross-window document closure on focus. Native runtime behavior is represented
by a mocked transport; no macOS/Windows desktop execution is claimed.

Validation completed:

```text
pnpm --dir app exec vitest run src/shell/windowLifecycle.test.tsx
Test Files  1 passed (1)
Tests       14 passed (14)

pnpm --dir app exec eslint src/shell/windowLifecycle.ts src/shell/windowLifecycle.test.tsx
exit 0

pnpm --dir app exec prettier --check src/shell/windowLifecycle.ts src/shell/windowLifecycle.test.tsx
All matched files use Prettier code style!
```

The tests first failed because the implementation module did not exist. Final
tests exercise the real session store and CloseDocumentDialog with a simulated
platform: cached-clean drafts are flushed, Cancel retains the document, Save
closes only after a successful kernel save, failed saves keep the window open,
and detached-view closes settle drafts without closing the shared document.
Rejoining a tab also settles the owner's current draft before activating it.
Storage failures produce a toast instead of throwing through a UI subscription.

Resource checks passed at start (5.23 GiB build, 0.00 GiB temporary) and final
validation (6.07 GiB build, 0.03 GiB temporary; concurrent tasks own the increase).
This task's scratch directory was empty and removed. No owned process remains.
Parent owns final `pnpm check`, broad `pnpm lint`, full suite and integrated
browser acceptance. Native macOS/Windows execution remains unverified.
