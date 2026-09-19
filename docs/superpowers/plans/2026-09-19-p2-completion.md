# P2c/P2d implementation plan

Status: complete, 2026-09-19. Design: [P2 completion](../specs/2026-09-19-p2-completion.md).
Acceptance: [P2c/P2d](../../p2cd-acceptance.md).

| Task | Scope | Owned implementation |
| --- | --- | --- |
| C1 | CFF1 writer, charstrings, sfnt conversion, independent-reader tests | `fc-sfnt` write modules; `fc-compile` OTF integration |
| D1 | Glyphs 2/3 importer, Glyphs 3 writer, passthrough and roundtrips | `fc-formats/src/glyphs/`, format tests |
| D2 | SFD importer/writer and native semantic roundtrips | `fc-formats/src/sfd/`, format tests |
| D3 | Type 1 PFA/PFB reader/writer and roundtrips | `fc-formats/src/type1/`, format tests |
| D4 | Structured problem scanner, bounded query and CLI | model/doc scan modules, API problem types/handler, CLI check |
| I1 | Shared format guards, API/CLI import-export and UI integration | shared registration, export types/handlers, existing dialogs |
| I2 | Independent Chinese localization, browser verification and review | locale, end-to-end tests, acceptance evidence |

Implement and verify format kernels before exposing their targets. Keep shared
registration/dependency edits coordinated. Tests must demonstrate native
interoperability rather than only decoding our own metadata. Complete scoped
reviews and an integrated regression gate before updating milestone status.

## Progress

- [x] Audit P2 scope, baseline and reusable dependencies.
- [x] Record design and implementation sequence.
- [x] C1 static OTF/CFF, fractional reader, safe table preservation and vertical metrics.
- [x] D1 Glyphs.
- [x] D2 SFD.
- [x] D3 Type 1.
- [x] D4 Find problems, including independent review regressions.
- [x] I1 API, CLI and UI integration.
- [x] I2 Sol localization, independent review and final acceptance (8/8 browser workflows).
