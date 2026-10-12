# P5 shell window transport

## Scope and ownership

Implement native Tauri and browser transport for document windows, detached tabs,
and detached sidebar, inspector and preview panels in the existing shared API
session. Add deterministic target URLs, focus/reuse, tab-state handoff, rejoin and
window-close events, plus frontend close-guard lifecycle. Shell components,
workspace persistence, localized prompts and session close decisions belong to the
parent shell task. No native desktop input automation or new dependencies.

Owned files: `app/src/transport/windows.ts`, supporting window-target module and
focused tests; `src-tauri/src/windows.rs`, narrow registration in
`src-tauri/src/lib.rs`, and detached-window capability scope.

## Public interface

`WindowTarget` distinguishes document, tab and panel targets and always carries a
session DocId. URLs use `fcWindow`, `doc`, `tab`, `gid` and `panel` query parameters.
`requestWindow(target, tab?)` opens or focuses a target. `readWindowTarget()` and
`readWindowTab(target)` restore routing and optional tab payload. `rejoinWindow`,
`onWindowRejoin`, and `onWindowClosed` coordinate restoration without closing the
kernel document. `installWindowCloseGuard` supplies asynchronous native close
confirmation and synchronous browser beforeunload dirty protection.

## Acceptance

Write focused failing tests first for target validation, tab-state round-trip,
popup failures/focus reuse, native invoke routing, rejoin/events and asynchronous
close cancellation/confirmation. Add Rust target URL/label validation tests.
Run focused Vitest and Rust tests with the shared Cargo flock. Parent runs final
frontend check/lint/suite and workspace Rust checks after integration. Record
native runtime verification as unavailable rather than claiming simulation.
