# T9b text metrics helper report

Branch: `p1-text-metrics`  
Base: `0ba779e` (`p1-outline-editing` when dispatched)  
Commits:

- `7c1ea7b feat(app): coalesce text metrics shortcuts`
- `4a3c15b fix(app): handle empty text metrics glyphs`

## Built

Added the headless text metrics keyboard helper requested by the T9b UI integration. `metricShortcut` maps platform Mod+Left to a negative LSB step and Mod+Right to a positive RSB step. Shift selects 10 units; macOS Meta+Ctrl selects 100. Linux Ctrl-as-Mod remains one unit, and Alt combinations remain unclaimed for later kerning work.

`TextMetricsRunner` accumulates LSB and RSB changes from immutable starting metrics, publishes immediate `{width, lsb, rsb, lsbDelta}` previews, and sends both absolute sidebearings through one `EditSession`. This preserves both values across queued-frame replacement and side switching. A run closes after 500 ms or explicit `end()`. Its idempotent close promise resolves after `doc.edit_end`, then invokes `onEnd`. The supplied controller gate is forwarded through `after`; the runner does not publish a second document hold.

For layers without outline bounds, `fc-doc` does not apply sidebearings like bounded geometry: LSB adds to width and RSB replaces width. The optional `hasOutline: false` path therefore accumulates both shortcuts into one absolute width command, keeps LSB at zero and RSB equal to width in the preview, and reports no geometry translation.

Tests cover modifier semantics, bounded and empty alternating edits, delayed begin with repeats, exact request/undo sequencing, deadlines, idempotent delayed close, and failure reporting.

## Verification

- `pnpm --dir app exec vitest run src/views/text/textMetrics.test.ts` — 10 passed
- `pnpm --dir app check` — passed
- `pnpm --dir app lint` — passed
- `pnpm --dir app exec prettier --check .` — passed
- `git diff --check` — passed

No UI, store, editor, i18n, Rust, dependency, headed, or desktop changes/tests were performed, per scope and user input-isolation request.
