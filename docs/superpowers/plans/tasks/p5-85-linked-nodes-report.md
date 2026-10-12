# P5-85/86 linked node implementation report

Implemented stable optional point/guide IDs; guide-owned normal-offset node
constraints (including two nonparallel guide intersections); X/Y servant
interpolation with adjacent-handle scaling; model resolution for evaluated
outlines; atomic authoring, projection, unlinking and baking commands. Shared
command dispatch resolves induced geometry before locked-path validation and
restores snapshots on failures. SetGuide retains identity. Referenced topology
removal fails with an explicit unlink instruction. Model/schema/native-source
persistence integration is coordinated with construction persistence owner.

Added three model unit cases and eight document integration tests covering
interpolation, handles, guide motion/intersection, snapping, cycles, dangling and
nonfinite inputs, reordering, transactional rejection, undo/redo and .fcf.

Validation: scoped rustfmt and git diff --check passed. Own queued model test
was cancelled before compilation at parent request; consolidated Rust
integration owner now runs `cargo test -p fc-model --lib node_links` and
`cargo test -p fc-doc --test node_link_commands`. No runtime test pass is claimed
here pending that report. Initial centralized compile found one missing command
impact arm after simultaneous shared-file edits; repaired it immediately.

Boundaries: a point can follow at most two nonparallel guides. A servant target
cannot simultaneously be guide-owned; guide-owned boundary nodes can drive
servants. Release servant constraints before projecting those nodes/handles.
UI acceptance and final broad Clippy/tests belong to construction parent.

Own unused scratch directory removed. Resource check PASS: build 6.11 GiB,
13,180 files; temporary 0.23 GiB, 8,356 files; free 668.65 GiB. No new dependencies,
external source adaptations, commits, or shared status changes.
