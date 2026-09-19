# FontCarver

A cross-platform desktop font editor: a Rust kernel behind a React/TypeScript UI,
packaged with [Tauri 2](https://tauri.app/) for Linux, macOS and Windows.

Official website: <https://fontcarver.com>

Repository: <https://github.com/fontcarver/fontcarver>

The product name is **FontCarver** (also written **Font Carver**); package and
command names use `fontcarver`.

Existing `.fcf` documents and UFO metadata from earlier versions remain readable.
On first launch, the default `fontcutter` configuration directory (preferences,
recent files and recovery snapshots) moves to `fontcarver` if that directory does
not already exist. Explicit `FC_CONFIG_DIR` paths are used as supplied.

FontCarver is a modern re-implementation of the capabilities that made
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

Early development. P1 outline editing and headless acceptance are complete,
and the user has tried and accepted the P1 desktop version. P2a horizontal
kerning authoring is complete for eligible source fonts: shared groups,
per-master pair rules, text gap editing, undo/redo, and matching preview/export
with headless acceptance. See
[`docs/p2a-acceptance.md`](docs/p2a-acceptance.md) for verified behavior and scope.
P2b editable OpenType layout is complete: persisted FEA drafts, located compiler
diagnostics, automatic features, imported lookup inspection, validated static
layout conversion and native restore, with proven kerning/anchor transfers.
Conversion remains conservative; unsupported or unproven layouts keep their
original tables. See [P2b acceptance](docs/p2b-acceptance.md) and the executable
[coverage matrix](crates/fc-layout/tests/LAYOUT_CAPABILITIES.md).
P2c/P2d are complete within the documented static scope: native OTF/CFF export,
Glyphs 2/3 import and Glyphs 3 export, SFD and Type 1 interoperability, and
Find problems through the UI and CLI. See [P2c/P2d acceptance](docs/p2cd-acceptance.md)
for format boundaries, independent reviews and final browser verification.
There is no release yet; native workflow verification is partial and release packaging is untested. See
[`docs/p1-acceptance.md`](docs/p1-acceptance.md) for measured results and current
limitations, including hinting removal after outline edits. The capability list
above describes the product target, not completed support for every format.

## Layout

```
crates/       Rust kernel: model, geometry, formats, sfnt, layout, compile, api, cli
src-tauri/    Tauri 2 desktop shell (thin wiring layer)
app/          React 19 + TypeScript + Vite frontend
fixtures/     test fonts
docs/         specs, plans, research, design
```

## Documentation

- [`docs/superpowers/specs/2026-09-03-fontcarver-design.md`](docs/superpowers/specs/2026-09-03-fontcarver-design.md)
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

FontCarver is licensed under either of

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

FontCarver is a from-scratch implementation, not a port of FontForge. FontForge
is studied as a _behavioural and format reference_ — what a mature editor does,
and how the file formats it reads and writes actually behave — and the findings
are written up in `docs/research/`. FontForge is distributed as a whole under the
GPLv3-or-later, but the large majority of its source, written by George Williams
between 2000 and 2012, carries the 3-clause ("revised") BSD license; only files
added later are GPL-only. FontCarver therefore may adapt code _only_ from
FontForge files whose own header is the revised BSD license, always with
attribution; anything that traces to a GPL-only file is re-implemented from the
format specification or from observed behaviour, without copying. FontCarver
never links against, embeds or ships FontForge, and the `fontforge/` reference
checkout used during research is gitignored and is not part of this project.

The full reasoning and the rules contributors follow are in
[`docs/licensing.md`](docs/licensing.md); every adaptation and every third-party
notice is recorded in [`THIRD_PARTY.md`](THIRD_PARTY.md).

FontCarver is not affiliated with, endorsed by, or sponsored by the FontForge
project. "FontForge" is used here only to describe that project.
