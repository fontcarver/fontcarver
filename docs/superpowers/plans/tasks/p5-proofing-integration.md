# P5 proofing integration

## Scope

Complete saved proof strings and grouped script-aware pairs/phrases with keyboard
stepping in the text workspace, standalone vector HTML/PDF proof exports, shaping
controls and cluster inspection, configurable preview/waterfall controls, and a
preview-only host for detachable shell windows. Preserve existing editor and
kerning transition barriers. Scope covers audit rows 44 (saved strings only),
73–76, 95, 109; optical metrics and shell host lifecycle have separate owners.

## Files and interfaces

Own TextView integration, ProofLibraryPanel, proof export request/download adapter,
PreviewWindow, narrow optional TextTabState fields, text shaping identity fixes,
and English keys. Child tasks own proofLibrary/useProofLibrary, proofExport,
PreviewStrip/canvas controls, TextShapingControls. Public exports are defined in
those child plans. No new dependency and no model schema changes: the library
uses namespaced FontInfo.user_data already covered by native persistence.

## Acceptance

Focused tests cover saved library behavior, stepping/filtering, PDF/HTML safety
and vector paths, preview controls, script/language stale guards, UI integration,
and failure states. A real headless agent-browser session verifies library
save/reload and proof export. Use parent final frontend checks, do not make
concurrent broad builds. Resource checks at boundaries; scratch owner proofing.
