# T12 desktop acceptance preparation — 2026-09-07

**Status: paused by user; desktop acceptance is not passed.** This preliminary run used `p1-desktop-acceptance` from P1 `84e881e`, before final T9 integration. No production files changed.

Bindings were generated with current-worktree Rust crates. `pnpm tauri build --debug --no-bundle` ran the frontend production build (2,225 modules, embedded `app/dist`) and built `target/debug/fontcutter-desktop` in 63.52 s. This proves a debug embedded-asset build only; release packaging was not tested.

The binary launched on `DISPLAY=:0` with `GDK_BACKEND=x11` and a fresh `/tmp/fontcutter-desktop-acceptance.*` `FC_CONFIG_DIR`. Automation selected only the process-owned window with WM_CLASS `Fontcutter-desktop`; no dev URL or port 1420 was used. The initial embedded UI rendered and reported `Kernel: tauri`.

Completed interaction before the stop:

- `Ctrl+O` opened the actual native **Open File** chooser.
- Visible chooser navigation selected the licensed `fixtures/fonts/DejaVuSans.ttf`.
- The native Open action loaded DejaVu Sans into the Tauri UI: Book master, document metadata, and the 6,253-glyph grid were visible.
- The app Export dialog and actual native **Save File** chooser opened.

The save destination/export was not completed before the user requested all interactive automation stop. No native exported artifact was produced or verified, and no post-T9 rebuild/smoke ran. Temporary own-window screenshots were removed instead of committed.

Cleanup: root verified `/proc/1052505/exe` was this worktree's binary and sent SIGTERM. PID 1052505 and its dialogs exited. Both exact temporary config directories, scoped output directory, and temporary screenshots were removed; the worktree is clean. No further global input automation occurred.
