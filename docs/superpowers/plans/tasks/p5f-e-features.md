# p5f-e: OpenType features and layout

Area id: `e-features`. Ports: API 8805, UI 1505. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                               | State at start                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| 46  | FEA parse/serialise round-trip into a typed lookup IR                                                                                                     | Source assembly and binary→typed conversion exist; a complete FEA→IR→FEA round trip for the supported lookup types is not established. |
| 47  | Prefix / Classes / Features tree with per-entry auto vs manual                                                                                            | `FeatureSourceTree`, generated-source protection; arbitrary per-entry auto/manual to verify.                                           |
| 48  | Automatic generation from glyph-name suffixes (liga, dlig, smcp, c2sc, ss01–ss20, onum/lnum/tnum/pnum, frac, ordn, sups/sinf/numr/dnom, locl, ccmp, aalt) | Seven generator families exist (`fc-model/feature_generation.rs`); the whole enumerated set to verify/implement.                       |
| 49  | Implicit kern/mark/mkmk/curs generated from kerning and anchors                                                                                           | Model-generated layout exists; verify all four tags and their precedence with hand-written source.                                     |
| 50  | Inline error diagnostics with jump-to-source; autocomplete; Alt-click class preview                                                                       | Diagnostics and completion exist (`docs/p5-feature-editor-diagnostics-acceptance.md`); Alt-click class preview to verify.              |
| 51  | [CJK] `vert`/`vrt2` generation and vertical alternates handling                                                                                           | Reusable evidence (P2b/P4); re-verify.                                                                                                 |
| 72  | Live HarfBuzz shaping in the editing canvas via an incrementally compiled GSUB/GPOS-only shaper font                                                      | Live shaping rebuilds a full font; see `tasks/p5-incremental-shaping.md`; a layout-only incremental shaper to implement/verify.        |
| 99  | Visual OpenType lookup editor for GSUB 1/2/3/4/6 and GPOS 1/2/4/6, round-tripping to FEA                                                                  | `VisualLookupEditor*`, `visualLookupParser`; see `docs/p5-visual-lookup-*-acceptance.md`; all nine types to verify.                    |
| 100 | Feature-code tokens (numbers, glyph properties, predicates) and `#ifdef VARIABLE` conditional blocks                                                      | `featureLanguage.ts`, tokens (`docs/p5-feature-tokens-acceptance.md`); `#ifdef VARIABLE` and predicates to verify.                     |

Existing records: `docs/p5-visual-lookup-authoring-acceptance.md`,
`docs/p5-visual-lookup-backend-acceptance.md`, `docs/p5-feature-editor-diagnostics-acceptance.md`,
`docs/p5-feature-tokens-acceptance.md`, `docs/p2b-acceptance.md`,
`docs/superpowers/plans/tasks/p5-visual-lookup-*.md`, `p5-99-visual-lookup-authoring.md`,
`p5-incremental-shaping.md`.

## Scope notes

- Row 46: parse FEA (via `fea-rs`) into the typed IR for the nine lookup
  types plus classes, languagesystems and lookupflags; serialise back; test
  semantic round trip (parse → serialise → parse gives an equal IR) on the
  fixture corpus and on real fonts' decompiled features; unsupported constructs
  are preserved as verbatim source blocks, never dropped.
- Row 47: each prefix/class/feature entry has an auto/manual switch; auto
  entries regenerate from the model, manual entries are user text; switching
  manual→auto asks before overwriting.
- Row 48: implement every listed feature generator from glyph-name suffixes
  and the glyph database; unit test each with a synthetic inventory; generated
  code appears as auto entries.
- Row 49: kern from kerning groups/pairs (including class pairs and
  exceptions), mark/mkmk from anchors (`_`-prefixed), curs from `entry`/`exit`
  anchors; precedence documented and tested against a manual `kern` feature.
- Row 50: Alt-click (and a keyboard equivalent) on a class name shows the
  member glyphs rendered.
- Row 72: an incremental shaper build that recompiles only GSUB/GPOS/GDEF
  (plus the minimal cmap/glyph-order shell) when features or kerning change,
  with a measured time budget in a test; falls back to the full build when
  outlines change; the canvas uses it live.
- Row 99: create/edit/delete rules for all nine lookup types visually; the
  editor writes FEA that parses back into the same IR; browser evidence for
  at least one rule per type.
- Row 100: `#ifdef VARIABLE` … `#else` … `#endif` blocks compile differently
  for static and variable exports; number tokens and glyph-property predicates
  resolve at compile time; errors have locations.

## Owned files

`crates/fc-layout/**`; `crates/fc-model/src/{features/**,features.rs,feature_draft.rs,feature_generation.rs}`;
`crates/fc-api/src/handlers/{features.rs,features_tests.rs,layout.rs,layout_conversion.rs,shaping.rs}`;
`crates/fc-api/src/types/{features.rs,layout.rs,layout_conversion.rs,shaping.rs}`;
`crates/fc-doc/src/command/features.rs`; `app/src/views/features/**`;
`app/src/views/text/{shapeCoordinator.ts,useTextShape.ts}` (incremental shaper client side);
`app/e2e/p2b-*.ts`, `app/e2e/p2bFixtures.ts`, `app/e2e/p2bSupport.ts`;
`docs/p5f-e-features-acceptance.md`; user docs for features.

Shared files (narrow patches only): `crates/fc-compile/src/**` (feature
compilation entry points; task f owns the crate), `crates/fc-model/src/{anchor.rs,kerning.rs}`,
`crates/fc-doc/src/fcf/schema.md`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`,
`app/src/shell/builtinCommands.ts`.

## Gap analysis

Recorded 2026-09-30 after reading the code and running the existing suites as a
baseline (all green: `fc-model` features 15 + 6, `fc-compile`
tokens/source/validation/generation/writers/shaping_incremental 34,
`fc-layout` 106, `fc-api` `features` 12 and `visual_lookup_authoring` 12 — the
last two were recorded as _interrupted_ by the earlier batch and are now
observed passing).

### Row 46 — FEA parse/serialise round trip into a typed lookup IR

_Works._ `Features::assemble_fea()` produces mapped FEA; `fc-compile`
validates it with `fea-rs`; `fc-layout::import` converts **binary** GSUB/GPOS
into a typed inventory (`LayoutLookup`/`LayoutRule`) and decompiles tables to
FEA text.

_Missing._ There is no FEA → typed IR → FEA path in Rust at all. The only FEA
parser that produces typed rules is the bounded TypeScript
`visualLookupParser`, which handles one rule per feature block and no
lookupflags, classes, lookup blocks or scripts/languages.

_Plan._ New owned module `crates/fc-layout/src/fea_ir/` (adds the existing
workspace `fea-rs` dependency to `fc-layout`): walk the public `fea-rs` CST
(`Node`/`Token`/`Kind` are public; `typed::*` accessors are `pub(crate)`, so the
walk is by `Kind`), build `FeaIr` = ordered statements covering
`languagesystem`, `@class` definitions, `markClass`, `lookupflag`,
`script`/`language`, named `lookup` blocks, `subtable`, aalt `feature` refs and
typed rules for GSUB 1/2/3/4/6/8 + `ignore sub` and GPOS 1/2/3/4/5/6 +
`ignore pos` (the nine required types plus cursive/mark-to-ligature/reverse,
which cost nothing extra and are needed by rows 49/48). Anything else is kept
as `Verbatim` with its exact original text, never dropped. Serialise back to
canonical FEA; test parse → serialise → parse IR equality on the FEA fixture
corpus **and** on the decompiled features of real fixture fonts.

### Row 47 — per-entry auto vs manual

_Works._ `FeatureSourceTree` shows Prefixes/Classes/Features plus an
`Automatic` section derived from `settings.generated`; generated entries are
read-only in the editor; a whole generator family can be frozen
(auto → manual) from `FeatureGenerationPanel`.

_Missing._ There is no per-entry switch: granularity is the generator family
(one checkbox drives all twenty stylistic sets), classes can never be
automatic, and manual → automatic is impossible from the entry at all, so
there is no "this will overwrite your text" confirmation.

_Plan._ Keep the family switches as the persisted recipe state and add
`manual_entries: Vec<String>` to `FeatureGenerationSettings`: an entry is
automatic iff its family is enabled and its tag/class name is not in
`manual_entries`. Add automatic **classes** (a recipe registry of
metadata-derived class names) so the switch is real for the Classes section
too. UI: an `Automatic` switch on every selected entry; switching to manual
keeps the generated text and adds the name to `manual_entries`; switching to
automatic asks for confirmation before the recipe overwrites the user text.
Prefixes and unrecognised tags show the switch disabled with the reason.

### Row 48 — generation from glyph-name suffixes

_Works._ Seven families: `smcp` (`.sc`), `case`, `vert`/`vrt2` (`.vert`),
`ss01`–`ss20`, `liga` (underscore names), `locl` and `dlig` (both explicit
mapping lists).

_Missing._ `c2sc`, `onum`, `lnum`, `tnum`, `pnum`, `frac`, `ordn`, `sups`,
`sinf`, `numr`, `dnom`, `ccmp` and `aalt` are not generated at all.

_Plan._ Move the rule builders into an owned `fc-model`
module (`features/generators.rs`) and call them from `fc-compile`'s existing
`generate_feature_source` through a narrow patch. New families:
`smcp`/`c2sc` split by the base glyph's case; figure styles
`lnum`/`onum`/`tnum`/`pnum`; positional figures `sups`/`sinf`/`numr`/`dnom`;
`frac` built from the numerator/denominator glyphs with a reverse-chaining
numerator lookup so any digit count works; `ordn` from `.ordn` glyphs and the
classic `ordfeminine`/`ordmasculine` rules after figures; `ccmp` from real
component structure (base + mark components → the precomposed glyph) plus
dotless-before-mark rules; `aalt` as canonical `feature <tag>;` references to
the alternate-producing features actually present. Each family gets a unit
test over a synthetic inventory, and the emitted source is compiled and shaped.

_Recorded limit._ Compound figure suffixes (`.tosf`, `.tonum`) are not
auto-assigned to two features at once, and `locl`/`dlig` stay explicit
mapping lists rather than being guessed from language suffixes (that is the
existing accepted P2b decision).

### Row 49 — implicit kern/mark/mkmk/curs

_Works._ `fc-compile`'s fontc source passes `Kern`, `Mark`, `Curs` and `Gdef`
writers in `Skip` mode gated by `writer_policy`; `feature_writers.rs` proves
`kern` and `mark` precedence against hand-written source and the
`# Automatic Code` marker.

_Missing._ `mkmk` and `curs` are never asserted, class-pair kerning with
group exceptions is not covered by a precedence test, and the precedence rules
are not written down for users.

_Plan._ An owned `fc-layout` integration test that shapes real compiled fonts
for all four tags, including group pairs with a glyph-pair exception, a
mark-to-mark stack and `entry`/`exit` cursive attachment, each against a
hand-written feature of the same tag with and without the marker. Document the
precedence in `docs/user/`.

### Row 50 — diagnostics, autocomplete, class preview

_Works._ Located diagnostics with jump-to-source, glyph/class/keyword
completion with a bounded dynamic loader, and Alt-click / Alt+Enter class
preview with nested classes and cycle detection (unit-tested).

_Missing._ Browser evidence only; the earlier task could not start a browser.

_Plan._ Headless browser verification of a diagnostic jump, a completion and an
Alt-click class preview on ports 8805/1505, plus a Playwright spec.

### Row 51 — `vert`/`vrt2`

_Works._ `VerticalForms` generates both tags from `.vert`; P4 covers vertical
metrics and compile.

_Missing._ No test shapes a vertical run to prove the substitution actually
fires in `vert`, and `.vrt2`-suffixed glyphs are not recognised.

_Plan._ Accept `.vrt2` as an additional source suffix for `vrt2` only, and add
a vertical shaping test (HarfRust vertical direction) that the generated
`vert` substitution applies and the horizontal run is unchanged.

### Row 72 — incremental GSUB/GPOS-only shaper

_Works._ `ShapingCompile` rebuilds only GSUB/GPOS/GDEF/BASE and keeps every
other table byte-identical; falls back to a full build when glyph identity,
cmap, metrics, outlines, UPM or metadata features change; `fc-layout`'s
debounced `Shaper` uses it, and the Text view/preview window shape live through
it. Five tests cover reuse, writers, invalidation and a 10 s loop budget.

_Missing._ The budget is not _comparative_ (it does not show the incremental
path is cheaper than a full compile), and there is no evidence the live canvas
takes the incremental path.

_Plan._ Add an owned comparative budget test on a larger synthetic font
(incremental refresh must be a fraction of the full compile and under a fixed
wall-clock budget) and assert the `fc-layout` cache's published font is the
incremental one after a feature-only edit.

### Row 99 — visual lookup editor for nine types

_Works._ `VisualLookupEditor` + `visualLookup(Parser)` create/edit/delete one
rule per feature body for all nine kinds, render canonical FEA, persist through
the existing draft/validate/apply/undo path, and compare shaping with the
feature off and on. Backend acceptance (`visual_lookup_authoring`, 12 tests)
passes here.

_Missing._ Browser evidence for at least one rule per type; the TS parser and
the new Rust IR could drift.

_Plan._ Playwright spec plus agent-browser screenshots covering one rule per
type, and a Rust test asserting the new IR parses the exact FEA the TS
serialiser emits for all nine kinds (shared fixture strings).

### Row 100 — tokens and `#ifdef VARIABLE`

_Works._ The model expands `$metric`, `${glyph:property}`, arithmetic,
`$customNumber`, `$[predicate]` and whole-line `#ifdef`/`#ifndef`/`#else`/
`#endif`, with bounded diagnostics mapped to original offsets; static and
variable compilation select their own branch. Tests pass.

_Missing._ The editor highlights `#ifdef VARIABLE` **as a comment** and gives
`$` tokens no highlighting or completion at all, so the dialect is invisible in
the only place it is authored.

_Plan._ Extend `featureLanguage.ts` with directive, token and predicate
highlighting and add the directives/token names to completion; unit-test the
stream tokenizer and show it in the browser.
