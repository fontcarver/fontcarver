# P5 PFM font adapter

Scope: `crates/fc-formats/src/metrics/pfm/font_adapter.rs` and
`crates/fc-formats/tests/pfm_font.rs`; parent owns codec and reexports.

Read/write metrics-only PFM paths using `PfmMetrics`. Imported fonts retain raw
PFM byte strings, header, extended metrics, charset and glyph byte assignments
in JSON metadata. Display names use a reversible Latin-1 byte mapping, without
claiming it is the file's encoding; new/edited names accept ASCII. Imported
byte slots use `pfmXX` glyph names and no guessed Unicode assignments. New
fonts support unique Windows ANSI ASCII assignments only. Widths and explicit
pair kerning follow the selected master; unsupported layout/class kerning and
out-of-range values fail before atomic destination replacement. Preserve
original fields unless their corresponding editable model value changed.

Acceptance: write tests first for unchanged semantic round trip, raw-byte names
and unusual charset, edited names/widths/kerning/metrics, new ASCII metrics,
invalid assignment/layout/class/numeric failures, and destination preservation.
Run focused `cargo test -p fc-formats --test pfm_font` under the shared Cargo
lock with task TMPDIR. Parent performs broad checks. No FontForge source or
new dependencies. Check artifact budgets at start/end; retain evidence in
`docs/p5-pfm-font-acceptance.md` and remove owned scratch outputs.
