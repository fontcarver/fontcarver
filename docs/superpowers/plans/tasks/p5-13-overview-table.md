# P5-13 — Glyph overview presentation

Scope: finish feature audit rows 10, 11 and 18 in the existing font overview.

Files: `app/src/views/font/{FontView,GlyphGrid,GlyphCell,GridToolbar,cellRenderer,GlyphTable,GlyphTableHeader,GlyphStatusBadges}.tsx`, supporting table/status helpers and CSS, `app/src/stores/documentStore.ts`, English UI keys, and focused tests. API metadata additions are owned by the glyph-inventory parent task.

Interfaces: per-document `viewMode` (`grid` or `list`) and ordered list columns; list rows use existing `GlyphQuery`, paging, selection and keyboard actions. Sort headers request the kernel's existing ascending Name, Unicode and Width ordering. Column movement is available through drag/drop and accessible buttons. New optional cell metadata drives metrics, annotations, custom information, compatibility and lock indicators; export status is already available. Empty encoded cells may draw a muted system-font reference character without changing font outlines.

Acceptance: write failing focused tests first, then verify grid/list switching, bounded rendering for a large font, selection/range/navigation/opening, API sort requests and column reordering; verify status labels and empty-character fallbacks, including invalid Unicode. Run app typecheck, lint and tests through coordinated parent validation; inspect the UI with agent-browser and record evidence. Check artifact budgets at task boundaries.
