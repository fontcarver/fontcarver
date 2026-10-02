# P5f wording translation pass

Independent Simplified Chinese review, 2026-10-02, by the assigned GPT-6.1 Sol translator.

Read both wave-2 reports and reviewed all 43 changed English values against the existing Chinese and relevant UI context. Changed one value: `contextHelp.topics.workspace.third` now quotes the actual `workspace.manage` label, “工作区…”, and identifies it as a command. Removed the obsolete bare `cid.unmapped`; retained `cid.unmapped_one` and `cid.unmapped_other` unchanged.

The other 42 Chinese values already express the revised English accurately. Kept 字号 for waterfall sizes, 步长 for range increments, 多字号预览 for waterfall, and 最近邻 / 区域平均 for bitmap filters. Confirmed both filter names against the rendered option keys in `BitmapTransfer.tsx` and the workspace command against `shellCommands.ts`. English apostrophe changes require no Chinese wording changes. No English ambiguity found.

Verification (exit 0 for each):

- `TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5f-wording-translation/tmp pnpm --dir app exec vitest run src/i18n/i18n.test.ts src/i18n/spelling.test.ts`: 2 test files passed, 3 tests passed.
- Same `TMPDIR`, `pnpm --dir app exec prettier --check src/i18n/en.json src/i18n/zh-CN.json`: “All matched files use Prettier code style!”
- Read-only Python checks: 3,935 matching leaves; identical interpolation placeholders; English plural pairs retained; bare CID key absent; quoted workspace/filter names match labels.
- `python3 scripts/check_artifacts.py`: PASS before and after checks; build 14.91 GiB / 18,131 files, temporary 8 files, free 606.02 GiB. Existing build-cache warning remains; coordinator owns final cleanup.

This is a linguistic review of the 43-value delta, not a new whole-catalogue linguistic or native-platform review. Full `pnpm check`, `pnpm lint`, and `pnpm test` are delegated to the coordinator for integration.
