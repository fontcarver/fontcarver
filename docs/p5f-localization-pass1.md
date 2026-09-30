# P5f localization pass 1 (Simplified Chinese)

Owner: localization engineer. Only `app/src/i18n/zh-CN.json` was edited;
`en.json` was read-only. A second, smaller pass will run after the remaining
agents finish adding English keys.

## Counts

- Added: 403 keys (English 3,520 leaves; zh-CN now 3,520).
- Changed (English meaning moved since HEAD, retranslated): 18.
- Removed (zh-CN keys no longer in English): 3 (`kerningTable.replace`,
  `crossDocument.select`, `crossDocument.selectGlyph`).
- The file was regenerated from `en.json`'s structure, so key order now follows
  English. HEAD's zh-CN order already differed, so the diff is noisy.

## Terminology decisions

- Existing terms reused exactly: 字形, 轮廓, 节点, 母版, 实例, 轴, 图层, 字偶距,
  侧边距 (not 边距), 点阵字号 (the file's term for strike), 工作区, 命名实例.
- Kerning "class" and "group" both map to 字组/分组 (per namespace); no 类 term.
- Scope in font info (Family, Font, Master, Export) is 层级, an inheritance
  level, not 作用范围 (rule scope) used elsewhere.
- Shape (path or component) is 形状; drag handle is 拖动柄, to avoid 控制柄.
- Beam is 测量线; cut-off band is 限定范围; Open Corner is 打开转角.
- Reference (read-only comparison font) is 参考字体; macro follows 脚本.

## Ambiguities settled from source

`kerningTable.classRule` and `kinds.groups` (filter and cell label),
`spacingMatrix.cutOff*` (a height band), `plugins.provenance` (source is a file
path), `cmd.view.previewCollapsed` (palette detail), `history.more` (appended
after a comma list).

## English oddities (not changed)

`exportProfiles.hint` is now unused (`projectHint` replaced it); `history.font`
and `history.current` look unreferenced; "Licence" and "Colour" vs "License" and
"Color"; point and node are mixed; `exportMatrix.profiles` still says
"Compiled-font profiles".

## Verification

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/i18n/i18n.test.ts
 ✓ src/i18n/i18n.test.ts (2 tests) 55ms
 Test Files  1 passed (1)
      Tests  2 passed (2)

pnpm --dir app exec prettier --check src/i18n/zh-CN.json
All matched files use Prettier code style!
```

`python3 scripts/check_artifacts.py`: PASS (build artifacts 15.02 GiB, warning
only). No Cargo runs, commits or subagents.
