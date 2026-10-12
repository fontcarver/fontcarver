# P2 preparation and horizontal kerning design

Date: 2026-09-07.
Status: P2a implemented and verified as of 2026-09-16; P2b implemented and
verified within its documented static conversion scope as of 2026-09-17;
P2c/P2d implemented and verified within documented static scope as of 2026-09-19.
Baseline: `p1-outline-editing` at `1305109`, production candidate `2e53ba6`.

## Context and authority

Latest instruction (2026-09-19): the user authorized completion of all remaining
P2, retaining multi-agent implementation/review and requiring Sol for Chinese
translation. P2c/P2d are now complete; see the
[completion design](2026-09-19-p2-completion.md) and
[acceptance report](../../p2cd-acceptance.md).

Previous instruction (2026-09-17): the user authorized completion of all P2b,
retaining approval for delegated implementation, translation, review and local
checkpoint commits. See the [P2b design](2026-09-17-p2b-editable-layout.md),
[completed implementation plan](../plans/2026-09-17-p2b-editable-layout.md) and
[acceptance report](../../p2b-acceptance.md).

Earlier instruction (2026-09-16): the user requested completion of all P2a and
authorized separate implementation, translation and review agents. The shared
resolver and ownership check now support validated pair/group transactions,
undo/redo, typed bounded queries, glyph group controls, text gap/pair editing,
and consistent compiler/persistence behavior. Source ownership still rejects
imported-table authority, active source-owned `kern`, and unparseable source.

The [implementation plan](../plans/2026-09-16-p2a-authoring.md) and
[acceptance report](../../p2a-acceptance.md) record the completed milestone.
P2a–P2d are now complete within their documented boundaries; see the
[project status](../../STATUS.md). The earlier
pauses on performance, comparison and native desktop automation remain in effect.

The user tried the P1 desktop version, found its current usability acceptable
apart from potential optimization, and requested continuation. Performance
investigation and the FontForge export comparison were explicitly deferred.
This resumes development after the earlier stop-after-P1 boundary. Native
desktop input automation remains paused; use isolated headless verification.

The binding product scope remains
`2026-09-03-fontcarver-design.md` §14, with interaction conventions from
`../../design/editor-ui.md`. This proposal subdivides P2 into independently
verifiable milestones; it does not redefine P2 as complete after kerning alone.

## Code audit

The following is present in the inspected baseline:

| Area            | Existing support                                                                                                | Next work                                                                  |
| --------------- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Metrics         | `fc-doc/src/metrics.rs`, `SetMetricsKeys`, `doc.metrics_resolve`, `MetricsPanel.tsx`, and text sidebearing keys | Retain and regression-test; these are already P1 features                  |
| Kerning model   | `KernSide`, `KernPair`, `KerningGroups`, per-master `kerning_h` / `kerning_v`, glyph group fields               | Add validated document edits and queries                                   |
| Compilation     | `fc-compile/src/source.rs` supplies horizontal pairs and groups to fontc                                        | Prove exception semantics and exported shaping through integration tests   |
| UFO             | `fc-formats/src/ufo/structure.rs` imports/exports groups and kerning; conversion fills glyph membership fields  | Preserve round trips after document edits                                  |
| Text UI         | Shaped runs, stable glyph names, pending compile handling, coalesced metrics edits                              | Add pair selection, group/exception controls and kerning shortcuts         |
| Imported layout | `FeaturesSource::ImportedTables`; raw layout tables are merged back during export                               | Imported-font kerning editing requires a separate layout conversion design |

In particular, `preserve.rs` restores imported GPOS for verified glyph order.
Simply exposing model kerning edits for those documents could let old GPOS
replace the newly compiled result. The milestone must enforce its supported
document types in the kernel as well as the interface.

## P2 delivery order

1. **P2a — Horizontal kerning authoring.** New fonts and eligible UFO/FCF source
   documents: groups, pairs, exceptions, text editing, undo, save/reopen and
   TTF export with matching preview. Existing metrics editing is retained.
2. **P2b — Editable OpenType layout.** FEA editor and diagnostics, automatic
   features, layout decompiler, and an explicit transition from imported tables
   to editable source. This stage enables supported imported TTF/OTF kerning
   editing while preserving unrelated layout behavior.
3. **P2c — Static OTF/CFF export.** Own CFF writer, export integration and
   round-trip checks. CID export remains the P4 scope in the binding roadmap.
4. **P2d — Source-format interoperability and validation.** Glyphs and SFD
   import/export, Type 1 read/write, and Find problems v1. Each format receives
   a separate implementation plan and corpus-based exit check.

The old P1 backlog is a triage input, not a list to implement blindly. Recheck
each item against current code; schedule correctness dependencies with their
owning milestone. Performance-only items stay deferred. P2 exit still requires
the professional Latin workflow and the specified source-format round trips.

## Approaches considered

**Recommended: finish source-font kerning first.** It builds on model,
compiler and UFO code already present and yields a testable authoring workflow.
Its temporary limitation is that imported binary fonts retain their existing
layout behavior but do not gain kerning editing in P2a.

**Alternative: implement layout decompilation first.** This would make binary
font editing available earlier, but requires resolving lookup coverage,
source ownership, diagnostics and shaping equivalence before exposing the
first kerning controls. It is the larger first milestone.

**Alternative: overlay new pairs onto imported GPOS.** This looks narrow but
creates a second layout-editing representation, with lookup ordering and
preservation interactions that must later coexist with FEA editing. It is not
selected for the first milestone.

## P2a user workflow

Create a font or open an eligible UFO/FCF document. Assign first-side and
second-side kerning groups in the glyph inspector. In the text tab, enter
`AVATAR`, select a gap between two glyphs, and adjust the kerning value.
The pair controls show the two glyph names, resolved value and the rule that
supplies it. A scope selector distinguishes glyph pairs, mixed glyph/group
pairs and group pairs. Controls requiring an unassigned group are unavailable.

Edits affect only the selected scope. A glyph exception of zero is an explicit
override; removing that exception is a separate action that restores the
inherited value. Group edits show their affected members before commitment.
Changing a number commits on Enter or blur; Escape restores its prior value.

The first text interaction supports unambiguous horizontal LTR glyph runs.
Ligature, mark-cluster or RTL cases that cannot identify a simple editable
pair remain previewable; their gap editor is unavailable with an explanation.
Explicit glyph-name pairs can still be authored in the pair panel. This avoids
interpreting a shaped GID as a document GID or a character boundary as a glyph
pair. Full direction-aware layout authoring is addressed with P2b/P4.

Kerning shortcuts use the existing reserved Mod+Alt+Left/Right combinations:
Left reduces and Right increases the selected value by 1 unit, or 10 with
Shift. On macOS, additional Control selects the existing 100-unit convention.
Numeric fields support direct larger values on every platform. Consecutive
key presses form one undo operation after the existing 500 ms idle boundary.
Text inputs, modal dialogs and unrelated shortcuts keep their normal behavior.

## Document eligibility and layout ownership

P2a edits require `FeaturesSource::Fea` and compiler-owned kerning. Fresh fonts
and UFO documents with ordinary groups/pairs qualify. FCF inherits its stored
layout authority; its file extension alone does not establish eligibility.

Documents with imported layout tables remain openable, previewable, editable
for existing P1 operations and exportable under the existing preservation
rules. Kerning authoring is unavailable until layout conversion is supported.
Explain this in product terms: the imported font's spacing rules cannot yet be
edited. Do not expose internal enum names in the UI.

FEA source that explicitly owns `kern`, or cannot be parsed enough to establish
that ownership, also makes pair editing unavailable in P2a. Determine this via
the existing compiler/parser facilities, including prefix blocks and lookup
references; do not use substring matching. Do not rewrite or delete the source.
The eligibility check is shared by queries and commands, so a direct API call
cannot bypass it. P2b will define the editable source/model ownership transition.

## Data and transaction rules

Keep existing model types. `Font.kerning_groups` is authoritative for membership;
commands update the matching glyph `kern_group_side1` / `kern_group_side2`
fields atomically. A glyph belongs to at most one group on each side. First
and second side are pair positions, not labels for visual left/right bearings.
Use canonical UFO group names `public.kern1.*` and `public.kern2.*` internally.

Resolution order for a concrete pair is glyph/glyph, glyph/group, group/glyph,
then group/group. Explicit zero counts as a found value. This follows the
[UFO 3 kerning exception rules](https://unifiedfontobject.org/versions/ufo3/kerning.plist/).
Membership validation follows the
[UFO 3 group rules](https://unifiedfontobject.org/versions/ufo3/groups.plist/).

New authoring operations reject non-finite values, wrong-side group names,
missing glyph/master references and duplicate side membership before mutation.
Existing imported UFO data with unresolved references is retained and reported;
opening a font must not silently discard such data. Unresolvable records are
excluded from compiled output with diagnostics, as required for source data.

Use the current document command and edit-session pipeline for setting/removing
pairs and creating, renaming, assigning or deleting groups. Group renaming
updates references in every master's horizontal pairs and glyph membership in
one undoable operation. In-use groups cannot be removed silently: the UI shows
the affected records and an explicit remove-group-and-pairs action executes
atomically. Glyph rename/delete behavior receives matching regression coverage.
Vertical kerning data is retained unchanged by horizontal operations.

Pair values are per master; group membership is shared at font scope. Mutations
and undo/redo advance document generation and invalidate compiled shaping.
Add a layout-specific change signal where existing events cannot describe
the mutation. A kerning-only edit must not mark outlines as touched or trigger
the P1 outline-edit hinting removal path. No-op edits create no undo record.

## API, UI and compilation boundaries

Add a dedicated kerning query/validation module in `fc-doc`, API request/response
types and handlers in `fc-api`, and generated TypeScript bindings. Queries expose
eligibility with a structured reason, paged pairs/groups, membership and the
resolved pair plus its supplying rule. Reject stale pagination generations so
the panel cannot combine results from two document states. Do not transfer a
complete font or expand every group pair into a Cartesian product for the UI.

Keep presentation components under `app/src/views/glyph/panels/` and
`app/src/views/text/`; place pair-edit session logic in a separate plain
TypeScript module. Reuse NumberField, panel styling, edit barriers, transport,
and the existing text request identity guards. Follow current light/dark,
keyboard accessibility and English/Chinese localization conventions.

Before changing document, pair, master, text mode, save/export or close, drain
the old pair edit using the established document edit barrier. Errors preserve
the last committed value and expose the failed action. A late response for a
previous document/master must never replace the current selection or preview.

The compiled font and kernel shaper remain authoritative for preview. Do not
add a client-side kerning offset on top of an already shaped advance. Reuse
pending/stale preview feedback while compilation completes. The pair inspector
can show the newly stored value before shaping, but a stale run is not presented
as current output. Existing fontc pair/group conversion is reused and corrected
only where the new behavior tests demonstrate a gap.

## Verification and exit criteria

P2a is complete only when the following agree through the real API/compiler:

1. A generated Latin font with 600-unit advances and `A V = -80` shapes `AV`
   to a total advance of 1120 with `kern` enabled and 1200 with it disabled.
   Removing the pair restores the unkerned advance.
2. A group pair of -100, glyph/group exception of -60, group/glyph exception
   of -40 and glyph/glyph zero resolve by the priority above. Remove the zero
   and observe -60 for the overlapping exceptions. Exported shaping agrees.
3. Pair/group edits, rename/delete and master switching survive FCF and UFO
   save/reopen. Undo/redo restores exact pair records, membership and dirty state.
4. A glyph rename updates all pair and membership references. Validation errors
   and no-op edits leave generation, data and undo history unchanged.
5. Kernel and UI both reject unsupported imported-table or source-owned-kern
   editing. An unchanged imported font retains its existing layout behavior
   through the existing P1 preservation pipeline.
6. Headless interaction tests select a gap, edit a group value, create/remove a
   zero exception, use keyboard repeat, undo, save, export and reopen. Include
   delayed failures and a master/document handover during a pending edit.
7. Preview, TTF export and UFO reimport agree on representative pairs; existing
   metrics-key and P1 persistence regressions remain green. Kerning-only edits
   preserve untouched-outline behavior under the existing provenance rules.

Run focused Rust/TypeScript tests per task and the integrated checks at the
milestone boundary. New UI strings require a separate translation and QA pass
under `CONVENTIONS.md`. No native desktop automation is implied. Do not claim
fresh Source Han timings, a desktop packaging pass, or full P2 completion from
these checks. Record commands and results in a new P2 acceptance report.

## Implementation planning boundary

Proposed model allocation: `gpt-6-astra` with `xhigh` reasoning for architecture,
layout decompilation, CFF correctness, cross-module integration and independent
review; `gpt-5.6-sol` with `high` or `xhigh` reasoning for bounded commands,
API/UI wiring, regression tests and a separate localization pass. These are
proposed assignments, not claims that agents have been started. Retain the
user's earlier preference for Sol on bounded work and Astra on difficult work.

Initial scheduling estimate, assuming 6–8 hours of active work per day,
limited parallel implementation, and review/testing included: P2a 1–2 working
days; P2b 3–7; P2c 2–4; P2d 3–7. Total planning envelope: roughly 2–4 working
weeks. This is a low-confidence engineering estimate, not a measured completion
rate or delivery commitment. Re-estimate after P2a; imported lookup coverage and
source-format round trips are the largest uncertainties. Performance work and
release packaging are excluded.

After review of this proposal, write the P2a implementation plan with separate
testable tasks for document rules/undo, API queries, compile/preservation
integration, inspector controls, text interactions, and localization/acceptance.
Assign exact interfaces and file ownership there. P2b, P2c and P2d each require
their own detailed design before implementation.
