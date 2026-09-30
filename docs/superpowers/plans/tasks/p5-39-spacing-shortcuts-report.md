# P5-39 spacing shortcuts acceptance

Implemented selected-side brackets for LSB, RSB and both: plain brackets use
the configured step, Shift multiplies by 10, platform Mod by 100, and
Shift+Mod forces exactly one font unit. Existing Mod+arrow shortcuts are
unchanged. Both-side edits publish one preview and one absolute SetMetrics
command per scheduler frame; blank glyphs receive the summed advance change.

Added accessible `SpacingControls` with side selection, disabled state and
modifier help. English keys live under `spacingControls`; the independent
localization owner handles Chinese. Split mapper tests into
`textMetricShortcuts.test.ts` to keep each test file below 400 lines.

Tests were written first: the initial focused run reported 13 failures before
implementation. Final command:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-39-spacing-shortcuts pnpm --dir app exec vitest run src/views/text/textMetrics.test.ts src/views/text/textMetricShortcuts.test.ts src/views/text/SpacingControls.test.tsx
Test Files  3 passed (3)
Tests       27 passed (27)
```

Focused ESLint on all five owned TypeScript files exited 0. Prettier checks
passed for these files and `SpacingControls.module.css`. Tests cover both
signs, fractional base steps, all modifier modes on both platforms, shifted
and physical bracket keys, fallback steps, legacy compatibility, atomic
both-side preview/commands, empty glyphs and edit-session closure.

The parent owns hook/TextBar/TextView integration, preference-step wiring,
locked-mode gating, browser verification and broad frontend checks. Row 39
must await that integration before acceptance; this report does not claim
end-to-end completion or real kernel undo/persistence verification.

Owned temporary directory was empty after tests and removed. Artifact budget
passed at start and completion; no build cache cleanup was required.
