# P5 proof preview controls acceptance

Implemented horizontal/vertical reflection, independent theme inversion, bounded
blur, validated custom waterfall sizes, selected-glyph mode, and pointer/keyboard
resize. Optional `activeGlyph`, `onDetach`, `height`, and `onHeightChange` retain
compatibility for existing callers. Large waterfall sizes now allocate sufficient
canvas height. Color-only glyphs invoke their painter in the shaped font frame.

English `proofPreview` strings were supplied to the parent, who integrated them;
this agent did not modify either locale file. No FontForge sources or new
dependencies were used.

Tests were written and observed failing before implementation. Final focused
command:

```text
TMPDIR=<workspace>/.scratch/p5-proof-preview-controls pnpm --dir app test \
  src/views/text/PreviewStrip.test.tsx src/views/text/previewPaint.test.ts
Test Files  2 passed (2)
     Tests  10 passed (10)
```

Targeted ESLint passed with exit 0. Prettier checked the seven owned frontend
files. Parent owns full frontend check/lint/test, store/TextView integration,
detached-window behavior, and browser visual acceptance; those are not claimed
here. Controls other than optional controlled height are local UI state.

Artifact check passed at start (5.23 GiB build) and after focused checks
(5.86 GiB build; workspace concurrent tasks account for changes). This task left
no test/server/browser process or temporary output. No open implementation
questions in this delegated scope.
