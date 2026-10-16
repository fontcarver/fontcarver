# FontCarver

A cross-platform desktop font editor: a Rust kernel behind a React/TypeScript UI,
packaged with [Tauri 2](https://tauri.app/) for Linux, macOS and Windows.

Official website: <https://fontcarver.com> (its source is
[fontcarver/fontcarver.com](https://github.com/fontcarver/fontcarver.com))

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

P0–P4 are accepted within their documented scopes. P5's professional-finish
checklist has **110 Complete / 2 Partial** rows. The integrated local milestone
passes 2,093 Rust tests, 2,914 frontend tests and all 105 default browser tests,
including the external 65,535-glyph CJK font and actual download workflows.
Real external `ttfautohint` execution and an independent English/Chinese review
are also complete. The latest browser walkthrough improved compact layouts,
editing gestures, shortcuts and focus; the welcome page now links directly to
getting-started help, command search and shortcut settings.
The later [200% zoom check](docs/p5ux-zoom-acceptance.md) fixes long command-palette
text, and the [menu repair](docs/p5ux-menu-zoom-acceptance.md) keeps Help and its
popup inside the window. Their eight targeted browser regressions pass together;
the complete 105-case run above belongs to the earlier milestone.
The subsequent [settings check](docs/p5ux-settings-zoom-acceptance.md) fixes
overflowing nudge fields and adds four passing browser regressions for Preferences
and Keyboard Shortcuts at zoom-reduced widths.

Actual Linux deb and thin AppImage candidates from `f6f6cf9` have been rebuilt and verified,
including isolated APT install/reinstall/removal and AppImage startup.
Native macOS/Windows execution, signing and publication remain pending; there
is no public release. A Complete checklist row includes the limits stated in
its acceptance report, rather than claiming exhaustive format or workflow
support. See [current status](docs/STATUS.md),
[integrated acceptance](docs/p5f-w3-integration-acceptance.md),
[remaining work](docs/TODO.md) and the [feature audit](docs/p5-feature-audit.md).

## Layout

```
crates/       Rust kernel: model, geometry, formats, sfnt, layout, compile, api, cli
src-tauri/    Tauri 2 desktop shell (thin wiring layer)
app/          React 19 + TypeScript + Vite frontend
fixtures/     test fonts
docs/         status, acceptance records, specs, plans, research, design, user docs
```

## Documentation

- [`docs/STATUS.md`](docs/STATUS.md) — where the project stands: phase status,
  work in progress, next step and a map of every other document.
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

Local Rust builds omit debug information and incremental caches to keep the
many integration-test executables from exhausting disk space. For debugger
sessions, opt in with `CARGO_PROFILE_DEV_DEBUG=1` or `CARGO_PROFILE_TEST_DEBUG=1`.
Run `python3 scripts/check_artifacts.py` before and after broad checks. The build
budget is 20 GiB; inspect and clear old development artifacts with
`cargo clean --profile dev` when necessary. Task scratch outputs belong under
`.scratch/<task-id>/` and are cleaned after each small task. See the resource
rules in [CONVENTIONS.md](CONVENTIONS.md) for retention and cleanup boundaries.

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
