# CFF1 authored flex export guard

Implemented a writer error for authored `PsHints.flex = true`; unsupported
metadata is no longer silently ignored by `write/hints.rs`. Three public
`write_cff` regressions cover authored flex, a flex-marked component base omitted
from output order, and successful evaluation of an ordinary cubic pair. The
parent owns the matching recursive guard in `write/cff.rs`.

Verification performed:

```text
rustfmt --edition 2021 --check crates/fc-sfnt/src/write/hints.rs crates/fc-sfnt/tests/cff_flex.rs
[exit 0, no output]
git diff --check -- crates/fc-sfnt/src/write/hints.rs crates/fc-sfnt/tests/cff_flex.rs docs/superpowers/plans/tasks/p5-02-cff-flex-guard.md
[exit 0, no output]
```

Tests were written before implementation. The first locked
`cargo test -p fc-sfnt --test cff_flex` attempt exited 101 during concurrent
`fc-model` edits (missing `preprocess`, unresolved `node_links` exports/methods,
and incomplete `metrics_keys` fields); no behavioral red result was observed.
The post-change run was cancelled while still queued, without starting Cargo,
at the parent's request to consolidate final verification. **All three flex
regressions remain pending the root's shared Cargo run.** Workspace Clippy,
formatting, and broad regression checks also belong to the root. Task scratch
was removed. Final artifact check: **PASS** (build 6.10 GiB / 13,179 files;
temporary 0.23 GiB / 7,659 files; workspace free 668.74 GiB).

Remaining limitation: locked `read-fonts` 0.43.3 `ps/cs.rs` exposes no flex
callback in `CommandSink` (line 116). Its `Flex`, `HFlex`, `HFlex1`, and `Flex1`
cases (lines 323–341) emit ordinary curves. Editable import therefore retains
geometry but cannot recover flex identity/depth or flag flex-specific hint loss.
Raw-table preservation paths are outside this finding. No model/import changes,
dependencies, or FontForge source were used. No open questions.
