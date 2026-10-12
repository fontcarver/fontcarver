# p5f-k acceptance: hinting completion — 2026-10-01

Area `k-hinting`, audit rows 61 and 98 plus the P5 TrueType scope. Task file:
[`p5f-k-hinting.md`](superpowers/plans/tasks/p5f-k-hinting.md), whose
`## Gap analysis` section records the decisions this report refers to. Protocol:
[`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md). Earlier records:
[hinting checkpoint](p5-hinting-checkpoint.md),
[resumed checkpoint](p5-hinting-resume-checkpoint.md),
[TT variable binary checkpoint](p5-tt-variable-binary-checkpoint.md).

`ttfautohint` is **not installed on this host** (`which ttfautohint` is empty).
Every claim below about the external tool is either about the absent path, which
is exercised directly, or about code paths tested with a controlled fake
executable; the one test that needs the real binary skips and says so.

## What changed

### Row 61, TrueType: the external bridge

`fc_hint::external` gained a validated `AutohintOptions` record covering the
hinting range, hinting limit, fallback stem width, x-height increase and
snapping exceptions, default and fallback scripts, strong stem widths, the
subglyph, composite, dehint, symbol, `TTFA` and Windows-compatibility switches,
and how much provenance the tool records in the `name` table. Nothing reaches a
process before `validate()` accepts it, so an out-of-range ppem or a script that
is not four lowercase letters fails the export request instead of becoming a
command-line switch. `availability()` reports the executable's path and its
`--version` line, and `autohint()` now returns an outcome carrying the version
and every message the tool wrote **while succeeding** — those become
`autohint-message` export warnings, where before a successful run's warnings were
discarded. The absent tool still produces `autohint-unavailable` and an unhinted
font; it is never reported as success.

A new API method `hints.tt_autohinter` answers whether the tool is installed, and
the export dialog shows an **External TrueType hinter** section with that answer
and the options whenever automatic hinting is enabled for a TTF, WOFF or WOFF2.

### Row 61, PostScript: analysis quality

`fc_hint::ps` was rebuilt around a result that carries four kinds of hint rather
than one compatible stem set:

- **Ghost hints.** An alignment edge that no stem constrains — inside a zone,
  long enough to matter, with ink on exactly one side — becomes a `-20` hint at
  a top edge or a `-21` hint whose stored start is a bottom edge plus 21. That
  convention is the one real fonts use and the one this repository's Nimbus Sans
  regression already asserted.
- **Hint replacement.** A stem that conflicts with the primary set is kept
  instead of dropped, provided it is at least as long as it is wide, and bound by
  point masks to the outline points it governs. Masks are emitted at the first
  on-curve point of the first contour and wherever the active set changes, which
  is exactly the `hintmask` / `OtherSubrs 3` semantics. A layer built from
  components has no point identity of its own, so it receives stems only.
- **Counter groups.** An axis whose whole primary set is one valid triple also
  receives a counter mask.
- **Per-dictionary private values.** Analysis no longer refuses a CID glyph
  assigned to a dictionary other than FD0: it groups the selection by dictionary
  and infers each one's standard widths and stem snaps from the glyphs assigned
  to it. `hints.analyze` returns them as `cid_private`, and the editor applies
  them.

### Row 98: flex, dot sections, general counter control

`PsHints` gained `flex_hints` (a contour, the stored index of the on-curve point
two consecutive curve segments leave, and a flex height) and `dot_sections` (the
indices of bracketed closed contours). The legacy boolean `flex` keeps its old
meaning — an imported flex whose curve pair could not be identified — so no
existing `.fcf` changes meaning, and export still refuses it rather than writing
ordinary curves.

- **CFF** writes `flex1` at the implied height of 50 where the endpoint returns
  to the starting x or y, and `flex` with its explicit height otherwise.
- **Type 1** writes the `Subrs` 1/2/0 sequence of specification section 8.3 with
  the reference point the specification prescribes, and reserves the four
  standard `Subrs` entries whenever flex or hint replacement is used — before,
  a font with flex and no replacement would have shipped `/Subrs 0 array`.
- **Dot sections** round-trip through Type 1. CFF reports
  `cff-dotsection-dropped`, because Type 2 lists the operator as obsolete.
- **General counter control** replaces the old "one triple per axis" limit:
  anything the `hstem3`/`vstem3` form cannot express is written through
  `OtherSubrs` 12 and 13 in the layout of Adobe Technical Note #5015 §2.2–2.6,
  and read back again.
- **Imported CFF flex is recovered.** The outline dependency expands every flex
  operator into two curves before any callback runs, so a separate bounded Type 2
  scan walks the same charstring, finds each flex and binds it to the imported
  points by matching all seven coordinates. An unbindable flex, a `seac`
  composition or a charstring the scan cannot follow reports
  `cff-flex-not-imported` and keeps the unresolved marker.

### Row 98 and row 61: CFF2 private data

`write_cff2` previously emitted an **empty** Private DICT, so a variable OTF
export silently dropped every PostScript private value the document held. It now
writes the design default master's values and blends each one that varies across
the design space through the DICT `blend` operator, with the private regions
registered first so they occupy variation-data group zero and no charstring has
to be told which group it uses. Masters that declare different array lengths
report `cff2-private-not-varied` and are written without variation.

On import, a CFF2 Private DICT that blends its values used to fail the whole
import, because the reader supplied no blend state. It now resolves the blends at
the default location and reports `cff2-private-variation-flattened`.

### P5 TrueType scope: `cvar`

`fc_sfnt::tt::with_instructions` accepts control-value variation regions and
authors a `cvar` table: one tuple per region with an embedded peak tuple, its own
point numbers (the only form FreeType and FontTools agree means "every control
value") and an intermediate region when the tent is not the one the peak implies.
`fc-compile` builds those regions from the masters' control value tables with the
same interpolation model as the outlines, and the compiler now allows a variable
TrueType export of authored **global** data. Authored **glyph** programs still
require a static export; the limit and its reason are recorded below.

## Commands

Every Rust command ran under `flock .scratch/p5f/cargo.lock` with an absolute
task `TMPDIR`. Frontend commands used one Vitest worker.

```text
cargo test -p fc-hint       18 passed, 0 failed, 0 ignored
cargo test -p fc-model     114 passed, 0 failed, 0 ignored
cargo test -p fc-sfnt      122 passed, 0 failed, 0 ignored
cargo test -p fc-formats   193 passed, 0 failed, 0 ignored
cargo test -p fc-compile   218 passed, 0 failed, 0 ignored
cargo test -p fc-doc       340 passed, 0 failed, 0 ignored
cargo test -p fc-api       575 passed, 0 failed, 2 existing ignored
cargo test -p fc-cli        32 passed, 0 failed
```

```text
cargo clippy -p fc-hint    --all-targets -- -D warnings   clean
cargo clippy -p fc-model   --all-targets -- -D warnings   clean
cargo clippy -p fc-sfnt    --all-targets -- -D warnings   clean
cargo clippy -p fc-formats --all-targets -- -D warnings   clean
cargo clippy -p fc-compile --all-targets -- -D warnings   clean
cargo clippy -p fc-doc     --all-targets -- -D warnings   clean
cargo clippy -p fc-api     --all-targets -- -D warnings   clean
```

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/hints src/dialogs/exportOptionState.test.ts \
  src/dialogs/ExportDialog.test.tsx src/shell/commandHelp.test.ts
  Test Files  5 passed (5)
       Tests  55 passed (55)

pnpm --dir app exec eslint e2e/hints.spec.ts src/views/hints   exit 0, no diagnostics
pnpm --dir app check    no diagnostics in this task's files
```

`pnpm --dir app check` additionally reports two pre-existing errors in
`app/e2e/text.spec.ts` (`getAttribute`/`textContent` on `SVGElement | HTMLElement`)
from another task's in-flight edit; they are not in this task's files.

The new Rust tests are:

| Test                                                                                  | Proves                                                                  |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `fc-hint` `absence_is_reported_and_never_claimed_as_success`                          | an empty `PATH` finds nothing; a later directory still wins             |
| `fc-hint` `version_is_read_from_the_tools_first_nonempty_line`                        | the version line, and an empty string when the tool prints none         |
| `fc-hint` `validated_options_become_exactly_the_documented_flags`                     | the full option set becomes exactly those command-line flags            |
| `fc-hint` `invalid_options_are_refused_before_any_process_starts`                     | eight invalid records fail with the validation error, not `Unavailable` |
| `fc-hint` `a_successful_run_keeps_the_tools_own_warnings`                             | stderr from a successful run becomes bounded messages                   |
| `fc-hint` `the_real_installed_hinter_produces_glyph_programs_or_the_test_skips`       | the real binary when present; **skipped here**                          |
| `fc-hint` `conflicting_stems_become_point_bound_replacement_hints`                    | the conflicting stem is kept and masked, not dropped                    |
| `fc-hint` `alignment_edges_without_a_facing_edge_become_ghost_hints`                  | `-21` at the baseline stores edge + 21; `-20` stores the edge           |
| `fc-hint` `three_evenly_spaced_stems_propose_a_counter_group`                         | the `hstem3`/`vstem3` counter mask                                      |
| `fc-hint` `automatic_analysis_infers_each_private_dictionary_it_uses`                 | per-dictionary inference, no mutation, unknown index refused            |
| `fc-model` `a_flex_pair_resolves_to_two_adjacent_curve_elements`                      | the path mapping, and that it agrees with the hint-mask traversal       |
| `fc-sfnt` `an_authored_flex_pair_is_written_and_imported_again`                       | `flex1` at depth 50, `flex` at 60, and the binding on re-import         |
| `fc-sfnt` `authored_control_value_variation_replaces_the_source_cvar`                 | `cvar` deltas decoded by read-fonts at three axis positions             |
| `fc-sfnt` `an_intermediate_control_value_region_peaks_inside_the_axis`                | the intermediate-region flag and a zero delta run                       |
| `fc-sfnt` `control_value_variation_needs_a_variable_font_and_control_values`          | four refusals before any output                                         |
| `fc-formats` `flex_and_dot_sections_round_trip_through_both_containers`               | PFA and PFB round trip both mechanisms                                  |
| `fc-formats` `a_flex_whose_endpoints_are_not_aligned_is_refused_before_writing`       | the destination still holds its old bytes                               |
| `fc-formats` `general_counter_groups_use_othersubrs_12_and_13_and_round_trip`         | the exact operand sequence and the round trip                           |
| `fc-compile` `the_private_dictionary_is_written_and_varies_with_the_masters`          | blended CFF2 private values, and none when the masters agree            |
| `fc-compile` `masters_that_declare_different_private_values_are_reported_not_guessed` | the warning and no variation                                            |
| `fc-compile` `authored_globals_export_with_control_value_variation`                   | `cvt `, `prep` and `cvar` in a variable TTF, with the right deltas      |
| `fc-compile` `an_authored_glyph_program_still_requires_a_static_variable_free_export` | the documented refusal                                                  |
| `fc-api` `cid_dictionaries_and_layer_assignment_...` (updated)                        | an FD1 glyph is analysable and gets its own proposed values             |

## Independent verification

Nothing below shares code with the writers it checks.

### FontTools and FreeType on emitted fonts

`crates/fc-formats/tests/type1_flex.rs` writes the cupped serif of Type 1 figure
8e and checks with FontTools that `Subrs` 0, 1 and 2 hold exactly the sequences
section 8.4 prescribes and that the charstring contains

```text
1 callsubr  50 0 rmoveto 2 callsubr  -35 0 rmoveto 2 callsubr  10 10 rmoveto 2 callsubr
25 0 rmoveto 2 callsubr  25 0 rmoveto 2 callsubr  10 -10 rmoveto 2 callsubr
15 0 rmoveto 2 callsubr  50 200 690 0 callsubr
```

which is the specification's own worked example, translated. FontTools then
draws the flexed and the unflexed font and finds identical outlines, FreeType
completes **12 renders** over both, and Ghostscript renders the flexed font with
ink on the page. `dotsection` appears exactly twice.

`crates/fc-sfnt/tests/cff_flex.rs` wraps the written CFF into the committed
Source Serif fixture and checks with FontTools that the charstring contains
`[15, 0, 10, 10, 25, 0, 25, 0, 10, -10, 15, 'flex1']` and no `flex`, that the
flexed and unflexed outlines are identical, and that FreeType completes **12
renders**.

`crates/fc-formats/tests/type1_replacement.rs` checks the general counter-control
data with FontTools: one `callothersubr` immediately after `hsbw`, operand count
12, OtherSubrs number 13, and the data
`[1, 10, 40, 65, 40, 96, -41, 1, 50, 40, 250, -40]` — one horizontal group of
three stems and one vertical group of two, with the last pair of each carrying
the distance to the far edge and a negated width.

`crates/fc-compile/tests/cff2_private.rs` reads the variable OTF back with
FontTools: `BlueValues` `[-12, 0, 700, 712]`, `StdHW` 60, `StdVW` 80 and
`StemSnapV` `[80, 96]` at the default instance, each as a blend list with one
delta per region, `LanguageGroup` 0 unvaried, and FreeType renders every glyph at
three sizes.

`crates/fc-compile/tests/tt_cvar.rs` reads the variable TTF back with FontTools:
`cvt ` `[60, 700, -12]`, one `cvar` tuple at `wght (0.0, 1.0, 1.0)` with
coordinates `[80, 0, -12]`, `maxStackElements` 32 and the exact `prep` bytecode;
FreeType then renders every glyph at three design coordinates and three sizes
with `FT_LOAD_TARGET_MONO`, which exercises the interpreter and the control value
variation.

`crates/fc-sfnt/tests/tt_variable.rs` decodes the authored `cvar` with
read-fonts' own `Cvar::deltas` and finds `0`, the full delta and exactly half the
delta at axis positions 0, 1 and 0.5.

### The PostScript analysis quality gate

`scripts/p5_hint_acceptance.py` gained a measured gate. For each committed CFF
fixture it exports the source's own hints untouched, exports the same source with
automatic hinting, and measures both with FreeType, plus the first with hinting
switched off. The measure is the **mean distance, in pixels, from a glyph's outer
edges to the pixel grid** at 9, 10, 11, 12, 14, 16 and 20 ppem — a hint set exists
to put alignment edges on whole pixels, so this measures how well it does that.

```text
python3 scripts/p5_hint_acceptance.py --cli target/debug/fontcarver --output <scratch>

SourceSans3Min   112 edges   source stems 15   analysed stems 13
                 with the source's hints 0.09291
                 with the analysis       0.09291
                 with no hints           0.15765

SourceSerif4Min   56 edges   source stems  0   analysed stems  8
                 with the source's hints 0.19894
                 with the analysis       0.13783
                 with no hints           0.19950

ttfautohint-absent   warning autohint-unavailable, 0 glyph programs, 162 renders
```

The gate asserts that the analysis is better than no hints, and — where the
source has hints at all — no worse than the source's own by more than one
hundredth of a pixel. On the hinted fixture it matches them exactly; on the
unhinted one it is a clear improvement. The script also runs the absent-hinter
path explicitly: a TrueType export with `--autohint` reports
`autohint-unavailable`, and the output has no `cvt `, `prep` or `fpgm` and no
glyph program, so nothing pretends the font was hinted.

The remaining difference against the source's own hints is recorded rather than
hidden: for `A` and `V` the Source Sans subset carries a whole-glyph "stem"
(`0 515`, `3 538`) on the axis that has no real stem, which is how command-line
hinters hand a rasteriser the extremes of a diagonal-only glyph. The analysis
does not propose one, because it is not a stem; the measured gate shows that
costs nothing at these sizes.

## Browser evidence

`app/e2e/hints.spec.ts`, run against `fc-devserver` on 8811 and Vite on 1511:

```text
FC_API_PORT=8811 FC_UI_PORT=1511 pnpm --dir app exec playwright test e2e/hints.spec.ts
  2 passed (28.3s)
```

The first spec builds a project whose one outline is the specification's cupped
serif with its flex hint, opens it through the Open Font dialog, opens the glyph
and the **PostScript hints** editor, and then:

- reads the editor's state and finds the flex hint as editable data with a
  resolved curve pair (`flex_paths[0]` is a two-curve path, drawn over the
  outline preview) — [`flex-preview.png`](screenshots/p5f-k-hinting/flex-preview.png),
  [`flex-editor.png`](screenshots/p5f-k-hinting/flex-editor.png);
- changes the flex depth to 60, adds a dot section, applies, and reads back
  `flex_hints` `[{0, 0, 60}]` and `dot_sections` `[0]` at a higher generation;
- points the flex hint at a point that does not start two curves, applies, and
  gets an alert while the draft stays in the dialog and the document's hints and
  generation are unchanged —
  [`flex-refused.png`](screenshots/p5f-k-hinting/flex-refused.png);
- undoes and redoes the applied edit as one step.

The second spec opens the export dialog, selects TTF, enables automatic hinting,
and finds the **External TrueType hinter** section stating that
`ttfautohint is not installed on PATH` before any export runs, with the option
fields beside it —
[`autohint-status.png`](screenshots/p5f-k-hinting/autohint-status.png). The
assertion is written against the live `hints.tt_autohinter` answer, so the same
spec passes on a host that has the tool.

## Remaining limits

1. **Authored TrueType glyph programs still require a static export.** Their
   point binding is the identity of the stored outline the editor showed, and the
   editor authors against a static compilation, so a variable compilation's
   binding never matches. Export refuses rather than rebinding a program to
   points it was not reviewed against. Authored global programs, the control
   value table and its `cvar` variation do export with a variable font. Closing
   this needs the TrueType editor to author against the variable compilation,
   which is a change to the editor, the API and the compiler together.
2. **CFF2 carries no stem, hint-mask or flex operators at all.** PostScript stem
   hinting is defined for static masters and instances; a variable OTF gets the
   private dictionary and its variation, nothing more. That is the format, not a
   gap in the implementation.
3. **A variable CFF2 import flattens its private variation** to the default
   location and says so. The document model has one master for an imported
   variable font, so there are no masters to spread the original deltas over; a
   re-export authors fresh variation from whatever masters the document has.
4. **CFF2 charstrings are not scanned for flex.** Their blended operands belong
   to a variation model the importer already flattens, and the flattening is
   disclosed separately.
5. **`dotsection` is not written to CFF.** Type 2 lists it as obsolete; the data
   stays in the document and `cff-dotsection-dropped` reports it.
6. **`LanguageGroup` cannot vary in CFF2.** It selects a hinting mode rather than
   a measurement, so the default master's value is used.
7. **The analysis proposes no diagonal hints** and no whole-glyph "stem" for an
   axis without one; both are recorded with their measurement above.
8. **A hint mask cannot bind to a flex pair's joint-to-end span.** Such a mask
   would have to take effect inside the single operator that draws both curves.
   A mask at the joint is allowed and is written just before that operator.
9. **The real external binary is absent here**, so every claim about an
   installed hinter rests on the fake-executable tests and the skipped
   real-binary test, not on a run of `ttfautohint` itself.

## Recommended status

| Row         | Status                                                                                                                                                                                                                                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 61          | **Complete** with limits 1, 2, 3, 7 and 9 — the PostScript analyser proposes stems, ghosts, replacement and counter groups with a measured quality gate against the fixtures' own hints; the external bridge passes validated options, captures warnings, reports its version and never claims success when absent. |
| 98          | **Complete** with limits 5, 6 and 8 — flex, dot sections, general counter control and per-FD private dictionaries are editable, persisted, exported and read back, each verified with FontTools and FreeType, and CFF2 private variation is authored and disclosed.                                                 |
| P5 TT scope | **Complete** with limit 1 — point-identity binding, CVT and maxima editing, hinted preview and `.fcf` persistence were already accepted; this task adds `cvar` authoring from per-master control values, verified with read-fonts, FontTools and FreeType.                                                          |

## Review round

The review returned **ACCEPT WITH FIXES** with seven findings. All seven are
fixed, each with a test.

| #   | Finding                                                                                                                                             | Fix                                                                                                                                                                                                                                                                            |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | CFF2 private variation deltas used the absolute basis, but `blend` works on the operand stack, where an array operator's operands are relative.     | `write/cff2.rs` now applies the same first-difference map to every delta row that relativises the default operands. Because that map is linear, transforming the model's deltas is exact. This is what FontTools writes (`varLib/cff.py` hands `getDeltas` the relative rows). |
| 2   | `ps.rs` rejected mask offsets 1–5 inside a flex and allowed 6, the inverse of the real constraint, so a stem could be declared and never activated. | The rule is now "reject 4 to 6, allow the joint at 3", with a message that says where a mask may go. `schema.md` and limit 8 above are corrected.                                                                                                                              |
| 3   | Analyse and `--autohint` replaced the whole hint set, dropping `flex_hints`, `dot_sections` and the unresolved-flex marker.                         | `fc_hint::ps::assemble` seeds its result from the layer's own flex data and refuses to place a replacement mask on a point a flex operator owns, so the carried data stays valid. Both callers were commented to say the replacement is complete.                              |
| 4   | The `cff-dotsection-dropped` warning looked gated on `Otf`, leaving bare CFF unreported.                                                            | Bare CFF already compiles through `CompileFormat::Otf`, so the warning did reach it; that is now proved rather than assumed.                                                                                                                                                   |
| 5   | The option inputs used `min={0}` where the kernel requires 2 or 1, and a refusal surfaced only as a generic message.                                | Each input carries its own lower bound, and `hints.tt_autohinter` now takes the options and returns the kernel's own reason for refusing them, which the section shows before an export runs.                                                                                  |
| 6   | The dot-section Remove buttons shared one accessible name.                                                                                          | Both the flex and the dot-section buttons are numbered.                                                                                                                                                                                                                        |
| 7   | The external hinter's options were unreachable from the command line.                                                                               | `fontcarver build` gained sixteen `--hint-…` flags, each requiring `--autohint`, validated by the same kernel code, and refused together with `--profile` because a stored profile carries its own options.                                                                    |

New tests: `the_blended_private_values_reach_each_master_at_its_own_location`
(read-fonts blends the written DICT at `wght` 1.0 and reaches the Heavy master's
own `BlueValues` `[-18, 0, 700, 724]`, `StemSnapV` `[150, 180]`, `StdHW` 110 and
`StdVW` 150 — the old basis produced `[-18, -6, 694, 718]` and `[150, 250]`),
`a_mask_may_bind_to_a_flex_joint_but_not_from_there_to_its_end` (`fc-model`),
`a_mask_bound_to_the_flex_joint_is_activated_before_the_flex_operator`
(`fc-sfnt`, through read-fonts' own command sink: `vstem vstem mask move mask
curve curve line line`), `a_mask_bound_to_the_flex_joint_round_trips_with_the_flex`
(`fc-formats`, including the refusal at the end point with the destination
intact), `analysis_keeps_flex_hints_and_dot_sections_it_did_not_propose`
(`fc-hint`), `dot_sections_are_reported_for_every_cff_flavoured_output`
(`fc-api`, both `Otf` and `Cff`),
`external_hinter_options_are_validated_and_shared_with_the_application`
(`fc-cli`, with `assert_cmd`), and three frontend tests for the bounds, the
kernel's refusal message and the numbered buttons. The FontTools check in
`cff2_private.rs` now reconstructs the instance under operand-stack semantics
instead of reading one location.

`crates/fc-cli/src/main.rs` boxes the `Build` variant: sixteen more flags pushed
its argument record past clippy's `large_enum_variant` threshold.

```text
cargo test -p fc-hint       19 passed, 0 failed
cargo test -p fc-model     115 passed, 0 failed
cargo test -p fc-sfnt      123 passed, 0 failed
cargo test -p fc-formats   194 passed, 0 failed
cargo test -p fc-compile   219 passed, 0 failed
cargo test -p fc-doc       342 passed, 0 failed
cargo test -p fc-api       578 passed, 0 failed, 2 existing ignored
cargo test -p fc-cli        33 passed, 0 failed

cargo clippy -p fc-hint -p fc-model -p fc-sfnt -p fc-formats \
             -p fc-compile -p fc-doc -p fc-api -p fc-cli \
             --all-targets -- -D warnings                      clean

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/hints src/dialogs src/transport src/shell/commandHelp.test.ts
  Test Files  24 passed (24)
       Tests  227 passed (227)

pnpm --dir app exec eslint ...                                 exit 0
pnpm --dir app check                                           no diagnostics in this task's files
FC_API_PORT=8811 FC_UI_PORT=1511 pnpm --dir app exec playwright test e2e/hints.spec.ts
                                                               2 passed (11.3s)
python3 scripts/p5_hint_acceptance.py                          gate unchanged: 0.09291 / 0.09291 / 0.15765
                                                               and 0.13783 / 0.19894 / 0.19950
```
