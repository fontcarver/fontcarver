# P2a independent frontend review

Date: 2026-09-16. Scope: text/group authoring, gap identity, edit sessions,
query races, and transition architecture. Product fixes were made by their
authors; this reviewer changed only review documentation.

## Findings and disposition

- Group creation could replace an existing group's membership. The UI now uses
  the kernel's atomic create-only command; collision regression passes.
- Explicit-name Escape could commit through blur. A cancellation guard fixes
  this. Name drafts also preserve newer typing during asynchronous pair changes
  and retire when an external selection changes.
- Gap highlighting could disagree with the explicitly selected pair, and
  reselecting the same gap did nothing. Selection now checks actual pair names
  and submits every gap selection with request/context guards.
- Old group resolution could remain editable during generation refresh.
  Queries now include generation, and numeric commands verify captured group
  identities after preceding edits finish.
- Generation refresh initially dropped repeated keys and scope changes. Runs
  now retain captured targets through active/closing handover; scope intent is
  captured before draining edits. Delayed-generation key regressions pass.

No additional blocking finding remains in those modules. Preview geometry
continues to use kernel positions; gap rendering adds handles only. Group
queries and impact previews are bounded, with keyboard-accessible controls.

## Verification

From `app`, ran `pnpm exec vitest run` on `textGaps.test.ts`,
`kerningEdits.test.ts`, `useTextKerning.test.tsx`, `TextKerningPanel.test.tsx`,
`useTextKerningControls.test.tsx`, `kerningImpact.test.ts` (all under
`src/views/text/`), and
`src/views/glyph/panels/KerningGroupsPanel.test.tsx`.

```text
Test Files  7 passed (7)
     Tests  39 passed (39)
  Duration  23.21s
```

Shell tab-transition hardening is a separate active task. Its author was
alerted to edits started during an already pending navigation snapshot. Full
checks, headless persistence/export acceptance, and localization QA belong to
the milestone report; this review did not run native automation or benchmarks.
