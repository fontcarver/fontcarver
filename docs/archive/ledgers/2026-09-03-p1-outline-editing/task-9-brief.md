### Task 9: Text tab (shaped multi-glyph editing), preview strip, dialogs (New Font, Add Glyphs, Export, Preferences, Recovery)

**Files:** `app/src/views/text/{TextView.tsx, textLayout.ts, PreviewStrip.tsx}`, `app/src/dialogs/{NewFontDialog,AddGlyphsDialog,ExportDialog,PreferencesDialog,RecoveryDialog}.tsx`, tests, e2e `app/e2e/{text.spec.ts, export.spec.ts}`.

**Behaviour:** the Glyph tab becomes a Text tab: type text (or paste) → `text.shape` → glyphs laid out with real advances/kerning; click a glyph to edit it in place with the same tools (the canvas transforms by the run offset); `Mod+←/→` adjust LSB/RSB of the active glyph by 1/10/100 (Shift/Ctrl); Escape toggles text/edit mode; sample-text menu (Latin pangrams, CJK samples, from-font "all encoded"); the bottom preview strip renders the current text at 3 sizes with feature toggles (`liga`, `kern`, `calt` when present) and master switch; `stale` shaping indicator while the background recompile runs. Dialogs: New Font (family, style, upem preset 1000/2048, metrics preset, starter glyph set), Add Glyphs (recipe + charset picker with counts, preview list), Export (format, destination, options from `ExportOptions`, warnings list after export, "reveal in folder"), Preferences (theme, language, autosave interval, snapping/grid, nudge steps — wired to `prefs.get/set`), Recovery (on launch when `session.recover_list` is non-empty).
- [ ] **Step 1: Tests**: text layout from a shaped run; LSB/RSB keyboard editing emits `SetMetrics`; export dialog maps to `session.export` params; preferences round trip.
- [ ] **Step 2: Implement**; e2e `export.spec.ts` covers the P1 exit criterion end to end (open DejaVu → move one point of `A` → export TTF → assert file exists and `fw inspect` reports glyph count; kernel-side test covers preservation).
- [ ] **Step 3:** commit `feat(app): text tab with shaping preview; new-font, add-glyphs, export, preferences and recovery dialogs`.

---

