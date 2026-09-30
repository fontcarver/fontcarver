# P5 shell integration

Scope: audit rows 1–6, using delegated window transport, named workspace
persistence, palette navigation, and proofing divider components. Own AppShell,
TabBar, Sidebar, Inspector and new shell lifecycle/action modules. Existing
session and UI store changes are narrow integrations only.

Interfaces: windowActions opens document windows and detaches active tabs/panels
through the transport; WindowHost routes detached windows into their document
and view. Native document close uses the existing save/discard/cancel flow.
Saved workspaces restore per document path, with glyph names remapped after
reopening. Detached views share the kernel and retain edit settlement barriers.

Acceptance: focused transport/action/lifecycle/component tests cover success,
failure, tab state transfer, document scoping, dirty-close cancellation, and
persistence restore. Headless agent-browser verifies integrated shell controls;
Linux desktop compilation verifies Tauri wiring without native input. Parent
owns final broad frontend/Rust regression checks. Native macOS/Windows execution
is outside this Linux task's evidence boundary.
