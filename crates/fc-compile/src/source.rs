//! The in-memory `fontir::source::Source` implementation.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use fc_model::{Font, GlyphName, KernSide, Master, MasterId};
use fontdrasil::{
    coords::NormalizedLocation,
    orchestration::{Access, AccessBuilder, Work},
    types::GlyphName as IrGlyphName,
};
use fontir::{
    error::Error as IrError,
    ir::{
        AnchorBuilder, Component as IrComponent, FeaturesSource as IrFeaturesSource, GlobalMetric,
        GlobalMetricsBuilder, Glyph as IrGlyph, GlyphInstance, GlyphOrder, KernGroup,
        KernSide as IrKernSide, KerningInstance, KerningLocations, StaticMetadata,
    },
    orchestration::{Context, Flags, IrWork, WorkId},
    source::Source,
};
use kurbo::Point;

use crate::metadata::{apply_misc_metadata, names};
use crate::Error;

/// Options that affect the fontc compilation phase.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileOptions {
    /// Replace source glyph names with their production names when available.
    pub production_names: bool,
}

/// Bytes and stable model identity produced by one static compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticCompile {
    /// Compiled TrueType-flavored sfnt bytes.
    pub bytes: Vec<u8>,
    /// Exact model glyph names in compiled glyph-ID order.
    ///
    /// A synthesized `.notdef` is included even when the model did not contain
    /// one; non-exporting glyphs are absent. These are model identities rather
    /// than optional production names from the emitted `post` table.
    pub glyph_order: Vec<GlyphName>,
}

#[derive(Clone, Debug)]
struct SourceData {
    font: Font,
    master: MasterId,
    options: CompileOptions,
    large_font: bool,
    fontc_safe_bmp_codepoints: Option<Arc<HashSet<(GlyphName, u32)>>>,
}

/// A static, single-master, in-memory source for fontc.
#[derive(Clone, Debug)]
pub struct FcSource {
    data: Arc<SourceData>,
}

impl FcSource {
    /// Copies a FontCarver font into a source selecting one master.
    pub fn from_font(
        font: &Font,
        master: &MasterId,
        options: &CompileOptions,
    ) -> Result<Self, Error> {
        if font.master(master).is_none() {
            return Err(Error::MissingMaster(master.clone()));
        }
        if !font.glyphs.values().any(|glyph| glyph.export) {
            return Err(Error::NoExportedGlyphs);
        }
        if !(16..=16_384).contains(&font.info.units_per_em) {
            return Err(Error::InvalidUnitsPerEm(font.info.units_per_em));
        }
        let large_font = crate::large_font::requires_workaround(font, options.production_names);
        Ok(Self {
            data: Arc::new(SourceData {
                font: font.clone(),
                master: master.clone(),
                options: options.clone(),
                large_font,
                fontc_safe_bmp_codepoints: large_font
                    .then(|| Arc::new(crate::large_font::fontc_safe_bmp_codepoints(font))),
            }),
        })
    }
}

/// Compiles one master of a model font to a static TrueType-flavored sfnt.
pub fn compile_static(
    font: &Font,
    master: &MasterId,
    options: &CompileOptions,
) -> Result<Vec<u8>, Error> {
    compile_static_with_metadata(font, master, options).map(|compiled| compiled.bytes)
}

/// Compiles one master and retains exact model names in compiled glyph-ID order.
pub fn compile_static_with_metadata(
    font: &Font,
    master: &MasterId,
    options: &CompileOptions,
) -> Result<StaticCompile, Error> {
    compile_static_impl(font, master, options, false)
}

fn compile_static_impl(
    font: &Font,
    master: &MasterId,
    options: &CompileOptions,
    force_large_patch: bool,
) -> Result<StaticCompile, Error> {
    let source = FcSource::from_font(font, master, options)?;
    let mut flags = Flags::PREFER_SIMPLE_GLYPHS;
    let mut flags_to_disable = Flags::empty();
    if options.production_names {
        flags |= Flags::PRODUCTION_NAMES;
    } else {
        flags_to_disable |= Flags::PRODUCTION_NAMES;
    }
    let fontc_options = fontc::Options {
        flags,
        flags_to_disable: flags_to_disable.into(),
        ..fontc::Options::default()
    };
    let bytes = fontc::generate_font(Box::new(source), fontc_options)?;
    let glyph_order = {
        let face = read_fonts::FontRef::new(&bytes)
            .map_err(|error| Error::parse("compiled sfnt", error))?;
        crate::large_font::compiled_glyph_order(&face, font, options.production_names)?
    };
    let bytes = if force_large_patch
        || crate::large_font::requires_workaround(font, options.production_names)
    {
        crate::large_font::patch_tables(&bytes, font, &glyph_order)?
    } else {
        bytes
    };
    let bytes = crate::metadata::patch_modified_timestamp(&bytes, &font.info)?;
    Ok(StaticCompile { bytes, glyph_order })
}

impl Source for FcSource {
    fn new(root: &Path) -> Result<Self, IrError> {
        Err(IrError::NoSuchPath(root.to_path_buf()))
    }

    fn create_static_metadata_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(StaticMetadataWork(Arc::clone(&self.data))))
    }

    fn create_global_metric_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(GlobalMetricsWork(Arc::clone(&self.data))))
    }

    fn create_glyph_ir_work(&self) -> Result<Vec<Box<IrWork>>, IrError> {
        Ok(self
            .data
            .font
            .glyphs
            .values()
            .cloned()
            .map(|glyph| {
                Box::new(GlyphWork {
                    glyph,
                    master: self.data.master.clone(),
                    fontc_safe_bmp_codepoints: self.data.fontc_safe_bmp_codepoints.clone(),
                }) as Box<IrWork>
            })
            .collect())
    }

    fn create_feature_ir_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(FeatureWork(feature_source(&self.data.font))))
    }

    fn create_kerning_locations_ir_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(KerningLocationsWork))
    }

    fn create_kerning_instance_ir_work(
        &self,
        at: NormalizedLocation,
    ) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(KerningInstanceWork {
            data: Arc::clone(&self.data),
            location: at,
        }))
    }

    fn create_color_palette_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(NoopWork(WorkId::ColorPalettes)))
    }

    fn create_color_glyphs_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(NoopWork(WorkId::PaintGraph)))
    }

    fn compilation_flags(&self) -> Flags {
        if self.data.options.production_names {
            Flags::PRODUCTION_NAMES
        } else {
            Flags::empty()
        }
    }
}

fn default_location() -> NormalizedLocation {
    NormalizedLocation::new()
}

#[derive(Debug)]
struct StaticMetadataWork(Arc<SourceData>);

impl Work<Context, WorkId, IrError> for StaticMetadataWork {
    fn id(&self) -> WorkId {
        WorkId::StaticMetadata
    }

    fn also_completes(&self) -> Vec<WorkId> {
        vec![
            WorkId::PreliminaryGlyphOrder,
            WorkId::PreliminaryGdefCategories,
        ]
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        let font = &self.0.font;
        let master = selected_master(&self.0)?;
        let mut postscript_names = HashMap::new();
        if self.0.options.production_names {
            postscript_names.extend(font.glyphs.values().filter_map(|glyph| {
                glyph.production_name.as_ref().map(|production| {
                    (
                        IrGlyphName::new(glyph.name.as_ref()),
                        IrGlyphName::new(production),
                    )
                })
            }));
        }
        if self.0.large_font {
            postscript_names.extend(
                crate::large_font::workaround_renames(font, self.0.options.production_names)
                    .into_iter()
                    .map(|(glyph, standard)| {
                        (IrGlyphName::new(glyph.as_ref()), IrGlyphName::new(standard))
                    }),
            );
        }
        let postscript_names = (!postscript_names.is_empty()).then_some(postscript_names);
        let build_vertical = font.glyphs.values().any(|glyph| {
            glyph
                .master_layer(&self.0.master)
                .is_some_and(|layer| layer.vertical_width.is_some())
        });
        let mut metadata = StaticMetadata::new(
            font.info.units_per_em,
            names(&font.info),
            Vec::new(),
            Vec::new(),
            HashSet::from([default_location()]),
            postscript_names,
            master.metrics.italic_angle,
            None,
            build_vertical,
        )?;
        apply_misc_metadata(&mut metadata, &font.info);
        context.static_metadata.set(metadata);

        let mut order = GlyphOrder::new();
        for name in font.glyph_order() {
            order.insert(IrGlyphName::new(name.as_ref()));
        }
        context.preliminary_glyph_order.set(order);
        context.preliminary_gdef_categories.set(Default::default());
        Ok(())
    }
}

fn selected_master(data: &SourceData) -> Result<&Master, IrError> {
    data.font
        .master(&data.master)
        .ok_or(IrError::NoDefaultMaster)
}

#[derive(Debug)]
struct GlobalMetricsWork(Arc<SourceData>);

impl Work<Context, WorkId, IrError> for GlobalMetricsWork {
    fn id(&self) -> WorkId {
        WorkId::GlobalMetrics
    }

    fn read_access(&self) -> Access<WorkId> {
        Access::Variant(WorkId::StaticMetadata)
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        let static_metadata = context.static_metadata.get();
        let master = selected_master(&self.0)?;
        let metrics = &master.metrics;
        let info = &self.0.font.info;
        let location = default_location();
        let mut builder = GlobalMetricsBuilder::new();
        for (metric, value) in [
            (GlobalMetric::Ascender, Some(metrics.ascender)),
            (GlobalMetric::Descender, Some(metrics.descender)),
            (GlobalMetric::HheaAscender, metrics.hhea_ascender),
            (GlobalMetric::HheaDescender, metrics.hhea_descender),
            (GlobalMetric::HheaLineGap, metrics.hhea_line_gap),
            (GlobalMetric::Os2TypoAscender, metrics.typo_ascender),
            (GlobalMetric::Os2TypoDescender, metrics.typo_descender),
            (GlobalMetric::Os2TypoLineGap, metrics.typo_line_gap),
            (GlobalMetric::Os2WinAscent, metrics.win_ascent),
            (GlobalMetric::Os2WinDescent, metrics.win_descent),
            (GlobalMetric::CapHeight, Some(metrics.cap_height)),
            (GlobalMetric::XHeight, Some(metrics.x_height)),
            (GlobalMetric::VheaAscender, metrics.vhea_ascender),
            (GlobalMetric::VheaDescender, metrics.vhea_descender),
            (GlobalMetric::VheaLineGap, metrics.vhea_line_gap),
            (
                GlobalMetric::CaretSlopeRise,
                info.open_type_hhea_caret_slope_rise.map(f64::from),
            ),
            (
                GlobalMetric::CaretSlopeRun,
                info.open_type_hhea_caret_slope_run.map(f64::from),
            ),
            (
                GlobalMetric::CaretOffset,
                info.open_type_hhea_caret_offset.map(f64::from),
            ),
            (
                GlobalMetric::VheaCaretSlopeRise,
                info.open_type_vhea_caret_slope_rise.map(f64::from),
            ),
            (
                GlobalMetric::VheaCaretSlopeRun,
                info.open_type_vhea_caret_slope_run.map(f64::from),
            ),
            (
                GlobalMetric::VheaCaretOffset,
                info.open_type_vhea_caret_offset.map(f64::from),
            ),
            (
                GlobalMetric::UnderlineThickness,
                info.postscript_underline_thickness,
            ),
            (
                GlobalMetric::UnderlinePosition,
                info.postscript_underline_position,
            ),
            (
                GlobalMetric::SubscriptXSize,
                info.open_type_os2_subscript_x_size,
            ),
            (
                GlobalMetric::SubscriptYSize,
                info.open_type_os2_subscript_y_size,
            ),
            (
                GlobalMetric::SubscriptXOffset,
                info.open_type_os2_subscript_x_offset,
            ),
            (
                GlobalMetric::SubscriptYOffset,
                info.open_type_os2_subscript_y_offset,
            ),
            (
                GlobalMetric::SuperscriptXSize,
                info.open_type_os2_superscript_x_size,
            ),
            (
                GlobalMetric::SuperscriptYSize,
                info.open_type_os2_superscript_y_size,
            ),
            (
                GlobalMetric::SuperscriptXOffset,
                info.open_type_os2_superscript_x_offset,
            ),
            (
                GlobalMetric::SuperscriptYOffset,
                info.open_type_os2_superscript_y_offset,
            ),
            (
                GlobalMetric::StrikeoutSize,
                info.open_type_os2_strikeout_size,
            ),
            (
                GlobalMetric::StrikeoutPosition,
                info.open_type_os2_strikeout_position,
            ),
        ] {
            builder.set_if_some(metric, location.clone(), value);
        }
        builder.populate_defaults(
            &location,
            static_metadata.units_per_em,
            Some(metrics.x_height),
            Some(metrics.ascender),
            Some(metrics.descender),
            Some(metrics.italic_angle),
        );
        context
            .global_metrics
            .set(builder.build(&static_metadata.axes)?);
        Ok(())
    }
}

#[derive(Debug)]
struct GlyphWork {
    glyph: fc_model::Glyph,
    master: MasterId,
    fontc_safe_bmp_codepoints: Option<Arc<HashSet<(GlyphName, u32)>>>,
}

impl Work<Context, WorkId, IrError> for GlyphWork {
    fn id(&self) -> WorkId {
        WorkId::Glyph(IrGlyphName::new(self.glyph.name.as_ref()))
    }

    fn read_access(&self) -> Access<WorkId> {
        Access::Variant(WorkId::StaticMetadata)
    }

    fn also_completes(&self) -> Vec<WorkId> {
        vec![WorkId::Anchor(IrGlyphName::new(self.glyph.name.as_ref()))]
    }

    fn write_access(&self) -> Access<WorkId> {
        AccessBuilder::new()
            .specific_instance(self.id())
            .specific_instance(WorkId::Anchor(IrGlyphName::new(self.glyph.name.as_ref())))
            .build()
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        let name = IrGlyphName::new(self.glyph.name.as_ref());
        let layer = self.glyph.master_layer(&self.master).ok_or_else(|| {
            IrError::NoSuchLayer(format!("{} / {}", self.glyph.name, self.master))
        })?;
        let contours = layer
            .contours
            .iter()
            .map(|contour| contour.to_bezpath())
            .collect();
        let components = layer
            .components
            .iter()
            .map(|component| IrComponent {
                base: IrGlyphName::new(component.base.as_ref()),
                transform: component.transform,
                anchor: None,
            })
            .collect();
        let instance = GlyphInstance {
            width: layer.width,
            height: layer.vertical_width,
            vertical_origin: layer.vertical_origin,
            contours,
            components,
        };
        // The final large-font cmap is rebuilt from every model codepoint. The
        // bounded BMP set here only keeps write-fonts 0.52's temporary format
        // 4 builder away from its signed-delta and 16-bit-offset panics.
        let codepoints = self
            .glyph
            .unicodes
            .iter()
            .copied()
            .filter(|codepoint| {
                self.fontc_safe_bmp_codepoints
                    .as_ref()
                    .is_none_or(|allowed| allowed.contains(&(self.glyph.name.clone(), *codepoint)))
            })
            .collect();
        let glyph = IrGlyph::new(
            name.clone(),
            self.glyph.export,
            codepoints,
            HashMap::from([(default_location(), instance)]),
        )?;
        context.glyphs.set(glyph);

        let mut anchors = AnchorBuilder::new(name);
        for anchor in &layer.anchors {
            anchors.add(
                anchor.name.clone().into(),
                default_location(),
                Point::new(anchor.x, anchor.y),
            )?;
        }
        context.anchors.set(anchors.build()?);
        Ok(())
    }
}

#[derive(Debug)]
struct FeatureWork(IrFeaturesSource);

impl Work<Context, WorkId, IrError> for FeatureWork {
    fn id(&self) -> WorkId {
        WorkId::Features
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        context.features.set(self.0.clone());
        Ok(())
    }
}

fn feature_source(font: &Font) -> IrFeaturesSource {
    let source = font.features.to_fea();
    if source.trim().is_empty() {
        IrFeaturesSource::Empty
    } else {
        IrFeaturesSource::from_string(source)
    }
}

#[derive(Debug)]
struct KerningLocationsWork;

impl Work<Context, WorkId, IrError> for KerningLocationsWork {
    fn id(&self) -> WorkId {
        WorkId::KerningLocations
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        context.kerning_locations.set(KerningLocations {
            locations: BTreeSet::from([default_location()]),
        });
        Ok(())
    }
}

#[derive(Debug)]
struct KerningInstanceWork {
    data: Arc<SourceData>,
    location: NormalizedLocation,
}

impl Work<Context, WorkId, IrError> for KerningInstanceWork {
    fn id(&self) -> WorkId {
        WorkId::KernInstance(self.location.clone())
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        let master = selected_master(&self.data)?;
        let prepared = crate::kerning::prepare(&self.data.font, &master.id);
        let kerns = prepared
            .pairs
            .0
            .iter()
            .map(|((left, right), value)| {
                (
                    (map_kern_side(left, true), map_kern_side(right, false)),
                    (*value).into(),
                )
            })
            .collect();
        let mut groups = BTreeMap::new();
        groups.extend(prepared.groups.side1.iter().map(|(name, members)| {
            (
                KernGroup::Side1(name.as_str().into()),
                members
                    .iter()
                    .map(|member| IrGlyphName::new(member.as_ref()))
                    .collect(),
            )
        }));
        groups.extend(prepared.groups.side2.iter().map(|(name, members)| {
            (
                KernGroup::Side2(name.as_str().into()),
                members
                    .iter()
                    .map(|member| IrGlyphName::new(member.as_ref()))
                    .collect(),
            )
        }));
        context.kerning_at.set(KerningInstance {
            location: self.location.clone(),
            kerns,
            groups,
        });
        Ok(())
    }
}

fn map_kern_side(side: &KernSide, is_left: bool) -> IrKernSide {
    match side {
        KernSide::Glyph(name) => IrKernSide::Glyph(IrGlyphName::new(name.as_ref())),
        KernSide::Group(name) if is_left => {
            IrKernSide::Group(KernGroup::Side1(name.as_str().into()))
        }
        KernSide::Group(name) => IrKernSide::Group(KernGroup::Side2(name.as_str().into())),
    }
}

#[derive(Debug)]
struct NoopWork(WorkId);

impl Work<Context, WorkId, IrError> for NoopWork {
    fn id(&self) -> WorkId {
        self.0.clone()
    }

    fn exec(&self, _context: &Context) -> Result<(), IrError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use fc_model::{Font, Glyph, GlyphName, Layer};
    use read_fonts::{types::Version16Dot16, FontRef, TableProvider};

    use super::{compile_static_impl, CompileOptions};

    #[test]
    fn forced_post_three_keeps_pre_patch_model_identity() {
        let mut font = Font::new_empty("Post Three Identity", 1_000);
        let master = font.default_master().id.clone();
        for (name, unicode) in [("A", Some(0x41)), ("unencoded.alt", None)] {
            let mut glyph = Glyph::new(name);
            glyph.unicodes.extend(unicode);
            glyph.layers.push(Layer::new_master(master.clone()));
            font.insert_glyph(glyph).expect("insert glyph");
        }

        let compiled = compile_static_impl(&font, &master, &CompileOptions::default(), true)
            .expect("compile with forced large-font patch");
        let face = FontRef::new(&compiled.bytes).expect("parse patched font");

        assert_eq!(
            face.post().expect("post").version(),
            Version16Dot16::VERSION_3_0
        );
        assert_eq!(
            compiled.glyph_order,
            [
                GlyphName::from(".notdef"),
                GlyphName::from("A"),
                GlyphName::from("unencoded.alt"),
            ]
        );
        assert_eq!(
            usize::from(face.maxp().expect("maxp").num_glyphs()),
            compiled.glyph_order.len()
        );
    }
}
