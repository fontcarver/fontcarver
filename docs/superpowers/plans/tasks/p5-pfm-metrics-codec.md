# P5 bounded PFM metrics codec

Owner: delegated `pfm_codec`; integration owner: `metrics_codecs`.

Implement `metrics::pfm::{PfmMetrics, read_pfm, write_pfm}` and public header,
extended-metric and kerning records. Windows v1 PostScript printer metrics use
a 117-byte header, 30-byte extension, 52-byte extended metrics, contiguous
8-bit widths and signed pair kerning. Preserve complete header scalar values,
raw byte strings and copyright bytes. Reject unsupported origin/track tables,
reserved/bitmap fields, overlapping or invalid ranges, duplicate kerning pairs,
bad counts and unclaimed nonzero bytes. Bound file size to 1 MiB and names to
255 bytes. Require PostScript driver, font type 0x81 and 1000-unit extents.

Own `crates/fc-formats/src/metrics/pfm.rs` and `tests/pfm_metrics.rs`.
Subtask `font_adapter` owns `metrics/pfm/font_adapter.rs`, `tests/pfm_font.rs`
and its separate plan/report: standalone Font import plus atomic selected-master
metrics export. Parent owns module registration and final workspace checks.

Reference: original Microsoft Windows 3.0 DDK Printer and Font Kit,
chapters 2 and 4, preserved at
https://www.pcjs.org/documents/books/mspl13/win/w3ddkprn/ ; Microsoft
FONTDIRENTRY documentation confirms the 32-bit `dfSize` header field:
https://learn.microsoft.com/en-us/windows/win32/menurc/fontdirentry .
No FontForge code or executable is used. No new dependency or external fixture.

Acceptance: hand-assembled independent fixture; semantic read/write roundtrip;
signed kerning and byte names; offset/count/truncation and unsupported-field
rejections; mutation panic-safety. Write tests first and record initial missing
API failure. Run focused Cargo tests with shared flock and task TMPDIR; narrow
rustfmt only. Artifact checks at start and completion; remove owned scratch.
