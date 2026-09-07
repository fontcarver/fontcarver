# T7 minor round report — 2026-09-07

Implemented all six deferred findings and both guards on `p1-t7-minors`.

- Tool gesture completion and tool switches now repaint cleared transient feedback. Drag cleanup preserves a newer drag/nudge preview.
- `ShapeDialog` mirrors current numeric values for same-event Enter confirmation. Tool tap/hold shortcuts stop while `modalCount > 0`.
- Pencil tolerance stays at 1 font unit at normal zoom and grows to one CSS pixel from its two-pixel sample spacing at low zoom.
- `curveFit.ts` is now a 60-line facade over cubic fit (322), corners (35), quadratic conversion (60), and shared geometry (78). Nearest cubic distance uses coarse endpoint/fit-parameter seeds plus clamped Newton refinement; evaluated curve points make its estimate conservative. Endpoint, stationary derivative/cusp, cubic, and quadratic error regressions pass.
- `LayerOutline.curve_format` is an optional API projection of existing model evidence (`glyf`/CFF/CFF2/master TT data). Actual layer points still win; curve-free CFF and quadratic TTC faces now differ correctly.
- Zoom/Hand match design §5: click/Alt-click/marquee zoom plus Hand/H, Space-drag and middle-button pan are covered by existing controller/GlyphView tests.

Profile, identical isolated warm workload (median of 5): 201 samples 14.0 → 9.5 ms; 501 32.6 → 25.2 ms; 1001 69.5 → 45.6 ms. Fitting remains synchronous; 1001 samples still cost about 46 ms.

Verification: `pnpm install --frozen-lockfile`; `pnpm --dir app gen:api`; `pnpm --dir app check`; `pnpm --dir app lint`; `pnpm --dir app exec prettier --check .`; Vitest at 50% workers: 79 files / 698 tests; `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test -p fc-api`: 105 tests. All passed.

Remaining integration concern: `editorController.ts` is temporarily 806 lines; T8 owns the coordinated extraction. Generated bindings were not committed.
