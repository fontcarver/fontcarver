# P5 proof shaping controls acceptance

Implemented `TextShapingControls.tsx`, token-based CSS, and six component tests.
The compact disclosure exposes Default/On/Off feature selectors, custom tags,
script/language override validation, cancellation, and glyph clusters in visual
order with UTF-8 source offsets and font-unit advances. Default removes the
override. Additional tags start at Default without altering shaping.

Tests were written first and initially failed because the component was absent.
The final focused verification commands/results were:

```text
pnpm --dir app test src/views/text/TextShapingControls.test.tsx --testTimeout=30000
Test Files  1 passed (1)
     Tests  6 passed (6)
  Duration  23.04s

pnpm --dir app exec eslint src/views/text/TextShapingControls.tsx src/views/text/TextShapingControls.test.tsx
exit 0
```

Prettier formatted only the three owned frontend files. An intermediate run
timed out after 5 seconds under shared load; the rerun used a CLI timeout only.
No dependencies or generated API files changed. English `proofShaping` keys were
handed to the parent; translations are independently owned.

TextView/store wiring, removing duplicate TextBar controls, browser visual
verification, and final `pnpm check`, `pnpm lint`, `pnpm test` belong to the parent
integration task and are not claimed here. No FontForge sources were used.

Artifact checks passed: initial build 5.23 GiB/12,405 files; pre-completion build
5.96 GiB/13,090 files, temporary 0.02 GiB/2,051 files, free 669.44 GiB. Growth is
shared concurrent work. This task's empty scratch directory was removed and no
owned test/server processes remain. No open questions.
