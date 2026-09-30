# Remaining interchange formats acceptance — 2026-09-30

Implemented bounded SVG-font/EPS outline and AFM/PFM metrics codecs (delegated),
raw-table TTX XML, TTC/OTC packing/extraction, standalone CFF1 and declarative
Type42. Public APIs are registered. API/CLI source opening handles each format;
CLI `collection --output PATH INPUT...` packs multiple inputs and expands all
faces of existing collections. Shared export handlers belong to export_workflows.

TTX preserves binary tables, rejects decompiled/split XML, signature invalidation
and ambiguous XML table tags. Collections preserve independent faces with aligned
absolute offsets/checksums, 256-face/128-MiB bounds and signature rejection.
AFM/PFM open with explicit metrics-only warnings. Supported subsets and limitations
are documented in `docs/interchange-formats.md` and the delegated codec reports.

Verification performed:

- Initial `cargo test -p fc-sfnt --test interchange` exited 101 as expected before
  implementation (missing collection/ttx modules).
- Subsequent focused attempt exited 101 on concurrent incomplete fc-model
  node-link definitions, before these tests ran.
- Final queued retry was cancelled, exit 143, on root's instruction to leave
  Cargo exclusively to integration. No acceptance test pass is claimed here.
- Scoped rustfmt and `git diff --check` passed. Owned scratch removed; no owned
  Cargo processes remain. Final artifact check passed: build 6.09 GiB/13,176
  files, shared temporary 0.22 GiB/8,090 files, free 668.67 GiB.

Root integration acceptance targets (already authored, not additional scope):

```
cargo test -p fc-sfnt --test interchange --test cff_interchange
cargo test -p fc-formats --test vector --test vector_eps --test metrics --test afm_bridge --test pfm_metrics --test pfm_font --test type42
cargo test -p fc-formats --lib vector::eps::tests
cargo test -p fc-cli --test collection --test interchange
```

Independent decoder tests use installed fontTools/Ghostscript when available;
source reading never executes PostScript. No FontForge source was read/copied or
executed. Existing permissive roxmltree is now directly reused, recorded in
THIRD_PARTY.md. Workspace Clippy and full acceptance remain owned by root.
