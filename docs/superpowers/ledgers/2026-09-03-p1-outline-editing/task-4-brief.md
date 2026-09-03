### Task 4: `fw-formats` — UFO 3 + designspace import/export

**Files:** `crates/fw-formats/src/ufo/*`, tests `crates/fw-formats/tests/{ufo_read.rs, ufo_write.rs, designspace.rs}`, fixture generator `crates/fw-fixtures/src/ufo.rs` (writes `fixtures/ufo/Minimal.ufo` deterministically; committed output).

**Interfaces:**
```rust
pub fn read_ufo(path: &Path) -> Result<Font, Error>;                        // one master; layers → master layer + background/custom layers; groups/kerning → KerningGroups + Master.kerning_h; features.fea → Features (single block, source_of_truth Fea); lib → Extra.lib; fontinfo → FontInfo/MasterMetrics/PsPrivate
pub fn read_designspace(path: &Path) -> Result<Font, Error>;                // axes, sources → masters (default source first), instances, per-source kerning; brace layers from "layer" sources
pub fn write_ufo(font: &Font, master: &MasterId, dir: &Path, opts: &UfoWriteOptions { incremental: bool }) -> Result<(), Error>;   // never deletes the target dir (write to temp dir + rename per file)
pub fn write_designspace(font: &Font, dir: &Path) -> Result<PathBuf, Error>;   // <family>.designspace + one UFO per master
```
Round-trip policy: UFO ⇄ model lossless for outlines, components (with UFO identifiers preserved in `Extra.lib` under `dev.fontworkshop.ufo.identifiers`), anchors, guides, kerning/groups, features text, fontinfo (108 fields — map explicitly), lib. Things UFO cannot express (bitmaps, TT instructions, raw tables) are stored under `lib["dev.fontworkshop.extra"]` base64 with a version key.
- [ ] **Step 1: Tests**: `Minimal.ufo` → Font → write → norad re-read equals the original norad `Font` (compare glyphs, kerning, groups, fontinfo, lib); a designspace with 2 masters yields 2 masters + axes + 1 instance; a Font from DejaVu (fw-sfnt) written as UFO and re-read is glyph-identical (points/components/anchors) and preserves `raw_tables` via lib; write is incremental (a second write with one glyph changed touches only that .glif — check mtimes).
- [ ] **Step 2: Implement**; register in `fw-doc::Session::open_path` (extension `.ufo` dir / `.designspace`) and in export.
- [ ] **Step 3:** commit `feat(fw-formats): UFO 3 and designspace import/export`.

---

