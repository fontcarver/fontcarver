//! Validate and serialize native static SFD structures before opening a file.

use super::{
    error, finite, guards,
    layout::{self, Lookup},
    outlines, strings, MAX_GLYPHS, MAX_ITEMS, SHADOW,
};
use crate::Error;
use fc_model::{AnchorKind, Font, GlyphName, KernSide, Layer, LayerKind, Master, PointKind};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn write(font: &Font, master: &Master) -> Result<String, Error> {
    if font.cid.is_some() || !font.bitmap_strikes.is_empty() {
        return Err(error(
            "static SFD export does not support CID fonts or bitmap strikes",
        ));
    }
    if font.glyphs.len() > MAX_GLYPHS {
        return Err(error("glyph count exceeds 65535"));
    }
    if !(16..=16384).contains(&font.info.units_per_em) {
        return Err(error("units per em must be between 16 and 16384"));
    }
    if !master.tt.fpgm.is_empty() || !master.tt.prep.is_empty() || !master.tt.cvt.is_empty() {
        return Err(error("SFD TrueType program serialization is unsupported; remove instructions explicitly first"));
    }
    if !master.kerning_v.0.is_empty() {
        return Err(error("vertical kerning SFD export is unsupported"));
    }
    let mut layout = layout::export(font)?;
    let glyph_ids: BTreeMap<_, _> = font
        .glyph_order()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect();
    let mut quadratic = true;
    let mut point_count = 0usize;
    for glyph in font.glyphs.values() {
        if !glyph.export {
            return Err(error(format!(
                "SFD cannot preserve export=false for glyph {}",
                glyph.name
            )));
        }
        if glyph.name.is_empty() || glyph.name.contains(['\n', '\r', '\0']) {
            return Err(error("invalid glyph name"));
        }
        if glyph.master_layer(&master.id).is_none() {
            return Err(error(format!("missing master layer in {}", glyph.name)));
        }
        for layer in &glyph.layers {
            match &layer.kind {
                LayerKind::Master(id) | LayerKind::Background(id) if id != &master.id => continue,
                LayerKind::Master(_) | LayerKind::Background(_) => {}
                _ => {
                    return Err(error(
                        "SFD export supports foreground and background layers only",
                    ))
                }
            }
            validate_layer(layer, &glyph_ids)?;
            point_count += layer.contours.iter().map(|c| c.points.len()).sum::<usize>();
            quadratic &= !layer
                .contours
                .iter()
                .flat_map(|c| &c.points)
                .any(|p| p.kind == PointKind::Curve);
        }
    }
    if point_count > MAX_ITEMS {
        return Err(error("outline point count exceeds limit"));
    }
    validate_components(font, master)?;
    let mut out = header(font, master, quadratic)?;
    let mut kern_lines = BTreeMap::<String, Vec<String>>::new();
    let kern_header = kerning(font, master, &glyph_ids, &mut kern_lines)?;
    let live_kerning = font.feature_settings.writer_policy.kerning
        && !layout
            .lookups
            .iter()
            .any(|lookup| lookup.features.iter().any(|(tag, _)| tag == "kern"));
    if !kern_header.is_empty() {
        out.push_str(
            &Lookup {
                kind: 258,
                flags: 0,
                name: "FontCarver.kern".into(),
                subtables: vec!["FontCarver.kern".into()],
                features: if live_kerning {
                    vec![("kern".into(), vec![("DFLT".into(), vec!["dflt".into()])])]
                } else {
                    Vec::new()
                },
            }
            .native(),
        );
        out.push_str(&kern_header);
    }
    for lookup in &layout.lookups {
        out.push_str(&lookup.native());
    }
    out.push_str(&anchor_classes(font, master, &layout.lookups)?);
    write_private(master, &mut out)?;
    super::info::write_names(font, &mut out)?;
    append_shadow(font.extra.lib.get(SHADOW), &mut out)?;
    out.push_str(&format!(
        "BeginChars: {} {}\n",
        font.glyphs.len(),
        font.glyphs.len()
    ));
    for (name, glyph) in &font.glyphs {
        let id = glyph_ids[name];
        let unicode = glyph.unicodes.first().map_or(-1, |cp| i64::from(*cp));
        for cp in &glyph.unicodes {
            if char::from_u32(*cp).is_none() {
                return Err(error("invalid Unicode scalar"));
            }
        }
        out.push_str(&format!(
            "StartChar: {}\nEncoding: {id} {unicode} {id}\n",
            glyph_name(name.as_ref())
        ));
        let layer = glyph
            .master_layer(&master.id)
            .ok_or_else(|| error("missing master layer"))?;
        if (glyph.category.as_deref() == Some("Mark")
            || layer
                .anchors
                .iter()
                .any(|anchor| matches!(anchor.kind(), AnchorKind::Mark(_))))
            && layer
                .anchors
                .iter()
                .any(|anchor| matches!(anchor.kind(), AnchorKind::Base(_)))
        {
            return Err(error(
                "mark-to-mark anchors require explicit native class-role support before SFD export",
            ));
        }
        out.push_str(&format!("Width: {}\n", finite(layer.width)?));
        if let Some(width) = layer.vertical_width {
            out.push_str(&format!("VWidth: {}\n", finite(width)?));
        }
        if glyph.unicodes.len() > 1 || !glyph.variation_selectors.is_empty() {
            out.push_str("AltUni2:");
            for cp in glyph.unicodes.iter().skip(1) {
                out.push_str(&format!(" {cp:06x}.ffffffff.0"));
            }
            for (cp, selector) in &glyph.variation_selectors {
                if char::from_u32(*cp).is_none()
                    || (!(0xfe00..=0xfe0f).contains(selector)
                        && !(0xe0100..=0xe01ef).contains(selector))
                {
                    return Err(error("invalid Unicode variation sequence"));
                }
                out.push_str(&format!(" {cp:06x}.{selector:x}.0"));
            }
            out.push('\n');
        }
        if !glyph.note.is_empty() {
            out.push_str(&format!("Comment: {}\n", strings::quote(&glyph.note)));
        }
        if let Some(category) = &glyph.category {
            match category.as_str() {
                "Mark" => out.push_str("GlyphClass: 4\n"),
                "Ligature" => out.push_str("GlyphClass: 3\n"),
                "Letter" => out.push_str("GlyphClass: 2\n"),
                _ => {}
            }
        }
        out.push_str("Flags: W\nFore\n");
        write_layer(layer, quadratic, &glyph_ids, font, &mut out)?;
        for background in glyph
            .layers
            .iter()
            .filter(|layer| matches!(&layer.kind,LayerKind::Background(id) if id==&master.id))
        {
            out.push_str("Back\n");
            write_layer(background, quadratic, &glyph_ids, font, &mut out)?;
        }
        if let Some(lines) = kern_lines.remove(name.as_ref()) {
            for line in lines {
                out.push_str(&line);
            }
        }
        if let Some(lines) = layout.glyph_lines.remove(name.as_ref()) {
            for line in lines {
                out.push_str(&line);
            }
        }
        append_shadow(glyph.user_data.get(SHADOW), &mut out)?;
        out.push_str("EndChar\n");
    }
    out.push_str("EndChars\nEndSplineFont\n");
    Ok(out)
}

fn header(font: &Font, master: &Master, quadratic: bool) -> Result<String, Error> {
    let info = &font.info;
    let family = plain(&info.family_name)?;
    let style = plain(if font.masters.len() > 1 && !master.name.is_empty() {
        &master.name
    } else {
        &info.style_name
    })?;
    let ps = info.postscript_font_name.clone().unwrap_or_else(|| {
        format!("{}-{}", family, style)
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    });
    if ps.is_empty()
        || ps
            .chars()
            .any(|c| !c.is_ascii_graphic() || "[](){}<>/%".contains(c))
    {
        return Err(error("invalid PostScript font name"));
    }
    let full = info
        .postscript_full_name
        .as_deref()
        .map(plain)
        .transpose()?
        .unwrap_or_else(|| format!("{family} {style}"));
    let upem = f64::from(info.units_per_em);
    let ascent = if master.metrics.ascender >= 0.0 && master.metrics.ascender <= upem {
        master.metrics.ascender.round()
    } else {
        (upem * 0.8).round()
    };
    let mut out=format!("SplineFontDB: 3.0\nFontName: {ps}\nFullName: {full}\nFamilyName: {family}\nWeight: {style}\nVersion: {}.{:03}\nAscent: {ascent}\nDescent: {}\nUFOAscent: {}\nUFODescent: {}\nItalicAngle: {}\nLayerCount: 2\nLayer: 0 {} \"Back\" 1\nLayer: 1 {} \"Fore\" 0\nEncoding: Custom\n",info.version_major,info.version_minor,upem-ascent,finite(master.metrics.ascender)?,finite(master.metrics.descender)?,finite(master.metrics.italic_angle)?,u8::from(quadratic),u8::from(quadratic));
    if let Some(copyright) = &info.copyright {
        out.push_str(&format!("Copyright: {}\n", plain(copyright)?));
    }
    if let Some(note) = &info.note {
        out.push_str(&format!("UComments: {}\n", strings::quote(note)));
    }
    for (key, value) in [
        ("UnderlinePosition", info.postscript_underline_position),
        ("UnderlineWidth", info.postscript_underline_thickness),
        ("OS2CapHeight", Some(master.metrics.cap_height)),
        ("OS2XHeight", Some(master.metrics.x_height)),
        ("LineGap", master.metrics.hhea_line_gap),
        ("OS2TypoLinegap", master.metrics.typo_line_gap),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("{key}: {}\n", finite(value)?));
        }
    }
    for (key, offset, value) in [
        ("HheadAscent", "HheadAOffset", master.metrics.hhea_ascender),
        (
            "HheadDescent",
            "HheadDOffset",
            master.metrics.hhea_descender,
        ),
        (
            "OS2TypoAscent",
            "OS2TypoAOffset",
            master.metrics.typo_ascender,
        ),
        (
            "OS2TypoDescent",
            "OS2TypoDOffset",
            master.metrics.typo_descender,
        ),
        ("OS2WinAscent", "OS2WinAOffset", master.metrics.win_ascent),
        ("OS2WinDescent", "OS2WinDOffset", master.metrics.win_descent),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("{key}: {}\n{offset}: 0\n", finite(value)?));
        }
    }
    if let Some(weight) = info.open_type_os2_weight_class {
        out.push_str(&format!("TTFWeight: {weight}\n"));
    }
    if let Some(width) = info.open_type_os2_width_class {
        out.push_str(&format!("TTFWidth: {width}\n"));
    }
    if let Some(vendor) = &info.open_type_os2_vendor_id {
        if vendor.len() != 4 || vendor.chars().any(|c| !c.is_ascii_graphic() || c == '\'') {
            return Err(error("invalid OS2 vendor tag"));
        }
        out.push_str(&format!("OS2Vendor: '{vendor}'\n"));
    }
    Ok(out)
}

fn plain(value: &str) -> Result<String, Error> {
    if value.contains(['\n', '\r', '\0']) {
        Err(error("SFD unquoted name contains a newline or NUL"))
    } else {
        Ok(value.into())
    }
}
fn glyph_name(value: &str) -> String {
    if value.is_ascii() && !value.chars().any(|c| c.is_whitespace() || c == '"') {
        value.into()
    } else {
        strings::quote(value)
    }
}

fn validate_layer(layer: &Layer, glyph_ids: &BTreeMap<GlyphName, usize>) -> Result<(), Error> {
    finite(layer.width)?;
    if layer.vertical_origin.is_some()
        || layer.background_image.is_some()
        || layer.color_layers.is_some()
    {
        return Err(error(
            "SFD export cannot represent this vertical origin, image or color layer",
        ));
    }
    if layer
        .tt_instructions
        .as_ref()
        .is_some_and(|v| !v.is_empty())
        || !layer.hints.d_stems.is_empty()
        || !layer.hints.hint_masks.is_empty()
        || layer.hints.flex
    {
        return Err(error(
            "SFD export does not support TrueType instructions, diagonal hints, hint masks or flex",
        ));
    }
    for component in &layer.components {
        if !glyph_ids.contains_key(&component.base) {
            return Err(error(format!("missing component base {}", component.base)));
        }
        if component.smart.is_some() || component.auto_align || component.anchor.is_some() {
            return Err(error(
                "SFD cannot preserve smart or automatically aligned components",
            ));
        }
        for value in component.transform.as_coeffs() {
            finite(value)?;
        }
    }
    Ok(())
}

fn write_layer(
    layer: &Layer,
    quadratic: bool,
    ids: &BTreeMap<GlyphName, usize>,
    font: &Font,
    out: &mut String,
) -> Result<(), Error> {
    for (key, stems) in [
        ("HStem", &layer.hints.h_stems),
        ("VStem", &layer.hints.v_stems),
    ] {
        if !stems.is_empty() {
            out.push_str(&format!("{key}:"));
            for hint in stems {
                out.push_str(&format!(" {} {}", finite(hint.start)?, finite(hint.width)?));
                if !hint.ranges.is_empty() {
                    out.push('<');
                    for (index, (a, b)) in hint.ranges.iter().enumerate() {
                        if index > 0 {
                            out.push(' ');
                        }
                        out.push_str(&format!("{} {}", finite(*a)?, finite(*b)?));
                    }
                    out.push('>');
                }
            }
            out.push('\n');
        }
    }
    outlines::write(&layer.contours, quadratic, out)?;
    for component in &layer.components {
        let id = ids[&component.base];
        let unicode = font
            .glyph(component.base.as_ref())
            .and_then(|g| g.unicodes.first())
            .map_or(-1, |cp| i64::from(*cp));
        let [a, b, c, d, e, f] = component.transform.as_coeffs();
        let flags = u8::from(component.use_my_metrics)
            | (u8::from(component.round_to_grid) * 2)
            | (u8::from(component.point_match.is_some()) * 4);
        out.push_str(&format!(
            "Refer: {id} {unicode} N {a} {b} {c} {d} {e} {f} {flags}"
        ));
        if let Some((a, b)) = component.point_match {
            out.push_str(&format!(" {a} {b}"));
        }
        out.push('\n');
    }
    let mut carets = Vec::new();
    for anchor in &layer.anchors {
        finite(anchor.x)?;
        finite(anchor.y)?;
        let (name, kind, index) = match anchor.kind() {
            AnchorKind::Base(name) => (name, "basechar", 0),
            AnchorKind::Mark(name) => (name, "mark", 0),
            AnchorKind::Ligature { name, index } => {
                if index == 0 {
                    return Err(error("ligature anchor index must be one-based"));
                }
                (name, "baselig", index - 1)
            }
            AnchorKind::CursiveEntry => ("cursive".into(), "entry", 0),
            AnchorKind::CursiveExit => ("cursive".into(), "exit", 0),
            AnchorKind::Caret(index) => {
                carets.push((index, anchor.x));
                continue;
            }
            AnchorKind::VCaret(_) => return Err(error("vertical ligature carets are unsupported")),
        };
        out.push_str(&format!(
            "AnchorPoint: {} {} {} {kind} {index}\n",
            strings::quote(&name),
            anchor.x,
            anchor.y
        ));
    }
    if !carets.is_empty() {
        carets.sort_by_key(|(index, _)| *index);
        out.push_str(&format!("LCarets2: {}", carets.len()));
        for (_, x) in carets {
            out.push_str(&format!(" {x}"));
        }
        out.push('\n');
    }
    Ok(())
}

fn anchor_classes(font: &Font, master: &Master, lookups: &[Lookup]) -> Result<String, Error> {
    let mut kinds = BTreeMap::<String, BTreeSet<i64>>::new();
    for anchor in font
        .glyphs
        .values()
        .filter_map(|g| g.master_layer(&master.id))
        .flat_map(|l| &l.anchors)
    {
        match anchor.kind() {
            AnchorKind::Base(name) => {
                kinds.entry(name).or_default().insert(260);
            }
            AnchorKind::Ligature { name, .. } => {
                kinds.entry(name).or_default().insert(261);
            }
            AnchorKind::Mark(name) => {
                kinds.entry(name).or_default();
            }
            AnchorKind::CursiveEntry | AnchorKind::CursiveExit => {
                kinds.entry("cursive".into()).or_default().insert(259);
            }
            _ => {}
        }
    }
    let mut out = String::new();
    let mut classes = String::new();
    for (name, types) in kinds {
        if types.len() > 1 {
            return Err(error(format!("anchor {name:?} is shared by base and ligature lookups; split classes before SFD export")));
        }
        let kind = types.iter().copied().next().unwrap_or(260);
        let feature = if kind == 259 { "curs" } else { "mark" };
        if lookups
            .iter()
            .any(|lookup| lookup.features.iter().any(|(tag, _)| tag == feature))
        {
            return Err(error(
                "explicit mark/cursive feature source with automatic anchors is unsupported",
            ));
        }
        let table = format!("FontCarver.anchor.{name}");
        let features = if font.feature_settings.writer_policy.anchors {
            vec![(feature.into(), vec![("DFLT".into(), vec!["dflt".into()])])]
        } else {
            Vec::new()
        };
        out.push_str(
            &Lookup {
                kind,
                flags: 0,
                name: table.clone(),
                subtables: vec![table.clone()],
                features,
            }
            .native(),
        );
        classes.push_str(&format!(
            " {} {}",
            strings::quote(&name),
            strings::quote(&table)
        ));
    }
    if !classes.is_empty() {
        out.push_str(&format!("AnchorClass2:{classes}\n"));
    }
    Ok(out)
}

fn kerning(
    font: &Font,
    master: &Master,
    ids: &BTreeMap<GlyphName, usize>,
    lines: &mut BTreeMap<String, Vec<String>>,
) -> Result<String, Error> {
    if master.kerning_h.0.is_empty()
        && font.kerning_groups.side1.is_empty()
        && font.kerning_groups.side2.is_empty()
    {
        return Ok(String::new());
    }
    let left: Vec<_> = font.kerning_groups.side1.iter().collect();
    let right: Vec<_> = font.kerning_groups.side2.iter().collect();
    if (left.len() + 1).saturating_mul(right.len() + 1) > MAX_ITEMS {
        return Err(error("kerning class matrix exceeds limit"));
    }
    for groups in [&font.kerning_groups.side1, &font.kerning_groups.side2] {
        let mut seen = BTreeSet::new();
        for members in groups.values() {
            for member in members {
                layout::glyph_token(member.as_ref())?;
                if !ids.contains_key(member) || !seen.insert(member) {
                    return Err(error("missing/overlapping SFD kerning group membership"));
                }
            }
        }
    }
    let members = |side: &KernSide, left_side: bool| -> Result<Vec<GlyphName>, Error> {
        match side {
            KernSide::Glyph(name) => {
                if !ids.contains_key(name) {
                    return Err(error("kerning references missing glyph"));
                }
                Ok(vec![name.clone()])
            }
            KernSide::Group(name) => {
                let groups = if left_side {
                    &font.kerning_groups.side1
                } else {
                    &font.kerning_groups.side2
                };
                Ok(groups
                    .get(name)
                    .ok_or_else(|| error("kerning references missing group"))?
                    .iter()
                    .cloned()
                    .collect())
            }
        }
    };
    let mut explicit = BTreeMap::<(GlyphName, GlyphName), (u8, f64)>::new();
    for ((a, b), value) in &master.kerning_h.0 {
        finite(*value)?;
        let a_members = members(a, true)?;
        let b_members = members(b, false)?;
        if matches!((a, b), (KernSide::Group(_), KernSide::Group(_))) {
            continue;
        }
        if a_members.len().saturating_mul(b_members.len()) > MAX_ITEMS {
            return Err(error("kerning exception expansion limit exceeded"));
        }
        let priority = match (a, b) {
            (KernSide::Glyph(_), KernSide::Glyph(_)) => 3,
            (KernSide::Glyph(_), _) => 2,
            _ => 1,
        };
        for a in a_members {
            for b in &b_members {
                let entry = explicit
                    .entry((a.clone(), b.clone()))
                    .or_insert((0, *value));
                if priority >= entry.0 {
                    *entry = (priority, *value);
                }
            }
        }
        if explicit.len() > MAX_ITEMS {
            return Err(error("kerning pair count exceeds limit"));
        }
    }
    for ((a, b), (_, value)) in explicit {
        lines
            .entry(a.to_string())
            .or_default()
            .push(format!("Kerns2: {} {value} \"FontCarver.kern\"\n", ids[&b]));
    }
    // An empty class matrix still ensures a pair-only font emits its lookup.
    let mut out = format!(
        "KernClass2: {} {} 0 \"FontCarver.kern\"\n",
        left.len() + 1,
        right.len() + 1
    );
    for (_, members) in left.iter().chain(right.iter()) {
        let names = members
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(" {} {names}\n", names.len()));
    }
    for row in 0..=left.len() {
        for col in 0..=right.len() {
            if col > 0 {
                out.push(' ');
            }
            let value = if row == 0 || col == 0 {
                0.0
            } else {
                master
                    .kerning_h
                    .0
                    .get(&(
                        KernSide::Group(left[row - 1].0.clone()),
                        KernSide::Group(right[col - 1].0.clone()),
                    ))
                    .copied()
                    .unwrap_or(0.0)
            };
            out.push_str(&value.to_string());
        }
        out.push('\n');
    }
    Ok(out)
}

fn write_private(master: &Master, out: &mut String) -> Result<(), Error> {
    let p = &master.private_dict;
    let mut entries = BTreeMap::<String, String>::new();
    for (key, values) in [
        ("BlueValues", &p.blue_values),
        ("OtherBlues", &p.other_blues),
        ("FamilyBlues", &p.family_blues),
        ("FamilyOtherBlues", &p.family_other_blues),
        ("StemSnapH", &p.stem_snap_h),
        ("StemSnapV", &p.stem_snap_v),
    ] {
        if !values.is_empty() {
            for v in values {
                finite(*v)?;
            }
            entries.insert(
                key.into(),
                format!(
                    "[{}]",
                    values
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            );
        }
    }
    for (key, value) in [
        ("BlueScale", p.blue_scale),
        ("BlueShift", p.blue_shift),
        ("BlueFuzz", p.blue_fuzz),
        ("StdHW", p.std_hw),
        ("StdVW", p.std_vw),
    ] {
        if let Some(v) = value {
            entries.insert(key.into(), finite(v)?.to_string());
        }
    }
    if let Some(v) = p.force_bold {
        entries.insert("ForceBold".into(), v.to_string());
    }
    if let Some(v) = p.language_group {
        entries.insert("LanguageGroup".into(), v.to_string());
    }
    for (key, value) in &p.other {
        let value = value
            .as_str()
            .ok_or_else(|| error("private dictionary extension must be an SFD string"))?;
        if key.chars().any(char::is_whitespace) || value.contains(['\n', '\r', '\0']) {
            return Err(error("invalid private dictionary extension"));
        }
        entries.entry(key.clone()).or_insert_with(|| value.into());
    }
    if entries.is_empty() {
        return Ok(());
    }
    out.push_str(&format!("BeginPrivate: {}\n", entries.len()));
    for (key, value) in entries {
        out.push_str(&format!("{key} {} {value}\n", value.len()));
    }
    out.push_str("EndPrivate\n");
    Ok(())
}

fn append_shadow(value: Option<&serde_json::Value>, out: &mut String) -> Result<(), Error> {
    let Some(value) = value else {
        return Ok(());
    };
    let lines = value
        .as_array()
        .ok_or_else(|| error("invalid SFD source shadow"))?;
    for line in lines {
        let line = line
            .as_str()
            .ok_or_else(|| error("invalid SFD shadow record"))?;
        if line.contains(['\n', '\r', '\0']) {
            return Err(error("invalid multiline SFD shadow"));
        }
        let (key, _) = line
            .split_once(':')
            .ok_or_else(|| error("invalid SFD shadow key"))?;
        guards::safe(key)?;
        out.push_str(line);
        out.push('\n');
    }
    Ok(())
}

fn validate_components(font: &Font, master: &Master) -> Result<(), Error> {
    fn visit(
        font: &Font,
        master: &Master,
        name: &GlyphName,
        stack: &mut BTreeSet<GlyphName>,
        done: &mut BTreeSet<GlyphName>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > 64 {
            return Err(error("component depth exceeds 64"));
        }
        if done.contains(name) {
            return Ok(());
        }
        if !stack.insert(name.clone()) {
            return Err(error(format!("component cycle at {name}")));
        }
        let glyph = font
            .glyph(name.as_ref())
            .ok_or_else(|| error("missing component base"))?;
        for layer in glyph.layers.iter().filter(
            |l| matches!(&l.kind,LayerKind::Master(id)|LayerKind::Background(id) if id==&master.id),
        ) {
            for component in &layer.components {
                visit(font, master, &component.base, stack, done, depth + 1)?;
            }
        }
        stack.remove(name);
        done.insert(name.clone());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for name in font.glyph_order() {
        visit(font, master, name, &mut BTreeSet::new(), &mut done, 0)?;
    }
    Ok(())
}
