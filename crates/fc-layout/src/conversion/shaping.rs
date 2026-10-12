//! Bounded full-position shaping cross-check, additional to graph equivalence.

use std::{
    collections::{BTreeSet, HashSet},
    str::FromStr,
};

use fc_compile::StaticCompile;
use fc_model::{Font, GlyphName, MasterId};
use harfrust::{
    Direction, Feature, Language, Script, ShapeOptions, Shaper, ShaperData, UnicodeBuffer,
};
use read_fonts::{types::Tag, FontRef};

use crate::{Error, LayoutImport};

const MAX_CONTEXTS: usize = 512;
const MAX_TEXTS: usize = 32;

pub(super) fn compare(
    original: &Font,
    imported: &LayoutImport,
    compiled: &StaticCompile,
    master: &MasterId,
) -> Result<(), Error> {
    // Reuse the candidate outlines/cmap so layout is the only compared input.
    let (baseline, _) = fc_compile::preserve_merge(
        &compiled.bytes,
        original,
        &HashSet::new(),
        &fc_compile::ExportOptions {
            master: Some(master.clone()),
            keep_tt_instructions: false,
            ..Default::default()
        },
    )?;
    let before = FontRef::new(&baseline).map_err(|error| Error::Parse(error.to_string()))?;
    for tag in [*b"GSUB", *b"GPOS", *b"GDEF", *b"BASE"] {
        let original_data = original
            .extra
            .raw_tables
            .get(&fc_model::Tag(tag))
            .map(Vec::as_slice);
        let actual_data = before.table_data(Tag::new(&tag));
        if actual_data.as_ref().map(|data| data.as_bytes()) != original_data {
            return Err(Error::LayoutConversion(format!(
                "{} original bytes were not preserved for the shaping baseline",
                fc_model::Tag(tag)
            )));
        }
    }
    let after = FontRef::new(&compiled.bytes).map_err(|error| Error::Parse(error.to_string()))?;
    let before_data = ShaperData::new(&before);
    let after_data = ShaperData::new(&after);
    let before_shaper = before_data.shaper(&before).build();
    let after_shaper = after_data.shaper(&after).build();
    let mut contexts = BTreeSet::new();
    for graph in imported.graphs() {
        for registration in &graph.registrations {
            contexts.insert((
                registration.script.clone(),
                registration.language.clone(),
                registration.feature.clone(),
            ));
        }
    }
    if contexts.len() > MAX_CONTEXTS {
        return Err(Error::LayoutConversion(
            "shaping context resource limit exceeded".into(),
        ));
    }
    let mut texts: BTreeSet<String> = [
        "fi",
        "ffi",
        "AV",
        "A\u{0301}\u{0301}",
        "\u{0644}\u{0627}",
        "\u{05d0}\u{05d1}",
        "abc",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for lookup in &imported.lookups {
        for rule in &lookup.rules {
            let text: String = rule
                .glyphs
                .iter()
                .filter_map(|name| {
                    original.glyph(name).and_then(|glyph| {
                        glyph
                            .unicodes
                            .iter()
                            .find_map(|value| char::from_u32(*value))
                    })
                })
                .take(64)
                .collect();
            if !text.is_empty() && texts.len() < MAX_TEXTS {
                texts.insert(text);
            }
        }
    }
    if contexts.is_empty() {
        contexts.insert(("DFLT".into(), "dflt".into(), "kern".into()));
    }
    for (script, language, feature) in contexts {
        let tag = Tag::new_checked(feature.as_bytes())
            .map_err(|error| Error::Parse(error.to_string()))?;
        for direction in [
            Direction::LeftToRight,
            Direction::RightToLeft,
            Direction::TopToBottom,
            Direction::BottomToTop,
        ] {
            for value in [0, 1, 2] {
                let features = [Feature::new(tag, value, ..)];
                for text in &texts {
                    let probe = Probe {
                        text,
                        script: &script,
                        language: &language,
                        direction,
                        features: &features,
                    };
                    let a = shape(&before_shaper, imported.glyph_order(), &probe)?;
                    let b = shape(&after_shaper, &compiled.glyph_order, &probe)?;
                    if a != b {
                        return Err(Error::LayoutConversion(format!(
                            "shaping differs for {script}/{language}/{feature}={value}, {direction:?}, {text:?}"
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

struct Probe<'a> {
    text: &'a str,
    script: &'a str,
    language: &'a str,
    direction: Direction,
    features: &'a [Feature],
}

#[derive(Debug, PartialEq, Eq)]
struct PositionedGlyph {
    name: GlyphName,
    cluster: u32,
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

fn shape(
    shaper: &Shaper<'_>,
    names: &[GlyphName],
    probe: &Probe<'_>,
) -> Result<Vec<PositionedGlyph>, Error> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(probe.text);
    buffer.set_direction(probe.direction);
    if let Ok(script) = Script::from_str(probe.script) {
        buffer.set_script(script);
    }
    // HarfRust's OpenType private-use tags select exact script/LangSys tags,
    // including registrations without a corresponding BCP-47 language code.
    if let Some(language) = Language::new(format!("x-hbsc{}-hbot{}", probe.script, probe.language))
    {
        buffer.set_language(language);
    }
    buffer.guess_segment_properties();
    let result = shaper.shape(buffer, ShapeOptions::new().features(probe.features));
    result
        .glyph_infos()
        .iter()
        .zip(result.glyph_positions())
        .map(|(info, position)| {
            let name = names.get(info.glyph_id as usize).cloned().ok_or_else(|| {
                Error::LayoutConversion("shaper returned an unmapped glyph".into())
            })?;
            Ok(PositionedGlyph {
                name,
                cluster: info.cluster,
                x_advance: position.x_advance,
                y_advance: position.y_advance,
                x_offset: position.x_offset,
                y_offset: position.y_offset,
            })
        })
        .collect()
}
