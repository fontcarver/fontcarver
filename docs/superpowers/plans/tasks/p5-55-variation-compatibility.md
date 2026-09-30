# P5-55 source compatibility inspection

Scope: replace the variation preview's undifferentiated source overlay with a
source legend, independent visibility controls, bounded point correspondence
inspection, compatibility guidance and the existing repair action.

Owned files: new `VariationCompatibility.tsx`, its CSS module, and
`variationCompatibility.ts` plus focused tests under `app/src/views/fontinfo/`.
The live-axes owner integrates the component into `VariationPreview.tsx` and
owns English keys; the independent localization owner translates them.

Public interface: `VariationCompatibility({ preview, busy, glyph, onRepair })`.
It owns the SVG preview, source controls, diagnostics and repair button. Preview
types remain generated from the existing API; repair delegates to the parent.

Acceptance: parser tests reject unsupported/malformed/oversize paths, recognize
contour and on/off-curve point topology, and preserve SVG point ordering.
Component tests independently hide sources, reveal matching correspondence,
explain mismatches without invented links, and disable repair while busy.
Parent runs full frontend checks and integrated browser verification. This
subtask does not claim complete variable-font authoring or new repair geometry.

Scratch: `.scratch/p5-variation-compatibility/`; focused test TMPDIR uses this
absolute path. No dependency additions, Rust edits, generated types, commits,
or shared status changes.

## Acceptance report — 2026-09-30

Built a 260-line compatibility component with independent source visibility,
theme-aware source colors, contour/point totals, numbered on/off-curve points,
matching-topology correspondence links, source-limit notice, API diagnostics,
manual-repair guidance, and the existing repair callback. Unsupported or large
paths retain their outline; parser work is capped at 100,000 characters / 2,048
points per source and marker rendering at 750 selected points.

Tests were written first; both suites initially failed on the absent modules.
Final focused command:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-variation-compatibility pnpm --dir app exec vitest run src/views/fontinfo/VariationCompatibility.test.tsx src/views/fontinfo/variationCompatibility.test.ts
Test Files  2 passed (2)
     Tests  16 passed (16)
  Duration  26.37s
```

Scoped ESLint for all four TypeScript files exited 0 with no output. Prettier
formatted only owned files. The live-axes owner integrated the component and
owns final TypeScript/full-suite/browser acceptance. Review flagged inherited
stale-preview identity and `fixedGlyph` initialization risks to that owner.
No new backend repair semantics or complete variable-font parity is claimed.

Artifact checker passed; task scratch was empty and removed. No owned server or
test process remains. Shared changes from other agents were left untouched.
