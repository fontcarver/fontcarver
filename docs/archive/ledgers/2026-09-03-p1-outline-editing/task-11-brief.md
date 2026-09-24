### Task 11: P0 backlog (UI) + e2e in CI

Batched Opus run: single `invalidateFor` already landed — extend to `change.masters` (em-box refresh); grid keeps scroll position across full reloads; Font Info live refresh on `change.font_info`; smooth-point display from `Point.smooth` (kernel inference: `fw-sfnt` sets `smooth` when handles are collinear within 1°, done in Task 10 → include here the renderer change); `GlyphGrid.tsx` split under 400 lines; Playwright `webServer` array starts `fw-devserver` + Vite so `pnpm e2e` is self-contained; CI runs `pnpm e2e` on Linux with the fixtures (CJK spec skips); kernel-warning codes mapped to i18n (already), `ApiError.code` → i18n toasts.
- [ ] **Step 1–3:** tests; `pnpm check && lint && test && e2e`; commit `chore(app): P0 backlog and self-contained e2e`.

---

