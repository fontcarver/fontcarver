# P0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Each task below is sized for one autonomous implementation run (Codex gpt-5.6-sol xhigh for Rust, Opus for UI). Tasks list exact interfaces and acceptance tests; implementers write the code within those interfaces and must follow `CONVENTIONS.md`.

**Goal:** A running FontWorkshop that opens any sfnt font (TTF/OTF/TTC/WOFF/WOFF2, including 65k-glyph CJK fonts) into the document model, shows it in a fast virtualised Font view with sidebar filters and search, shows a read-only glyph view and Font Info, saves/reloads the native `.fwf` format, and exposes the kernel through a Tauri transport, a dev HTTP/WebSocket server and a CLI.

**Architecture:** Rust workspace of small crates (`fw-model` → `fw-sfnt`/`fw-doc`/`fw-glyphdata` → `fw-api` → `fw-devserver`/`src-tauri`/`fw-cli`) and a React 19 + TypeScript app talking to the kernel through a single `api_call(method, params)` dispatch plus an event stream. The kernel owns documents and undo; the UI renders from query results.

**Tech Stack:** Rust 1.94 stable, read-fonts 0.43 / skrifa 0.46 / write-fonts 0.52, kurbo 0.13, serde/serde_json, smol_str, indexmap, uuid, ts-rs, axum + tokio (dev server), tauri 2.11 (+ plugin-dialog, plugin-fs), clap (CLI); pnpm, Vite 7, React 19, TypeScript 5.9, Zustand, i18next, Vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-09-03-fontworkshop-design.md` (sections 2–6, 9, 11, 12.1–12.2, 13, 14 row P0). Research: `docs/research/rust-crate-spike.md` (crate APIs), `docs/research/fontforge-data-model.md` (modelling lessons).

## Global Constraints

- Rust edition 2021, toolchain pinned in `rust-toolchain.toml` to `1.94.0`; `cargo clippy --all-targets -- -D warnings` clean; no `unwrap`/`expect`/`panic!` on font input.
- Crate names and directories exactly as in `CONVENTIONS.md` § Repository layout.
- All model coordinates `f64`, y-up font units; `kurbo::Point`/`Affine` for geometry; `GlyphName` newtype over `SmolStr`.
- Serialization with `serde`; unknown fields tolerated (`deny_unknown_fields` off); every optional field `#[serde(default)]`.
- Performance gates (Task 12): open Source Han Sans SC OTF (65,535 glyphs) < 3 s release; glyph-list page of 300 cells < 30 ms after warm-up; `.fwf` save+load of that font < 6 s.
- Frontend: TypeScript strict, no default exports, all strings through `t()`, CSS Modules + `tokens.css`, kernel access only via `app/src/transport/`.
- Commits: Conventional Commits; each task ends with its own commits.
- Large fixtures are never committed; `fixtures/README.md` lists external paths. Source Han Sans OTFs live at `/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/` (tests skip if absent). FontForge fixtures at `fontforge/tests/fonts/` (skip if absent).

---

## File structure (P0)

```
Cargo.toml                       workspace: members = crates/*, src-tauri; [workspace.dependencies] pins
rust-toolchain.toml              channel = "1.94.0"
.github/workflows/ci.yml         cargo fmt/clippy/test + pnpm check/lint/test/build
CONVENTIONS.md                   (exists)
fixtures/README.md               external fixture paths
fixtures/fonts/                  small committed fonts (< 2 MB each): see Task 1
crates/fw-fixtures/              tiny helper crate: fixture path resolution + skip helpers
crates/fw-model/                 src/lib.rs, ids.rs, font.rs, info.rs, master.rs, glyph.rs, layer.rs, contour.rs,
                                 component.rs, anchor.rs, guide.rs, kerning.rs, features.rs, bitmap.rs, extra.rs,
                                 query/{mod.rs, flatten.rs, bbox.rs, dependents.rs, lookup.rs}, validate.rs, error.rs
crates/fw-glyphdata/             src/lib.rs, aglfn.rs (generated table), unicode.rs, category.rs, names.rs
crates/fw-sfnt/                  src/lib.rs, error.rs, read/{mod.rs, container.rs, glyf.rs, cff.rs, metrics.rs,
                                 names.rs, cmap.rs, os2.rs, post.rs, vertical.rs, hinting.rs, variations.rs,
                                 bitmaps.rs, raw.rs}
crates/fw-doc/                   src/lib.rs, session.rs, document.rs, command.rs, undo.rs, cache.rs,
                                 fwf/{mod.rs, schema.md, write.rs, read.rs}, autosave.rs, error.rs
crates/fw-api/                   src/lib.rs, types/{mod.rs, session.rs, glyphs.rs, font.rs, commands.rs, events.rs},
                                 api.rs (facade + dispatch), error.rs, bin/gen_ts.rs
crates/fw-devserver/             src/main.rs, server.rs
crates/fw-cli/                   src/main.rs, cmd/{inspect.rs, convert.rs}
src-tauri/                       Cargo.toml, tauri.conf.json, capabilities/default.json, build.rs, src/{main.rs, lib.rs, commands.rs}, icons/
app/                             package.json, vite.config.ts, tsconfig*.json, index.html, e2e/, src/…
scripts/dev.sh                   starts fw-devserver + vite for browser development
```

---

### Task 1: Workspace scaffold, fixtures and CI

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/ci.yml`, `fixtures/README.md`, `fixtures/fonts/*`, `crates/fw-fixtures/{Cargo.toml,src/lib.rs}`, one `Cargo.toml` + `src/lib.rs` for each of `fw-model fw-glyphdata fw-sfnt fw-doc fw-api fw-devserver fw-cli` (empty crate with `//!` doc and an `Error` enum), `src-tauri/*` (Tauri 2 shell that compiles and opens a window showing the Vite app), `app/*` (Vite React TS scaffold with pnpm, ESLint, Prettier, Vitest), `scripts/dev.sh`, `package.json` (root, workspace scripts), `pnpm-workspace.yaml`.
- Modify: `.gitignore` (add `app/node_modules`, `src-tauri/target`, `src-tauri/gen`).

**Interfaces:**
- Produces: `fw_fixtures::path(name: &str) -> PathBuf` (resolves `fixtures/fonts/<name>` relative to workspace root), `fw_fixtures::external(key: &str) -> Option<PathBuf>` where keys `"source-han-sans-sc-otf"` → `/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/OTF/SimplifiedChinese/SourceHanSansSC-Regular.otf` (verify the real path when implementing; list what exists under that directory and pick the SC Regular static OTF), `"fontforge-tests"` → `<workspace>/fontforge/tests/fonts`; returns `None` when the file is missing. Macro `skip_unless_external!(key)` that `return`s from a test with an `eprintln!` when absent.
- Fixtures to commit (all freely licensed, each < 2 MB): download `DejaVuSans.ttf` (from the system `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf` if present, else fonts-dejavu release), `NotoSans-Regular.ttf` (Google Fonts GitHub `ofl/notosans` static), `NotoSansSC-Regular.otf` is too large — do **not** commit; instead commit `fontforge/tests/fonts/test1012.otf`, `CantarellMin.woff2` and `NotoSerifTibetan-Regular.ttf` copies (they are test fixtures in FontForge's repo, GPL/OFL; record source and license in `fixtures/README.md`). Also produce `fixtures/fonts/DejaVuSans.woff` by wrapping the TTF (write a tiny throwaway script; the woff is committed). If `DejaVuSans.ttf` exceeds 2 MB use `DejaVuSansMono.ttf` instead.
- Workspace `[workspace.dependencies]` pins: `read-fonts = { version = "0.43", features = ["experimental_traverse"] }`, `write-fonts = { version = "0.52", features = ["read"] }`, `skrifa = "0.46"`, `font-types = "0.12"`, `kurbo = { version = "0.13", features = ["serde"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = { version = "1", features = ["preserve_order"] }`, `smol_str = { version = "0.3", features = ["serde"] }`, `indexmap = { version = "2", features = ["serde"] }`, `uuid = { version = "1", features = ["v4"] }`, `thiserror = "2"`, `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `ts-rs = { version = "10", features = ["serde-compat", "no-serde-warnings"] }` (check the latest ts-rs major on crates.io and use it), `tokio = { version = "1", features = ["full"] }`, `axum = { version = "0.8", features = ["ws"] }`, `clap = { version = "4", features = ["derive"] }`, `anyhow = "1"`, `flate2 = "1"`, `woff2-patched = "0.4"`, `wuff = "0.2"`, `criterion = "0.5"`, `tauri = "2"`, `tauri-build = "2"`, `tauri-plugin-dialog = "2"`, `tauri-plugin-fs = "2"`. Verify each resolves; if a version is wrong, use the latest compatible and note it in the commit message.
- `app/package.json` scripts: `dev`, `build`, `check` (tsc -b), `lint` (eslint .), `format`, `test` (vitest run), `e2e` (playwright test), `gen:api` (`cargo run -p fw-api --bin gen_ts`), `tauri`. Dependencies: react, react-dom, zustand, i18next, react-i18next, @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/plugin-fs; dev: vite, @vitejs/plugin-react, typescript, vitest, jsdom, @testing-library/react, eslint + typescript-eslint + eslint-plugin-react-hooks, prettier, @playwright/test, @tauri-apps/cli.
- `src-tauri/tauri.conf.json`: identifier `dev.fontworkshop.app`, productName `FontWorkshop`, window 1400×900 min 1000×700, `devUrl` `http://localhost:1420`, `frontendDist` `../app/dist`, `beforeDevCommand` `pnpm --dir ../app dev`, `beforeBuildCommand` `pnpm --dir ../app build`. Capabilities: `core:default`, `dialog:default`, `fs:default`.

- [ ] **Step 1: Create the Cargo workspace and empty crates.** Each crate's `lib.rs` has a crate-level doc comment and `pub mod error;` with `#[derive(Debug, thiserror::Error)] pub enum Error {}` placeholder variants removed later — declare at least one variant `#[error("internal error: {0}")] Internal(String)` so the enum is non-empty.
- [ ] **Step 2: Create `crates/fw-fixtures`** with the functions above and a test that `path("DejaVuSans.ttf")` exists.
- [ ] **Step 3: Create the fixtures** as listed and `fixtures/README.md` (file, size, source URL, license, sha256).
- [ ] **Step 4: Scaffold `app/`** (`pnpm create vite app --template react-ts` equivalent, then add the scripts, ESLint flat config, Prettier config, Vitest config with jsdom, Playwright config pointing at `http://localhost:1420`, an `App.tsx` that renders the text "FontWorkshop" and a Vitest test asserting it renders).
- [ ] **Step 5: Scaffold `src-tauri/`** (Tauri 2 library crate `fontworkshop` with `lib.rs` `run()` registering dialog + fs plugins; `main.rs` calls `fontworkshop_lib::run()`; icons generated with `pnpm tauri icon` from a simple placeholder PNG you create with Python/PIL or copy from the Tauri template).
- [ ] **Step 6: Write `.github/workflows/ci.yml`** running on ubuntu-latest: install rust 1.94 + Tauri Linux deps (`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`), pnpm 10, then `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm lint`, `pnpm test`, `pnpm build`.
- [ ] **Step 7: `scripts/dev.sh`** runs `cargo run -p fw-devserver -- --port 8787` and `pnpm --dir app dev` concurrently (use a trap to kill both).
- [ ] **Step 8: Verify** `cargo check --workspace`, `cargo test --workspace`, `cd app && pnpm install && pnpm check && pnpm lint && pnpm test && pnpm build`, `cargo build -p fontworkshop` all succeed. Commit as `chore: scaffold workspace, app, tauri shell, fixtures, CI`.

---

### Task 2: `fw-model` — the document model

**Files:**
- Create: everything under `crates/fw-model/src/` listed in the file structure; tests in `crates/fw-model/tests/{invariants.rs, flatten.rs, serde_roundtrip.rs}`.

**Interfaces (produced; exact):**

```rust
// ids.rs
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)] pub struct GlyphName(pub SmolStr);          // Display, From<&str>, From<String>, Deref<Target=str>
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)] pub struct GlyphId(pub u32); // session handle, not persisted in .fwf
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)] #[serde(transparent)] pub struct MasterId(pub SmolStr);
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(transparent)] pub struct LayerId(pub SmolStr);
impl MasterId { pub fn new() -> Self /* uuid v4 simple */ } impl LayerId { pub fn new() -> Self }
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)] #[serde(transparent)] pub struct Tag(pub [u8; 4]); // Display "GSUB", FromStr

// font.rs
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Font {
    #[serde(default)] pub info: FontInfo,
    #[serde(default)] pub axes: Vec<Axis>,
    #[serde(default)] pub masters: Vec<Master>,
    #[serde(default)] pub instances: Vec<Instance>,
    #[serde(default)] pub glyphs: IndexMap<GlyphName, Glyph>,
    #[serde(default)] pub kerning_groups: KerningGroups,
    #[serde(default)] pub features: Features,
    #[serde(default)] pub bitmap_strikes: Vec<BitmapStrike>,
    #[serde(default)] pub cid: Option<CidInfo>,
    #[serde(default)] pub guides: Vec<Guide>,
    #[serde(default)] pub extra: Extra,
    #[serde(default)] pub user_data: serde_json::Value,
}
impl Font {
    pub fn new_empty(family: &str, units_per_em: u16) -> Font;      // one master "Regular", no glyphs
    pub fn default_master(&self) -> &Master;                        // masters[0]
    pub fn master(&self, id: &MasterId) -> Option<&Master>;
    pub fn glyph(&self, name: &str) -> Option<&Glyph>;
    pub fn glyph_mut(&mut self, name: &str) -> Option<&mut Glyph>;
    pub fn insert_glyph(&mut self, glyph: Glyph) -> Result<(), Error>;   // Err(DuplicateGlyph) if name exists; adds missing master layers
    pub fn remove_glyph(&mut self, name: &str) -> Option<Glyph>;
    pub fn rename_glyph(&mut self, from: &str, to: &str) -> Result<(), Error>; // updates components, kerning, groups
    pub fn add_master(&mut self, master: Master);                   // adds an empty master layer to every glyph
    pub fn glyph_order(&self) -> impl Iterator<Item = &GlyphName>;
}

pub struct Axis { pub tag: Tag, pub name: String, pub min: f64, pub default: f64, pub max: f64, pub hidden: bool, pub map: Vec<(f64, f64)> /* user→design */ }
pub type DesignLocation = BTreeMap<Tag, f64>;
pub struct Instance { pub name: String, pub location: DesignLocation, pub style_name: String, pub family_name: Option<String>, pub active: bool, pub is_variable_named_instance: bool, pub user_data: serde_json::Value }
pub struct CidInfo { pub registry: String, pub ordering: String, pub supplement: u32, pub cid_map: BTreeMap<u32, GlyphName> }

// info.rs — field names follow UFO 3 fontinfo.plist so norad mapping is 1:1
pub struct FontInfo {
    pub family_name: String, pub style_name: String, pub units_per_em: u16,
    pub version_major: u32, pub version_minor: u32,
    pub copyright: Option<String>, pub trademark: Option<String>, pub note: Option<String>,
    pub open_type_name_designer: Option<String>, pub open_type_name_designer_url: Option<String>,
    pub open_type_name_manufacturer: Option<String>, pub open_type_name_manufacturer_url: Option<String>,
    pub open_type_name_license: Option<String>, pub open_type_name_license_url: Option<String>,
    pub open_type_name_version: Option<String>, pub open_type_name_unique_id: Option<String>,
    pub open_type_name_description: Option<String>, pub open_type_name_preferred_family_name: Option<String>,
    pub open_type_name_preferred_subfamily_name: Option<String>, pub open_type_name_compatible_full_name: Option<String>,
    pub open_type_name_sample_text: Option<String>, pub open_type_name_wws_family_name: Option<String>,
    pub open_type_name_wws_subfamily_name: Option<String>, pub postscript_font_name: Option<String>,
    pub postscript_full_name: Option<String>,
    pub names: Vec<NameRecord>,                    // every name record verbatim (id, platform-agnostic BCP47 lang, text); the fields above are the "en" defaults
    pub open_type_os2_vendor_id: Option<String>, pub open_type_os2_weight_class: Option<u16>, pub open_type_os2_width_class: Option<u16>,
    pub open_type_os2_selection: Vec<u8>, pub open_type_os2_type: Vec<u8>, pub open_type_os2_panose: Option<[u8; 10]>,
    pub open_type_os2_family_class: Option<(u8, u8)>, pub open_type_os2_unicode_ranges: Vec<u8>, pub open_type_os2_code_page_ranges: Vec<u8>,
    pub open_type_os2_subscript_x_size: Option<f64>, pub open_type_os2_subscript_y_size: Option<f64>, pub open_type_os2_subscript_x_offset: Option<f64>, pub open_type_os2_subscript_y_offset: Option<f64>,
    pub open_type_os2_superscript_x_size: Option<f64>, pub open_type_os2_superscript_y_size: Option<f64>, pub open_type_os2_superscript_x_offset: Option<f64>, pub open_type_os2_superscript_y_offset: Option<f64>,
    pub open_type_os2_strikeout_size: Option<f64>, pub open_type_os2_strikeout_position: Option<f64>,
    pub open_type_head_created: Option<String>, pub open_type_head_flags: Vec<u8>, pub open_type_head_lowest_rec_ppem: Option<u16>,
    pub open_type_gasp_range_records: Vec<GaspRecord>, pub open_type_hhea_caret_slope_rise: Option<i32>, pub open_type_hhea_caret_slope_run: Option<i32>, pub open_type_hhea_caret_offset: Option<i32>,
    pub open_type_vhea_caret_slope_rise: Option<i32>, pub open_type_vhea_caret_slope_run: Option<i32>, pub open_type_vhea_caret_offset: Option<i32>,
    pub postscript_is_fixed_pitch: Option<bool>, pub postscript_default_width_x: Option<f64>, pub postscript_nominal_width_x: Option<f64>,
    pub postscript_underline_position: Option<f64>, pub postscript_underline_thickness: Option<f64>, pub postscript_unique_id: Option<i32>,
    pub macintosh_fond_name: Option<String>, pub macintosh_fond_family_id: Option<i32>,
    pub user_data: serde_json::Value,
}
pub struct NameRecord { pub name_id: u16, pub language: String /* "en", "zh-Hans", … */, pub text: String, pub platform: NamePlatform /* Windows | Mac | Unicode */ }
pub struct GaspRecord { pub range_max_ppem: u16, pub behavior: Vec<u8> }

// master.rs
pub struct Master {
    pub id: MasterId, pub name: String, pub location: DesignLocation,
    pub metrics: MasterMetrics, pub alignment_zones: Vec<Zone>, pub stems: Vec<Stem>, pub guides: Vec<Guide>,
    pub kerning_h: Kerning, pub kerning_v: Kerning,
    pub private_dict: PsPrivate, pub tt: TtMasterData, pub user_data: serde_json::Value,
}
pub struct MasterMetrics { pub ascender: f64, pub descender: f64, pub x_height: f64, pub cap_height: f64, pub italic_angle: f64,
    pub hhea_ascender: Option<f64>, pub hhea_descender: Option<f64>, pub hhea_line_gap: Option<f64>,
    pub typo_ascender: Option<f64>, pub typo_descender: Option<f64>, pub typo_line_gap: Option<f64>,
    pub win_ascent: Option<f64>, pub win_descent: Option<f64>,
    pub vhea_ascender: Option<f64>, pub vhea_descender: Option<f64>, pub vhea_line_gap: Option<f64>,
    pub default_vertical_origin: Option<f64>, pub default_vertical_advance: Option<f64> }
pub struct Zone { pub position: f64, pub size: f64 }           // negative size = bottom zone (Glyphs convention)
pub struct Stem { pub name: String, pub horizontal: bool, pub width: f64 }
pub struct PsPrivate { pub blue_values: Vec<f64>, pub other_blues: Vec<f64>, pub family_blues: Vec<f64>, pub family_other_blues: Vec<f64>,
    pub blue_scale: Option<f64>, pub blue_shift: Option<f64>, pub blue_fuzz: Option<f64>, pub std_hw: Option<f64>, pub std_vw: Option<f64>,
    pub stem_snap_h: Vec<f64>, pub stem_snap_v: Vec<f64>, pub force_bold: Option<bool>, pub language_group: Option<u8>, pub other: BTreeMap<String, serde_json::Value> }
pub struct TtMasterData { pub cvt: Vec<i16>, pub fpgm: Vec<u8>, pub prep: Vec<u8>, pub max_stack_elements: Option<u16>, pub max_storage: Option<u16>,
    pub max_function_defs: Option<u16>, pub max_instruction_defs: Option<u16>, pub max_twilight_points: Option<u16>, pub max_zones: Option<u16>, pub max_size_of_instructions: Option<u16> }

// glyph.rs / layer.rs / contour.rs / component.rs / anchor.rs / guide.rs
pub struct Glyph { pub name: GlyphName, pub unicodes: Vec<u32>, pub variation_selectors: Vec<(u32, u32)> /* (base cp, VS cp) */,
    pub production_name: Option<String>, pub category: Option<String>, pub sub_category: Option<String>, pub script: Option<String>,
    pub export: bool /* default true */, pub color_label: Option<u8>, pub tags: BTreeSet<String>, pub note: String,
    pub kern_group_side1: Option<String>, pub kern_group_side2: Option<String>, pub metrics_keys: MetricsKeys,
    pub layers: Vec<Layer>, pub locked: bool, pub user_data: serde_json::Value }
impl Glyph { pub fn new(name: impl Into<GlyphName>) -> Glyph; pub fn master_layer(&self, m: &MasterId) -> Option<&Layer>; pub fn master_layer_mut(&mut self, m: &MasterId) -> Option<&mut Layer>; pub fn layer(&self, id: &LayerId) -> Option<&Layer>; }
pub struct MetricsKeys { pub left: Option<String>, pub right: Option<String>, pub width: Option<String>, pub vertical_origin: Option<String>, pub vertical_width: Option<String> }
pub enum LayerKind { Master(MasterId), Brace { master: MasterId, location: DesignLocation }, Bracket { master: MasterId, axis: Tag, min: Option<f64>, max: Option<f64> }, Background(MasterId), Backup { of: MasterId, time: i64 }, Custom(String) }
pub struct Layer { pub id: LayerId, pub kind: LayerKind, pub name: String, pub width: f64, pub vertical_width: Option<f64>, pub vertical_origin: Option<f64>,
    pub contours: Vec<Contour>, pub components: Vec<Component>, pub anchors: Vec<Anchor>, pub guides: Vec<Guide>,
    pub background_image: Option<Image>, pub hints: PsHints, pub tt_instructions: Option<Vec<u8>>, pub color_layers: Option<Vec<ColorLayer>>, pub user_data: serde_json::Value }
impl Layer { pub fn new_master(master: MasterId) -> Layer; pub fn is_empty(&self) -> bool; }
pub struct Contour { pub closed: bool, pub points: Vec<Point>, pub name: Option<String>, pub id: Option<String> }
pub enum PointKind { Move, Line, Curve, QCurve, OffCurve }
pub struct Point { pub x: f64, pub y: f64, pub kind: PointKind, pub smooth: bool, pub name: Option<String>, pub tt_index: TtIndex }
pub enum TtIndex { Unnumbered, Implied, Index(u16) }             // default Unnumbered
impl Contour { pub fn is_quadratic(&self) -> bool /* any QCurve */; pub fn to_bezpath(&self) -> kurbo::BezPath /* handles implied on-curve points between consecutive quadratic off-curves; open contours start with Move */; pub fn from_bezpath(path: &kurbo::BezPath) -> Vec<Contour>; }
pub struct Component { pub base: GlyphName, pub transform: kurbo::Affine, pub anchor: Option<String>, pub auto_align: bool, pub use_my_metrics: bool, pub round_to_grid: bool, pub smart: Option<BTreeMap<String, f64>>, pub point_match: Option<(u16, u16)> }
pub struct Anchor { pub name: String, pub x: f64, pub y: f64 }
impl Anchor { pub fn kind(&self) -> AnchorKind /* by name: "_x"→Mark(x), "x"→Base(x), "x_N"→Ligature{x,N}, "entry"/"exit"→Cursive*, "caret_N"→Caret(N), "vcaret_N"→VCaret(N) */ }
pub struct Guide { pub x: f64, pub y: f64, pub angle: f64, pub name: Option<String>, pub locked: bool, pub color: Option<String> }
pub struct Image { pub file_name: String, pub transform: kurbo::Affine, pub color: Option<String>, pub data: Option<Vec<u8>> /* embedded PNG */ }
pub struct PsHints { pub h_stems: Vec<StemHint>, pub v_stems: Vec<StemHint>, pub d_stems: Vec<DiagonalHint>, pub flex: bool, pub hint_masks: Vec<HintMask> }
pub struct StemHint { pub start: f64, pub width: f64 /* -20/-21 ghost per Type1 */, pub ranges: Vec<(f64, f64)> }
pub struct DiagonalHint { pub left: (f64, f64), pub right: (f64, f64), pub unit: (f64, f64) }
pub struct HintMask { pub contour: usize, pub point: usize, pub mask: Vec<u8> }
pub struct ColorLayer { pub palette_index: Option<u16>, pub contours: Vec<Contour>, pub components: Vec<Component>, pub paint: Option<serde_json::Value> }

// kerning.rs / features.rs / bitmap.rs / extra.rs
pub enum KernSide { Glyph(GlyphName), Group(String) }
#[derive(Default)] pub struct Kerning(pub BTreeMap<(KernSide, KernSide), f64>);   // serde as list of {left,right,value}
#[derive(Default)] pub struct KerningGroups { pub side1: BTreeMap<String, BTreeSet<GlyphName>>, pub side2: BTreeMap<String, BTreeSet<GlyphName>> }
pub struct Features { pub prefixes: Vec<FeatureBlock>, pub classes: Vec<FeatureClass>, pub features: Vec<FeatureBlock>, pub source_of_truth: FeaturesSource /* Fea | ImportedTables */ }
pub struct FeatureBlock { pub tag: String, pub code: String, pub automatic: bool, pub disabled: bool, pub notes: String }
pub struct FeatureClass { pub name: String, pub code: String, pub automatic: bool }
pub struct BitmapStrike { pub ppem: u16, pub depth: u8, pub ascent: i16, pub descent: i16, pub properties: BTreeMap<String, BdfValue>, pub glyphs: BTreeMap<GlyphName, BitmapGlyph> }
pub enum BdfValue { Int(i64), Str(String) }
pub struct BitmapGlyph { pub bitmap: Bitmap, pub advance: i16, pub vertical_advance: Option<i16>, pub refs: Vec<BitmapRef> }
pub struct Bitmap { pub origin_x: i16, pub origin_y: i16, pub width: u16, pub height: u16, pub depth: u8, pub rows: Vec<u8> /* packed MSB-first, row-major, row stride = ceil(width*depth/8) */ }
pub struct BitmapRef { pub base: GlyphName, pub dx: i16, pub dy: i16 }
#[derive(Default)] pub struct Extra { pub raw_tables: BTreeMap<Tag, Vec<u8>> /* serde base64 */, pub source: Option<SourceProvenance>, pub lib: BTreeMap<String, serde_json::Value> }
pub struct SourceProvenance { pub path: Option<String>, pub format: String, pub sha256: String, pub table_tags: Vec<Tag>, #[serde(skip)] pub original_bytes: Option<std::sync::Arc<Vec<u8>>> }

// query/
pub fn flatten_layer(font: &Font, glyph: &str, master: &MasterId) -> Result<kurbo::BezPath, Error>;   // contours + components recursively (Err(ComponentCycle) on cycles, max depth 64)
pub fn layer_bbox(font: &Font, glyph: &str, master: &MasterId) -> Result<Option<kurbo::Rect>, Error>;
pub fn dependents(font: &Font) -> HashMap<GlyphName, Vec<GlyphName>>;                                  // base → glyphs using it as a component
pub fn glyph_for_unicode(font: &Font, cp: u32) -> Option<&Glyph>;
pub fn masters_compatible(font: &Font, glyph: &str) -> Result<(), Vec<Incompatibility>>;               // contour count/point count/kind sequence/component list
pub fn validate(font: &Font) -> Vec<Issue>;                                                            // duplicate unicodes, dangling component bases, missing master layers, kern group references
```

- [ ] **Step 1: Write failing tests** in `tests/invariants.rs`: `insert_glyph` rejects duplicates; `insert_glyph` adds a layer for every master; `rename_glyph` updates component bases and kerning pairs and group members; `add_master` adds layers to all glyphs; `Contour::to_bezpath` on a quadratic contour `[(0,0,Line),(50,100,OffCurve),(150,100,OffCurve),(200,0,Line)]` produces two quads with an implied on-curve at (100,100); cubic contour round-trips through `from_bezpath`; `flatten_layer` applies component transforms and detects a cycle A→B→A; `validate` reports a dangling component; serde round-trip of a `Font` with every field populated is equal (`PartialEq` derive on all model types).
- [ ] **Step 2: Implement** all files. Keep each file focused (see file structure). `Kerning` serialises as `Vec<KernPair { left, right, value }>`; `Extra.raw_tables` values serialise as base64 strings (`serde_with` or a small custom module).
- [ ] **Step 3: Run** `cargo test -p fw-model`, `cargo clippy -p fw-model --all-targets -- -D warnings`; fix until green.
- [ ] **Step 4: Commit** `feat(fw-model): document model with invariants, queries and serde`.

---

### Task 3: `fw-glyphdata` — Unicode, names and categories

**Files:** `crates/fw-glyphdata/src/{lib.rs, aglfn.rs, unicode.rs, category.rs, names.rs}`, `crates/fw-glyphdata/build.rs` (generates `aglfn.rs` from a vendored `aglfn.txt` in `crates/fw-glyphdata/data/`; vendor the file from https://github.com/adobe-type-tools/agl-aglfn, BSD licence noted), tests in `crates/fw-glyphdata/tests/names.rs`.

**Interfaces:**
```rust
pub fn unicode_for_name(name: &str) -> Option<u32>;          // AGLFN, "uniXXXX", "uXXXX(X)(X)", ligature/suffix stripped ("f_i" → None, "a.sc" → 'a')
pub fn name_for_unicode(cp: u32) -> String;                  // AGLFN name else "uniXXXX" (BMP) / "uXXXXX"
pub fn block_name(cp: u32) -> &'static str;                  // Unicode block name via icu_properties (or a generated table)
pub fn script(cp: u32) -> &'static str;                      // ISO 15924 4-letter, "Zyyy" for common
pub enum Category { Letter, Number, Punctuation, Symbol, Separator, Mark, Format, Private, Other }
pub fn category(cp: u32) -> Category;                        // from General_Category
pub fn sub_category(cp: u32) -> &'static str;                // "Uppercase", "Lowercase", "Decimal", "Currency", "Spacing", "Nonspacing", "Ideograph", "Kana", "Hangul", … (GlyphData-like buckets)
pub fn is_cjk_ideograph(cp: u32) -> bool;
pub fn cjk_charset_membership(cp: u32) -> CjkCharsets;       // bitflags GB2312 | GB18030_LEVEL1 | BIG5 | JIS_X_0208 | KS_X_1001 (P0: GB2312 and Big5 from vendored tables; others may be empty with a TODO comment referencing the table source)
```
- [ ] **Step 1: Tests**: `unicode_for_name("A") == Some(0x41)`, `"uni4E00" → 0x4E00`, `"u1F600" → 0x1F600`, `"a.sc" → 0x61`, `"f_i" → None`; `name_for_unicode(0x4E00) == "uni4E00"`, `(0x41) == "A"`, `(0x1F600) == "u1F600"`; `category(0x41) == Letter`, `sub_category(0x41) == "Uppercase"`, `block_name(0x4E00) == "CJK Unified Ideographs"`, `script(0x4E00) == "Hani"`, `is_cjk_ideograph(0x4E00)`.
- [ ] **Step 2: Implement** with `icu_properties` 2.x (`props::GeneralCategory`, `props::Script`, `props::Block` if available; otherwise vendor `Blocks.txt` and generate). Use `icu_properties = "2"` in workspace deps.
- [ ] **Step 3:** `cargo test -p fw-glyphdata`, clippy clean, commit `feat(fw-glyphdata): AGLFN names, Unicode properties, categories`.

---

### Task 4: `fw-sfnt` — read sfnt containers into the model

**Files:** `crates/fw-sfnt/src/{lib.rs, error.rs, read/mod.rs, read/container.rs, read/glyf.rs, read/cff.rs, read/metrics.rs, read/names.rs, read/cmap.rs, read/os2.rs, read/post.rs, read/vertical.rs, read/hinting.rs, read/variations.rs, read/bitmaps.rs, read/raw.rs}`, tests `crates/fw-sfnt/tests/{read_ttf.rs, read_otf.rs, read_containers.rs, read_cjk.rs}`, bench `crates/fw-sfnt/benches/open.rs` (criterion, optional).

**Interfaces:**
```rust
pub struct ReadOptions { pub font_index: Option<u32> /* TTC face; None = all faces */, pub keep_original_bytes: bool /* default true */, pub decode_bitmaps: bool /* P0: false → strikes left raw */ }
pub struct ReadOutcome { pub fonts: Vec<fw_model::Font>, pub warnings: Vec<Warning> }
pub struct Warning { pub code: &'static str, pub message: String }
pub fn sniff(bytes: &[u8]) -> Option<Container>;             // enum Container { Sfnt, Ttc, Woff, Woff2 } by magic (00010000/OTTO/true/ttcf/wOFF/wOF2)
pub fn read_bytes(bytes: &[u8], opts: &ReadOptions) -> Result<ReadOutcome, Error>;   // WOFF/WOFF2 are decompressed to sfnt bytes first (wuff or woff2-patched), then parsed
pub fn read_path(path: &Path, opts: &ReadOptions) -> Result<ReadOutcome, Error>;
pub fn face_count(bytes: &[u8]) -> Result<u32, Error>;
pub fn face_names(bytes: &[u8]) -> Result<Vec<String>, Error>;   // full name (id 4) per face, for TTC pickers
```
Behaviour:
- **Model mapping**: one `Master` (`name = subfamily`, `location` = default location if `fvar` present, else empty); `FontInfo` from `name` (all records → `names` with BCP47 language derived from platform/language IDs; Windows/Unicode preferred for the convenience fields), `head`, `hhea`, `OS/2`, `post`, `gasp`, `vhea`; `MasterMetrics` from `OS/2` (`sxHeight`, `sCapHeight`, typo/win) + `hhea` + `vhea` + `post.italicAngle`; `PsPrivate` from CFF Private DICT (default FD for CID); `TtMasterData` from `cvt `/`fpgm`/`prep`/`maxp`.
- **Glyphs**: names from `post` (format 2.0) or CFF charset, else `.notdef`, `glyphNNNNN` synthesised (warn); `unicodes` from every cmap subtable merged (prefer 3/10 then 3/1 then 0/x; keep 3/0 symbol codes mapped with the `0xF000` PUA stripped as an extra unicode); format-14 variation selectors → `variation_selectors`; CID fonts populate `Font::cid` with the charset mapping and glyph names `cidNNNNN`.
- **Outlines**: `glyf` → quadratic contours: on-curve points `QCurve`/`Line` (a point is `Line` when both neighbours are on-curve), off-curve `OffCurve`, `tt_index = Index(n)` for real points; do **not** materialise implied points (the model keeps consecutive off-curves; `Contour::to_bezpath` handles them). Composite glyphs → `Component`s with `use_my_metrics`, `round_to_grid`, `point_match` (ARGS_ARE_XY_VALUES clear), scale/2x2 transforms mapped into `Affine`. Per-glyph instructions → `Layer::tt_instructions` (only when non-empty). CFF/CFF2 → cubic contours via skrifa's `OutlinePen` (`Curve`/`Line`/`OffCurve` points), hint data (hstem/vstem/hintmask) → `PsHints` via read-fonts' charstring hinting callbacks if exposed, else leave empty with a warning `code = "cff-hints-not-imported"`.
- **Metrics**: `hmtx` advances → `Layer.width`; `vmtx`/`VORG` → `vertical_width`/`vertical_origin` when present.
- **Variations**: `fvar` axes → `Font.axes` (with `avar` maps), named instances → `Instance { is_variable_named_instance: true }`; `gvar`/`HVAR`/`MVAR`/`cvar`/`STAT` kept raw in `extra.raw_tables` (P4 materialises masters).
- **Raw preservation**: every table not consumed (and also `GSUB GPOS GDEF BASE MATH JSTF STAT COLR CPAL SVG EBDT EBLC CBDT CBLC sbix EBSC hdmx VDMX LTSH PCLT meta DSIG kern VORG` in P0) is stored in `extra.raw_tables` **except** `DSIG` (dropped). `extra.source` records path, format (`"ttf"|"otf"|"ttc"|"woff"|"woff2"`), sha256, table tags, and `original_bytes` when `keep_original_bytes`.
- Malformed fonts return `Err(Error::Parse { table, reason })`, never panic; fuzz-ish test feeds truncated copies of every fixture at 10 cut points and asserts no panic.

- [ ] **Step 1: Tests** (write first, using `fw_fixtures`):
  - `read_ttf.rs`: DejaVuSans.ttf → glyph count equals `maxp.numGlyphs`; glyph `A` has unicode 0x41, width equals `hmtx` advance; `A`'s contour points all have `tt_index = Index(_)`; composite `Aacute` (if composite in the fixture; else `agrave` in NotoSans) has two components with translation transforms; `info.family_name == "DejaVu Sans"`; `raw_tables` contains `GSUB` and `GPOS` and not `glyf`/`hmtx`/`DSIG`; `masters[0].tt.prep` non-empty; `extra.source.sha256` is 64 hex chars.
  - `read_otf.rs`: `test1012.otf` → contours are cubic (no `QCurve`), names come from CFF charset; a CID OTF (Source Han Sans, external) → `cid.is_some()`, registry `Adobe`, ordering `Identity`.
  - `read_containers.rs`: `DejaVuSans.woff` and `CantarellMin.woff2` → same glyph count as their TTF (for woff compare to DejaVuSans.ttf); TTC: build a two-face TTC fixture at test time is not possible in P0, so test `face_count` on any `.ttc` found in `/usr/share/fonts` (skip if none) and assert `read_bytes` with `font_index: None` returns that many fonts.
  - `read_cjk.rs` (skips without external): Source Han Sans SC OTF opens in < 3 s in release (`cargo test --release`; the test measures with `Instant` and asserts `< 3s` only when `cfg!(not(debug_assertions))`), glyph count 65535, `glyph_for_unicode(0x4E00)` is some, `cid` present.
  - Robustness: truncation test described above over all fixture files.
- [ ] **Step 2: Implement** using read-fonts (`FontRef::from_index`, `TableProvider`, `table_directory` for tags, `table_data(tag)` for raw bytes), skrifa for outlines/metrics (`OutlineGlyphCollection`, `DrawSettings::unhinted`, and a custom `OutlinePen` collecting points). For `glyf` real point indices, read `glyf` directly with read-fonts `Glyf`/`SimpleGlyph::points()` rather than skrifa, so `tt_index` and on/off flags are exact. Names: map Windows LCID and Mac language IDs to BCP47 (vendor a small table).
- [ ] **Step 3:** `cargo test -p fw-sfnt` (debug) and `cargo test -p fw-sfnt --release` (for the timing test), clippy clean, commit `feat(fw-sfnt): read TTF/OTF/TTC/WOFF/WOFF2 into the document model`.

---

### Task 5: `fw-doc` — session, documents, commands, undo, `.fwf`

**Files:** `crates/fw-doc/src/{lib.rs, error.rs, session.rs, document.rs, command.rs, undo.rs, cache.rs, autosave.rs, fwf/mod.rs, fwf/schema.md, fwf/write.rs, fwf/read.rs}`, tests `crates/fw-doc/tests/{commands_undo.rs, fwf_roundtrip.rs, session.rs}`.

**Interfaces:**
```rust
pub struct Session { .. }                                    // Send + Sync; wraps documents in RwLock; single instance per process
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)] pub struct DocId(pub u32);
impl Session {
    pub fn new() -> Session;
    pub fn open_path(&self, path: &Path, face_index: Option<u32>) -> Result<DocId, Error>;   // dispatches: .fwf → fwf::read; sfnt/woff → fw_sfnt::read_path; (later: other formats)
    pub fn open_font(&self, font: Font, provenance: Option<PathBuf>) -> DocId;
    pub fn new_document(&self, family: &str, units_per_em: u16) -> DocId;
    pub fn close(&self, id: DocId) -> Result<(), Error>;
    pub fn ids(&self) -> Vec<DocId>;
    pub fn with_doc<R>(&self, id: DocId, f: impl FnOnce(&Document) -> R) -> Result<R, Error>;
    pub fn with_doc_mut<R>(&self, id: DocId, f: impl FnOnce(&mut Document) -> R) -> Result<R, Error>;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DocumentEvent>;
}
pub struct Document { pub font: Font, pub path: Option<PathBuf>, pub dirty: bool, pub generation: u64, .. }
impl Document {
    pub fn glyph_id(&self, name: &str) -> Option<GlyphId>;   pub fn glyph_name(&self, id: GlyphId) -> Option<&GlyphName>;   // stable handles allocated on open/insert
    pub fn glyph_generation(&self, id: GlyphId) -> u64;
    pub fn apply(&mut self, cmd: Command, label: impl Into<String>) -> Result<Change, Error>;   // pushes an undo entry, bumps generations, emits event
    pub fn begin_edit(&mut self, label: impl Into<String>) -> EditToken;            // coalescing: subsequent apply_in_edit calls merge into one undo entry
    pub fn apply_in_edit(&mut self, token: &EditToken, cmd: Command) -> Result<Change, Error>;
    pub fn end_edit(&mut self, token: EditToken);
    pub fn undo(&mut self) -> Result<Option<Change>, Error>;  pub fn redo(&mut self) -> Result<Option<Change>, Error>;
    pub fn undo_label(&self) -> Option<&str>;  pub fn redo_label(&self) -> Option<&str>;
    pub fn flattened_path(&self, id: GlyphId, master: &MasterId) -> Result<std::sync::Arc<kurbo::BezPath>, Error>;   // cached by (id, glyph_generation, dependents' generations)
    pub fn save(&mut self, path: &Path) -> Result<(), Error>;   // .fwf; sets path, dirty=false
}
pub enum Command {
    AddGlyph { glyph: Glyph }, RemoveGlyphs { names: Vec<GlyphName> }, RenameGlyph { from: GlyphName, to: GlyphName },
    SetUnicodes { glyph: GlyphName, unicodes: Vec<u32> }, SetGlyphMeta { glyph: GlyphName, export: Option<bool>, color_label: Option<Option<u8>>, tags: Option<BTreeSet<String>>, note: Option<String> },
    SetWidth { glyph: GlyphName, layer: LayerId, width: f64 },
    ReplaceLayerOutline { glyph: GlyphName, layer: LayerId, contours: Vec<Contour>, components: Vec<Component>, anchors: Vec<Anchor> },
    SetFontInfo { info: Box<FontInfo> }, SetMasterMetrics { master: MasterId, metrics: MasterMetrics },
    Batch(Vec<Command>),
}
pub struct Change { pub glyphs: Vec<GlyphId>, pub font_info: bool, pub masters: bool, pub glyph_order: bool }
pub struct DocumentEvent { pub doc: DocId, pub generation: u64, pub change: Change, pub undo_label: Option<String>, pub redo_label: Option<String>, pub dirty: bool }
```
`.fwf` format (`fwf/schema.md` documents it): `{"format":"fontworkshop","version":1,"font":<Font serde>}` written with `serde_json` pretty printer, 2-space indent, `preserve_order` so glyph order is stable, `f64` written without trailing `.0` noise (use `ryu`/default serde is fine). Reading accepts `version` ≤ 1 and unknown fields. A `.fwf.gz` path is gzip-wrapped.

Undo: `UndoStack { entries: Vec<UndoEntry>, cursor: usize, memory_budget: usize (default 512 MiB, oldest dropped) }`; `UndoEntry { label, inverse: Vec<Snapshot>, forward: Vec<Snapshot> }` where `Snapshot` is `Glyph(GlyphName, Option<Arc<Glyph>>)` (None = did not exist) or `FontInfo(Arc<FontInfo>)`, `Master(MasterId, Arc<Master>)`, `GlyphOrder(Vec<GlyphName>)`. Applying a command captures before/after snapshots for touched items only.

- [ ] **Step 1: Tests**: apply `SetWidth` then `undo` restores width and `dirty` toggles appropriately; `begin_edit`/`apply_in_edit` ×10/`end_edit` produces exactly one undo entry; `RenameGlyph` undo restores component references; `RemoveGlyphs` undo restores glyph order position; `flattened_path` cache invalidates when a component base changes (generation bump of dependents); `.fwf` round trip on DejaVuSans (open via `fw_sfnt`, save, read back → `font == original.font` with `original_bytes` ignored); `Session::open_path` on `.fwf` and `.ttf`; events are emitted on apply/undo with correct `change.glyphs`.
- [ ] **Step 2: Implement.** Keep `command.rs` (definitions + application to `Font`), `undo.rs` (stack), `document.rs` (glue) separate. Autosave: `autosave::spawn(session, interval)` writes `<path>.autosave.fwf` (or `~/.local/share/fontworkshop/autosave/<docid>.fwf` when unsaved) for dirty documents every `interval` (default 60 s); `autosave::list_recoverable()`.
- [ ] **Step 3:** `cargo test -p fw-doc`, clippy clean, commit `feat(fw-doc): session, commands, coalescing undo, .fwf format, autosave`.

---

### Task 6: `fw-api` — typed API, dispatch and TypeScript generation

**Files:** `crates/fw-api/src/{lib.rs, error.rs, api.rs, types/mod.rs, types/session.rs, types/glyphs.rs, types/font.rs, types/commands.rs, types/events.rs}`, `crates/fw-api/src/bin/gen_ts.rs`, tests `crates/fw-api/tests/{dispatch.rs, glyph_list.rs}`; generated output dir `app/src/api/generated/` (gitignored, regenerated by `pnpm gen:api`; commit a `.gitkeep` and document the command in `app/README.md`).

**Interfaces:**
```rust
pub struct Api { session: Arc<Session> }
impl Api {
    pub fn new(session: Arc<Session>) -> Api;
    pub fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, ApiError>;   // single dispatch used by all transports
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentEvent>;
}
#[derive(Serialize, TS)] pub struct ApiError { pub code: String /* "not_found" | "invalid_params" | "io" | "parse" | "internal" */, pub message: String }
```
Methods (`method` string → params type → result type; all types `#[derive(Serialize, Deserialize, TS)] #[ts(export)]`):
| method | params | result |
|---|---|---|
| `session.open` | `OpenParams { path: String, face_index: Option<u32> }` | `DocSummary` |
| `session.faces` | `{ path }` | `Vec<String>` |
| `session.new` | `{ family: String, units_per_em: u16 }` | `DocSummary` |
| `session.close` | `{ doc: DocId }` | `null` |
| `session.list` | `{}` | `Vec<DocSummary>` |
| `session.save` | `{ doc, path: Option<String> }` | `DocSummary` |
| `session.recent` | `{}` | `Vec<String>` (persisted in `dirs::config_dir()/fontworkshop/recent.json`, max 20) |
| `doc.summary` | `{ doc }` | `DocSummary { doc, path, dirty, family_name, style_name, units_per_em, glyph_count, masters: Vec<MasterSummary{id,name}>, axes: Vec<AxisSummary>, undo_label, redo_label, format: String, warnings: Vec<String> }` |
| `doc.font_info` | `{ doc }` | `FontInfoView { info: FontInfo, masters: Vec<Master> }` (model types re-exported with TS derive via `#[ts]` on model types — add `ts-rs` derives to `fw-model` types behind a `ts` feature) |
| `doc.sidebar` | `{ doc }` | `SidebarCounts { categories: Vec<(String,u32)>, scripts: Vec<(String,u32)>, blocks: Vec<(String,u32)>, unencoded: u32, total: u32, color_labels: Vec<(u8,u32)> }` |
| `glyphs.list` | `GlyphListParams { doc, master: Option<MasterId>, filter: GlyphFilter, sort: GlyphSort /* Order | Unicode | Name | Width */, offset: u32, limit: u32 (≤ 2000), include_paths: bool }` | `GlyphListResult { total: u32, items: Vec<GlyphCell> }` |
| `glyphs.cell` | `{ doc, gid, master }` | `GlyphCell { gid: GlyphId, name: String, unicodes: Vec<u32>, width: f64, bbox: Option<[f64;4]>, path: Option<String> /* SVG path d in font units, y-up, components flattened */, color_label: Option<u8>, export: bool, has_components: bool, is_empty: bool, generation: u64 }` |
| `glyphs.detail` | `{ doc, gid }` | `GlyphDetail { glyph: Glyph, paths: Vec<LayerPath { layer: LayerId, flattened: String }> }` |
| `glyphs.search` | `{ doc, query: String, limit: u32 }` | `Vec<GlyphCell>` (name prefix/substring, `U+XXXX`, single character, `uniXXXX`) |
| `doc.apply` | `{ doc, command: Command, label: String }` | `Change` |
| `doc.edit_begin` / `doc.edit_apply` / `doc.edit_end` | `{ doc, label }` / `{ doc, token, command }` / `{ doc, token }` | `EditTokenId` / `Change` / `null` |
| `doc.undo` / `doc.redo` | `{ doc }` | `Option<Change>` |
| `prefs.get` / `prefs.set` | `{}` / `{ prefs: Prefs }` | `Prefs { theme: String, language: String, recent_limit: u32, autosave_seconds: u32 }` persisted as JSON in the config dir |
`GlyphFilter { query: Option<String>, categories: Vec<String>, scripts: Vec<String>, blocks: Vec<String>, unencoded_only: bool, color_labels: Vec<u8>, tags: Vec<String>, unicode_ranges: Vec<[u32;2]>, has_components: Option<bool>, changed_only: bool }` — all conditions AND-ed; `query` matches name substring, `U+XXXX`, or a literal character. Categories/scripts/blocks come from `fw-glyphdata` on the glyph's first unicode (unencoded glyphs count as `"Unencoded"`).

`gen_ts` binary: runs `ts-rs` export for every `#[ts(export)]` type into `app/src/api/generated/` and writes `index.ts` re-exporting all, plus `methods.ts` containing a typed `ApiMethods` interface mapping method names to `{ params, result }` (generate from a static list in Rust so TS and Rust stay in sync).

- [ ] **Step 1: Tests**: `dispatch.rs` — `call("session.open", {path: DejaVuSans})` returns a summary with correct glyph count; `call("nope", {})` → `ApiError{code:"not_found"}`; invalid params → `invalid_params`. `glyph_list.rs` — filter by `categories:["Letter"]` on DejaVu returns only letters; `sort: Unicode` is ascending; `include_paths` produces a non-empty `path` for `A` that starts with `M`; paging `offset/limit` is consistent with `total`; `glyphs.search` for `"U+0041"` returns `A`.
- [ ] **Step 2: Implement.** Paths: build from `Document::flattened_path` with `kurbo::BezPath::to_svg()`. Sidebar counts computed once per document generation and cached.
- [ ] **Step 3:** `cargo test -p fw-api`, `cargo run -p fw-api --bin gen_ts` produces `.ts` files that `tsc` accepts (add a `pnpm check` run to the test instructions), commit `feat(fw-api): typed API dispatch, glyph listing/filtering, TS type generation`.

---

### Task 7: `fw-devserver` and `src-tauri` wiring

**Files:** `crates/fw-devserver/src/{main.rs, server.rs}`, `src-tauri/src/{lib.rs, commands.rs}`, `src-tauri/capabilities/default.json`, `app/src/transport/{Transport.ts, TauriTransport.ts, HttpTransport.ts, index.ts}` (the TS side of this task is small and mechanical, so it stays here), test `crates/fw-devserver/tests/http.rs`.

**Interfaces:**
- Dev server: `POST /api/<method>` with JSON body → `200 {result}` or `400/404/500 {error: ApiError}`; `GET /events` WebSocket streaming `DocumentEvent` JSON; `GET /health` → `{"ok":true}`. CLI flags `--port` (default 8787), `--open <path>` (opens a font at start for e2e), CORS allow `http://localhost:1420`.
- Tauri: single command `#[tauri::command] fn api_call(method: String, params: serde_json::Value, state: State<Arc<Api>>) -> Result<serde_json::Value, ApiError>`; events forwarded with `app.emit("fw:event", event)` from a task subscribed to `Api::subscribe`. Native file dialogs are used from the frontend via plugin-dialog (the kernel receives paths).
- TS: `interface Transport { call<M extends keyof ApiMethods>(method: M, params: ApiMethods[M]['params']): Promise<ApiMethods[M]['result']>; onEvent(cb: (e: DocumentEvent) => void): () => void; kind: 'tauri' | 'http' }`; `createTransport()` picks Tauri when `window.__TAURI_INTERNALS__` exists, else HTTP with base URL from `import.meta.env.VITE_FW_API ?? 'http://localhost:8787'`.

- [ ] **Step 1: Tests**: `http.rs` spins the server on a random port, `POST /api/session.open` with DejaVu, asserts glyph count; `POST /api/nope` → 404 with `code:"not_found"`; WebSocket receives an event after `doc.apply SetWidth`.
- [ ] **Step 2: Implement** both transports; `pnpm check` passes with the generated types.
- [ ] **Step 3:** Commit `feat(transport): dev server, tauri api_call command, TS transport layer`.

---

### Task 8: `fw-cli`

**Files:** `crates/fw-cli/src/{main.rs, cmd/inspect.rs, cmd/convert.rs}`, tests `crates/fw-cli/tests/cli.rs` (use `assert_cmd`).

**Interfaces:** binary name `fw`.
- `fw inspect <font> [--face N] [--json]`: prints family/style/upem/glyph count/masters/axes/tables (tag + length + preserved?) and warnings; `--json` emits the `DocSummary` plus table list.
- `fw convert <in> <out.fwf|out.fwf.gz>`: opens any supported input, writes `.fwf`. (Export to binary formats arrives with `fw-compile` in P1; `fw convert x.fwf y.ttf` must fail with a clear "not yet supported" message.)
- `fw glyphs <font> [--filter cat=Letter] [--limit N]`: lists glyph name, unicodes, width.

- [ ] **Step 1: Tests**: `fw inspect DejaVuSans.ttf` exit 0 and stdout contains `DejaVu Sans`; `fw convert DejaVuSans.ttf out.fwf` then `fw inspect out.fwf --json` reports the same glyph count; unsupported output extension exits 2 with the message.
- [ ] **Step 2: Implement** with clap derive; reuse `fw_doc::Session`.
- [ ] **Step 3:** Commit `feat(fw-cli): inspect, convert, glyphs commands`.

---

### Task 9: App shell (UI) — layout, theme, i18n, stores, command palette

**Files:** `app/src/{main.tsx, App.tsx, styles/tokens.css, styles/global.css, i18n/{index.ts,en.json,zh-CN.json}, stores/{uiStore.ts, sessionStore.ts, documentStore.ts}, api/{client.ts, index.ts}, shell/{AppShell.tsx, MenuBar.tsx, TabBar.tsx, Sidebar.tsx, Inspector.tsx, StatusBar.tsx, CommandPalette.tsx, commands.ts, shortcuts.ts}, components/{Button.tsx, IconButton.tsx, Panel.tsx, SplitPane.tsx, Toolbar.tsx, Field.tsx, Select.tsx, Slider.tsx}}` with CSS modules, tests `app/src/**/*.test.tsx`.

**Interfaces:**
- `sessionStore`: `docs: DocSummary[]`, `activeDoc: DocId | null`, actions `openDialog()` (Tauri dialog or, in HTTP mode, a modal prompting for a path — also used by e2e), `open(path, faceIndex?)`, `close(doc)`, `save(doc, asPath?)`, `refreshSummary(doc)`; subscribes to transport events and updates `dirty`/undo labels.
- `documentStore` (per active doc): `sidebar: SidebarCounts | null`, `selection: Set<GlyphId>`, `filter: GlyphFilter`, `sort`, `master: MasterId`, `cellSize` (24–256, default 64), `labelMode: 'name'|'unicode'|'index'`.
- `uiStore`: `theme: 'light'|'dark'|'system'`, `language`, `panels: { sidebar: boolean; inspector: boolean }`, `tabs: Tab[]` where `Tab = { id, kind: 'font' | 'glyph', doc, gid? }`, `activeTab`.
- `commands.ts`: registry `registerCommand({ id, title: string /* i18n key */, shortcut?: string, when?: () => boolean, run: () => void })`; built-ins: `file.new`, `file.open`, `file.openRecent`, `file.save`, `file.saveAs`, `file.close`, `edit.undo`, `edit.redo`, `view.toggleSidebar`, `view.toggleInspector`, `view.zoomIn/Out` (cell size), `view.theme.*`, `glyph.open` (opens Glyph tab for selection), `palette.open`. `shortcuts.ts` binds them (Ctrl/Cmd aware; `Mod+O`, `Mod+S`, `Mod+Shift+S`, `Mod+W`, `Mod+Z`, `Mod+Shift+Z`, `Mod+Shift+P`, `Mod+1`/`Mod+2` sidebar/inspector, `Mod+=`/`Mod+-`).
- Layout (CSS grid): menu bar (in-app, rendered from the command registry: File / Edit / Glyph / View / Window / Help), tab bar, left sidebar (resizable 200–400 px), centre content, right inspector (resizable 240–420 px), status bar (doc name, glyph count, selection count, dirty marker, zoom).
- Design tokens: spacing scale 4/8/12/16/24, radius 4/6, font stack system UI, accent colour, surface/border/text tokens for light and dark, `data-theme` on `<html>`; follows `prefers-color-scheme` when `system`.
- i18n: i18next with `en` complete; `zh-CN.json` is created with the same keys but **left for the translation task** (Task 13) — until then it may contain English values.

- [ ] **Step 1: Tests**: AppShell renders menu bar, empty-state ("Open a font…") when no docs; command palette opens with `Mod+Shift+P`, filters by title, runs a command on Enter; theme toggle sets `data-theme`; `sessionStore.open` calls the transport and adds a tab (mock transport).
- [ ] **Step 2: Implement.** Use `agent-browser` against `scripts/dev.sh` to verify visually (light and dark); save screenshots to `docs/screenshots/p0-shell-{light,dark}.png`.
- [ ] **Step 3:** `pnpm check && pnpm lint && pnpm test`, commit `feat(app): application shell, theme, i18n, command palette, stores`.

---

### Task 10: Font view (virtualised glyph grid, sidebar filters, search)

**Files:** `app/src/views/font/{FontView.tsx, GlyphGrid.tsx, GlyphCell.tsx, useVirtualGrid.ts, cellRenderer.ts, GridToolbar.tsx, SidebarFilters.tsx, GlyphSearch.tsx}`, `app/src/canvas/pathCache.ts`, `app/src/canvas/glyphPaint.ts`, tests `app/src/views/font/*.test.tsx`, e2e `app/e2e/fontview.spec.ts`.

**Interfaces:**
- `useVirtualGrid({ total, cellSize, gap, containerRef }) → { rows, cols, range: [first, last], scrollTo(index), onScroll }`; renders only cells within the viewport ± 2 rows.
- Data loading: `GlyphGrid` requests `glyphs.list` pages of 300 (`include_paths: true`) for the visible range (debounced 16 ms, cancellation of stale requests by request id), caches `GlyphCell` by `gid+generation` in `pathCache` (LRU 20k entries) and `Path2D` objects by `gid+generation`; invalidates on `DocumentEvent.change.glyphs`.
- `cellRenderer.ts`: `paintCell(ctx, cell, { size, upem, ascender, descender, labelMode, selected, theme })` — draws the glyph fitted to the em box (y flipped), baseline tick, label, colour label stripe, selection ring, "empty" marker; DPR-aware.
- Toolbar: master selector, cell size slider, label mode, sort, "show only encoded"; Sidebar: sections **Categories / Scripts / Unicode blocks / Colours / Unencoded / All** with counts from `doc.sidebar`; multi-select filters update `documentStore.filter`; search box (debounced) → `filter.query`.
- Interaction: click select, Shift range, Ctrl/Cmd toggle, rubber-band, arrow-key navigation, double-click or Enter opens Glyph tab (`glyph.open`), Ctrl/Cmd+A select all in filter; selection count in status bar; right-click context menu with `glyph.open`, `edit.copyGlyphNames` (copies names to clipboard).

- [ ] **Step 1: Tests**: `useVirtualGrid` computes ranges for 65,535 items at 64 px cells in a 1200×800 viewport (rows/cols/range values asserted); `cellRenderer` draws without throwing on a mocked 2D context and calls `fill` for a non-empty path; `FontView` with a mocked transport shows the total count and requests only visible pages.
- [ ] **Step 2: Implement.** Verify with the dev server on Source Han Sans SC (`scripts/dev.sh` then `POST /api/session.open`): scroll from top to bottom must stay smooth (record with `agent-browser` a performance trace or measure frame times via `requestAnimationFrame` deltas logged in dev mode: 95th percentile < 20 ms). Save `docs/screenshots/p0-fontview-cjk.png`.
- [ ] **Step 3:** e2e `fontview.spec.ts`: start dev server with `--open fixtures/fonts/DejaVuSans.ttf`, load app, assert the grid shows glyph `A`, type `U+0042` in search → first cell is `B`, click Categories › Letter → status shows a count < total.
- [ ] **Step 4:** Commit `feat(app): virtualised font view with sidebar filters and search`.

---

### Task 11: Glyph view (read-only) and Font Info panel

**Files:** `app/src/views/glyph/{GlyphView.tsx, GlyphCanvas.tsx, viewport.ts, drawOutline.ts, drawMetrics.ts, GlyphInspector.tsx}`, `app/src/views/fontinfo/{FontInfoPanel.tsx, NamesTable.tsx, MetricsTable.tsx}`, `app/src/canvas/Viewport.ts` (shared pan/zoom transform class), tests.

**Interfaces:**
- `Viewport` class: `scale`, `offset`, `fit(bbox, padding)`, `zoomAt(screenPoint, factor)`, `pan(dx, dy)`, `toScreen(p)`, `toFont(p)`, `setSize(w,h,dpr)`; unit tested.
- `GlyphCanvas` draws for the active master layer: filled preview (toggle with space, default outline), contours with on-curve points (squares for corners, circles for smooth), off-curve handles and lines, start-point marker, direction arrow, component outlines in grey, anchors, metrics lines (baseline, x-height, cap-height, ascender, descender, LSB/RSB/advance), guides, rulers with font-unit ticks; wheel zoom (Ctrl for zoom, plain for scroll), middle/space+drag pan, `Mod+0` fit, `Mod+=`/`Mod+-` zoom. Hover shows coordinates in the status bar. Data from `glyphs.detail`; re-fetches on events for that gid.
- `GlyphInspector` (right inspector when a glyph tab is active): name, unicodes, category/sub-category/script (from `Glyph` fields, or computed client-side via a small lookup if empty), width/LSB/RSB (read-only in P0), components list, anchors list, layers list (masters), tags/colour/export/note (read-only display; editing arrives in P1).
- `FontInfoPanel` (opened via `file.fontInfo` command / `Mod+I` as a tab `kind: 'fontinfo'`): read-only sections Font (family/style/version/vendor/copyright/license), Names (table of all name records with language), Masters (metrics table per master), Axes/Instances (if any), Tables (raw table tags with sizes).

- [ ] **Step 1: Tests**: `Viewport` maths (fit + round trips); `drawOutline` handles quadratic contours with implied points (mock ctx: expect `quadraticCurveTo` calls) and cubic (expect `bezierCurveTo`); `FontInfoPanel` renders name records from a mocked summary.
- [ ] **Step 2: Implement**; verify with agent-browser on DejaVu `g` and a CJK glyph; save `docs/screenshots/p0-glyphview.png`, `docs/screenshots/p0-fontinfo.png`.
- [ ] **Step 3:** Commit `feat(app): read-only glyph view with metrics and font info panel`.

---

### Task 12: Integration, performance gates and P0 acceptance

**Files:** `crates/fw-sfnt/benches/open.rs` (criterion), `scripts/perf-open.sh`, `app/e2e/cjk.spec.ts` (skips when the external font is missing), `docs/p0-acceptance.md`.

- [ ] **Step 1:** Run the full acceptance list and record results in `docs/p0-acceptance.md` with commands and timings: (a) `cargo test --workspace --release`; (b) `fw inspect` on every fixture and Source Han Sans; (c) `fw convert SourceHanSansSC-Regular.otf shs.fwf` then `fw inspect shs.fwf` — same glyph count, timings for save/load < 6 s; (d) e2e suite green; (e) Tauri app: `cargo tauri dev` opens, `Mod+O` opens a font via native dialog, grid renders, glyph tab opens, Font Info opens, `Mod+S` writes `.fwf`, reopening restores.
- [ ] **Step 2:** Fix whatever fails; commit `test: P0 acceptance and performance gates`.

---

### Task 13: zh-CN localisation pass (translation agent only)

**Files:** `app/src/i18n/zh-CN.json`.

- [ ] **Step 1:** Translate every key in `en.json` into Simplified Chinese suitable for a professional font editor UI (use standard terms: 字形, 字体信息, 字距, 轮廓, 节点, 控制点, 侧边距, 字宽, 基线, x 高度, 大写高度, 上升部, 下降部, 母版, 实例, 轴, 特性, 点阵, 网格). Keep placeholders (`{{count}}`) intact.
- [ ] **Step 2:** `pnpm test` (a test asserts `zh-CN.json` has exactly the keys of `en.json`), commit `feat(i18n): zh-CN translation`.

---

## Dependency graph and delegation plan

```
T1 scaffold
 └─ T2 fw-model
     ├─ T3 fw-glyphdata      (parallel with T4/T5)
     ├─ T4 fw-sfnt read      (parallel)
     └─ T5 fw-doc            (parallel)
         └─ T6 fw-api (needs T3,T4,T5)
             ├─ T7 devserver + tauri + transport
             ├─ T8 cli
             └─ T9 app shell ── T10 font view ── T11 glyph view/font info ── T12 acceptance ── T13 zh-CN
```

Delegation: T1–T8 → Codex (gpt-5.6-sol, xhigh) via `scripts/codex-task.sh`, each in its own git worktree/branch, merged by the orchestrator after review; T9–T12 → Opus subagents with agent-browser screenshot verification; T13 → Sonnet/Opus translation agent. Cross-review: Codex output reviewed by an Opus reviewer against the task's acceptance list; Opus UI output reviewed by Codex (`codex exec` read-only review prompt).

## Self-review notes

- Spec coverage for the P0 row of §14: model ✔ (T2), doc/undo ✔ (T5), api ✔ (T6), sfnt read ✔ (T4), `.fwf` ✔ (T5), devserver ✔ (T7), CLI ✔ (T8), shell + Font view + read-only Glyph view + Font Info + i18n + theming ✔ (T9–T11, T13), performance gates ✔ (T12). Bitmap strikes are read raw only (spec 6.2 step 5 decoding is deferred to P3 — stated in T4). Glyph data sidebar categories ✔ (T3 + T6 `doc.sidebar`).
- Names used consistently: `Session`, `Document`, `Command`, `Change`, `DocumentEvent`, `GlyphCell`, `GlyphFilter`, `DocSummary`, `SidebarCounts`, `Transport`, `ApiMethods`, `flattened_path`, `Contour::to_bezpath`.
