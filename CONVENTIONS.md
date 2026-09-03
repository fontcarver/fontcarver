# FontWorkshop Engineering Conventions

These rules apply to every contributor, human or AI agent. Task prompts reference this file; do not restate it, follow it.

## Repository layout

```
Cargo.toml            # workspace
crates/
  fw-model/           # document model (pure data + invariants + queries)
  fw-geom/            # geometry on kurbo; booleans via flo_curves
  fw-glyphdata/       # glyph naming / Unicode / Unihan database
  fw-layout/          # FEA <-> model, layout tables <-> model, shaping
  fw-sfnt/            # TTF/OTF/TTC/WOFF/WOFF2 read/write, CFF writer, bitmap tables
  fw-formats/         # UFO, Glyphs, SFD, Type1, SVG, BDF/PCF/FNT/FON/PSF, AFM/PFM, TTX
  fw-compile/         # export pipeline (fontc Source impl, preservation merge, post-processing)
  fw-raster/          # thumbnails, hinted previews, strike generation, bitmap ops
  fw-hint/            # PS hints, TT instruction asm/disasm, ttfautohint bridge
  fw-doc/             # session, documents, commands, undo, autosave, .fwf
  fw-api/             # typed request/response/event types (ts-rs exported) + session facade
  fw-devserver/       # axum HTTP+WS transport for browser dev and e2e tests
  fw-cli/             # `fw` binary
src-tauri/            # Tauri 2 shell (thin: wires fw-api to invoke/emit)
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
- Serialization: `serde` with explicit field names, `#[serde(default)]` for optional fields, `deny_unknown_fields` **off** (forward compatibility). The `.fwf` schema lives in `fw-doc/src/fwf/schema.md` and must be updated with every model change.
- Tests: unit tests next to code (`#[cfg(test)]`), integration tests in `crates/<name>/tests/`, fixtures via `fixtures::path("name")`. Round-trip tests compare **semantics** (parsed structures, shaping output), not bytes, unless byte identity is the requirement.
- Performance-sensitive paths (grid thumbnails, open, export) get a `criterion` benchmark or at least a timed integration test with a budget assertion.
- Dependencies: prefer the fontations family (`read-fonts`, `write-fonts`, `skrifa`), `kurbo`, `norad`, `fea-rs`, `fontc`, `harfrust`, `tiny-skia`, `flo_curves`. Adding a new dependency requires a one-line justification in the PR/commit message.

## TypeScript / React

- `strict: true`, ESLint (typescript-eslint recommended + react-hooks) and Prettier clean; `pnpm check` (tsc) must pass.
- Function components + hooks; **no default exports**; one component per file; files ≤ ~400 lines.
- State: Zustand stores per domain (`useDocumentStore`, `useEditorStore`, `useUiStore`); derived data via selectors; no prop drilling deeper than two levels (use context or store).
- Kernel access only through `app/src/transport/` (`Transport` interface with `TauriTransport` and `HttpTransport`). Components never call `invoke` directly.
- API types are generated from Rust (`fw-api` → `app/src/api/generated/`); never hand-edit generated files.
- Canvas rendering code lives in plain TypeScript classes under `app/src/canvas/` (no React inside); React components own lifecycle and pass input events in.
- Every user-visible string goes through `t('key')` (i18next); keys in `app/src/i18n/en.json`; `zh-CN.json` is produced by a translation pass (never by the same agent that wrote the feature).
- Styling: CSS Modules + design tokens in `app/src/styles/tokens.css`; support light/dark via `data-theme`; no inline style objects except for computed geometry.
- Accessibility: every interactive element keyboard reachable; icon buttons have `aria-label`.
- Tests: Vitest + Testing Library for stores/components; e2e via `scripts/e2e/*.ts` using agent-browser against `fw-devserver`.

## Git

- Conventional Commits (`feat(fw-sfnt): …`, `fix(app): …`, `docs: …`, `test: …`, `chore: …`).
- Small, focused commits; every commit builds and passes tests.
- Never commit `fontforge/`, `target/`, `node_modules/`, generated API types (they are regenerated by `pnpm gen:api`), or large fixtures (> 2 MB) — reference them in `fixtures/README.md`.

## Task protocol for delegated work

1. A task file (`docs/superpowers/plans/tasks/<phase>-<nn>-<slug>.md`) defines scope, files, public interfaces, and acceptance tests. Do exactly that scope; if something outside is blocking, stop and report instead of widening.
2. Write the failing tests first when the task defines behaviour; make them pass; refactor.
3. Finish with `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test -p <crate>` (Rust) or `pnpm check && pnpm lint && pnpm test` (app). Paste the final command output in your report.
4. Report: what was built, what was verified (commands + results), what was not done and why, open questions. Keep it under 300 words.
