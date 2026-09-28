//! The in-memory `fontir::source::Source` implementation.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use fc_geom::variation::{compiler_axes, glyph_sources, normalized_location};
use fc_model::{Font, GlyphName, KernSide, Master, MasterId};
use fontdrasil::{
    coords::{NormalizedLocation, UserCoord},
    orchestration::{Access, AccessBuilder, Work},
    types::GlyphName as IrGlyphName,
};
use fontir::{
    error::Error as IrError,
    ir::{
        AnchorBuilder, Component as IrComponent, FeaturesSource as IrFeaturesSource, GlobalMetric,
        GlobalMetricsBuilder, Glyph as IrGlyph, GlyphInstance, GlyphOrder, KernGroup,
        KernSide as IrKernSide, KerningInstance, KerningLocations, NamedInstance, StaticMetadata,
    },
    orchestration::{Context, Flags, IrWork, WorkId},
    source::Source,
};
use kurbo::Point;

use crate::metadata::{apply_misc_metadata, names};
use crate::Error;

mod metrics;
use metrics::GlobalMetricsWork;

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
    variable: bool,
    variations: Option<fontir::ir::VariableFeature>,
    options: CompileOptions,
    large_font: bool,
    fontc_safe_bmp_codepoints: Option<Arc<HashSet<(GlyphName, u32)>>>,
}

/// An in-memory source for static or variable fontc compilation.
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
        if font.extra.lib.contains_key("ufo.designspace.rules") {
            return Err(Error::InvalidSource(
                "preserved native designspace rules require conversion before compilation".into(),
            ));
        }
        crate::feature_validation::check_writer_conflicts(font)?;
        let large_font = crate::large_font::requires_workaround(font, options.production_names);
        let (baked, _) =
            fc_geom::bake_effects_font(font).map_err(|e| Error::InvalidSource(e.to_string()))?;
        let mut resolved =
            fc_geom::resolve_smart_font(&baked).map_err(|e| Error::InvalidSource(e.to_string()))?;
        crate::vertical::apply_metric_defaults(&mut resolved);
        Ok(Self {
            data: Arc::new(SourceData {
                font: resolved,
                master: master.clone(),
                variable: false,
                variations: None,
                options: options.clone(),
                large_font,
                fontc_safe_bmp_codepoints: large_font
                    .then(|| Arc::new(crate::large_font::fontc_safe_bmp_codepoints(font))),
            }),
        })
    }

    /// Build a variable source from every master and intermediate layer.
    pub fn from_variable_font(font: &Font, options: &CompileOptions) -> Result<Self, Error> {
        let (prepared, variations) = crate::bracket::prepare(font)?;
        let font = &prepared;
        font.validate_designspace()
            .map_err(|e| Error::InvalidSource(e.to_string()))?;
        if font.axes.is_empty() {
            return Err(Error::InvalidSource(
                "variable export requires at least one axis".into(),
            ));
        }
        let master = font
            .design_default_master()
            .map_err(|e| Error::InvalidSource(e.to_string()))?;
        let mut source = Self::from_font(font, &master.id, options)?;
        for glyph in source.data.font.glyphs.values() {
            fc_geom::variation::interpolate_glyph(&source.data.font, glyph, &master.location)
                .map_err(|e| Error::InvalidSource(format!("{}: {e}", glyph.name)))?;
        }
        Arc::make_mut(&mut source.data).variable = true;
        Arc::make_mut(&mut source.data).variations = variations;
        Ok(source)
    }
}

/// Compile all masters and intermediate sources into a variable TrueType font.
pub fn compile_variable(font: &Font, options: &CompileOptions) -> Result<StaticCompile, Error> {
    compile_variable_with_sources(font, options).map(|(compiled, _)| compiled)
}

pub(crate) fn compile_variable_with_sources(
    font: &Font,
    options: &CompileOptions,
) -> Result<(StaticCompile, Font), Error> {
    let source = FcSource::from_variable_font(font, options)?;
    let prepared = source.data.font.clone();
    let compiled = compile_source(source, &prepared, options, false)?;
    Ok((compiled, prepared))
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
    compile_source(source, font, options, force_large_patch)
}

fn compile_source(
    source: FcSource,
    font: &Font,
    options: &CompileOptions,
    force_large_patch: bool,
) -> Result<StaticCompile, Error> {
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
        self.data
            .font
            .glyphs
            .values()
            .cloned()
            .map(|glyph| {
                let layers = if self.data.variable {
                    glyph_sources(&self.data.font, &glyph)
                        .map_err(|e| IrError::InvalidEntry("glyph sources", e.to_string()))?
                        .into_iter()
                        .map(|(at, layer)| {
                            (
                                normalized_location(&self.data.font.axes, &at),
                                layer.clone(),
                            )
                        })
                        .collect()
                } else {
                    vec![(
                        default_location(),
                        glyph
                            .master_layer(&self.data.master)
                            .ok_or_else(|| {
                                IrError::NoSuchLayer(format!(
                                    "{} / {}",
                                    glyph.name, self.data.master
                                ))
                            })?
                            .clone(),
                    )]
                };
                Ok(Box::new(GlyphWork {
                    glyph,
                    layers,
                    model_anchors: self.data.font.feature_settings.writer_policy.anchors,
                    fontc_safe_bmp_codepoints: self.data.fontc_safe_bmp_codepoints.clone(),
                }) as Box<IrWork>)
            })
            .collect()
    }

    fn create_feature_ir_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(FeatureWork(feature_source(&self.data)?)))
    }

    fn create_kerning_locations_ir_work(&self) -> Result<Box<IrWork>, IrError> {
        Ok(Box::new(KerningLocationsWork(
            self.data
                .master_locations()
                .into_iter()
                .map(|(_, loc)| loc)
                .collect(),
        )))
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

impl SourceData {
    fn master_locations(&self) -> Vec<(&Master, NormalizedLocation)> {
        self.font
            .masters
            .iter()
            .filter(|m| self.variable || m.id == self.master)
            .map(|m| {
                (
                    m,
                    if self.variable {
                        normalized_location(&self.font.axes, &m.location)
                    } else {
                        default_location()
                    },
                )
            })
            .collect()
    }
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
            if self.0.variable {
                compiler_axes(&font.axes)
                    .map_err(|e| IrError::InvalidEntry("axes", e.to_string()))?
            } else {
                Vec::new()
            },
            if self.0.variable {
                font.instances
                    .iter()
                    .filter(|i| i.active && i.is_variable_named_instance)
                    .map(|i| NamedInstance {
                        name: i.name.clone(),
                        postscript_name: None,
                        location: font
                            .axes
                            .iter()
                            .map(|a| {
                                (
                                    fontdrasil::types::Tag::new(&a.tag.0),
                                    UserCoord::new(
                                        a.design_to_user(
                                            i.location
                                                .get(&a.tag)
                                                .copied()
                                                .unwrap_or_else(|| a.user_to_design(a.default)),
                                        ),
                                    ),
                                )
                            })
                            .collect(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            self.0
                .master_locations()
                .into_iter()
                .map(|(_, loc)| loc)
                .collect(),
            postscript_names,
            master.metrics.italic_angle,
            None,
            build_vertical,
        )?;
        apply_misc_metadata(&mut metadata, &font.info);
        metadata.variations = self.0.variations.clone();
        // Use fontc's explicit Skip policy: a manual feature without a marker
        // owns its tag, and `# Automatic Code` inserts model rules only when
        // this document explicitly permits that writer. Conversion can keep
        // model anchors while disabling all competing automatic layout rules.
        let policy = &font.feature_settings.writer_policy;
        let writers = [
            (fontir::ir::KnownFeatureWriter::Kern, policy.kerning),
            (fontir::ir::KnownFeatureWriter::Mark, policy.anchors),
            (fontir::ir::KnownFeatureWriter::Curs, policy.anchors),
            (fontir::ir::KnownFeatureWriter::Gdef, policy.anchors),
        ];
        metadata.misc.feature_generation = Some(
            writers
                .into_iter()
                .filter_map(|(writer, enabled)| {
                    enabled.then_some(fontir::ir::FeatureWriterSpec {
                        writer,
                        mode: fontir::ir::FeatureWriterMode::Skip,
                        features: None,
                    })
                })
                .collect(),
        );
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
struct GlyphWork {
    glyph: fc_model::Glyph,
    layers: Vec<(NormalizedLocation, fc_model::Layer)>,
    model_anchors: bool,
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
        let mut instances = HashMap::new();
        let mut anchors = AnchorBuilder::new(name.clone());
        for (location, layer) in &self.layers {
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
            instances.insert(location.clone(), instance);
            for anchor in layer.anchors.iter().filter(|_| self.model_anchors) {
                anchors.add(
                    anchor.name.clone().into(),
                    location.clone(),
                    Point::new(anchor.x, anchor.y),
                )?;
            }
        }
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
        let glyph = IrGlyph::new(name.clone(), self.glyph.export, codepoints, instances)?;
        context.glyphs.set(glyph);

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

fn feature_source(data: &SourceData) -> Result<IrFeaturesSource, IrError> {
    let source = crate::vertical::feature_source(&data.font, &data.master, data.variable)
        .map_err(|error| IrError::InvalidEntry("vertical kerning", error.to_string()))?;
    if source.trim().is_empty() {
        Ok(IrFeaturesSource::Empty)
    } else {
        Ok(IrFeaturesSource::from_string(source))
    }
}

#[derive(Debug)]
struct KerningLocationsWork(BTreeSet<NormalizedLocation>);

impl Work<Context, WorkId, IrError> for KerningLocationsWork {
    fn id(&self) -> WorkId {
        WorkId::KerningLocations
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        context.kerning_locations.set(KerningLocations {
            locations: self.0.clone(),
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
        let master = self
            .data
            .master_locations()
            .into_iter()
            .find(|(_, loc)| *loc == self.location)
            .map(|(m, _)| m)
            .ok_or(IrError::NoDefaultMaster)?;
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
