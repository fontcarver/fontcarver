# P2b — editable OpenType layout

Date: 2026-09-17. Status: independently reviewed design; implementation pending.

## 1. Outcome and authority

The user authorized advancing the proposed next step: detailed P2b design and
task decomposition. P2a is complete. This document makes P2b implementable; it
does not claim that its features have shipped. The user separately authorized
local checkpoint commits. The binding product scope remains
[product design §§6.2–6.3, 12.5, 14](2026-09-03-fontcarver-design.md) and the
[P2 delivery order](2026-09-07-p2-kerning-design.md).

P2b delivers an FEA source editor with diagnostics, automatic features, a
read-only imported lookup inspector, static layout decompilation, and an
explicit undoable transition from original tables to compiled source. Supported
imported TTF/OTF fonts gain source editing and, where lossless lifting is proven,
the P2a pair/group controls. OTF input does not imply OTF/CFF output: that is P2c.

P2c/P2d, variable-layout authoring, full vertical text editing, performance
investigation, FontForge comparison and native desktop automation remain later
or paused work. Static vertical rules in imported tables must still survive
conversion; a horizontal UI is not permission to drop them.

## 2. Inspected baseline and prerequisites

| Current code | Consequence for P2b |
| --- | --- |
| `fc-model/src/features.rs`: structured blocks and global `Fea` / `ImportedTables`; `automatic` is metadata | Add real generation policy and distinguish saved drafts from active source |
| `fc-compile/src/source.rs`: `Features::to_fea()` plus fontc kerning/anchor writers | Validate with the same assembled source and writer options used by export |
| `fc-formats/src/ufo/structure.rs`: whole-file FEA import/export has separate assembly | Unify assembly; migrate the historical synthetic `fea` block without double wrapping |
| `fc-sfnt/src/read/mod.rs`: GSUB/GPOS/GDEF trigger imported authority | Include BASE-only and legacy spacing-table cases in authority inventory |
| `fc-compile/src/preserve.rs`: GSUB/GPOS/GDEF/BASE restored only under imported authority; legacy `kern` is separate | Converting GPOS alone is unsafe; decide every owned table together |
| `fc-layout` depends on `fc-compile`; it currently provides shaping | Compiler helpers cannot depend back on `fc-layout` |
| `fc-doc/src/undo.rs`: `Impact.layout` captures kerning snapshots | Add distinct feature/draft/conversion snapshots and invalidation |
| `app/src/editor/editSession.ts`, `shell/tabActions.ts`: outline/text edit handovers | Register Features drafts as document edit participants; propagate persistence errors |
| `fc-compile/src/shaping.rs`: Latin/default-feature advance fingerprint | Add full glyph/cluster/offset/script/language comparison for layout acceptance |

No new runtime dependency is required by the design work. Implementation uses
the locked fontations, fea-rs, fontc and harfrust versions. CodeMirror 6 packages
are a planned frontend dependency, with exact versions and permissive-license
verification recorded when introduced; no speculative dependency upgrade.

## 3. Source, draft and authority

There are two independent axes: the active representation (original tables or
accepted FEA) and an optional editable draft. Merely opening the Features tab,
decompiling, typing, saving a draft or running diagnostics never changes active
layout. The action **Compile & preview** validates a candidate and activates it
atomically on success. Failure retains the draft and previous active layout.

Persist a `FeatureDraft` in the native document, including ordered source blocks,
stable block IDs, generation options and a draft revision. Invalid text is valid
draft data. Keep `Font.features` as the effective source/authority so existing
consumers do not accidentally compile draft text. The optional new fields have
serde defaults; update the FCF schema and old-document fixtures. Old accepted
source that no longer compiles remains repairable in the editor.

The source tree contains Prefixes, Classes and Features. Stable IDs are editor
identity, not tags or array offsets: reorder/rename/duplicate tags must not move
a diagnostic to another block. A whole imported `.fea` file is an opaque prefix,
not a fabricated `feature fea { ... }`. Assembly preserves comments and order;
it does not pretty-print user source. Handle existing synthetic `fea` records
on load through a narrowly tested compatibility migration.

Use one pure mapped assembler in `fc-model`: source text plus byte ranges mapping
back to block IDs and local source. `to_fea()` remains a compatibility wrapper.
Compiler, eligibility checks, diagnostics and UFO export consume this assembly.
Generated wrappers map to the enclosing block; generated rules map to their
generator and glyph names. UI diagnostic offsets use UTF-16; explicitly convert
from Rust UTF-8 byte spans, including non-ASCII comments and CRLF fixtures.

Draft mutations are undoable/coalesced, mark the document dirty and emit a new
feature-source change signal. They do not mark outlines touched or invalidate
active shaping. Introduce an effective compile revision/cache key that excludes
draft-only edits while still including every existing glyph/metrics/layout
invalidation. Successful activation, conversion and revert each form one undo
entry containing source, draft, authority and affected lifted model data.
Do not reuse `Snapshot::Kerning` as if it captured feature text.

Native save/autosave persists active state and draft, even when invalid. Binary
export and UFO export reject unapplied draft changes with a specific reason and
actions to apply or discard them. Never silently export an older accepted source
while the editor appears to show the exported rules. Source export of the draft,
if offered, is explicitly labelled and never changes active layout.
An unchanged materialized view is not an unapplied edit: opening a source tree
or displaying a decompiled baseline keeps the document clean and exportable.
Native/binary export follows the applicable preservation policy. UFO/designspace
source export requires accepted FEA authority even for a clean imported font:
ask the user to convert its layout first. Enforce this in the source writer as
well as API/CLI, before creating or replacing output. An empty `to_fea()` under
imported authority must never masquerade as a successful layout source export.

## 4. Compiler, API and concurrency contract

Names below are proposed interfaces, to be fixed by tasks 01–03 before parallel
UI implementation. Rust defines them and generated TypeScript mirrors them.

| Operation | Contract |
| --- | --- |
| `features.get` | Active authority, draft/source blocks, revisions, generation policy and source status |
| `features.set_draft` / `features.discard_draft` | Expected draft revision and document generation; ordinary undoable draft mutation |
| `features.validate` | Pure candidate validation; diagnostics keyed by document/master/draft and effective compile-input revisions; no activation |
| `features.apply` | Snapshot, compile outside lock, recheck identity, atomically activate or return diagnostics/stale result |
| `layout.inspect` / `layout.rules` | Bounded table/lookup/rule pages with generation-checked continuation |
| `layout.prepare_conversion` | Read-only conversion candidate, coverage report, proof results and bounded server-held token |
| `layout.apply_conversion` | Consume a validated token only if its document/source/glyph/master revisions still match |
| `layout.revert_original` | Undoable return to original tables after provenance and affected-data checks |

Heavy parsing/decompilation/compilation happens outside the session lock, like
the existing shaping handler. All master contexts affected by font-global FEA
must validate before activation. Tokens are in-memory, scoped to document and
input hashes, bounded in count/lifetime, invalidated on close or input changes,
and never serialized into FCF. A failed compile or stale result creates no undo
record and changes neither dirty state nor authority.
`features.apply` rejects `ImportedTables` authority. Imported source/lookup
renderings remain read-only until `layout.apply_conversion` commits the exact
baseline candidate hash covered by its token and equivalence checks. Ordinary
source edits follow that conversion and may intentionally change behavior.
Neither generic source apply nor generation/freeze commands can bypass the
coverage gate by supplying different source alongside a valid token.

Keep validation/generation in `fc-compile`; decompilation and imported layout
comparison in `fc-layout`; pure types/assembly in `fc-model`; persistence and
undo in `fc-doc`; orchestration and token storage in `fc-api`. Do not add a
`fc-compile -> fc-layout` dependency. An internal typed, non-deserializable
prepared-change path in `fc-doc` commits validated changes. Public `doc.apply`
must not accept an unchecked source-of-truth switch or caller-supplied proof.
Direct Rust consumers must use the same validation service before this trusted
commit path; document that trust boundary and test all public API entry points.

Parser success alone is insufficient: validate glyph names, lookup references,
writer interactions and actual compilation. Reject external `include()` access
in the in-memory editor with a located diagnostic, matching current eligibility
policy. Bundled include-file import is a separate future design, not implicit
filesystem access. Limits for source bytes, rules, recursion and diagnostics
must be checked before expensive work; truncated diagnostics report truncation.

Applied source is the last accepted text, not a promise that future glyph edits
cannot invalidate it. Rename/delete/export-flag/master changes trigger fresh
validation. Do not regex-rewrite FEA on glyph rename. Show stale/failed preview
state and block export if the current glyph set cannot compile. Any later
semantic rename refactor requires parser-backed edits and its own tests.
Validation results also carry the effective compile-input revision. An unchanged
draft can become invalid after a glyph/metadata change; matching draft revision
alone is insufficient to display a result. Test delayed diagnostics across
glyph deletion and undo/redo, and discard results from obsolete input revisions.

## 5. Automatic features and ownership

Keep one owner per generated feature. Existing model kerning and anchors use
fontc writers; source-owned `kern` continues to disable P2a pair editing unless
an explicit lossless lift transfers ownership. Do not weaken the P2a parser
gate simply because a document once came through conversion.

Automatic name features are opt-in per font and deterministic: `.sc -> smcp`,
`.ss01`–`.ss20`, `.case -> case`, `.vert -> vert/vrt2`, and unambiguous underscore
ligatures -> `liga`. Discretionary ligatures and `locl` require explicit stored
configuration (feature tag, script/language and mappings); ambiguous names are
diagnosed rather than guessed. Missing bases, duplicate targets, class/name
collisions, ordering and user-owned feature conflicts have deterministic tests.

Show generated text in an Automatic section, with the responsible glyphs.
Freeze copies the generated snapshot into ordinary editable source and disables
that generator in one undo step. Regeneration never overwrites frozen/user text.
Existing `automatic: true` metadata alone must not silently enable new rules
when an old FCF/UFO opens. Respect supported `# Automatic Code` writer markers;
pin behavior with the installed fontc implementation, including markers inside
source-owned feature blocks. Manual source wins or produces an explicit conflict;
duplicate feature execution is never the fallback.

Imported conversion starts with all additional generators disabled. Existing
anchors/kerning must not cause extra generated rules beside decompiled rules.
P2b also implements bounded whole-lookup mark/mkmk anchor lifting where lossless,
as required by the product design. Otherwise keep mark/mkmk/curs rules in FEA.
The ownership policy must suppress
automatic writers for source-owned imported rules, not delete model anchors.
Name-based automatic source is a materialized snapshot refreshed only through
Compile & preview; glyph changes mark its regeneration status out of date.
Existing model kerning/anchor writers keep their existing live behavior under
their explicit ownership policy. Draft generation options are never active
before successful application.

## 6. Imported layout and supported coverage

`LayoutImport` is a typed runtime cache in `fc-layout`: parsed owned tables,
lookup graph, decompiled rendering, coverage diagnostics and source fingerprints.
The original raw tables and glyph-order provenance already persist in FCF;
do not force write-fonts table types into the serde model or store a second
unbounded binary copy. Rebuild derived cache after reopen. Persist conversion
metadata and any minimal pre-conversion model snapshot needed for revert.

The source transition covers GSUB, GPOS, GDEF and BASE together. Inspect legacy
`kern` and AAT `kerx`/`morx`/`mort` as competing layout authorities too. The first
P2b converter blocks fonts containing these competing tables unless a separate
tested conversion policy handles them; source activation must never resurrect
legacy kerning through the generic raw-table merge. Original-table mode retains
its existing preservation behavior. Fix BASE-only authority classification and
fail closed on old FCFs with contradictory source/table authority.
JSTF contains layout lookup indices as well as glyph references; reject conversion
when JSTF is present until a separately tested index-remapping policy exists.

| Coverage | First end-to-end slice | P2b completion target |
| --- | --- | --- |
| GSUB | Single and ligature substitutions | Static types 1–8, including alternate, multiple, contextual/chaining and reverse chaining |
| GPOS | Single and pair adjustments | Static types 1–9, including cursive, marks and contextual/chaining |
| Extension wrappers | GSUB 7 / GPOS 9 wrapping supported lookups | Same coverage as their wrapped type; bounded traversal |
| Shared structure | Feature/script/language assignment, lookup order | Required features where compiler-representable, shared lookups, flags and filtering sets |
| GDEF | Glyph classes | FEA-representable classes, attachments, carets and mark filtering dependencies |
| BASE | Detect and report | Static FEA-representable axis/tag/script coordinates |

This is semantic coverage, not byte reproduction after conversion. Unknown
formats, malformed offsets, cyclic lookup references, unsupported FeatureParams,
device/contour-point forms not preserved by the pinned compiler, variation stores,
FeatureVariations, nonrepresentable BASE/MinMax data, and resource-limit hits
block the entire conversion. A coverage report names the table/lookup/subfield.
Missing harmless tables are allowed; unsupported or unreachable-but-present
records are not silently discarded. No partial “convert anyway” action in P2b.
Contour-point references in GDEF attachment/caret data and GPOS anchors also
need a proven output point mapping; recompiling equivalent FEA alone is not
sufficient to accept them after outline conversion.

The initial simple slice does not satisfy the whole milestone. Implement and
verify the static contextual/class-based and mark paths before declaring P2b
complete. A capability matrix checked into the tests records supported syntax
and explicit rejection fixtures against the pinned compiler.

Original mode exports the original layout bytes when provenance permits. After
conversion, the compiled source owns all four tables, including absence of a
table: stale raw originals cannot fill an intentionally removed table. Other
raw-table preservation retains existing glyph-order checks.

Revert restores original layout authority and only the model kerning/anchor data
changed by conversion/lifting, retaining outline and metadata edits. Explain
that subsequent spacing/source changes will be replaced; undo restores them.
Revert is unavailable when original glyph-order provenance no longer matches,
required raw tables are missing, or restoring lifted data would create invalid
references. Save/reopen must not remove the ability to revert an eligible font.
Here save/reopen means the native FCF document. Exported TTF/WOFF/WOFF2 files
contain the newly compiled layout, not the editing history or old-layout backup;
opening one creates a new import baseline. Binary-reopen acceptance compares
output behavior, while original-layout revert is tested after FCF reopen.

## 7. Lossless kerning lift

Decompilation and lifting are separate outcomes. A font can have editable FEA
while its spacing stays source-owned. Pair controls activate only when all
effective horizontal `kern` behavior has moved to the model with equivalence.

Start with one nonshared, noncontextually referenced `kern` PairPos lookup,
including supported extension wrapping. Value1 contains only horizontal advance;
Value2's format must actually be zero (an encoded zero record can affect lookup
traversal). Preserve ordered subtables, coverage, script/language associations,
flags and GDEF mark filtering. Multiple additive lookups and context-sensitive
kerning remain source-owned until an explicit proven normalizer exists.

Class 0 is the correct complement over the relevant glyph universe, not an
empty class. Keep group data bounded without glyph-pair Cartesian expansion.
Any synthesized complement is materialized with provenance; adding/removing
glyphs cannot silently extend its meaning. Group partitions must fit the P2a
one-group-per-side invariant. Zero-valued records that suppress later subtables
must survive as semantic rules. A pure glyph pair numeric spot-check is not
enough to prove this.

Compile the lifted candidate using the actual fontc writer. Compare canonical
effective pair behavior, flags/mark filtering, script/language feature graph,
subtable traversal and ordering interactions. Fontc currently chooses scripts
and mark flags and may omit zero group pairs; reject the lift if these differ.
Only remove the source `kern` owner in the same transaction that installs the
validated model groups/pairs. Source-only conversion remains available when
full layout conversion succeeds but lift fails.

For anchor lifting, begin with complete nonshared mark-to-base/mark-to-mark
lookups representable by per-master named model anchors. Require static XY
anchors, consistent mark classes, no conflicting existing anchors and no
contextual callers. Exclude point/device anchors and ligature/cursive forms
until an explicit mapping is proven. Preserve filters, language assignment and
lookup order via the same actual-writer equivalence gate. Remove FEA rules only
in the atomic transaction that installs their model anchors; otherwise retain
all source rules. Record affected glyph/layer/anchor identities for undo/revert
without marking their outlines touched. Milestone acceptance needs both a
successful mark/mkmk lift fixture and a source-owned fallback fixture.
Build the positive fixture from the pinned writer's simple model `top`/`_top`
anchors, then import, decompile and lift it back; this proves a representable
path without claiming arbitrary mark lookup inference.

## 8. Features UI and edit handovers

Add a Features document tab using CodeMirror 6: FEA highlighting, glyph/class/tag
completion, a Prefixes/Classes/Features tree, located diagnostics, generation
toggles and Compile & preview. Imported fonts also have a read-only Lookups
view and conversion summary. The UI reports original rules, edited draft,
applied source, pending validation and failed validation distinctly.
Lookup rules render referenced glyphs as well as names, following product
design §12.5; large classes use bounded pages rather than an unbounded preview.

Register a generic document edit participant so Features joins the P2a barriers
for tab/document/master changes, save/export, undo/redo and close. Draining saves
the current draft; it does not implicitly compile it. Persistence failures block
the handover and leave the editor intact. Close follows existing unsaved-document
handling; native save can safely store an invalid draft. Late diagnostics and
conversion results are ignored by document, master and revision identity.

Source Ctrl/Cmd+Z uses one intentional history policy: local keystrokes coalesce
into document draft transactions, and navigation/apply boundaries flush them;
prevent CodeMirror and document undo from independently undoing the same edit.
IME composition must finish safely before flushing; Ctrl/Cmd+S inside the
contenteditable editor must reach document save despite existing input guards.
Pair controls and text preview refresh only after successful activation.

Use existing tokens, CSS Modules, accessible controls and English i18n keys.
Chinese translation is produced by a different agent, then independently checked.
Visual acceptance covers English/light and Chinese/dark; isolated headless
browser testing follows P2a practice. No native input automation is resumed.

## 9. Acceptance contract

1. Source assembly is identical across validation, compilation and UFO export;
   mixed prefixes/classes/features and old whole-file source round-trip.
2. Invalid/non-ASCII drafts survive save/autosave/reopen without switching
   authority; apply success/failure, undo/redo and stale races are atomic.
3. Synthetic fixtures cover every claimed lookup type, class-0 behavior,
   shared/contextual references, required features, flags, zero values and
   supported GDEF/BASE fields. Unsupported fixtures block without mutation.
4. Untouched imported layout is byte-preserved where promised. Conversion
   compares normalized rule/feature graphs, then shapes a corpus using the
   same outlines/glyph order before and after. Corpus agreement alone is not
   proof of equivalence for arbitrary text.
5. Shape comparisons include glyph names (map GIDs), clusters, x/y advances,
   x/y offsets, feature values, script, language and direction; include ligatures,
   marks, contextual rules, RTL and stored vertical rules. Add a low-level test
   helper where the current boolean-only feature API cannot express alternates.
6. Lifted kerning passes the P2a zero/remove/group/undo workflow plus mark,
   subtable-order and script/language tests. Unrelated features stay equivalent.
   Lossless mark/mkmk anchor lifting and rejected-lift source fallback also pass
   preview/export/reopen and undo/revert checks.
7. FCF save/reopen, UFO source export/reimport, TTF/WOFF/WOFF2 export/reopen and
   revert preserve the active semantics and planned draft/provenance policy.
8. Run focused failing tests first, then relevant Rust/frontend suites, generated
   API checks, lint/formatting, headless workflows and independent review. The
   final acceptance report records actual counts and corpus coverage, not the
   old P2a counts. Performance/desktop/FontForge work stays paused.

## 10. References and implementation evidence

Format semantics were checked against the primary
[OpenType common layout structures](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2),
[GPOS specification](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos),
and [Adobe FEA specification](https://adobe-type-tools.github.io/afdko/OpenTypeFeatureFileSpecification.html).
The [JSTF specification](https://learn.microsoft.com/en-us/typography/opentype/spec/jstf)
defines the cross-table lookup-index dependency relevant to conversion blocking.
These describe format behavior; our support claims must additionally be proven
against the locked fea-rs/fontc implementation. In particular, inspect fontbe
`features/kern.rs` for generated script/flag/zero-value behavior and fea-rs table
builders for representability; specification syntax alone is not sufficient.

See the [implementation plan](../plans/2026-09-17-p2b-editable-layout.md) for
dependencies, bounded task files and the first implementation step.
