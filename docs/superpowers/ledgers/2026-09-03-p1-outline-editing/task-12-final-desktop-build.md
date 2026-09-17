# T12 final embedded desktop build

Date: 2026-09-07  
Model: `gpt-5.6-sol`  
Worktree: clean `.worktrees/p1-desktop-acceptance`  
Candidate: `2e53ba6432b9bee3aa15000a7918f472c9289fa9`

The cached desktop branch was fast-forwarded from `6234825` to the exact merged P1 tip. Dependency manifests, lockfiles, Rust toolchain, and `fc-api` were unchanged between those commits; the prior frozen install and generated bindings therefore remained current.

```text
pnpm --dir app tauri build --debug --no-bundle
passed; 16.03s
```

The Tauri command ran `pnpm --dir ../app build`. TypeScript and Vite completed, 2,249 frontend modules were transformed, the current `app/dist` was embedded, and affected Rust/Tauri crates recompiled successfully.

```text
target/debug/fontcarver-desktop
size: 534,198,552 bytes
mode: 775
sha256: 3155f6057d7cae505a3c14db950ee1630c753ff225aa2976ae1c0892b43bdd61
```

The exact artifact was copied byte-for-byte, with mode preserved, to `target/p1-acceptance/fontcarver-desktop` so worktree cleanup cannot remove it. Root's standard `target/debug` executable was not touched.

Non-fatal output was limited to existing Tauri workspace-inheritance notices, the `.app` identifier warning, and Vite's large-chunk warning. The worktree remained clean.

This is an unbundled debug embedded-assets build only. The executable was not launched, no native window/global input was used, and `/proc/*/exe` contained no process for the artifact. Release packaging and final desktop interaction acceptance are not claimed.
