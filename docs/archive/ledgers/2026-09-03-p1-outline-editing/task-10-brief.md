### Task 10: P0 backlog and hardening (kernel)

Batched mechanical items from the P0 final review, one Codex run, each its own commit: `SetGlyphMeta` `Option<Option<u8>>` → `double_option` serde (clear works over JSON); `Document::begin_edit` documents/handles a live token; `fw-doc` undo budget reachable via `Document::set_undo_budget` + eviction test; rename-mid-batch failure test; `fw-sfnt` CID fonts get `uniXXXX` names from cmap when the CFF charset gives only CIDs; platform 2/4 name records kept (model `NamePlatform::Other(u16)`); `head.created` stored as UFO date string; cmap streaming (no full `Vec` per subtable); duplicate-name suffix for CID; `#![warn(missing_docs)]` in fw-sfnt/fw-glyphdata/fw-devserver/fw-cli/fw-fixtures; `src-tauri` CSP set to a strict default; collection-warning helper de-duplicated into fw-sfnt; devserver config dir threaded without `set_var`; redundant `.gitignore` lines; fw-glyphdata unused `Error` removed or used.
- [ ] **Step 1–3:** tests per behavioural item; `cargo test --workspace`; commit series `chore(p0-backlog): …`.

---

