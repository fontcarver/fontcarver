# p5f-h: Preview, proofing and proof export

Area id: `h-proofing`. Ports: API 8808, UI 1508. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                    | State at start                                                                                                       |
| --- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| 73  | Preview strip + detachable preview window; instance picker; invert; flip; blur                 | `PreviewStrip`, `PreviewWindow`; instance picker, flip and blur to verify/implement.                                 |
| 74  | Waterfall (text and single-glyph) with configurable steps                                      | Fixed waterfall exists; configurable steps and single-glyph mode to verify/implement.                                |
| 75  | Feature toggles (tri-state), script/language override, axis sliders, cluster inspector         | `TextShapingControls`; see `docs/acceptance/p5-proof-shaping-controls.md`; tri-state, overrides, clusters to verify. |
| 76  | Sample-string library in the project, grouped, script-aware                                    | `textSamples.ts`, `ProofLibraryPanel`, `proofLibrary.ts`; grouped project library to verify.                         |
| 95  | Pairs-and-phrases proof library with Up/Down stepping during spacing                           | Library exists; stepping during spacing to verify/implement.                                                         |
| 109 | Proof sheet export to PDF/HTML                                                                 | `proofExport.ts`, `ProofExportControls`; see `docs/p5-proof-export-acceptance.md`; verify PDF/HTML independently.    |
| 111 | [CJK] Vertical writing mode in all proofing surfaces (task l owns the Edit View and VORG part) | Vertical Edit View exists; preview strip, waterfall, proof export in vertical mode to verify/implement.              |

Existing records: `docs/acceptance/p5-proofing-integration.md`,
`docs/acceptance/p5-proof-shaping-controls.md`, `docs/p5-proof-export-acceptance.md`,
`docs/superpowers/plans/tasks/p5-proof-*.md`, `p5-proofing-integration.md`,
`docs/superpowers/reports/p5-proof-library-rust.md`, `docs/p5-cjk-vertical-acceptance.md`, `docs/p1-acceptance.md`.

## Scope notes

- Row 73: the strip and the detached window share state (text, instance,
  features); instance picker lists static instances and masters; invert, flip
  (horizontal/vertical) and blur (radius) render in Canvas2D; settings persist
  with the workspace (use the workspace store API owned by task a; do not edit it).
- Row 74: steps as a list or range with step size; text and single-glyph
  modes; per-step size label; export of the waterfall is part of proof export.
- Row 75: tri-state feature toggles (default/on/off) with the shaper honouring
  each; script and language override; axis sliders for variable fonts; a
  cluster inspector listing glyphs, clusters, advances and positions.
- Row 76/95: project-stored, grouped, script-aware library (Latin, Cyrillic,
  Greek, CJK, Arabic samples at minimum; pairs and phrases); Up/Down steps to
  the previous/next entry while spacing in the text view; import/export as text.
- Row 109: HTML export self-contained (embedded WOFF2 of the current export and
  CSS); PDF export with real embedded glyph outlines (not screenshots) via the
  existing `proofExport.ts`; verify the PDF with an independent parser (Python
  `pypdf` if installed, otherwise `qpdf --check`/`pdffonts` if available, or a
  minimal structural check) and the HTML by loading it in the headless browser.
- Row 111: vertical mode for the strip, waterfall, proof export (HTML and PDF)
  using the vertical metrics and `vert` features; evidence with a CJK fixture.

## Owned files

`app/src/views/text/**` except the kerning/spacing files owned by task d
(`kerningEdits.ts,KerningGhostControls*,kerningGhosts.ts,kerningImpact.ts,KerningModeControl.tsx,kerningModel.ts,kerningModes.ts,kerningQuery.ts,KerningRenameDialog.tsx,kerningTableData.ts,KerningTable*,SpacingControls*,TextGapPicker*,textGaps.ts,drawTextGaps.ts,textMetrics.ts,TextKerningPanel*,useTextKerning.ts,useTextMetrics.ts`)
and the shaper files owned by task e (`shapeCoordinator.ts,useTextShape.ts`);
`crates/fc-api/src/handlers/variable_proof.rs`, `crates/fc-api/src/types/variable_proof.rs`;
`crates/fc-doc/src/document/**` proof-library parts only (narrow); `app/e2e/text.spec.ts` (proofing cases);
`docs/p5f-h-proofing-acceptance.md`; user docs for proofing.

Shared files (narrow patches only): `app/src/views/text/{TextView.tsx,TextBar.tsx,textLayout.ts}`
(task d also patches these; keep hunks small), `crates/fc-doc/src/fcf/schema.md`,
`app/src/shell/{builtinCommands,shortcuts}.ts`, `app/src/stores/uiStore.ts`,
`app/src/i18n/en.json`, `crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.
