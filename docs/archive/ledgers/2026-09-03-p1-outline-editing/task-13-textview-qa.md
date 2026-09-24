# T13 `textView` zh-CN QA — 2026-09-07

Independent QA of the four-value translation artifact.

## Required correction

- `textView.previewMenu`: change “文本预览条” to “文本预览栏”. “栏” is the natural UI term for the bottom preview strip; “条” reads like a literal physical strip.

## Approved values

- `metricsUndo`: “调整侧边距” matches the established LSB/RSB term and works as an undo label.
- `runCount`: “排版结果包含 {{count}} 个字形” is a natural screen-reader status and preserves the shaped-output meaning.
- `textStatus`: “{{count}} 个字符” correctly reports input character count.

Both `{{count}}` placeholders are preserved exactly. Apply after the `previewMenu` correction. This QA covers only these four artifact values; later T9 UI keys remain pending.
