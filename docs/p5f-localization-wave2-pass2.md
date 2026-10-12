# P5f localization wave 2, pass 2 (Simplified Chinese)

Only `app/src/i18n/zh-CN.json` was edited. It now has 3,936 leaves in `en.json` order, no missing or extra keys.

## Delta

- Added 7: `hints.ttAutohintInvalid`, `hints.removeFlex`, `hints.removeDotSection`, `construction.magnetHud_{one,other}`, `construction.magnetSnapHud_{one,other}` (the last four reuse the old bare-key text).
- Removed 2: `construction.magnetHud`, `construction.magnetSnapHud`.
- Changed: 0 needed. `proofPreview.sizesTruncated`, `sizesError.*`, `sizesHint`, `proof.library.stepping`, `cjkConstruction.*`, `bitmapGrid.inexact`, `hints.ttAutohintNumbersHelp` and `infoBox.originPoint.4` already matched the current English.

## Pass-1 review

All 415 keys new since HEAD plus the 19 retranslated ones were read against the English. Placeholders, plural pairs, `…` and full-width punctuation pass a scripted check; the only unconverted English is `Flex` and `CJK`. Quoted UI names in the help text match the command labels.

Corrected (awkward wording, meaning unchanged):

- `quickHelp.bitmap.tool.erase`
- `quickHelp.bitmap.tool.line`
- `quickHelp.tool.eraser`

## Terminology kept

Hint 提示, flex and dot section 点区段, blue zone 对齐区, magnet 磁铁, servant 从属节点, guide 参考线, radical 部首, quick help 快速帮助, strike 点阵字号, proof 校样, waterfall 多字号预览, group rule 字组规则, sheet 图集, history budget 内存预算 (confirmed in `fc-doc`). Existing advance wording is mixed; bitmap keys stay on 步进宽度.

## Verification

```text
 ✓ src/i18n/i18n.test.ts (2 tests) 55ms
 ✓ src/i18n/spelling.test.ts (1 test) 10ms
 Test Files  2 passed (2)
      Tests  3 passed (3)
All matched files use Prettier code style!
```

## Notes

- `python3 scripts/check_artifacts.py` now fails: build 21.47 GiB, was 9.14 GiB at start. I ran no builds; the concurrent Rust checks are the likely cause. Nothing deleted.

## English strings that look wrong (not changed)

- `proofPreview.sizesError.range` mixes "size" and "step".
- 24 new strings use a straight apostrophe, 5 a curly one.
- `contextHelp.topics.workspace.third` says "Manage Workspaces"; the command is "Workspaces…".
- `bitmapParity.filters.Nearest` reads "Nearest pixel" but `transferHint` says "Nearest".
- `cid.unmapped` has a bare key and a `_one`/`_other` pair; the call passes `count`.
