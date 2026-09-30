# P5f localization pass 2 (Simplified Chinese)

Only `app/src/i18n/zh-CN.json` was edited; it is regenerated in `en.json`
order (3,524 leaves each, no missing or extra keys).

## Delta

- Added: 4 (`fontInfo.scoped.derived`, `variation.scrubAxis`,
  `variation.resetAxis`, `variation.chooseConfig`).
- Changed: 1 (`fontSettings.upmHint`, English corrected after pass 1). All 18
  keys whose English moved since HEAD already had current zh-CN text.
- Removed: 0.

## Review of pass 1 (12 corrections)

- `scopedInfo.exportOverridesHint`: "classes" are weight and width classes
  (derived from the instance location), not glyph classes; fixed.
- `features.recipes.positional_figures`, `features.generator.PositionalFigures`:
  定位数字 replaced by 上下标与分子分母数字 (sups/sinf/numr/dnom).
- `kerningTable.renameScopes.record`, `undoRetarget`, `retargetUnavailable`:
  重新指定 became 重新指向 (clearer for "retarget").
- `kerningText.isException`: 分组规则 became 字组规则, matching the table.
- `history.layerRole.smart`: 智能图层, matching its sibling roles.
- `cmd.reason.noOpenCorner`: 已打开转角, matching the 打开转角 command.
- `toast.glyphInfoUpdated_*`: dropped the repeated 字形信息 wording.
- `crossDocument.mergeReview`: 历史记录条目, matching `history.list`.

Placeholders, plural pairs, punctuation and untranslated Latin words are clean.

## Terminology

- Class rule and group rule are both 字组规则 (one concept; the English still
  says both). HEAD mixes 字组 and 分组 for kerning groups; left as is.
- Beam 测量线 and cut-off 限定范围 kept (docs: a band hiding everything outside it).

## Verification

```text
 ✓ src/i18n/i18n.test.ts (2 tests) 228ms
 Test Files  1 passed (1)
      Tests  2 passed (2)
All matched files use Prettier code style!
```

## English strings that look wrong (not changed)

- `fontSettings.upmHint` says "resolved"; the kernel says "bake or remove".
- `exportMatrix.loading`, `changed`, `noInstances`, `error.invalid-instance`
  and `profiles` still describe only named instances and "compiled-font".
- `exportProfiles.hint` and `history.font` are unused (`history.current` is used).
- "Licence"/"Colour" versus "License"/"Color".
- `kerningTable.classRule` and `compressHint` say "class"; `kinds.groups` says "group".
- `editor.audit.detail.coincident` says "nodes", `coincidentManual` "points".
- `history.layerRole.smart` "Smart layer" versus `glyphInspector.layerKind.smart`
  "Smart source".

`check_artifacts.py` now FAILs (25 GiB build cache) from another task's
`cargo test --workspace`; I ran no Cargo.
