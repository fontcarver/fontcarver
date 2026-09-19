//! Bounded, line-oriented native SFD decoding.

use super::guards::safe;
use super::info::{apply_header, private_entry, read_names};
use super::kerning::{install_kern_class, read_kern_class};
use super::{
    error, integer,
    layout::{self, Lookup},
    number, outlines, strings, MAX_GLYPHS, MAX_ITEMS, SHADOW,
};
use crate::Error;
use fc_model::{Anchor, Component, Font, Glyph, KernSide, Layer, LayerKind, MasterId, StemHint};
use kurbo::Affine;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read(source: &str) -> Result<Font, Error> {
    let mut lines = source.lines().map(str::trim).peekable();
    let first = lines.next().ok_or_else(|| error("empty SFD file"))?;
    let version = first
        .strip_prefix("SplineFontDB:")
        .ok_or_else(|| error("missing SplineFontDB header"))?
        .trim();
    if !matches!(version, "1.0" | "2.0" | "3.0" | "3.1" | "3.2") {
        return Err(error(format!("unsupported SFD version {version}")));
    }
    let mut font = Font::new_empty("Untitled", 1000);
    let master = font.default_master().id.clone();
    let mut header = BTreeMap::<String, String>::new();
    let mut shadow = Vec::<String>::new();
    let mut layer_defs = BTreeMap::from([
        (0, ("Back".to_owned(), false, true)),
        (1, ("Fore".to_owned(), false, false)),
    ]);
    let mut lookups = Vec::new();
    let mut kern_classes = Vec::new();
    let mut count = None;
    let mut encoding_slots = 0;
    let mut anchor_classes = BTreeMap::new();
    let mut names = Vec::new();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        let (key, value) = field(line)?;
        match key {
            "BeginChars" => {
                let t = strings::tokens(value)?;
                if t.len() != 2 {
                    return Err(error("malformed BeginChars"));
                }
                let slots = integer(&t[0])?;
                let n = integer(&t[1])?;
                if !(0..=MAX_GLYPHS as i64).contains(&n) || !(0..=0x11_ffff).contains(&slots) {
                    return Err(error("glyph/encoding count exceeds limit"));
                }
                count = Some(n as usize);
                encoding_slots = slots;
                break;
            }
            "Lookup" => lookups.push(Lookup::parse(value)?),
            "KernClass2" => kern_classes.push(read_kern_class(value, &mut lines)?),
            "Layer" => {
                let t = strings::tokens(value)?;
                if !(3..=4).contains(&t.len()) {
                    return Err(error("malformed layer declaration"));
                }
                let id = integer(&t[0])?;
                if !(0..=255).contains(&id) {
                    return Err(error("layer index exceeds limit"));
                }
                layer_defs.insert(
                    id,
                    (
                        t[2].clone(),
                        boolean(&t[1])?,
                        t.get(3).map(|v| boolean(v)).transpose()?.unwrap_or(id == 0),
                    ),
                );
            }
            "LayerCount" => {
                if integer(value)? > 256 {
                    return Err(error("layer count exceeds limit"));
                }
            }
            "BeginPrivate" => {
                let n = integer(value)?;
                if !(0..=1024).contains(&n) {
                    return Err(error("private dictionary size exceeds limit"));
                }
                for _ in 0..n {
                    let line = lines
                        .next()
                        .ok_or_else(|| error("truncated private dictionary"))?;
                    private_entry(&mut font, line)?;
                }
                if lines.next() != Some("EndPrivate") {
                    return Err(error("missing EndPrivate"));
                }
            }
            "AnchorClass2" => {
                let t = strings::tokens(value)?;
                if t.len() % 2 != 0 {
                    return Err(error("malformed anchor classes"));
                }
                for pair in t.chunks_exact(2) {
                    if anchor_classes
                        .insert(pair[0].clone(), pair[1].clone())
                        .is_some()
                    {
                        return Err(error("duplicate anchor class"));
                    }
                }
            }
            "MarkAttachClasses" if value == "1" => {}
            "MarkAttachSets" if value == "0" => {}
            "LangName" => {
                names.push(value);
            }
            "FontName" | "FullName" | "FamilyName" | "Weight" | "Copyright" | "Version"
            | "UComments" | "Comments" | "Ascent" | "Descent" | "ItalicAngle" | "UFOAscent"
            | "UFODescent" | "OS2TypoAscent" | "OS2TypoDescent" | "OS2TypoAOffset"
            | "OS2TypoDOffset" | "OS2TypoLinegap" | "OS2TypoLineGap" | "OS2WinAscent"
            | "OS2WinDescent" | "OS2WinAOffset" | "OS2WinDOffset" | "HheadAscent"
            | "HheadDescent" | "HheadAOffset" | "HheadDOffset" | "LineGap"
            | "UnderlinePosition" | "UnderlineWidth" | "TTFWeight" | "TTFWidth" | "OS2Vendor"
            | "Order2" | "Encoding" | "OS2CapHeight" | "OS2XHeight" => {
                header.insert(key.into(), value.into());
            }
            "OnlyBitmaps" | "Multilayer" if value == "0" => {}
            _ => {
                safe(key)?;
                shadow.push(line.into());
            }
        }
    }
    let count = count.ok_or_else(|| error("missing BeginChars"))?;
    let mut lookup_names = BTreeSet::new();
    let mut subtable_names = BTreeSet::new();
    for lookup in &lookups {
        if !lookup_names.insert(&lookup.name)
            || lookup
                .subtables
                .iter()
                .any(|name| !subtable_names.insert(name))
        {
            return Err(error("duplicate lookup or subtable name"));
        }
        if lookup.subtables.len() != 1 {
            return Err(error(
                "multiple native lookup subtables require preserving subtable boundaries",
            ));
        }
    }
    if kern_classes.len() > 1 {
        return Err(error("multiple kerning lookups/classes cannot be merged without changing additive or subtable semantics"));
    }
    apply_header(&mut font, &header)?;
    for names in names {
        read_names(&mut font, names)?;
    }
    if let Some(order) = header.get("Order2") {
        for (_, quadratic, _) in layer_defs.values_mut() {
            *quadratic = boolean(order)?;
        }
    }
    let mut pending = Vec::new();
    let mut ids = BTreeMap::new();
    let mut rules = BTreeMap::<String, Vec<String>>::new();
    let mut ended = false;
    let mut total_points = 0usize;
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if line == "EndChars" {
            ended = true;
            break;
        }
        let name = strings::text(
            line.strip_prefix("StartChar:")
                .ok_or_else(|| error("expected StartChar or EndChars"))?
                .trim(),
        )?;
        if name.is_empty() || name.contains(['\n', '\r', '\0']) {
            return Err(error("invalid glyph name"));
        }
        let mut data = GlyphData::new(name, &master);
        let mut layer = 1i64;
        let mut ended_glyph = false;
        while let Some(line) = lines.next() {
            if line.is_empty() {
                continue;
            }
            if line == "EndChar" {
                ended_glyph = true;
                break;
            }
            if line == "Fore" {
                layer = 1;
                continue;
            }
            if line == "Back" {
                layer = 0;
                continue;
            }
            if line == "SplineSet" || is_spline(line) {
                let mut splines = Vec::new();
                if line != "SplineSet" {
                    splines.push(line);
                }
                let mut closed = false;
                for line in lines.by_ref() {
                    if line == "EndSplineSet" {
                        closed = true;
                        break;
                    }
                    if !line.is_empty() {
                        splines.push(line);
                    }
                    if splines.len() > MAX_ITEMS {
                        return Err(error("spline command limit exceeded"));
                    }
                }
                if !closed {
                    return Err(error("missing EndSplineSet"));
                }
                let quadratic = layer_defs
                    .get(&layer)
                    .ok_or_else(|| error("undefined layer"))?
                    .1;
                let contours = outlines::read(&splines, quadratic)?;
                total_points += contours
                    .iter()
                    .map(|contour| contour.points.len())
                    .sum::<usize>();
                if total_points > MAX_ITEMS {
                    return Err(error("total outline point limit exceeded"));
                }
                data.layer(layer, &master, &layer_defs)?
                    .contours
                    .extend(contours);
                continue;
            }
            let (key, value) = field(line)?;
            match key {
                "Encoding" => {
                    let t = strings::tokens(value)?;
                    if !(2..=3).contains(&t.len()) {
                        return Err(error("malformed glyph Encoding"));
                    }
                    let slot = integer(&t[0])?;
                    if slot < -1 || slot >= encoding_slots {
                        return Err(error("glyph encoding index is outside BeginChars slots"));
                    }
                    let cp = integer(&t[1])?;
                    if cp < -1 {
                        return Err(error("invalid Unicode encoding sentinel"));
                    }
                    if cp >= 0 {
                        data.glyph.unicodes.push(codepoint(cp)?);
                    }
                    data.gid = Some(if t.len() == 3 {
                        integer(&t[2])?
                    } else {
                        pending.len() as i64
                    });
                }
                "Width" => data.glyph.layers[0].width = number(value)?,
                "VWidth" => data.glyph.layers[0].vertical_width = Some(number(value)?),
                "Layer" => {
                    layer = integer(value)?;
                    data.layer(layer, &master, &layer_defs)?;
                }
                "LayerCount" => {
                    if integer(value)? > 256 {
                        return Err(error("layer count exceeds limit"));
                    }
                }
                "Refer" | "Ref" => {
                    let (id, component) = read_reference(value)?;
                    let index = data.layer_index(layer, &master, &layer_defs)?;
                    data.references.push((index, id, component));
                }
                "AnchorPoint" => {
                    let anchor = read_anchor(value)?;
                    data.layer(layer, &master, &layer_defs)?
                        .anchors
                        .push(anchor);
                }
                "Kerns2" => {
                    let t = strings::tokens(value)?;
                    if t.len() % 3 != 0 {
                        return Err(error(
                            "kern device tables or malformed kerning are unsupported",
                        ));
                    }
                    for pair in t.chunks_exact(3) {
                        data.kerns
                            .push((integer(&pair[0])?, number(&pair[1])?, pair[2].clone()));
                    }
                }
                "Substitution2" | "MultipleSubs2" | "AlternateSubs2" | "Ligature2"
                | "Position2" | "PairPos2" => {
                    layout::glyph_token(data.glyph.name.as_ref())?;
                    let (table, rule, kind) =
                        layout::native_rule(key, data.glyph.name.as_ref(), value)?;
                    let lookup = find_lookup(&lookups, &table)?;
                    if lookup.kind != kind {
                        return Err(error("layout rule does not match lookup type"));
                    }
                    rules.entry(table).or_default().push(rule);
                }
                "AltUni2" => {
                    for triple in value.split_whitespace() {
                        let parts: Vec<_> = triple.split('.').collect();
                        if parts.len() != 3 {
                            return Err(error("malformed AltUni2 triple"));
                        }
                        let cp = u32::from_str_radix(parts[0], 16)
                            .map_err(|_| error("invalid AltUni2 Unicode"))?;
                        codepoint(i64::from(cp))?;
                        if parts[2] != "0" {
                            return Err(error("unsupported alternate-Unicode reserved field"));
                        }
                        if parts[1] == "ffffffff" {
                            if !data.glyph.unicodes.contains(&cp) {
                                data.glyph.unicodes.push(cp);
                            }
                        } else {
                            let selector = u32::from_str_radix(parts[1], 16)
                                .map_err(|_| error("invalid variation selector"))?;
                            if !(0xfe00..=0xfe0f).contains(&selector)
                                && !(0xe0100..=0xe01ef).contains(&selector)
                            {
                                return Err(error("invalid variation selector"));
                            }
                            data.glyph.variation_selectors.push((cp, selector));
                        }
                    }
                }
                "Comment" => {
                    data.glyph.note = if value.starts_with('"') {
                        strings::text(value)?
                    } else {
                        strings::decode(value)?
                    }
                }
                "HStem" => {
                    data.layer(layer, &master, &layer_defs)?.hints.h_stems = read_stems(value)?
                }
                "VStem" => {
                    data.layer(layer, &master, &layer_defs)?.hints.v_stems = read_stems(value)?
                }
                "LCarets2" => {
                    let t = strings::tokens(value)?;
                    let n = t.first().ok_or_else(|| error("missing caret count"))?;
                    if integer(n)? != t.len() as i64 - 1 {
                        return Err(error("caret count mismatch"));
                    }
                    for (i, x) in t[1..].iter().enumerate() {
                        data.glyph.layers[0].anchors.push(Anchor {
                            name: format!("caret_{}", i + 1),
                            x: number(x)?,
                            y: 0.0,
                        });
                    }
                }
                "Flags" => {}
                "GlyphClass" => {
                    data.glyph.category = match integer(value)? {
                        0 | 1 => None,
                        2 => Some("Letter".into()),
                        3 => Some("Ligature".into()),
                        4 => Some("Mark".into()),
                        _ => return Err(error("unsupported glyph class")),
                    };
                }
                _ => {
                    safe(key)?;
                    data.shadow.push(line.into());
                }
            }
        }
        if !ended_glyph {
            return Err(error("missing EndChar"));
        }
        let gid = data.gid.ok_or_else(|| error("glyph is missing Encoding"))?;
        if !(0..=MAX_GLYPHS as i64).contains(&gid)
            || ids.insert(gid, data.glyph.name.clone()).is_some()
        {
            return Err(error("invalid or duplicate original glyph index"));
        }
        pending.push(data);
        if pending.len() > MAX_GLYPHS || pending.len() > count {
            return Err(error("glyph count exceeds BeginChars"));
        }
    }
    if !ended || pending.len() != count {
        return Err(error("truncated or inconsistent glyph count"));
    }
    let tail: Vec<_> = lines.filter(|line| !line.is_empty()).collect();
    if tail != ["EndSplineFont"] {
        return Err(error(
            "missing EndSplineFont or unsupported bitmap/trailing data",
        ));
    }
    let native_kern_tables: BTreeSet<_> = pending
        .iter()
        .flat_map(|data| data.kerns.iter().map(|(_, _, table)| table))
        .chain(kern_classes.iter().map(|class| &class.table))
        .collect();
    if native_kern_tables.len() > 1 {
        return Err(error("multiple kerning lookups/classes cannot be merged without changing additive or subtable semantics"));
    }
    for mut data in pending {
        for (index, id, mut component) in data.references {
            component.base = ids
                .get(&id)
                .cloned()
                .ok_or_else(|| error("component references missing glyph index"))?;
            data.glyph.layers[index].components.push(component);
        }
        for (id, value, table) in data.kerns {
            let right = ids
                .get(&id)
                .ok_or_else(|| error("kerning references missing glyph index"))?;
            let lookup = find_lookup(&lookups, &table)?;
            if lookup.kind != 258 {
                return Err(error("kerning subtable has wrong lookup type"));
            }
            if lookup.simple_kern() {
                font.feature_settings.writer_policy.kerning = !lookup.features.is_empty();
                font.masters[0].kerning_h.0.insert(
                    (
                        KernSide::Glyph(data.glyph.name.clone()),
                        KernSide::Glyph(right.clone()),
                    ),
                    value,
                );
            } else {
                rules.entry(table).or_default().push(format!(
                    "pos {} {} {value};",
                    layout::fea_glyph(data.glyph.name.as_ref()),
                    layout::fea_glyph(right.as_ref())
                ));
            }
        }
        if !data.shadow.is_empty() {
            data.glyph.user_data = serde_json::json!({SHADOW:data.shadow});
        }
        font.insert_glyph(data.glyph)
            .map_err(|e| error(e.to_string()))?;
    }
    for class in kern_classes {
        install_kern_class(&mut font, &lookups, class)?;
    }
    for glyph in font.glyphs.values() {
        for layer in &glyph.layers {
            for component in &layer.components {
                if font.glyph(component.base.as_ref()).is_none() {
                    return Err(error("missing component base"));
                }
            }
        }
    }
    layout::install(&mut font, &lookups, &rules)?;
    layout::export(&font)?;
    let mut anchor_policy = None;
    let mut cursive_classes = 0;
    for (name, table) in anchor_classes {
        let lookup = find_lookup(&lookups, &table)?;
        if !matches!(lookup.kind, 259..=261) {
            return Err(error("anchor class references non-anchor lookup"));
        }
        if lookup.kind == 259 {
            cursive_classes += 1;
            if cursive_classes > 1 {
                return Err(error(
                    "multiple cursive anchor classes cannot be merged into one entry/exit model",
                ));
            }
        }
        if lookup.flags != 0
            || !lookup.features.iter().all(|(tag, scripts)| {
                matches!(tag.as_str(), "mark" | "mkmk" | "curs")
                    && scripts
                        .iter()
                        .all(|(script, langs)| script == "DFLT" && langs == &["dflt"])
            })
        {
            return Err(error(format!("anchor class {name:?} has unsupported script/flags; cannot preserve automatic mark semantics")));
        }
        let enabled = !lookup.features.is_empty();
        if anchor_policy.is_some_and(|previous| previous != enabled) {
            return Err(error(
                "mixed active/inactive anchor classes require a finer writer policy",
            ));
        }
        anchor_policy = Some(enabled);
    }
    if let Some(enabled) = anchor_policy {
        font.feature_settings.writer_policy.anchors = enabled;
    }
    font.extra
        .lib
        .insert(SHADOW.into(), serde_json::json!(shadow));
    Ok(font)
}

struct GlyphData {
    glyph: Glyph,
    gid: Option<i64>,
    references: Vec<(usize, i64, Component)>,
    kerns: Vec<(i64, f64, String)>,
    shadow: Vec<String>,
    layer_ids: BTreeMap<i64, usize>,
}
impl GlyphData {
    fn new(name: String, master: &MasterId) -> Self {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        Self {
            glyph,
            gid: None,
            references: Vec::new(),
            kerns: Vec::new(),
            shadow: Vec::new(),
            layer_ids: BTreeMap::from([(1, 0)]),
        }
    }
    fn layer_index(
        &mut self,
        id: i64,
        master: &MasterId,
        defs: &BTreeMap<i64, (String, bool, bool)>,
    ) -> Result<usize, Error> {
        if let Some(index) = self.layer_ids.get(&id) {
            return Ok(*index);
        }
        let (name, _, background) = defs
            .get(&id)
            .ok_or_else(|| error("undefined layer index"))?;
        let index = self.glyph.layers.len();
        let kind = if id == 0 {
            LayerKind::Background(master.clone())
        } else {
            LayerKind::Custom(name.clone())
        };
        if !background && id != 1 {
            return Err(error("multiple foreground layers are unsupported"));
        }
        self.glyph.layers.push(Layer {
            kind,
            name: name.clone(),
            ..Default::default()
        });
        self.layer_ids.insert(id, index);
        Ok(index)
    }
    fn layer(
        &mut self,
        id: i64,
        master: &MasterId,
        defs: &BTreeMap<i64, (String, bool, bool)>,
    ) -> Result<&mut Layer, Error> {
        let index = self.layer_index(id, master, defs)?;
        Ok(&mut self.glyph.layers[index])
    }
}

fn field(line: &str) -> Result<(&str, &str), Error> {
    line.split_once(':')
        .map(|(a, b)| (a, b.trim()))
        .ok_or_else(|| error(format!("unexpected SFD record {line:?}")))
}
fn boolean(value: &str) -> Result<bool, Error> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(error("expected 0 or 1")),
    }
}
fn codepoint(value: i64) -> Result<u32, Error> {
    let cp = u32::try_from(value).map_err(|_| error("invalid Unicode scalar"))?;
    char::from_u32(cp).ok_or_else(|| error("invalid Unicode scalar"))?;
    Ok(cp)
}
fn is_spline(line: &str) -> bool {
    line.split_whitespace().nth(2) == Some("m")
}
pub(super) fn find_lookup<'a>(lookups: &'a [Lookup], table: &str) -> Result<&'a Lookup, Error> {
    lookups
        .iter()
        .find(|lookup| lookup.subtables.iter().any(|name| name == table))
        .ok_or_else(|| error(format!("undefined lookup subtable {table:?}")))
}

fn read_reference(value: &str) -> Result<(i64, Component), Error> {
    let t = strings::tokens(value)?;
    if t.len() != 10 && t.len() != 12 {
        return Err(error("malformed reference"));
    }
    let id = integer(&t[0])?;
    integer(&t[1])?;
    if !matches!(t[2].as_str(), "N" | "S") {
        return Err(error("invalid reference selection flag"));
    }
    let mut matrix = [0.0; 6];
    for (target, source) in matrix.iter_mut().zip(&t[3..9]) {
        *target = number(source)?;
    }
    let flags = integer(&t[9])?;
    if flags & !7 != 0 {
        return Err(error("unsupported component flags"));
    }
    let point_match = if flags & 4 != 0 {
        if t.len() != 12 {
            return Err(error("missing component point match"));
        }
        Some((
            t[10].parse().map_err(|_| error("invalid point index"))?,
            t[11].parse().map_err(|_| error("invalid point index"))?,
        ))
    } else {
        None
    };
    Ok((
        id,
        Component {
            transform: Affine::new(matrix),
            use_my_metrics: flags & 1 != 0,
            round_to_grid: flags & 2 != 0,
            point_match,
            ..Default::default()
        },
    ))
}

fn read_anchor(value: &str) -> Result<Anchor, Error> {
    let t = strings::tokens(value)?;
    if t.len() != 5 {
        return Err(error(
            "anchor device tables or point matching are unsupported",
        ));
    }
    let index = integer(&t[4])?;
    let name = match t[3].as_str() {
        "basechar" => t[0].clone(),
        "basemark" => {
            return Err(error(
                "mark-to-mark anchor roles are not supported by SFD conversion",
            ))
        }
        "mark" => format!("_{}", t[0]),
        "baselig" => {
            if !(0..65535).contains(&index) {
                return Err(error("invalid ligature anchor index"));
            }
            format!("{}_{}", t[0], index + 1)
        }
        "entry" => "entry".into(),
        "exit" => "exit".into(),
        _ => return Err(error("unsupported anchor type")),
    };
    let anchor = Anchor {
        name,
        x: number(&t[1])?,
        y: number(&t[2])?,
    };
    if (t[3] == "basechar" && !matches!(anchor.kind(), fc_model::AnchorKind::Base(_)))
        || (t[3] == "baselig" && !matches!(anchor.kind(), fc_model::AnchorKind::Ligature { .. }))
    {
        return Err(error(
            "native anchor class name conflicts with model attachment-role conventions",
        ));
    }
    Ok(anchor)
}

fn read_stems(value: &str) -> Result<Vec<StemHint>, Error> {
    let t = strings::tokens(value)?;
    let mut pos = 0;
    let mut result = Vec::new();
    while pos < t.len() {
        if pos + 1 >= t.len() {
            return Err(error("truncated stem hint"));
        }
        let mut hint = StemHint {
            start: number(&t[pos])?,
            width: number(&t[pos + 1])?,
            ranges: Vec::new(),
        };
        pos += 2;
        if t.get(pos).map(String::as_str) == Some("<") {
            pos += 1;
            while t.get(pos).map(String::as_str) != Some(">") {
                if pos + 1 >= t.len() {
                    return Err(error("truncated stem range"));
                }
                hint.ranges.push((number(&t[pos])?, number(&t[pos + 1])?));
                pos += 2;
            }
            pos += 1;
        }
        result.push(hint);
    }
    Ok(result)
}
