# P5 anchor mark-cloud preview

Scope: mount an opt-in mark preview within AnchorsPanel; add isolated preview UI,
data loading, anchor matching and SVG framing helpers. No backend/schema,
renderGlyph, layer/component UI, translation, status or generated-type edits.

Behavior: match base attachment anchors to underscore mark anchors in the exact
selected master; display base and translated mark outlines together. Fetch Mark
category candidates in bounded pages with a bounded detail-request pool. Expose
pagination, empty/loading/error/retry states and discard stale asynchronous
answers after glyph/document/master changes. Refresh visible previews on document
changes. Missing exact-master layers never fall back to another master.

Acceptance: write failing pure matching/translation/framing tests and React async
loading/error/stale tests, then implement. Run focused Vitest, targeted lint and
format checks; parent owns integrated frontend checks and browser acceptance.
Resource checks precede work and completion; scratch belongs to p5-mark-cloud.
No third-party code adaptation or new dependency is required.
