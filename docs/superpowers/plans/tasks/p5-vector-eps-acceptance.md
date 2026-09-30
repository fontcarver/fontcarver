# P5 bounded EPS glyph interchange acceptance

Implemented the scoped native EPS reader/writer in `fc-formats::vector`.
The writer exports one selected glyph, preserves its name/advance/UPEM in inert
DSC comments, elevates quadratic segments, and validates before atomic output.
The reader builds editable contours from bounded literal path declarations and
one nonzero fill; it never executes imported PostScript. Unsupported operators,
multiple paints, executable syntax, malformed metadata and exceeded numeric,
operand, operation, element or byte limits produce errors.

Eight unit tests cover cubic/metadata round trip, ordinary EPS implicit closure,
unsafe/unsupported syntax, destination preservation, invalid metadata, bounds,
quadratic/multiple-contour geometry and independent Ghostscript acceptance of
only generated known-safe bytes. A public integration test times a synthetic
10,000-contour import with a 30-second budget.

## Verification

- Narrow `rustfmt` and `git diff --check`: passed.
- Initial `cargo test -p fc-formats --lib vector::eps::tests -- --nocapture`:
  failed before EPS compilation because sibling work temporarily lacked
  `fc-sfnt::ttx` and the `VariableStroke` match in `fc-geom`. Tests were written
  before implementation; this was not a behavioral red result.
- Final focused unit execution: queued without output, then cancelled at the
  root coordinator's stop instruction. The owned waiting `flock` process was
  terminated; the command exited 143. **Not passed.**
- Public performance integration and workspace Clippy/regressions: delegated to
  the integration coordinator; no additional queued Cargo command was started
  after the scope freeze.
- Final static commands exited 0: `rustfmt --edition 2021 --check
  crates/fc-formats/src/vector/eps.rs crates/fc-formats/tests/vector_eps.rs` and
  `git diff --check -- crates/fc-formats/src/vector/eps.rs
  crates/fc-formats/tests/vector_eps.rs`.
- Final artifact checkpoint: passed, build 6.08 GiB / 13,176 files; temporary
  0.18 GiB / 7,069 files; free space 668.75 GiB. Removed the task's empty scratch
  directory after cancelling its wait; no task processes remain.

Required integration commands (use the shared lock and coordinator TMPDIR):

```sh
cargo test -p fc-formats --lib vector::eps::tests -- --nocapture
cargo test -p fc-formats --test vector_eps -- --nocapture
```

The Ghostscript test was written but has not run; no interpreter compatibility
pass or performance result is claimed until the integration commands complete.

## Boundaries and provenance

This supports one EPS glyph and a literal declarative subset, not arbitrary
PostScript programs, transforms, strokes, colors, clipping, images or font-wide
data. Names are limited to 100 UTF-8 bytes so emitted DSC lines stay bounded.
No FontForge code was consulted or adapted; no dependencies were added.
Header/bounding-box conventions were checked against the
[Adobe EPSF 3.0 specification](https://printtechnologies.org/standards/files/epsf_spec_v3_0.pdf).
