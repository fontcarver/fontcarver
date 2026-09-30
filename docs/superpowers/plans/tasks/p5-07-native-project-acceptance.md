# P5-07 — Native directory project acceptance handoff

Date: 2026-09-30. Scope frozen after the user's instruction to finish current work
and stop; no additional features or background jobs started.

## Delivered

- Explicit `.fcfp` project directories, with pretty per-glyph JSON files and an
  ordered `project.json` manifest containing native font-wide state.
- Deterministic name/content revision filenames, preserving unchanged glyph
  files. Flush new glyphs before atomic manifest publication; cooperating reader
  and writer processes use standard-library advisory locks. Unix directory sync
  orders publication. Failed precommit writes preserve the last committed font.
- Prune only obsolete regular files from the previous manifest after successful
  manifest publication and sync. Preserve arbitrary files, Git data and unknown
  orphan revisions; interrupted writes are ignored during reopen.
- Reject unsafe manifest filenames, duplicates, missing/corrupt glyphs,
  embedded-name mismatches, unsupported versions and managed symlinks.
- Existing `.fcf`/gzip/legacy behavior preserved. Session and API accept native
  directories; API reports `format: "fcfp"`. CLI owners integrated `.fcfp`
  conversion routing. The native schema documents durability and file layout.
- UI exposes Open Project Folder and `.fcfp` save/path handling. Recovery remains
  independent complete `.autosave.fcf` snapshots; project freshness uses the
  committed manifest mtime. Invalid recovery font payloads are filtered.

## Evidence and pending integrated execution

New `fc-doc/tests/directory_project.rs` contains eight cases: semantic/session
reopen, localized edits and Git preservation, corrupt/unsafe references,
interrupted writes and invalid-save preservation, symlink rejection, a 512-glyph
reopen budget, injected revision collision before commit, and manual text edits.
New `fc-api/tests/directory_project.rs` exercises native save/open, autosave,
recovery, and saving recovery back to the directory. History/recovery work adds
six integration and three autosave unit cases; see its companion task report.

Owned Rust files pass narrow rustfmt and diff whitespace checks. The UI agent
ran 34 focused tests, ESLint and Prettier successfully; see
[picker acceptance](p5-project-directory-ui-acceptance.md). Browser visual
acceptance was not obtained while shared Vite modules repeatedly reloaded.

Queued Rust invocations were cancelled before acquiring the shared lock at the
root integration coordinator's request (exit 143; no passing runtime test claim).
The coordinator owns final workspace Clippy, generated types and regression,
including:

```sh
cargo test -p fc-doc --lib --test directory_project --test fcf_roundtrip --test history_recovery --test session --test edit_sessions --test commands_undo --test feature_drafts
cargo test -p fc-api --test directory_project
```

The first integrated compile found unstable `str::as_str` on GlyphName's deref;
this was immediately corrected to `name.as_bytes()`. Final runtime acceptance
must be recorded from root integration output before row 7 is marked Complete.
Row 8 already has labeled document history and one-entry-per-gesture semantics;
new regressions substantiate failure/cancellation behavior when executed.

## Cleanup and boundaries

All owned Cargo waiters and UI/browser/server processes are stopped. Own scratch
contained only an empty verification log (4 KiB directory total) and was removed;
UI scratch (11 MiB) and the separate short browser temp path were also removed.
No dependencies, FontForge source adaptation, commits or shared status edits.
Unknown orphan revisions are deliberately retained rather than guessed safe to
delete. Power-loss behavior is based on file/directory ordering, not a hardware
crash test; native macOS/Windows execution is not claimed.
