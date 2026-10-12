//! Database-driven metadata changes preserve all source geometry.
use fc_glyphdata::{explicit_unicodes, parse_overlay, resolve_info, GlyphDataOverlay};
use fc_model::{Anchor, Component, Font, FontInfo, Glyph, GlyphName, Layer};
use kurbo::Affine;
use std::collections::{BTreeMap, BTreeSet};

use crate::Error;

/// Font-info user-data key for the document's versioned glyph database overlay.
pub const GLYPH_DATA_OVERLAY_KEY: &str = "com.fontcarver.glyphData";

/// Read and validate the persisted document overlay without changing the model.
pub fn glyph_data_overlay(info: &FontInfo) -> Result<GlyphDataOverlay, Error> {
    match info.user_data.get(GLYPH_DATA_OVERLAY_KEY) {
        Some(value) => parse_overlay(&value.to_string()).map_err(Error::InvalidCommand),
        None => parse_overlay(r#"{"version":1,"glyphs":{}}"#).map_err(Error::InvalidCommand),
    }
}

pub(super) fn update(font: &mut Font, glyphs: &[GlyphName]) -> Result<(), Error> {
    if glyphs.is_empty() || glyphs.len() > 65_536 {
        return Err(Error::InvalidCommand(
            "Update Glyph Info requires between one and 65536 glyphs".into(),
        ));
    }
    let overlay = glyph_data_overlay(&font.info)?;
    let mut changes = Vec::new();
    let mut seen = BTreeSet::new();
    for name in glyphs {
        if !seen.insert(name) {
            continue;
        }
        let glyph = font
            .glyph(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        if glyph.locked {
            return Err(Error::InvalidCommand(format!("{name} is locked")));
        }
        changes.push((
            name,
            resolve_info(name.as_ref(), &glyph.unicodes, &overlay),
            explicit_unicodes(name.as_ref(), &overlay),
        ));
    }
    for (name, info, encodings) in changes {
        let glyph = font
            .glyph_mut(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        // Encodings are the one field a refresh may not invent or drop. The
        // database only speaks for a name it lists exactly, so a suffixed or
        // private-use glyph keeps its own cmap entry, and a name the database
        // does not know keeps whatever the font already says. An explicitly
        // empty record is a statement about the database, not an instruction to
        // unencode a glyph, so it is ignored here too.
        if let Some(encodings) = encodings.filter(|values| !values.is_empty()) {
            glyph.unicodes = encodings;
        }
        glyph.category = info.category;
        glyph.sub_category = info.sub_category;
        glyph.script = info.script;
        glyph.production_name = info.production_name;
    }
    Ok(())
}

/// What building one glyph from the database composition produced.
#[derive(Clone, Debug, PartialEq)]
pub struct ComposedGlyph {
    /// The glyph to insert, with one layer per master.
    pub glyph: Glyph,
    /// Component base names used, in composition order; empty for a blank glyph.
    pub components: Vec<GlyphName>,
    /// Composition members the font does not contain, so the glyph stays blank.
    pub unavailable: Vec<GlyphName>,
}

/// Builds a new glyph, using the glyph database's composition when it is usable.
///
/// Each member is placed by one rule. A member whose layer has an entry anchor
/// (`_top`) matching an anchor already on the layer being built is a mark: it is
/// offset so the two coincide, and it does not move the pen. Every other member
/// is set at the pen and advances it by its own width, which composes `f_f` as a
/// ligature and `Aacute` as a base plus an attached accent. Marks stack, so an
/// attached mark's own `top` replaces the base's for the next member, and the
/// finished layer's advance is the pen. A member the font does not contain
/// leaves the glyph blank and is reported in `unavailable`, so a caller can tell
/// "generated empty" from "generated a composite". Anchors are copied from real
/// layers, never invented: the database names anchors but holds no coordinates.
///
/// `staged` holds glyphs an enclosing batch is about to insert, so one recipe can
/// create `A` and then build `Aacute` from it.
pub fn composed_glyph(
    font: &Font,
    staged: &BTreeMap<GlyphName, Glyph>,
    name: &GlyphName,
    unicodes: Vec<u32>,
    overlay: &GlyphDataOverlay,
) -> ComposedGlyph {
    let mut glyph = Glyph::new(name.clone());
    glyph.unicodes = unicodes;
    let info = resolve_info(name.as_ref(), &glyph.unicodes, overlay);
    let members: Vec<GlyphName> = info
        .composition
        .iter()
        .map(|member| GlyphName::from(member.as_str()))
        .collect();
    let available = |member: &GlyphName| staged.get(member).or_else(|| font.glyph(member));
    let unavailable: Vec<GlyphName> = members
        .iter()
        .filter(|member| available(member).is_none() || *member == name)
        .cloned()
        .collect();
    if members.is_empty() || !unavailable.is_empty() {
        for master in &font.masters {
            glyph.layers.push(Layer::new_master(master.id.clone()));
        }
        return ComposedGlyph {
            glyph,
            components: Vec::new(),
            unavailable,
        };
    }
    for master in &font.masters {
        let mut layer = Layer::new_master(master.id.clone());
        let mut anchors: Vec<Anchor> = Vec::new();
        let mut pen = 0.0;
        for member in &members {
            let source = available(member).and_then(|member| member.master_layer(&master.id));
            let (offset, matched) = match attachment(source, &anchors) {
                Some((offset, name)) => (offset, Some(name)),
                None => ((pen, 0.0), None),
            };
            layer.components.push(Component {
                base: member.clone(),
                transform: Affine::translate(offset),
                anchor: matched.clone(),
                auto_align: matched.is_some(),
                ..Component::default()
            });
            if let Some(source) = source {
                if matched.is_none() {
                    pen += source.width;
                }
                merge_anchors(&mut anchors, source, offset);
            }
        }
        layer.width = pen;
        layer.anchors = anchors;
        glyph.layers.push(layer);
    }
    ComposedGlyph {
        glyph,
        components: members,
        unavailable,
    }
}

/// Finds the offset that puts a mark's entry anchor on the matching base anchor.
///
/// `None` means this member is not a mark of what has been built so far, and the
/// caller places it at the pen instead.
fn attachment(source: Option<&Layer>, anchors: &[Anchor]) -> Option<((f64, f64), String)> {
    let source = source?;
    source.anchors.iter().find_map(|entry| {
        let target = entry.name.strip_prefix('_')?;
        let base = anchors.iter().find(|anchor| anchor.name == target)?;
        Some(((base.x - entry.x, base.y - entry.y), target.to_owned()))
    })
}

/// Adds a member's base anchors at their placed positions, newest winning.
fn merge_anchors(anchors: &mut Vec<Anchor>, source: &Layer, offset: (f64, f64)) {
    for anchor in &source.anchors {
        if anchor.name.starts_with('_') {
            continue;
        }
        let placed = Anchor {
            name: anchor.name.clone(),
            x: anchor.x + offset.0,
            y: anchor.y + offset.1,
        };
        match anchors
            .iter_mut()
            .find(|existing| existing.name == placed.name)
        {
            Some(existing) => *existing = placed,
            None => anchors.push(placed),
        }
    }
}
