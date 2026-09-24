# T9b Sol → Astra handoff

Branch/worktree: `p1-text-ui` at `2098d94`, `/home/chen/githubprojects/fontpixel/fontcarver/.worktrees/p1-text-ui`. Writer stopped with clean worktree.

## Completed

- Stable Text tab state/identity and suffix collision handling: `cbb4679`, `ee0fe57`, `ca6af63`.
- Bounded run geometry, zero-advance hits, fixed 4096px preview window/512 cap: `74ec6a7`.
- Tool-safe target transition and externally gated edit closure: `0f8311a`, `426772e`.
- Visible-only cell loader with generation isolation, concurrency 8, stable request signature, first-error cancellation: `d01e69e`, `27c85c1`.
- Typed canvas bridge compiles at `2098d94`: passive glyph-local Path2D rendering, bounded visibility callback, run hit routing, stable fit key support, origin-preserving target changes, text-mode active-outline suppression, second-Escape callback, external settler and `prepareTransition` canvas handle.
- `useTextShape` defers coordinator updates during document edits.

Validation at handoff: `pnpm --dir app check` passes; full Vitest 96 files/896 tests passes. No browser or native input was run.

## Remaining integration

- Build `TextView`, route Text tabs in `TabContent`, migrate initial glyph text once, and reconcile active glyph by cluster/name/gid without overwriting typed text.
- Extend `GlyphView` with actual tab id, canvas bridge props/handle, text-mode source gating, loading continuity, stable label behavior, and tab-id-based stepping.
- Wire `TextBar`, bounded cell index union, `PreviewStrip`, status/menus/Tool T/focus, English placeholder i18n keys, and CSS.
- Metrics helper is pending root approval: `7c1ea7b` + empty-outline `4a3c15b`; pass `hasOutline` from actual active outline bounds and expose runner as canvas external settler.
- Add focused TextView/canvas integration tests, headless Playwright acceptance and four headless screenshots; write `task-9b-report.md`.
- Extract the transition state from `editorController.ts` (now 859 lines) into a coherent helper before final.

## Hazards to preserve

- Use the caller's actual tab id everywhere; deterministic `glyph:doc:gid` can belong to a typed Text tab targeting another glyph.
- Await `prepareTransition()` before replacing active target/master/size; guard delayed activation callbacks against stale layout/state.
- Keep one GlyphCanvas/controller mounted. Editing coordinates remain glyph-local; passive run positions subtract `activeOrigin`.
- Never load paths for the whole run. Keep the fixed preview window and total requested indexes bounded to 512.
- Empty-outline metrics send width only. `onEnd` must occur after EditSession close so the controller gate cannot be overtaken.
- No headed browser, native mouse/keyboard/focus, xdotool, or global desktop interaction. Use isolated headless sessions only.
