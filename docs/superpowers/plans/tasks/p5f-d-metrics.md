# p5f-d: Metrics, spacing and kerning

Area id: `d-metrics`. Ports: API 8804, UI 1504. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                         | State at start                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 37  | Metrics keys with the full syntax (`=n`, `=n±k`, `=\|n`, `=50`, `==n` local, `=n@y`, `=+k` on auto-aligned)         | Reference/offset/mirror/literal exist; local `==`, `@y` height and `=+k` on auto-aligned components to verify/implement. See `docs/p5-metrics-keys-acceptance.md`. |
| 38  | Out-of-sync indicators + Update Metrics (current / all masters)                                                     | Current-layer behaviour exists; all-masters update to verify.                                                                                                      |
| 39  | Spacing shortcuts with LSB / RSB / both, ×1 / ×10 / ×100 / true-1                                                   | LSB/RSB exist; both sides, true-1 and modifier grammar to verify.                                                                                                  |
| 40  | Kerning modes: off / on / locked (kern without risk of respacing)                                                   | `kerningModes.ts`, `KerningModeControl`; locked mode to verify.                                                                                                    |
| 41  | Kerning groups per side; class-to-class by default; modifier-held class suppression for exceptions                  | Groups exist; modifier-held exception creation to verify.                                                                                                          |
| 42  | Kerning colour coding in canvas (negative vs positive) and group-member ghosting                                    | `kerningGhosts.ts`, `KerningGhostControls`; verify colours and ghosts.                                                                                             |
| 43  | Kerning table: sortable, filterable, group rename with scope prompt, Clean Up, Compress, copy/paste between masters | `KerningTable`, `KerningRenameDialog`, `kerningTableData`; see `tasks/p5-kerning-maintenance*.md`.                                                                 |
| 44  | Spacing tab with editable metrics matrix, beam measurement, cut-off display, saved proof strings                    | `SpacingControls`; matrix, beam, cut-off and saved strings to verify/implement.                                                                                    |
| 45  | [CJK] Vertical kerning and vertical metrics editing                                                                 | Reusable evidence (P4, `docs/p5-cjk-vertical-acceptance.md`); re-verify.                                                                                           |

Existing records: `docs/p5-metrics-keys-acceptance.md`, `docs/p5-metrics-ui-acceptance.md`,
`docs/p5-cjk-vertical-acceptance.md`, `docs/p2a-acceptance.md`,
`docs/superpowers/plans/tasks/p5-kerning-maintenance*.md`, `p5-metrics-ui.md`,
`p5-metrics-afm*.md`.

## Scope notes

- Row 37: implement every listed form in the parser, resolver and the UI
  field with inline validation and a tooltip explaining the syntax; `==n` is
  local to the current layer; `=n@y` measures the sidebearing at height `y`;
  `=+k` applies on components auto-aligned via anchors; cyclic keys are
  detected and reported, never looped.
- Row 38: out-of-sync badge in the cell (coordinate with task b: it reads
  the status you expose through the API), the metrics panel and the Spacing
  tab; Update Metrics for the current master and for all masters, each one
  undo unit.
- Row 39: keyboard grammar for LSB, RSB and both sides at ×1/×10/×100 and a
  true-1 (ignores rounding/grid) step, documented in the shortcuts doc.
- Row 40: locked mode kerns without ever changing sidebearings even when the
  gesture would otherwise respace; the mode is visible in the text bar.
- Row 41: class-to-class default with a held modifier producing a glyph
  exception; exceptions are visually marked in canvas and table.
- Row 43: sortable/filterable table over pair, groups, value and master;
  rename with a prompt whether to rename in one master or all; Clean Up removes
  zero/redundant pairs; Compress folds pairs into class pairs where safe;
  copy/paste kerning between masters with a preview of the count.
- Row 44: the Spacing tab shows an editable matrix (LSB, RSB, advance, keys) for
  the glyphs in the text run, a draggable beam that measures at a height, a
  cut-off display (show only the spacing zone), and saved proof strings
  (project-stored, coordinate with the proof library file owned by task h:
  use the existing `proofLibrary.ts` store API without editing it).

## Owned files

`app/src/views/text/{kerningEdits.ts,KerningGhostControls*,kerningGhosts.ts,kerningImpact.ts,KerningModeControl.tsx,kerningModel.ts,kerningModes.ts,kerningQuery.ts,KerningRenameDialog.tsx,kerningTableData.ts,KerningTable*,SpacingControls*,TextGapPicker*,textGaps.ts,drawTextGaps.ts,textMetrics.ts,TextKerningPanel*,useTextKerning.ts,useTextMetrics.ts}` and tests;
`app/src/views/glyph/panels/{KerningGroup*,kerningGroup*,useKerningGroupQuery.ts,MetricsPanel.tsx,metricsKeyCommands.ts,useResolvedMetrics.ts,VerticalMetricsPanel.tsx}`;
`app/src/views/glyph/{drawMetrics.ts,metricsHandles.ts}`;
`crates/fc-doc/src/{metrics.rs,kerning/**,kerning.rs,command/metrics.rs,command/kerning.rs}`;
`crates/fc-model/src/kerning.rs`; `crates/fc-api/src/handlers/kerning.rs`, `crates/fc-api/src/types/kerning.rs`;
`app/e2e/text.spec.ts`, `app/e2e/p2a-kerning.spec.ts`; `docs/p5f-d-metrics-acceptance.md`;
`docs/keyboard-shortcuts.md` (spacing section only); user docs for spacing/kerning.

Shared files (narrow patches only): `app/src/views/text/{TextView.tsx,TextBar.tsx,textLayout.ts}`,
`crates/fc-model/src/{glyph.rs,master.rs}`, `crates/fc-doc/src/fcf/schema.md`,
`crates/fc-api/src/handlers/{glyphs.rs,glyph_status.rs}`, `app/src/shell/{builtinCommands,shortcuts}.ts`,
`app/src/i18n/en.json`, `crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.

## Gap analysis

Written 2026-09-30 after reading the code and the prior records, before
implementing. "Works" means I read the implementation and its tests; browser
evidence is collected separately and recorded in the acceptance report.

### Decisions taken without asking

- **File ownership overlaps.** Task h owns `app/src/views/text/**` except my
  listed files, so `textCanvas.ts`, `textCanvasBridge.ts`,
  `TextWorkspaceControls.tsx`, `useTextKerningControls.ts`, `useKerningTable.ts`,
  `useKerningGhosts.ts` and `useKerningScope.ts` are task h's even though they
  implement my rows. I keep every change to them additive and a few lines long,
  and I prefer designs that need no change at all (for example computing gap
  kerning inside `drawTextGaps.ts`, which I own, instead of widening
  `identifyTextGaps`' caller). New files for row 44 (`SpacingMatrix*`,
  `spacingMatrix.ts`, `spacingBeam.ts`) are created by me under my row-44
  mandate; nobody else can be mid-edit in a file that does not exist yet.
- `VerticalMetricsPanel.tsx` is claimed by both this task and task l (CJK). I
  treat it as task l's and only verify it.
- `crates/fc-api/src/api.rs` is in nobody's list but is the only place a new
  method can be registered. I add the two-line rows for my new method there and
  name it in the report.
- New backend surface for row 44 goes into **new** files
  `crates/fc-api/src/{handlers,types}/metrics.rs`, registered through the shared
  `mod.rs` files my task already allows.
- `crates/fc-doc/tests/{metrics,kerning*}.rs` are the tests of code I own, so I
  treat them as mine.
- Vertical metrics _formulas_ (`MetricsKeys.vertical_origin` /
  `vertical_width`) are persisted model fields that nothing resolves. Rather
  than leave a field that silently does nothing, the metrics commands now reject
  them with an explicit "not supported" error, and the limit is documented. Row
  37 only lists horizontal forms, so this is a limit, not a missing row item.

### Row 37 — metrics keys, full syntax

Works: the kernel grammar covers `=n`, `=n±k`, `=n*k`, `=n/k`, `=|n`, `=50`,
`==n` (layer-local, rejected in glyph scope), `=n@y`, `=+k`/`=-k` on
auto-aligned components, same-glyph mirrored bearings, cycle and depth
detection (`Error::MetricsKeyCycle`), and unkeyed-metric derivation.
`crates/fc-doc/tests/metrics.rs` has 12 cases covering all of it.

Missing: the **UI field** side of the row. The three key fields are plain text
rows with a one-token placeholder; there is no inline validation (an invalid
formula is only refused by the kernel, after a command round trip), no tooltip
explaining the syntax, and a bare `H` — which the requirement lists as
equivalent to `=H` — is rejected instead of normalised.

Plan: add a TypeScript validator mirroring the kernel grammar, a dedicated
`MetricsKeyField` with `aria-invalid`, an inline message, a full-syntax tooltip
and Escape-reverts, normalise a bare reference/number to `=…`, and never send a
command for text the validator rejects.

### Row 38 — out-of-sync indicators and Update Metrics

Works: `UpdateMetrics { glyph, master: Some|None }` resolves every requested
master layer before mutating any of them, is one undo unit, rejects conflicting
or out-of-range results without mutation, and is idempotent. The font-view cell
badge (`metrics_out_of_sync`, task b) and the panel's in-sync/out-of-sync/error
line plus both update buttons exist.

Missing: the two Glyphs commands (`Mod+Ctrl+M`, `Mod+Alt+Ctrl+M`) have no menu
or palette entry and no binding, so the workflow is panel-only; and the Spacing
tab has no out-of-sync indicator at all.

Plan: register `glyph.updateMetrics` / `glyph.updateMetricsAllMasters` commands
with those bindings, acting on the edited glyph or the font-view selection, and
show a per-row out-of-sync badge with an Update button in the new spacing
matrix.

### Row 39 — spacing shortcuts

Works: `[`/`]` change the selected side (LSB / RSB / both) by the configured
nudge step, Shift ×10, Mod ×100, Shift+Mod exactly one unit; legacy
Mod+Left/Right remain LSB/RSB; both-side edits publish one preview and one
absolute `SetMetrics` per frame and blank glyphs take the summed advance.
`SpacingControls` selects the side and is keyboard reachable.

Missing: the grammar is documented nowhere. `docs/keyboard-shortcuts.md` has no
spacing section.

Plan: write that section (my task owns it) and keep the bindings as they are;
the recommended `Alt+arrow` model from the research doc is deliberately not
adopted because Alt+arrow is already the kerning gesture.

### Row 40 — kerning modes off / on / locked

Works: the per-tab mode select is in the text bar with per-mode tooltips; off
forces `kern`/`vkrn` off without destroying stored feature choices and blocks
kerning mutation; locked disables the metrics keyboard path, the metrics and
vertical-metrics panels, and the metric-handle drag (`asideGestures`).

Missing: **the Info box width / LSB / RSB fields are not guarded**, so locked
mode can still be respaced from the inspector's Info rows — exactly the
accident the mode exists to prevent. `InfoGlyphRows.tsx` is task c's file.

Plan: patch that file with an attribute-level guard (six lines) and report it.

### Row 41 — groups per side, class-to-class default, held-modifier exceptions

Works: class-to-class is the default scope for a new pair, an explicit scope
choice survives query reloads, and `Ctrl`/`Alt` held with the `,`/`.` kerning
keys drop the first / second class so the edit lands on a glyph-scoped
exception with its own captured edit session.

Missing: the held-modifier path is only enabled in **locked** mode, because
bare `,`/`.` would otherwise be typed into the text run. So in the ordinary
"kerning on" mode there is no way to make an exception with a held modifier.
Exceptions are also not marked anywhere: neither the panel, the canvas nor the
table says that a record is an exception.

Plan: allow `Ctrl`/`Alt`-modified `,`/`.` in every mode (a modified key is never
typed — `useTextTyping` already ignores modified keys) and keep bare `,`/`.`
locked-mode only; report whether the winning rule is an exception from the
kernel and mark it in the panel, the canvas and the table, with an
exceptions-only table filter.

### Row 42 — canvas colour coding and group ghosts

Works: group-member ghosting (bounded to four members per side, exact name and
active master, abort on selection change, viewport culled) with a keyboard
checkbox and a signed textual legend; the **selected** gap handle is coloured by
the sign of its resolved stored value.

Missing: every other gap is drawn in one neutral colour, so the run gives no
"where is the kerning" overview, which is what the row asks for. There is no
non-colour equivalent for unselected pairs either.

Plan: colour every gap handle by the sign of the kerning actually applied in
the shaped run (shaped advance minus stored advance, so any source shows up)
and draw the signed number next to non-zero gaps, which is also the
colour-blind-safe redundancy.

### Row 43 — kerning table

Works: bounded generation-consistent pagination, sort by first side / second
side / value, a contains filter over both side names, per-record removal, group
rename with a scope statement, Clean Up (invalid records), Compress (redundant
exceptions), whole-table copy with a record count and paste into the current
master with merge-or-replace, generation preflight, undo and FCF persistence.
Twelve `fc-doc` maintenance tests pass.

Missing:

- Clean Up only removes _invalid_ records; the row asks for zero/redundant ones.
- Compress only _removes redundant exceptions_; it never **folds glyph pairs
  into class pairs**, which is what the row and the research text describe.
- Paste has no keep-existing conflict resolution.
- Copy/paste "between masters" only works by changing the whole text tab's
  master, and the table has no master column or master control.
- The rename dialog states one scope; the row asks for a scope _prompt_.
- No filter by side kind (group / glyph) or by exception, and no match mode.

Plan: add a `CleanupObsolete` operation (invalid plus redundant explicit zero),
extend `Compress` with a safe class-pair promotion pass, add `keep_existing` to
`Paste`, add a table-local master selector plus a master column so cross-master
copy/paste never disturbs the proof, add group/glyph/exception filters and match
modes, and give the rename dialog two real scopes: rename the group font-wide,
or retarget only this record.

### Row 44 — Spacing tab

Works: nothing of the row's four parts except the saved proof strings, which
`ProofLibraryPanel` (task h) already provides from the project-stored
`proofLibrary` API, and the existing side selector.

Missing: the editable metrics matrix, the beam and the cut-off display.

Plan: a new bounded `metrics.spacing` API returning, for the glyph names in the
run and one master, the stored advance/LSB/RSB, the effective formulas with
their scope, the resolved metrics, an out-of-sync flag and — when a beam height
is given — the left and right margins measured at that height. A new
`SpacingMatrix` panel edits advance, LSB, RSB and the formula per row, shows the
out-of-sync badge with an Update button, and drives the beam. The canvas draws
the beam line with its per-glyph measurements and a cut-off band that hides
everything outside the measured zone. Beam dragging: the beam height is dragged
with its own slider and typed in a field, because grabbing the line in the
canvas needs `GlyphCanvas.tsx` pointer code owned by task c — the exact patch is
named in the report.

### Row 45 — vertical kerning and vertical metrics

Works (reusable P4 evidence, re-verified here): `VerticalMetricsPanel` edits
TSB/BSB/origin/advance through `SetVerticalMetrics`; `kerning.resolve_vertical`,
`kerning.pairs_vertical` and `SetVerticalKerning` give vertical pair authoring;
the text tab's `ttb` direction switches the gap geometry, the table and the
shaping features to vertical.

Missing: only fresh end-to-end verification in this checkout, plus an
independent check that edited vertical kerning reaches an exported font.

Plan: re-verify in the headless browser and with FontTools/HarfBuzz, and record
the result rather than re-implementing.
