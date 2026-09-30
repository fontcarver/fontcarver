# P5 AFM metrics interchange acceptance

Implemented typed horizontal AFM parsing/writing and delegated Font adapters.
Preserved properties, comments, codes, widths, optional bounds, ligatures and
direct kerning; added strict record counts, framing, duplicate detection, ASCII
and finite-number validation. Explicit unsupported-data errors cover vertical
metrics, composites, track kerning and unknown extensions. The model bridge
preserves source metadata, scales advances/kerning, computes component bounds,
applies partial changes atomically and replaces destination files atomically.

Nine codec tests cover handwritten fixtures, semantic round trips, malformed
and truncated input, injection, contradictory fixed-pitch declarations, and a
1024-glyph open/export time budget. Nine bridge tests cover model persistence,
scaling, unsupported data, atomic failure and destination preservation.
Independent codec review findings were fixed and given regression coverage.

No FontForge source or execution, new dependencies or external fixtures.
Reference: [Adobe AFM Specification 4.1](https://raw.githubusercontent.com/adobe-type-tools/font-tech-notes/main/pdfs/5004.AFM_Spec.pdf).

Tests were written before implementation. Initial command failed in concurrent
`fc-geom` work (`VariableStroke` match not yet exhaustive), so no behavioral-red
claim is made. The queued codec test was cancelled before execution at root's
request (exit 143, no output), NOT passed. The bridge test reached compilation
and failed (exit 101): AFM `GlyphName.as_str()` calls were corrected to `as_ref()`;
shared `fc-compile` also had an unrelated `Validate` import error. No tests ran.
Parent owns final integrated Clippy and the acceptance command:
`cargo test -p fc-formats --test metrics --test afm_bridge --test pfm_metrics --test pfm_font`.
No further Cargo commands were launched. Narrow rustfmt passed.

Artifact start: 5.23 GiB build, 670.49 GiB free, PASS. Latest checkpoint:
6.11 GiB build, 0.22 GiB shared temporary, 668.72 GiB free, PASS after cleanup.
Owned `.scratch/p5-metrics-afm` removed; no owned processes remain. Narrow
`rustfmt --check` and `git diff --check` exited 0. Final integration is delegated
to root; no local test or Clippy success is claimed.
