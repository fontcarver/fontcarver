# P2a text kerning editor

Scope: reusable explicit-pair editor, scope selection, validated numeric drafts,
and coalesced keyboard edits. The root agent owns TextView and canvas integration.

Owned files: `app/src/views/text/kerning*.ts`, `useTextKerning.ts`,
`TextKerningPanel.tsx`, its CSS module and tests, plus
the `kerningText` section of `app/src/i18n/en.json`. Chinese translation is delegated separately.

Contract: `useTextKerning({doc, master, generation, selectedPair, prepare})`
returns `{finish, onKerningKey, panel}`. `TextKerningPanel` receives `model`,
optional translated `gapDisabledReason`, and `pending`. `finish` drains numeric
drafts and every captured keyboard session before transitions. `prepare` closes
the current controller edit without invoking the transition settler recursively.

Acceptance: explicit zero and remove-to-inherit remain distinct; group scopes
require memberships; unsupported ownership is disabled; Enter/blur commit and
Escape cancels; shortcuts respect inputs/modals/platform modifiers and coalesce
for 500 ms; delayed reads and edit closures preserve captured document/master/
pair targets and cannot overtake subsequent commands. No client layout offsets.

Verification: failing focused Vitest tests first, then implementation; focused
tests, `pnpm check`, `pnpm lint`, and `pnpm test`. Root performs integrated browser
and full-project acceptance.

## Result

Implemented the agreed hook/panel contract with generated API types. Group scopes
show at most eight affected members per side plus exact member totals, using the
resolution generation. Numeric and keyboard edits validate the captured group
identity against a fresh query before applying. Queries hide obsolete results;
active and closing keyboard runs retain repeats during their own generation
reload. Name-field drafts preserve later typing and cancel safely on Escape.

Focused verification: 21 passing tests across the runner (5), impact reads (2),
hook (8), and panel (6). Cases include 500 ms coalescing, explicit zero/removal,
reentrant preparation, old master capture, delayed replies, modal/input guards,
membership changes, and selection-away-and-back draft retirement.

`pnpm check` and `pnpm lint` passed. The full app test snapshot ran 988 tests:
985 passed; two glyph-opening assertions needed the separately delegated async
tab-transition update, and one existing TextView integration test exceeded its
5 second timeout under the parallel load. Required focused reruns are tracked
by the root acceptance report.

The focused TextView rerun passed all four tests; the earlier timeout case took
3.08 seconds. Numeric draft changes also publish a document edit revision so
strict tab handover can drain input staged while a previous edit was closing.
