# T12 final binary preparation — 2026-09-07

**Build preparation only; this is not a final acceptance pass.** No server, browser, or desktop process was started.

Root P1 was `b41ccb0beb3d32590640348c5a7df36011ab6e8f`. Builder worktree `p1-acceptance-browser` was clean at `0b82b90559451cdc82e168e62fda5ea31fdf03df`. `git diff --quiet p1-acceptance-browser..p1-outline-editing -- Cargo.toml Cargo.lock rust-toolchain.toml crates src-tauri` confirmed all Rust inputs, including tests and the Tauri shell, were identical. Both branches resolve `Cargo.lock` to blob `69a6b8c2e05be61d3363f38da3839f91ed5d05a7`; file SHA-256 is `25d814a46492308b85170703c8c8b6e2a34bc98ce1ad4343ecb923ee1b7de45b`.

The exact freshness command passed:

`cargo build --release -p fc-devserver -p fc-cli --bin fc-devserver --bin fontcutter`

Cargo validated the cached optimized artifacts in 0.19 s; no input required recompilation. The two executable ELF files were copied byte-for-byte into root `target/release/` with mode 775:

- `fc-devserver`: 28,506,936 bytes; SHA-256 `cb49f7c31816654b20ca3f816b34a3a06fb45fbd719a00b333713fe95925451a`; Build ID `11e0afc85b79c9ff812d6949d121d0e5b38fccf1`.
- `fontcutter`: 25,254,112 bytes; SHA-256 `76de1748f3418bfd1799195e8710bda3cd9ec36f955ae7abca50828689d2af10`; Build ID `aedbf20b1f1d5b381fb7aed89b8f5dcb209369f1`.

`cmp` proved each root copy byte-identical to its builder artifact. Root copies passed `--help`; root Git status remained clean because `target/` is ignored. They are ready for final headless tests.
