# T12 CJK acceptance report — 2026-09-07

Executed from clean worktree `p1-cjk-acceptance` at `cc17701`, with its own target directory. `cargo metadata --no-deps --format-version 1` resolved the workspace, build directory, and all local crates under `.worktrees/p1-cjk-acceptance`; no stale repository path was used.

The repaired lookup found the actual sibling fixture `/home/chen/githubprojects/fontpixel/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf`: 16,529,832 bytes, SHA-256 `f1d8611151880c6c336aabeac4640ef434fa13cbfbf1ffe82d0a71b2a5637256`.

- `cargo test --release -p fc-sfnt --test read_cjk -- --nocapture`: executed, 1 passed; 65,535 glyphs, `.notdef` first, U+4E00 mapped, CID present; **899.866326 ms**, below 3 s.
- `cargo test --release -p fc-compile --test roundtrip_shaping source_han_export_meets_release_budget_when_available -- --nocapture`: executed, 1 passed; exported `maxp` has 65,535 glyphs and U+4E00 cmap exists; **58.696529177 s**, below 60 s with only 1.303 s headroom. This was not an early fixture skip.
- `cargo test --release -p fc-sfnt --test read_otf reads_external_cid_cff_metadata_when_available -- --nocapture`: 1 passed; actual import has Adobe/Identity CID metadata, a nonempty CID map, and U+4E00 glyph name `uni4E00`.
- Compiled identity regressions passed: `static_compile_metadata_tracks_model_identity_and_exclusions` and `retained_names_survive_when_compiled_post_has_no_names` (1 each, release).

The real CJK export test exposes compiled glyph count and cmap identity, but not its internal compiled glyph-name vector. No blocker or source change was found; branch remains clean with no new commit. Final UI/desktop acceptance remains separate.
