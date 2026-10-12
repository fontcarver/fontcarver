You are implementing **P1 Task 4: `fw-formats` — UFO 3 + designspace import/export** for FontWorkshop (Tauri 2 font editor: Rust kernel + React/TS UI). This directory is the git worktree for branch `p1-t4`. Create the new crate `crates/fw-formats` (workspace member) and the fixture generator in `crates/fw-fixtures`; do not modify other crates except root `Cargo.toml`/`Cargo.lock` and `crates/fw-fixtures`. Registration in `fw-doc::Session::open_path` and in the export path belongs to Task 5 — expose the functions only.

## Read first
1. `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-4-brief.md` — requirements; interface block normative.
2. `CONVENTIONS.md` (binding); model: `crates/fw-model/src/` (`Font`, `FontInfo` — its field names follow UFO fontinfo.plist deliberately, `Master`, `MasterMetrics`, `PsPrivate`, `Glyph`, `Layer`/`LayerKind`, `Contour`/`Point` (UFO point semantics), `Component`, `Anchor`, `Guide`, `Kerning`/`KernSide`, `KerningGroups`, `Features`/`FeatureBlock`, `Extra.lib`).
3. `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/research/rust-crate-spike.md` §2.g (norad 0.18: `Font::load_requested_data`, `LayerContents`, private fields with accessors, `Kerning`/`Groups` shapes, `FontInfo` 108 fields, `designspace` module `DesignSpaceDocument::load/save`, `Font::save` writes UFO 3 only and DELETES the target directory — hence our incremental writer).

## Decisions (do not revisit)
- Dependencies: `norad = { version = "0.18", features = ["kurbo", "rayon"] }`, `plist`, `serde`, `thiserror`, `tracing`, `walkdir`; add to workspace deps.
- Reading: the UFO's default layer (`public.default`) → the master layer; other layers → `LayerKind::Custom(name)` except `public.background` → `Background(master)`; glif point kinds map 1:1 (`move/line/curve/qcurve/offcurve`, `smooth`), identifiers/names preserved (`Point.name`, `Contour.id`), component `base`/transform, anchors, guidelines (glyph and font level), images (path + transform, data loaded lazily → store file name only), lib per glyph → `Glyph.user_data["ufo.lib"]`; `groups.plist` → `KerningGroups` (`public.kern1.`/`public.kern2.` prefixes → side1/side2; other groups → `Extra.lib["ufo.groups"]`), `kerning.plist` → `Master.kerning_h` with `KernSide::Group` for `public.kern*` names; `features.fea` → `Features { features: [FeatureBlock { tag: "fea", code, automatic: false }], source_of_truth: Fea }`; `fontinfo.plist` → `FontInfo` (explicit field-by-field mapping, all 108; unmapped → `FontInfo.user_data["ufo.fontinfo"]`) + `MasterMetrics` (ascender/descender/xHeight/capHeight/italicAngle + openTypeHhea*/openTypeOS2Typo*/Win*/openTypeVhea*) + `PsPrivate` (postscriptBlueValues…); `lib.plist` → `Extra.lib["ufo.lib"]` with `public.glyphOrder` driving glyph order; `metainfo.plist` creator/formatVersion recorded in `Extra.lib["ufo.metainfo"]`.
- Designspace: `DesignSpaceDocument::load`; axes → `Axis { tag, name, min/default/max, map }`; sources → masters (each source's UFO read; default source first; a source with `layer` set → brace layer on the corresponding master's glyphs); instances → `Instance`; rules ignored with a warning (`Extra.lib["ufo.designspace.rules"]` keeps them raw).
- Writing (`write_ufo`): build a `norad::Font` in memory from the model (inverse mapping; extras the UFO can't hold → `lib["dev.fontworkshop.extra"]` as `{ "version": 1, "raw_tables": {tag: base64}, "bitmap_strikes": …, "tt_instructions": {glyph: base64}, "hints": … }`), then write **incrementally**: serialise each file to bytes, compare with the existing file, write only when different (atomic temp + rename), remove stale `.glif`s listed in the old `contents.plist` but not in the new one; never delete the directory. `write_designspace` writes `<family>.designspace` + one UFO per master (`<family>-<master>.ufo`).
- Fixture: `fw_fixtures::ufo::write_minimal(dir)` builds a 3-glyph UFO (A with two contours, B with a component + anchor, space) with groups/kerning/features/lib via norad; run it once to commit `fixtures/ufo/Minimal.ufo` (deterministic output; add a test asserting regeneration is byte-identical).
- Commit on branch p1-t4 with Conventional Commits; do not push.

## Report contract
Full report to `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-4-report.md`. Final message under 15 lines: Status, commits, test summary, concerns, report path.

## Verification you must run before reporting
```
cargo fmt --all --check
cargo clippy -p fw-formats -p fw-fixtures --all-targets -- -D warnings
cargo test -p fw-formats -p fw-fixtures
```
Do not spawn other agents; do exactly this task's scope.
