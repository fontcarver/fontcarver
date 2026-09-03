//! Proof that a THIRD-PARTY crate can implement `fontir::source::Source` over its own
//! in-memory model and drive the whole fontc pipeline via `fontc::generate_font`.
//! Nothing here touches the filesystem.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use fontdrasil::coords::NormalizedLocation;
use fontdrasil::orchestration::{Access, AccessBuilder, Work};
use fontdrasil::types::GlyphName;
use fontir::error::Error;
use fontir::ir::{
    FeaturesSource, GlobalMetricsBuilder, Glyph, GlyphAnchors, GlyphInstance, GlyphOrder,
    KernGroup, KerningInstance, KerningLocations, KernSide, NameKey, StaticMetadata,
};
use fontir::orchestration::{Context, IrWork, WorkId};
use fontir::source::Source;
use kurbo::BezPath;
use write_fonts::types::NameId;

// ---------------------------------------------------------------- editor model
#[derive(Clone, Debug)]
struct EditorGlyph {
    name: &'static str,
    codepoint: u32,
    width: f64,
    /// (x, y, w, h) rectangle
    rect: Option<(f64, f64, f64, f64)>,
}

#[derive(Clone, Debug)]
struct EditorDoc {
    family: String,
    upem: u16,
    glyphs: Vec<EditorGlyph>,
    fea: String,
    kern: Vec<(&'static str, &'static str, f64)>,
}

impl EditorDoc {
    fn demo() -> Self {
        EditorDoc {
            family: "SpikeCustom".into(),
            upem: 1000,
            glyphs: vec![
                EditorGlyph { name: ".notdef", codepoint: 0, width: 600.0, rect: None },
                EditorGlyph { name: "space", codepoint: 32, width: 250.0, rect: None },
                EditorGlyph { name: "H", codepoint: 72, width: 700.0, rect: Some((80.0, 0.0, 540.0, 700.0)) },
                EditorGlyph { name: "I", codepoint: 73, width: 300.0, rect: Some((100.0, 0.0, 100.0, 700.0)) },
            ],
            fea: "feature ss01 { sub H by I; } ss01;\n".to_string(),
            kern: vec![("H", "I", -55.0)],
        }
    }
    fn order(&self) -> Vec<GlyphName> {
        self.glyphs.iter().map(|g| GlyphName::new(g.name)).collect()
    }
}

// ---------------------------------------------------------------- Source impl
struct EditorSource(Arc<EditorDoc>);

impl Source for EditorSource {
    fn new(_root: &Path) -> Result<Self, Error> {
        // A real editor never loads from a path; construct via `EditorSource::from_doc`.
        Ok(EditorSource(Arc::new(EditorDoc::demo())))
    }
    fn create_static_metadata_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(StaticMetaWork(self.0.clone())))
    }
    fn create_global_metric_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(MetricsWork(self.0.clone())))
    }
    fn create_glyph_ir_work(&self) -> Result<Vec<Box<IrWork>>, Error> {
        Ok(self
            .0
            .glyphs
            .iter()
            .cloned()
            .map(|g| Box::new(GlyphWork(g)) as Box<IrWork>)
            .collect())
    }
    fn create_feature_ir_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(FeatureWork(self.0.clone())))
    }
    fn create_kerning_locations_ir_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(KernLocWork))
    }
    fn create_kerning_instance_ir_work(&self, at: NormalizedLocation) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(KernInstWork(self.0.clone(), at)))
    }
    fn create_color_palette_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(NoopWork(WorkId::ColorPalettes)))
    }
    fn create_color_glyphs_work(&self) -> Result<Box<IrWork>, Error> {
        Ok(Box::new(NoopWork(WorkId::PaintGraph)))
    }
}

fn default_loc() -> NormalizedLocation {
    NormalizedLocation::new()
}

// ---------------------------------------------------------------- work items
#[derive(Debug)]
struct StaticMetaWork(Arc<EditorDoc>);
impl Work<Context, WorkId, Error> for StaticMetaWork {
    fn id(&self) -> WorkId { WorkId::StaticMetadata }
    fn also_completes(&self) -> Vec<WorkId> {
        vec![WorkId::PreliminaryGlyphOrder, WorkId::PreliminaryGdefCategories]
    }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        let mut names = HashMap::new();
        names.insert(NameKey::new_bmp_only(NameId::FAMILY_NAME), self.0.family.clone());
        names.insert(NameKey::new_bmp_only(NameId::SUBFAMILY_NAME), "Regular".to_string());
        names.insert(
            NameKey::new_bmp_only(NameId::UNIQUE_ID),
            format!("{} Regular", self.0.family),
        );
        names.insert(
            NameKey::new_bmp_only(NameId::FULL_NAME),
            format!("{} Regular", self.0.family),
        );
        names.insert(NameKey::new_bmp_only(NameId::VERSION_STRING), "Version 1.000".to_string());
        names.insert(
            NameKey::new_bmp_only(NameId::POSTSCRIPT_NAME),
            format!("{}-Regular", self.0.family),
        );

        let static_metadata = StaticMetadata::new(
            self.0.upem,
            names,
            Vec::new(),            // no axes -> STATIC font
            Vec::new(),            // no named instances
            HashSet::from([default_loc()]),
            None,                  // postscript names
            0.0,                   // italic angle
            None,                  // glyphs.app number values
            false,                 // build_vertical
        )
        .map_err(Error::VariationModelError)?;

        context.static_metadata.set(static_metadata);
        let mut order = GlyphOrder::new();
        for name in self.0.order() {
            order.insert(name);
        }
        context.preliminary_glyph_order.set(order);
        context.preliminary_gdef_categories.set(Default::default());
        Ok(())
    }
}

#[derive(Debug)]
struct MetricsWork(Arc<EditorDoc>);
impl Work<Context, WorkId, Error> for MetricsWork {
    fn id(&self) -> WorkId { WorkId::GlobalMetrics }
    fn read_access(&self) -> Access<WorkId> { Access::Variant(WorkId::StaticMetadata) }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        let sm = context.static_metadata.get();
        let mut b = GlobalMetricsBuilder::new();
        b.populate_defaults(
            &default_loc(),
            self.0.upem,
            Some(500.0),   // x-height
            Some(800.0),   // ascender
            Some(-200.0),  // descender
            Some(0.0),     // italic angle
        );
        context.global_metrics.set(b.build(&sm.axes)?);
        Ok(())
    }
}

#[derive(Debug)]
struct GlyphWork(EditorGlyph);
impl Work<Context, WorkId, Error> for GlyphWork {
    fn id(&self) -> WorkId { WorkId::Glyph(GlyphName::new(self.0.name)) }
    fn read_access(&self) -> Access<WorkId> { Access::Variant(WorkId::StaticMetadata) }
    fn also_completes(&self) -> Vec<WorkId> { vec![WorkId::Anchor(GlyphName::new(self.0.name))] }
    fn write_access(&self) -> Access<WorkId> {
        AccessBuilder::new()
            .specific_instance(WorkId::Glyph(GlyphName::new(self.0.name)))
            .specific_instance(WorkId::Anchor(GlyphName::new(self.0.name)))
            .build()
    }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        let mut contours = Vec::new();
        if let Some((x, y, w, h)) = self.0.rect {
            let mut p = BezPath::new();
            p.move_to((x, y));
            p.line_to((x, y + h));
            p.line_to((x + w, y + h));
            p.line_to((x + w, y));
            p.close_path();
            contours.push(p);
        }
        let inst = GlyphInstance {
            width: self.0.width,
            height: None,
            vertical_origin: None,
            contours,
            components: Vec::new(),
        };
        let codepoints = if self.0.codepoint == 0 {
            HashSet::new()
        } else {
            HashSet::from([self.0.codepoint])
        };
        let glyph = Glyph::new(
            GlyphName::new(self.0.name),
            true,
            codepoints,
            HashMap::from([(default_loc(), inst)]),
        )?;
        context.glyphs.set(glyph);
        context
            .anchors
            .set(GlyphAnchors::new(GlyphName::new(self.0.name), Vec::new()));
        Ok(())
    }
}

#[derive(Debug)]
struct FeatureWork(Arc<EditorDoc>);
impl Work<Context, WorkId, Error> for FeatureWork {
    fn id(&self) -> WorkId { WorkId::Features }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        // FEA straight from a String -- no file on disk.
        context.features.set(FeaturesSource::from_string(self.0.fea.clone()));
        Ok(())
    }
}

#[derive(Debug)]
struct KernLocWork;
impl Work<Context, WorkId, Error> for KernLocWork {
    fn id(&self) -> WorkId { WorkId::KerningLocations }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        context.kerning_locations.set(KerningLocations {
            locations: BTreeSet::from([default_loc()]),
        });
        Ok(())
    }
}

#[derive(Debug)]
struct KernInstWork(Arc<EditorDoc>, NormalizedLocation);
impl Work<Context, WorkId, Error> for KernInstWork {
    fn id(&self) -> WorkId { WorkId::KernInstance(self.1.clone()) }
    fn exec(&self, context: &Context) -> Result<(), Error> {
        let mut kerns = BTreeMap::new();
        for (a, b, v) in &self.0.kern {
            kerns.insert(
                (
                    KernSide::Glyph(GlyphName::new(*a)),
                    KernSide::Glyph(GlyphName::new(*b)),
                ),
                (*v).into(),
            );
        }
        context.kerning_at.set(KerningInstance {
            location: self.1.clone(),
            kerns,
            groups: BTreeMap::<KernGroup, BTreeSet<GlyphName>>::new(),
        });
        Ok(())
    }
}

#[derive(Debug)]
struct NoopWork(WorkId);
impl Work<Context, WorkId, Error> for NoopWork {
    fn id(&self) -> WorkId { self.0.clone() }
    fn exec(&self, _context: &Context) -> Result<(), Error> { Ok(()) }
}

// ---------------------------------------------------------------- main
fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let source = EditorSource(Arc::new(EditorDoc::demo()));
    let bytes = match fontc::generate_font(Box::new(source), fontc::Options::default()) {
        Ok(b) => b,
        Err(e) => { eprintln!("generate_font FAILED: {e}"); std::process::exit(1); }
    };
    println!("custom Source -> fontc::generate_font -> {} bytes", bytes.len());
    std::fs::write("/tmp/spike_custom.ttf", &bytes).unwrap();

    use read_fonts::{FontRef, TableProvider};
    let font = FontRef::new(&bytes).unwrap();
    let tags: Vec<String> = font.table_directory().table_records().iter().map(|r| r.tag().to_string()).collect();
    println!("tables: {}", tags.join(" "));
    println!("num_glyphs = {}", font.maxp().unwrap().num_glyphs());
    let name = font.name().unwrap();
    for rec in name.name_record().iter().take(3) {
        let s = rec.string(name.string_data()).unwrap();
        println!("  name[{:?}] = {}", rec.name_id(), s);
    }

    use skrifa::{instance::{LocationRef, Size}, MetadataProvider};
    let gm = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let h = font.charmap().map('H').unwrap();
    println!("advance H = {:?}", gm.advance_width(h));

    use harfrust::{FontRef as HrFontRef, ShapeOptions, ShaperData, UnicodeBuffer};
    let hf = HrFontRef::from_index(&bytes, 0).unwrap();
    let d = ShaperData::new(&hf);
    let sh = d.shaper(&hf).build();
    let mut buf = UnicodeBuffer::new();
    buf.push_str("HI");
    buf.guess_segment_properties();
    let out = sh.shape(buf, ShapeOptions::new());
    for (i, p) in out.glyph_infos().iter().zip(out.glyph_positions().iter()) {
        println!("  shaped gid={} x_adv={}", i.glyph_id, p.x_advance);
    }
    println!("(expect H advance 700 -> 645 after -55 kern)");
}
