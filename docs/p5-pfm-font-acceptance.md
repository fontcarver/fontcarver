# PFM font adapter acceptance

Implemented metrics-only PFM path import/export in
`crates/fc-formats/src/metrics/pfm/font_adapter.rs`. The codec owner provides
bounded binary parsing/writing and public reexports. Imported raw names,
60-byte copyright, charset, header and extended metrics survive in JSON
metadata. `pfmXX` glyphs retain byte codes through renames; no Unicode is
guessed. New fonts require unique ASCII assignments, Windows ANSI charset 0,
and the codec's 1000-unit PostScript subset. Display names use a Latin-1 byte
fallback; edited names must be ASCII.

Selected-master widths, explicit kerning, names and mapped global metrics are
updated. Fixed-pitch edits validate final advances, optional imported underline
and strikeout fields clear to zero, and unrelated imported scalar fields remain
intact. Unsupported layout, class/vertical kerning, vertical metrics, invalid
byte assignments and numeric overflow fail before atomic destination replacement.

Eleven focused integration tests cover untouched semantics/raw names, edits and
renames, new sparse ASCII fonts, excluded glyphs/deleted pairs, atomic failures,
fixed-pitch transitions and consistency, optional metric removal and vertical
metric rejection. Tests were written before their implementation; the initial
command was blocked by concurrent `fc-model` edits (`Layer.metrics_keys` E0560).

Validation: narrow `rustfmt --edition 2021 --check` passed. The focused
`cargo test -p fc-formats --test pfm_font` retry was cancelled at root request
while still waiting for the shared Cargo lock (owned flock PID 2264095, no child;
SIGTERM, exit 143). This is interrupted validation, **not a passing test run**.
Root owns execution of all 11 tests and workspace Clippy/regression checks.
No new dependencies or FontForge implementation used. Owned scratch removed;
final artifact check passed (build 6.10 GiB; temporary 0.21 GiB; 668.75 GiB free).
