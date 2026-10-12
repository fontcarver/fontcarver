# Export workflows integration acceptance

Implementation scope: persisted named export profiles, profile/instance batch exports,
read-only preflight with glyph navigation, export-only extrema/rounding/decomposition,
static CFF1 repeated-program subroutinization, and common CLI/API options.

The API additionally routes SVG, single-glyph EPS, AFM/PFM, TTX, TTC, bare CFF and
Type 42 through the independently implemented codecs. TTC accepts additional open
documents via `collection_docs` and stages the final collection only after all faces
compile. EPS requires an explicit glyph. Unsupported options fail before replacing
output, including geometry flags on bitmap-only OTB and subroutines outside static
OTF/CFF. Type 1 production-name requests keep their existing explicit warning.
Linked source PNGs are embedded only in UFO/designspace export copies, using the
source document path, per-image validation and a 128 MiB aggregate limit.

## Focused evidence

- `pnpm --dir app exec vitest run src/dialogs/exportFormats.test.ts`: 17 tests passed.
- Export dialog/profile/matrix/preflight suites: 76 focused tests passed; focused ESLint passed.
- Browser verification used the actual dialog with mocked transport, including profile save, preflight navigation and a 2/2 batch. Light/dark screenshots are retained in `docs/screenshots/p5-export-preflight-matrix-light.png` and `docs/screenshots/p5-export-matrix-results-dark.png`.
- Narrow `rustfmt` and `git diff --check` passed for API/CLI export edits.
- API regression target `export_options` covers ignored-option rejection, output
  preservation, real static/Type 1 geometry, TTX/TTC semantics, failed collection
  append and EPS selection. CLI target `cli` covers shared processing flags.
- Initial Cargo invocation was blocked by concurrently incomplete shared modules;
  it did not reach the tests. No failed or interrupted run is recorded as passed.
- Per root coordination, `/root/rust_integration` owns final execution of
  `cargo test -p fc-api --test export_options --test export --test type1_export`,
  `cargo test -p fc-compile --test export_geometry`, CFF subroutine tests and
  `cargo test -p fc-cli --test cli`. Feature regression execution subsequently remained with root.
- Central integration regenerated 359 TypeScript types and passed workspace formatting and Clippy; see `docs/superpowers/reports/p5-rust-api-integration.md`. These checks do not replace the pending feature tests.
- Frontend acceptance is recorded in `docs/p5-export-ui-acceptance.md`; geometry and CFF details
  are recorded in `p5-58-export-geometry.md` and `p5-60-subroutinize.md`.

## Boundaries

CFF optimization shares identical complete charstrings; it does not extract
arbitrary repeated fragments or optimize CFF2/Type 1. Geometry preprocessing is
static only; incompatible variable exports are rejected. CLI collection supports
multiple faces; the export dialog's TTC choice currently exports its selected
single document. Profiles use application-local persistence rather than altering
the font source. No FontForge source or new dependencies were used by this task.

## Resource closure

Owned export test, browser and Vite processes have stopped. Task scratch directories
were removed. Final frontend artifact check passed: build 6.29 GiB, temporary
0.23 GiB, workspace free 668.58 GiB. No source, fixtures or shared stores were removed.

Latest parent-shared workspace checkpoint: build 18.35 GiB, temporary 0 GiB,
free 656.73 GiB; checker passed with the >10 GiB build-cache warning. Root was
notified to plan profile-cache cleanup before additional broad builds; this task
did not remove another task’s active artifacts.
