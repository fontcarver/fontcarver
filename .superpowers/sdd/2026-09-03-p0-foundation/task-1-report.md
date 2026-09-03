# Task 1 report

Status: **DONE_WITH_CONCERNS**
Commit: `dce3d1b chore: scaffold workspace, app, tauri shell, fixtures, CI`

## Implemented

- Rust 1.94 workspace: eight requested crates, dependency pins, documented `Error` enums, and optional `fw-model/ts` using `ts-rs` 12.0.1.
- `fw_fixtures::{path, external}` and `skip_unless_external!`; six hashed fixtures under 2 MB and documented external corpora.
- Vite 7/React 19/TS 5.9 app: strict typing, ESLint 10, Prettier, i18n, CSS Modules, Vitest, and Playwright.
- Thin Tauri 2 shell with dialog/fs permissions and generated icons; CI, lockfiles, root scripts, and concurrent `scripts/dev.sh`.

## Verification (final run)

```text
$ cargo fmt --all --check
(no output; exit 0)
$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile ... in 0.71s
$ cargo test --workspace
fw_fixtures: 1 passed; all workspace/doc-test targets: ok
$ cargo build -p fontworkshop
Finished `dev` profile ... in 0.56s
$ cd app && pnpm install && pnpm check && pnpm lint && pnpm test && pnpm build
Done in 371ms; tsc -b: exit 0; eslint .: exit 0
Test Files  1 passed (1); Tests  1 passed (1)
✓ 52 modules transformed; ✓ built in 660ms
```

Additional checks: `cargo check --workspace`, all 26 dependency specifications, `fw-model --features ts`, Prettier, and staged-diff whitespace passed. Tauri identified React/Vite and both plugins; its dev smoke test reached the desktop binary before manual interruption.

## Files created

123 files: root workspace/CI files; crate sources; six fonts and README; `app/`; Tauri config, sources, capabilities, and icons; `scripts/dev.sh`. Three required `.gitignore` entries were committed.

## Self-review and concerns

Fixed Node typings, Vitest e2e exclusion, and monorepo Tauri discovery. `app/src` has no default exports; public Rust items are documented; fixture limits/hashes pass. Google Fonts’ requested static URL is now 404, so `NotoSans-Regular.ttf` came from authoritative `notofonts/noto-fonts`. pnpm’s ignored-esbuild-script warning is non-failing. The pre-existing `/.superpowers/` ignore edit remains uncommitted.

## Fix round 1

1. Removed the explicit pnpm version from `pnpm/action-setup@v4`; root `packageManager` is now the single source of truth. CI Node 24 satisfies the committed dependency engine ranges, and `pnpm --dir app install --frozen-lockfile` completed with pnpm 10.32.1.
2. Added a Vitest setup file that installs jest-dom matchers and registers Testing Library cleanup, plus a second component test that sees exactly one `FontWorkshop` node after the first test is cleaned up.
3. Declared `crates/fw-cli/src/main.rs` as the `fw` binary target.
4. Bound `skip_unless_external!`'s key expression once and added a missing-key unit test that detects reevaluation and verifies the early return does not panic.
5. Made `scripts/dev.sh` change to the repository root before starting either process.

```text
$ cargo fmt --all --check
(no output; exit 0)
$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
$ cargo test -p fw-fixtures -p fw-cli
fw-cli library/binary: 0 tests; fw-fixtures: 2 passed; doc-tests: 0 failed
$ cargo build -p fw-cli && ls target/debug/fw
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
target/debug/fw
$ cd app && pnpm check && pnpm lint && pnpm test
tsc -b: exit 0; eslint .: exit 0
Test Files  1 passed (1)
Tests  2 passed (2)
```
