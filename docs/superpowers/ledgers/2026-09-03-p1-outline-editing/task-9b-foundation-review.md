# T9b foundation review — 2026-09-07

Reviewer: Astra/root. Reviewed new modules and tests at 0db4f11. Core interfaces are appropriate: compiled IDs are never reused as document handles; exact name lookup, byte-boundary mapping after // contraction, explicit UPEM, last-good bounded cache and generation-scoped warnings address the early review feedback. Implementation reports 14 focused / 705 full tests passing; root source-reviewed the tests and kernel search ranking, not yet rerun final tests.

Verdict: changes required before UI integration.

1. Explicit RTL only applies within Unicode chunks; buildTextLayout concatenates tokens in input order, so /A /B remains A,B and mixed named/literal chunks are misplaced. Respect explicit RTL token visual order while preserving original source byte positions. Test all-name and literal/name/literal runs. Full Unicode bidi paragraph analysis is not newly required here.
2. Promise.all launches unbounded glyphs.search/glyphs.cell calls for every distinct name, including the required All encoded glyphs sample (potentially 65k calls, each scanning the font). Bound normal-text concurrency; for large sets use paginated glyphs.list with exact name matching and chosen master, include_paths:false. Propagate cancellation into layout building so a superseded shape response does not start its entire lookup phase. Test bounded fanout and cancellation before mapping/while pool jobs remain.

Send back to the Sol implementer as a focused followup commit. No UI integration, dependency or Rust changes required for these two corrections. Separately, root scheduled compiled identity retention for post3 fonts; safe unnamed diagnostic remains an appropriate fallback.
