# Visual lookup backend acceptance

2026-09-30. Scope: the backend portion of
`superpowers/plans/tasks/p5-99-visual-lookup-authoring.md`.

Added `crates/fc-api/tests/visual_lookup_authoring.rs` with 12 acceptance tests.
Nine cases cover escaped FEA for GSUB 1/2/3/4/6 and GPOS 1/2/4/6 through
feature draft validation/application, shaping, disabled-feature comparison,
and exported lookup inventory. Additional cases cover class substitution and
class positioning. The combined case checks undo/redo shaping, rejected
contextual edits, and `.fcf` preservation of active source plus invalid draft.
Position assertions include both advances and offsets, including the second
pair value. These are acceptance assertions, not claims of successful execution.

`rustfmt --edition 2021 crates/fc-api/tests/visual_lookup_authoring.rs` passed.
The focused test command was:

```sh
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-feature-acceptance \
flock /home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-resume/cargo.lock \
cargo test -p fc-api --test visual_lookup_authoring -- --test-threads=1
```

It exited **101 before building/running these tests** because shared changes
failed compilation: unstable `str::as_str` uses in `fc-formats` AFM output and
`fc-doc` project persistence; an unresolved `write_fonts::Validate` import in
`fc-compile`; incorrect `font.units_per_em`, ambiguous `bounding_box`, and type
inference in `fc-doc` optical metrics; and an unhandled `Command::NodeLinks`
variant. The integration owner received exact file locations. No unrelated
production changes or repeat Cargo run were made. The parent owns consolidated
Rust tests and Clippy after those blockers are fixed.

Read-only independent review checked the fixture syntax against the visual
serializer. Browser/UI verification belongs to the feature owner. No claim is
made for arbitrary FEA or unrepresented lookup kinds. The owned temporary
directory was empty after compilation and removed. Final artifact check passed
at 6.11 GiB build output, 0.23 GiB temporary output, and 668.65 GiB free space.
