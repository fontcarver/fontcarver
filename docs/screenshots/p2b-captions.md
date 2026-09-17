# P2b browser acceptance evidence

Captured on 2026-09-17 with the `agent-browser` skill, using an isolated headless
Chromium profile and the real debug development server. Viewport: 1440 × 1000.
The temporary two-master fixture derives from the repository's licensed Minimal
UFO. Its twelve glyphs deliberately share a simple visible outline; these images
show the actual fixture outlines, not previews drawn by the test.

- [English, light: feature source](p2b-features-en-light.png). An accepted prefix
  contains `liga` and `kern`, the opt-in small-cap recipe is materialized, and
  validation succeeds in both masters. Source navigation, editor, diagnostics,
  generation controls, and the document inspector remain visible.
- [Chinese, dark: feature source](p2b-features-zh-dark.png). The same document and
  applied source after changing language and theme through Preferences.
- [Chinese, dark: original lookups](p2b-lookups-zh-dark.png). The exported TTF is
  reopened and its original GSUB ligature shows three actual glyph outlines.
  Conversion passes the backend equivalence check; this source-authored kerning
  remains in editable feature source rather than becoming model pair controls.
  Backend comparison details are initially collapsed; the visible conversion
  status and source-owned explanations are localized.

## Automated verification

```sh
cargo build -p fc-devserver
cd app
pnpm exec playwright test --config e2e/p2b.config.ts
```

Final result: **10 passed (33.7 seconds)** after rebuilding the debug backend
from the final Rust sources, including the final comparison-details disclosure
change. The suite covers
invalid Unicode draft native save/reopen; validation versus activation; source
undo/redo; UFO, TTF, WOFF and WOFF2 export/reopen shape comparison; opt-in generation
and freezing; stale revisions and delayed diagnostics; master/tab/save/IME
barriers; failed-write recovery; imported glyph inspection and conversion;
native restore preserving unrelated width edits; proven kerning lift with group
fallback and explicit zero; source-only fallback; and unsupported JSTF blockers.

The scoped configuration owns ports 1436/8803, disables hot replacement, polls
file changes to avoid exhausting shared inotify resources, and uses a separate
configuration directory. Fixtures and documents are isolated between tests.
Screenshot inspection used separate ports 1438/8805 and a separate browser
profile. No native desktop automation, Source Han timing, or FontForge comparison
is represented by this evidence.
