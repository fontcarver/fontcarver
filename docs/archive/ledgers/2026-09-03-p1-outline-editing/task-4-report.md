# P1 Task 4 Report — `fw-formats`

Status: Complete.

Commit: `edbda79 feat(fw-formats): add UFO 3 and designspace import export`.

Implemented the new `fw-formats` workspace crate with the normative `read_ufo`, `read_designspace`, `write_ufo`, and `write_designspace` interfaces. UFO conversion covers ordered glyphs, all layers, UFO point semantics, components, anchors, font/glyph guides, lazy image references and image copying on export, groups, per-master kerning, feature text, first-class fontinfo/master/private fields, and complete raw fontinfo/lib/metainfo preservation. Object identifiers and object libs are retained in `dev.fontworkshop.ufo.identifiers`; non-UFO model data is base64/serde encoded under `dev.fontworkshop.extra`.

The UFO writer stages canonical norad output and atomically synchronizes files without deleting the target directory. Identical files retain mtimes, stale GLIFs are removed from old/new `contents.plist` differences, and unrelated files remain untouched. Designspace import orders the default source first, imports per-source masters/kerning, axes/maps/instances and brace layers, and preserves ignored rules with a warning. Export writes one UFO per master plus the family designspace and brace-layer sources.

Added `fw_fixtures::ufo::write_minimal`, the committed deterministic `fixtures/ufo/Minimal.ufo`, a regeneration identity test, and integration coverage including a 6,253-glyph DejaVu round trip.

Verification:

- `cargo fmt --all --check` — passed.
- `cargo clippy -p fw-formats -p fw-fixtures --all-targets -- -D warnings` — passed.
- `cargo test -p fw-formats -p fw-fixtures` — passed: 9 tests, 0 failures; doctests passed.

Not done: `fw-doc::Session::open_path` and export-path registration, reserved for Task 5.

Concern: fields absent from `fw-model` remain lossless opaque UFO metadata rather than first-class editable values; discrete designspace axes are represented by their min/max envelope because the model has no discrete-axis flag.

## Fix round 1

Status: all seven Important findings and requested Minors fixed, regression-tested first.

1. Axis-less export now writes `<Family>-<Master>.ufo` and returns `Error::NoAxes`; static-font routing is documented (`error.rs:52`, `ufo/mod.rs:37-63`, `designspace.rs:153-160`). Test: `axisless_fonts_are_written_as_ufo_instead_of_an_invalid_designspace`.
2. Designspace UFOs are cached and brace sources convert the already-imported owned layer in place, eliminating cycle duplication (`designspace.rs:54-66,262-294`). Test: `brace_layers_do_not_duplicate_across_two_round_trips`.
3. Imported custom layers record `ufo.master`; export selects that owner, with ownerless layers limited to the default master (`convert.rs:327-336,618-631`). Test: `custom_layers_are_exported_only_with_their_owning_master`.
4. Every master retains raw `ufo.fontinfo`, metrics, guides, and exports from its own baseline with `master.name` as style (`convert.rs:30-46,190-198`; `info.rs:171-191`). Test: `each_master_preserves_and_exports_its_own_fontinfo`.
5. Export copies all source `data/` and `images/` entries, including unreferenced images, to new or in-place UFOs (`extra.rs:14-69`). Test: `copies_data_and_unreferenced_images_to_new_ufo_and_preserves_them_in_place`.
6. Decoded extras are removed from raw `ufo.lib`; empty extras are omitted; TT indices and component-only metadata use packed encodings (`extra.rs:72-289,292-438`). Extras round-trip all six payload classes; DejaVu `lib.plist` is <1 MB (`ufo_write.rs:219-314,419-456`).
7. Colliding master names receive deterministic `-2`, `-3` suffixes (`designspace.rs:163-176`). Test: `duplicate_master_names_get_distinct_ufo_filenames`.

Minors: non-default Unicode/note suppression (`convert.rs:350-360`); instance/axis metadata preservation (`designspace.rs:119-141,353-389,457-526`); `convert.rs` split to 731 lines via `structure.rs`; 10 s DejaVu budget; destructive fixture behavior documented (`fw-fixtures/src/ufo.rs:13-16`); orthogonal guides use `Line::Angle` (`info.rs:368-373`).

Verification output:

- `cargo fmt --all --check` — passed.
- `cargo clippy -p fw-formats -p fw-fixtures --all-targets -- -D warnings` — passed, 0 warnings.
- `cargo test -p fw-formats -p fw-fixtures` — passed: 19 tests, 0 failures; doctests passed.
