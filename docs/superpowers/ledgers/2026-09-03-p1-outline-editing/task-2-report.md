# P1 Task 2 Report — `fw-doc` editing commands

**Status:** DONE  
**Commit:** `ac465c0 feat(fw-doc): add editing commands and edit sessions`

## Built

- Added the normative outline, structure, metrics, layer, glyph-order, master, and font-guide commands. `AddGlyphsFromRecipe` is intentionally absent: `fw-api` expands recipes into `AddGlyph` commands.
- All path transforms, split/reverse/conversion, simplification, overlap removal, and stroke expansion delegate to `fw-geom`. Geometry failures are atomic and surface as `Error::Geometry`.
- Added snapshot-backed undo for font guides, duplicate-glyph handle allocation, precise `Change::{metrics,layers,guides}` flags, and `Document::preview_edit` as the coalesced `apply_in_edit` alias.
- Added metrics-key resolution for references, mirrored sides, offsets, literals, and cycles. Existing persisted `metrics_keys` were verified and documented; edit/resolution state is transient.
- Added focused integration coverage for exact undo, handle movement/leveling, curve split/delete error, path operations, component decomposition, guides, layer/background operations, metrics, cycles, and 50-update edit coalescing.

## Verification

- `cargo fmt --all --check` — PASS
- `cargo clippy -p fw-doc --all-targets -- -D warnings` — PASS
- `cargo test -p fw-doc` — PASS, 44 tests, 0 failed
- `cargo test -p fw-doc --release` — PASS, 43 tests, 0 failed (one test is debug-only)

## Concerns

The task-required `Error` variants make the existing exhaustive mapping in `fw-api/src/error.rs` non-exhaustive; a read-only workspace check found this expected downstream integration gap. Task 5/controller must map `InvalidCommand`, `Geometry`, and `MetricsKeyCycle`. `KeepShape` uses `fw_geom::simplify` as the available public refitting oracle; the mandated split/delete sampled error is ≤1 unit, but `fw-geom` exposes no dedicated exact single-segment fit API.
