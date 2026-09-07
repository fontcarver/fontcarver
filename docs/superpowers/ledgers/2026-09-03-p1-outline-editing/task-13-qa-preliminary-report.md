# T13 preliminary zh-CN language QA — 2026-09-07

Scope: independent review of the 400 translated values in `p1-zh-cn` at `b039884`, before the later T9b top-up. This is not final T13 acceptance.

## Required corrections

1. `cmd.reason.noStartPoint`, `cmd.reason.noBreak`, `infoBox.onCurve`: “on-curve” is translated as “轮廓节点”, which loses the operational distinction from off-curve control points. Use “曲线上节点”: “请选择一个曲线上节点”; “请选择一个曲线上节点以断开轮廓”; badge “曲线上”.
2. `dialog.expandStroke.joinBevel`, `joinMiter`, `miterLimit`: the join terminology is misleading/swapped. Use “斜角”, “斜接”, and “斜接限制”, respectively.

## Optional terminology/polish

- `warning.cffHintsNotImported`: use “字干” for font “stem” instead of “主干”. The existing P0 `fontInfo.field.stems` also says “主干”; change both together if updating the glossary.
- `infoBox.scaleX` / `scaleY`: “X/Y 轴缩放（%）” reads more naturally than “X/Y 缩放 %”.
- Unicode block labels would be more standard as “基本拉丁字母”, “拉丁字母扩展 A”, and “拉丁字母-1 补充”; apply consistently to both New Font and Add Glyphs dialogs.

## Verification

- 847/847 leaf keys; exactly 400 changed values. The bundles have a pre-existing
  ordering difference beginning at `fontInfo.noTables`; key-set parity passes.
- Interpolation placeholders match for every changed key; all `_one`/`_other` pairs are complete.
- No unintended untranslated sentence found; `app.name` (`Font Cutter`) is the sole unchanged English phrase.
- `pnpm --dir app exec vitest run src/i18n/i18n.test.ts`: 2 tests passed.
- Warning causes and consequences are otherwise preserved; existing “左侧边距” / “右侧边距” terminology remains consistent.

## Resolution

Applied all six required corrections and all optional polish in
`c3c3017 fix(i18n): refine zh-CN outline terminology`. The update standardizes
“曲线上节点”, bevel/miter terms, 字干, axis-scale labels, and Unicode block
names. Key parity (847/847), interpolation placeholder parity, the two focused
i18n tests, and the zh-CN Prettier check pass.
