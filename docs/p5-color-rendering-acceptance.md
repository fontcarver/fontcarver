# P5 color rendering integration acceptance

Implemented bounded shared color loading for inventory cells (including table
thumbnails), the main text canvas and the preview strip. COLRv0 uses validated
layer outlines with palette alpha and dynamic foreground; static COLRv1 uses the
existing bounded draft-font compiler and browser FontFace renderer. SVG stays in
isolated image mode, preserves selected-element ancestry and shared definitions,
and supplies palette CSS variables and foreground. Color-only glyphs render even
when their monochrome outline is empty. Document generation/master replacement
drops stale responses; released contexts remove font faces and blob URLs.

Imported static COLR/SVG now exposes validated temporary preview data without
changing editable ownership, raw tables, generation or undo history. COLRv1
preview accepts an explicit saved master. Backend regressions are documented in
[p5-color-render-api-acceptance.md](p5-color-render-api-acceptance.md); root's final
fc-api run remains authoritative because the delegated runs met unrelated compile
errors before executing tests.

## Verification

- Focused Vitest: shared cache/hook/painter and existing cell tests, **45 passed**;
  color-only cell regression, **1 passed**; after candidate filtering, cache/hook
  rerun **6 passed**.
- SVG serializer: **12 passed**; text/ghost/preview integration: **16 passed**.
- Scoped ESLint and Prettier checks passed. Root owns generated APIs, final full
  TypeScript/lint/test checks and Rust integration checks.
- Isolated agent-browser page invoked the real development API and the same
  inventory/text painters: COLRv0, native static COLRv1 gradient and SVG all
  loaded. Grid chromatic pixels were **3528 / 4394 / 9010**; text preview contained
  **12091** chromatic pixels including red, blue and green. Releasing the loaded
  resources reduced `document.fonts.size` from **1 to 0**.
- [Browser screenshot](screenshots/p5-color-rendering/cell-text.png). The complete
  application was not used for final visual acceptance because concurrent shared
  hot reloads repeatedly replaced its document. Hook and integration unit tests
  cover wiring; this screenshot covers the actual production painters and API.

## Boundaries

Palette zero is used in inventory/text views. Static COLRv0 and static COLRv1
render saved-master geometry; SVG artwork is master independent. Variable COLRv1
paint evaluation and interpolated axis-location color geometry are not implemented.
Browsers without COLRv1 support, rejected data and resource-cap overflow retain
available COLRv0/SVG/monochrome fallback. Proof exports remain monochrome.

The shared cache limits each document/master/generation to 128 color resources
and four concurrent loads; monochrome glyphs consume no resource slots. COLRv1
retains at most 32 MiB of live font bytes per loader, with the existing 8 MiB/font
backend cap. SVG source is limited to 2 MiB, image dimensions to 512 pixels, and
foreground variants to four per resource. SVG uses the established 1.3-em preview
frame; artwork beyond this frame is clipped. No new dependencies or UI strings.

Owned browser/session, development servers and scratch data are removed at task
completion; only this report and the final screenshot are retained. No commits,
shared status edits or native/global desktop automation were performed.
