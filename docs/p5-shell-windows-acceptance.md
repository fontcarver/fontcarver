# Shell window transport acceptance

Implemented deterministic document, tab and sidebar/inspector/preview window
routes in the shared kernel session. Native commands reuse and focus matching
windows; browser transport uses named popups and reports blocked creation. Tab
payloads preserve text/master/features and follow a stable tab id when its current
glyph changes. Rejoin focuses the owning document workspace or main window;
missing owners keep the detached window open. Completed native closes and browser
pagehide notify the owner. Native close decisions await the shell's asynchronous
save/cancel flow; browser exits retain standard dirty protection.

## Verification

- `pnpm --dir app exec vitest run src/transport/windows.test.ts`: **27 passed**.
  Initial run failed because the implementation module did not yet exist.
- `pnpm --dir app exec eslint src/transport/windows.ts src/transport/windowTargets.ts src/transport/windows.test.ts`: **exit 0**.
- Focused Prettier, `rustfmt --edition 2021 --config skip_children=true
  src-tauri/src/lib.rs src-tauri/src/windows.rs`, and `git diff --check`: **passed**.
- `cargo test -p fontcarver --lib windows::tests` was queued under the shared
  flock, then **cancelled before execution** so the root integration run can
  consolidate Cargo validation. Three native routing/validation tests are added;
  this record does **not** claim a native compile or native runtime pass.
- Parent shell task owns app integration, aggregate TypeScript checks and browser
  acceptance. Native desktop input automation was not used.

No dependencies or FontForge source were introduced. Capabilities include detached
windows and the `fs:allow-write-file` permission requested by proof export; the
filesystem plugin still controls selected-path scope. Temporary task data is
removed before handoff. The final artifact budget check passed.
