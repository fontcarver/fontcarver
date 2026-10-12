# Welcome-screen command discovery

Date: 2026-10-02. The user explicitly requested one additional small next task
before the current usage window resets. Scope is one first-use discoverability
improvement from the future experience backlog, not the entire backlog.

Add a compact, optional discovery section to the no-document welcome screen:
Getting started (existing welcome help article), Command Palette (existing
command) and Keyboard Shortcuts (existing settings). Keep Open/New primary and
recent files working. Display live effective bindings for actions that have one,
respect custom/unbound maps and platform formatting; do not hard-code chords.
No automatic tour/modal, new dependency, new editor workflow or broad redesign.
Use existing neutral tokens and small buttons/rows, with keyboard focus visible
and the whole panel reachable in a short/narrow viewport.

Implementation owner: keyboard agent; own EmptyState.tsx/module.css, directly
related new focused tests and at most a few new English keys. Reuse existing
labels/help where appropriate. Separate GPT-6.1 Sol translator owns zh-CN after
English freeze. No other production files unless a concrete blocker is reported.
Test action routing, live custom/unbound shortcuts, keyboard reachability and
recent-file preservation. Parent checks real browser light/dark layouts and
existing dialogs, repeats final integrated checks, and rebuilds final packages.
Record acceptance in docs/p5ux-start-discovery-acceptance.md; no commits/push by
agent. Check artifact budgets and remove only owned temporary data.

Browser verification found an entry blocker in the existing command palette:
StrictMode replays its effect cleanup, restoring the welcome button while the
palette remains open. The additional authorized scope is the narrow palette
focus lifecycle and a StrictMode regression: the search field must retain entry
focus and Escape must close and restore the opener. Preserve focus chosen by an
executed command, existing modal rules and IME behavior. No other scope expands.

## Gap analysis before implementation

EmptyState currently offers only file actions/recent paths. Add three quiet
rows below recent files, reusing the welcome article and registered commands.
Read effective bindings from the registry plus the live keymap, including both
palette entry IDs, and omit unbound hints. Subscribe to late registry updates
because AppShell registers commands after child mounting.

The existing vertically centered flex item can overflow above a short scroll
container. Use a non-shrinking card with automatic block margins in a column
layout so it centers when there is room and remains scrollable from its top.
One new English heading (`empty.explore`) is sufficient; existing labels are
reused. Focused tests cover routing, mutable bindings/registration, Tab order
and existing file actions. Parent owns actual viewport rendering checks.

## Browser-discovered prerequisite: palette focus under StrictMode

The parent twice reproduced the new palette button opening a dialog while
focus returned to the underlying welcome button. React development StrictMode
replays the focus effect's cleanup/setup: cleanup restores the opener, but
React's initial `autoFocus` does not repeat in setup. Parent authorizes a
narrow `CommandPaletteDialog.tsx` setup-focus repair and StrictMode regressions
in `CommandPalette.test.tsx`. Verify click/keyboard activation, immediate
typing, Escape restoring the opener, command-selected focus and existing IME
behavior. No additional product scope or locale changes.
