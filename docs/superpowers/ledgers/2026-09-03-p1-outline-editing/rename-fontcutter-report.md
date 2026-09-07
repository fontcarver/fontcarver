# Rename report — FontWorkshop → Font Cutter

Branch `p1-outline-editing`, main working tree. Two commits, not pushed:

| Commit | Subject |
| --- | --- |
| `f6c5ade` | `chore(rename): FontWorkshop → Font Cutter (fontcutter, fontcutter.com)` |
| `a166c23` | `chore(rename): fw → fc internals, .fwf → .fcf, CLI fw → fontcutter` |

Both commits were verified independently with the full block (fmt, clippy,
workspace tests, `gen_ts`, app check/test/lint/prettier, e2e, Tauri crate
build). The lockfile changes were small enough to fold into each commit
(`Cargo.lock` only; `pnpm-lock.yaml` does not record workspace package names
and was unchanged — `pnpm install --frozen-lockfile` succeeds).

---

## Commit 1 — product rename

### Inventory (179 hits before, 91 files)

| Bucket | Where | Action |
| --- | --- | --- |
| Display name | `app/index.html` title, `src-tauri/tauri.conf.json` `productName` + window `title`, `app/src/i18n/{en,zh-CN}.json` (6 strings each: app name, Help ▸ About, Save-As hint/filter, recovery intro, status about line), `crates/fw-cli/src/main.rs` clap `about`, `crates/fw-devserver/src/main.rs` clap `about` + startup log, `crates/fw-doc/src/error.rs` (4 messages), `src-tauri/capabilities/default.json` description, ~40 Rust module docs, `app/src/components/Dialog.tsx`, `app/src/styles/tokens.css`, `app/src/stores/sessionStore.ts`, `app/playwright.config.ts`, `app/src/App.test.tsx` heading assertion, README/CONVENTIONS/PLAN/docs headings | `FontWorkshop` → `Font Cutter` |
| Machine identifiers | Tauri `identifier` `dev.fontworkshop.app` → `com.fontcutter.app`; `src-tauri/Cargo.toml` package `fontworkshop` → `fontcutter`, lib `fontworkshop_lib` → `fontcutter_lib` (+ `src-tauri/src/main.rs`); `package.json` `fontworkshop-workspace` → `fontcutter-workspace`; `app/package.json` `fontworkshop-app` → `fontcutter-app`; config dir `~/.config/fontworkshop` → `fontcutter` (`fw-api/src/handlers/prefs.rs`); autosave dir `fontworkshop/autosave` → `fontcutter/autosave` (3 branches in `fw-doc/src/autosave.rs`); thread names `fontworkshop-{autosave,events,shaping}`; test temp-dir prefixes; `gen_ts` temp file names; `app/src/stores/uiStore.ts` `STORAGE_KEY` `fontworkshop.ui` → `fontcutter.ui` | renamed |
| Persisted namespaces | `fw-formats/src/ufo/extra.rs` `dev.fontworkshop.extra`, `metadata.rs` `dev.fontworkshop.ufo.identifiers`, `value.rs` `$fontworkshop.plist.type`, `fw-fixtures/src/ufo.rs` + `fixtures/ufo/Minimal.ufo/lib.plist` `com.fontworkshop.fixture` | → `com.fontcutter.*` / `$fontcutter.plist.type` |
| `.fwf` format tag | `fw-doc/src/fwf/mod.rs` `FORMAT` | `"fontworkshop"` → `"fontcutter"`, **reader still accepts the old tag** |
| Domain | `Cargo.toml` `[workspace.package] homepage`, `homepage.workspace = true` on all 13 member crates, `homepage` in both `package.json`, README "Project home" link | added |
| Attribution | `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD_PARTY.md` (15 hits), `docs/licensing.md` (12), `CONVENTIONS.md`, `PLAN.md`, `docs/{p0-acceptance,design,research}`, `docs/superpowers/{plans,specs}`, `fixtures/{README.md,LICENSE-OFL.txt,build_font_subsets.py}` | `Font Cutter contributors` / `Font Cutter` |

### Judgement calls in commit 1

- **`package.json` names.** The ruling said "`name` → `fontcutter`" for both
  manifests. Two packages in one pnpm workspace cannot usefully share a name, so
  the existing suffixes were kept: `fontcutter-workspace` and `fontcutter-app`.
- **`dev.` → `com.` for UFO lib keys.** The ruling names `com.fontcutter.*` as
  the target namespace and moves the bundle identifier to `com.`, so
  `dev.fontworkshop.extra` and `dev.fontworkshop.ufo.identifiers` became
  `com.fontcutter.extra` / `com.fontcutter.ufo.identifiers`. **No compatibility
  branch exists for these keys** — a UFO written by an older build loses its
  preserved extras and UFO identifiers on re-read. Say the word and a legacy
  key fallback is a ten-line change.
- **`localStorage` key.** `fontworkshop.ui` → `fontcutter.ui` discards the
  developer's stored panel sizes / theme once. No migration was added.
- **Spec file renamed.** `docs/superpowers/specs/2026-09-03-fontworkshop-design.md`
  → `…-fontcutter-design.md` (`git mv`), with `README.md` and both plans
  updated. Five files under `docs/superpowers/ledgers/` still link the old path;
  the ledgers are frozen snapshots and were not touched.
- **Machine-absolute paths in research docs.** Four `docs/research/*.md` and one
  spike-dir line embedded `/home/chen/githubprojects/pixelfonts/fontworkshop/…`.
  The repository *directory* is not being renamed, so a blind substitution would
  have made them wrong; they were rewritten repo-relative
  (`fontforge/fontforge/`, "the FontForge reference checkout in this repository").
- **`PLAN.md` §附.** `~/githubprojects/pixelfontworkshop/` names *other* pixel-font
  projects, not this one. It was corrected to the real path
  `~/githubprojects/pixelfonts/`; `pixel-font-workshop` and friends were left alone.
- **`docs/p0-acceptance.md`** is a historical transcript. Its commands and window
  titles were rewritten to the new names as instructed; only the fixture family
  names in its recorded CLI output were left (see below).

### Deliberately left in commit 1

- **OFL fixture family names** — `FontWorkshop CFF Sans Fixture`,
  `FontWorkshop CFF Serif Fixture`, `FontWorkshopCFFSansFixture-Regular`,
  `FontWorkshopCFFSerifFixture-Regular`. These strings are baked into the
  committed `fixtures/fonts/SourceSans3Min.otf` and `SourceSerif4Min.otf`, whose
  bytes and SHA-256 hashes are recorded in `fixtures/README.md` and
  `THIRD_PARTY.md` and asserted by `crates/fc-sfnt/tests/read_otf.rs`. The ruling
  forbids changing those binaries, so the names stay in
  `fixtures/build_font_subsets.py` (the values it writes), `fixtures/README.md`,
  `THIRD_PARTY.md`, `docs/p0-acceptance.md` (recorded output) and the sfnt test.
  Non-content strings in the script *were* renamed (HTTP `User-Agent`, temp-dir
  prefix). Rebuilding them is a decision for the user (see below).
- `docs/superpowers/ledgers/**`, `.superpowers/**`, `fontforge/`, `.worktrees/**`,
  `node_modules`, `target`, `fixtures/fonts/*`, `build_font_subsets.py` URLs,
  git history.

---

## Commit 2 — internal short forms

### Inventory before

Roughly 1 100 token occurrences across 188 files: `fw_model` (223), `fw_fixtures`
(99), `.fwf` (94), `fw_doc` (94), `fw_sfnt` (78), `fw-sfnt` (53), `fw_geom` (53),
`fwf` (52), bare `fw` (51), the remaining ten crate names in both spellings,
`Fwf` (12), `FW_CONFIG_DIR` (10), `FwSource` (9), `VITE_FW_API` (6),
`FW_DEV_PROXY` (5), `fw_binary` (5), `fw-hint` / `fw-raster` / `fw-io` /
`fw-render` / `fw-tauri` / `fw-binary` (planned crates named only in docs),
`fw:event` (3), `FW_UI_PORT` (2), `FW_API_PORT` (2).

### Changes

| Bucket | Action |
| --- | --- |
| Crates | `git mv crates/fw-* crates/fc-*` (12 directories); package names `fw-api` → `fc-api` … `fw-sfnt` → `fc-sfnt`; Rust paths `fw_api` → `fc_api` …; workspace members, every `[dependencies]` path entry, `use`/`extern` paths, `.gitignore`, `.github/workflows/ci.yml`, `scripts/dev.sh`, `scripts/perf-open.sh`, docs and plans. Doc-only future crates renamed too (`fc-raster`, `fc-hint`, `fc-io`, `fc-render`, `fc-tauri`, `fc-binary`). |
| CLI | `[[bin]] name = "fw"` → `"fontcutter"`, clap `#[command(name = …)]`, `crates/fc-cli/tests/cli.rs` (`cargo_bin!`), `app/e2e/support.ts`, `scripts/perf-open.sh`, `docs/p0-acceptance.md`, plans and spec. |
| Event channel | `fw:event` → `fc:event` in `src-tauri/src/lib.rs`, `app/src/transport/TauriTransport.ts`, the P0 plan. |
| Environment | `FW_API_PORT`→`FC_API_PORT`, `FW_UI_PORT`→`FC_UI_PORT`, `FW_DEV_PROXY`→`FC_DEV_PROXY`, `FW_CONFIG_DIR`→`FC_CONFIG_DIR`, `VITE_FW_API`→`VITE_FC_API` (playwright config, `app/vite.config.ts`, `app/src/transport/index.ts`, `fc-api` tests, `fc-devserver`, CI, docs). |
| Document format | `.fwf` → `.fcf` in dialogs/filters/i18n/e2e/tests/docs; module `fw-doc::fwf` → `fc-doc::fcf` (`git mv` of `src/fwf/` and `tests/fwf_roundtrip.rs`); `ExportFormat::Fwf` → `Fcf` (TS bindings regenerated, `exportFormats.ts`, i18n label `FWF` → `FCF`); errors `InvalidFwfFormat`/`UnsupportedFwfVersion` → `InvalidFcfFormat`/`UnsupportedFcfVersion`; `FwSource` → `FcSource`; autosave sidecars `*.autosave.fcf`. |

### Compatibility branches (the only surviving old-name hits)

| Location | Behaviour | Test |
| --- | --- | --- |
| `crates/fc-doc/src/fcf/mod.rs` `LEGACY_FORMAT` | reads the `"fontworkshop"` envelope tag, re-stamps as `"fontcutter"` on save | `crates/fc-doc/tests/fcf_roundtrip.rs::reader_accepts_the_pre_rename_format_tag` |
| `crates/fc-doc/src/session.rs` `is_fcf_path` | `Session::open_path` accepts `.fwf` / `.fwf.gz` | `crates/fc-doc/tests/session.rs::session_opens_a_document_with_the_pre_rename_extension` |
| `crates/fc-doc/src/fcf/write.rs` `is_gzip_path` | `.fwf.gz` still selects the gzip writer | same test (asserts the `1f 8b` magic) |
| `crates/fc-doc/src/autosave.rs` `is_autosave_path` | `.autosave.fwf` snapshots stay recoverable | `crates/fc-doc/tests/session.rs::autosave_listing_finds_pre_rename_snapshots` |
| `crates/fc-api/src/handlers/session.rs` `is_fcf_path` | `session.save` accepts a `.fwf` target | covered by the store test + existing dispatch tests |
| `crates/fc-cli/src/cmd/convert.rs` `is_fcf_path` | `fontcutter convert x.ttf out.fwf` still writes a native document | — |
| `app/src/stores/sessionStore.ts` `NATIVE_EXTENSIONS` | a `.fwf` path is a native save target | `app/src/stores/sessionStore.test.ts::treats a pre-rename .fwf path as a native save target` |

### Binary-name collision (resolved, but worth knowing)

After the CLI became `fontcutter`, both `fc-cli`'s `[[bin]] fontcutter` and the
Tauri package `fontcutter` (implicit bin from `src/main.rs`) wrote
`target/debug/fontcutter`. Cargo warned about the output-filename collision and
the e2e suite, which shells out to that path, would have run whichever binary was
built last. `src-tauri/Cargo.toml` now declares:

```toml
[[bin]]
name = "fontcutter-desktop"
path = "src/main.rs"
```

The package is still `fontcutter`, `cargo build -p fontcutter` still works, and
bundles are still named "Font Cutter" via `productName`. `docs/p0-acceptance.md`
was updated to name `target/debug/fontcutter-desktop`. `cargo tauri dev/build`
were not exercised (no display / bundling toolchain in this session) — Tauri 2
takes the binary from Cargo's output, so no `mainBinaryName` was needed, but this
is the one path worth a manual smoke test.

---

## Verification

Run for **each** commit, with identical results unless noted.

| Step | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean, no warnings |
| `cargo test --workspace` | commit 1: 441 passed / 0 failed. commit 2: 443 passed / 0 failed (two new legacy-extension tests) |
| `cargo run -p fc-api --bin gen_ts` | exit 0, bindings regenerated (`ExportFormat` now has `Fcf`) |
| `pnpm --dir app check` | exit 0 |
| `pnpm --dir app test` | commit 1: 79 files / 690 tests. commit 2: 79 files / 691 tests |
| `pnpm --dir app lint` | exit 0 |
| `pnpm --dir app exec prettier --check .` | "All matched files use Prettier code style!" |
| `cargo build -p fc-devserver` | ok |
| e2e | commit 1 `FW_API_PORT=8787 FW_UI_PORT=1428`, commit 2 `FC_API_PORT=8787 FC_UI_PORT=1428`: **14 passed, 1 skipped** (CJK — external Source Han Sans fixture absent) |
| `cargo build -p fontcutter` | ok (Tauri shell compiles) |
| `./target/debug/fontcutter inspect fixtures/fonts/DejaVuSans.ttf` | `family: DejaVu Sans … glyphs: 6253` |
| `pnpm install --frozen-lockfile` | ok, `pnpm-lock.yaml` unchanged |

**Port note.** `FW_UI_PORT=1420` / `FC_UI_PORT=1420` cannot be used on this
machine: port 1420 is held by an unrelated "One True PDF" Vite dev server
(pid 818371), and `playwright.config.ts` has `reuseExistingServer: !CI`, so the
suite silently attached to that app and 14 specs failed against a PDF editor.
Re-run on **1428** everything passed. Nothing in the repository is at fault; the
stray server just needs stopping, or `FC_UI_PORT` pointing elsewhere.

**One flake.** On the first full commit-2 e2e run,
`fontview.spec.ts › reports the selection in the status bar` failed on a grid
click; it passed on an isolated re-run and on a second full run. Not
rename-related.

## Final inventory

`grep -rIn --exclude-dir={node_modules,target,.worktrees,fontforge,.git,ledgers} -e FontWorkshop -e fontworkshop -e "Font Workshop" -e FONTWORKSHOP .`

- 4 compatibility hits: `fc-doc/src/fcf/mod.rs:18` (`LEGACY_FORMAT`),
  `fc-doc/tests/fcf_roundtrip.rs:100` (its test), `fc-doc/src/fcf/schema.md:14`
  and the design spec §"native format" (documenting the accepted legacy tag).
- 13 fixture-family hits across `THIRD_PARTY.md`, `fixtures/README.md`,
  `fixtures/build_font_subsets.py`, `crates/fc-sfnt/tests/read_otf.rs`,
  `docs/p0-acceptance.md` — the baked-in names of committed binaries.
- Nothing else.

`grep -rInE '\bfw[-_:]|\bFW_|\bFw[A-Za-z]|\.fwf|\bfwf\b|\bfw\b|VITE_FW'`
(same exclusions, plus `Cargo.lock`, which contains zero `fw-` entries)

- 22 hits, all of them the `.fwf` compatibility branches listed above, their
  comments, and their tests. Nothing else.

## Unmerged worktrees

`.worktrees/p1-t7` (`1dea9d7`) and `.worktrees/p1-t8` (`3b1bd4b`) both predate
these two commits and still use the old names throughout. When either is merged,
the same two renames have to be applied to the incoming changes — expect
conflicts on every `crates/fw-*` path (the merge will see them as
rename-vs-modify), on `.fwf` strings, and on `FW_*` environment variables. The
cheapest path is probably to rebase each worktree onto `a166c23` and re-run the
two substitution passes over the conflicting hunks rather than resolving them by
hand.

## Decisions for the user

1. **The `.fwf` → `.fcf` extension was changed** per the scope extension. Old
   files still open, and the API/CLI still accept a `.fwf` save target, so nothing
   breaks; new documents get `.fcf`.
2. **OFL fixture fonts still say "FontWorkshop".** Renaming them means running
   `python3 fixtures/build_font_subsets.py` (fontTools 4.63.0 is installed, the
   two Adobe sources download from the pinned tags), then updating the sizes and
   SHA-256 hashes in `fixtures/README.md` and `THIRD_PARTY.md`, the family
   assertions in `crates/fc-sfnt/tests/read_otf.rs`, and the recorded output in
   `docs/p0-acceptance.md`. That is a ~30-minute change and touches committed
   binaries, which this task forbade. Want it done?
3. **UFO lib keys have no legacy fallback.** `dev.fontworkshop.extra` and
   `dev.fontworkshop.ufo.identifiers` became `com.fontcutter.*` with no read-side
   compatibility. A UFO exported by an older build loses its preserved extras on
   re-import. Add a fallback, or accept it pre-release?
4. **Repository URL.** No `repository` field exists in any manifest and none was
   added, per the ruling. If the project moves to a `fontcutter` GitHub org, both
   `package.json` files and `[workspace.package]` want a `repository` entry.
5. **Repository directory name.** The checkout is still
   `/home/chen/githubprojects/pixelfonts/fontworkshop`. Renaming the directory is
   a separate action; nothing in the tree depends on the path any more (the
   research docs that did were made repo-relative).
6. **The `fontcutter-desktop` binary name** for the Tauri shell (see above) — a
   different name is fine, it just has to differ from the CLI's.
7. **`app/src/i18n/zh-CN.json` has four untranslated recovery-dialog strings**
   (`dialogs.recovery.{title,intro,untitled,savedAt}`) that were already English
   before this work. Out of scope here, but worth a follow-up.
