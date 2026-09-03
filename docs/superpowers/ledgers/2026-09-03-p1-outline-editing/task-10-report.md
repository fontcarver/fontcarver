# P1 Task 10 report

Status: complete. Commits `66b8685..4ca6cab` (16 focused commits); not pushed.

1. Added reusable double-option serde support (`crates/fw-doc/src/double_option.rs:10`), API wire restoration (`crates/fw-api/src/types/commands.rs:89`), and optional-nullable TS generation (`crates/fw-api/src/bin/gen_ts.rs:37`). Test: `explicit_null_clears_glyph_color_label_through_json_dispatch` (`dispatch.rs:78`). `command.rs` stayed untouched as required.
2. Explicitly finalised prior gestures in `begin_edit` (`document.rs:150`). Test: `beginning_an_edit_finalizes_the_previous_gesture` (`commands_undo.rs:69`).
3. Exposed `Document::set_undo_budget` (`document.rs:238`). Eviction/cursor/label test: `commands_undo.rs:119`.
4. Added failing mid-batch rename rollback coverage for references, order, and handles (`commands_undo.rs:187`).
5. CID naming now prefers cmap-derived production names with `.gidN` deduplication (`names.rs:104-178`). Source Han U+4E00 test: `read_otf.rs:145` (ran, not skipped).
6. Added serde-compatible `NamePlatform::Other(u16)` (`info.rs:18`), retained/decoded all platform records (`names.rs:199-253`), and rendered numeric platforms (`NamesTable.tsx:17`). Tests: `read_ttf.rs:194`, `serde_roundtrip.rs:412`.
7. Converted `head.created` to UTC UFO syntax (`names.rs:16,53`). DejaVu assertion: `read_ttf.rs:25`.
8. Streamed cmap 4/12/13 mappings (`cmap.rs:38`). Eager-equivalence test on DejaVu/Noto: `cmap.rs:276`.
9. Inferred collinear smooth points within 1° (`read/mod.rs:214,236`). DejaVu `o`/`H` test: `read_ttf.rs:120`.
10. Enabled `missing_docs` in all five requested crate roots (`src/lib.rs:3`); clippy reports no warnings.
11. Installed the exact strict CSP (`src-tauri/tauri.conf.json:24`); `cargo build -p fontworkshop` passed. Dev check: run `cargo tauri dev`, exercise open/edit/events, and inspect WebView console for CSP violations.
12. Centralised `fw_sfnt::collection_warning` (`read/mod.rs:85`) and used it in API/doc/CLI. Existing disguised-extension API test: `dispatch.rs:361`.
13. Added `Api::with_config_dir`/immutable configuration (`api.rs:65-87`) and removed devserver environment mutation (`server.rs:70`). Tests: `dispatch.rs:53`, `http.rs:136`.
14. Removed redundant ignore rules (`.gitignore:5,17`).
15. Removed the unused glyphdata error module/export/dependency (`fw-glyphdata/src/lib.rs:5`, `Cargo.toml:8`).

Verification final lines:
```
cargo fmt --all --check                                  # exit 0
cargo clippy --workspace --all-targets -- -D warnings    # Finished dev profile
cargo test --workspace                                   # Doc-tests fw_sfnt: ok
cargo run -p fw-api --bin gen_ts && (...) pnpm check     # tsc -b; exit 0
```

Not done: none. Generated bindings remain ignored per repository convention.
