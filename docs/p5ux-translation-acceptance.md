# Independent UX Chinese translation delta

2026-10-02. Assigned GPT-6.1 Sol translator.

Compared English with `29c97d2`: exactly four delta values, one new and three changed; no removals. Reviewed and translated:

- `proofPreview.settings`: “预览设置” for the bottom text-preview settings button, consistent with existing workspace help.
- `cmd.view.zoomIn`, `cmd.view.zoomOut`: “放大” / “缩小”, appropriate for the shared grid and canvas commands.
- `bitmap.editHint`: added Space-drag / middle-button pan and Escape cancellation instructions. Confirmed the gesture handlers in `BitmapCanvas.tsx`; existing baseline and clipping information is retained.

No other catalogue wording or UI was edited.

Verification after the final four-value delta, all exit 0:

- `TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5ux-translation/tmp pnpm --dir app exec vitest run src/i18n/i18n.test.ts src/i18n/spelling.test.ts`: 2 files passed, 3 tests passed.
- Same `TMPDIR`, `pnpm --dir app exec prettier --check src/i18n/en.json src/i18n/zh-CN.json`: “All matched files use Prettier code style!”
- Read-only Python checks: 3,944 matching leaves; identical interpolation placeholders; retained plural pairs; exactly the four expected English delta values since `29c97d2`.
- `python3 scripts/check_artifacts.py`: PASS at start, after checks, and after owned scratch cleanup. Build 5.33 GiB / 12,257 files. No owned test processes remain.

Independent language review covers these four values and their terminology, not a new whole-catalogue linguistic or native-platform review. Full UI/integration checks remain with the coordinator. No commit or push made.
