# P5 hinting checkpoint — 2026-09-22

Status: the first hinting slice is implemented and verified. P5 and the complete
hinting milestone remain open. The complete exit audit is
[p5-feature-audit.md](p5-feature-audit.md); user instructions are in
[user/hinting.md](user/hinting.md).

## Implemented

- `fc-hint`: byte-preserving TT assembler/disassembler covering all 256 opcode
  bytes, bounded PS straight-edge/curve-extrema analysis, blue/stem inference,
  and an optional out-of-process ttfautohint bridge.
- PostScript editor in the glyph inspector and Font Info masters: stems, ghosts,
  replacement/counter masks, closing-edge masks, zones and private parameters.
  Preview is read-only; applying a draft is revision-checked and one undo step.
- CFF1 import/output of representable PS hints and private values. Missing hints
  due to unrepresentable transformations are reported. Multiple CID private
  dictionaries produce a distinct limitation warning.
- Static sfnt export options for overlap removal and automatic hinting, available
  in the UI and shared CLI. Sources remain unchanged. Topology-changing overlap
  removal checks retained point-dependent layout before returning an output.
- Native mask fields documented with backward-compatible defaults. Geometry edits
  invalidate hints on affected sources/components while preserving independent
  master and background layers; undo restores hints and geometry together.

## Verification

- Workspace Rust run: **1,010 passed, 2 ignored**, explicitly excluding
  `licensed_real_font_inventory_reports_actual_coverage_without_mutation`.
  This is the previously recorded Tibetan layout expansion-budget failure, still
  scheduled for P5 diagnosis. No unrestricted green workspace is claimed.
- Two later regression tests pass: curved-bowl stem detection and invalid active
  overlap masks. Latest focused compile validation passes **41 tests** across
  hinting, static exports and point-dependent layout preservation. There are
  **1,012 distinct passing Rust tests** across the workspace and later additions.
- Frontend: **1,118 tests passed in 137 files**. TypeScript, ESLint, Prettier,
  workspace Clippy with denied warnings, Rust formatting and production build
  pass. The existing large JavaScript chunk warning remains.
- Embedded Linux desktop build passes with the production assets (`cargo build
-p fontcarver --features tauri/custom-protocol`). The desktop was not launched.
- Headless browser session `fontcarver-p5-hints20260922`: edit/apply/save/export
  and reopen a 70-unit stem, inspect the outline overlay, analyze/cancel and
  reopen without changing the source, export automatic hints and independently
  confirm the source remains at 70 while the exported stem is 67. The session
  was closed. Verified field-layout screenshots are
  `/tmp/fontcarver-p5-ps-editor-fixed.png` and
  `/tmp/fontcarver-p5-ps-fields-fixed.png`.
- `scripts/p5_hint_acceptance.py`: independent FontTools decoding and FreeType
  rendering for Source Sans/Serif CFF exports and a Noto Sans Latin subset.
  Authored 48-stem output exercises Type 2 stack splitting. Real ttfautohint
  1.8.4 creates **125 glyph programs** per TTF/WOFF/WOFF2 export, with unchanged
  coordinates, glyph mapping and horizontal metrics. **2,460 glyph/size renders**
  pass. Web containers are decoded with FontTools before rendering because the
  host FreeType cannot load WOFF2 directly. Their rendering digests match TTF.
  A separate browser-produced OTF adds **30 successful renders**.
- Process failure, invalid output and timeout/reaping are tested. The real external
  tool was extracted under `/tmp/fontcarver-p5-ttfautohint`, not installed globally
  or bundled. Evidence is under `/tmp/fontcarver-p5-hint-acceptance`.

## Remaining hinting work

Type 1 path-bound hint import/output; TT source-index binding, instruction/CVT/fpgm/prep
editing, authored-program export and hinted preview; variable hinting; multiple
editable CID private dictionaries; broader PS analysis quality and format-aware
handling of diagonal hints, active ranges and flex. These remain open and are
not represented as completed checklist rows. The other P5 categories have not
been completed by this checkpoint.

The implementation uses existing permissive dependencies and original code.
No FontForge source or executable was used. The optional external hinter is a
separate user tool; its upstream manual is linked in the user documentation.

## Type 1 follow-up

The next slice retains the supported private parameters and whole-glyph H/V stems
in native PFA/PFB. Fractional stem operands and ghost sentinels survive round trips.
Import adds the charstring sidebearing to vertical stems and applies positive,
axis-aligned FontMatrix normalization. A bounded metadata walker follows ordinary
subroutines and division without executing PostScript. The private parser retains
literal values and distinguishes glyph names from private dictionary keys.
Runtime private-field computation is not executed or mistaken for a constant: the
affected field is omitted and disclosed. The Nimbus Sans fixture exercises this
with interpreter-dependent StemSnap compatibility procedures.

The real `n019003l.pfb` fixture exposed a shared validation bug: BlueValues pairs
need not be globally sorted. Validation now checks each low/high pair and all
pairwise overlaps while preserving the baseline pair and original zone order.
This follows section 5.3 of the [Adobe Type 1 specification](https://adobe-type-tools.github.io/font-tech-notes/pdfs/T1_SPEC.pdf).

Complex glyph hint mechanisms remain unsupported. Import warnings persist in
source metadata and appear through the API and CLI; FCF save/reopen retains them.
Unrepresentable transformed private values are preserved as source metadata.
The exporter rejects unsupported masks, flex, diagonal hints, ranges and overlapping
stems before replacing any destination. TrueType instruction loss is reported
when present. Type 1 automatic hinting and overlap preprocessing remain open.

Focused verification includes an independently encoded source with a nonzero
sidebearing, subroutine stems and a translated/nonuniform FontMatrix, reserved
private-key glyph names, invalid/computed private values, reflected private zones,
real Nimbus Sans data, and FontTools operand decoding plus FreeType rendering at
six sizes for both PFA and PFB. This does not close the complete hinting milestone.

Follow-up verification:

- Related Rust crates (`fc-model`, `fc-formats`, `fc-sfnt`, `fc-hint`, `fc-api`,
  `fc-cli`): **527 passed, 0 failed, 2 existing ignored**, with serial test
  execution. This includes **29 Type 1 tests** and persistent API import warnings.
  The previously recorded `fc-layout` Tibetan case is outside this run.
- FontTools checks exact emitted stem operands and private values. FreeType
  performs **12 native-hinted and 12 unhinted renders**, with a confirmed rendering
  difference at tested pixel sizes.
- Frontend: **1,118 passed in 137 files**, using two workers. Earlier concurrent
  runs exceeded existing wall-clock limits; isolated repeats and the final full
  run pass without changing test timeouts. The 65K CJK search/preview recheck takes
  approximately **449/155 ms**, respectively, and the serial Rust run also passes.
- Workspace Clippy with denied warnings, Rust formatting, TypeScript, ESLint,
  changed-file Prettier and `git diff --check` pass.
- Evidence: `/tmp/fontcarver-p5-type1-regression.log`,
  `/tmp/fontcarver-p5-type1-frontend-final.log`,
  `/tmp/fontcarver-p5-type1-independent.log`, and
  `/tmp/fontcarver-p5-type1-cjk-recheck.log`.

## TrueType binary foundation

`fc-sfnt::tt` now reads actual compiled point order, including off-curve controls
and expanded components, and computes versioned bindings over geometry and
metrics. Bindings include component dependencies and metric-selection flags, but
exclude instruction bytes. Reads reject cycles, truncated streams, conflicting
component transforms and excessive expansion. Empty, header-only and explicitly
zero-contour glyphs share equivalent point identities.

The instruction writer checks bindings before replacing all glyph programs and
CVT/fpgm/prep. It preserves coordinate and component encoding, rebuilds loca and
padding, updates interpreter maxima and removes obsolete signatures and device
metric caches. Unsupported point bindings and malformed pushes fail before output
is returned. Glyph instruction limits and interpreter maxima are bounded. Runtime
program correctness is not inferred from successful bytecode framing.

This is a library foundation, not a completed editor or export-pipeline feature.
Model persistence, revision-checked API, undo/redo, UI and preview integration
remain open. Variable inputs are rejected. Point views currently reject the rare
case of components aligned through phantom points; ordinary point matching works.
Reported horizontal phantom coordinates are nominal parent hmtx coordinates:
`USE_MY_METRICS` is exposed separately because the interpreter can replace them.

Verification:

- `cargo test -p fc-sfnt -- --test-threads=1`: **57 passed**, including **16** TT
  binding/writer/interpreter tests. Clippy with denied warnings, Rust formatting,
  Python Ruff checks/formatting and `git diff --check` pass.
- The `tt_instruction_acceptance` example and
  `scripts/p5_tt_instruction_acceptance.py` independently compare **6,253** DejaVu
  Sans outlines, **252** indexed point arrays and **3** exact authored programs.
  FontTools also checks globals and maxima. Native FreeType 2.13.2 and 2.14.3 each
  complete **36** hinted/unhinted renders, with ordinary and composite programs
  affecting native outlines compared with an otherwise identical control font.
- Zero-contour instructions are legal under the
  [glyf specification](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf),
  but FontTools 4.63 skips parsing them and both tested FreeType versions skip
  executing them. The Python check reads their exact bytes through the raw table
  reader. A separate pedantic Skrifa interpreter regression proves that the
  authored phantom-only program increases an empty glyph's advance by one pixel.
  Native FreeType execution of that case is **not** claimed.
- FreeType 2.14.3 was built only under `/tmp/fontcarver-p5-freetype-2.14.3` for
  verification; the system library and application dependencies are unchanged.
- Evidence: `/tmp/fontcarver-p5-tt-regression-final.log`,
  `/tmp/fontcarver-p5-tt-clippy-final.log`,
  `/tmp/fontcarver-p5-tt-native-2.13.2.log`,
  `/tmp/fontcarver-p5-tt-native-2.14.3.log` and
  `/tmp/fontcarver-p5-tt-phantom.log`.
