# Font Cutter

A cross-platform desktop font editor: a Rust kernel behind a React/TypeScript UI,
packaged with [Tauri 2](https://tauri.app/) for Linux, macOS and Windows.

Project home: <https://fontcutter.com>

Font Cutter is a modern re-implementation of the capabilities that made
FontForge the reference free-software font editor, built for the workflow and
polish people expect from Glyphs 3, FontLab 8 and FontCreator.

## What it does

- **Vector and bitmap fonts are both first-class.** Cubic and quadratic outline
  editing and bitmap-strike editing live in the same document.
- **Latin and CJK are both first-class.** The glyph grid, search, batch
  operations and file I/O are designed for 65,535-glyph fonts; components,
  vertical metrics and vertical layout features are core, not add-ons.
- **Wide format support.** TTF, OTF (CFF/CFF2), TTC/OTC, WOFF, WOFF2, variable
  fonts, UFO 2/3 + designspace, Glyphs 2/3, SFD, Type 1, SVG, BDF/PCF/FNT/FON/PSF,
  embedded bitmap tables, FEA, AFM/PFM and TTX.
- **Full OpenType layout.** GSUB/GPOS/GDEF/BASE/MATH/STAT/COLR/CPAL, edited as
  FEA source or through the UI, compiled with `fea-rs`/`fontc`.
- **Lossless where you did not touch anything.** Tables the model does not
  represent are preserved verbatim, and untouched glyphs keep their hinting.

The font logic is built on the permissively licensed Rust font ecosystem:
[fontations](https://github.com/googlefonts/fontations) (`read-fonts`,
`write-fonts`, `skrifa`), `fontc`, `kurbo`, `flo_curves`, `harfrust`, `norad`,
`tiny-skia` and friends.

## Status

Early development. The kernel crates, the Tauri shell and the font/glyph views
exist and are under active work (outline editing is the current phase); there is
no release yet and formats and APIs still move.

## Layout

```
crates/       Rust kernel: model, geometry, formats, sfnt, layout, compile, api, cli
src-tauri/    Tauri 2 desktop shell (thin wiring layer)
app/          React 19 + TypeScript + Vite frontend
fixtures/     test fonts
docs/         specs, plans, research, design
```

## Documentation

- [`docs/superpowers/specs/2026-09-03-fontcutter-design.md`](docs/superpowers/specs/2026-09-03-fontcutter-design.md)
  — the binding design specification: product definition, architecture, document
  model, crate boundaries.
- [`docs/design/editor-ui.md`](docs/design/editor-ui.md) — the editor UI design
  specification: design tokens, interaction grammar, tools, panels.
- [`CONVENTIONS.md`](CONVENTIONS.md) — engineering conventions every contributor
  (human or agent) follows.
- [`docs/licensing.md`](docs/licensing.md) — licensing policy and the FontForge
  relationship.
- [`THIRD_PARTY.md`](THIRD_PARTY.md) — third-party notices, fixture licenses and
  the dependency license survey.

## Building

Requires Rust (the version pinned in `rust-toolchain.toml`), Node 24+ and pnpm,
plus the usual Tauri system dependencies (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev` on Debian/Ubuntu).

```sh
pnpm install
cargo test --workspace     # Rust kernel
pnpm --dir app check       # TypeScript
pnpm tauri dev             # run the desktop app
```

## License

Font Cutter is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### Relationship to FontForge

Font Cutter is a from-scratch implementation, not a port of FontForge. FontForge
is studied as a *behavioural and format reference* — what a mature editor does,
and how the file formats it reads and writes actually behave — and the findings
are written up in `docs/research/`. FontForge is distributed as a whole under the
GPLv3-or-later, but the large majority of its source, written by George Williams
between 2000 and 2012, carries the 3-clause ("revised") BSD license; only files
added later are GPL-only. Font Cutter therefore may adapt code *only* from
FontForge files whose own header is the revised BSD license, always with
attribution; anything that traces to a GPL-only file is re-implemented from the
format specification or from observed behaviour, without copying. Font Cutter
never links against, embeds or ships FontForge, and the `fontforge/` reference
checkout used during research is gitignored and is not part of this project.

The full reasoning and the rules contributors follow are in
[`docs/licensing.md`](docs/licensing.md); every adaptation and every third-party
notice is recorded in [`THIRD_PARTY.md`](THIRD_PARTY.md).

Font Cutter is not affiliated with, endorsed by, or sponsored by the FontForge
project. "FontForge" is used here only to describe that project.
