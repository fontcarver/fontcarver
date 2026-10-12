# Bitmap native saving and vertical proofing

Task: p5f-w3-bitmap-fixes, authorized by parent under current user request.

## Scope and ownership

Own app/src/views/bitmap/BitmapSheet.tsx, BitmapSheet.test.tsx, BitmapWaterfall.tsx, BitmapWaterfall.test.tsx and new local layout helper/tests if needed; narrow en.json bitmap.* keys; docs/user bitmap chapter; docs/p5f-w3-bitmap-fixes-acceptance.md. Do not edit shared saveArtifact.ts without parent coordination. Backend bitmap preview/types changes require explicit file coordination and Cargo lock; request app/e2e checks from /root/w3_e2e.

Parent additionally approved narrowly scoped preview metric resolution in
`crates/fc-api/src/handlers/bitmap.rs` and its regression in
`crates/fc-api/tests/bitmap_parity.rs`; no other backend files are owned.

## Interfaces and acceptance

Audit found two ordinary gaps: BitmapSheet only uses Blob download anchor, unlike proof saveArtifact's native Tauri dialog+file write; route PNG Save/Download via established native/browser saving while retaining preview, cancellation and error handling. BitmapWaterfall currently only horizontal while row111 promises vertical proofing; add meaningful vertical writing option with correct glyph placement/advances/origins using available model vertical metrics, or coordinate necessary narrowly scoped backend support (no fake centered placement marketed as correct vertical metrics). Verify regression tests and real kernel browser behavior through E2E owner. Add translations only in English and send key list to parent for separate Sol6.1. Do not weaken audit requirement merely by renaming it.

## Protocol

Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/evidence first. Append gap analysis before implementation. Shared checkout: no commits/push/reset/clean or others' edits, no nested agents. Parent owns integration/status, coordinates builds. Scratch only .scratch/p5f-w3-bitmap-fixes, absolute task TMPDIR, artifact checker at start/batches/end, Cargo via flock .scratch/p5f/cargo.lock with shared target/default profiles. No native/global desktop input automation. Preserve exact evidence in own report, stop ownprocesses and delete owned scratch after acceptance. User defers publication and has no signingcredentials; no publishing or inventednative evidence. Report under300words.

## Gap analysis

- Row 71: PNG preparation/import and embedded layout already work, but the
  prepared image is saved through an HTML download link alone. Route the save
  button through the established `saveArtifact` native-dialog/browser path;
  retain the prepared preview on cancellation or failure and permit retry.
- Rows 101/111: every strike already renders horizontally, including named
  unencoded glyphs. The bitmap preview has no vertical option. Its glyph data
  already contains `vertical_advance` and `vertical_origin`, but optional
  metrics currently pass through without document inheritance. Add an explicit
  vertical view and a geometry helper that places stored pixels relative to
  their real origins, advances top to bottom, and includes negative bearings
  and zero advances in its bounds. Coordinate backend fallback resolution with
  the parent; retain and document the existing no-OpenType-shaping boundary.
- Native/global desktop input remains prohibited: prove the native save path
  through mocked Tauri dialog/write calls and delegate real kernel browser PNG
  roundtrip and vertical-pixel checks to the E2E owner. Actual native dialog
  execution is not claimed.
