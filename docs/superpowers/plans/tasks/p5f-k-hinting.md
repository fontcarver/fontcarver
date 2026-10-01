# p5f-k: Hinting completion

Area id: `k-hinting`. Ports: API 8811, UI 1511. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                    | State at start                                                                                                                                                                                                                                                         |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 61  | Autohinting (ttfautohint for TT, PS autohint from stems/zones)                                                                                 | Partial: static PS analysis and the external ttfautohint bridge work through export UI/CLI (`scripts/p5_hint_acceptance.py`). Variable-font hinting and analysis quality remain open. `ttfautohint` is not installed on this host; keep availability reporting honest. |
| 98  | Manual PostScript hint editing (stems, ghosts, hint replacement)                                                                               | Partial: editor, undo, persistence, CFF stem/mask/counter round trips, Type 1 parameters, seac inheritance, CFF1 per-FD preservation. Flex, dot sections, general counter control, editable per-FD dictionaries and CFF2 private variation remain.                     |
| —   | P5 TT scope: authored TT programs bound to point identities, CVT/maxp, hinted preview, variable-font hint data (`cvar`), imported flex binding | See `docs/p5-hinting-checkpoint.md`, `docs/p5-hinting-resume-checkpoint.md`, `docs/p5-tt-variable-binary-checkpoint.md`.                                                                                                                                               |

Existing records: `docs/p5-hinting-checkpoint.md`, `docs/p5-hinting-resume-checkpoint.md`,
`docs/p5-tt-variable-binary-checkpoint.md`, `docs/p5-cff-private-editor-acceptance.md`,
`docs/acceptance/p5-cff-*.md`, `docs/acceptance/p5-ps-analysis.md`,
`docs/superpowers/plans/tasks/p5-tt-variable-binary.md`, `p5-type1-seac-hints.md`,
`p5-02-cff-flex-guard.md`, `docs/user/hinting.md`, `scripts/p5_hint_acceptance.py`,
`scripts/p5_tt_instruction_acceptance.py`.

## Scope notes

- Row 61 (PS): improve the autohinter to the level expected of a professional
  tool: stems from zones and stem lists, ghost hints at alignment zones, hint
  replacement where stems conflict, counter control for `hstem3`/`vstem3`
  patterns, and a quality gate that compares against the font's existing hints
  on the fixture corpus (FreeType renders at small ppem should not regress in
  a measured way; record the metric). Variable fonts: CFF2 has no hints, so
  document that PS autohint applies to static instances; for TT variable
  fonts, either implement `cvar`-aware CVT authoring or record the exact limit.
- Row 61 (TT): the external ttfautohint bridge must detect availability,
  pass options through, capture warnings and never claim success when the tool
  is absent; add a test that simulates absence and one that runs when the
  binary exists (skipped otherwise). Document installation in `docs/user/hinting.md`.
- Row 98: flex hints (editable flex on two curve segments, Type 1 and CFF
  `flex`/`flex1` output), `dotsection`, general counter control groups, editable
  per-FD private dictionaries for CID-keyed CFF (not only preservation), and
  CFF2 private-dictionary variation data preserved through import/export.
  Every editing path: undo, `.fcf` persistence, export verified with FontTools
  (`T2CharString` decompile) and FreeType rendering.
- TT: authored programs keep binding to point identities through edits;
  hinted preview at several ppem in the panel; `cvar` and per-master CVT values
  for variable fonts if the P5 checkpoint left them open; validated compiled
  bindings with independent interpreter tests (FreeType with hinting enabled).

## Owned files

`crates/fc-hint/**`; `crates/fc-sfnt/src/{write/hints.rs,write/cff.rs,write/cff2.rs,write/charstring.rs,tt/**,cff_interchange.rs}`;
`crates/fc-model/src/{ps.rs,tt.rs}`; `crates/fc-doc/src/command/hints.rs`;
`crates/fc-api/src/handlers/{hints.rs,tt_hints.rs}`, `crates/fc-api/src/types/{hints.rs,tt_hints.rs}`;
`crates/fc-raster/src/hint.rs`; `app/src/views/hints/**`;
`crates/fc-formats/src/type1/**` (hint parts); `scripts/p5_hint_acceptance.py`,
`scripts/p5_tt_instruction_acceptance.py`; `docs/user/hinting.md`; `docs/p5f-k-hinting-acceptance.md`.

Shared files (narrow patches only): `crates/fc-compile/src/**` (hint pipeline
hooks; task f owns the crate), `crates/fc-sfnt/src/{write/mod.rs,read/**}`,
`crates/fc-model/src/glyph.rs`, `crates/fc-doc/src/fcf/schema.md`,
`app/src/dialogs/ExportProcessingOptions.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.

## Gap analysis

Written after reading the code for every row. `ttfautohint` is **not installed**
on this host (`which ttfautohint` is empty), so the external-tool paths are
tested with a controlled fake executable plus a real-binary test that skips.

### Row 61 — TrueType (external ttfautohint bridge)

Works now: `fc_hint::external::executable()` finds the binary on `PATH` without a
shell; `autohint()` runs it in a private temporary directory with a 120 s
deadline, size caps, child reaping and output revalidation;
`fc-compile/src/export.rs` turns `Error::Unavailable` into the
`autohint-unavailable` export warning instead of claiming success.

Missing: (1) no options reach the tool — the bridge hardcodes `--no-info` and
nothing else, so the hinting range, hinting limit, scripts, stem-width mode,
x-height snapping and `--dehint` are unreachable; (2) stderr is read **only on
failure**, so a successful run's warnings are discarded; (3) nothing reports the
tool's presence or version before an export, so a user cannot tell whether
"Automatic hinting" will do anything; (4) no test covers absence, and the
real-binary path is untested.

Plan: add a validated `AutohintOptions` record and an `AutohintOutcome`
(bytes + version + captured messages); add `availability()` returning path and
`--version`; surface it through a new `hints.tt_autohinter` API method and the
export dialog; pass the options from `ExportOptions` (additive field, serde
default). Tests: absence (`PATH` without the binary), a fake hinter that emits
warnings on success, argument construction, and a real-binary test gated on
`executable()`.

### Row 61 — PostScript autohinter quality

Works now: facing-edge detection with curve-extrema tangents, ink
cross-sections, master-stem affinity, blue/other-blue/stem-snap inference, work
and candidate budgets.

Missing, each a professional expectation: ghost hints (`-20`/`-21`) for
alignment edges with no facing edge — the analyser emits none, so a stem-less
cap or baseline edge is unhinted; hint replacement — conflicting stems are
**dropped**, so a glyph such as `m` keeps one stem set instead of per-segment
masks; counter control — no `hstem3`/`vstem3` triple is ever proposed; stem
edges are not snapped to the master's alignment zones; and there is no measured
quality gate against a font's own hints.

Plan: implement all four in `fc-hint::ps`, with a `PsAnalysis` result that
carries stems, ghosts, masks and counter groups; add a FreeType-measured quality
gate to `scripts/p5_hint_acceptance.py` that compares our analysis with the
source font's own hints over the committed CFF fixtures and reports the metric.

Ghost convention is fixed by the real Nimbus Sans data already asserted in
`crates/fc-formats/tests/type1_hints.rs`: width `-20` means the stored `start`
_is_ the top/right edge; width `-21` means the real bottom/left edge is
`start - 21`.

### Row 61 — variable fonts

CFF2 has no hint operators at all, so PS autohinting is defined only for static
masters and instances; that stays a documented boundary. For TrueType the
checkpoint left `cvar` authoring open: `fc_sfnt::tt::with_instructions` can
_preserve_ an existing `cvar` but requires a byte-identical CVT, and
`fc-compile/src/tt.rs` rejects authored programs for `options.variable`
altogether. Plan: author `cvar` from the per-master `TtMasterData.cvt` using the
same `fontdrasil` variation model the CFF2 path already uses, allow variable
export when every master's `fpgm`/`prep` agree, and keep rejecting anything else
with a precise message.

### Row 98 — flex

`PsHints::flex` is a bare `bool` that no importer ever sets; CFF, Type 1 and SFD
output all reject it. Imported flex is expanded to two ordinary curves by
`read-fonts` before our sink sees it, **without a warning**, so an OTF → OTF
round trip loses flex silently. Decision: make flex editable data bound to point
identities — `PsHints.flex_hints: Vec<FlexHint { contour, segment, depth }>`,
where `segment` is the index of the first of two consecutive cubic segments.
Keep the legacy `flex: bool` as the "unrepresentable imported flex" marker so no
existing `.fcf` changes meaning. Validate the Adobe conditions (exactly two
curves, outer endpoints sharing x or y, joint and its neighbouring controls at
the extreme, flex height ≤ 20 units). Emit CFF `flex`/`flex1` and the Type 1
`Subrs 1/2/0` sequence of §8.3, with the reference point computed by the
specification's rule. Import: a bounded Type 2 segment scan records flex pairs
and their depth, and the Type 1 reader accepts the standard sequence.

### Row 98 — dotsection

Nothing exists. Type 1 §8.2 brackets one dot subpath between two `dotsection`
commands. Decision: `PsHints.dot_sections: Vec<usize>` (contour indices);
Type 1 import and export; the editor exposes it. Type 2 lists `dotsection` as
obsolete, so CFF output keeps the data in the document and reports
`cff-dotsection-dropped` rather than writing an obsolete operator — recorded as
a deliberate limit, not a silent loss.

### Row 98 — general counter control

CFF already supports arbitrary `cntrmask` groups. Type 1 only accepts a single
`hstem3`/`vstem3` triple per axis. Technical Note #5015 §2.2–2.6 defines the
general form through `OtherSubrs` 12/13 with `#H HG1..HGn #V VG1..VGn m`,
each group a run of (distance-from-previous-stem, width) pairs whose last pair
carries the distance to the far edge and a negated width, split into ≤22-operand
calls from the end of the data. Plan: implement that encoding and its import,
keeping `hstem3`/`vstem3` for the exact triple case (smaller and understood by
every interpreter).

### Row 98 — per-FD private dictionaries

Already editable: `HintCommand::CffPrivate` and `CffPrivateAssignment`, the
`CffPrivateSelector` panel, FCF fields and CFF1 output. The real remaining gap
is that automatic analysis refuses any layer assigned to a dictionary other than
FD0, so a CID font's non-FD0 glyphs cannot be analysed at all. Plan: analyse
per dictionary and infer each dictionary's private values from the glyphs
assigned to it.

### Row 98 — CFF2 private variation data

`write_cff2` emits an **empty** Private DICT, so a variable OTF export drops
every PostScript private value the document holds, silently. Import resolves
CFF2 blends to the default instance and warns only about multiple dictionaries.
Plan: author the CFF2 Private DICT from the masters' `private_dict` values and
emit DICT `blend` (operator 23) with a dedicated `vsindex`, computing deltas with
the same variation model as the outlines; warn on import when a CFF2 private
value carries variation deltas that are flattened to the default instance. A
document imported from a variable CFF2 has one master, so re-export authors
fresh variation data rather than replaying opaque deltas — recorded as the
boundary.

### P5 TT scope

Point-identity binding, CVT/maxp editing, the hinted preview at up to eight
ppem and FCF persistence are implemented and were verified by the earlier
checkpoints; this task re-verifies them and adds `cvar`/per-master CVT authoring
plus independent FreeType interpreter checks of the compiled variable result.
