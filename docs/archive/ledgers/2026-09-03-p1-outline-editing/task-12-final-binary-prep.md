# T12 final binary preparation — 2026-09-07

Model: `gpt-5.6-sol`  
Candidate: `2e53ba6432b9bee3aa15000a7918f472c9289fa9`  
Builder: clean `.worktrees/p1-text-kernel`

The builder branch was fast-forwarded to the exact merged P1 tip. Root had the same HEAD, and `git diff` against the candidate over `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates`, and `src-tauri` was empty.

```text
cargo build --release -p fc-devserver -p fc-cli
passed; 186.30s
```

The two resulting executables were copied into root `target/release/` with their executable mode preserved. Source and destination SHA-256 values matched:

- `fc-devserver`: 28,507,136 bytes, mode 775, `b306d1f0de5dd0b2c3fd69cd5dcd26878ea3ec6b36358eb615dc7b3be591290b`
- `fontcarver`: 25,254,312 bytes, mode 775, `c1dc9d917fadd40a2ba76623a5a6c42b7b17ab4648330f9110e16b89403e19f3`

This is artifact preparation, not an acceptance result. No server, browser, or desktop executable was started. The binaries are ready for the serialized final headless suite.
