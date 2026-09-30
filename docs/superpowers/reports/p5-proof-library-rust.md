# Proof-library native persistence regression coverage

Added `crates/fc-doc/tests/proof_library.rs` with two integration tests:

- Plain `.fcf` and gzip `.fcf.gz` preserve versioned proof entries, exact Latin,
  Arabic and Han text, all three directions, combining characters and whitespace.
- Existing `SetFontInfo` commands add, edit and clear the library with independent
  undo/redo entries, change notifications and dirty state. Whole-font comparisons
  verify unrelated metadata, font information and glyph data remain unchanged.

No production model/API changes or new dependencies. Temporary native documents
use a lifetime cleanup guard.

Verification:

```text
rustfmt --check --edition 2021 crates/fc-doc/tests/proof_library.rs
exit 0

TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-proof-library-rust \
flock /home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-resume/cargo.lock \
cargo test -p fc-doc --test proof_library
```

**Runtime acceptance is pending root integration.** The first attempt stopped
before tests with `Layer.metrics_keys` missing (E0560). The second stopped before
tests because parallel edits temporarily lacked `glyphdata.json`, NodeLink types
and related methods. Those files subsequently appeared. The third attempt stayed
queued; the root then requested cancellation to prioritize integration under high
host load. Only this task's waiting flock was terminated (exit 143). This is an
interrupted run, not a passing test result. Broad checks belong to the root.

Owned scratch was removed. Cleanup resource check:

```text
python3 scripts/check_artifacts.py
build: 6.14 GiB, 13,184 files
temporary: 0.13 GiB, 4,115 files
workspace free: 668.72 GiB
PASS
```

Task-start shared build size was 5.23 GiB; the increase is concurrent workspace
work, not an additional task cache. Git status confirms only the new test, scoped
plan and this report in this task's owned paths.
