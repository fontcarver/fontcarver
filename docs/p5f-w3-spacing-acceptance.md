# Remaining spacing keyboard increments

Date: 2026-10-02. Task: [p5f-w3-spacing](superpowers/plans/tasks/p5f-w3-spacing.md).
Audit rows 39 (four spacing increments) and 40 (spacing lock regression).

## Changes

The canvas now accepts Alt+[ / Alt+] for 100 times the first Preferences nudge
distance, and Alt+Shift+[ / Alt+Shift+] for exactly one font unit. These operate
on LSB, RSB or both selected sides. Alt is Option on macOS. Existing bare and
Shift brackets keep their base and ×10 behavior. Every Ctrl/Meta bracket chord
remains unclaimed by spacing, including Ctrl+Alt/AltGr; Mod+[ / Mod+] continue
to navigate between glyphs without editing the glyph being left.

All eight fixed bracket chords now participate in keymap conflict detection.
The default map and all named presets remain conflict-free; assigning a global
command to one of those chords produces an explicit conflict. The accessible
spacing help no longer advertises the obsolete navigation-conflicting bindings.
The user guide and shortcut table describe all four working increments.

The existing edit-session and undo grouping implementation is reused. A focused
hook test proves that a 2.5-unit base step with both sides selected, Alt+] then
Alt+Shift+[, adds 249 units to each side, increases the width by 498, and uses
one `doc.edit_begin` / `doc.edit_end` pair. Both added gestures are consumed
without making any API request while spacing is locked.

## Validation

Tests were changed before implementation. The initial focused run of four test
files returned **16 failed, 24 passed**, specifically exercising absent Alt
gestures, missing reserved conflicts and incorrect accessible guidance.

With `TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5f-w3-spacing/tmp`:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/text/textMetricShortcuts.test.ts \
  src/views/text/useTextMetrics.test.tsx \
  src/views/text/SpacingControls.test.tsx \
  src/views/text/textMetrics.test.ts \
  src/shell/keymapIntegration.test.ts src/shell/keymap.test.ts \
  src/views/text/kerningEdits.test.ts
Test Files  7 passed (7)
     Tests  65 passed (65)
  Duration  12.02s

# After adding two explicit active-glyph navigation dispatch regressions:
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/shell/keymapIntegration.test.ts
Test Files  1 passed (1)
     Tests  13 passed (13)
  Duration  2.22s

pnpm --dir app check
> tsc -b
exit 0

pnpm --dir app exec eslint src/views/text/textMetrics.ts \
  src/views/text/textMetricShortcuts.test.ts \
  src/views/text/useTextMetrics.test.tsx \
  src/views/text/SpacingControls.test.tsx \
  src/shell/keymapTargets.ts src/shell/keymapIntegration.test.ts
exit 0
```

Prettier was run only over the owned files. A subsequent type-check during
parallel bitmap work reported only an unfinished `bitmapWaterfallLayout` test
import; the final integrated type-check belongs to the parent after that work
settles. This does not replace the earlier clean type-check result above.

The E2E owner added and ran `app/e2e/spacing-shortcuts.spec.ts` against the real
kernel on Linux headless Chromium:

```text
FC_API_PORT=8851 FC_UI_PORT=1461 \
TMPDIR=$REPO/.scratch/p5f-w3-e2e/tmp \
pnpm --dir app exec playwright test e2e/spacing-shortcuts.spec.ts \
  --repeat-each=3 --output=$REPO/.scratch/p5f-w3-e2e/spacing-results
3 passed (21.3s)
```

This selects an actual shaped glyph into edit mode, sets a 2.5-unit nudge and
both-side spacing, checks +250 then −1 on both sidebearings through
`metrics.spacing`, verifies separate settled undo/redo entries, blocks both
shortcuts while spacing is locked, and confirms Ctrl+] navigation leaves all
five source glyphs’ metrics unchanged. The first browser attempts exposed
setup assumptions (proofing mode without an active glyph, font order differing
from proof order), which were corrected in the test; they are not evidence of
old-mapper behavior. No claim of native macOS or Windows keyboard validation
follows from the synthetic platform-mapping unit tests.

## Translation and review

The independent GPT-6.1 Sol translation pass must review changed
`spacingControls.help` and new `shortcuts.fixedSpacing`. The English help keeps
the `{{mod}}` placeholder for the navigation description. This feature author
did not edit `zh-CN.json`. No dependencies or FontForge code were introduced.

## Review round

The [independent editing review](p5f-w3-review-editing.md) found no blocking
spacing defect and independently passed 40 tests in four files before the two
extra navigation dispatch tests were added. No implementation fixes were
requested. The additional dispatch tests enable the real glyph-navigation
commands and prove both Mod-bracket directions select only navigation on both
platform mappings.

Recommendation: row 39 **Complete** for all four spacing increments; row 40
**Complete** with the added lock regression. Independent Chinese translation
is still a parent-owned integration gate at this checkpoint.

## Resources

Start: build 14.91 GiB / 18,131 files, temporary 0.00 GiB / 1 file, free
607.07 GiB; checker **PASS**, build-cache warning above 10 GiB.
The focused tests generated no persistent task artifacts. No server or browser
was started by this owner; the E2E owner manages its own browser resources.
The task scratch directory was removed before handoff. Final checker:

```text
build: 15.66 GiB, 19,214 files
temporary: 0.02 GiB, 574 files
workspace free: 606.29 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

Shared build and temporary totals include the other concurrent tasks; this
task did not create Cargo artifacts or retain scratch files.
