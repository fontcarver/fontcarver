# p5f-d acceptance: metrics, spacing and kerning

Task: `superpowers/plans/tasks/p5f-d-metrics.md`. Audit rows **37–45**. Ports
API 8804 / UI 1504. Date 2026-09-30. Protocol: `p5f-protocol.md`.

The gap analysis behind this report — what already worked, what was missing and
why each decision was taken — is appended to the task file.

## What was built

**Row 37.** The kernel grammar already covered every listed form; the field did
not. The three formula fields are now `MetricsKeyField`: a TypeScript validator
that mirrors `fc_doc::metrics::parse_key` refuses a malformed formula **before**
any command, naming the problem under the field with `aria-invalid` and keeping
the draft so it can be corrected; the full syntax is both a tooltip and a
visible, `aria-describedby`-linked paragraph; and a bare `H` or `50` is
normalised to the `=H` / `=50` the requirement says it means. The two grammars
are kept in step by one shared table asserted on both sides
(`VALID_METRICS_KEYS` / `INVALID_KEYS`).

**Row 38.** `Glyph → Update Metrics` (`Mod+Alt+M`) and `Update Metrics in All
Masters` (`Mod+Alt+Shift+M`) are registered commands acting on the edited glyph
or on the whole Font-tab selection as one undo entry. The Spacing matrix marks
every out-of-sync row and offers its own Update button. Glyphs binds these to
⌘⌃M/⌘⌥⌃M; our portable shortcut grammar has no separate Control (Control _is_
Mod off macOS), so the bindings differ and are remappable.

**Row 39.** The grammar was implemented but undocumented; it is now written up
in `docs/keyboard-shortcuts.md` § Spacing and kerning and in the user guide.

**Row 40.** Locked mode had a hole: the Info box's advance/LSB/RSB fields were
not guarded, so a kerning session could still respace a glyph from the
inspector. They are now disabled and their commit path is gated like every other
spacing control.

**Row 41.** Held-modifier exceptions only worked in the locked mode, because
bare `,`/`.` are typed into the run. A **modified** punctuation key is never
typed, so `Alt+,`/`Alt+.` (and `Ctrl`, `Ctrl+Alt` on macOS) now create an
exception in every kerning mode. `Ctrl+,` is no longer claimed off macOS, where
it is the platform Mod chord and previously fired both the kerning edit and the
user's own `Mod+,` binding. Exceptions are reported by the kernel
(`KerningResolution.exception`, `KerningPairRecord.exception`) and marked in the
panel, the canvas and the table.

**Row 42.** Only the selected boundary was coloured. Every boundary is now
coloured by the kerning the shaped run actually applied — the shaped advance
minus the glyph's stored advance, so any source shows up — and the signed value
is printed beside each non-zero boundary, which is the non-colour redundancy. An
exception gets a ring around its handle.

**Row 43.** `Clean Up` now removes obsolete records, not only invalid ones: an
explicit zero whose removal changes no resolved value goes, a zero that
suppresses a group value stays. `Compress` gained the missing half — it folds
pairs into a class pair wherever every member combination of the two groups
already agrees on one nonzero value, then removes redundant exceptions, so no
resolved value changes. Paste gained keep-existing conflict resolution
(`PasteKeepingExisting`) beside overwrite and replace. The table has its own
master control, so copy/paste between masters no longer moves the proof; filters
over name (contains / is exactly / begins with / ends with / does not contain)
and kind (all / group rules / glyph pairs / exceptions); a Rule column; and a
rename dialog with two real scopes — rename the group font-wide, or retarget
that one record at another group.

**Row 44.** New. `metrics.spacing` answers, for the glyph names of one run and
one master in a single bounded request, the stored advance/LSB/RSB, both scopes
of formulas, the resolved metrics, an out-of-sync flag, per-glyph errors and —
with a beam height — the margins measured on that line. The **Spacing matrix**
panel renders that as a spreadsheet whose LSB, RSB and Advance cells each take a
number _or_ a formula (a number typed over a formula replaces it, as one undo
entry), plus the beam and cut-off controls. The canvas draws the beam with the
real distance between each pair of outlines on it, and the cut-off band hides
everything outside the measured zone. Saved proof strings are the existing
project-stored proof library (task h), reachable in the same tab.

**Row 45.** Re-verified rather than rebuilt: vertical direction switches the
boundaries, the kerning panel and the table to vertical records, and edited
vertical kerning reaches `vkrn` in an exported font.

## Commands and results

Rust, serialised under the shared Cargo lock, with an absolute task `TMPDIR`:

```text
cargo test -p fc-doc --test metrics --test kerning --test kerning_commands \
  --test kerning_maintenance --test kerning_eligibility --test vertical
  kerning 9 · kerning_commands 18 · kerning_eligibility 15 ·
  kerning_maintenance 19 · metrics 15 · vertical 2   → 78 passed, 0 failed

cargo test -p fc-api --test metrics_spacing --test kerning --test kerning_workflow
  kerning 8 · kerning_workflow 6 · metrics_spacing 8 → 22 passed, 0 failed

cargo clippy -p fc-doc --all-targets -- -D warnings             → clean
cargo clippy -p fc-api --lib --bins -- -D warnings              → clean
rustfmt --edition 2021 <9 owned files>                          → clean
```

`cargo clippy -p fc-api --all-targets` additionally fails in
`crates/fc-api/tests/rescale_upm.rs`, an untracked test file belonging to another
task (`needless_update`); no owned file is implicated.

Frontend, from `app`:

```text
pnpm exec vitest run --maxWorkers=1 --minWorkers=1 src/views/text/ \
  src/views/glyph/panels/{MetricsPanel.test.tsx,metricsKeyCommands.test.ts,\
  useResolvedMetrics.test.tsx,KerningGroupsPanel.test.tsx,VerticalMetricsPanel.test.tsx} \
  src/views/glyph/InfoBox.test.tsx src/views/glyph/panels/metricsCommands.test.ts
  Test Files  53 passed (53)
  Tests       330 passed (330)

pnpm check                                    → 0 errors (whole app)
pnpm exec eslint <owned dirs and files>       → clean
pnpm exec prettier --check <owned files>      → all files use Prettier style
```

`src/views/glyph/panels/EditInspector.test.tsx` (task c's file, covering the
Metrics panel) passes in isolation: 28 passed in 6.7 s. In one combined run it
exceeded the five-second per-test limit under concurrent load; that run is not
recorded as a pass.

Browser, against the real kernel (`fc-devserver` + Vite on 8804/1504):

```text
FC_API_PORT=8804 FC_UI_PORT=1504 pnpm exec playwright test --config e2e/p2a.config.ts
  10 passed, 1 failed (1.6m)

FC_API_PORT=8804 FC_UI_PORT=1504 pnpm exec playwright test --config e2e/p2b.config.ts \
  e2e/p2b-imported.spec.ts
  4 passed (1.0m)
```

All five new cases pass:

- _edits metrics formulas with inline validation and synchronises every master_ —
  a malformed formula is refused in place with nothing applied; a bare `V`
  becomes `=V`; `=V+25` reports out of sync; the all-masters command brings both
  masters to the reference plus 25 and one undo restores them.
- _measures and edits the run through the spacing matrix, beam and cut-off_ — a
  typed number changes the advance, a typed formula replaces it, Update
  resynchronises, the beam columns and slider measure, cut-off engages.
- _marks kerning exceptions and maintains the table across masters_ — Exception
  and Class rules are labelled, the exceptions filter narrows the table, the
  table's own master control plus keep-existing paste moves kerning to the other
  master, and Compress preserves every resolved value.
- _locks every spacing control while kerning with spacing locked_ — the side
  selector and matrix cells are disabled and two `]` presses change nothing.
- _edits vertical kerning and vertical metrics in the vertical direction_.

Screenshots: [spacing matrix](screenshots/p5f-d-metrics/spacing-matrix.png),
[beam and cut-off on the canvas](screenshots/p5f-d-metrics/beam-cutoff.png),
[kerning table](screenshots/p5f-d-metrics/kerning-table.png).

The spacing case also drags the beam on the canvas itself, which is how task c's
`GlyphCanvas` pointer wiring and this task's three beam hooks are verified
together.

### Independent verification

Driving the running kernel over HTTP, then decoding the output with FontTools
and shaping it with HarfBuzz — never with FontCarver's own reader:

```text
spacing rows: A lsb 50 rsb 50 beam_left 121.43 …; a out_of_sync false, lsb = A.lsb + 40
export warnings: []
features: ['kern', 'vkrn']
upem=1000  AV kerned=1060  unkerned=1200        (glyph exception −140 applied)
                      av: unkerned − kerned = 55 (class pair applied to members)
AV vertical kerned=-2365 unkerned=-2400          (vertical pair −35 applied)
independent export verification passed
```

So a metrics formula plus `UpdateMetrics` reaches the advance width, a class
pair and its glyph exception both reach `kern`, and an edited vertical pair
reaches `vkrn`.

## The one failure that is not this task's

`p2a-kerning.spec.ts` › _saves a pending edit and keeps document handover
isolated_ opens four documents. Its page now shows **two pinned Overview tabs for
the same file** (`Pending.fcf`), because reopening a saved document restores a
text tab it already has; the duplicated views then leave the kerning panel
unreachable and the test times out. The tab lifecycle belongs to task a (row 1).
Three stale locators in this spec and its shared helper were repaired here so the
rest of the suite could run; they are listed under _Shared files_ in the report.

## Remaining limits

- **Control-held class suppression is macOS only.** Off macOS, Control _is_ the
  primary modifier, so `Ctrl+,` belongs to the user's own binding; the
  first-class and both-class scopes are chosen in the Kerning panel's Rule scope
  menu there. `Alt` works on every platform.
- **Compress bounds its promotion.** A candidate class pair whose two groups
  would need more than 4,096 member combinations is left exactly as it is rather
  than making the command unbounded on a CJK-sized group.
- **Vertical metrics formulas are not resolved.** `MetricsKeys.vertical_origin`
  and `vertical_width` are persisted version-1 fields that no resolver reads.
  Editing commands now refuse to _introduce or change_ either, while carrying an
  existing value through untouched, so nothing is silently inert and no older
  document is rejected. Vertical metrics are edited as explicit values.
- **The spacing matrix edits the glyph-wide formula scope.** A layer-local `==`
  formula is displayed with its scope prefix, and typing over it or emptying it
  clears _both_ scopes so a cell cannot be left inheriting the formula it just
  replaced; but a cell that creates a _new_ local override is not offered, since
  the inspector owns layer scope.
- **Cut-off is a canvas display mode**, realised by covering what lies outside
  the band. It is not part of any export or proof sheet.
- The kerning table shows one master at a time by design. Its master control
  makes cross-master work two clicks; there is no side-by-side master diff.

## Review fixes (second round)

An independent review returned _accept with fixes_. All seven items are fixed
with tests, and the beam limit above is withdrawn because task c landed the
canvas hooks it asked for.

1. **The current-master update could rewrite every master.** `UpdateMetrics`
   reads a null master as "every master layer", so the row button and
   `Mod+Alt+M` resynchronised the whole family whenever the view's master had not
   resolved yet. Both paths now return early, the row button is disabled and says
   why, and the command's `when` guard is false without a master. Covered by
   `metricsCommands.test.ts` and `SpacingMatrix.test.tsx`.
2. **A number typed over a layer-local formula left the glyph formula live**, so
   the row stayed out of sync. `clearEffectiveMetricsKey` now clears every scope
   that defines the metric, in the same batch as the value. Two new
   `SpacingMatrix.test.tsx` cases assert the exact batch for typing over such a
   cell and for emptying it.
3. **`=H-` was refused by the field but accepted by the kernel**, making a glyph
   named `H-` unreachable from it. `-` is no longer treated as a dangling
   operator, and `=H-` is in both shared grammar tables.
4. **Only the selected exception was ringed.** A new bounded `kerning.boundaries`
   method answers value and exception for every editable boundary of a run in one
   request at one generation; `useKerningExceptions` feeds the set to the canvas
   and `drawTextGaps` rings each one. Covered by two Rust cases in
   `metrics_spacing.rs`, `useKerningExceptions.test.tsx` and
   `drawTextGaps.test.ts`.
5. **A refused cell was title-only and dropped silently.** The message is now
   rendered in a `role="alert"` beside the cell, as the inspector's field does.
6. **Record-scope rename** is disabled for a record whose value cannot be read,
   with the reason shown, and the name field completes from the groups that
   already exist on that side; a name that is not one of them says it will be
   created empty.
7. **Dead keys removed**: `kerningTable.replace` and
   `panel.metrics.keyError.missingEquals`. `metricsKeyError` now normalises its
   input, so the bare `n` spelling validates as `=n` and that branch cannot exist.

**Beam dragging is no longer a limit.** Task c landed `beamPointer.ts` and the
`GlyphCanvas` pointer wiring; this task supplies the run's `onBeamGrab`,
`onBeamDrag` and `onBeamRelease`. During a drag the measured height is frozen at
where the beam was grabbed and the on-canvas distances are blanked, so a pointer
move is never a request and no number ever belongs to a height the beam has left.

**One regression found by task e and fixed.** Typing `0` into the kerning value
stopped recording a zero exception (`p2b-imported.spec.ts:149`). The panel opened
a pair at the _broadest_ scope its memberships allowed even when a narrower rule
was the one actually applying, so the zero landed on a class rule that the
existing glyph exception kept overriding. A pair now opens at the scope of the
rule that applies, and `useKerningScope` settles that answer once per pair, so a
held-modifier exception still does not move the panel's own scope afterwards.
Covered by a new `useTextKerning.classes.test.tsx` case; all four `p2b-imported`
cases pass.

## Recommended status

| Row | Recommendation                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 37  | **Complete** — full grammar, inline validation, syntax help, cycle reporting, browser evidence.                                                  |
| 38  | **Complete** — badge, panel, matrix, current/all-master commands with bindings, one undo unit each.                                              |
| 39  | **Complete** — LSB/RSB/both at ×1/×10/×100/true-1, now documented.                                                                               |
| 40  | **Complete** — locked mode covers every spacing entry point including the Info box.                                                              |
| 41  | **Complete** with the documented macOS-only Control chord.                                                                                       |
| 42  | **Complete** — signed colour plus printed value on every boundary, exception ring, group ghosts.                                                 |
| 43  | **Complete** — sortable, filterable, scoped rename, obsolete clean-up, class-folding Compress, cross-master copy/paste with conflict resolution. |
| 44  | **Complete** with the documented slider-dragged beam.                                                                                            |
| 45  | **Complete** for horizontal-tab vertical authoring and `vkrn` output; vertical _metrics_ tables stay with the CJK acceptance record.             |

No new dependency was added, and no FontForge source was read or adapted.
