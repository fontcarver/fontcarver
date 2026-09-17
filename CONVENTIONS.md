# FontCarver Engineering Conventions

These rules apply to every contributor, human or AI agent. Task prompts reference this file; do not restate it, follow it.

## Repository layout

```
Cargo.toml            # workspace
crates/
  fc-model/           # document model (pure data + invariants + queries)
  fc-geom/            # geometry on kurbo; booleans via flo_curves
  fc-glyphdata/       # glyph naming / Unicode / Unihan database
  fc-layout/          # FEA <-> model, layout tables <-> model, shaping
  fc-sfnt/            # TTF/OTF/TTC/WOFF/WOFF2 read/write, CFF writer, bitmap tables
  fc-formats/         # UFO, Glyphs, SFD, Type1, SVG, BDF/PCF/FNT/FON/PSF, AFM/PFM, TTX
  fc-compile/         # export pipeline (fontc Source impl, preservation merge, post-processing)
  fc-raster/          # thumbnails, hinted previews, strike generation, bitmap ops
  fc-hint/            # PS hints, TT instruction asm/disasm, ttfautohint bridge
  fc-doc/             # session, documents, commands, undo, autosave, .fcf
  fc-api/             # typed request/response/event types (ts-rs exported) + session facade
  fc-devserver/       # axum HTTP+WS transport for browser dev and e2e tests
  fc-cli/             # `fontcarver` binary
src-tauri/            # Tauri 2 shell (thin: wires fc-api to invoke/emit)
app/                  # React 19 + TypeScript + Vite frontend
fixtures/             # test fonts (small ones committed; large ones referenced by path in fixtures/README.md)
docs/                 # specs, plans, research, user docs
scripts/              # dev/automation scripts
```

## Rust

- Edition 2021, stable toolchain (rust-toolchain.toml pins the version), `cargo fmt` clean, `cargo clippy --all-targets -- -D warnings` clean.
- One `Error` enum per crate via `thiserror`; never `unwrap`/`expect` outside tests and `const` contexts; never `panic!` on untrusted input (fonts are untrusted input).
- `tracing` for logging; no `println!` in library code.
- Every `pub` item has a doc comment. Modules ≤ ~800 lines; split by responsibility.
- Numeric conventions: font units are `f64` in the model; sfnt writers round at the boundary. Coordinates use y-up (font space). `kurbo::Point`/`Affine` are the geometry primitives; do not invent parallel types.
- Glyphs are referenced by `GlyphName` (SmolStr newtype) in the model and by `GlyphId` handles in sessions. Never index glyphs by position in APIs.
- Serialization: `serde` with explicit field names, `#[serde(default)]` for optional fields, `deny_unknown_fields` **off** (forward compatibility). The `.fcf` schema lives in `fc-doc/src/fcf/schema.md` and must be updated with every model change.
- Tests: unit tests next to code (`#[cfg(test)]`), integration tests in `crates/<name>/tests/`, fixtures via `fixtures::path("name")`. Round-trip tests compare **semantics** (parsed structures, shaping output), not bytes, unless byte identity is the requirement.
- Performance-sensitive paths (grid thumbnails, open, export) get a `criterion` benchmark or at least a timed integration test with a budget assertion.
- Dependencies: prefer the fontations family (`read-fonts`, `write-fonts`, `skrifa`), `kurbo`, `norad`, `fea-rs`, `fontc`, `harfrust`, `tiny-skia`, `flo_curves`. Adding a new dependency requires a one-line justification in the PR/commit message **and a permissive license** — MIT, Apache-2.0, BSD-2/3-Clause, 0BSD, ISC, Zlib, BSL-1.0, Unicode-DFS/Unicode-3.0, or a public-domain equivalent. Never GPL/LGPL/AGPL/SSPL/CDDL/EUPL, direct or transitive. MPL-2.0 only with an explicit ruling recorded in `THIRD_PARTY.md`. The same rule applies to npm dependencies. See `docs/licensing.md` §5.

## TypeScript / React

- `strict: true`, ESLint (typescript-eslint recommended + react-hooks) and Prettier clean; `pnpm check` (tsc) must pass.
- Function components + hooks; **no default exports**; one component per file; files ≤ ~400 lines.
- State: Zustand stores per domain (`useDocumentStore`, `useEditorStore`, `useUiStore`); derived data via selectors; no prop drilling deeper than two levels (use context or store).
- Kernel access only through `app/src/transport/` (`Transport` interface with `TauriTransport` and `HttpTransport`). Components never call `invoke` directly.
- API types are generated from Rust (`fc-api` → `app/src/api/generated/`); never hand-edit generated files.
- Canvas rendering code lives in plain TypeScript classes under `app/src/canvas/` (no React inside); React components own lifecycle and pass input events in.
- Every user-visible string goes through `t('key')` (i18next); keys in `app/src/i18n/en.json`; `zh-CN.json` is produced by a translation pass (never by the same agent that wrote the feature).
- Styling: CSS Modules + design tokens in `app/src/styles/tokens.css`; support light/dark via `data-theme`; no inline style objects except for computed geometry.
- Accessibility: every interactive element keyboard reachable; icon buttons have `aria-label`.
- Tests: Vitest + Testing Library for stores/components; automated e2e with Playwright in `app/e2e/` against `fc-devserver` + Vite; agent-browser is used for interactive visual verification (screenshots) during UI tasks.

## Licensing and porting from FontForge

FontCarver is licensed `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`). FontForge is a **behavioural and format reference**, not a codebase we translate. The full reasoning is in `docs/licensing.md`; these are the rules you follow.

1. **Read the C file's header before adapting anything from `fontforge/`.** FontForge as a whole is GPLv3-or-later, but most of its source is 3-clause BSD. Only a file whose own header states the revised (3-clause) BSD license may be adapted. A GPL-only file — or one with no license, or another license (Open Group, Adobe) — must be re-implemented from the format specification or from observed behaviour, without reading and transcribing it. Naming a FontForge flag or policy in a doc comment to say which behaviour you match is description, not copying, and is fine.
2. **Attribute every adapted block.** Put a comment at the top of the adapted function or module:
   ```rust
   // Adapted from FontForge <path> (3-clause BSD, Copyright (C) <years> George Williams / <author>)
   ```
   reproducing the exact copyright line from that file's header, and add a row to the table in `THIRD_PARTY.md` §1.2 (our file, the FontForge source file, the license found in its header).
3. **Never link or embed FontForge.** No linking against libfontforge, no bundled binaries, no shelling out to `fontforge`, no vendored FontForge sources. The `fontforge/` checkout is gitignored research material and is never committed or shipped; tests reach its font corpus only through `fc_fixtures::external("fontforge-tests")`, which skips when absent.
4. **New dependencies must be permissively licensed** — see the dependency rule under *Rust* above and `docs/licensing.md` §5. Test fixtures are data, but they are still redistributed: record every committed fixture in `fixtures/README.md` and `THIRD_PARTY.md` §2 with its license verified from the font's own `name` table, and prefer OFL or public-domain fonts.
5. **Reviewers check rules 1–4.** A task that ports FontForge code must name, in its report, every FontForge source file it drew from and the license found in each of those files' headers; a report that does not is incomplete and the review fails. A reviewer who cannot verify the license of an adapted block rejects the change.

## Git

- Conventional Commits (`feat(fc-sfnt): …`, `fix(app): …`, `docs: …`, `test: …`, `chore: …`).
- Small, focused commits; every commit builds and passes tests.
- Never commit `fontforge/`, `target/`, `node_modules/`, generated API types (they are regenerated by `pnpm gen:api`), or large fixtures (> 2 MB) — reference them in `fixtures/README.md`.

## Task protocol for delegated work

1. A task file (`docs/superpowers/plans/tasks/<phase>-<nn>-<slug>.md`) defines scope, files, public interfaces, and acceptance tests. Do exactly that scope; if something outside is blocking, stop and report instead of widening.
2. Write the failing tests first when the task defines behaviour; make them pass; refactor.
3. Finish with `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test -p <crate>` (Rust) or `pnpm check && pnpm lint && pnpm test` (app). Paste the final command output in your report.
4. Report: what was built, what was verified (commands + results), what was not done and why, open questions. Keep it under 300 words.
