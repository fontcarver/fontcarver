# P1 worktree cleanup inventory

Observed: 2026-09-07 14:15 CEST  
Ancestry target: `p1-outline-editing` at `56f9408`  
Action taken: read-only inventory; nothing removed or stopped.

## Clean and already merged

Each HEAD below is an ancestor of the ancestry target and had an empty `git status --short`:

| Worktree / branch | HEAD |
| --- | --- |
| `p1-acceptance-browser` | `0b82b90` |
| `p1-acceptance-kernel` | `c2fe856` |
| `p1-cjk-acceptance` | `cc17701` |
| `p1-desktop-acceptance` | `84e881e` |
| `p1-http-timeouts` | `727708e` |
| `p1-t7-minors` | `10b612e` |
| `p1-t8` | `7849801` |
| `p1-t9b` | `e814cda` |
| `p1-text-controls` | `3973ed4` |
| `p1-text-kernel` | `cc17701` |
| `p1-text-metrics` | `4a3c15b` |
| `p1-zh-cn` | `c7e7623` |
| `p1-zh-text` | `a5931f6` |

## Unmerged or dirty

`p1-text-ui` at `742f39c` is not an ancestor of the target. At observation time the branches had diverged by 2 target-only and 14 text-ui-only commits. Its worktree had seven unstaged paths owned by the active T9 integration:

```text
M app/src/shell/ProgressLine.tsx
M app/src/shell/tabActions.test.ts
M app/src/stores/uiStore.ts
M app/src/views/glyph/renderGlyph.ts
M app/src/views/text/TextView.test.tsx
M app/src/views/text/TextView.tsx
?? app/src/views/text/useTextFeedback.ts
```

The shared index was empty.

## Processes referencing worktrees

Only `p1-text-ui` had referencing processes. The 14:15 snapshot showed Astra's active headless suite:

- `pnpm` PID 1228502; Playwright shell/node PIDs 1228528/1228529.
- release `fc-devserver` on port 8794, shell/PIDs 1228546/1228547.
- Vite on port 1427, shell/pnpm/node PIDs 1228569/1228570/1228582; `esbuild` PID 1228595.
- Transient `chrome-headless` children were also observed during the suite and had exited by the follow-up process snapshot.

These are evidence of an in-progress test run, not cleanup targets. Recheck status, ancestry, and live PIDs immediately before later removal.
