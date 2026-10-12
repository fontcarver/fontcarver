Review complete. Findings below.

## Spec Compliance

| Requirement | Verdict |
|---|---|
| `read_ufo` / `read_designspace` / `write_ufo(…, &UfoWriteOptions{incremental})` / `write_designspace` signatures | ✅ exact (`crates/fw-formats/src/ufo/mod.rs:36-62`) |
| No crates touched except `fw-formats` (new), `fw-fixtures`, root manifests | ✅ |
| Point kinds/smooth/names, contour ids, components, anchors, glyph+font guides, images (file name only), per-glyph lib | ✅ verified by norad-level equality probe |
| Groups → `KerningGroups` (`public.kern1/2`), other groups preserved | ✅ (`convert.rs:579-628`) |
| Kerning → `Master.kerning_h` with group sides | ✅ |
| `features.fea` → `Features`, `source_of_truth: Fea` | ✅ |
| fontinfo → `FontInfo`+`MasterMetrics`+`PsPrivate`, unmapped preserved | ✅ 90/108 fields explicit; the other 18 (`year`, `woff*`, `styleMap*`, `postscriptWeightName`, …) have no `fw-model` field and survive losslessly through `user_data["ufo.fontinfo"]`, which is also the write baseline. Verified round-trip |
| `lib.plist` + `public.glyphOrder` ordering, `metainfo` preserved | ✅ custom `creator` survives (`writer.rs:45-47`) |
| Identifiers under `dev.fontworkshop.ufo.identifiers` | ✅ per-master copy in `master.user_data`, correct for designspace |
| Extras blob (raw tables, TT instructions, hints, strikes) versioned | ✅ verified round trip of all six sub-keys |
| Writer never deletes target; atomic temp+rename; stale glifs via old `contents.plist` | ✅ verified; no-op second write touches **zero** files |
| Incremental proven by a real test | ✅ real mtimes, real filesystem |
| Fixture committed + deterministic regeneration test | ✅ `regeneration_is_byte_identical` passes |
| Designspace: axes+maps, default source first, instances, rules kept raw w/ warning | ✅ |
| `layer` sources → brace layers | ❌ imported, but duplicated (below) |
| Robustness: malformed → `Err`, never panic | ✅ 8 malformed/edge inputs, all `Err`, no panic |
| Conventions (docs, `#![warn(missing_docs)]`, no unwrap/expect, fmt, clippy `-D warnings`) | ✅ |

`cargo test -p fw-formats -p fw-fixtures`: 9 pass. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean. DejaVu (6,253 glyphs) write 1.4 s / read 2.3 s debug.

## Strengths

- The writer is the best part: staging dir, per-file byte compare, atomic rename, `StagingGuard` RAII cleanup, stale-glif removal driven by the old `contents.plist`. A no-change rewrite touches nothing and unrelated files/dirs survive.
- Lossless fontinfo by construction — keeping the raw plist and using it as the write baseline is the right call given `fw-model` can't be modified in this task.
- `value.rs` plist↔JSON bridge correctly preserves data/date/uid and the real-vs-integer distinction (verified with a plist containing all of them).
- Tests assert real behaviour throughout — actual mtimes, actual norad re-reads, a 6k-glyph font.

## Issues

#### Important

1. **`write_designspace` output is unreadable for any font without axes** — `location_to_norad` (`designspace.rs:385-394`) maps over `font.axes`, so an axis-less font emits `<location/>`, and `read_designspace` on that file fails with `missing field 'dimension'`. Since `read_ufo` and `fw-sfnt` both produce axis-less single-master fonts, this breaks the most common export→import path.

2. **Brace-layer import duplicates the layer, and it compounds** — the master's UFO is imported whole (so the `{650}` layer arrives as `LayerKind::Custom`), then `import_brace_layer` (`designspace.rs:213-253`) loads the same UFO again and pushes a second, identical `LayerKind::Brace`. Probe: glyph `A` gets `[Master, Custom{650}, Master, Brace]` on read; after one export+re-read it is `[Master, Custom{650}, Master, Custom{650}, Brace]` — one extra copy per cycle.

3. **Custom layers leak into every master's UFO** — `layer_belongs` returns `true` for `LayerKind::Custom(_)` regardless of master (`convert.rs:726`). Probe: exporting a 2-master font where only Regular had `{650}` produced `Test-Bold.ufo/glyphs.{650}` with Regular's outlines.

4. **Every exported master UFO gets the default master's fontinfo** — `to_norad` passes the font-wide `font.info` (`convert.rs:187`), so the Bold UFO is written with `styleName = Regular`. Conversely, on import `merge_master` (`designspace.rs:255-264`) drops each non-default source's `info`, raw `ufo.fontinfo` and guides, so per-master fontinfo is lost in both directions.

5. **UFO `data/` and unreferenced `images/` are dropped on export to a new directory** — `to_norad` builds a fresh `norad::Font` (`convert.rs:181`) and never copies `source.data`. Probe: a UFO with `data/com.example/state.json` and `images/unreferenced.png` round-tripped to a new path lost both. They survive an in-place rewrite only because the writer doesn't delete. This contradicts the report's "non-model UFO metadata remains opaque but lossless".

6. **The extras blob is duplicated in the model after import** — `extra::import` (`extra.rs:220-230`) decodes `dev.fontworkshop.extra` into first-class fields but leaves the JSON copy inside `extra.lib["ufo.lib"]`. For DejaVu that is **2.5 MB of redundant JSON** alongside the decoded data, which any `.fwf` save would persist. `read_ufo(write_ufo(f))` is therefore not equal to `f`. A `remove` after decoding fixes it.

7. **Duplicate master names collide on one filename** — `write_designspace` (`designspace.rs:126-130`) derives the UFO name solely from `master.name` with no deduplication. Two masters named "Regular" both write `Minimal-Regular.ufo`; the second silently overwrites the first's glyph data and both `<source>` elements point at the same file.

#### Minor

- Non-default layers gain the default layer's `unicodes` and `note` on export (`convert.rs:331`, `:337`). Probe: a background glyph with no codepoints and no note came back with `{'A'}` and `Some("hello note")`. Additive only, but it dirties every background `.glif` on first export.
- `dev.fontworkshop.extra` is inserted unconditionally (`convert.rs:216-218`), so re-saving a pristine third-party UFO always adds a FontWorkshop key to `lib.plist`.
- `tt_indices` is encoded as one plist dict per point; DejaVu's `lib.plist` comes out at **12.4 MB** for a 700 KB font. A packed/base64 encoding would be far cheaper.
- Designspace instance `filename`, `postscriptfontname`, `stylemapfamilyname`/`stylemapstylename` are dropped even though `Instance.user_data` is free (`designspace.rs:396-436`); axis `label_names` and discrete-axis `values` are likewise dropped on export (`axis_to_norad` sets `values: None`). The report's "min/max envelope" note covers the import side only — the export side loses discreteness entirely, and neither is recorded in `extra.lib`.
- `convert.rs` is 828 lines, over the ~800-line module guideline in CONVENTIONS.md.
- The DejaVu perf budget asserts `< 60 s` (`ufo_write.rs:198`) against an actual 1.4 s — too loose to catch a regression.
- `fw_fixtures::ufo::write_minimal` calls `norad::Font::save`, which deletes the target directory; the doc comment doesn't say so.
- Guides at angle 0/90 are written as `Line::Horizontal`/`Vertical`, discarding the orthogonal coordinate (`info.rs:376-390`); `Line::Angle { degrees: 0.0 }` would preserve it.

## Assessment

Task quality: **Needs fixes**. The single-master UFO path is genuinely strong — the incremental writer, the fontinfo strategy and the extras round trip all hold up under adversarial probing — but the designspace half has a hard round-trip failure for axis-less fonts, layer duplication that compounds on every cycle, and cross-master contamination of layers and fontinfo, all inside the brief's stated scope.