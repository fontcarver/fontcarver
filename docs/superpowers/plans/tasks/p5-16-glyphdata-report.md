# P5-16 glyph database workflow acceptance

Implemented a versioned shipped JSON database, validated partial document overlays,
name/Unicode lookup, inherited suffix metadata, and a Glyph inspector database
section with recipe inspection, JSON import/editing, and Update Glyph Info.
The overlay persists at `font.info.user_data["com.fontcarver.glyphData"]` and uses
existing FontInfo snapshots. Metadata refresh is one `UpdateGlyphInfo` command;
it preserves source layers, anchor coordinates, tags, and notes.

The typed API exposes `glyphdata.state`, `glyphdata.set_overlay`, and
`glyphdata.update`; both writes reject stale document generations. Invalid overlay
content and locked glyph selections fail without partial mutation. The integration
tests cover overlay/update undo and redo, native FCF roundtrip, stale and malformed
requests, and locked-selection atomicity.

Verification:

- Narrow rustfmt check passed for all new doc/API modules and integration tests.
- Focused Cargo test was canceled while waiting for the shared Cargo lock, exit
  143; it is **interrupted**, not passed. Root Rust integration owns final execution
  of `cargo test -p fc-api --test glyph_data` and
  `cargo test -p fc-glyphdata --test database`.
- `pnpm exec vitest run src/views/glyph/panels/GlyphDatabasePanel.test.tsx --maxWorkers=1`: 8/8 tests passed. Scoped ESLint exited 0; Prettier check passed for the component, CSS, tests, and English strings.
- Interactive visual verification and broad workspace checks remain with the root
  integration pass; this report does not claim them.
- Own temporary directory was removed after canceling the waiter. Artifact check
  passed: build 6.06 GiB / 13,157 files, temporary 0.11 GiB / 6,581 files,
  workspace free 668.97 GiB.

No FontForge source was used. Added dependencies are existing workspace serde and
serde_json plus the internal fc-glyphdata crate; no new external dependency.
