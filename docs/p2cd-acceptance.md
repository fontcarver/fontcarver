# P2c/P2d acceptance

Date: 2026-09-19. Status: complete within the documented static scope.

Scope: the remaining P2c/P2d work from the binding roadmap, following
[the completion design](superpowers/specs/2026-09-19-p2-completion.md) and
[implementation plan](superpowers/specs/2026-09-19-p2-completion.md#implementation-plan).
P2a and P2b remain the accepted authoring baseline. Implementation, independent
review, isolated browser acceptance and Sol Chinese localization used separate
agents, as requested by the user.

## Delivered behavior

- Static OTF/CFF1 export from the selected master, preserving fractional cubic
  outlines and raising quadratics exactly before bounded 16.16 quantization.
  Components are flattened, glyph order matches the compiler, and CFF charset,
  widths, FontMatrix, final bounds, horizontal and vertical metrics are rebuilt.
- Fractional CFF import no longer truncates coordinates through the integer
  OutlinePen adapter. CID input, subroutines and default CFF2 evaluation retain
  the existing reader's supported semantics.
- Imported layout is preserved only when safe for replaced CFF geometry.
  OpenType and AAT point-index dependencies, stale variation references and
  malformed counted arrays are rejected instead of copied silently.
- Native Glyphs 2/3 import and Glyphs 3 export; static SFD import/export;
  native encrypted PFA/PFB import/export. API, CLI, file opening and export UI
  expose every new format. Glyphs carries all masters; SFD and Type 1 export
  the selected master. Native fields take precedence over retained extras.
- Read-only Find problems via Font menu, command palette, `doc.find_problems`
  and `fontcarver check`. Results include severity, stable codes, locations,
  generation, total/error state and truncation; glyph issues navigate to the
  editor. Unsaved edits settle before scanning; stale results require refresh.
- Unapplied feature drafts block all external outputs. Source formats retain
  the imported-layout conversion guard and active FEA validation. Type 1
  discloses losses. New single-file writers stage output and replace atomically.

## Independent review regressions

- CFF `.notdef` production-name collision, absolute control-point overflow,
  maximum charstring length, post-quantization bounding boxes and aggregate
  right-side-bearing validation now have explicit tests.
- Preserved OpenType table readers cannot skip malformed arrays or optional
  variation pointers through truncated headers. Vertical metrics use final CFF
  bounds, including empty outlines and edited vertical origins.
- Find problems validates saved active FEA exactly; regenerated candidates
  cannot mask an invalid saved rule. Generator diagnostics propagate truncation.
  An acyclic exponential component DAG is rejected before compiler allocation.
- Returning to a document after discarding an in-flight scan response no longer
  leaves its dialog permanently busy.
- Glyphs preserves generated feature identities and user-space mapped-axis
  bounds, honors native external edits, and rejects unsupported component/paint
  semantics. Real Glyphs 2/3 and a 95-glyph Yarndings source are covered.
- SFD uses canonical kerning groups and membership, escapes FEA keyword names,
  preserves explicit-versus-generated kerning ownership, and refuses unsupported
  additive lookup/anchor semantics. Selected-master validation ignores unrelated
  incomplete masters. Source exports share bounded component preflight.
- Type 1 avoids duplicate Unicode assignments for unencoded alternates, derives
  bounding boxes from emitted quantized paths, reads native matrix values without
  UPEM drift, and preserves Notice/copyright. Unsupported export options emit
  warnings rather than being silently ignored.
- Shared FEA validation estimates class/rule expansion before semantic or full
  compilation, including direct binary compilation. Independent review verified
  class aliases, numeric ranges with shared suffixes and compact class kerning.
  Both previously aborting FEA inputs now return `feature-source-limit` under
  the same 512 MiB process limit; this was an isolated test limit, not host OOM.

## Explicit format boundaries

This is static name-keyed CFF1 output, not variable CFF2 or CID output. CFF
charstrings are unhinted and unsubroutinized; output reports hint loss. Supported
CFF advances are 0–32767 font units; unrepresentable coordinates, widths or
charstrings fail clearly. WOFF/WOFF2 remain TrueType-flavored. PostScript
autohinting and export overlap removal are not newly implemented.

Unsafe preserved layout requires validated source conversion or explicitly
turning off preservation; retained JSTF is conservatively rejected until its
embedded positioning is supported. No unsafe point references are dropped
without an error merely to make OTF export succeed.

SFD scope is static outline fonts, not SFDir, CID, bitmap or variable projects.
Unsupported layout/behavior is rejected explicitly. Type 1 does not carry
OpenType layout, modern Unicode mapping, editable source layers or original
hints; limitations are disclosed before export and in the export report.
Glyphs smart/conditional/color semantics are not claimed as general support.
SFD contextual/state-machine layout, device adjustments, multiple native editable
kerning lookups/subtables, mkmk/conflicting anchor roles and TrueType instructions
are explicitly rejected. Type 1 supports static horizontal fonts; stroked/MM
fonts, `sbw`, metric overrides and unsupported advance matrices are rejected.
Type 1 also rejects unverified large-integer arithmetic patterns in native
charstrings/subroutines rather than allowing the locked reader to overflow;
its portable output coordinate/advance limit is 32,000 units.

Find problems v1 is a structural/feature validator, not an exhaustive design
quality, overlap, rasterization or interpolation audit. Results default to 200
and are capped at 2000. Before compiler validation, expansion is bounded to
100,000 point/node records per exported master layer, 5,000,000 per font and
component depth 64; violations produce diagnostics and skip compilation.
FEA work estimates allow at most 100,000 records per class/rule and 1,000,000
in aggregate. Compact non-enumerated class kerning is not expanded as a Cartesian
product by this check. The limits are conservative input guards, not a claim
that arbitrary font compilation is a resource-isolated sandbox.

## CLI examples

```sh
fontcarver build source.ufo output.otf
fontcarver convert source.glyphs selected.sfd --master Regular
fontcarver build source.fcf output.pfb --master Regular
fontcarver check source.glyphs --json --limit 200
```

Source export from imported OpenType layout still requires validated layout
conversion; it is not an implicit lossy fallback. `check` returns exit code 1
for errors, including errors omitted from a truncated response.

## Verification evidence

The following integrated batches passed; final focused reviews cover the later
FEA work-budget and Type 1 arithmetic guards without double-counting reruns:

| Gate                                                                                                                                                                                | Result                                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `cargo test -p fc-compile -p fc-doc -p fc-model -p fc-geom -p fc-layout -p fc-sfnt -p fc-glyphdata -p fc-devserver -- --skip source_han_export_meets_release_budget_when_available` | 592 passed; 1 explicitly filtered performance gate                                                     |
| `cargo test -p fc-api -p fc-cli -p fc-formats`                                                                                                                                      | 304 passed; 2 existing release-only acceptance tests ignored                                           |
| Feature validation/generation/source/writer/export/work-budget focused suites                                                                                                       | 30 passed; final 5 work-budget cases passed again under a 512 MiB process limit; CID range unit passed |
| Final Type 1 independent rerun                                                                                                                                                      | 19 integration + 1 arithmetic-guard unit passed; external native fontTools/FreeType probes passed      |
| Final `cargo clippy --workspace --all-targets -- -D warnings`                                                                                                                       | Passed, including desktop shell type-checking                                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                    | Passed                                                                                                 |
| API generation                                                                                                                                                                      | Passed; generated files remain uncommitted/ignored by policy                                           |
| `pnpm test`                                                                                                                                                                         | 128 files / 1,096 tests passed; final localization/export warning addition passed 16 focused tests     |
| `pnpm check`, `pnpm lint`, `pnpm build`                                                                                                                                             | Passed; existing Vite large-chunk advisory remains                                                     |
| Sol localization                                                                                                                                                                    | 1,184 leaf keys in each language, no missing/extra keys or placeholder mismatches                      |
| `pnpm exec playwright test --config e2e/p2cd.config.ts`                                                                                                                             | 8 passed (1.0 minute), against the final rebuilt backend                                               |

The final browser suite covers OTF/Glyphs/SFD export and reopening, all-master
Glyphs versus selected-master SFD output, both Type 1 containers and loss
disclosures, all-five-format draft barriers, problem navigation/refresh, and
delayed stale scan responses after glyph deletion. An earlier run was blocked
by a recovery dialog left by test autosave; the suite now seeds disabled autosave
in its own temporary preferences before server startup. The full eight-case
rerun passed without a product-code change for that harness issue.

Four named-session agent-browser screenshots were opened and visually inspected;
see [captions and images](screenshots/p2cd-captions.md) for English/light and
Chinese/dark export/problem dialogs.

No performance investigation, FontForge execution/comparison, native desktop
input automation, release packaging, push or main-branch merge is part of this
acceptance. Optional local font corpora are read only and are not redistributed.
Dependency reasons/licenses are recorded in [THIRD_PARTY.md](../THIRD_PARTY.md).
