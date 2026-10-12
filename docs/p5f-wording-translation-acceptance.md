# English wording and Chinese delta acceptance

Date: 2026-10-02. Branch: `p5-color-authoring`. Validation recorded before commit.

Closed the five wording items from the wave-2 translation reviews. English
changes comprise nine wording corrections and 34 apostrophe normalizations:
waterfall sizes versus range increments, actual bitmap filter names, the
Workspaces… command, and consistent curly apostrophes. Removed the unused bare
`cid.unmapped` key from both catalogues, retaining counted plural forms.
Updated existing preview/workspace test expectations to match the copy.

The user-assigned **GPT-6.1 Sol sub-agent** independently reviewed all 43 changed
English values. It revised the Chinese workspace command reference; the other
42 existing translations remained accurate. Both catalogues contain **3,935
matching keys**, with matching interpolation placeholders. See the
[translator's report](p5f-wording-translation-pass.md) and
[task scope](superpowers/plans/tasks/p5f-wording-translation.md).

Validation:

- `pnpm check` and `pnpm lint`: exit 0.
- Existing i18n/spelling checks: **2 files, 3 tests passed**.
- i18next runtime checks for `cid.unmapped`, English/Chinese counts 1 and 2:
  **4 passed**, with no fallback to the removed key.
- ESLint and Prettier checks on the changed catalogues/test files: exit 0.
- Initial `pnpm test`: **338 files / 2,774 tests passed**, one workspace
  assertion still expected a straight apostrophe. That assertion was corrected.
- Final invocation ran the complete suite despite supplied file arguments:

```text
pnpm test -- src/dialogs/WorkspaceDialog.test.tsx src/views/text/PreviewStrip.test.tsx src/i18n/i18n.test.ts src/i18n/spelling.test.ts --maxWorkers=1
Test Files  339 passed (339)
     Tests  2775 passed (2775)
  Duration  354.86s
```

This is a wording delta and independent translation review, not a new linguistic
audit of every string or a browser/native-platform acceptance run. The earlier
browser-download environment limitation remains outside this task. No feature,
release or external-hinter work was started.

Resources: owned test processes finished and task scratch removed. Final
`python3 scripts/check_artifacts.py`: **PASS**; build cache unchanged at
**14.91 GiB** (existing 10 GiB warning, below the 20 GiB ceiling).
