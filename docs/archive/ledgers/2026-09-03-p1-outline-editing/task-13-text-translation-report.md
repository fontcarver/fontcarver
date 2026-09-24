# T13 text-controls translation report — 2026-09-07

Implemented on fresh branch `p1-zh-text` from P1 `28031a8`, in commit `b3a9e57 feat(i18n): zh-CN for text controls`.

Translated 27 English placeholder values in the new `textControls` namespace only. Coverage includes the controlled text field, preview display size, master selector, OpenType feature controls, horizontal and deferred vertical directions, sample choices, recompilation/error status, preview theme, and collapse controls. “Preview size” uses 预览字号 to mean display size, and “Master” uses the established product term 母版. Deferred vertical text and kerning samples say 稍后推出.

Intentionally retained technical or literal content:

- `px`, `LTR`, and `RTL` labels.
- `OpenType` within the translated feature label.
- Literal `/glyphname` syntax in the input example.
- The `{{size}}` interpolation placeholder and `px` unit in the preview label.

Validation:

- Recursive check: 877/877 locale leaf keys; all interpolation placeholders match; all 27 changed leaves are under `textControls`.
- `pnpm --dir app exec vitest run src/i18n/i18n.test.ts`: 2 tests passed.
- `pnpm --dir app check`: passed.
- Prettier for `zh-CN.json` and `git diff --check`: passed.

Generated API bindings were created from this worktree only to run TypeScript checks and remain ignored. This is a preliminary namespace top-up; any later T9b UI keys and independent language QA remain outstanding. No merge or push performed.

Independent QA correction `a5931f6 fix(i18n): align text control terminology` replaced the dispatch's incorrect 主版 with 母版 and polished the kerning sample to 字偶间距对. Key/placeholder parity, focused i18n tests, and Prettier pass.
