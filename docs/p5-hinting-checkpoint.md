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

Advanced Type 1 hint mechanisms; TT source-index binding, instruction/CVT/fpgm/prep
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
when present. At that checkpoint, Type 1 automatic hinting and overlap
preprocessing were still open; the later section below records their completion.

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

At this foundation checkpoint, model persistence, revision-checked API, undo/redo,
UI and preview integration remained open; the next section records that follow-up.
Variable inputs are rejected. Point views currently reject the rare
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

## TrueType static-master editing and export

The model stores imported/authored point bindings and the UPEM at global authoring
time. Native JSON and compressed FCF retain them without original binary bytes.
Geometry edits keep the source available for correction; stale authored programs
fail export. Glyph and global commands form one undoable transaction. Imported
programs are retained alongside authored ones only when their compiled point
identities still match and imported-hint retention is enabled.

The revision-checked TT API compiles outside the document lock, then checks the
revision again before applying. Its separate state keeps malformed TT bytes from
breaking PS editing. The bilingual editor exposes glyph/fpgm/prep assembly, CVT,
maxima, paged on/off-curve point coordinates, nominal phantom metrics and preview
at up to eight pixel sizes. Incompatible imported programs start as reference;
stale authored programs remain editable and require review before rebinding.
Language changes preserve unsaved source. Preview errors retain the draft and
never silently substitute an unhinted result.

Static TTF/WOFF/WOFF2 writes authored programs independently of the imported-hint
retention option. Google WOFF2 normalization discards zero-contour programs even
with transforms disabled, so this case now uses an original null-transform
encoder implementing the W3C WOFF2 directory/header format with the existing
Brotli 7 dependency. Both FontTools and Google's decoder reproduce every table
and program byte (excluding the recomputed head checksum). Other fonts retain the
ordinary compressed-transform path.

Verification:

- Related model/hint/sfnt/doc/compiler/API regression: **733 passed, 2 existing
  ignored** before the new editor handlers. Subsequent targeted checks cover
  **7** authored-export cases, **6** instruction/import cases, and **5** API
  workflows, including native save/reopen and destination protection on failure.
  The final complete API/raster regression passes **232 tests, 2 existing ignored**.
- Frontend full regression: **1,123 passed in 138 files**. The subsequent language
  preservation test passes with all **6** TT editor tests and **10** transport
  tests. TypeScript, ESLint, workspace Clippy with denied warnings, formatting and
  whitespace checks pass.
- Headless browser verification authors and previews an actual program, applies
  and reopens it, switches to Chinese, and preserves the saved source. Independent
  FontTools decoding verifies exact browser-authored bytes in TTF/WOFF/WOFF2;
  FreeType completes **30** native renders with **15** hinted outline differences.
- The large WOFF2 acceptance also checks **6,253** glyph outlines, **252** point
  arrays, exact programs and **36** native renders. Early composite instruction
  flags and zero-contour/header-only import have explicit regressions.
- Evidence: `/tmp/fontcarver-p5-tt-integration-regression.log`,
  `/tmp/fontcarver-p5-tt-api-tests.log`,
  `/tmp/fontcarver-p5-tt-api-raster-regression.log`,
  `/tmp/fontcarver-p5-tt-editor-frontend.log`,
  `/tmp/fontcarver-p5-tt-editor-final-tests.log`,
  `/tmp/fontcarver-p5-tt-browser-export.log`,
  `/tmp/fontcarver-p5-tt-woff2-independent.log` and browser screenshots
  `/tmp/fontcarver-p5-tt-preview-en.png`, `/tmp/fontcarver-p5-tt-editor-zh.png`.

Remaining boundaries: authored TT output currently rejects variable fonts,
generated instances, smart/bracket resolution, overlap removal and automatic TT
hinting. Recompiled point-dependent raw layout is rejected. Phantom-point component
alignment is not supported in the point view. Skrifa bounds VM execution, uses
pedantic glyph execution but compatibility-mode global initialization, and may
tolerate some global stack errors. Neither preview success nor successful assembly
proves universal interpreter compatibility. FreeType zero-contour execution is
still not claimed. Complex Type 1, per-FD parameters and the rest of P5 remain open.

## Type 1 replacement and counter triples

Standard OtherSubrs 3 replacement sequences now import as point-bound active
masks, including nested local subroutines, cubic endpoints, subsequent contours,
explicit closing segments and closepath edges. Horizontal-first mask bits are
assigned after all stems are known. Repeated stem definitions reuse their identity.
Whole-glyph sets remain compact. The tested outline evaluator already handles
the canonical replacement sequence; no speculative outline adapter was added.

PFA/PFB export validates every active set, writes deduplicated replacement
subroutines and reserves the standard first four subroutine slots. CFF and Type 1
share the model's point-to-path mask mapping, including quadratic implied points
and closing curves. Type 1 additionally validates its quantized 16.16 stem values.
The OtherSubrs 3 callback is original PostScript code implementing the format's
lock-operator selection and old-interpreter fallback; no FontForge or restricted
Adobe routine was copied.

Representable hstem3/vstem3 groups import and export through counter masks.
Triples require positive nonoverlapping stems, equal outer widths and equally
spaced centers, with at most one fixed triple on each axis and no ordinary stems
on that axis. All three members must be active together. General counter control,
flex, dot sections, seac hint inheritance and per-FD dictionaries remain open.
Type 1 export preprocessing is covered in the next section. The full P5 milestone
is not done.

Verification for this slice:

- Full `fc-compile` test binaries: **150 passed**. Full `fc-model`, `fc-sfnt`
  and `fc-formats` regression: **234 passed, 0 failed**. The final `fc-formats`
  repeat after stricter sequence and byte-budget guards: **117 passed** (included
  in that 234 total, not counted twice).
- FontTools verifies the exact replacement call sequences, deduplicated subroutines
  and counter operands. FreeType performs **24 native-hinted and 24 unhinted
  renders** for the replacement/counter fixture across both containers.
- Ghostscript executes both exported fonts and tests the original callback's five
  lock-operator/fallback cases. The callback test makes only its test array
  readable; the exported font retains the required access restrictions.
- Workspace all-target Clippy with `-D warnings`, Rust formatting, Markdown
  formatting and `git diff --check` pass.
- No frontend behavior changed in this slice. Advanced Type 1 mechanisms and the
  rest of P5 remain open; this is not a full-workspace-test completion claim.

## Type 1 export preprocessing

The shared API and CLI now apply overlap removal and geometric PostScript
hint analysis to PFA/PFB export copies. Overlap removal runs first, expands
components and clears stale glyph hints. Rehinting then computes fresh stems and
private parameters. Without rehinting, cleared PS hints produce a localized
warning. TrueType loss reporting inspects the original snapshot, before programs
are cleared. Failed analysis or serialization leaves the destination untouched.

The export dialog exposes only these two processing options for Type 1. Existing
source-format and variable-font option behavior is retained. The options component
was extracted so the dialog stays below 400 lines. Labels and warning text are
available in English and zh-CN; developer comments and documentation are English.

Verification:

- Full `fc-api` suite: **232 passed, 2 existing ignored**. New coverage checks
  both masters, both containers, all four option combinations, original hints,
  source equality, unchanged generation/undo history and destination protection.
- Full frontend suite: **1,126 passed in 138 files**. TypeScript, ESLint, workspace
  all-target Clippy with `-D warnings`, formatting and whitespace checks pass.
- Isolated browser exports exercise English PFB with both options, and Chinese
  PFA with overlap removal only. Both screenshots were visually checked; the
  clearing warning is translated. The source hint/outline API state remains
  unchanged, with generation zero.
- CLI `build --autohint --remove-overlap` produces the same glyph program as
  the browser. FontTools confirms one merged contour, width 333 and the expected
  `(50, 130)` vertical stem in both hinted outputs, and no stems in the unhinted
  PFA. FreeType renders the three outputs at six sizes (**18 native renders**).

This closes these Type 1 export options, not the remaining advanced Type 1
mechanisms or the full P5 milestone.
