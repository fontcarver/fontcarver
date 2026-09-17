# P2a-01: Glyph inspector kerning groups

## Scope and ownership

Implement first- and second-side membership controls in the glyph inspector.
Own `KerningGroupsPanel.tsx`, supporting `kerningGroup*`/`KerningGroup*`
modules, CSS and tests under `app/src/views/glyph/panels/`, and the
`EditInspector.tsx` integration. Do not change text editing or generated API.
English strings are integrated under `kerningGroups` in `app/src/i18n/en.json` for the
root agent to merge under `kerningGroups`; another agent translates Chinese.

## Contract

Use typed `kerning.glyph`, `kerning.groups`, `kerning.members` queries and
existing group document commands. Pages are bounded and carry generation.
Requests from obsolete document/glyph/generation contexts cannot update the
current panel. Display group membership and pair counts before group-wide
operations. Removing an in-use group requires an explicit group-and-pairs
action. Imported/source-owned kerning stays read-only with a product explanation.
Register document barriers before pending edits can be overtaken by save/close.
Creation uses atomic `CreateKerningGroup` followed by `AssignKerningGroup` in
one batch. An existing name is rejected without replacing its members.

## Acceptance

Test creation and assignment commands, generation-bearing pagination, atomic
rename and explicit pair deletion, unsupported eligibility, delayed query
handover, mutation errors, and barriers. Run focused component tests, TypeScript
check and lint; the root agent runs combined application/headless acceptance.

## Completed implementation and focused verification

- Inspector includes independent first/second-side managers with searchable
  25-group pages and generation-checked 25-member pages. Canonical UFO names
  may be entered whole or as a suffix. Group creation, assignment, rename,
  and explicit group-and-pair deletion use existing document undo commands.
- Membership/pair-count previews precede group operations. Eligibility and
  malformed imported membership receive localized explanations.
- Query identities include document, glyph, side, generation and page.
  Mutation completion guards protect later selections; document barriers
  are registered before awaiting editor settlement or an API response.
- `pnpm exec vitest run src/views/glyph/panels/KerningGroupsPanel.test.tsx`:
  11 tests passed. Existing `EditInspector.test.tsx`: 26 tests passed.
- Scoped ESLint passed. Root coordinates integrated TypeScript, lint and the
  full application suite while independent agents finish adjacent files.
- Separate Chinese translation and combined headless visual acceptance are
  handled by the assigned translation and root/headless agents.

## Visual QA followup

The root agent assigned the empty-text-mode state in `EditSidebar.tsx` and
`EditInspector.tsx`. When text mode intentionally has no active glyph, both
show a localized selection hint; actual edit-mode requests retain loading
feedback. Four new regressions cover both states in both components. The
complete sidebar and inspector suites pass: 33 tests. English test resources
now import the committed `en.json` catalog instead of the temporary snippet.
