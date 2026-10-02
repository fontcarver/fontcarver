# Welcome-screen discovery acceptance

2026-10-02. One bounded first-use improvement: three quiet, optional rows below
recent files open Getting started, Command Palette and Keyboard Shortcuts.
The welcome article and existing commands are reused; nothing opens
automatically. New/Open/recent-file behavior remains covered.

Shortcut hints use current registry metadata and effective keymap overrides.
Both palette bindings are shown when present, equivalent labels are deduplicated,
and null/unbound hints disappear. Late command registration and live keymap
changes update the display; macOS and Linux formatting are tested.

The card uses automatic vertical margins in a non-shrinking column layout:
it centers with spare height and scrolls from its top when content is taller.
Actions and shortcut rows wrap at narrow widths. Buttons retain the shared
visible focus treatment. Actual viewport rendering is the parent's browser
acceptance, not a jsdom claim.

Only `empty.explore` was added in English: “Explore FontCarver”. Existing
localized action labels are reused. The separate GPT-6.1 Sol translator owns
the Chinese addition.

Five discovery regressions initially failed; keyboard traversal and file actions
remain covered.

Browser verification exposed StrictMode replay restoring the palette opener
without refocusing search. Setup now re-establishes search focus. Three failing
regressions now pass: pointer/keyboard activation, typing/Escape restoration,
and command-target focus. Final palette/routes/IME/welcome verification:
**37/37 (6.08s)**; type/lint/format clean. No additional locale changes.

```sh
pnpm --dir app exec vitest run src/shell/EmptyState.test.tsx src/shell/contextHelp.test.tsx src/shell/CommandPalette.test.tsx src/shell/ShortcutSettingsDialog.test.tsx --maxWorkers=1
pnpm --dir app check
pnpm --dir app exec eslint src/shell/EmptyState.tsx src/shell/EmptyState.test.tsx
```

```text
Test Files  4 passed (4)
     Tests  39 passed (39)
  Duration  5.75s
tsc / ESLint / Prettier / git diff --check: exit 0
```

Task scratch removed; artifact checkpoints passed (build 5.33 GiB).
No dependencies, commits, publication or broad build/test run by this agent.
