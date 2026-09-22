//! Vertical metrics and model-owned vertical kerning source generation.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use fc_model::{Font, GlyphName, KernSide, Master, MasterId};
use fea_rs::{parse::SourceLoadError, Kind, NodeOrToken};

use crate::Error;

/// Resolve defaults on the compilation copy so every source uses the same policy.
pub(crate) fn apply_metric_defaults(font: &mut Font) {
    let vertical = font.masters.iter().any(|m| {
        m.metrics.default_vertical_advance.is_some() || m.metrics.default_vertical_origin.is_some()
    }) || font.glyphs.values().any(|g| {
        g.layers
            .iter()
            .any(|l| l.vertical_width.is_some() || l.vertical_origin.is_some())
    });
    if !vertical {
        return;
    }
    let upem = f64::from(font.info.units_per_em);
    for glyph in font.glyphs.values_mut() {
        for layer in &mut glyph.layers {
            let master = match &layer.kind {
                fc_model::LayerKind::Master(id)
                | fc_model::LayerKind::Brace { master: id, .. }
                | fc_model::LayerKind::Bracket { master: id, .. }
                | fc_model::LayerKind::Smart { master: id, .. } => {
                    font.masters.iter().find(|m| &m.id == id)
                }
                _ => None,
            };
            if let Some(master) = master {
                layer
                    .vertical_width
                    .get_or_insert(master.metrics.default_vertical_advance.unwrap_or(upem));
                layer.vertical_origin.get_or_insert(
                    master
                        .metrics
                        .default_vertical_origin
                        .unwrap_or(master.metrics.ascender),
                );
            }
        }
    }
}

pub(crate) fn feature_source(
    font: &Font,
    master: &MasterId,
    variable: bool,
) -> Result<String, Error> {
    let mut source = font.features.to_fea();
    let masters: Vec<_> = font
        .masters
        .iter()
        .filter(|m| variable || &m.id == master)
        .collect();
    if !font.feature_settings.writer_policy.kerning
        || font.features.source_of_truth == fc_model::FeaturesSource::ImportedTables
        || !masters.iter().any(|m| !m.kerning_v.0.is_empty())
        || source_owns_vkrn(&source)?
    {
        return Ok(source);
    }
    let mut expanded = Vec::new();
    let mut budget = 1_000_000usize;
    for master in &masters {
        expanded.push(expand(font, master, &mut budget)?);
    }
    let pairs: std::collections::BTreeSet<_> = expanded
        .iter()
        .flat_map(|pairs| pairs.keys().cloned())
        .collect();
    if pairs.len().saturating_mul(masters.len()) > 1_000_000 {
        return Err(Error::Limit("vertical kerning variation records"));
    }
    let locations: Vec<_> = masters
        .iter()
        .map(|master| {
            font.axes
                .iter()
                .map(|a| {
                    let value = master
                        .location
                        .get(&a.tag)
                        .copied()
                        .unwrap_or_else(|| a.user_to_design(a.default));
                    format!("{}={}d", a.tag, value)
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    let mut lookup = "FCVerticalKerning".to_string();
    while source.contains(&lookup) {
        lookup.push('_');
    }
    source.push_str(&format!("\nlookup {lookup} {{\n"));
    for (left, right) in pairs {
        let left_name = fea_name(&left)?;
        let right_name = fea_name(&right)?;
        let values: Vec<_> = expanded
            .iter()
            .map(|pairs| {
                pairs
                    .get(&(left.clone(), right.clone()))
                    .copied()
                    .unwrap_or_default()
            })
            .collect();
        // GPOS advance adjustments use the logical advance direction. Negative
        // values tighten; the shaper negates the final advance for top-to-bottom.
        let metric = if variable && values.windows(2).any(|v| v[0] != v[1]) {
            let entries: Result<Vec<_>, Error> = locations
                .iter()
                .zip(&values)
                .map(|(location, value)| Ok(format!("{location}:{}", y_adjustment(*value)?)))
                .collect();
            format!("({})", entries?.join(" "))
        } else {
            y_adjustment(values[0])?.to_string()
        };
        source.push_str(&format!(
            "  pos [{left_name}] [{right_name}] <0 0 0 {metric}>;\n"
        ));
        if source.len() > 8_000_000 {
            return Err(Error::Limit("vertical kerning feature source"));
        }
    }
    source.push_str(&format!("}} {lookup};\nfeature vkrn {{\n"));
    for script in scripts(font) {
        source.push_str(&format!(
            "  script {script};\n  language dflt;\n  lookup {lookup};\n"
        ));
    }
    source.push_str("} vkrn;\n");
    Ok(source)
}

fn y_adjustment(value: f64) -> Result<i16, Error> {
    let y = (value + 0.5).floor();
    if !y.is_finite() || !(-32768.0..=32767.0).contains(&y) {
        return Err(Error::InvalidSource(
            "vertical kerning exceeds signed 16-bit range".into(),
        ));
    }
    Ok(y as i16)
}

fn fea_name(name: &GlyphName) -> Result<String, Error> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._+-*:^~".contains(c))
    {
        return Err(Error::InvalidSource(format!(
            "glyph `{name}` cannot be represented in vertical kerning feature syntax"
        )));
    }
    Ok(format!("\\{name}"))
}

type Pairs = BTreeMap<(GlyphName, GlyphName), f64>;

fn expand(font: &Font, master: &Master, budget: &mut usize) -> Result<Pairs, Error> {
    for groups in [&font.kerning_groups.side1, &font.kerning_groups.side2] {
        let mut seen = BTreeSet::new();
        for name in groups.values().flatten() {
            if !seen.insert(name) {
                return Err(Error::InvalidSource(format!(
                    "glyph {name} belongs to multiple vertical kerning groups on one side"
                )));
            }
        }
    }
    let members = |side: &KernSide, first| -> Result<Vec<GlyphName>, Error> {
        let names = match side {
            KernSide::Glyph(name) => vec![name.clone()],
            KernSide::Group(name) => {
                let groups = if first {
                    &font.kerning_groups.side1
                } else {
                    &font.kerning_groups.side2
                };
                groups
                    .get(name)
                    .ok_or_else(|| {
                        Error::InvalidSource(format!("missing vertical kerning group `{name}`"))
                    })?
                    .iter()
                    .cloned()
                    .collect()
            }
        };
        let mut result = Vec::new();
        for name in names {
            let glyph = font.glyph(&name).ok_or_else(|| {
                Error::InvalidSource(format!("missing vertical kerning glyph `{name}`"))
            })?;
            if glyph.export {
                result.push(name);
            }
        }
        Ok(result)
    };
    let mut records: Vec<_> = master.kerning_v.0.iter().collect();
    // Insert general rules first, then the two exception levels and exact pairs.
    records.sort_by_key(|((l, r), _)| {
        (
            matches!(l, KernSide::Glyph(_)),
            matches!(r, KernSide::Glyph(_)),
        )
    });
    let mut pairs = Pairs::new();
    for ((left, right), value) in records {
        y_adjustment(*value)?;
        let left = members(left, true)?;
        let right = members(right, false)?;
        let work = left.len().saturating_mul(right.len());
        *budget = budget
            .checked_sub(work)
            .ok_or(Error::Limit("vertical kerning group expansion"))?;
        for l in &left {
            for r in &right {
                pairs.insert((l.clone(), r.clone()), *value);
            }
        }
        if pairs.len() > 100_000 {
            return Err(Error::Limit("vertical kerning pairs"));
        }
    }
    Ok(pairs)
}

fn source_owns_vkrn(source: &str) -> Result<bool, Error> {
    if source.trim().is_empty() {
        return Ok(false);
    }
    let source: Arc<str> = source.into();
    const ROOT: &str = "fontcarver-vertical.fea";
    let (tree, diagnostics) = fea_rs::parse::parse_root(
        ROOT.into(),
        None,
        Box::new(move |path: &Path| {
            if path == Path::new(ROOT) {
                Ok(source.clone())
            } else {
                Err(SourceLoadError::new(
                    path.to_owned(),
                    "external includes are unavailable",
                ))
            }
        }),
    )
    .map_err(|e| Error::InvalidSource(e.to_string()))?;
    if diagnostics.has_errors() {
        return Err(Error::InvalidSource(
            "cannot establish vertical kerning ownership from invalid feature source".into(),
        ));
    }
    Ok(tree
        .root()
        .iter_children()
        .filter(|node| matches!(node.kind(), Kind::FeatureNode | Kind::VariationNode))
        .any(|node| {
            node.as_node().and_then(|node| {
                node.iter_children()
                    .find(|child| child.kind() == Kind::Tag)
                    .and_then(NodeOrToken::token_text)
            }) == Some("vkrn")
        }))
}

// Register the generated lookup for the font's scripts as well as DFLT.
// Otherwise a script added by fontc's horizontal kerning writer can mask it.
fn scripts(font: &Font) -> BTreeSet<String> {
    let mut scripts: BTreeSet<String> = ["DFLT".into()].into();
    for cp in font
        .glyphs
        .values()
        .filter(|g| g.export)
        .flat_map(|g| &g.unicodes)
    {
        let Some(ch) = char::from_u32(*cp) else {
            continue;
        };
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str(ch.encode_utf8(&mut [0; 4]));
        buffer.guess_segment_properties();
        let tag = buffer.script().tag().to_string().to_ascii_lowercase();
        match tag.as_str() {
            "zzzz" | "zyyy" | "zinh" => {}
            "hira" | "kana" => {
                scripts.insert("kana".into());
            }
            "laoo" => {
                scripts.insert("lao".into());
            }
            "yiii" => {
                scripts.insert("yi".into());
            }
            "nkoo" => {
                scripts.insert("nko".into());
            }
            "vaii" => {
                scripts.insert("vai".into());
            }
            _ => {
                scripts.insert(tag.clone());
            }
        }
        for (old, new) in [
            ("beng", "bng2"),
            ("deva", "dev2"),
            ("gujr", "gjr2"),
            ("guru", "gur2"),
            ("knda", "knd2"),
            ("mlym", "mlm2"),
            ("orya", "ory2"),
            ("taml", "tml2"),
            ("telu", "tel2"),
            ("mymr", "mym2"),
        ] {
            if tag == old {
                scripts.insert(new.into());
            }
        }
    }
    scripts
}
