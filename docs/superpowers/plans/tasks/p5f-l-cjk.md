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

## Gap analysis

Written before implementation, from reading the code at `5bd842e` and running the
existing focused suites (frontend 74/74 in eight CJK/vertical files;
`cargo test -p fc-compile --test cid --test otf_vertical` 11/11).

### Row 19 — vertical metrics in the inspector (TSB/BSB, origin, advance)

Works: `VerticalMetricsPanel.tsx` is mounted at the end of the glyph Metrics
section and shows four fields — vertical advance, vertical origin, TSB, BSB —
plus the hint and the **Use master vertical metrics** reset. TSB is
`origin − bbox.y1` and BSB is `advance − origin + bbox.y0`; `bbox` comes from
`glyphs.outline`, which flattens components (`fc_geom::flatten_layer_by_id`), so
the bearings really are measured on resolved ink. Editing TSB writes the origin
only and editing BSB writes the advance only. Every write goes through the
undoable `SetVerticalMetrics` command, honours `spacingLockedForDocument`, and
bumps a reconciliation revision so a rejected draft is reset. The kernel
validates the OpenType ranges in `fc-doc/src/command/metrics.rs::set_vertical`
and the generic `editor_metadata::preflight` refuses a locked glyph through
`apply::impact`.

Missing: (a) no test proves the lock and read-only refusals or the
range rejection for _this_ command, which is exactly the class of hole wave 1
found elsewhere; (b) no `.fcf` save/reopen evidence for an edited TSB/BSB pair;
(c) no independent `vmtx`/`VORG` check of an inspector-edited origin and advance.

Plan: add fc-doc integration cases (locked glyph, read-only document,
out-of-range value, save/reopen) and an acceptance script that exports an edited
font and decodes `vmtx`/`VORG`/`vhea` with FontTools, cross-checked with
FreeType and HarfBuzz. No UI change is needed; the panel already does what the
row asks.

### Row 110 — CID mapping, CID-ordered sort, CID-keyed export

Works: `Font Info → CID mapping` (`CidEditor.tsx`, `cidSession.ts`) stages
registry/ordering/supplement and `CID<TAB>name` lines, offers Adobe-Identity
auto-fill, validates uniqueness/CID 0/limits on both sides
(`cidSession.parseCid`, `fc-doc/src/command/cid.rs`), applies through a
revision-checked `cid.apply` that batches `SetCidInfo` with an optional
`SetGlyphOrder`, and joins the document edit-participant handover.
`GlyphSort::Cid` is implemented in `handlers/glyphs.rs` from the document's
`cid_map` and is offered by `GridToolbar` as **CID**, so the grid sort hook task
b extended already exists and needs no new patch. The CFF1 writer
(`fc-sfnt/src/write/cff.rs`) emits a CID-keyed font with ROS, a format-0
charset of CIDs, FDSelect format 0 and an FDArray built from
`master.private_dict` plus `master.cff_private_dicts`, selected per layer by
`cff_private_dict_index`.

Missing: no test or independent decode covers **more than one FontDICT**. The
existing `fc-compile/tests/cid.rs` has a single FD, so FDSelect is all zeroes
there and the FDArray index has one entry; the row explicitly asks for
FontTools verification on a fixture with two FDs. The editor also gives no
warning when exported glyphs have no CID, which turns into an export-time
failure.

Plan: add a two-FD CID export test (Rust) and decode its FDArray/FDSelect,
charset and ROS with FontTools in the acceptance run; add a live count of
unmapped glyphs to the CID dialog so the export failure is visible while
editing.

### Row 111 (Edit View and `VORG` part)

Works: `drawMetrics.ts` already draws the vertical origin and the vertical
advance (at `origin − advance`) as dashed horizontal lines with labels whenever
the layer or the master supplies them, and `renderGlyph` snaps nodes to the
vertical origin. `rebuild_vertical_metrics` in `fc-compile/src/otf_vertical.rs`
writes `vhea`, `vmtx` and `VORG` for CFF output, rounds like fontc, recovers
fontc's effective origin for unmodelled layers, keeps empty glyphs out of the
`vhea` extrema and — the row's requirement — filters the `VORG` records to the
origins that differ from the default, choosing the most common origin as the
default when the master declares none.

Missing: **the vertical origin and vertical advance are not editable on the
canvas at all.** `metricsHandles.ts` only knows `lsb`, `rsb` and `width`, all
hit-tested on x, so there is no vertical-mode gesture in the Edit View. There is
also no Edit-View "vertical writing mode": the vertical lines are drawn
unconditionally and faintly, with no way to say "I am spacing this font
vertically". And no test asserts that `VORG` _omits_ default-valued records.

Plan: add a `verticalMode` overlay (View menu + palette, like the other overlay
toggles) that emphasises the vertical origin/advance lines and the vertical
advance box; add `vorigin`/`vadvance` metric handles to `metricsHandles.ts`
hit-tested on y, dragged through the existing `MetricsDrag`
`edit_begin`/`edit_apply`/`edit_end` protocol so a drag is one undo entry,
previewing in the canvas, with an `ns-resize` cursor, a HUD readout, Escape
cancellation and clamping to the kernel's ranges. A drag writes only the field
it owns: the other field is sent as the layer's **stored** override (possibly
`null`), so dragging the origin never freezes an inherited advance. Add the
missing `VORG` default-record test and browser evidence.

Files outside my list that this needs (all additive, none owned by another
wave-2 task; named again in the final report): `app/src/editor/asideGestures.ts`
(route the font _point_, pick the cursor and the undo label per handle, pass the
stored overrides), `app/src/editor/editorFrame.ts` (widen the previewed
`aside.metrics`), `app/src/editor/editorTypes.ts` (two more HUD handles, the
stored vertical overrides on `EditorSource`, one more undo label),
`app/src/views/glyph/useEditorLabels.ts` (translate them),
`app/src/views/glyph/GlyphView.tsx` (publish the stored overrides),
`app/src/views/glyph/renderGlyph.ts` and `app/src/views/glyph/glyphCommands.ts`
(the overlay key and its command).

### Row 112 — radical library, IDS input, per-component transform grid

Works: the inspector's collapsed **CJK construction** section parses the twelve
original IDS operators with nesting, `[glyph.name]` leaves and bounded limits
(`cjkConstruction.ts`), resolves each leaf against the document without
mutating it and rejects missing/ambiguous/self/empty/zero-width sources
(`cjkResolve.ts`), shows an SVG preview, invalidates a stale preview on any
document generation change, and appends the components as one `Batch` of
`AddComponent` — one undo unit. `CjkTransformGrid.tsx` edits all six affine
coefficients of every component in the layer.

Missing: (a) the radical library is **not** built from the Unihan data shipped
in `fc-glyphdata`. `radicalCodepoints()` hard-codes the Kangxi Radicals block
plus a hand-typed string of variants and the panel then filters the font by
those codepoints, so the library has no radical numbers, no stroke counts, no
member counts and no relation to `kRSUnicode`; nor does it offer the glyphs the
font already uses as components. (b) The transform grid exposes raw affine
coefficients with no preview and commits each field separately, where the row
asks for position and scale **with live preview, applied in one undo unit**.
(c) There is no browser evidence and no user documentation for any of it.

Decision recorded here: the pinned Unicode 17.0 Unihan subset shipped in
`crates/fc-glyphdata/data` carries `kRSUnicode`/`kTotalStrokes` but **no IDS
decomposition property**, and no IDS database may be added without a new
licensed data file, which is out of scope for this task. The radical library is
therefore derived from the shipped data in the one way the data supports: for
radical _n_, every unified ideograph whose `kRSUnicode` has residual 0 for _n_
writes that radical, and the lowest such scalar is its canonical ideograph. I
verified against Python's Unicode database that this reproduces the NFKC form of
all 214 Kangxi Radicals block characters with zero mismatches, so the derivation
is exact rather than approximate. Per-character IDS _decomposition_ stays user
input — typed, or assembled by picking radicals — and is recorded as the row's
one explicit limit.

Plan: a new `cjk.radicals` kernel method returning the 214 radicals (number,
Kangxi block character, canonical ideograph, variant ideographs, stroke count,
the document glyphs that encode any of them, and how many document glyphs use
the radical) together with the glyphs the document already uses as components,
most used first; rebuild `CjkRadicalLibrary` on it with a radicals tab and a
components tab; rebuild `CjkTransformGrid` as a staged position/scale grid with
a live preview and one `Batch` apply; add the user documentation; verify in the
browser against a real CJK face.
