# Keyboard usability audit and repair

Date: 2026-10-02. User asked whether the editor feels modern and friendly, has
rich shortcuts, and asked us to actually try it and improve concrete problems.

Read CONVENTIONS.md. Audit current command registration, defaults/custom keymaps,
focus, menu/palette discovery and help documentation against actual code and
existing evidence. Initially read-only: report a concise supported shortcut
inventory and concrete reproducible usability gaps, with priority and exact
files. Parent owns the actual browser session and shared UI design changes;
request an explicit disjoint repair assignment before editing. No broad builds.

Preserve existing professional workflows and user customizations. No new
dependencies. New English UI text needs separate GPT-6.1 Sol translation. Use
only task-owned scratch; check artifact budgets. No desktop/global input
automation. Write findings in docs/p5ux-shortcuts-review.md, with claims bounded
to inspected or actually exercised behavior. Do not commit or publish.

## Authorized repair scope

The parent assigns repair of the confirmed duplicate tool shortcut rows and
global-listener interference with customized tool tap/hold, menu keyboard focus,
IME-safe command-palette Enter, and physical Control+Tab parsing. Own
keymapTargets/shortcutSyntax/shortcuts, MenuBar, CommandPaletteDialog and directly
related tests. Preserve command routing, existing platform semantics and text
editing shortcuts. Do not edit canvas/bitmap/layout files or shared locale files
without coordinating key deltas. Reproduce with meaningful failing regressions;
run focused tests and relevant type/lint/format checks. Parent visually verifies
the integrated behavior and handles final broad tests/translation.

## Gap analysis before repair

The registry and tool hook independently consume customized tool bindings;
the shortcut editor also lists both registrations. Menu entry focus depends
on an `open` state transition and execution loses the previous editor focus.
Palette key handling lacks composition guards. `Ctrl` is historically a
portable `Mod` alias, so changing its meaning would break saved maps.

Add integrated regressions before repairing these paths. Parent approved an
explicit `Control` modifier, preserving legacy `Ctrl`/`Cmd` aliases; necessary
scope includes keymapDocument, keymap conflict comparison,
workspaceTabCommands and keyboard-shortcuts.md. Preserve physical Control
through import/export and compare overlaps according to the active platform.
No new locale strings are expected. Browser/native-platform verification stays
with the parent; this task exercises synthetic keyboard/DOM behavior.

## Final browser gate follow-up: bitmap entry focus

The full browser suite reproduced `bitmap.spec.ts`'s first M key doing nothing.
The browser owner confirmed focus remained on the closed Bitmap menu title
after opening its asynchronously loaded editor. Interactive-control protection
correctly ignored that letter; customized tool dispatch was uninvolved.

Parent authorizes `BitmapCanvas.tsx` and a focused entry regression. On mount,
focus the new canvas only from the body or a closed menu title, with no modal.
Verify both pointer and keyboard menu activation, deferred mount, every bitmap
tool key, and no focus theft from fields, buttons, an open menu or a modal.
Preserve menu Escape/Tab behavior, existing canvas restoration, outline tool
tap/hold and all gesture handling. Browser owner keeps the original E2E
assertion; independent reviewer checks this narrow follow-up.

## Full-suite follow-up

The full browser suite reproduced a new-view focus regression: opening Bitmap
editor from the menu leaves focus on the closed menu title, suppressing bare
tool shortcuts. Scope includes the narrow menu/new-view focus handoff and its
tests, including bitmap view/canvas entry if necessary. Verify pointer and
keyboard menu activation, preserve dialog focus, Escape/Tab behavior, existing
canvas return and remapped outline tap/hold. Do not work around it by focusing
the canvas only inside the old browser test. The browser owner retains spec
ownership; parent repeats integrated checks after the repair.
