# T9b independent presentation controls report

Status: complete on `p1-text-controls` at `ae4e158`.

## Built

- `TextBar` is fully controlled. `TextBarProps` exposes `text/onTextChange`, `inputRef`, bounded `size/onSizeChange` (1–512 px), `masters/master/onMasterChange`, tuple `features/onFeaturesChange`, `direction/onDirectionChange`, and typed `onSample(TextSampleId)`. `TEXT_SAMPLE_TEXT` exports the Latin pangram and Chinese sentence; `all-encoded` remains a caller-resolved action. Kerning P2 and vertical P4 controls are visibly disabled.
- `PreviewStrip` accepts `PreviewGlyph[]` (`path`, `x`, `y`, `xAdvance`, `yAdvance`) in font units plus `unitsPerEm`, controlled `status`, dismissal, independent `theme`, and collapse callbacks. Three scrollable 12/24/48 px canvases use per-size UPEM scales, a negative-y canvas transform, and supplied `Path2D` data only. Backing width is capped at 4096 pixels.
- Responsive CSS Modules use repository tokens and independent system light/dark canvas colors. No store, transport, editor, shell, or shaping pipeline was added.

## i18n

Added the `textControls.*` namespace (bar/text/size/master/features/direction, sample labels, preview status/theme/collapse/size labels) to `en.json`, with identical English placeholders in `zh-CN.json` for the separate translation pass.

## Verification

- `pnpm --dir app gen:api`: passed (ignored bindings only)
- `pnpm --dir app check`: passed
- `pnpm --dir app lint`: passed
- `pnpm --dir app exec prettier --check .`: passed
- focused Vitest: 2 files, 6 tests passed
- full Vitest: 84 files, 724 tests passed

Browser captures are deferred to integrated T9b UI acceptance as dispatched.
