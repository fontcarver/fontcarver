# T13 preliminary zh-CN translation report — 2026-09-07

Implemented on `p1-zh-cn` at `b039884 feat(i18n): zh-CN for P1 editor and dialogs`, based on T8 head `112ab38`.

Translated 400 English placeholder values added by T6/T7/T8/T9a/T11. Coverage includes editor commands and undo labels, tools, snapping, info box, glyph panels, transform controls, New Font/Add Glyphs/Export/Preferences/Recovery/path dialogs, progress and recovery notices, API errors, preservation warnings, and accessibility labels. Existing P0 terminology was retained: 字形, 节点, 控制柄, 轮廓, 字宽, 左/右侧边距, 参考线, 锚点, 组件, 图层, 母版, 度量, 字族.

Intentionally unchanged English/technical content:

- Product and language names: `Font Cutter`, `English`.
- Format/standard names: TTF, WOFF/WOFF2, UFO, FCF, Designspace, GB 2312, Unicode, PostScript, Macintosh, OS/2, PANOSE.
- Field identifiers and controls: `defaultWidthX`, `nominalWidthX`, Shift, Command, X/Y/W/H, delta/HUD symbols, `px/em`.
- Literal path examples, coordinate-only values, and pass-through interpolation-only messages such as `{{message}}`.

Validation:

- Recursive comparison: 847/847 leaf keys in identical order; interpolation placeholders matched for every key.
- `pnpm --dir app exec vitest run src/i18n/i18n.test.ts`: 2 passed.
- `pnpm --dir app check`, `pnpm --dir app lint`, and Prettier check: passed.
- `pnpm --dir app test`: 92 files, 856 tests passed.

Generated API bindings were created only to run checks and remain ignored. T9b UI strings require a later top-up after integration. Independent second-agent language QA remains required before final T13 acceptance.
