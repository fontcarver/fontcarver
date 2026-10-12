# T13 `textControls` zh-CN QA — 2026-09-07

Scope: independent language QA of the 30-leaf `textControls` namespace on `p1-zh-text` at `b3a9e57` (base `28031a8`). This covers only the namespace top-up; later T9 UI keys remain pending.

## Required correction

- `textControls.master`: change “主版” to “母版”. The rest of the product consistently uses “母版” for font masters, including the export dialog, inspector, layers, guides, and “显示其他母版”. “主版” would introduce a second term for the same object.

## Optional polish

- `textControls.sample.kerning`: “字偶间距组合” is understandable, but “字偶间距对——稍后推出” maps “Kerning pairs” more precisely. This choice is disabled and deferred, so it does not block the current UI.

## QA result

- “预览字号” preserves the display-size meaning: the control sets a 1–512 px true pixels-per-em preview size. `sizeInvalid` and `preview.size` retain the range, unit, and `{{size}}` interpolation.
- All 30 namespace leaves match the English key structure; 27 are translated. The intentionally unchanged `px`, `LTR`, and `RTL` values are appropriate.
- `/glyphname`, `OpenType`, direction meaning, deferred-state meaning, preview status/actions, and sample meanings are preserved. Chinese punctuation and wording are otherwise natural.
- Full locale parity remains 877/877 leaves, with no interpolation mismatch.
- `pnpm --dir app exec vitest run src/i18n/i18n.test.ts`: 2 tests passed.

Disposition: approve after the required `master` terminology correction.

Root scoped follow-up: `a5931f6` changes the required label to 母版 and adopts
the suggested kerning-pairs wording. The exact two-value diff matches QA;
translation branch approved and merged. Later T9 integration keys remain pending.
