# TrueType variable binary checkpoint — 2026-09-30

The binary `fc_sfnt::tt` APIs now accept existing `glyf` variable fonts. This
does not enable authored variable export through the editor/compiler. Its
existing rejection remains in place.

`point_basis` exposes stored points before variation interpolation. A separate
`tt-variable-points-v1` binding includes the GID at every component dependency,
plus a cached digest of `fvar`, `avar`, `gvar`, `HVAR`, `VVAR`, `MVAR`, and `cvar`.
Static v1 digests remain unchanged. Including GIDs matters because identical
default outlines can have different variation rows.

`with_instructions` preserves variation tables exactly and requires byte-identical
indexed CVT contents when retaining `cvar`. Validation checks fvar headers/axis
records, gvar headers/offsets, and cvar tuple framing, packed indices and delta
runs. Optional metric/axis-mapping variation tables remain opaque; this does not
claim complete font validation, instance interpolation or interpreter acceptance.
Orphan variation tables and VARC bindings are rejected. Phantom component anchors
now fail consistently through `fingerprints` as well as `point_basis`.

The compiler rejects authored TT output with the new add-extrema, coordinate
rounding and component-decomposition export options, preventing preprocessing
from discarding the authored point identity.

## Verification checkpoint

Eight new variable binary regressions cover preservation, stale variation/GID
bindings, static digest compatibility, unchanged-CVT enforcement and malformed
variation data. Existing component and authored-export tests gained guard cases.

File-scoped rustfmt and changed-file `git diff --check` pass. The initial
`cargo test -p fc-sfnt --test tt_variable` did not execute tests: concurrent
`cff_interchange.rs` and `ttx.rs` changes failed compilation. The combined rerun
below was cancelled while waiting for the shared Cargo lock (exit 143), at the
root's request to serialize final integration checks. No passing test result is
claimed at this checkpoint.

```sh
cargo test -p fc-sfnt --test tt_basis --test tt_components --test tt_programs --test tt_variable
cargo test -p fc-compile --test tt_authored
```

Root integration owns final Clippy and test execution. Scratch cleanup and the
repository resource check were completed before handoff.

Implementation is original and uses existing dependencies, following the
[OpenType gvar](https://learn.microsoft.com/en-us/typography/opentype/spec/gvar)
and [cvar](https://learn.microsoft.com/en-us/typography/opentype/spec/cvar)
specifications. No FontForge source or executable was used. Variable editor
authoring, compiled variable binding, cvar editing, phantom component placement,
and general runtime program correctness remain open.
