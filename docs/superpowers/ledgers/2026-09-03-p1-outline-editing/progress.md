# SDD ledger — plan: docs/superpowers/plans/2026-09-03-p1-outline-editing.md

Spec: docs/superpowers/specs/2026-09-03-fontworkshop-design.md (binding). Branch: p1-outline-editing (from main cd12fd6). Parallel kernel tasks run in .worktrees/p1-t1 (fw-geom), p1-t3 (fw-compile), p1-t4 (fw-formats), p1-t10 (P0 kernel backlog) — disjoint crates; T2 waits for T1; T5 waits for T2/T3/T4/T10.

## Pre-flight scan (2026-09-03)
| Pair / task | Produces vs consumes | Finding |
|---|---|---|
| T1 → T2 | fw-geom fns (remove_overlap, split_segment, transform, simplify…) consumed by fw-doc PathOp/InsertPointOnSegment | consistent |
| T2 → T5/T6 | Command variants, PointRef, EditToken flow | consistent; T5 exposes them through doc.apply/edit_*; T6 emits them |
| T3 → T5 | fw_compile::export / compile_static; ExportOptions/ExportReport | consistent (session.export, text.shape via compile_static) |
| T4 → T5 | read_ufo/read_designspace/write_ufo registered in Session::open_path + export | consistent |
| T10 vs T1/T3/T4 | T10 touches fw-sfnt/fw-doc/fw-model/fw-api/src-tauri/fw-devserver/fw-glyphdata; T1/T3/T4 create new crates only | disjoint except Cargo.toml/lock (merge by hand) — **Ruling:** T10 must not touch fw-doc command.rs (T2 owns it) nor add new API methods (T5 owns METHODS). |
| T6 ↔ T7 ↔ T8 | editor core → tools → panels build on selection/editSession | sequential |
| T11 vs T6 | UI backlog touches views/font + shell + playwright config; T6 touches views/glyph + editor/ | disjoint enough for a worktree; merge before T8 |
| Process | models | **Ruling:** implementation seats Opus or Codex (sol xhigh); translation/QA Codex; reviews cross-vendor; Sonnet only for review seats. |

## Task log
- T1, T3, T4, T10 dispatched to Codex in parallel worktrees (base cd12fd6).
- T1: implemented by Codex (76c3fbf on p1-t1, DONE, 27 tests). Review dispatched (Opus).
- T10: implemented by Codex (cd12fd6..4ca6cab on p1-t10, 16 commits, DONE). Review dispatched (Sonnet).
- T4: implemented by Codex (edbda79 on p1-t4, DONE, 9 tests; concerns: opaque-but-lossless UFO metadata, discrete axes as min/max envelope). Review dispatched (Opus).
- Ruling (user, 2026-09-03): UI design is Fable's (Opus fallback), never Codex; UI implementation Opus only; Codex limited to kernel/CLI/mechanical wiring. Fable to write docs/design/editor-ui.md + mockups before P1 UI tasks T6–T9.
- T3: implemented by Codex (7c4a99d on p1-t3, DONE, 10 tests; DejaVu 0.51 s, SHS 41 s; concern: cmap format 12 / post format 3 for large fonts due to fontc capacity limits). Review dispatched (Opus). User confirmed the editor design spec direction ('设计的还可以，保持').
- T4 review (Opus): Needs fixes — 7 Important (designspace axis-less write unreadable, brace-layer duplication, custom-layer leak, per-master fontinfo lost, data/images dropped, extras blob duplicated 2.5 MB, duplicate master filenames) + minors. Ruling on #1: write_designspace returns Error::NoAxes for axis-less fonts (UFO path instead). Fix round 1 dispatched to Codex.
- T1 review (Opus): Needs fixes — Critical: area oracle NonZero vs EvenOdd + self-compare fast path (breaks Round stroke on closed cornered contours and remove_overlap on same-winding nested contours); Important: remove_internal/external not independent, convex-nib tracer flattens and ignores cap/join, test gaps. Fix round 1 dispatched to Codex.
- T10: complete (review Approved; merged as 46099dd). Parked: CSP is only enforced in production builds (devUrl bypasses it) — note added; double_option helper duplicated at API boundary (minor).
- T3 review (Opus): Needs fixes — Critical: instructions re-attached to recompiled outlines (silent hinting corruption), positional gid mapping, first glyph renamed to .notdef; Important: gid-indexed raw tables copied after glyph-set change, name languages collapsed, provenance-less layout drop with wrong warning, UVS dropped silently, large-font post-3/threshold; minors. Rulings: verbatim glyf/loca/maxp when untouched (no re-injection), name-based gid map, SourceProvenance.glyph_order (additive fw-model + fw-sfnt), always drop variation tables in static export, UVS warning only (P2). p1-t3 rebased onto 46099dd; fix round 1 dispatched to Codex.
- T4 fix round 1/5: Codex 52072eb (19 tests); scoped re-review dispatched (Sonnet).
- T1 fix round 1/5: Codex f03285d (42 tests); scoped re-review dispatched (Opus).
- T4: complete (edbda79..52072eb, review clean after fix round 1; merged as 525b738; deferred low: per-master guide identifier alignment when guide counts differ; redundant data/images re-read).
- T4 merged as 525b738; integration fix 03ed42f (fw-formats NamePlatform::Other arms after the T10 merge; controller merge-resolution work). Workspace 221 tests green.
- T1 re-review round 1 (Opus): all 5 addressed (B-counter clause ruled contradictory — NonZero semantics kept); NEW Important regression: closed-contour strokes polygonalised; minor oracle-before-correct_direction; observation: remove_overlap strict fails 347/3583 untouched DejaVu glyphs, stroke 409/3583. Ruling: round 2 fixes the regression, makes the fast path exact for non-crossing glyphs, and adds remove_overlap_with_fallback (flattened i_overlay + refit) so the feature never errors on real fonts. Dispatched to Codex.
