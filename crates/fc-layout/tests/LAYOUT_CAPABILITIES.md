# Static imported layout capability matrix

P2b conversion is a whole-document gate: checked inventory, complete FEA source,
actual pinned `fontc` compilation for every master, canonical graph comparison,
and supplementary full shaping comparisons. Parsing or a matching text sample
alone does not authorize conversion. All original tables remain authoritative on
any rejection. No FontForge implementation or executable is used.

## Accepted representations and executable evidence

| Table / kind | Static representation | Actual compiler / graph-proof coverage |
| --- | --- | --- |
| GSUB 1 | Formats 1/2, checked delta arithmetic and explicit replacements | `gsub::tests::whole_gsub_graph_roundtrips_simple_types_through_actual_binary_writer` |
| GSUB 2 | Format 1, multiple outputs and deletion; mixed single/multiple-output lookups | Same simple graph fixture; singleton-only lookup retyping is explicitly rejected by a real-writer regression |
| GSUB 3 | Format 1, ordered alternates | Same simple graph fixture; conversion shaping helper accepts numeric feature values |
| GSUB 4 | Format 1, ordered ligature components and preference | Same simple graph fixture; unsupported compiler reordering rejected |
| GSUB 5/6 | Formats 1/2/3, glyph/class/coverage contexts, class-zero complements, ignore rules, nested/shared references | `whole_context_graph_roundtrips_all_six_formats_for_substitution_and_positioning` (six formats × two tables), `whole_context_graph_preserves_forward_and_nested_dependencies`, `ignore_and_repeated_position_actions_keep_binary_execution_semantics` |
| GSUB 7 | Format 1 wrapping any accepted non-extension kind | `layout_inventory::gsub_and_gpos_extension_wrappers_preserve_flags_and_shaping` |
| GSUB 8 | Format 1, reverse chaining, backtrack reversal and explicit replacements | Simple whole-graph fixture and raw reverse fixture |
| GPOS 1 | Formats 1/2, x/y placement and x/y advance, explicit zeros | `gpos::tests::compiler_roundtrip_single_and_pair_preserves_full_shaping` plus raw both-format fixtures |
| GPOS 2 | Formats 1/2, class-zero complements, explicit-zero masking, absent versus encoded-zero second value records | Same full compiler/graph/shaping fixture, `class_pairs_also_preserve_encoded_zero_second_records` |
| GPOS 3 | Format 1, cursive entry/exit XY anchors, null entry/exit | `compiler_roundtrip_cursive_and_all_mark_attachments_preserves_full_shaping` |
| GPOS 4/5/6 | Format 1, mark-to-base/ligature/mark, sparse anchor classes and null ligature components | Same full compiler/graph/shaping fixture; numeric class order >10 fixture |
| GPOS 7/8 | Formats 1/2/3 contextual/chaining positioning | The twelve-context whole-graph fixtures above |
| GPOS 9 | Format 1 wrapping any accepted non-extension kind | Extension whole-graph fixture above |
| Lookup graph | Shared references, ordered execution across enabled features, flags, required features, scripts/languages | `layout_import` registration tests; distinct equal-body lookup sharing regression in `compare::tests` |
| GDEF | Glyph classes, coordinate ligature carets, used mark attachment classes/filter sets | `gdef::tests` pinned compiler roundtrip and `layout_inventory` dependency fixtures; GPOS full gate compares GDEF too |
| BASE | Horizontal/vertical baseline tags, default baseline and ordered script coordinates | `base::tests` pinned compiler roundtrip and `layout_inventory::base_only_binary_import_owns_layout_and_roundtrips_without_gsub_or_gpos` full gate |

Both original and candidate shaping records include names, clusters, advances and
offsets. Positioning tests cover LTR, RTL and TTB. Source generation interns
repeated glyph sets and orders lookup declarations by dependencies; the complete
canonical proof still rejects any resulting change in feature execution order.
Equal-body lookups retain distinct graph identities, because shared execution
and two independent executions can produce different positioning.

## Explicit rejection / conservative fallback

| Category | Reason and checked fixtures |
| --- | --- |
| Legacy `kern`, AAT `kerx`/`morx`/`mort`, JSTF | Competing authority or lookup-index dependency; actual legacy-only sfnt import and no-mutation inventory fixtures |
| GSUB/GPOS FeatureVariations, GDEF/BASE variation stores | Variable layout is outside P2b; table-specific rejection fixtures |
| FeatureParams | Unsupported compiler-preservation semantics; reports table/feature tag |
| GPOS device/value variation fields | No proven representation; rejects even encoded field presence with zero values |
| GPOS contour-point anchors | Recompiled outline point identity is unproven; reject format 2 |
| GPOS anchor format 3 | Accept only both device offsets zero; reject present device data |
| GDEF attachment/point carets/device carets | No output point mapping or accepted device preservation; reject |
| BASE MinMax, point/device coordinate references | Reject; format 3 with null device offset normalizes to a static coordinate |
| Singleton-only GSUB2 lookup | Pinned FEA compiler rewrites it as GSUB1; no proven cross-kind normalizer, so canonical gate rejects |
| Empty alternate sets, empty context sets, unreachable lookups/features, empty scripts/LangSys, empty subtables | Pinned compiler cannot retain their stored structure or fallback semantics |
| Descending contextual application indices | FEA cannot retain the original reference execution ordering |
| Ligature preference changed by compiler sorting | Reject before conversion |
| Empty mark classes; all-null base/mark rows | Compiler cannot retain coverage; sparse nonempty classes and null ligature components remain supported |
| Unused GDEF mark dependency records | Would be dropped by FEA compilation; reject |
| Nonzero values in unreachable PairPos class matrix cells | Unrepresentable stored rules; all-zero unused matrix padding is harmless |
| Unknown versions/formats, bad offsets/counts/GIDs, unsorted records, cycles | Checked readers and graph validation reject without mutation |
| Unsafe original glyph provenance / old contradictory authority | Conversion rejects; valid saved conversion backup permits original-table inspection |
| Resource limits | ≤32 MiB/table, ≤4096 lookups/features/scripts/languages, ≤100000 rules/subtables/feature references, ≤1.6M expanded glyph references, ≤16 MiB intermediate rule text, ≤1 MiB final editable source, depth ≤64, glyph names ≤127 bytes |
| Canonical mismatch after compilation | Source-only conversion also rejects. There is no “convert anyway” option; corpus agreement cannot override structural failure |

Malformed and shared-offset amplification fixtures are in `layout_inventory` and
`context`; every truncated prefix is tested for representative GSUB/GDEF/BASE
and context tables. Inspector pages expose table headers and rejection reasons
for unsupported layouts. Individual rule previews are capped at 4096 characters
and 64 glyph names; full checked references are paged separately.

## Existing licensed corpus outcome

The regression `layout_import::licensed_real_font_inventory_reports_actual_coverage_without_mutation`
uses the repository's existing licensed fixtures; no new binary fixture is added.

| Original fixture | Actual outcome |
| --- | --- |
| SourceSans3Min / SourceSerif4Min | Blocked: retained empty language/script structures and `size` FeatureParams |
| NotoSans-Regular | Blocked: `ss03` FeatureParams; unconverted GPOS rendering may also exceed the source limit |
| NotoSerifTibetan-Regular | 1182 lookups parsed; complete source compiles after class interning. GPOS canonical equivalence is conservatively unproven, so conversion is rejected |
| DejaVuSans | Blocked: legacy `kern`, a contour-point anchor, and unused mark attachment dependencies |

These corpus rejections are preservation results, not evidence of successful
conversion. Positive conversion coverage comes from compiler-built synthetic
fixtures covering each accepted format, plus model-writer kerning and mark/mkmk
lift roundtrips in the conversion tests. Support does not mean every font using
one of these lookup numbers is convertible; every subfield and the whole graph
must pass the gate.
