# P5 PFM metrics codec acceptance

Implemented Windows v1 PostScript metrics read/write: header/extended metrics,
byte-preserving names/copyright, widths and signed kerning. All regions receive
bounds/overlap checks; unsupported fields, count mismatches and unknown nonzero
payload fail explicitly. The delegated Font adapter imports byte-addressed
glyphs, preserves original metadata and exports metrics atomically.

References: Microsoft Windows DDK Printer and Font Kit chapters 2/4, as linked
in the task plan; Microsoft FONTDIRENTRY for the DWORD file-length field.
No FontForge code, execution, new dependency or redistributed external fixture.

Tests were written first. Initial compilation was blocked by concurrent
`fc-model` E0560 (`Layer::metrics_keys`), so no behavioral-red result is claimed.
Acceptance command:
`TMPDIR=$PWD/.scratch/p5-pfm-codec/tmp flock .scratch/p5-resume/cargo.lock cargo test -p fc-formats --test pfm_metrics`.
The acceptance command remained queued and was cancelled at the integration
owner's explicit request (SIGTERM/exit 143, no Cargo child had started); it did
not run and is not passed. Root owns final tests:
`cargo test -p fc-formats --test pfm_metrics --test pfm_font` (6 binary tests and
11 adapter tests), Clippy and broad checks. Narrow rustfmt passed. Independent
read-only review found no blocking offset/count or supported-data-loss issue.

Boundaries: metrics only, 1000 UPEM, v1 PostScript, 8-bit codes, 255-byte names,
1 MiB input. New model encodings are ASCII Windows ANSI; imported charset bytes
are retained without Unicode inference. PCL, track/origin metrics and extended
PFM variants are rejected. Byte layout is canonicalized; original zero padding
and kerning order are not retained as byte-identity promises.

Artifact start: 5.23 GiB build, 670.47 GiB free, PASS.
Owned scratch (876 KiB primary-reference downloads and empty TMPDIR) removed;
no build cache or other task's output was touched. No owned processes remain.

Final `python3 scripts/check_artifacts.py` output:
```text
build: 6.12 GiB, 13,181 files
temporary: 0.21 GiB, 7,704 files
workspace free: 668.73 GiB
PASS
```
