# Crate feasibility spike — Tauri 2 font editor (Rust kernel + TS UI)

Spike dir: `/tmp/claude-1000/-home-chen-githubprojects-pixelfonts-fontworkshop/f0187d23-6841-4655-bad5-89d6bc891ebe/scratchpad/crate-spike`
Toolchain: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, `cargo 1.94.0`, Linux x86_64, 2026-09-03.
All crate sources read from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

Everything below marked **VERIFIED** was compiled and/or executed in this spike, or copied verbatim from
crate source. A handful of items are marked **(unverified)**.

---

## 0. Executive summary

| Question | Answer |
|---|---|
| Do all named crates co-resolve in one workspace? | **Yes.** 558 transitive deps, cold `cargo check --all-targets` = **35.5 s**, exit 0, zero warnings. |
| Can a third-party crate implement `fontir::source::Source` and run the fontc pipeline as a library? | **Yes — proven end-to-end.** `examples/custom_source.rs` builds a static TTF with GSUB+GPOS from a purely in-memory editor model, zero filesystem I/O. |
| Is layout-table round trip (read-fonts → write-fonts → binary) lossless? | **Semantically yes, byte-wise no.** GDEF was byte-identical; GSUB/GPOS re-dumped *smaller* (write-fonts dedups + repacks) and shaping output was bit-identical. |
| Curve-preserving boolean ops in pure Rust? | **Yes — `flo_curves` 0.8.1.** Union of two circles: 9 cubic segments, 0.006 % area error, no degenerate segments. `path_remove_interior_points` is the "Remove Overlap" primitive. |
| WOFF2 encoder? | Yes, but **no pure-Rust encoder exists**. `woofwoof` 1.0.2 (Google woff2 C++ + Rust brotli) and `woff` 0.6.3 both vendor C/C++ and need a `cc`/C++ toolchain. Decoders are pure Rust (`wuff`, `woff2-patched`). |
| skia-safe pure-cargo? | **No.** It downloads a 4.1 MB prebuilt tarball from GitHub releases at build time (244 MB target dir), or falls back to a full Skia source build (depot_tools + python + gn + ninja + clang). Do not depend on it. |
| Biggest gaps | Type1/CFF *writing*, BDF/PCF/SFD, `.glyphs` writer, MATH/JSTF/VDMX/LTSH/PCLT/EBSC parsing, EBDT/CBDT/sbix **writing**, TT instruction assembler/disassembler, PS autohint, TT autoinstruct. |

---

## 1. The workspace that resolves and checks

### 1.1 `Cargo.toml` (verbatim, this file `cargo check`s clean)

```toml
[package]
name = "crate-spike"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"

[dependencies]
# --- fontations (googlefonts) ---
read-fonts   = { version = "0.43", features = ["experimental_traverse", "ift", "serde"] }
write-fonts  = { version = "0.52", features = ["read", "serde", "dot2"] }
skrifa       = { version = "0.46", features = ["traversal"] }
font-types   = { version = "0.12", features = ["serde"] }

# --- fontc pipeline (googlefonts) ---
fea-rs        = "1.0"
fontc         = "1.0"
fontir        = "1.0"
fontdrasil    = "1.0"
fontbe        = "1.0"
glyphs-reader = "1.0"
ufo2fontir    = "1.0"
glyphs2fontir = "1.0"

# --- shaping ---
harfrust = "0.13"

# --- sources / IO ---
norad     = { version = "0.18", features = ["kurbo", "rayon"] }
plist     = "1.10"
quick-xml = { version = "0.42", features = ["serialize"] }
serde     = { version = "1", features = ["derive"] }

# --- geometry / raster ---
kurbo      = { version = "0.13", features = ["serde"] }
tiny-skia  = "0.12"
zeno       = "0.3"

# --- boolean path ops candidates ---
flo_curves        = "0.8"
i_overlay         = "8.1"
cavalier_contours = "0.9"
lyon_path         = "1.0"
lyon_algorithms   = "1.0"

# --- webfont compression ---
woff2-patched = "0.4"   # WOFF2 decode (fork of woff2-rs)
woofwoof      = "1.0"   # WOFF2 encode + decode, Rust brotli + vendored C++ woff2
wuff          = "0.2"   # WOFF1 + WOFF2 decode, pure rust
woff          = "0.6"   # bodoni: WOFF1 (sfnt2woff-zopfli C) + WOFF2 (Google C++)
brotli        = "9"
flate2        = "1"
zopfli        = "0.8"

# --- app shell (resolution check only) ---
tauri = "2"
tracing-subscriber = "0.3"
```

### 1.2 Build facts (**VERIFIED**)

| Metric | Value |
|---|---|
| `cargo generate-lockfile` | 558 dependency packages (559 lock entries incl. the spike crate) |
| Cold `cargo check --all-targets` (empty `target/`) | **35.46 s** wall (866 % CPU, peak RSS 1.1 GB), exit 0 |
| Cold `cargo build --release` (examples) | 1 m 58 s |
| Warnings / errors | **zero** |
| Crates present at >1 version | 43 (all benign co-existence: `bitflags` 1/2, `syn` 1/2/3, `thiserror` 1/2, `quick-xml` 0.38/0.41/0.42, `brotli` 7/8/9, `windows-*`, `toml`, `schemars`, `winnow`, `indexmap`…). No unsatisfiable constraint anywhere. |

### 1.3 Version-conflict notes

* **Two pitfalls found and fixed while resolving:**
  1. `norad` has **no `serde` feature**; its features are exactly `{default = ["object-libs"], kurbo, object-libs, rayon}`. Asking for `features = ["serde"]` fails resolution.
  2. Everything else resolved on the first try at the latest versions.
* **`fontbe` 1.0.0 pins `icu_properties = "=2.1"` (exact `=`).** This is the only exact pin anywhere in the fontc stack and is the single most likely future conflict if the app also uses ICU. Resolved here to `2.1.2`.
* **`fontc` 1.0.0 depends on `fontra2fontir 0.5.0`** — the only pre-1.0 crate in the pipeline. Not on the `Source`/`generate_font` path, so it is only a semver-churn nuisance.
* `fontc` default features are `["cli", "rayon"]`; `cli` pulls `clap`. For an embedded kernel use
  `fontc = { version = "1.0", default-features = false, features = ["rayon"] }` — `generate_font` is **not** behind `cli`, only `run` is.
* Tauri 2.11.5 resolves and checks alongside everything (it pulled GTK3/WebKit2GTK sys crates; the system dev packages were present).

### 1.4 Resolved versions

```
read-fonts 0.43.3   write-fonts 0.52.0  skrifa 0.46.2      font-types 0.12.4
fea-rs 1.0.0        fontc 1.0.0         fontir 1.0.0       fontdrasil 1.0.0
fontbe 1.0.0        glyphs-reader 1.0.0 ufo2fontir 1.0.0   glyphs2fontir 1.0.0
fontra2fontir 0.5.0 harfrust 0.13.3     norad 0.18.4       plist 1.10.0
quick-xml 0.42.0    serde 1.0.229       kurbo 0.13.1       tiny-skia 0.12.0
zeno 0.3.3          flo_curves 0.8.1    i_overlay 8.1.0    cavalier_contours 0.9.0
lyon_path 1.0.19    lyon_algorithms 1.0.21                 woff2-patched 0.4.0
woofwoof 1.0.2      wuff 0.2.9          woff 0.6.3         brotli 9.0.0
flate2 1.1.10       zopfli 0.8.3        tauri 2.11.5       icu_properties 2.1.2
polycool 0.4.0 (new kurbo 0.13 dep)
```

### 1.5 Working examples in the spike

| File | What it proves |
|---|---|
| `examples/boolops.rs` | flo_curves vs i_overlay union/intersect/difference/remove-overlap, area accuracy, PNG render |
| `examples/webfont.rs` | WOFF1/WOFF2 encode+decode round trips against a real TTF |
| `examples/fontc_pipeline.rs` | in-memory `.glyphs` → `fontc::generate_font` → read-fonts + skrifa + harfrust |
| `examples/custom_source.rs` | **third-party `fontir::source::Source` impl** driving the whole fontc pipeline |
| `examples/roundtrip.rs` | GSUB/GPOS/GDEF read→write round trip, `FontBuilder` mixing, `fea-rs` from a string |
| `examples/raster.rs` | hinted glyph @16 ppem → 8-bit alpha mask via tiny-skia `Mask` and zeno |
| `skia-probe/` | separate crate proving skia-safe's build path |

---

## 2. API notes

### 2.a `read-fonts` 0.43.3

#### Opening a font
```rust
pub struct FontRef<'a> {
    data: FontData<'a>,
    pub table_directory: TableDirectory<'a>,   // public field
    ttc_index: u32, in_ttc: bool, table_directory_sorted: bool,
}
impl<'a> FontRef<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, ReadError>;
    pub fn from_index(data: &'a [u8], index: u32) -> Result<Self, ReadError>;
    pub fn data(&self) -> FontData<'a>;
    pub fn ttc_index(&self) -> Option<u32>;
    pub fn table_directory(&self) -> &TableDirectory<'a>;
    pub fn table_data(&self, tag: Tag) -> Option<FontData<'a>>;
    pub fn fonts(data: &'a [u8]) -> impl Iterator<Item = Result<FontRef<'a>, ReadError>> + 'a + Clone;
}

pub enum FileRef<'a> { Font(FontRef<'a>), Collection(CollectionRef<'a>) }
impl<'a> FileRef<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, ReadError>;
    pub fn fonts(&self) -> impl Iterator<Item = Result<FontRef<'a>, ReadError>> + 'a + Clone;
}
pub struct CollectionRef<'a> { .. }
impl<'a> CollectionRef<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, ReadError>;
    pub fn len(&self) -> u32;
    pub fn get(&self, index: u32) -> Result<FontRef<'a>, ReadError>;
    pub fn iter(&self) -> impl Iterator<Item = Result<FontRef<'a>, ReadError>> + 'a + Clone;
}

pub enum ReadError {
    OutOfBounds, InvalidFormat(i64), InvalidSfnt(u32), InvalidTtc(Tag),
    InvalidCollectionIndex(u32), InvalidArrayLen, ValidationError, NullOffset,
    TableIsMissing(Tag), MetricIsMissing(Tag), MalformedData(&'static str),
}
```

#### Listing table tags
```rust
for rec in font.table_directory().table_records() {   // -> &'a [TableRecord]
    let tag: Tag = rec.tag();
    let (off, len) = (rec.offset(), rec.length());
}
```
```rust
#[repr(C, packed)]
pub struct TableRecord {
    pub tag: BigEndian<Tag>, pub checksum: BigEndian<u32>,
    pub offset: BigEndian<u32>, pub length: BigEndian<u32>,
}
```

#### Raw bytes for an arbitrary / unknown table
```rust
pub fn table_data(&self, tag: Tag) -> Option<FontData<'a>>;
// FontData::as_bytes(&self) -> &'a [u8]
let raw: &[u8] = font.table_data(Tag::new(b"fpgm")).map(|d| d.as_bytes()).unwrap_or(&[]);
```
`FontData` also offers `slice`, `split_off`, `read_at::<T: Scalar>`, `read_be_at`, `read_ref_at`,
`read_array`, `read_with_args`. This is the escape hatch for every table read-fonts does not model.

#### `TableProvider` — the complete accessor list (`src/table_provider.rs`)
```rust
pub trait TopLevelTable { const TAG: Tag; }

pub trait TableProvider<'a> {
    fn data_for_tag(&self, tag: Tag) -> Option<FontData<'a>>;
    fn expect_data_for_tag(&self, tag: Tag) -> Result<FontData<'a>, ReadError>;
    fn expect_table<T: TopLevelTable + FontRead<'a, Args = ()>>(&self) -> Result<T, ReadError>;

    fn head(&self)  -> Result<tables::head::Head<'a>, ReadError>;
    fn name(&self)  -> Result<tables::name::Name<'a>, ReadError>;
    fn hhea(&self)  -> Result<tables::hhea::Hhea<'a>, ReadError>;
    fn vhea(&self)  -> Result<tables::vhea::Vhea<'a>, ReadError>;
    fn hmtx(&self)  -> Result<tables::hmtx::Hmtx<'a>, ReadError>;   // derives numberOfHMetrics from hhea
    fn hdmx(&self)  -> Result<tables::hdmx::Hdmx<'a>, ReadError>;
    fn vmtx(&self)  -> Result<tables::vmtx::Vmtx<'a>, ReadError>;
    fn vorg(&self)  -> Result<tables::vorg::Vorg<'a>, ReadError>;
    fn fvar(&self)  -> Result<tables::fvar::Fvar<'a>, ReadError>;
    fn avar(&self)  -> Result<tables::avar::Avar<'a>, ReadError>;
    fn hvar(&self)  -> Result<tables::hvar::Hvar<'a>, ReadError>;
    fn vvar(&self)  -> Result<tables::vvar::Vvar<'a>, ReadError>;
    fn mvar(&self)  -> Result<tables::mvar::Mvar<'a>, ReadError>;
    fn maxp(&self)  -> Result<tables::maxp::Maxp<'a>, ReadError>;
    fn os2(&self)   -> Result<tables::os2::Os2<'a>, ReadError>;
    fn post(&self)  -> Result<tables::post::Post<'a>, ReadError>;
    fn gasp(&self)  -> Result<tables::gasp::Gasp<'a>, ReadError>;
    fn loca(&self, is_long: impl Into<Option<bool>>) -> Result<tables::loca::Loca<'a>, ReadError>;
    fn glyf(&self)  -> Result<tables::glyf::Glyf<'a>, ReadError>;
    fn gvar(&self)  -> Result<tables::gvar::Gvar<'a>, ReadError>;
    fn cvt(&self)   -> Result<&'a [BigEndian<i16>], ReadError>;      // raw i16 array, no struct
    fn cvar(&self)  -> Result<tables::cvar::Cvar<'a>, ReadError>;
    fn cff(&self)   -> Result<tables::cff::Cff<'a>, ReadError>;
    fn cff2(&self)  -> Result<tables::cff2::Cff2<'a>, ReadError>;
    fn cmap(&self)  -> Result<tables::cmap::Cmap<'a>, ReadError>;
    fn gdef(&self)  -> Result<tables::gdef::Gdef<'a>, ReadError>;
    fn gpos(&self)  -> Result<tables::gpos::Gpos<'a>, ReadError>;
    fn gsub(&self)  -> Result<tables::gsub::Gsub<'a>, ReadError>;
    fn feat(&self)  -> Result<tables::feat::Feat<'a>, ReadError>;    // AAT
    fn ltag(&self)  -> Result<tables::ltag::Ltag<'a>, ReadError>;    // AAT
    fn ankr(&self)  -> Result<tables::ankr::Ankr<'a>, ReadError>;    // AAT
    fn trak(&self)  -> Result<tables::trak::Trak<'a>, ReadError>;    // AAT
    fn morx(&self)  -> Result<tables::morx::Morx<'a>, ReadError>;    // AAT
    fn mort(&self)  -> Result<tables::mort::Mort<'a>, ReadError>;    // AAT
    fn kerx(&self)  -> Result<tables::kerx::Kerx<'a>, ReadError>;    // AAT
    fn kern(&self)  -> Result<tables::kern::Kern<'a>, ReadError>;
    fn colr(&self)  -> Result<tables::colr::Colr<'a>, ReadError>;
    fn cpal(&self)  -> Result<tables::cpal::Cpal<'a>, ReadError>;
    fn cblc(&self)  -> Result<tables::cblc::Cblc<'a>, ReadError>;
    fn cbdt(&self)  -> Result<tables::cbdt::Cbdt<'a>, ReadError>;
    fn eblc(&self)  -> Result<tables::eblc::Eblc<'a>, ReadError>;
    fn ebdt(&self)  -> Result<tables::ebdt::Ebdt<'a>, ReadError>;
    fn sbix(&self)  -> Result<tables::sbix::Sbix<'a>, ReadError>;
    fn stat(&self)  -> Result<tables::stat::Stat<'a>, ReadError>;
    fn svg(&self)   -> Result<tables::svg::Svg<'a>, ReadError>;
    fn varc(&self)  -> Result<tables::varc::Varc<'a>, ReadError>;    // variable composites
    #[cfg(feature = "ift")] fn ift(&self)  -> Result<tables::ift::IftPatchMap<'a>, ReadError>;
    #[cfg(feature = "ift")] fn iftx(&self) -> Result<tables::ift::IftPatchMap<'a>, ReadError>;
    fn meta(&self)  -> Result<tables::meta::Meta<'a>, ReadError>;
    fn base(&self)  -> Result<tables::base::Base<'a>, ReadError>;
    fn dsig(&self)  -> Result<tables::dsig::Dsig<'a>, ReadError>;
}
```

Everything the brief asked about is present **except** the list below.

#### Tables read-fonts 0.43.3 does **NOT** parse

Confirmed absent (no module, no `TableProvider` method, no `TopLevelTable` impl):

* **`MATH`** — Math typesetting. (`data/generated/generated_tables.rs` reserves a *raw-bytes* slot for it, but there is no parser.)
* **`JSTF`** — Justification.
* **`VDMX`**, **`LTSH`** — Windows device metrics / linear threshold.
* **`PCLT`** — PCL 5 data.
* **`EBSC`** — Embedded bitmap scaling (explicitly commented out in `scripts/gen_tables.py`).
* **`bdat` / `bloc`** — Apple's pre-OT bitmap tables (only OT `EBLC`/`EBDT` and `CBLC`/`CBDT` are modeled).
* **`fpgm`**, **`prep`** — no typed accessor at all (only `table_data(Tag::new(b"fpgm"))`). `cvt ` is available but only as `&[BigEndian<i16>]`.
* AAT extras not modeled: `bsln`, `just`, `opbd`, `lcar`, `prop`, `acnt`, `Zapf`.

For a FontForge-class editor this means **MATH, JSTF, VDMX, LTSH, PCLT and bdat/bloc parsers are ours to write.**
`sbix`/`EBDT`/`CBDT`/`EBLC`/`CBLC` are *readable* but there is no writer anywhere in the stack (§3).

#### Variations helpers
`read_fonts::tables::variations` gives the shared primitives:
```rust
pub enum DeltaSetIndexMap<'a> { Format0(..), Format1(..) }
impl DeltaSetIndexMap<'_> { pub fn get(&self, index: u32) -> Result<DeltaSetIndex, ReadError>; }
pub struct ItemVariationStore<'a> { .. }
impl ItemVariationStore<'_> {
    pub fn compute_delta(&self, index: DeltaSetIndex, coords: &[F2Dot14]) -> Result<i32, ReadError>;
    pub fn compute_float_delta(&self, index: DeltaSetIndex, coords: &[F2Dot14]) -> Result<FloatItemDelta, ReadError>;
}
pub struct VariationRegion<'a> { .. }
impl VariationRegion<'_> {
    pub fn compute_scalar(&self, coords: &[F2Dot14]) -> Fixed;
    pub fn compute_scalar_f32(&self, coords: &[F2Dot14]) -> f32;
}
// tuple-variation store: TupleVariationData, TupleVariation, PackedPointNumbers, PackedDeltas, Tuple, TupleIndex
```

#### `experimental_traverse`
Enables `pub mod traversal` — generic per-field introspection of every codegen'd table plus a `Debug`
impl that delegates to it. Without the feature most tables have **no `Debug` impl at all**. This is what
backs `otexplorer`, and it is exactly what you want behind a "table inspector" panel in the UI.
```rust
pub trait SomeTable<'a> { fn type_name(&self) -> &str; fn get_field(&self, idx: usize) -> Option<Field<'a>>; }
pub enum FieldType<'a> { I8(..), U16(..), Tag(Tag), Fixed(Fixed), GlyphId16(..), NameId(..),
    ResolvedOffset(ResolvedOffset<'a>), Record(RecordResolver<'a>), Array(Box<dyn SomeArray<'a> + 'a>), Unknown, .. }
```

#### New in 0.43: `read_fonts::model` (feature `experimental_font_api`) — **highly relevant to Tauri**
```rust
pub use font::{Font, FontBlob, FontFeatureVariations, FontFormat, FontInstance,
               FontInstanceBuilder, FontKind, FontSource, FontTableFunction, FontTables,
               NormalizedCoord};

pub struct Font(Arc<FontRepr>);            // OWNED, refcounted, no lifetime parameter
impl Font {
    pub fn new(source: impl Into<FontSource>, index: u32) -> Result<Self, ReadError>;
    pub fn kind(&self) -> FontKind<'_>;
    pub fn tables(&self) -> &FontTables;
}
impl<'a> TableProvider<'a> for &'a FontTables { .. }   // full typed access
// FontBlob: From<Vec<u8>>, From<&'static [u8]>, From<Arc<dyn AsRef<[u8]> + Send + Sync>>
// FontTableFunction::new(Arc<dyn Fn(Tag) -> Option<FontBlob> + Send + Sync>)  <- feed tables lazily
```
`FontTables` also exposes **55 raw `<tag>_data(&self) -> Option<&[u8]>` accessors** covering every
known tag *including* `math_data()`, `jstf_data()`, `fpgm_data()`, `prep_data()`.
An `Arc`-based owned font with no lifetime and a pluggable table-provider callback is exactly the shape
you want behind a Tauri command boundary; the `FontTableFunction` variant lets the editor serve its own
live, unsaved tables to the same reading/shaping code. Flagged **experimental** by upstream, so pin it.

---

### 2.b `write-fonts` 0.52.0

#### read → write conversion (`src/from_obj.rs`)
```rust
pub trait FromObjRef<T: ?Sized>: Sized {
    fn from_obj_ref(from: &T, data: FontData) -> Self;
}
pub trait FromTableRef<T>: FromObjRef<T> {
    fn from_table_ref(from: &T) -> Self { Self::from_obj_ref(from, FontData::new(&[])) }
}
pub trait ToOwnedObj<T>   { fn to_owned_obj(&self, data: FontData) -> T; }
pub trait ToOwnedTable<T>: ToOwnedObj<T> { fn to_owned_table(&self) -> T; }

impl<U, T> ToOwnedObj<U>   for T where U: FromObjRef<T>   { .. }   // blanket
impl<U, T> ToOwnedTable<U> for T where U: FromTableRef<T> { .. }   // blanket
```
Blanket impls also cover `BigEndian<T>`, `Vec<T>`/`BTreeSet<T>` from slices, `Option<T>`,
`OffsetMarker`/`NullableOffsetMarker` from resolved offsets, and `ArrayOfOffsets` → `Vec<OffsetMarker<..>>`.

Usage:
```rust
use write_fonts::from_obj::ToOwnedTable;
let read_gsub  = font.gsub()?;                                    // read_fonts::tables::gsub::Gsub<'_>
let mut owned: write_fonts::tables::gsub::Gsub = read_gsub.to_owned_table();
owned.lookup_list.lookups.push(/* … */);
let bytes = write_fonts::dump_table(&owned)?;
```

#### Which tables support it
42 generated modules implement `FromObjRef`/`FromTableRef`. For layout, **all of it**:
* GSUB: `Gsub`, `SubstitutionLookup`, `SingleSubst`(F1/F2), `MultipleSubstFormat1`, `Sequence`,
  `AlternateSubstFormat1`, `AlternateSet`, `LigatureSubstFormat1`, `LigatureSet`, `Ligature`, `ExtensionSubtable`.
* GPOS: `Gpos`, `PositionLookup`, `AnchorTable`(F1/F2/F3), `MarkArray`, `MarkRecord`, `SinglePos`(F1/F2),
  `PairPos`(F1/F2), `PairSet`, `PairValueRecord`, `Class1Record`/`Class2Record`, `CursivePosFormat1`,
  `EntryExitRecord`, `MarkBasePosFormat1`, `BaseArray`/`BaseRecord`, `MarkLigPosFormat1`,
  `LigatureArray`/`LigatureAttach`/`ComponentRecord`, `MarkMarkPosFormat1`, `Mark2Array`/`Mark2Record`, `ExtensionSubtable`.
* GDEF: `Gdef`, `AttachList`/`AttachPoint`, `LigCaretList`/`LigGlyph`, `CaretValue`(F1/F2/F3), `MarkGlyphSets`.
* Shared layout: `ScriptList`/`Script`/`LangSys`, `FeatureList`/`Feature`, `LookupList<T>`/`Lookup<T>`,
  `CoverageTable`(F1/F2), `ClassDef`(F1/F2), `SequenceContext`, `ChainedSequenceContext` + all rule sets,
  `Device`/`VariationIndex`/`DeviceOrVariationIndex`, `FeatureVariations`/`FeatureVariationRecord`,
  `ConditionSet`/`Condition`(F1–F5), `SizeParams`, `StylisticSetParams`.
* Variations: `ItemVariationStore`, `ItemVariationData`, `VariationRegionList`, `VariationRegion`,
  `RegionAxisCoordinates`, `DeltaSetIndexMap`(F0/F1), `TupleVariationHeader`, `Tuple`.

**Round-trip fidelity measured on DejaVuSans.ttf (VERIFIED, `examples/roundtrip.rs`):**
```
GSUB: orig 5598 B -> re-dumped 4960 B, byte-identical: false ; re-parses: true
GPOS: orig 40586 B -> re-dumped 26348 B, byte-identical: false ; re-parses: true
GDEF: orig  658 B -> re-dumped  658 B, byte-identical: TRUE  ; re-parses: true
shape[original]   = 36:1270 57:1270 36:1242 55:1092 36:1401 53:1423 3:651 58:1894 68:1255 3:651 5042:1290
shape[rebuilt]    = (identical)
shape[regen-GSUB] = (identical)
```
So: **not byte-preserving, but semantically lossless and 35 % smaller** — `TableWriter` (`src/write.rs`)
dedups identical subtables and re-packs the offset graph with a HarfBuzz-`hb-repacker`-style algorithm
(`src/graph.rs`). Shaping results (incl. the `fi` ligature, gid 5042) were bit-identical before and after.
For an editor this is the right trade; if you need byte-for-byte preservation of a table you did not
touch, use `FontBuilder::add_raw` with the original bytes instead.

#### `FontBuilder` (`src/font_builder.rs`)
```rust
pub struct FontBuilder<'a> { tables: BTreeMap<Tag, Cow<'a, [u8]>> }
impl<'a> FontBuilder<'a> {
    pub fn new() -> Self;
    #[cfg(feature = "tables")]
    pub fn add_table<T>(&mut self, table: &T) -> Result<&mut Self, BuilderError>
        where T: FontWrite + Validate + TopLevelTable;
    pub fn add_raw(&mut self, tag: Tag, data: impl Into<Cow<'a, [u8]>>) -> &mut Self;
    pub fn copy_missing_tables(&mut self, font: FontRef<'a>) -> &mut Self;
    pub fn contains(&self, tag: Tag) -> bool;
    pub fn ordered_tags(&self) -> Vec<Tag>;
    pub fn build(&mut self) -> Vec<u8>;
}
pub struct BuilderError { pub tag: Tag, pub inner: Error }   // #[non_exhaustive]
```
`build()` computes checksums, `head.checksumAdjustment`, the sfnt version (`OTTO` vs `0x00010000`) and
the OpenType-recommended table order. There is **no `build_from`**. `TTCHeader` writing is
`panic!("TTCHeader writing not supported (yet)")` — **we must write our own .ttc assembler.**

Mixing regenerated + verbatim (**VERIFIED**, produced a 759 720 B font identical in size to the input,
20 tables preserved, `font_revision` changed to 2.5, shaping unchanged):
```rust
let mut head: write_fonts::tables::head::Head = font.head()?.to_owned_table();
head.font_revision = Fixed::from_f64(2.5);
let bytes = FontBuilder::new()
    .add_table(&head)?          // recompiled
    .copy_missing_tables(font)  // everything else copied verbatim
    .build();
```

#### CFF / CFF2 — **write-fonts cannot author them**
`src/tables/` has **no** `cff.rs`/`cff2.rs`. CFF lives under `src/ps/cff.rs` + `src/ps/cff/v2.rs` and
`generated/generated_cff{,2}.rs`, and contains only *framing* types:
* `CffHeader`/`Cff2Header` — `{ hdr_size, off_size|top_dict_length, _padding: Vec<u8>, trailing_data|top_dict_data: Vec<u8> }`
* `Index { count, off_size, offsets: Vec<u8>, data: Vec<u8> }`, `Index::from_items(Vec<Vec<u8>>)` — packs *already-encoded* blobs.
* `FdSelect` (F0/F3/F4), `CustomCharset`/`CharsetFormat0/1/2` — real structured writers, but these are
  simple index maps, not the DICT/charstring bytecode.

There is **no DICT operand encoder and no charstring operator encoder** anywhere in write-fonts, and no
top-level `Cff`/`Cff2` type implementing `TopLevelTable + FontWrite`. read-fonts *does* have the full
reader side (`src/ps/{dict,cs,charset,encoding,fd_select,index,stack,blend,hinting,type1}.rs`).

**`fontbe` 1.0.0 does not emit CFF either** — `grep -ri cff fontbe/src` matches only a hex Unicode range
(`0x0CFF`) and a commit-hash substring in a URL comment. The `WorkId` enum has no CFF variant.

⇒ **A CFF/CFF2 writer (Type2 charstring encoder, DICTs, subroutinizer, FDSelect/FDArray, hint
generation) is entirely ours to write, or we ship TrueType-only.**

#### Other pieces
```rust
pub trait Validate  { fn validate(&self) -> Result<(), ValidationReport> { .. }
                      fn validate_impl(&self, ctx: &mut ValidationCtx); }
pub trait FontWrite { fn write_into(&self, writer: &mut TableWriter);
                      fn table_type(&self) -> TableType { TableType::Unknown } }
#[cfg(feature = "tables")]
pub fn dump_table<T: FontWrite + Validate>(table: &T) -> Result<Vec<u8>, Error>;
#[cfg(feature = "read")] pub extern crate read_fonts as read;   // `write_fonts::read::FontRef`
pub extern crate font_types as types;                            // `write_fonts::types::*`
```
`ItemVariationStore` **building** is supported (`src/tables/variations/ivs_builder.rs`):
```rust
pub struct VariationStoreBuilder;
impl VariationStoreBuilder {
    pub fn new(axis_count: u16) -> Self;
    pub fn new_with_implicit_indices(axis_count: u16) -> Self;    // HVAR
    pub fn add_deltas<T: Into<i32>>(&mut self, deltas: Vec<(VariationRegion, T)>) -> TemporaryDeltaSetId;
    pub fn build(self) -> (ItemVariationStore, VariationIndexRemapping);
}
```

---

### 2.c `skrifa` 0.46.2

Features: `default = ["std", "autohint_shaping"]`, plus `libm`, `spec_next`, `traversal`.

#### Outlines
```rust
pub enum OutlineGlyphFormat { Glyf, Cff, Cff2, Varc }

pub struct OutlineGlyphCollection<'a>;
impl<'a> OutlineGlyphCollection<'a> {
    pub fn new(font: &FontRef<'a>) -> Self;                              // prefers glyf > CFF2 > CFF
    pub fn with_format(font: &FontRef<'a>, format: OutlineGlyphFormat) -> Option<Self>;
    pub fn format(&self) -> Option<OutlineGlyphFormat>;
    pub fn get(&self, glyph_id: GlyphId) -> Option<OutlineGlyph<'a>>;
    pub fn iter(&self) -> impl Iterator<Item = (GlyphId, OutlineGlyph<'a>)> + 'a + Clone;
    pub fn prefer_interpreter(&self) -> bool;
    pub fn require_interpreter(&self) -> bool;      // FreeType "tricky font" equivalent
    pub fn fractional_size_hinting(&self) -> bool;
}

pub struct OutlineGlyph<'a>;
impl<'a> OutlineGlyph<'a> {
    pub fn format(&self) -> OutlineGlyphFormat;
    pub fn glyph_id(&self) -> GlyphId;
    pub fn has_overlaps(&self) -> Option<bool>;     // None for CFF
    pub fn has_hinting(&self) -> Option<bool>;
    pub fn draw_memory_size(&self, hinting: Hinting) -> usize;
    pub fn draw<'s>(&self, settings: impl Into<DrawSettings<'a>>, pen: &mut impl OutlinePen)
        -> Result<AdjustedMetrics, DrawError>;
}
pub struct AdjustedMetrics { pub has_overlaps: bool, pub lsb: Option<f32>, pub advance_width: Option<f32> }

pub struct DrawSettings<'a>;
impl<'a> DrawSettings<'a> {
    pub fn unhinted(size: Size, location: impl Into<LocationRef<'a>>) -> Self;
    pub fn hinted(instance: &'a HintingInstance, is_pedantic: bool) -> Self;
    pub fn with_memory(self, memory: Option<&'a mut [u8]>) -> Self;      // zero-alloc rendering
    pub fn with_path_style(self, path_style: PathStyle) -> Self;
}

pub trait OutlinePen {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32);
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32);
    fn close(&mut self);
}
```
Variations are supplied through the location:
```rust
let loc = font.axes().location(&[("wght", 650.0), ("wdth", 100.0)]);
glyph.draw(DrawSettings::unhinted(Size::new(16.0), &loc), &mut pen)?;
```
`Size::unscaled()` yields font units; `Size::new(ppem)` scales. `NormalizedCoord = F2Dot14`.
`OutlineGlyphCollection::with_format(&font, OutlineGlyphFormat::Cff2)` forces a specific outline source
when a font has both — useful for a "show me the CFF outline vs the glyf outline" editor view.

#### Hinting — **there is a real TrueType bytecode interpreter, a real CFF hinter and a full autohinter**
```rust
pub struct HintingInstance;
impl HintingInstance {
    pub fn new<'a>(outline_glyphs: &OutlineGlyphCollection, size: Size,
                   location: impl Into<LocationRef<'a>>, options: impl Into<HintingOptions>)
        -> Result<Self, DrawError>;
    pub fn reconfigure<'a>(&mut self, outlines: &OutlineGlyphCollection, size: Size,
                           location: impl Into<LocationRef<'a>>, options: impl Into<HintingOptions>)
        -> Result<(), DrawError>;
    pub fn size(&self) -> Size;
    pub fn location(&self) -> LocationRef<'_>;
    pub fn target(&self) -> Target;
    pub fn is_enabled(&self) -> bool;
}
pub struct HintingOptions { pub engine: Engine, pub target: Target }
pub enum Engine { Interpreter, Auto(Option<GlyphStyles>), #[default] AutoFallback }
pub enum Target { Mono, Smooth { mode: SmoothMode, symmetric_rendering: bool, preserve_linear_metrics: bool } }
pub enum SmoothMode { #[default] Normal, Light, Lcd, VerticalLcd }
```
* `src/outline/glyf/hint/engine/` is a complete TT VM (`arith`, `control_flow`, `cvt`, `data`, `delta`,
  `dispatch`, `graphics`, `logical`, `misc`, `outline`, `round`, `stack`, `storage`), with a
  `MAX_RUN_INSTRUCTIONS = 1_000_000` guard. `src/outline/glyf/mod.rs` loads `fpgm` and `prep` and runs
  them before each glyph's own instructions. **So `fpgm`/`prep`/glyf instructions all execute.**
* `src/outline/cff/hint.rs` implements PostScript hinting (`HintParams { blues, family_blues, other_blues,
  family_other_blues, blue_scale, blue_shift, blue_fuzz, language_group }`) — private module, driven
  automatically by `Engine::Interpreter` when the outline source is CFF/CFF2.
* `src/outline/autohint/` is a from-scratch Rust port of FreeType's `autofit` (public:
  `GlyphStyles`, `ScriptClass`, `ScriptGroup`, `StyleClass`, `SCRIPT_CLASSES`, `STYLE_CLASSES`,
  `BlueZones`, `Scale`, `ScaledStyleMetrics`, `Edge`, `Segment`, `Axis`, `Direction`, plus
  `EdgeHint`/`PointHint`/`HintAction` recorders that let you *inspect* what the autohinter decided).
* `autohint_shaping` (on by default) switches the autohinter from `ShaperMode::Nominal` (cmap only) to
  `ShaperMode::BestEffort` (runs GSUB to pick correct blue-zone reference glyphs). Keep it on.

**Grid-fit preview: VERIFIED.** In `examples/fontc_pipeline.rs` a hinted draw at 16 ppem returned
`M0.796875 11.203125 L8.796875 11.203125 L8.796875 0 L0.796875 0 Z` — i.e. grid-fit device-space
coordinates. In `examples/raster.rs`, hinting DejaVuSans 'g' at 16 ppem with `Engine::Interpreter`
gave bbox `x0: 0.890625, y0: -3.0, x1: 8.703125, y1: 9.0` and a plausible 10×14 bitmap.

**skrifa does not rasterize.** There is no scan converter in the crate; pen output must go to
tiny-skia / zeno (§2.j).

#### Metrics
```rust
pub struct Metrics {
    pub units_per_em: u16, pub glyph_count: u16, pub is_monospace: bool, pub italic_angle: f32,
    pub ascent: f32, pub descent: f32, pub leading: f32,
    pub cap_height: Option<f32>, pub x_height: Option<f32>,
    pub average_width: Option<f32>, pub max_width: Option<f32>,
    pub underline: Option<Decoration>, pub strikeout: Option<Decoration>,
    pub bounds: Option<BoundingBox>,
}
impl Metrics { pub fn new<'a>(font: &FontRef<'a>, size: Size, location: impl Into<LocationRef<'a>>) -> Self; }

pub struct GlyphMetrics<'a>;
impl<'a> GlyphMetrics<'a> {
    pub fn new(font: &FontRef<'a>, size: Size, location: impl Into<LocationRef<'a>>) -> Self;
    pub fn glyph_count(&self) -> u32;
    pub fn advance_width(&self, glyph_id: GlyphId) -> Option<f32>;      // applies HVAR/gvar phantom points
    pub fn left_side_bearing(&self, glyph_id: GlyphId) -> Option<f32>;
    pub fn bounds(&self, glyph_id: GlyphId) -> Option<BoundingBox>;
}
```

#### Color glyphs — COLRv0 **and** COLRv1
```rust
pub trait ColorPainter {
    fn push_transform(&mut self, transform: Transform);
    fn pop_transform(&mut self);
    fn push_clip_glyph(&mut self, glyph_id: GlyphId);
    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>);
    fn pop_clip(&mut self);
    fn fill(&mut self, brush: Brush<'_>);
    fn push_layer(&mut self, composite_mode: CompositeMode);
    // defaulted:
    fn fill_glyph(&mut self, gid: GlyphId, brush_transform: Option<Transform>, brush: Brush<'_>) { .. }
    fn paint_cached_color_glyph(&mut self, _g: GlyphId) -> Result<PaintCachedColorGlyph, PaintError> { .. }
    fn pop_layer(&mut self) {}
    fn pop_layer_with_mode(&mut self, _mode: CompositeMode) { self.pop_layer(); }
}
pub enum Brush<'a> {
    Solid { palette_index: u16, alpha: f32 },
    LinearGradient { p0, p1, color_stops: &'a [ColorStop], extend: Extend },
    RadialGradient { c0, r0, c1, r1, color_stops: &'a [ColorStop], extend: Extend },
    SweepGradient { c0, start_angle, end_angle, color_stops: &'a [ColorStop], extend: Extend },
}
pub struct ColorGlyph<'a>;
impl<'a> ColorGlyph<'a> {
    pub fn format(&self) -> ColorGlyphFormat;          // ColrV0 | ColrV1
    pub fn bounding_box(&self, location: impl Into<LocationRef<'a>>, size: Size) -> Option<BoundingBox<f32>>;
    pub fn paint(&self, location: impl Into<LocationRef<'a>>, painter: &mut impl ColorPainter)
        -> Result<(), PaintError>;
}
pub enum PaintError { ParseError(ReadError), GlyphNotFound(GlyphId), PaintCycleDetected, DepthLimitExceeded }
```
Full COLRv1 paint-graph traversal with cycle detection lives in `src/color/traversal.rs`.

#### Bitmap strikes — read-only
```rust
pub enum BitmapFormat { Sbix, Cbdt, Ebdt }
pub struct BitmapStrikes<'a>;
impl<'a> BitmapStrikes<'a> {
    pub fn new(font: &FontRef<'a>) -> Self;                 // prefers sbix > CBDT > EBDT
    pub fn with_format(font: &FontRef<'a>, format: BitmapFormat) -> Option<Self>;
    pub fn glyph_for_size(&self, size: Size, glyph_id: GlyphId) -> Option<BitmapGlyph<'a>>;
    pub fn iter(&self) -> impl Iterator<Item = BitmapStrike<'a>> + 'a + Clone;
}
pub struct BitmapGlyph<'a> {
    pub data: BitmapData<'a>,
    pub bearing_x: f32, pub bearing_y: f32,             // font units
    pub inner_bearing_x: f32, pub inner_bearing_y: f32, // pixels
    pub ppem_x: f32, pub ppem_y: f32,
    pub advance: Option<f32>, pub width: u32, pub height: u32,
    pub placement_origin: Origin,                        // TopLeft | BottomLeft
}
pub enum BitmapData<'a> { Bgra(&'a [u8]), Png(&'a [u8]), Mask(MaskData<'a>) }
pub struct MaskData<'a> { pub bpp: u8, pub is_packed: bool, pub data: &'a [u8] }
impl MaskData<'_> {
    pub fn decode(&self, width: u32, height: u32) -> Result<Vec<u8>, MaskDataDecodeError>;
    pub fn decode_to_slice(&self, w: u32, h: u32, dst: &mut [u8]) -> Result<(), MaskDataDecodeError>;
}
```
1/2/4/8-bpp packed and byte-aligned masks, plain PNG and 32-bit BGRA are all decoded. **There is no writer.**

#### `MetadataProvider` (12 methods, implemented for `FontRef`)
```rust
pub trait MetadataProvider<'a>: Sized {
    fn attributes(&self) -> Attributes;
    fn axes(&self) -> AxisCollection<'a>;
    fn named_instances(&self) -> NamedInstanceCollection<'a>;
    fn localized_strings(&self, id: StringId) -> LocalizedStrings<'a>;
    fn glyph_names(&self) -> GlyphNames<'a>;
    fn metrics(&self, size: Size, location: impl Into<LocationRef<'a>>) -> Metrics;
    fn glyph_metrics(&self, size: Size, location: impl Into<LocationRef<'a>>) -> GlyphMetrics<'a>;
    fn charmap(&self) -> Charmap<'a>;
    fn outline_glyphs(&self) -> OutlineGlyphCollection<'a>;
    fn color_glyphs(&self) -> ColorGlyphCollection<'a>;
    fn color_palettes(&self) -> ColorPalettes<'a>;
    fn bitmap_strikes(&self) -> BitmapStrikes<'a>;
}
```
Note: there is **no `variation_settings` method** — variation locations come from
`AxisCollection::location(&[("wght", 650.0)])`.

`skrifa::FontRef` **is** `read_fonts::FontRef` (a re-export), and `skrifa::raw` re-exports read-fonts wholesale.

---

### 2.d `fontir` / `fontc` / `fontdrasil` / `fontbe` 1.0.0

#### `fontir::source::Source` — **`pub`, object-safe via `Box<dyn Source>`, and third-party implementable**
```rust
pub trait Source {
    /// path is to the root entry, e.g. .glyphs file, .designspace, etc
    fn new(root: &Path) -> Result<Self, Error> where Self: Sized;
    fn create_static_metadata_work(&self) -> Result<Box<IrWork>, Error>;
    fn create_global_metric_work(&self) -> Result<Box<IrWork>, Error>;
    fn create_glyph_ir_work(&self) -> Result<Vec<Box<IrWork>>, Error>;
    fn create_feature_ir_work(&self) -> Result<Box<IrWork>, Error>;
    fn create_kerning_locations_ir_work(&self) -> Result<Box<IrWork>, Error>;
    fn create_kerning_instance_ir_work(&self, at: NormalizedLocation) -> Result<Box<IrWork>, Error>;
    fn create_color_palette_work(&self) -> Result<Box<IrWork>, Error>;
    fn create_color_glyphs_work(&self) -> Result<Box<IrWork>, Error>;
    fn compilation_flags(&self) -> Flags { Flags::empty() }   // defaulted
}
pub type IrWork = dyn Work<Context, WorkId, Error> + Send;
```
Corrections to the brief: there is **no** `create_kerning_group_ir_work` (groups ride inside each
`KerningInstance`) and **no** `create_paint_graph_work` (that is `create_color_glyphs_work`, whose
`WorkId` is `PaintGraph`).

#### The `Work` trait (`fontdrasil::orchestration`)
```rust
pub trait Work<C, I, E>: Debug where I: Identifier {
    fn id(&self) -> I;
    fn also_completes(&self) -> Vec<I> { Vec::new() }
    fn read_access(&self) -> Access<I> { Access::None }
    fn write_access(&self) -> Access<I> { /* own id + also_completes */ }
    fn exec(&self, context: &C) -> Result<(), E>;
}
pub enum Access<I> { None, Unknown, All, SpecificInstanceOfVariant(I), Variant(I), Set(HashSet<AccessType<I>>) }
pub struct AccessBuilder<I>;   // .variant(id) / .specific_instance(id) / .build()
```

#### `fontir::orchestration`
```rust
pub enum WorkId {
    StaticMetadata, GlobalMetrics, Glyph(GlyphName), PreliminaryGlyphOrder, GlyphOrder,
    PreliminaryGdefCategories, GdefCategories, Features, KerningLocations,
    KernInstance(NormalizedLocation), Anchor(GlyphName), ColorPalettes, PaintGraph,
}
bitflags! { pub struct Flags: u32 {
    const PREFER_SIMPLE_GLYPHS = 0b100;
    const FLATTEN_COMPONENTS = 0b1000;
    const DECOMPOSE_TRANSFORMED_COMPONENTS = 0b10000;
    const KEEP_DIRECTION = 0b1000000;
    const PRODUCTION_NAMES = 0b10000000;
    const DECOMPOSE_COMPONENTS = 0b100000000;
    const ERASE_OPEN_CORNERS = 0b1000000000;
    const PROPAGATE_ANCHORS = 0b10000000000;
} }
// Default = PREFER_SIMPLE_GLYPHS | PRODUCTION_NAMES

pub struct Context {
    pub flags: Flags,
    pub static_metadata: FeContextItem<ir::StaticMetadata>,
    pub preliminary_glyph_order: FeContextItem<ir::GlyphOrder>,
    pub glyph_order: FeContextItem<ir::GlyphOrder>,
    pub preliminary_gdef_categories: FeContextItem<ir::PreliminaryGdefCategories>,
    pub gdef_categories: FeContextItem<ir::GdefCategories>,
    pub global_metrics: FeContextItem<ir::GlobalMetrics>,
    pub glyphs: FeContextMap<ir::Glyph>,
    pub features: FeContextItem<ir::FeaturesSource>,
    pub kerning_locations: FeContextItem<ir::KerningLocations>,
    pub kerning_at: FeContextMap<ir::KerningInstance>,
    pub anchors: FeContextMap<ir::GlyphAnchors>,
    pub colors: FeContextItem<ir::ColorPalettes>,
    pub paint_graph: FeContextItem<ir::ColorGlyphs>,
}
```
`ContextItem`/`ContextMap` are `Arc<RwLock<…>>` — **pure in-memory, no disk cache on the
`generate_font` path.** `fontir::paths` does not exist; `fontdrasil::paths` only has filename sanitizers.

#### Key IR types
```rust
pub struct StaticMetadata {
    pub units_per_em: u16, pub all_source_axes: Axes,
    pub axes: Axes,                       // "If empty this is a static font."
    pub named_instances: Vec<NamedInstance>, pub variation_model: VariationModel,
    pub number_values: HashMap<NormalizedLocation, BTreeMap<SmolStr, OrderedFloat<f64>>>,
    pub names: HashMap<NameKey, String>, pub postscript_names: Option<PostscriptNames>,
    pub italic_angle: OrderedFloat<f64>, pub build_vertical: bool,
    pub misc: MiscMetadata, pub variations: Option<VariableFeature>,
}
impl StaticMetadata {
    pub fn new(units_per_em: u16, names: HashMap<NameKey, String>, axes: Vec<Axis>,
               named_instances: Vec<NamedInstance>, global_locations: HashSet<NormalizedLocation>,
               postscript_names: Option<PostscriptNames>, italic_angle: f64,
               glyphsapp_number_values: Option<HashMap<NormalizedLocation, BTreeMap<SmolStr, OrderedFloat<f64>>>>,
               build_vertical: bool) -> Result<StaticMetadata, VariationModelError>;
}
pub struct Glyph { pub name: GlyphName, pub emit_to_binary: bool, pub codepoints: HashSet<u32>, /* private sources */ }
impl Glyph { pub fn new(name: GlyphName, emit_to_binary: bool, codepoints: HashSet<u32>,
                        instances: HashMap<NormalizedLocation, GlyphInstance>) -> Result<Self, BadGlyph>; }
pub struct GlyphInstance { pub width: f64, pub height: Option<f64>, pub vertical_origin: Option<f64>,
                           pub contours: Vec<BezPath>, pub components: Vec<Component> }
pub struct Component { pub base: GlyphName, pub transform: Affine, pub anchor: Option<SmolStr> }

pub enum FeaturesSource {
    Empty,
    File   { fea_file: PathBuf, include_dir: Option<PathBuf> },
    Memory { fea_content: String, include_dir: Option<PathBuf> },   // <-- FEA from a String
}
pub struct KerningLocations { pub locations: BTreeSet<NormalizedLocation> }
pub struct KerningInstance {
    pub location: NormalizedLocation,
    pub kerns: BTreeMap<(KernSide, KernSide), OrderedFloat<f64>>,
    pub groups: BTreeMap<KernGroup, BTreeSet<GlyphName>>,   // per-instance, not font-wide
}
pub enum KernGroup { Side1(SmolStr), Side2(SmolStr) }
pub enum KernSide { Glyph(GlyphName), Group(KernGroup) }

pub enum AnchorKind { Base(GroupName), Mark(GroupName), Ligature { group_name, index },
                      ComponentMarker(usize), Caret(usize), VCaret(usize), CursiveEntry, CursiveExit }
pub struct AnchorBuilder;   // ::new(glyph_name) / .add(name, loc, pos) / .build() -> GlyphAnchors
pub struct GlyphOrder(IndexSet<GlyphName>);   // glyph_id(name), glyph_name(idx), insert, iter, names
```

#### `fontc`'s public API — **NOT what the brief expected**
`Config`, `JobTimer` and `create_timer` **do not exist in fontc 1.0.0** (grepped the whole crate; zero
hits). The real surface (`fontc/src/lib.rs`):
```rust
pub use error::Error;
pub use fontir::orchestration::Flags;
pub mod work;

pub enum Input { DesignSpacePath(PathBuf), GlyphsPath(PathBuf), FontraPath(PathBuf), GlyphsMemory(String) }
impl Input {
    pub fn new(path: &Path) -> Result<Self, Error>;         // dispatch on extension
    pub fn from_glyphs(source: String) -> Self;
    pub fn create_source(&self) -> Result<Box<dyn Source>, Error>;
}
impl TryFrom<&Path> for Input { .. }

#[derive(Debug, Default)]
pub struct Options {
    pub flags: Flags, pub flags_to_disable: DisableFlags,
    pub skip_features: bool, pub compile_debg: bool,
    pub output_file: Option<PathBuf>, pub debug_dir: Option<PathBuf>,
}

#[cfg(feature = "cli")]
pub fn run(input: Input, options: Options) -> Result<(), Error>;

/// Run and return an OpenType font. This is the library entry point to fontc.
pub fn generate_font(source: Box<dyn Source>, options: Options) -> Result<Vec<u8>, Error>;

pub fn init_paths(options: &Options) -> Result<(), Error>;      // no-op when output_file/debug_dir are None
pub fn write_font_file(options: &Options, be_context: &BeContext) -> Result<(), Error>;
pub fn require_dir(dir: &Path) -> Result<(), Error>;
pub fn version() -> String;
```
`generate_font` takes `Box<dyn Source>` and **never downcasts or matches on `Input`**; `Input` is only a
CLI convenience for picking among the four bundled readers.

**⇒ VERIFIED end to end.** `examples/custom_source.rs` implements `Source` for a toy editor model
(`EditorDoc { family, upem, glyphs: Vec<EditorGlyph>, fea: String, kern: Vec<(&str,&str,f64)> }`) with
nine `Work` impls, zero axes, `NormalizedLocation::new()` as the only location, and
`FeaturesSource::from_string(..)`. Output:
```
custom Source -> fontc::generate_font -> 1004 bytes
tables: GPOS GSUB OS/2 cmap glyf head hhea hmtx loca maxp name post
num_glyphs = 4
  name[FAMILY_NAME] = SpikeCustom
  name[SUBFAMILY_NAME] = Regular
  name[UNIQUE_ID] = SpikeCustom Regular
advance H = Some(700.0)
  shaped gid=2 x_adv=645       <- 700 - 55 kern, applied via generated GPOS
  shaped gid=3 x_adv=300
```
No filesystem access. `Source::new(&Path)` must exist to satisfy the trait but is never called by
`generate_font`; the editor constructs its `Source` directly.

Also **VERIFIED**: `examples/fontc_pipeline.rs` compiles an in-memory `.glyphs` 3 string via
`Input::from_glyphs(String)` → `create_source()` → `generate_font()`, producing
`GPOS GSUB OS/2 cmap glyf head hhea hmtx loca maxp name post`, 4 glyphs, upem 1000, kern applied
(A advance 600 → shaped 560).

#### How fontc uses fea-rs, kerning and anchors
Two-phase, in `fontbe/src/features.rs`:
1. `FeatureFirstPassWork` — `fea_rs::parse::parse_root(root_path, Some(glyph_map), resolver)` where
   `resolver` is fontbe's own `InMemoryResolver: SourceResolver` for `FeaturesSource::Memory`; then
   `fea_rs::compile::validate(ast, glyph_map, Some(&var_info))`; then a GSUB-only compile
   `fea_rs::compile::compile::<_, NopFeatureProvider>(&ast, &glyph_map, Some(&var_info), None, Opts::new().compile_gpos(false))`.
2. `FeatureCompilationWork` — the real compile with a `FeatureProvider`:
   ```rust
   fea_rs::compile::compile(&ast.ast, &marks.glyphmap, Some(&var_info),
                            Some(&feature_writer), Opts::new().compile_debg(compile_debg))
   ```
   where `FeatureWriter::add_features(&self, builder: &mut FeatureBuilder)` injects the generated
   kerning lookups (`FeaRsKerns`), mark lookups (`FeaRsMarks`) and feature variations, honouring
   `# Automatic Code` insertion markers in the user's FEA.

Kerning: `GatherIrKerningWork` → `AllKerningPairs` (split per script/direction via
`icu_properties::props::BidiClass`) → parallel `KerningFragmentWork` chunks of
`KERNS_PER_BLOCK = 256` building `write_fonts::tables::gpos::builders::PairPosBuilder` →
`KerningGatherWork` merges into `FeaRsKerns: FeatureProvider`. Pairs mixing bases and marks are split
to a mark-aware lookup, mirroring ufo2ft's `baseFeatureWriter.py`.

Anchors: `fontbe/src/features/marks.rs` groups glyphs by `AnchorKind` into `MarkGroup { bases, marks }`
and feeds `fea_rs::compile::{AnchorBuilder, MarkToBaseBuilder, MarkToLigBuilder, MarkToMarkBuilder,
CursivePosBuilder}`, producing `mark`/`mkmk`/`curs`/`abvm`/`blwm`.

#### Output tables — `fontbe::orchestration::WorkId`
```rust
pub enum WorkId {
    Features, FeaturesAst, Avar, Cmap, Colr, Cpal, Font, Fvar, Gasp, Glyf, GlyfFragment(GlyphName),
    Gpos, Gsub, Gdef, Gvar, GvarFragment(GlyphName), Head, Hhea, Hmtx, Hvar, Meta, Vhea, Vmtx, Vvar,
    GatherIrKerning, KernFragment(KernBlock), GatherBeKerning, Loca, LocaFormat, Marks, Maxp, Mvar,
    Name, Os2, Post, Stat, ExtraFeaTables,
}
pub struct ExtraFeaTables {   // tables lifted straight out of the FEA compilation
    pub name: Option<Name>, pub head: Option<Head>, pub hhea: Option<Hhea>, pub vhea: Option<Vhea>,
    pub os2: Option<Os2>, pub base: Option<Base>, pub stat: Option<Stat>,
    pub debg: Option<Vec<u8>>, pub os2_builder: Option<fea_rs::compile::Os2Builder>,
}
```
So the fontc pipeline can emit: **avar, BASE, cmap, COLR, CPAL, Debg, fvar, gasp, GDEF, glyf, GPOS,
GSUB, head, hhea, hmtx, HVAR, loca, maxp, meta, MVAR, name, OS/2, post, STAT, vhea, vmtx, VVAR, gvar.**
It does **not** emit: CFF, CFF2, EBLC/EBDT, CBLC/CBDT, sbix, SVG, kern, hdmx, VDMX, LTSH, MATH, JSTF,
DSIG, prep/fpgm/cvt (no hinting output at all).

#### Static single-master + in-memory: both fully supported
`StaticMetadata::axes` doc: *"If empty this is a static font."* Guards exist in
`fontbe/src/fvar.rs` (`if static_metadata.axes.is_empty() { trace!("Skip fvar; this is not a variable font"); return None }`),
`avar.rs` and `gvar.rs`. Proven by `examples/custom_source.rs` (zero axes, static TTF out).

In-memory: `Context` is `Arc<RwLock<…>>`-only, `init_paths`/`write_font_file` are no-ops when
`Options::output_file`/`debug_dir` are `None`, and `FeaturesSource::Memory` covers FEA. The bundled
`GlyphsIrSource::new_from_memory(data: &str)` supports in-memory `.glyphs`;
`ufo2fontir::DesignSpaceIrSource` has **no** memory constructor (UFO is a directory format) — for
in-memory UFO you write your own `Source` (which is exactly what we want anyway).

---

### 2.e `fea-rs` 1.0.0

```rust
pub use common::{GlyphIdent, GlyphMap, GlyphSet};
pub use compile::{Compiler, Opts};
pub use diagnostic::{Diagnostic, DiagnosticSet, Level};
pub use parse::{ParseTree, TokenSet};
pub use token_tree::{Kind, Node, NodeOrToken, Token, typed};
```
There is **no `FeaSource` trait**. The real functions:
```rust
pub fn compile<V: VariationInfo, T: FeatureProvider>(
    tree: &ParseTree, glyph_map: &GlyphMap,
    var_info: Option<&V>, extra_features: Option<&T>, opts: Opts,
) -> Result<(Compilation, DiagnosticSet), DiagnosticSet>;

pub fn validate<V: VariationInfo>(node: &ParseTree, glyph_map: &GlyphMap, fvar: Option<&V>) -> DiagnosticSet;
```
```rust
pub struct Compilation {
    pub head: Option<write_fonts::tables::head::Head>,
    pub hhea: Option<write_fonts::tables::hhea::Hhea>,
    pub vhea: Option<write_fonts::tables::vhea::Vhea>,
    pub os2:  Option<write_fonts::tables::os2::Os2>,
    pub os2_builder: Option<Os2Builder>,
    pub gdef: Option<write_fonts::tables::gdef::Gdef>,
    pub base: Option<write_fonts::tables::base::Base>,
    pub name: Option<write_fonts::tables::name::Name>,
    pub stat: Option<write_fonts::tables::stat::Stat>,
    pub gsub: Option<write_fonts::tables::gsub::Gsub>,
    pub gpos: Option<write_fonts::tables::gpos::Gpos>,
    pub gdef_classes: Option<HashMap<GlyphId16, GlyphClassDef>>,
    pub insert_markers: HashMap<Tag, InsertionPoint>,
    pub debg: Option<Vec<u8>>,
}
impl Compilation {
    pub fn to_font_builder(&self) -> Result<write_fonts::FontBuilder<'_>, BuilderError>;
    pub fn to_binary(&self, glyph_map: &GlyphMap) -> Result<Vec<u8>, BuilderError>;
    pub fn has_non_layout_tables(&self) -> bool;
    pub fn remap_name_ids(&mut self, first_avail_id: u16);
}
pub struct Opts;   // .make_post_table(bool) .max_error_messages(usize) .compile_gsub(bool)
                   // .compile_gpos(bool) .compile_debg(bool); defaults: gsub+gpos on, post off, 100 errors
```

#### Compiling a .fea **string**
```rust
/// Panics if the input contains any include statements.
pub fn parse_string(text: impl Into<Arc<str>>) -> (ParseTree, DiagnosticSet);
/// With includes: supply your own resolver.
pub fn parse_root(path: PathBuf, glyph_map: Option<&GlyphMap>, resolver: Box<dyn SourceResolver>)
    -> Result<(ParseTree, DiagnosticSet), SourceLoadError>;
pub trait SourceResolver {
    fn get_contents(&self, path: &Path) -> Result<Arc<str>, SourceLoadError>;
    fn resolve_raw_path(&self, path: &Path, included_from: Option<&Path>) -> PathBuf { .. }
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, SourceLoadError> { .. }
    fn resolve(&self, path: &Path) -> Result<Source, SourceLoadError> { .. }
}
// blanket impl for any Fn(&Path) -> Result<Arc<str>, SourceLoadError>
pub struct FileSystemResolver; impl FileSystemResolver { pub fn new(project_root: PathBuf) -> Self; }
```
Copy `fontbe`'s `InMemoryResolver` (`fontbe/src/features.rs`, ~L87–128) for "editor buffer as root FEA,
disk fallback for `include()`".

#### `GlyphMap`
```rust
pub enum GlyphIdent { Name(GlyphName), Cid(u16) }   // From<&str>, From<GlyphName>, From<SmolStr>, From<u16>
pub struct GlyphMap;
impl GlyphMap {
    pub fn new<T: Into<GlyphIdent>, I: IntoIterator<Item = T>>(iter: I) -> Result<Self, GlyphOrderError>;
    pub fn get<Q: ?Sized + AsGlyphIdent>(&self, key: &Q) -> Option<GlyphId16>;
    pub fn contains<Q: ?Sized + AsGlyphIdent>(&self, key: &Q) -> bool;
    pub fn reverse_map(&self) -> BTreeMap<GlyphId16, GlyphIdent>;
    pub fn iter(&self) -> impl Iterator<Item = GlyphIdent> + '_;
    pub fn make_post_table(&self) -> write_fonts::tables::post::Post;
}
pub fn parse_glyph_order(glyphs: &str) -> Result<GlyphMap, GlyphOrderError>;   // one name per line
```
There is **no `FromIterator` impl** — use `GlyphMap::new([...])`. GID = iteration index.

#### Diagnostics
```rust
pub enum Level { Error, Warning, Info }
pub struct Diagnostic { pub message: Message, pub level: Level }
impl Diagnostic { pub fn text(&self) -> &str; pub fn span(&self) -> Range<usize>; pub fn is_error(&self) -> bool; }
pub struct DiagnosticSet;
impl DiagnosticSet {
    pub fn len(&self) -> usize; pub fn has_errors(&self) -> bool;
    pub fn set_max_to_print(&mut self, n: usize);
    pub fn discard_warnings(&mut self);
    pub fn split_off_warnings(&mut self) -> Option<DiagnosticSet>;
    pub fn diagnostics(&self) -> &[Diagnostic];
    pub fn display(&self) -> impl std::fmt::Display + '_;
}
pub enum CompilerError { SourceLoad(..), ParseFail(DiagnosticSet), ValidationFail(DiagnosticSet),
                         CompilationFail(DiagnosticSet), WriteFail(BuilderError) }
```
**VERIFIED** (`examples/roundtrip.rs`) — compiled a 14-line FEA string over a 7-glyph `GlyphMap`:
```
fea-rs parse diagnostics: 0 (clean)
fea-rs compile OK; warnings=0; gsub=true gpos=true gdef=true name=false head=false os2=false stat=false base=false
Compilation::to_binary -> 312 B, tables: GDEF GPOS GSUB maxp
GSUB lookups: 1
GPOS lookups: 1

validate() on bad fea -> 1 diagnostics, has_errors=true
error: glyph not in font
in parse::parse_string at 1:26
  |
1 | feature liga { sub f i by nonexistent; } liga;
  |                           ^^^^^^^^^^^
```
`Diagnostic::span()` + `DiagnosticSet::display()` give byte ranges and rendered carets — ready to wire
into a CodeMirror/Monaco FEA editor in the TS UI.

#### Variable support
```rust
pub trait VariationInfo {
    type Error: std::error::Error;
    fn axis_count(&self) -> u16;
    fn axis(&self, axis_tag: Tag) -> Option<(usize, &fontdrasil::types::Axis)>;
    fn resolve_variable_metric(&self, locations: &HashMap<NormalizedLocation, i16>)
        -> Result<(i16, Vec<(VariationRegion, i16)>), Self::Error>;
    fn resolve_glyphs_number_value(&self, name: &str)
        -> Result<HashMap<NormalizedLocation, f64>, Self::Error>;
}
pub struct NopVariationInfo;   // 0 axes
pub enum AxisLocation { User(f64), Design(f64), Normalized(f64) }
```
Variable metrics in FEA syntax **are supported** (a fontc/Glyphs superset, not AFDKO spec):
`<anchor 0 (wght=200:-100 wght=900:-150 wght=900,wdth=150:-120)>` parses to
`AstKind::VariableMetricNode` and compiles to `Metric { default: i16, device_or_deltas: DeviceOrDeltas }`
via `VariationInfo::resolve_variable_metric`.

#### No FEA writer
Grepped the whole crate: **no `.fea` pretty-printer / serializer exists.** fea-rs is one-directional
(text → tables). Emitting FEA from an editor model is ours to write.

---

### 2.f `harfrust` 0.13.3

```rust
pub use read_fonts::{types::{GlyphId, Tag}, FontRef};      // harfrust::FontRef IS read_fonts::FontRef
pub use hb::buffer::{GlyphBuffer, GlyphFlags, GlyphInfo, GlyphPosition, UnicodeBuffer};
pub use hb::common::{script, Direction, Feature, Language, Script, Variation};
pub use hb::face::{hb_font_t as Shaper, GlyphExtents, ShapeOptions, ShaperBuilder, ShaperData, ShaperInstance};
pub use hb::ot_shape_plan::{hb_ot_shape_plan_t as ShapePlan, ShapePlanKey};
pub type NormalizedCoord = read_fonts::types::F2Dot14;
```
```rust
impl ShaperData {
    pub fn new(font: &FontRef) -> Self;
    pub fn shaper<'a>(&'a self, font: &FontRef<'a>) -> ShaperBuilder<'a>;
}
impl ShaperInstance {
    pub fn from_variations<V>(font: &FontRef, variations: V) -> Self
        where V: IntoIterator, V::Item: Into<Variation>;
    pub fn from_coords(font: &FontRef, coords: impl IntoIterator<Item = NormalizedCoord>) -> Self;
    pub fn from_named_instance(font: &FontRef, index: usize) -> Self;
    pub fn set_variations<V>(&mut self, font: &FontRef, variations: V);
    pub fn coords(&self) -> &[F2Dot14];
}
impl<'a> ShaperBuilder<'a> {
    pub fn instance(self, instance: Option<&'a ShaperInstance>) -> Self;
    pub fn build(self) -> Shaper<'a>;
}
impl<'a> ShapeOptions<'a> {
    pub fn new() -> Self;
    pub fn plan(self, plan: Option<&'a ShapePlan>) -> Self;
    pub fn scale(self, scale: Option<i32>) -> Self;
    pub fn scale_separate(self, scale: Option<(i32, i32)>) -> Self;
    pub fn point_size(self, point_size: Option<f32>) -> Self;
    pub fn features(self, features: &'a [Feature]) -> Self;
    pub fn font_funcs(self, funcs: Option<&'a mut (dyn FontFuncs + 'a)>) -> Self;
}
impl<'a> Shaper<'a> {
    pub fn units_per_em(&self) -> i32;
    pub fn coords(&self) -> &'a [NormalizedCoord];
    pub fn shape(&self, buffer: UnicodeBuffer, options: ShapeOptions<'_>) -> GlyphBuffer;
}
#[repr(C)] pub struct GlyphInfo { pub glyph_id: u32, mask, pub cluster: u32, vars }
impl GlyphInfo { pub fn unsafe_to_break(&self) -> bool; pub fn unsafe_to_concat(&self) -> bool;
                 pub fn safe_to_insert_tatweel(&self) -> bool; }
#[repr(C)] pub struct GlyphPosition { pub x_advance: i32, pub y_advance: i32,
                                      pub x_offset: i32, pub y_offset: i32, var }
impl GlyphBuffer { pub fn serialize(&self, font: &impl SerializerFont, flags: SerializeFlags) -> String; }
```
**Note:** `Shaper::shape` takes `ShapeOptions<'_>`, **not** a `&[Feature]` slice — the brief's assumed
signature is wrong. Use `ShapeOptions::new().features(&feats)`.

**Shaping an in-memory write-fonts font: VERIFIED.** `harfrust::FontRef` is literally
`read_fonts::FontRef`, so `FontBuilder::build() -> Vec<u8>` → `FontRef::from_index(&bytes, 0)` →
`ShaperData::new(&f)` works with zero glue. Demonstrated three times in this spike (fontc output,
custom-Source output, and a rebuilt DejaVuSans).

AAT (`morx`, `mort`, `kerx`, `ankr`, `feat`, `ltag`, `trak`) **is implemented** but the `hb::aat` module
is private — `ShaperData::new` builds an `AatCache` automatically and the shaper selects OT vs AAT the
way HarfBuzz does. No public API to force one.

README: *"Matches HarfBuzz v14.3.1."* Caveats it states: malformed fonts error out instead of falling
back to a dummy shaper; most of "boring-expansion-spec" unsupported; "less than 25% slower than
HarfBuzz on most common fonts".

---

### 2.g `norad` 0.18.4 (UFO)

```rust
pub use data_request::DataRequest;
pub mod datastore; pub mod designspace; pub mod error; pub mod fontinfo;
pub mod groups; pub mod kerning;
pub use font::{Font, FormatVersion, MetaInfo};
pub use fontinfo::FontInfo;
pub use glyph::{AffineTransform, Anchor, Codepoints, Component, Contour, ContourPoint, Glyph, Image, PointType};
pub use name::Name;
pub use groups::Groups;
pub use guideline::{Guideline, Line};
pub use identifier::Identifier;
pub use kerning::Kerning;
pub use layer::{Layer, LayerContents};
pub use shared_types::{Color, Plist};
pub use util::user_name_to_file_name;
pub use write::{QuoteChar, WriteOptions};
```
```rust
#[non_exhaustive]
pub struct Font {
    pub meta: MetaInfo, pub font_info: FontInfo,
    pub layers: LayerContents,     // NOT "LayerSet"
    pub lib: Plist,                // = plist::Dictionary
    pub groups: Groups,            // = BTreeMap<Name, Vec<Name>>
    pub kerning: Kerning,          // = BTreeMap<Name, BTreeMap<Name, f64>>
    pub features: String,          // raw features.fea text
    pub data: DataStore, pub images: ImageStore,
}
impl Font {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Font, FontLoadError>;
    pub fn load_requested_data(path: impl AsRef<Path>, request: DataRequest) -> Result<Font, FontLoadError>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FontWriteError>;
    pub fn save_with_options(&self, path: impl AsRef<Path>, options: &WriteOptions) -> Result<(), FontWriteError>;
    pub fn default_layer(&self) -> &Layer;  pub fn default_layer_mut(&mut self) -> &mut Layer;
    pub fn iter_layers(&self) -> impl Iterator<Item = &Layer>;
    pub fn get_glyph(&self, key: &str) -> Option<&Glyph>;
    pub fn glyph_count(&self) -> usize;
    pub fn guidelines(&self) -> &[Guideline];
    pub fn kerning_resolver(&self) -> KerningResolver<'_>;
}
```
Method name correction: it is **`load_requested_data`**, not `load_requested_contents`.
`DataRequest` is a builder: `all()`, `none()`, `.layers(bool)`, `.default_layer(bool)`,
`.filter_layers(|name, path| ..)`, `.lib/.groups/.kerning/.features/.data/.images(bool)` — this is the
lever for fast partial loads in an editor.

`LayerContents`: `get`, `get_mut`, `default_layer(_mut)`, `iter(_mut)`, `names`, `new_layer(&str)`,
`get_or_create_layer`, `remove`, `rename_layer`, `retain`, `remove_empty_layers`.
`Layer`: `name`, `path`, `is_default`, `entry(Name)`, `get_glyph(_mut)`, `contains_glyph`,
`insert_glyph`, `remove_glyph`, `rename_glyph`, `iter(_mut)`, `retain`, `clear`.

```rust
pub struct Glyph {
    name: Name,                        // PRIVATE -- use .name()
    pub height: f64, pub width: f64, pub codepoints: Codepoints,
    pub note: Option<String>, pub guidelines: Vec<Guideline>, pub anchors: Vec<Anchor>,
    pub components: Vec<Component>, pub contours: Vec<Contour>,
    pub image: Option<Image>, pub lib: Plist,
}
pub struct Contour { pub points: Vec<ContourPoint>, identifier, lib }
impl Contour { pub fn is_closed(&self) -> bool;
               #[cfg(feature = "kurbo")] pub fn to_kurbo(&self) -> Result<kurbo::BezPath, ConvertContourError>; }
pub struct ContourPoint { pub x: f64, pub y: f64, pub typ: PointType, pub smooth: bool,
                          pub name: Option<Name>, identifier, lib }
pub enum PointType { Move, Line, OffCurve, Curve, QCurve }
pub struct AffineTransform { pub x_scale, xy_scale, yx_scale, y_scale, x_offset, y_offset: f64 }
pub enum Line { Vertical(f64), Horizontal(f64), Angle { x: f64, y: f64, degrees: f64 } }
```
`identifier`/`lib` on `Anchor`/`Component`/`Contour`/`ContourPoint`/`Guideline` are **private** with
accessor methods (`.identifier()`, `.lib()`, `.lib_mut()`, `.replace_lib()` behind `object-libs`).

`FontInfo` has **108 public fields** and is "hard-wired to the available attributes in UFO version 3" —
effectively complete UFO3 `fontinfo.plist` coverage (all `openTypeOS2*`, `openTypeHhea*`,
`openTypeVhea*`, `openTypeName*` + `openTypeNameRecords`, `openTypeHead*`, `openTypeGaspRangeRecords`,
`postscript*` incl. blues/stem snaps, all `woffMetadata*`, guidelines, macintoshFOND*). It uses
`Integer`/`Float`/`IntegerOrFloat` wrappers to preserve int-vs-float on round trip.

**`norad` DOES have a designspace module** (contradicting the brief's doubt):
```rust
pub struct DesignSpaceDocument {
    pub format: f32, pub axes: Vec<Axis>, pub axis_mappings: Option<AxisMappings>,   // avar2
    pub rules: Rules, pub sources: Vec<Source>, pub instances: Vec<Instance>, pub lib: Dictionary,
}
impl DesignSpaceDocument {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<DesignSpaceDocument, DesignSpaceLoadError>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DesignSpaceSaveError>;
}
```
`ufo2fontir` 1.0.0 depends on `norad = "0.18.4", default-features = false` and uses
`norad::designspace::{self, Dimension}` directly — so **norad is the designspace crate the whole
googlefonts stack uses.** No separate crate needed.

**Fidelity caveats:**
* `Font::save` **errors** if `meta.format_version != FormatVersion::V3`
  (`return Err(FontWriteError::Downgrade)`, "downgrading below UFO v3 is not currently supported").
  `Font::load` upconverts v1/v2 → v3 in memory. So **norad reads UFO 1/2/3 and writes UFO 3 only.**
* With the default `object-libs` feature, `public.objectLibs` is norad-managed; setting it manually
  yields `FontWriteError::PreexistingPublicObjectLibsKey`.
* Unknown `lib` keys round-trip fine (`Plist = plist::Dictionary`).
* `WriteOptions` controls only XML formatting (`indent(char, count)`, `quote_char(QuoteChar)`).
* `Font::save` **removes and recreates the whole target directory** — not an incremental save. For an
  editor with a large UFO on a slow disk this matters; we may want our own incremental writer.
* glif: reads format 1.0 and 2.0, writes 2.0 only.

---

### 2.h `kurbo` 0.13.1

*(see §2.h-detail below — filled from the dedicated source read)*

---

### 2.i `glyphs-reader` 1.0.0

```rust
pub use font::{Anchor, Axis, AxisRule, Color, ColorStop, Component, CustomParameters, FeatureSnippet,
               Font, FontMaster, Glyph, InstanceType, Layer, LayerAttributes, Node, NodeType, Path,
               Shape, ShapeAttributes, glyphs_to_opentype_lang_id};
pub use plist::Plist;
pub mod error; pub mod glyphdata;
```
```rust
impl Font {
    pub fn load_from_string(data: &str) -> Result<Font, Error>;
    pub fn load(glyphs_file: &Path) -> Result<Font, Error>;   // .glyphs OR .glyphspackage directory
}
pub struct Font {
    pub units_per_em: u16, pub axes: Vec<Axis>, pub masters: Vec<FontMaster>,
    pub default_master_idx: usize, pub glyphs: BTreeMap<SmolStr, Glyph>, pub glyph_order: Vec<SmolStr>,
    pub axis_mappings: UserToDesignMapping, pub virtual_masters: Vec<BTreeMap<String, OrderedFloat<f64>>>,
    pub features: Vec<FeatureSnippet>, pub all_names: BTreeMap<String, Vec<(String, String)>>,
    pub instances: Vec<Instance>, pub version_major: i32, pub version_minor: u32,
    pub date: Option<String>, pub kerning_ltr: Kerning, pub kerning_rtl: Kerning,
    pub custom_parameters: CustomParameters, pub user_data: BTreeMap<SmolStr, Plist>,
}
pub struct Kerning(BTreeMap<String, BTreeMap<(SmolStr, SmolStr), OrderedFloat<f64>>>);  // master -> pair -> value
```
**Glyphs 2 and 3 both supported.** Version detection:
```rust
enum FormatVersion { #[default] V2, V3 }
impl FromPlist for FormatVersion {
    fn parse(tokenizer: &mut Tokenizer) -> Result<Self, crate::plist::Error> {
        let raw: i64 = FromPlist::parse(tokenizer)?;
        if raw == 3 { Ok(FormatVersion::V3) }
        else { Err(crate::plist::Error::Parse("'3' is the only known format version".into())) }
    }
}
```
Absence of `.formatVersion` ⇒ V2 (Glyphs 2 files omit it). `FormatVersion::codepoint_radix()` is 16 for
v2 (hex unicode strings) and 10 for v3; v2 `paths`/`components` vs v3 unified `shapes`; v2 bracket-layer
name suffixes vs v3 `attributes.axisRules` — all handled.
`.glyphspackage` directories are detected by extension and read (`fontinfo.plist` + `glyphs/*.glyph`,
honouring `order.plist`, ignoring `UIState.plist`).

```rust
pub struct Glyph { pub name: SmolStr, pub export: bool, pub layers: Vec<Layer>,
                   pub bracket_layers: Vec<Layer>, pub unicode: BTreeSet<u32>,
                   pub left_kern: Option<SmolStr>, pub right_kern: Option<SmolStr>,
                   pub category: Option<Category>, pub sub_category: Option<Subcategory>,
                   pub production_name: Option<SmolStr>,
                   pub smart_component_axes: BTreeMap<SmolStr, RangeInclusive<i64>> }
pub struct Layer { pub layer_id: String, pub associated_master_id: Option<String>,
                   pub width: OrderedFloat<f64>, pub vert_width: Option<OrderedFloat<f64>>,
                   pub vert_origin: Option<OrderedFloat<f64>>, pub shapes: Vec<Shape>,
                   pub anchors: Vec<Anchor>, pub attributes: LayerAttributes,
                   pub smart_component_positions: BTreeMap<SmolStr, AxisPole>, pub hints: Vec<Hint> }
pub enum Shape { Path(Path), Component(Component) }
pub struct Path { pub closed: bool, pub nodes: Vec<Node>, pub attributes: ShapeAttributes }
pub struct Component { pub name: SmolStr, pub transform: Affine, pub anchor: Option<SmolStr>,
                       pub smart_component_values: BTreeMap<SmolStr, f64>, pub attributes: ShapeAttributes }
pub struct Node { pub pt: Point, pub node_type: NodeType }
pub enum NodeType { Line, LineSmooth, OffCurve, Curve, CurveSmooth, QCurve, QCurveSmooth }
pub struct Anchor { pub name: SmolStr, pub pos: Point }
pub struct FontMaster { pub id: String, pub name: String, pub axes_values: Vec<OrderedFloat<f64>>,
                        metric_values /* private; .ascender()/.descender()/.x_height()/.cap_height()/.italic_angle() */,
                        pub number_values, pub custom_parameters: CustomParameters,
                        pub user_data, pub metrics_source_id: Option<String> }
pub struct Instance { pub name: String, pub active: bool, pub type_: InstanceType,
                      pub axis_mappings, pub axes_values, pub custom_parameters, properties }
pub enum InstanceType { Single, Variable }
```
`CustomParameters` is a ~50-field struct with recognized parameter strings including
`"Propagate Anchors"`, `"Use Typo Metrics"`, `"typoAscender"`/`"typoDescender"`/`"typoLineGap"`,
`"winAscent"`/`"winDescent"`, `"hheaAscender"`… , `"vheaVert*"`, `"underlineThickness/Position"`,
`"strikeoutPosition/Size"` (+ `openTypeOS2*` aliases), `"sub/superscript*"`, `"fsType"`,
`"unicodeRanges"`, `"codePageRanges"`, `"openTypeHeadLowestRecPPEM"`, `"openTypeHheaCaretSlope*"`,
`"meta Table"`, `"Don't use Production Names"`, `"Link Metrics With First Master"`,
`"Link Metrics With Master"`, `"openTypeHeadFlags"`, `"Virtual Master"`, `"panose"`, `"glyphOrder"`,
`"gasp Table"`, `"Feature for Feature Variations"`, `"Color Palettes"`, `"Name Table Entry"`.
`"openTypeOS2FamilyClass"` is recognized-but-ignored (warning). **Unrecognized custom parameters are
dropped with a one-time `log::warn!` — silent data loss on round trip.**

**There is NO `.glyphs` writer.** No `Serialize` impls, no `save`, no `to_glyphs_string`. Despite the
module doc saying "reading and writing", the crate is **import-only**. Its own TODOs flag "numerous
unsupported attributes" on `Shape`/`RawShape`, incomplete `LayerAttributes` parsing, and unhandled hint
types mapped to `HintType::Unknown`.

---

### 2.j Rasterization — `tiny-skia` 0.12.0 / `zeno` 0.3.3

`tiny_skia::Mask` gives a **plain `Vec<u8>` coverage buffer** — exactly what an EBDT/CBDT strike needs:
```rust
pub enum MaskType { Alpha, Luminance }
impl Mask {
    pub fn new(width: u32, height: u32) -> Option<Self>;
    pub fn from_pixmap(pixmap: PixmapRef, mask_type: MaskType) -> Self;
    pub fn data(&self) -> &[u8];
    pub fn data_mut(&mut self) -> &mut [u8];
    pub fn take(self) -> Vec<u8>;
    pub fn fill_path(&mut self, path: &Path, fill_rule: FillRule, anti_alias: bool, transform: Transform);
}
```
`zeno::Mask` additionally returns a `Placement { left, top, width, height }`, i.e. it computes the
bitmap origin/bearings for you:
```rust
let (buf, placement) = zeno::Mask::new(&commands[..])
    .format(zeno::Format::Alpha)
    .origin(zeno::Origin::BottomLeft)
    .render();
```
Note `zeno::PathData` is **not dyn-compatible** (`fn copy_to(&self, sink: &mut impl PathBuilder)`), so
pass `&cmds[..]`, not `&dyn PathData`.

**VERIFIED** on DejaVuSans 'g', TT-interpreter-hinted at 16 ppem (`examples/raster.rs`):
```
glyph 'g' @ 16ppem hinted bbox = Rect { x0: 0.890625, y0: -3.0, x1: 8.703125, y1: 9.0 }

tiny-skia Mask 10x14, 140 bytes of u8 coverage:      zeno placement { left: 0, top: -3, w: 9, h: 12 }:
  |          |                                         |  -#%#.**|
  |  .*@#.=* |                                         | :%= .*#*|
  | .@+ .++* |                                         | **   .%*|
  | +#    %* |                                         | %=    #*|
  | #+    ** |                                         | @-    **|
  | @=    ** |                                         | %=    #*|
  | #+    ** |                                         | **   .%*|
  | +#    %* |                                         | :%= .*#*|
  | .@+ .++* |                                         |  -#%#.**|
  |  .*@#.** |                                         |       %=|
  |       #+ |                                         |  +. .*%.|
  |  *. .+@. |                                         |  -#%%*. |
  |  :#@%*:  |
  |          |
thresholded >=128: 49 set pixels of 140
```
**Recommendation:** use **zeno** for glyph→bitmap (it hands back the placement, which becomes the
EBDT `BigGlyphMetrics`/`SmallGlyphMetrics` bearings directly, and it is a tiny dependency), and
**tiny-skia** for canvas/thumbnail compositing where you want gradients, strokes, blend modes and PNG
I/O (`Pixmap::save_png`, `Pixmap::decode_png`). Both are pure Rust with no C dependencies.

---

### 2.k Boolean path operations — **`flo_curves` is the answer**

#### Survey
| Crate | Ops | Representation | Verdict |
|---|---|---|---|
| **`flo_curves` 0.8.1** | `path_add`, `path_sub`, `path_intersect`, `path_cut`, `path_full_intersect`, `path_remove_interior_points`, `path_remove_overlapped_points`, `path_add_chain`, `path_combine` | **cubic Bézier**, exact | **Use this.** |
| `i_overlay` 8.1.0 | `OverlayRule::{Subject, Clip, Intersect, Union, Difference, InverseDifference, Xor}` | polygons (flattened) | fallback / fast path; loses curves |
| `cavalier_contours` 0.9.0 | `BooleanOp::{Or, And, Not, Xor}` | polylines with **arc bulges** | wrong primitive for font outlines |
| `lyon_algorithms` 1.0.21 | **none** — modules are `aabb, area, fit, hatching, hit_test, length, measure, raycast, rect, rounded_polygon, walk, winding` | — | no booleans at all |
| `kurbo` 0.13.1 | none | — | confirmed no booleans |
| `geo` 0.33 | polygon booleans (`geo::BooleanOps`) | polygons | not evaluated further; same flattening problem as i_overlay |

#### `flo_curves` signatures (verbatim, `src/bezier/path/arithmetic/`)
```rust
pub fn path_add<POut>(path1: &Vec<impl BezierPath<Point = POut::Point>>,
                      path2: &Vec<impl BezierPath<Point = POut::Point>>, accuracy: f64) -> Vec<POut>
    where POut: BezierPathFactory, POut::Point: Coordinate + Coordinate2D;
pub fn path_sub<POut>(path1: &Vec<..>, path2: &Vec<..>, accuracy: f64) -> Vec<POut> where ..;
pub fn path_intersect<POut>(path1: &Vec<..>, path2: &Vec<..>, accuracy: f64) -> Vec<POut> where ..;
pub fn path_cut<POut>(path1: &Vec<..>, path2: &Vec<..>, accuracy: f64) -> PathCut<POut> where ..;
pub fn path_full_intersect<POut>(path1: &Vec<..>, path2: &Vec<..>, accuracy: f64) -> PathIntersection<POut>;
pub fn path_remove_interior_points<P1: BezierPath, POut: BezierPathFactory>(path: &Vec<P1>, accuracy: f64) -> Vec<POut>;
pub fn path_remove_overlapped_points<P1: BezierPath, POut: BezierPathFactory>(path: &Vec<P1>, accuracy: f64) -> Vec<POut>;
pub fn path_add_chain<POut>(paths: &Vec<Vec<impl BezierPath<Point = POut::Point>>>, accuracy: f64) -> Vec<POut>;
pub fn path_combine<P: BezierPathFactory>(operation: PathCombine<P>, accuracy: f64) -> Vec<P>;

pub type SimpleBezierPath = (Coord2, Vec<(Coord2, Coord2, Coord2)>);   // start point + (c1, c2, end)*
pub struct BezierPathBuilder<P: BezierPath>;
impl<P: BezierPath> BezierPathBuilder<P> {
    pub fn start(start: P::Point) -> BezierPathBuilder<P>;
    pub fn line_to(self, point: P::Point) -> Self;
    pub fn curve_to(self, (cp1, cp2): (P::Point, P::Point), end_point: P::Point) -> Self;
    pub fn build(self) -> P;
}
```
Bonus: `flo_curves::bezier::path::stroke_path` (expand stroke on Bézier paths, with
`StrokeOptions { accuracy, min_sample_distance, join: LineJoin, start_cap/end_cap: LineCap, closed,
remove_interior_points }`) and `flo_curves::bezier::{fit, offset, offset_lms, offset_scaling,
offset_subdivision_lms, nearest_point, intersection, overlaps, characteristics, length, walk,
distort, deform, vectorize, rasterize}`.

#### Two-overlapping-circles test (**VERIFIED**, `examples/boolops.rs`, release build)
Circles r = 100 at (0,0) and (120,0), built as `kurbo::Circle::to_path(0.01)` (5 cubic segments each),
converted to `SimpleBezierPath`, results converted back to `kurbo::BezPath` and measured with
`BezPath::area()` (Green's theorem, exact for cubics). Analytic expectations:
lens = 8945.904, union = 53885.949, A−B = 22470.022.

```
--- flo_curves 0.8.1 (accuracy = 0.01) ---
path_add (union)       subpaths=1  cubic_segs=9   degenerate(line-like)=0  max_ctrl_dev=25.464  area=53889.205  err=0.0060%
path_intersect         subpaths=1  cubic_segs=5   degenerate(line-like)=0  max_ctrl_dev=25.464  area= 8947.243  err=0.0150%
path_sub (A - B)       subpaths=1  cubic_segs=8   degenerate(line-like)=0  max_ctrl_dev=25.464  area=22470.987  err=0.0043%
remove_overlapped_pts  subpaths=2  cubic_segs=14  degenerate(line-like)=0                       area=62836.448  err=16.6101%   <-- NOT a union
remove_interior_pts    subpaths=1  cubic_segs=9   degenerate(line-like)=0                       area=53889.205  err=0.0060%   <-- this IS "Remove Overlap"

--- i_overlay 8.1.0 (flattened at tol=0.05) --- subj pts=106 clip pts=106
Union        shapes=1  total_pts=151  area=53862.761  err=0.0430%
Intersect    shapes=1  total_pts= 63  area= 8936.154  err=0.1090%
Difference   shapes=1  total_pts=106  area=22463.303  err=0.0299%

remove-overlap of 12 overlapping circles: 2 subpaths / 36 cubic segs in 497.904µs   (flo_curves)
i_overlay same case:                      1 shapes  / 296 points   in 236.734µs
```

**Findings:**
1. **Output stays cubic Bézier.** `SimpleBezierPath` is structurally `(start, [(c1,c2,end)])` so every
   segment is a cubic; more importantly the *control points are real* — `max_ctrl_dev` (max distance of
   a control point from its chord) stayed at 25.464, identical to the input circle's, and
   `degenerate(line-like) = 0` for every operation. Nothing was flattened.
2. **Segment counts are minimal.** Union of two 5-segment circles = 9 segments (the original arcs, split
   at the two intersection points, with the interior arcs dropped). Compare i_overlay's 151 points.
3. **Geometry is correct.** Area error 0.004 %–0.015 %, and the rendered PNGs
   (`/tmp/boolops_{union,intersect,sub}.png`, produced with tiny-skia) show the expected two-lobe blob,
   crescent and lens shapes.
4. **`path_remove_interior_points` is the Remove Overlap primitive**, not
   `path_remove_overlapped_points` (which returned 2 disjoint circles, area 62836 = 2πr², i.e. it does
   something else entirely). On the 12-overlapping-circles ring it correctly returned 2 subpaths (outer
   contour + centre hole) with 36 cubic segments in 498 µs.
5. **Performance is fine.** ~2× slower than the flattening-based i_overlay while producing 8× fewer
   elements. For interactive per-glyph Remove Overlap this is a non-issue.

**Caveats / risks with `flo_curves`:**
* Only 274 k lifetime downloads, single maintainer (Logicalshift). Actively updated (0.8.1, 2026-08-25).
* API takes `&Vec<impl BezierPath>` (not `&[..]`) and needs a `Coord2`↔`kurbo::Point` shim; ours is ~40
  lines (`kurbo_to_flo` / `flo_to_kurbo` in `examples/boolops.rs`).
* Quadratic inputs must be degree-elevated to cubics first (TrueType glyphs) — `kurbo::QuadBez::raise()`.
* It does not preserve which contour a segment came from, so glyph "keep original point positions"
  behaviour (FontForge preserves on-curve points where possible) would need extra work.
* No fuzz/robustness guarantees on self-touching or degenerate contours; budget for hardening
  and for an `i_overlay` fallback path when `flo_curves` produces a topologically wrong result.

**Recommendation:** `flo_curves` as the primary curve-preserving engine (Remove Overlap, boolean
operations in the glyph editor), `i_overlay` kept as a second opinion / fast preview / validation
oracle (compare areas; if they diverge by more than a tolerance, flag the glyph).

