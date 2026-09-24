# Kernel and CLI acceptance scoped review

Root/Astra reviewed `d0c2df1` and `c2fe856`. Approved for P1 integration.

The release-only performance gate measures real API mutations in a 20,000 populated-glyph corpus, excluding request construction and transport. Warmup, measured sample count and conservative p95 threshold are explicit. The semantic tail checks cumulative movement, a single coalesced history entry and undo restoration, so no-op calls cannot satisfy the gate.

The real-font workflow edits three distinct glyphs, checks dependent invalidation, reparses exported coordinates and verifies layout-table bytes and shaping. Hinting expectations follow the recorded P1 preservation ruling. The scratch workflow uses the current Cargo-built CLI and verifies exported outlines and metrics.

Implementer reports release p95 602.373 us against 4 ms, DejaVu export 581.987386 ms against 5 s, 118 API/CLI tests, eight preservation and eight UFO tests passing, with clean fmt/clippy. These are scoped kernel/CLI results; Source Han release, browser pointer-to-paint, desktop and final integrated P1 acceptance remain outstanding.
