# P5 export workflows (rows 58, 60, 63)

Scope: durable export profiles, an instance/profile export matrix, preflight problem navigation, real export-only geometry preprocessing, and CLI/API parity. Existing formats must reject options they cannot honor; integrate additional codecs delivered by the remaining-formats owner.

Ownership: `fc-api/src/types/export.rs`, `fc-api/src/handlers/export.rs` and export workflow modules; CLI build/convert; export dialog components. Delegate export UI/profile persistence and compile preprocessing as disjoint files. Coordinate generated API bindings and dispatch with the parent. Do not edit shared status/audit files or translations.

Implementation plan:
1. Add tests for export geometry changes on copies, source preservation and incompatible options; implement extrema insertion, coordinate rounding and component decomposition on static compilation copies. Share repeated static CFF1 charstrings through global subroutines and reject unsupported containers and variable CFF2.
2. Add persisted named UI export profiles and a bounded Cartesian instance/profile batch workflow with collision validation and individual reports.
3. Add preflight results using existing problem APIs and glyph navigation before export.
4. Expose compile flags through the API and CLI, integrate remaining format writers, and test selection and option validation without mutating source documents.
5. Run focused tests under the shared Cargo lock, record acceptance boundaries, and check resource budgets. Parent owns final broad regression checks.

Acceptance: geometry preprocessing measurably changes exported copies, leaves source/history intact and refuses unsupported variable operations; saved profiles survive remount; matrix output paths are unique and failures are visible; preflight findings navigate to glyphs; CLI/API exercise the same option mapping. CFF1 subroutinization must preserve decoded outlines and hint events; Type 1 and CFF2 remain explicit unsupported cases.
