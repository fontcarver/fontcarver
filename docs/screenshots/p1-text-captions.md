# P1 text view captures

Captured from the application in an isolated, headless Chromium session at 1440 × 1000, using the original DejaVu Sans fixture and candidate `929d826`. No desktop window or global input was used.

- **p1-text-runs.png** — `Hamburgefonstiv` shaped into 15 glyphs in text mode, with the shared font-unit canvas and 12/24/48 px previews. The sidebar and inspector currently show the generic loading placeholder when there is no active glyph in text mode.
- **p1-text-edit-in-place.png** — The third glyph, `m`, activated in its original run position; only its nodes and local metrics are editable.
- **p1-text-preview-strip.png** — The same active glyph with an independently dark preview strip.
- **p1-text-stale.png** — A real node drag held open on `m`; the previous shaped layout remains visible with “Recompiling…” and the document progress line. Releasing the pointer subsequently returned the preview to fresh without another edit.

The previews draw document outline paths at positions supplied by compiled shaping. These images do not demonstrate hinted raster rendering.
