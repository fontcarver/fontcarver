# P5 row 108 context-help acceptance

Implemented offline contextual help with sixteen topics, current dialog titles,
focused-control descriptions, topic switching, and nested-dialog focus restore.
The Help command declares modal/input eligibility and default F1; the shortcut
owner integrates it with editable bindings. Live hover context clears on pointer
exit, focus changes and window blur; background hover cannot override an open
modal. `helpHint(topic)` provides translated tooltip and screen-reader guidance.
English strings were handed off and merged into `contextHelp`; translation is
owned by the independent localization pass.

Tests were written first: the initial run failed because the implementation was
absent; the hover regression failed with `welcome` instead of `export` before
pointer tracking was added.

Final focused command output (2026-09-30):

```text
$ TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-context-help pnpm exec vitest run src/shell/contextHelp.test.tsx
✓ src/shell/contextHelp.test.tsx (16 tests) 664ms
Test Files  1 passed (1)
Tests       16 passed (16)

$ pnpm exec eslint src/shell/contextHelp.ts src/shell/contextHelp.test.tsx src/shell/ContextHelpDialog.tsx src/shell/ContextHelpArticle.tsx
[exit 0; no output]
```

File-scoped Prettier completed. Start artifact check: 5.23 GiB build,
670.47 GiB free, PASS. Completion checkpoint: 6.07 GiB build,
0.03 GiB temporary, 669.21 GiB free, PASS; shared build growth belongs to the
parallel workspace. This task created no servers or browser processes.

Full frontend checks, the integrated F1 dispatcher and headless visual acceptance
are owned by the parent integration task. This report does not claim those checks
or native desktop verification. No dependencies or FontForge code were used.
