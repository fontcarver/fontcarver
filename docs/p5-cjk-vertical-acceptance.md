# P5 CJK vertical inspector and CID verification

Implemented on 2026-09-30. Final shared-workspace integration remains the
controller's responsibility. This report distinguishes executed checks from
new Rust tests awaiting that integration.

## Vertical inspector

`VerticalMetricsPanel.tsx` extends the existing glyph Metrics inspector with
TSB and BSB. Values use the kernel's resolved ink bounds (including components)
and effective layer/master vertical metrics. In y-up font coordinates:

- TSB = origin − ink maximum y; changing it changes only the origin override.
- BSB = advance − origin + ink minimum y; changing it changes only the advance
  override.

Both fields are disabled for empty ink. Existing advance/origin controls,
explicit zero overrides and one-command reset to master defaults remain.
Every write uses the existing undoable `SetVerticalMetrics` command and reloads
after success or failure. A reconciliation revision resets a rejected draft
even when the model's value did not change. The new controls honor the shared
locked-spacing mode. English keys are under `glyphView.metric`; independent
localization owns translated strings.

The task adds no data model or file-format fields. Existing P4 document tests
cover vertical edit atomicity/undo; compiler tests cover `vmtx`/`VORG` import,
export, effective defaults, edited origins and overflow validation.

## CID and vertical alternates

Source audit found no confirmed functional gap within P4's existing scope:

- Generation materializes `.vert` substitutions in both `vert` and `vrt2`,
  rejects manual tag conflicts, and preserves frozen source edits.
- CID authoring validates unique bounded IDs, glyph names, ROS and CID 0.
  Revision-checked numeric ordering retains the relative order of unmapped
  glyphs. Existing document tests cover rename/delete, undo and native FCF.
- Static CID CFF export supplies implicit `.notdef`/CID 0 and requires mappings
  for other exported glyphs. Variable CFF2 CID metadata is not claimed here.

New **unexecuted** tests in `crates/fc-compile/tests/feature_generation.rs`
exercise TTF/OTF horizontal/default vertical shaping, both alternates disabled,
either feature independently enabled, manual collisions and frozen edits.
Strengthened `crates/fc-compile/tests/cid.rs` reopens non-identity CID CFF,
checks ROS, exact CID keys and Unicode/advance associations, and distinguishes
incomplete mappings from duplicate glyph mappings.

## Executed verification

The new panel test was first run before implementation and failed because the
component did not exist. After implementation:

```text
pnpm exec vitest run src/views/glyph/panels/VerticalMetricsPanel.test.tsx \
  src/views/glyph/panels/EditInspector.test.tsx
Test Files  2 passed (2)
Tests       36 passed (36)
```

The eight new cases cover effective defaults, each bearing command, preservation
of other overrides including zero, atomic reset, empty outlines, invalid numeric
input and rejected-write reconciliation. Focused ESLint on the new component
and tests passes; Prettier and focused `git diff --check` pass.

The CID draft suite passed 2/2 and feature draft suite passed 12/12. FeaturesView
passed 5/6 initially; its generated-preview test exceeded the five-second limit
under concurrent build load. The isolated retry passed in 440 ms (one passed,
five skipped, 20-second test timeout). This does not erase the original timeout.
The Rust test files were formatted; no Cargo run is claimed because the
controller requested waiting for shared Rust integration.

## Browser boundary and handoff

An isolated `agent-browser` session attached to a separately launched headless
Chrome after the CLI's launcher failed. The full app did not reach an interactive
state within the bounded ready check. Vite reported the temporarily missing
`PreviewWindow` import from `DetachedPanel.tsx` during ongoing shell integration,
followed by repeated module and catalog reloads. No screenshot or completed
browser editing workflow is claimed. The session, Chrome, and owned API/UI
servers on ports 8797/1437 were stopped.

Suggested final checks, after creating the absolute task TMPDIR and acquiring
the shared Cargo lock:

```sh
cargo test -p fc-compile --test cid --test feature_generation --test otf_vertical
cargo test -p fc-doc --test cid --test vertical
cargo test -p fc-api --test cjk cid_authoring_orders_numerically_and_rejects_stale_writes
```

Row 19 now has functional TSB/BSB controls and component coverage; final browser
acceptance is outstanding. Rows 51 and 110 have existing P4 acceptance plus
stronger pending Rust regressions. Row 111 remains conditional on the proofing
owner's coverage of every requested surface; this inspector work alone does not
establish that row. No native platform acceptance or new workspace-wide green
build is claimed.

Task-owned scratch/profile cleanup removed approximately 61 MiB. The final
artifact check and shared status inspection were performed before handoff.
