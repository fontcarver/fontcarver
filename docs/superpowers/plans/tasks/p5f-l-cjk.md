# p5f-l: CJK completion

Area id: `l-cjk`. Ports: API 8812, UI 1512. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                              | State at start                                                                                                                                                                                |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 19  | [CJK] Vertical metrics in the inspector (TSB/BSB, vertical origin, vertical advance)                                                     | Vertical advance/origin editing exists (P4); explicit TSB/BSB fields to verify/implement in `VerticalMetricsPanel` (coordinate with task d, which owned that file in wave 1; you own it now). |
| 110 | [CJK] CID mapping, CID-ordered sort, and CID-keyed export                                                                                | Reusable evidence (P4 + FontTools-decoded CID-keyed CFF); re-verify with the current code.                                                                                                    |
| 111 | [CJK] Vertical writing mode in Edit View and in all proofing surfaces, with `VORG` and vertical origin editing (Edit View and VORG part) | Vertical Edit View and VORG export exist (P4); verify editing of the vertical origin on canvas and `VORG` output.                                                                             |
| 112 | [CJK] Component-based CJK construction helpers: radical library, ideographic description sequence input, per-component transform grid    | `CjkConstructionPanel`, `CjkRadicalLibrary`, `CjkTransformGrid`, `cjkResolve.ts`; see `docs/p5-cjk-construction-acceptance.md`; verify end to end.                                            |

Existing records: `docs/p4-acceptance.md`, `docs/p5-cjk-construction-acceptance.md`,
`docs/p5-cjk-vertical-acceptance.md`, `docs/user/variable-and-cjk.md`,
`scripts/extract_unihan.py`, `crates/fc-glyphdata` (Unihan data).

## Scope notes

- Row 19: TSB/BSB derived from vertical origin and advance with the glyph
  bounds, editable (editing TSB moves the origin, editing BSB changes the
  advance), shown in the inspector with the vertical origin and advance; undo
  and persistence; export to `vmtx`/`VORG` verified with FontTools.
- Row 110: CID mapping editor (assign/reorder), CID-ordered sort in the grid
  (coordinate with task b's sort API; if a hook is missing, add it with a
  narrow patch), CID-keyed CFF export with FDArray/FDSelect verified by
  FontTools on a fixture with two FDs.
- Row 111: vertical origin handle on the canvas in vertical mode, `VORG`
  written only when it differs from the default, vertical advance handle;
  browser evidence.
- Row 112: radical library populated from the Unihan/IDS data shipped with
  `fc-glyphdata` and from the current font's components; IDS input
  (⿰⿱⿲⿳⿴⿵⿶⿷⿸⿹⿺⿻ operators) that resolves to a component layout proposal;
  per-component transform grid (position, scale) with live preview, applied as
  ordinary components in one undo unit; verified in the browser with a CJK
  fixture; export decomposes or keeps components per option.

## Owned files

`app/src/views/font/{CjkBatchDialog.tsx,CjkFilters.tsx,cjk.module.css}`;
`app/src/views/fontinfo/{CidEditor.tsx,cidSession.ts}`;
`app/src/views/glyph/panels/{Cjk*,cjk*,UnihanPanel.tsx,VerticalMetricsPanel.tsx}`;
`crates/fc-glyphdata/src/**` Unihan/IDS parts (task b owned the crate in wave 1; you own the CJK data parts now);
`crates/fc-api/src/handlers/{cjk.rs,cjk_batch.rs,cid.rs}`, `crates/fc-api/src/types/{cjk.rs,cjk_batch.rs}`;
`crates/fc-doc/src/command/cid.rs`; `scripts/extract_unihan.py`; `app/e2e/cjk.spec.ts`;
`docs/user/variable-and-cjk.md` (CJK sections); `docs/p5f-l-cjk-acceptance.md`.

Shared files (narrow patches only): `crates/fc-sfnt/src/write/**` (vmtx/VORG/CID
output), `crates/fc-model/src/{glyph.rs,font.rs}`, `crates/fc-doc/src/fcf/schema.md`,
`app/src/views/glyph/{GlyphCanvas.tsx,drawMetrics.ts,metricsHandles.ts}` (vertical
handles), `app/src/views/font/{glyphOrder.ts,GridToolbar.tsx}` (CID sort hook),
`app/src/shell/builtinCommands.ts`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.
