# P2d-04 Find problems kernel

Implement the Find problems section of `../../specs/2026-09-19-p2-completion.md`.
Own `fc-doc/src/problems.rs` (or folder), API `types/problems.rs`,
`handlers/problems.rs`, relevant tests, and `fc-cli/src/cmd/check.rs` if useful.
Parent owns shared mod/dispatch registration; communicate exact changes needed.

Provide a deterministic read-only bounded scanner, stable codes/severity, glyph
and master/layer/contour locations, total/truncated indication and document
generation. Cover existing model validation plus invalid Unicode, malformed/open
contours/nonfinite geometry, component cycles, invalid widths, kerning references
and groups. Add compiler feature validation in API where dependency allows.
API method `doc.find_problems`; params `doc`, optional `limit` (default 200,
maximum 2000). Results include `generation`, `issues`, `truncated` and optionally
total. Each issue: `code`, `severity`, `message`, nullable `glyph`, `master`,
`layer`, optional contour/point index. Tell parent/frontend final wire contract.

Acceptance: positive/negative scans, deterministic order, bounded results, no
document mutation, large/cyclic references terminate, API and CLI behavior.
Read conventions; focused tests and Clippy. Do not commit independently.

## Completion evidence

Implemented deterministic structural diagnostics, generation-bound API responses
and CLI exit/error behavior. Independent review corrected active-source validation,
generator truncation and exponential component expansion. A shared bounded graph
preflight also protects source-export validation; selected-master SFD validation
does not require unrelated masters to be complete.

Focused checks: 8 document scanner tests, 7 API problem tests and 2 CLI check
tests passed. API export regressions passed 15 tests after integration review.
The independent bounded-memory DAG reproduction now returns a diagnostic without
entering the compiler. Final integrated gates are recorded in
[`docs/p2cd-acceptance.md`](../../../p2cd-acceptance.md).
