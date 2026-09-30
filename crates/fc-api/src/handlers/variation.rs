use crate::{
    Api, ApiError, Change, DocParams, VariationApplyParams, VariationPreview,
    VariationPreviewParams, VariationShape, VariationShapeStatus, VariationSourcePreview,
    VariationState,
};

pub(crate) fn state(api: &Api, params: DocParams) -> Result<VariationState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| VariationState {
            generation: doc.generation,
            axes: doc.font.axes.clone(),
            masters: doc.font.masters.clone(),
            instances: doc.font.instances.clone(),
            virtual_masters: Some(doc.font.virtual_masters.clone()),
            first_glyph: doc
                .font
                .glyph_order()
                .find(|name| name.as_ref() != ".notdef")
                .map(ToString::to_string),
        })
        .map_err(ApiError::from)
}

pub(crate) fn apply(api: &Api, params: VariationApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "design space changed; reload before applying your edits",
                ));
            }
            doc.apply(fc_doc::Command::DesignSpace(params.command), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(
    api: &Api,
    params: VariationPreviewParams,
) -> Result<VariationPreview, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let glyph = font
                .glyph(&params.glyph)
                .ok_or_else(|| ApiError::not_found("preview glyph does not exist"))?;
            if params.location.len() > 16
                || params
                    .location
                    .keys()
                    .any(|tag| !font.axes.iter().any(|a| a.tag.to_string() == *tag))
            {
                return Err(ApiError::invalid_params("preview contains an unknown axis"));
            }
            let mut location = fc_model::DesignLocation::new();
            for axis in &font.axes {
                let user = params
                    .location
                    .get(&axis.tag.to_string())
                    .copied()
                    .unwrap_or(axis.default);
                if !user.is_finite() || user < axis.min || user > axis.max {
                    return Err(ApiError::invalid_params(
                        "preview value is outside the axis range",
                    ));
                }
                location.insert(axis.tag, axis.user_to_design(user));
            }
            let mut result = VariationPreview {
                generation: doc.generation,
                path: String::new(),
                width: 0.0,
                vertical_width: None,
                vertical_origin: None,
                units_per_em: font.info.units_per_em,
                sources: Vec::new(),
                shape_status: Vec::new(),
                issues: Vec::new(),
            };
            let sources = fc_geom::variation::glyph_sources(font, glyph)
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
            if sources.len() > 1024 {
                return Err(ApiError::invalid_params("preview exceeds 1024 sources"));
            }
            let reference = font
                .design_default_master()
                .ok()
                .and_then(|master| glyph.master_layer(&master.id));
            let mut preview_elements = 0usize;
            let mut source_shapes: Vec<Vec<fc_geom::SourceShape>> = Vec::new();
            for (_, layer) in sources {
                let name = match &layer.kind {
                    fc_model::LayerKind::Master(id) => font
                        .master(id)
                        .map_or_else(|| id.to_string(), |m| m.name.clone()),
                    _ => {
                        if layer.name.is_empty() {
                            layer.id.to_string()
                        } else {
                            layer.name.clone()
                        }
                    }
                };
                if let Some(reference) = reference {
                    if let Err(issues) = fc_geom::compatible(reference, layer) {
                        for issue in issues
                            .into_iter()
                            .take(64usize.saturating_sub(result.issues.len()))
                        {
                            result.issues.push(format!("{name}: {}", issue.reason));
                        }
                    }
                }
                if result.sources.len() < 16 {
                    match fc_geom::flatten_layer_by_id(font, &params.glyph, &layer.id) {
                        Ok(path) => {
                            preview_elements =
                                preview_elements.saturating_add(path.elements().len());
                            if preview_elements <= 100_000 {
                                // Shape-order data is what the compatibility
                                // view colours, numbers and reorders.
                                let shapes = match fc_geom::source_shapes(font, layer) {
                                    Ok(shapes) => shapes,
                                    Err(error) => {
                                        if result.issues.len() < 64 {
                                            result.issues.push(format!("{name}: {error}"));
                                        }
                                        Vec::new()
                                    }
                                };
                                source_shapes.push(shapes.clone());
                                result.sources.push(VariationSourcePreview {
                                    name,
                                    layer: layer.id.to_string(),
                                    path: path.to_svg(),
                                    shapes: shapes.into_iter().map(describe_shape).collect(),
                                });
                            } else if result.issues.len() < 64 {
                                result
                                    .issues
                                    .push("source overlays exceed the preview work limit".into());
                            }
                        }
                        Err(error) if result.issues.len() < 64 => {
                            result.issues.push(format!("{name}: {error}"))
                        }
                        Err(_) => {}
                    }
                }
            }
            result.shape_status = fc_geom::compare_shapes(&source_shapes)
                .into_iter()
                .map(|comparison| VariationShapeStatus {
                    index: u32::try_from(comparison.index).unwrap_or(u32::MAX),
                    component: comparison.component.map(|name| name.to_string()),
                    status: comparison.status.as_str().to_owned(),
                    segments: comparison
                        .segments
                        .into_iter()
                        .map(|status| status.as_str().to_owned())
                        .collect(),
                    max_angle: comparison.max_angle,
                })
                .collect();
            match fc_geom::variation::interpolate_glyph(font, glyph, &location) {
                Ok(layer) => {
                    result.width = layer.width;
                    result.vertical_width = layer.vertical_width;
                    result.vertical_origin = layer.vertical_origin;
                    match fc_geom::variation::interpolated_path(font, &params.glyph, &location) {
                        Ok(path) => result.path = path.to_svg(),
                        Err(error) => result.issues.push(error.to_string()),
                    }
                }
                Err(error) => result.issues.push(error.to_string()),
            }
            Ok(result)
        })
        .map_err(ApiError::from)?
}

/// Converts one analysed shape into its API payload.
fn describe_shape(shape: fc_geom::SourceShape) -> VariationShape {
    VariationShape {
        index: u32::try_from(shape.index).unwrap_or(u32::MAX),
        component: shape.component.map(|name| name.to_string()),
        closed: shape.closed,
        path: shape.path.to_svg(),
        start: [shape.start.x, shape.start.y],
        center: [shape.center.x, shape.center.y],
        points: shape
            .points
            .into_iter()
            .map(|point| [point.x, point.y])
            .collect(),
        on_curve: shape.on_curve,
    }
}

/// Largest number of stored points one units-per-em change will rewrite.
const MAX_RESCALE_POINTS: usize = 8_000_000;

/// Changes units per em, scaling the selected groups as one undo entry.
///
/// The operation is all or nothing: it refuses before touching the document when
/// the source carries data whose scaling this operation does not model, so a
/// rescaled font never mixes coordinate systems silently.
pub(crate) fn rescale_upm(
    api: &Api,
    params: crate::RescaleUpmParams,
) -> Result<crate::RescaleUpmResult, ApiError> {
    if !(16..=16_384).contains(&params.units_per_em) {
        return Err(ApiError::invalid_params(
            "units per em must be between 16 and 16384",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "the document changed; reload before changing units per em",
                ));
            }
            let font = doc.font.clone();
            let previous = font.info.units_per_em;
            if previous == 0 {
                return Err(ApiError::invalid_params(
                    "this document has no units per em to scale from",
                ));
            }
            let factor = f64::from(params.units_per_em) / f64::from(previous);
            let scaling = params.scale_outlines
                || params.scale_metrics
                || params.scale_kerning
                || params.scale_guides;
            if scaling {
                if let Some(blocker) = rescale_blocker(&font) {
                    return Err(ApiError::invalid_params(blocker));
                }
            }
            let mut commands = Vec::new();
            let mut result = crate::RescaleUpmResult {
                factor,
                glyphs: 0,
                layers: 0,
                cleared_programs: 0,
                warnings: Vec::new(),
                change: Change::default(),
            };
            let mut info = font.info.clone();
            info.units_per_em = params.units_per_em;
            if params.scale_metrics {
                for number in [
                    &mut info.open_type_os2_subscript_x_size,
                    &mut info.open_type_os2_subscript_y_size,
                    &mut info.open_type_os2_subscript_x_offset,
                    &mut info.open_type_os2_subscript_y_offset,
                    &mut info.open_type_os2_superscript_x_size,
                    &mut info.open_type_os2_superscript_y_size,
                    &mut info.open_type_os2_superscript_x_offset,
                    &mut info.open_type_os2_superscript_y_offset,
                    &mut info.open_type_os2_strikeout_size,
                    &mut info.open_type_os2_strikeout_position,
                    &mut info.postscript_underline_position,
                    &mut info.postscript_underline_thickness,
                    &mut info.postscript_default_width_x,
                    &mut info.postscript_nominal_width_x,
                ]
                .into_iter()
                .flatten()
                {
                    *number *= factor;
                }
            }
            commands.push(fc_doc::Command::SetFontInfo {
                info: Box::new(info),
            });
            if params.scale_metrics || params.scale_kerning || params.scale_guides {
                for master in &font.masters {
                    let mut replacement = master.clone();
                    if params.scale_metrics {
                        scale_master_metrics(&mut replacement.metrics, factor);
                        for zone in &mut replacement.alignment_zones {
                            zone.position *= factor;
                            zone.size *= factor;
                        }
                        for stem in &mut replacement.stems {
                            stem.width *= factor;
                        }
                        scale_private(&mut replacement.private_dict, factor);
                        for private in &mut replacement.cff_private_dicts {
                            scale_private(private, factor);
                        }
                    }
                    if params.scale_guides {
                        for guide in &mut replacement.guides {
                            guide.x *= factor;
                            guide.y *= factor;
                        }
                    }
                    if params.scale_kerning {
                        for value in replacement.kerning_h.0.values_mut() {
                            *value *= factor;
                        }
                        for value in replacement.kerning_v.0.values_mut() {
                            *value *= factor;
                        }
                    }
                    commands.push(fc_doc::Command::SetMaster {
                        master: replacement,
                    });
                }
            }
            if params.scale_guides && !font.guides.is_empty() {
                let guides = font
                    .guides
                    .iter()
                    .map(|guide| {
                        let mut guide = guide.clone();
                        guide.x *= factor;
                        guide.y *= factor;
                        guide
                    })
                    .collect();
                commands.push(fc_doc::Command::SetFontGuides { guides });
            }
            if params.scale_outlines || params.scale_metrics {
                let mut points = 0usize;
                for glyph in font.glyphs.values() {
                    let mut replacement = glyph.clone();
                    let mut rewritten = false;
                    if params.scale_outlines {
                        for layer in &mut replacement.layers {
                            points = points.saturating_add(
                                layer
                                    .contours
                                    .iter()
                                    .map(|contour| contour.points.len())
                                    .sum::<usize>(),
                            );
                            if points > MAX_RESCALE_POINTS {
                                return Err(ApiError::invalid_params(
                                    "this font exceeds the scaling work limit; scale it in the command line instead",
                                ));
                            }
                            if scale_layer(layer, factor, params.scale_guides) {
                                result.cleared_programs += 1;
                            }
                            result.layers += 1;
                        }
                        rewritten = true;
                    }
                    if params.scale_metrics {
                        // Metrics formulas hold font units in both scopes, so a
                        // rescale that leaves them alone would silently keep the
                        // previous sidebearings.
                        let mut keys = vec![&mut replacement.metrics_keys];
                        keys.extend(
                            replacement
                                .layers
                                .iter_mut()
                                .filter_map(|layer| layer.metrics_keys.as_mut()),
                        );
                        for record in keys {
                            match scale_metrics_keys(record, factor) {
                                Ok(present) => rewritten |= present,
                                Err(key) => {
                                    return Err(ApiError::invalid_params(format!(
                                        "{}: metrics formula {key} is not a form this operation can rescale; resolve it before changing units per em",
                                        glyph.name
                                    )))
                                }
                            }
                        }
                    }
                    if rewritten {
                        result.glyphs += 1;
                        commands.push(fc_doc::Command::ReplaceGlyph {
                            glyph: replacement,
                        });
                    }
                }
            }
            if !font.extra.raw_tables.is_empty() || font.extra.source.is_some() {
                result.warnings.push(
                    "Preserved imported tables still describe the previous units per em; turn off sfnt table preservation when exporting this font."
                        .to_owned(),
                );
            }
            if !font.bitmap_strikes.is_empty() {
                result.warnings.push(
                    "Embedded bitmap strikes are stored in pixels and were not rescaled."
                        .to_owned(),
                );
            }
            result.change = doc
                .apply(fc_doc::Command::Batch(commands), params.label.clone())
                .map_err(ApiError::from)?;
            Ok(result)
        })
        .map_err(ApiError::from)?
}

/// Why a units-per-em change cannot scale this source, if it cannot.
fn rescale_blocker(font: &fc_model::Font) -> Option<String> {
    if font.math.is_some() {
        return Some(
            "editable MATH values are stored in font units; remove or re-author them before changing units per em"
                .to_owned(),
        );
    }
    if font.svg.is_some() {
        return Some(
            "editable SVG documents are stored in font units; remove them before changing units per em"
                .to_owned(),
        );
    }
    for glyph in font.glyphs.values() {
        for layer in &glyph.layers {
            let blocker = if !layer.effects.is_empty() {
                Some("layer effects")
            } else if !layer.constructions.is_empty() {
                Some("reusable constructions")
            } else if !layer.node_links.is_empty() {
                Some("linked nodes")
            } else if !layer.linked_guides.is_empty() {
                Some("guide-owned nodes")
            } else if layer
                .color_layers
                .as_ref()
                .is_some_and(|layers| !layers.is_empty())
            {
                Some("color outlines")
            } else {
                None
            };
            if let Some(blocker) = blocker {
                return Some(format!(
                    "{} carries {blocker}, whose scaling this operation does not model; bake or remove them before changing units per em",
                    glyph.name
                ));
            }
        }
    }
    None
}

/// Scales one layer in place; returns whether a TrueType program was cleared.
fn scale_layer(layer: &mut fc_model::Layer, factor: f64, guides: bool) -> bool {
    layer.width *= factor;
    if let Some(value) = &mut layer.vertical_width {
        *value *= factor;
    }
    if let Some(value) = &mut layer.vertical_origin {
        *value *= factor;
    }
    for point in layer
        .contours
        .iter_mut()
        .flat_map(|contour| &mut contour.points)
    {
        point.x *= factor;
        point.y *= factor;
        point.corner_radius *= factor;
    }
    for component in &mut layer.components {
        let mut coefficients = component.transform.as_coeffs();
        coefficients[4] *= factor;
        coefficients[5] *= factor;
        component.transform = kurbo::Affine::new(coefficients);
    }
    for anchor in &mut layer.anchors {
        anchor.x *= factor;
        anchor.y *= factor;
    }
    if guides {
        for guide in &mut layer.guides {
            guide.x *= factor;
            guide.y *= factor;
        }
    }
    for stem in layer
        .hints
        .h_stems
        .iter_mut()
        .chain(&mut layer.hints.v_stems)
    {
        stem.start *= factor;
        // Type 1 ghost hints use the reserved widths -20 and -21.
        if stem.width != -20.0 && stem.width != -21.0 {
            stem.width *= factor;
        }
        for range in &mut stem.ranges {
            range.0 *= factor;
            range.1 *= factor;
        }
    }
    for diagonal in &mut layer.hints.d_stems {
        diagonal.left.0 *= factor;
        diagonal.left.1 *= factor;
        diagonal.right.0 *= factor;
        diagonal.right.1 *= factor;
    }
    if let Some(image) = &mut layer.background_image {
        let mut coefficients = image.transform.as_coeffs();
        coefficients[4] *= factor;
        coefficients[5] *= factor;
        image.transform = kurbo::Affine::new(coefficients);
    }
    // A TrueType program describes the previous grid and its control values.
    let cleared = layer.tt_instructions.is_some() || layer.tt_binding.is_some();
    layer.tt_instructions = None;
    layer.tt_binding = None;
    cleared
}

/// Scales the master metrics that are measured in font units.
fn scale_master_metrics(metrics: &mut fc_model::MasterMetrics, factor: f64) {
    metrics.ascender *= factor;
    metrics.descender *= factor;
    metrics.x_height *= factor;
    metrics.cap_height *= factor;
    for number in [
        &mut metrics.hhea_ascender,
        &mut metrics.hhea_descender,
        &mut metrics.hhea_line_gap,
        &mut metrics.typo_ascender,
        &mut metrics.typo_descender,
        &mut metrics.typo_line_gap,
        &mut metrics.win_ascent,
        &mut metrics.win_descent,
        &mut metrics.vhea_ascender,
        &mut metrics.vhea_descender,
        &mut metrics.vhea_line_gap,
        &mut metrics.default_vertical_origin,
        &mut metrics.default_vertical_advance,
    ]
    .into_iter()
    .flatten()
    {
        *number *= factor;
    }
}

/// Scales the PostScript private values that are measured in font units.
fn scale_private(private: &mut fc_model::PsPrivate, factor: f64) {
    for value in private
        .blue_values
        .iter_mut()
        .chain(&mut private.other_blues)
        .chain(&mut private.family_blues)
        .chain(&mut private.family_other_blues)
        .chain(&mut private.stem_snap_h)
        .chain(&mut private.stem_snap_v)
    {
        *value *= factor;
    }
    for number in [
        &mut private.blue_shift,
        &mut private.blue_fuzz,
        &mut private.std_hw,
        &mut private.std_vw,
    ]
    .into_iter()
    .flatten()
    {
        *number *= factor;
    }
}

/// Namespaced collection holding the project's editor settings.
const EDITOR_SETTINGS: &str = "editor_settings";

/// Reads the per-font editor settings and the values they resolve to.
pub(crate) fn editor_settings_get(
    api: &Api,
    params: DocParams,
) -> Result<crate::EditorSettingsView, ApiError> {
    let prefs = super::prefs::get(api, crate::EmptyParams {})?;
    api.session
        .with_doc(params.doc, |doc| {
            resolve_editor_settings(read_editor_settings(&doc.font.info.user_data)?, &prefs)
        })
        .map_err(ApiError::from)?
}

/// Replaces the per-font editor settings as one ordinary undo entry.
pub(crate) fn editor_settings_set(
    api: &Api,
    params: crate::SetEditorSettingsParams,
) -> Result<crate::EditorSettingsView, ApiError> {
    let settings = validate_editor_settings(params.settings)?;
    let prefs = super::prefs::get(api, crate::EmptyParams {})?;
    let value =
        serde_json::to_value(settings).map_err(|error| ApiError::internal(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |doc| {
            // A malformed stored value must not be replaced silently.
            read_editor_settings(&doc.font.info.user_data)?;
            super::inventory::write_collection(
                doc,
                EDITOR_SETTINGS,
                value,
                "Edit font editor settings",
            )?;
            resolve_editor_settings(settings, &prefs)
        })
        .map_err(ApiError::from)?
}

fn read_editor_settings(
    user_data: &serde_json::Value,
) -> Result<crate::FontEditorSettings, ApiError> {
    let Some(value) = super::inventory::read_collection(user_data, EDITOR_SETTINGS)? else {
        return Ok(crate::FontEditorSettings::default());
    };
    let settings =
        <crate::FontEditorSettings as serde::Deserialize>::deserialize(value).map_err(|error| {
            ApiError::invalid_params(format!("invalid stored editor settings: {error}"))
        })?;
    validate_editor_settings(settings)
}

fn validate_editor_settings(
    settings: crate::FontEditorSettings,
) -> Result<crate::FontEditorSettings, ApiError> {
    if settings
        .grid_size
        .is_some_and(|size| !size.is_finite() || !(0.0..=10_000.0).contains(&size))
    {
        return Err(ApiError::invalid_params(
            "grid size must be between 0 and 10000 font units",
        ));
    }
    if settings
        .grid_subdivision
        .is_some_and(|value| !(1..=100).contains(&value))
    {
        return Err(ApiError::invalid_params(
            "grid subdivision must be between 1 and 100",
        ));
    }
    if let Some(nudge) = settings.nudge {
        if nudge
            .iter()
            .any(|step| !step.is_finite() || !(0.001..=10_000.0).contains(step))
        {
            return Err(ApiError::invalid_params(
                "keyboard increments must be between 0.001 and 10000 font units",
            ));
        }
        if nudge[0] > nudge[1] || nudge[1] > nudge[2] {
            return Err(ApiError::invalid_params(
                "keyboard increments must not decrease from the plain to the command step",
            ));
        }
    }
    Ok(settings)
}

fn resolve_editor_settings(
    settings: crate::FontEditorSettings,
    prefs: &crate::Prefs,
) -> Result<crate::EditorSettingsView, ApiError> {
    Ok(crate::EditorSettingsView {
        stored: settings,
        grid_size: settings.grid_size.unwrap_or(prefs.grid_size),
        grid_subdivision: settings.grid_subdivision.unwrap_or(prefs.grid_subdivision),
        nudge: settings.nudge.unwrap_or(prefs.nudge),
    })
}

/// Rewrite the font-unit numbers inside one metrics formula.
///
/// The grammar is `fc-doc`'s: `=50` is a constant, `=+25` offsets an automatic
/// metric, `=H+10` adjusts a reference, `=H@200` measures at a height, `=H/2`
/// and `=H*2` are ratios, `=|H` mirrors and `==H` is layer-local. Constants,
/// offsets, additive adjustments and measurement heights are font units and
/// scale; ratios and glyph names do not. `None` means the formula did not match
/// the grammar, which the caller reports instead of rewriting blindly.
fn scale_metrics_key(key: &str, factor: f64) -> Option<String> {
    let trimmed = key.trim();
    let local = trimmed.strip_prefix('=')?.starts_with('=');
    let prefix = if local { "==" } else { "=" };
    let body = trimmed.strip_prefix('=')?;
    let body = if local {
        body.strip_prefix('=')?.trim()
    } else {
        body.trim()
    };
    let number = |value: f64| -> Option<String> {
        let scaled = value * factor;
        scaled.is_finite().then(|| format!("{scaled}"))
    };
    if body.starts_with(['+', '-']) {
        let value = body.parse::<f64>().ok()?;
        let scaled = value * factor;
        return scaled
            .is_finite()
            .then(|| format!("{prefix}{}{}", if scaled < 0.0 { "" } else { "+" }, scaled));
    }
    if let Ok(value) = body.parse::<f64>() {
        return Some(format!("{prefix}{}", number(value)?));
    }
    let (head, height) = match body.rsplit_once('@') {
        Some((head, height)) => (head, Some(number(height.parse::<f64>().ok()?)?)),
        None => (body, None),
    };
    // `split_operation` takes the last operator whose suffix parses as a number.
    let mut rebuilt = head.to_owned();
    for (index, character) in head.char_indices().rev() {
        if index == 0 || !matches!(character, '+' | '-' | '*' | '/') {
            continue;
        }
        let Ok(value) = head[index + character.len_utf8()..].parse::<f64>() else {
            continue;
        };
        rebuilt = match character {
            // Only additive adjustments are measured in font units.
            '+' | '-' => format!("{}{character}{}", &head[..index], number(value)?),
            _ => head.to_owned(),
        };
        break;
    }
    Some(match height {
        Some(height) => format!("{prefix}{rebuilt}@{height}"),
        None => format!("{prefix}{rebuilt}"),
    })
}

/// Rewrites every formula of one metrics record.
///
/// `Ok(true)` means at least one formula was present, so the glyph has to be
/// rewritten; the error names the first formula this operation cannot rescale.
fn scale_metrics_keys(keys: &mut fc_model::MetricsKeys, factor: f64) -> Result<bool, String> {
    let mut present = false;
    for field in [
        &mut keys.left,
        &mut keys.right,
        &mut keys.width,
        &mut keys.vertical_origin,
        &mut keys.vertical_width,
    ]
    .into_iter()
    .flatten()
    {
        present = true;
        match scale_metrics_key(field, factor) {
            Some(scaled) => *field = scaled,
            None => return Err(field.clone()),
        }
    }
    Ok(present)
}

#[cfg(test)]
mod metrics_key_tests {
    use super::scale_metrics_key;

    #[test]
    fn scales_only_the_font_unit_numbers() {
        for (key, expected) in [
            ("=50", "=100"),
            ("=+25", "=+50"),
            ("=-25", "=-50"),
            ("=H", "=H"),
            ("=|H", "=|H"),
            ("=H+10", "=H+20"),
            ("=H-10", "=H-20"),
            ("=H@200", "=H@400"),
            ("=H+10@200", "=H+20@400"),
            ("==H+10", "==H+20"),
            // Ratios are dimensionless and must not change.
            ("=H/2", "=H/2"),
            ("=H*1.5", "=H*1.5"),
            ("=|H/2@100", "=|H/2@200"),
        ] {
            assert_eq!(
                scale_metrics_key(key, 2.0).as_deref(),
                Some(expected),
                "{key}"
            );
        }
    }

    #[test]
    fn reports_a_formula_it_cannot_rewrite() {
        assert_eq!(scale_metrics_key("H", 2.0), None);
        assert_eq!(scale_metrics_key("=H@nonsense", 2.0), None);
        assert_eq!(scale_metrics_key("=+nonsense", 2.0), None);
    }
}
