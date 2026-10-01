# P5f localization wave 2, pass 1 (Simplified Chinese)

Only `app/src/i18n/zh-CN.json` was edited (regenerated in `en.json` order; 3,931 leaves each, no missing or extra keys). A delta pass follows for keys the two feature agents add later.

## Counts

- Added: 410. Changed (English meaning moved since HEAD, retranslated): 18 of 34 moved; the other 16 are spelling or wording already matched. Removed: 3 (`exportProfiles.hint`, `history.current`, `history.font`).
- One consistency edit: `construction.magnetHelp` prefix 磁性关联 became 磁铁.

## Terminology (new concepts)

- Hint 提示, as the file already says (字形提示, 笔画提示). Flex 提示 and dot section 点区段 keep the PostScript operator names in Latin where the help text cites them.
- ttfautohint: 外部 TrueType 提示工具 (matches 自动提示); stem width 字干宽度; blue zone 对齐区.
- Magnet tool 磁铁; kept existing 从属节点 (servant) and 参考线 (guide), not 从动节点 or 引导线.
- Radical 部首, stroke count 画, tolerance 容差, scale filter 最近邻 / 区域平均.
- Quick help 快速帮助; waterfall 多字号预览 and proof string 校样文本 follow existing keys.
- Trace "ring" is 轮廓; skew is 错切 (the panel's shear term).

## Ambiguities settled from source

`imageWorkflow.droppedRings` (a closed contour), `hints.flexStale` (suffix after the flex number), `cid.unmapped` (names capped at six, so the zh form says 例如), `cmd.view.language.*` (hidden for the active language, so the English entry carries a Chinese gloss).

## Verification

```text
 ✓ src/i18n/i18n.test.ts (2 tests) 52ms
 ✓ src/i18n/spelling.test.ts (1 test) 9ms
 Test Files  2 passed (2)
      Tests  3 passed (3)
All matched files use Prettier code style!
```

## English strings that look wrong (not changed)

- `proofPreview.sizesHint` says "steps"; the other keys say "sizes".
- `bitmapParity.transferHint` says "Nearest", the option reads "Nearest pixel".
- `construction.magnetHud` and `magnetSnapHud` say "1 nodes" (no plural pair).
- `contextHelp.topics.workspace.third` says "Manage Workspaces"; the command reads "Workspaces…".
- Straight and curly apostrophes are mixed.
