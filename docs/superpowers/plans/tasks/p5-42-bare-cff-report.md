# Bare CFF acceptance handoff

Implemented standalone single-font CFF1 import and export in
`fc-sfnt/src/cff_interchange.rs`. Reads reuse the existing CFF outline/hint
importer through temporary sfnt metrics, then remove synthetic tables and
preservation bytes. CFF names, private hints, CID identity, fractional geometry,
advances and font-matrix UPEM are retained. File reads are bounded at 128 MiB.
Import diagnostics identify unavailable Unicode/layout/sfnt metadata; only
unambiguous glyph names infer Unicode. CFF2 and multi-font sets are rejected.

Added four integration tests for semantic 1000/2048-UPEM model round trips,
existing CFF fixture advances/outlines, CID identity, and truncated/invalid
input. The round-trip test includes a 10-second budget.

Validation performed:

```text
rustfmt --edition 2021 crates/fc-sfnt/src/cff_interchange.rs crates/fc-sfnt/tests/cff_interchange.rs
exit 0
python3 scripts/check_artifacts.py
build: 6.10 GiB, 13,171 files
temporary: 0.13 GiB, 5,204 files
workspace free: 668.73 GiB
PASS
```

Cargo tests and Clippy were **not run** by this task: root suspended new Cargo
work for final integration and will execute `cargo test -p fc-sfnt --test
cff_interchange` in its acceptance batch. No test is claimed passed. No owned
build/test/server processes or scratch artifacts were created. No dependencies
were added, and no FontForge source was read or adapted.

Separate Type42 delegated implementation/report covers its own files and
unexecuted tests. Parent owns registry wiring, CLI/API integration and final
workspace acceptance. Stop after this handoff per the latest user instruction.
