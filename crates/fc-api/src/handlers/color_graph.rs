//! Compile a bounded, isolated font for native browser COLRv1 rendering.
use crate::{Api, ApiError, ColorGraphPreview, ColorGraphPreviewParams};
use fc_model::{ColrPaint, Contour, Font, Glyph, GlyphName, Layer};
use std::collections::HashSet;

fn invalid(error: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(error.to_string())
}

pub(crate) fn preview(
    api: &Api,
    params: ColorGraphPreviewParams,
) -> Result<ColorGraphPreview, ApiError> {
    let (generation, font) = api
        .session
        .with_doc(params.doc, |doc| {
            subset(&doc.font, params).map(|font| (doc.generation, font))
        })
        .map_err(ApiError::from)??;
    let report = fc_compile::export(
        &font,
        &|_| 1,
        &fc_compile::ExportOptions {
            preserve_untouched_tables: false,
            keep_tt_instructions: false,
            ..Default::default()
        },
    )
    .map_err(invalid)?;
    if report.bytes.len() > 8 * 1024 * 1024 {
        return Err(invalid("COLRv1 preview exceeds 8 MiB"));
    }
    Ok(ColorGraphPreview {
        generation,
        bytes: report.bytes,
    })
}

fn subset(source: &Font, params: ColorGraphPreviewParams) -> Result<Font, ApiError> {
    let source_master = params
        .master
        .as_ref()
        .unwrap_or(&source.default_master().id);
    if source.master(source_master).is_none() {
        return Err(ApiError::not_found("preview master does not exist"));
    }
    let mut color = params.color;
    color
        .validate(&source.glyph_order().cloned().collect::<Vec<_>>())
        .map_err(invalid)?;
    let palette = color
        .palettes
        .get(usize::from(params.palette_index))
        .cloned()
        .ok_or_else(|| invalid("preview palette does not exist"))?;
    let graph = color
        .graph
        .as_mut()
        .ok_or_else(|| invalid("no COLRv1 graph"))?;
    let root = *graph
        .glyphs
        .get(params.glyph.as_str())
        .ok_or_else(|| invalid("preview root does not exist"))?;
    let mut roots: HashSet<GlyphName> = [params.glyph.clone().into()].into();
    let mut names = roots.clone();
    let mut seen = HashSet::new();
    let mut pending = vec![root];
    while let Some(index) = pending.pop() {
        if !seen.insert(index) {
            continue;
        }
        if seen.len() > 2048 {
            return Err(invalid(
                "COLRv1 preview supports at most 2048 reachable paints",
            ));
        }
        let paint = &graph.paints[index as usize];
        match paint {
            ColrPaint::Glyph { glyph, .. } => {
                names.insert(glyph.clone());
            }
            ColrPaint::ColrGlyph { glyph } => {
                names.insert(glyph.clone());
                roots.insert(glyph.clone());
                pending.push(graph.glyphs[glyph]);
            }
            _ => {}
        }
        pending.extend(paint.children());
    }
    if names.len() > 1024 {
        return Err(invalid("COLRv1 preview supports at most 1024 outlines"));
    }
    graph.glyphs.retain(|name, _| roots.contains(name));
    graph.retain_glyphs(|name| names.contains(name));
    color.glyphs.clear();
    color.palette_version = 0;
    color.entry_labels.clear();
    color.palettes = vec![fc_model::ColorPalette {
        colors: palette.colors,
        ..Default::default()
    }];
    let mut font = Font::new_empty("FontCarver Color Preview", source.info.units_per_em);
    let master = font.default_master().id.clone();
    let mut points = 0usize;
    for name in names {
        let layer = source
            .glyph(&name)
            .and_then(|g| g.master_layer(source_master))
            .ok_or_else(|| invalid("preview outline has no selected-master layer"))?;
        let path = fc_geom::flatten_layer(source, &name, source_master).map_err(invalid)?;
        points = points.saturating_add(path.elements().len());
        if points > 100_000 {
            return Err(invalid("COLRv1 preview outline work limit"));
        }
        let mut outline = Layer::new_master(master.clone());
        outline.width = layer.width;
        outline.contours = Contour::from_bezpath(&path);
        let mut glyph = Glyph::new(name.clone());
        glyph.layers = vec![outline];
        if name.as_ref() == params.glyph {
            glyph.unicodes = vec![0xE000];
        }
        font.insert_glyph(glyph).map_err(invalid)?;
    }
    font.color = Some(color);
    font.color_tables_owned = true;
    Ok(font)
}
