# P2c/P2d browser evidence

Captured and visually inspected on 2026-09-19 at 1440 × 1000 using the named
`fc-p2cd-acceptance` agent-browser session. The browser used a temporary profile
and a dedicated development server on API 8813 / UI 1446 with isolated preferences.
No native desktop session or user browser profile was used.

The displayed font is the repository's original `fixtures/ufo/Minimal.ufo`.
For problem-report screenshots only, B was assigned A's Unicode value through
the normal editing API in memory. The source fixture was not saved or changed.

| Screenshot | Visible acceptance evidence |
| --- | --- |
| [OTF export, English/light](p2cd-otf-en-light.png) | CFF format selection, unhinted CFF disclosure, selected master, relevant options, and the complete wrapped format selector. |
| [Find problems, English/light](p2cd-problems-en-light.png) | Real duplicate-Unicode diagnostic, severity, glyph shortcut, compiler-validation deferral, result limit, details and refresh. |
| [Type 1 export, Chinese/dark](p2cd-type1-zh-dark.png) | PFB selection and Chinese disclosure of layout, hinting, master, component and Unicode losses before exporting. |
| [Find problems, Chinese/dark](p2cd-problems-zh-dark.png) | Translated diagnostics, severity, glyph shortcut, result limit, details and refresh in the dark theme. |

All four images were opened for visual inspection: dialog text and controls are
readable, format tabs wrap without clipping, and the action buttons remain visible.
The browser session and both dedicated servers were closed after capture.

The automated companion suite is `app/e2e/p2cd.config.ts`. It exercises actual
exports and reopened outlines/shaping, distinct master preservation, Type 1 loss,
feature-draft guards, and generation-bound problem navigation and refresh.

Final run after rebuilding the kernel:
`pnpm exec playwright test --config e2e/p2cd.config.ts` — **8 passed**.
The suite seeds autosave-disabled preferences
before starting its dedicated kernel, so timed recovery files cannot leak between
scenarios. Initial runs exposed SFD selected-master rejection and Type 1 alternate
glyph Unicode collisions; both were fixed by the format owners and the final run
verifies their UI-to-kernel roundtrips.
