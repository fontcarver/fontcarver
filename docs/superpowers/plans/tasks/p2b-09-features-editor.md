# P2b 09 — Features editor, lookup inspector and edit handovers

Date: 2026-09-17. Status: implemented and independently reviewed.
Dependencies: 02–05 interfaces fixed; integrate 07/08 before completion.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`app/src/views/features/`, shell tabs/menus, editor edit-participant registry,
transport hooks, English i18n, component/headless tests; package manifests.

## Interfaces and work

Add CodeMirror 6 directly with minimal state/view/language/autocomplete/lint/
commands packages. Verify exact locked dependency licenses and justify them in
THIRD_PARTY/commit text. Add FEA highlighting, completion, source tree, diagnostics,
automatic controls, Compile & preview, lookup inspector and conversion/revert UI.
Render lookup rules as glyphs plus names with bounded paging. Imported FEA is
read-only until a proven baseline conversion; generic apply cannot bypass it.

Generalize document edit participants beyond EditorController. Flush to saved
draft on navigation/save/history; propagate persistence errors. Validation
failure alone does not block switching tabs after draft persistence. One undo
owner, IME-safe flush, Ctrl/Cmd+S inside contenteditable, revision-bound late
responses, accessible keyboard paths and generated transport types are required.
Connect imported pair controls only to task 08's proven eligibility result.
Write English keys; another agent owns Chinese translation.

## Acceptance tests

Component tests cover untouched-view cleanliness, diagnostic navigation,
Unicode/IME, no-op/undo, source/manual conflicts, apply failure, stale results,
slow/failed save, cross-document/tab/master transitions and close recovery.
Also delay diagnostics across glyph deletion and undo/redo: accept results only
when document/master/draft and effective compile-input revisions still match.
Real API headless tests prove source editing and imported conversion/revert.
Read browser skill and inspect light/dark screenshots using the existing design.

## Verification and handoff

`pnpm check`, `pnpm lint`, focused Vitest then full app suite at acceptance,
`pnpm build`; isolated P2b Playwright and visual verification. No native automation.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.

## Completion evidence

Implemented the complete source/lookup UI, CodeMirror completion and UTF-16
diagnostics, document-owned undo/IME handling, generation/freeze, source proof,
conversion and native restore. Generic edit participants protect both new FEA
drafts and existing outline/spacing edits during handovers and final close.
All 13 added/transitive editor packages are MIT, recorded in `THIRD_PARTY.md`.

Independent review passed 59 Features tests and 39 navigation/handover tests.
The complete frontend suite passed 1,075 tests; TypeScript, ESLint, production
build and isolated real-backend workflows passed. Final visual review moved
optional compiler proof details under a translated disclosure; 11 targeted
tests cover that change. See [P2b acceptance](../../../p2b-acceptance.md).
