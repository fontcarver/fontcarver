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
