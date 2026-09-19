# P2c/P2d UI integration

Own frontend changes for export/import format affordances and Find problems.
Read design and engineering/UI conventions. Parent owns Rust/API integration.
New ExportFormat variants: `Otf`, `Glyphs`, `Sfd`, `Pfa`, `Pfb`.
OTF is binary; Glyphs exports all masters; SFD and Type 1 select one master.
File filters, extensions, CLI-equivalent options and export warnings must agree.
Retain existing edit barriers and feature-draft guards. Type 1 format selection
must disclose lack of OpenType layout/hint preservation before exporting.

Add a keyboard-accessible Find problems dialog reachable through menu/palette,
using `doc.find_problems` with doc and bounded limit. Coordinate wire contract
with the problems agent. Show issue severity, glyph location, truncation, refresh
and stale-document state. Glyph selection opens that glyph; avoid stale results
overwriting a switched document. No automatic repairs.

Own English locale keys; do not edit zh-CN (a separate Sol translator owns it).
Use existing components/tokens. Add meaningful component tests for navigation,
stale results, error state and format selection; run check/lint/focused tests.
No desktop automation or user browser profile; parent handles integrated browser
acceptance. Do not commit independently.

## Completion evidence

Implemented all five new export targets, opening filters, extensions and format
loss disclosures. Find problems supports bounded refresh, generation checks,
glyph navigation and document/edit barriers. Independent review found and
verified the fix for switching away during a scan and returning to the document.

Final full frontend suite: 128 files / 1,096 tests passed, including that
regression. TypeScript, ESLint and production build passed. The final warning
addition passed 16 locale/export tests. Sol independently maintained Chinese
translations: 1,184 matching leaf keys with no missing/extra keys or placeholder
mismatches. Named-session browser screenshots and final real-backend workflows
are recorded in [P2c/P2d acceptance](../../../p2cd-acceptance.md).
