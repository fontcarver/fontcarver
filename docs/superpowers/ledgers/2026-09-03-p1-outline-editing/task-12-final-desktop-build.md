# T12 final embedded desktop build

Date: 2026-09-07  
Model: `gpt-5.6-sol`  
Worktree: `.worktrees/p1-desktop-acceptance`  
Candidate: `62348254bbdf8a141bb771919881303238b7916e`

The clean `p1-desktop-acceptance` branch was fast-forwarded from `84e881e` to the exact integrated candidate; no independent commit was created. Required paths were present, and the tree was clean before and after generation/build.

Commands and wall times:

```text
pnpm --dir app install --frozen-lockfile                 2.74s
pnpm --dir app gen:api                                   3.19s
pnpm --dir app tauri build --debug --no-bundle          45.87s
```

The Tauri command ran `pnpm --dir ../app build`; TypeScript and Vite completed, 2,248 frontend modules were transformed, and the current `app/dist` assets were embedded. Rust then built the native application successfully:

```text
target/debug/fontcutter-desktop
size: 534,197,784 bytes
sha256: 76dddfe54c6555edf0b11768315c20600f2630ce45a7796060e9172dc60f39f0
```

Non-fatal output comprised the existing Tauri workspace-inheritance notices, `.app` bundle-identifier warning, and Vite's large-chunk warning.

This is build-only evidence for an unbundled debug Tauri executable. The executable was not launched, no native window or global input was used, and a `/proc/*/exe` check found no running process for this exact artifact. Release packaging and final desktop interaction acceptance are not claimed.
