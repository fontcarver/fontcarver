You are implementing **P1 Task 10: P0 backlog and hardening (kernel)** for FontWorkshop (Tauri 2 font editor: Rust kernel + React/TS UI). This directory is the git worktree for branch `p1-t10`. This is a batch of small, independent items; each is its own commit with a test where behavioural. Other agents are concurrently creating the NEW crates `fw-geom`, `fw-compile`, `fw-formats` and, later, editing `crates/fw-doc/src/command.rs` and `crates/fw-api` `METHODS` — you must NOT touch `crates/fw-doc/src/command.rs` and must NOT add or rename API methods; everything else in existing crates is yours.

## Read first
1. `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-10-brief.md` — the item list (normative) with the source of each item: `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/superpowers/ledgers/2026-09-03-p0-final-review.md` (Minor list + deferred triage) and the archived ledger `docs/superpowers/ledgers/2026-09-03-p0-foundation.md` (search "deferred").
2. `CONVENTIONS.md` (binding).

## Items (exact)
1. `fw-doc` `SetGlyphMeta.color_label: Option<Option<u8>>` → serde `double_option` so JSON `null` clears and absence leaves unchanged (find/implement a `double_option` helper module); update the ts-rs type override to `number | null | undefined`-equivalent (`"?: number | null"`); regenerate bindings (`cargo run -p fw-api --bin gen_ts`); test clearing over JSON through `Api::call("doc.apply")`.
2. `Document::begin_edit` while a token is live: document the behaviour and make it explicit — finalise the previous edit (as `apply` does) rather than silently discarding; test.
3. `Document::set_undo_budget(bytes)` public + eviction test (tiny budget → oldest entries dropped, cursor/labels consistent).
4. Test for `Batch` rollback with a failing `RenameGlyph` mid-batch (references, order and handles restored).
5. `fw-sfnt` CID-keyed fonts: when the CFF charset yields only CIDs, name glyphs `uniXXXX`/`uXXXXX` from the cmap's first codepoint (falling back to `cidNNNNN`), de-duplicated with the existing `.gidN` suffix logic; test on Source Han Sans (external, skip if absent) — glyph for U+4E00 is named `uni4E00`.
6. `fw-sfnt` name records on platforms 2/4: add `NamePlatform::Other(u16)` to `fw-model` (additive, serde tolerant) and keep them; test with a synthetic name table built via write-fonts.
7. `fw-sfnt` `open_type_head_created` as the UFO date string `YYYY/MM/DD HH:MM:SS` (UTC) instead of raw seconds; test.
8. `fw-sfnt` cmap: fold format 4/12/13 pairs into the owners map while iterating (no full `Vec` per subtable); assert equal results on DejaVu and NotoSans.
9. `fw-sfnt` inference of `Point.smooth`: after reading contours, set `smooth = true` on an on-curve point whose incoming and outgoing tangents are collinear within 1° and both handles exist (cubic) or both neighbours are off-curve (quadratic); test on DejaVu `o` (all on-curves smooth) and `H` (none).
10. `#![warn(missing_docs)]` in fw-sfnt, fw-glyphdata, fw-devserver, fw-cli, fw-fixtures — fix every warning (real doc comments, not placeholders).
11. `src-tauri/tauri.conf.json`: strict CSP (`default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src ipc: http://ipc.localhost`) — verify `cargo build -p fontworkshop` still passes and note how to test in dev.
12. Collection-warning helper de-duplicated: one `fw_sfnt::collection_warning(bytes) -> Option<Warning>` used by fw-api, fw-doc, fw-cli.
13. `fw-devserver`: thread the config dir through `Api`/`ServeOptions` without `std::env::set_var` (add an `Api::with_config_dir(dir)` constructor in fw-api — allowed, it is not a METHODS change); tests updated.
14. `.gitignore`: drop the redundant `/target/` vs `**/target/` and `src-tauri/gen/schemas/` lines.
15. `fw-glyphdata`: remove the unused scaffold `Error` enum (or use it) so no dead public API remains.

## Verification (paste final lines)
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p fw-api --bin gen_ts && (cd app && pnpm gen:api >/dev/null 2>&1; pnpm check)
```
Report to `/home/chen/githubprojects/pixelfonts/fontworkshop/.superpowers/sdd/2026-09-03-p1-outline-editing/task-10-report.md` (per item: what changed with file:line, test). Commit on branch p1-t10 (`chore(p0-backlog): …` or `fix(...)`); do not push. Final message under 15 lines: Status, commits, test summary, items not done and why.
