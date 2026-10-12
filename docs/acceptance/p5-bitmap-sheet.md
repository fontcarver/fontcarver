# PNG sprite-sheet implementation acceptance

Implemented on 2026-09-30. The component accepts `doc` and `BitmapStrikeInfo`;
parent integrates it into the bitmap view and supplies generated `SetGlyphs`.

The panel exports browser PNGs and imports an explicit row-major glyph mapping
into one atomic command. Bottom-left fixed cells preserve existing bitmap bounds,
origins, horizontal/vertical advances and vertical origins. Resolved composites
become independent pixel images on import. New images receive cell-sized bounds.
Other glyphs remain outside the mutation. PNG alpha is coverage below 32 bpp.
Browser canvas processing may normalize RGB under transparency.

Validation rejects duplicate/excessive names, oversized PNG bytes or advertised
dimensions before decoding, clipping, painted padding/unassigned cells,
dimension mismatches and generation changes. Decoded images and canvas storage
are released on both success and failure; replaced export URLs are revoked.

Verification:

```text
pnpm --dir app test src/views/bitmap/bitmapSheet.test.ts src/views/bitmap/BitmapSheet.test.tsx src/canvas/bitmapSheetImage.test.ts
Test Files  3 passed (3)
     Tests  20 passed (20)
  Duration  48.08s

pnpm --dir app exec eslint src/views/bitmap/bitmapSheet.ts src/views/bitmap/BitmapSheet.tsx src/views/bitmap/BitmapSheet.test.tsx src/views/bitmap/bitmapSheet.test.ts src/canvas/bitmapSheetImage.ts src/canvas/bitmapSheetImage.test.ts
Exit 0; no diagnostics.

pnpm --dir app check
Exit 2. Shared generated API types are stale across the resumed feature work,
including the parent-owned BitmapCommand::SetGlyphs variant. Regeneration and
the final integrated TypeScript check remain with the parent task.
```

Tests cover independent cell coordinates, all packed coverage depths, RGBA
channels, empty glyphs, prototype-like names, metric preservation, unsafe
allocation bounds, PNG decoder cleanup, atomic command submission, and stale
generation rejection. The initial missing-module test failed before implementation.

Integrated browser acceptance and full frontend regression belong to the parent
bitmap task. No task-owned browser or devserver was started. Independent review
identified the generation and trailing-cell checks included above. No FontForge
source or new dependency was used.

Final resource check: build 6.10 GiB / 13,171 files; temporary 0.14 GiB /
4,881 files; workspace free 668.70 GiB. All budgets passed. The empty owned
`.scratch/p5-bitmap-sheet` directory was removed after verification.
