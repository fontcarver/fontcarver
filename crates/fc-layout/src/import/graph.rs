//! Ordered OpenType script, feature and lookup graph.
use super::{
    binary::{Reader, Result, Rules},
    context, gpos, gsub,
};
use fc_model::GlyphName;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_LOOKUPS: usize = 4096;
#[derive(Clone, Debug)]
pub(crate) struct Lookup {
    pub kind: u16,
    pub flags: u16,
    pub filter: Option<u16>,
    pub rules: Rules,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Registration {
    pub script: String,
    pub language: String,
    pub feature: String,
    pub required: bool,
    pub lookups: Vec<u16>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Graph {
    pub tag: String,
    pub lookups: Vec<Lookup>,
    pub registrations: Vec<Registration>,
}
impl Graph {
    pub fn inspect(
        tag: &str,
        reader: Reader<'_>,
        names: &[GlyphName],
    ) -> Result<(Self, Option<String>)> {
        if reader.u16(0)? != 1 || reader.u16(2)? > 1 {
            return Err("unsupported layout table version".into());
        }
        if reader.u16(2)? == 1 && reader.u32(10)? != 0 {
            return Err("FeatureVariations are unsupported".into());
        }
        let list = reader.offset16(8)?;
        let count = list.u16(0)? as usize;
        if count > MAX_LOOKUPS {
            return Err("lookup count exceeds resource limit".into());
        }
        let mut graph = Self {
            tag: tag.into(),
            ..Self::default()
        };
        let mut total = 0;
        let mut subtable_count = 0usize;
        let mut text_bytes = 0usize;
        let mut glyph_references = 0usize;
        for index in 0..count {
            let lookup = list.offset16(2 + index * 2)?;
            let mut kind = lookup.u16(0)?;
            let flags = lookup.u16(2)?;
            if flags & 0x00e0 != 0 {
                return Err(format!("lookup {index}: reserved flag bits are set"));
            }
            let sub_count = lookup.u16(4)? as usize;
            subtable_count = subtable_count.saturating_add(sub_count);
            if subtable_count > super::binary::MAX_RULES {
                return Err("subtable count exceeds resource limit".into());
            }
            if sub_count == 0 {
                return Err(format!(
                    "lookup {index}: empty lookup cannot be retained by compiler"
                ));
            }
            let filter = (flags & 0x10 != 0)
                .then(|| lookup.u16(6 + sub_count * 2))
                .transpose()?;
            let mut all = Rules::default();
            let mut wrapped_kind = None;
            for sub in 0..sub_count {
                let mut reader = lookup.offset16(6 + sub * 2)?;
                let mut sub_kind = kind;
                let extension = if tag == "GSUB" { 7 } else { 9 };
                if kind == extension {
                    if reader.u16(0)? != 1 {
                        return Err(format!("lookup {index}: unsupported extension format"));
                    }
                    sub_kind = reader.u16(2)?;
                    if sub_kind == extension
                        || wrapped_kind.is_some_and(|previous| previous != sub_kind)
                    {
                        return Err(format!("lookup {index}: recursive or mixed extension"));
                    }
                    wrapped_kind = Some(sub_kind);
                    let offset = reader.u32(4)? as usize;
                    if offset == 0 {
                        return Err("null extension offset".into());
                    }
                    reader = reader.at(offset)?;
                }
                let name = format!("FC_{tag}_{index}_{sub}");
                let mut rules = match (tag, sub_kind) {
                    ("GSUB", 5 | 6) => context::parse(reader, sub_kind == 6, false, names, tag),
                    ("GPOS", 7 | 8) => context::parse(reader, sub_kind == 8, true, names, tag),
                    ("GSUB", _) => gsub::parse(reader, sub_kind, names, &name),
                    ("GPOS", _) => gpos::parse(reader, sub_kind, names, &name),
                    _ => Err("unknown layout table".into()),
                }
                .map_err(|error| format!("lookup {index}, subtable {sub}: {error}"))?;
                if rules.statements.is_empty() {
                    return Err(format!("lookup {index}, subtable {sub}: empty subtable cannot be retained by the compiler"));
                }
                total += rules.statements.len() + usize::from(sub > 0);
                if sub > 0 {
                    text_bytes = text_bytes.saturating_add("subtable;".len());
                }
                text_bytes = text_bytes
                    .saturating_add(rules.statements.iter().map(String::len).sum::<usize>())
                    .saturating_add(rules.preamble.iter().map(String::len).sum::<usize>());
                glyph_references = glyph_references
                    .saturating_add(rules.glyphs.iter().map(Vec::len).sum::<usize>());
                if text_bytes > super::binary::MAX_TEXT_BYTES || glyph_references > 1_600_000 {
                    return Err("table source expansion resource limit".into());
                }
                if total > super::binary::MAX_RULES {
                    return Err("table rule count exceeds resource limit".into());
                }
                if sub > 0 {
                    all.statements.push("subtable;".into());
                    all.glyphs.push(Vec::new());
                }
                all.preamble.append(&mut rules.preamble);
                all.statements.append(&mut rules.statements);
                all.glyphs.append(&mut rules.glyphs);
                all.references.append(&mut rules.references);
            }
            if let Some(inner) = wrapped_kind {
                kind = inner;
            }
            graph.lookups.push(Lookup {
                kind,
                flags,
                filter,
                rules: all,
            });
        }
        match registrations(reader, count) {
            Ok(registrations) => graph.registrations = registrations,
            Err(error) => return Ok((graph, Some(error))),
        }
        let blocker = graph.check_graph().err();
        Ok((graph, blocker))
    }
    fn check_graph(&self) -> Result<()> {
        fn visit(index: usize, graph: &Graph, state: &mut [u8], depth: usize) -> Result<()> {
            if depth > 64 {
                return Err("context lookup recursion limit exceeded".into());
            }
            match state.get(index) {
                None => return Err("context references missing lookup".into()),
                Some(1) => return Err("cyclic contextual lookup references".into()),
                Some(2) => return Ok(()),
                _ => (),
            }
            state[index] = 1;
            for next in &graph.lookups[index].rules.references {
                visit(*next as usize, graph, state, depth + 1)?;
            }
            state[index] = 2;
            Ok(())
        }
        let mut state = vec![0; self.lookups.len()];
        for index in 0..self.lookups.len() {
            visit(index, self, &mut state, 0)?;
        }
        // Unreachable lookups cannot be kept in a font by the feature compiler.
        let mut reachable = BTreeSet::new();
        let mut pending: Vec<_> = self
            .registrations
            .iter()
            .flat_map(|r| r.lookups.iter().copied())
            .collect();
        while let Some(index) = pending.pop() {
            if reachable.insert(index) {
                pending.extend(&self.lookups[index as usize].rules.references);
            }
        }
        if reachable.len() != self.lookups.len() {
            return Err("unreachable lookup would be discarded by compilation".into());
        }
        Ok(())
    }
    pub fn source(&self) -> String {
        self.source_excluding(&BTreeSet::new())
    }
    pub fn source_excluding(&self, excluded: &BTreeSet<u16>) -> String {
        let mut output = String::new();
        for (index, lookup) in self
            .lookups
            .iter()
            .enumerate()
            .filter(|(index, _)| !excluded.contains(&(*index as u16)))
        {
            let _ = index;
            for preamble in &lookup.rules.preamble {
                output.push_str(preamble);
                output.push('\n');
            }
        }
        fn ordered(
            index: usize,
            graph: &Graph,
            visited: &mut BTreeSet<usize>,
            order: &mut Vec<usize>,
        ) {
            if visited.insert(index) {
                for dependency in &graph.lookups[index].rules.references {
                    ordered(*dependency as usize, graph, visited, order);
                }
                order.push(index);
            }
        }
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        for index in 0..self.lookups.len() {
            ordered(index, self, &mut visited, &mut order);
        }
        for index in order {
            let lookup = &self.lookups[index];
            if excluded.contains(&(index as u16)) {
                continue;
            }
            let name = format!("FC_{}_{index}", self.tag);
            output.push_str(&format!("lookup {name} {{\n"));
            let mut flags = Vec::new();
            for (mask, text) in [
                (1, "RightToLeft"),
                (2, "IgnoreBaseGlyphs"),
                (4, "IgnoreLigatures"),
                (8, "IgnoreMarks"),
            ] {
                if lookup.flags & mask != 0 {
                    flags.push(text.to_owned());
                }
            }
            if lookup.flags >> 8 != 0 {
                flags.push(format!(
                    "MarkAttachmentType @FC_MARK_ATTACH_{}",
                    lookup.flags >> 8
                ));
            }
            if let Some(filter) = lookup.filter {
                flags.push(format!("UseMarkFilteringSet @FC_MARK_FILTER_{filter}"));
            }
            output.push_str(&format!(
                "  lookupflag {};\n",
                if flags.is_empty() {
                    "0".into()
                } else {
                    flags.join(" ")
                }
            ));
            for statement in &lookup.rules.statements {
                output.push_str("  ");
                output.push_str(statement);
                output.push('\n');
            }
            output.push_str(&format!("}} {name};\n"));
        }
        let mut features: BTreeMap<&str, Vec<&Registration>> = BTreeMap::new();
        for registration in self
            .registrations
            .iter()
            .filter(|registration| registration.lookups.iter().any(|id| !excluded.contains(id)))
        {
            features
                .entry(&registration.feature)
                .or_default()
                .push(registration);
        }
        for (feature, registrations) in features {
            output.push_str(&format!("feature {feature} {{\n"));
            for registration in registrations {
                output.push_str(&format!(
                    "  script {};\n  language {} exclude_dflt{};\n",
                    registration.script,
                    registration.language,
                    if registration.required {
                        " required"
                    } else {
                        ""
                    }
                ));
                for index in registration
                    .lookups
                    .iter()
                    .filter(|id| !excluded.contains(id))
                {
                    output.push_str(&format!("  lookup FC_{}_{index};\n", self.tag));
                }
            }
            output.push_str(&format!("}} {feature};\n"));
        }
        output
    }
}
fn registrations(reader: Reader<'_>, lookup_count: usize) -> Result<Vec<Registration>> {
    let feature_list = reader.offset16(6)?;
    let count = feature_list.u16(0)? as usize;
    if count > MAX_LOOKUPS {
        return Err("feature count exceeds resource limit".into());
    }
    let mut feature_references = 0usize;
    let mut registration_references = 0usize;
    let mut features = Vec::new();
    let mut previous_feature: Option<String> = None;
    for index in 0..count {
        let tag = feature_list.tag(2 + index * 6)?;
        if previous_feature
            .as_ref()
            .is_some_and(|previous| previous > &tag)
        {
            return Err("feature records are not ordered".into());
        }
        previous_feature = Some(tag.clone());
        let feature = feature_list.offset16(6 + index * 6)?;
        if feature.u16(0)? != 0 {
            return Err(format!("feature {tag}: FeatureParams are unsupported"));
        }
        let reference_count = feature.u16(2)? as usize;
        feature_references = feature_references.saturating_add(reference_count);
        if feature_references > super::binary::MAX_RULES {
            return Err("feature lookup references exceed resource limit".into());
        }
        let lookups = feature.array(4, reference_count)?;
        if lookups.is_empty() {
            return Err(format!(
                "feature {tag}: empty feature cannot be retained by compiler"
            ));
        }
        if lookups.iter().any(|i| *i as usize >= lookup_count) {
            return Err(format!("feature {tag}: lookup index out of range"));
        }
        features.push((tag, lookups));
    }
    let scripts = reader.offset16(4)?;
    let mut result = Vec::new();
    let mut used = BTreeSet::new();
    let mut script_tags = BTreeSet::new();
    let mut previous_script: Option<String> = None;
    let script_count = scripts.u16(0)? as usize;
    if script_count > MAX_LOOKUPS {
        return Err("script count exceeds resource limit".into());
    }
    let mut language_count = 0usize;
    for index in 0..script_count {
        let tag = scripts.tag(2 + index * 6)?;
        if previous_script
            .as_ref()
            .is_some_and(|previous| previous >= &tag)
        {
            return Err("script records are not strictly ordered".into());
        }
        previous_script = Some(tag.clone());
        if !script_tags.insert(tag.clone()) {
            return Err("duplicate script record".into());
        }
        let script = scripts.offset16(6 + index * 6)?;
        let mut languages = Vec::new();
        if script.u16(0)? != 0 {
            languages.push(("dflt".into(), script.offset16(0)?));
        }
        let count = script.u16(2)? as usize;
        language_count = language_count.saturating_add(count + usize::from(script.u16(0)? != 0));
        if language_count > MAX_LOOKUPS {
            return Err("language count exceeds resource limit".into());
        }
        let mut previous_language: Option<String> = None;
        for language in 0..count {
            let language_tag = script.tag(4 + language * 6)?;
            if previous_language
                .as_ref()
                .is_some_and(|previous| previous >= &language_tag)
            {
                return Err("language records are not strictly ordered".into());
            }
            previous_language = Some(language_tag.clone());
            languages.push((language_tag, script.offset16(8 + language * 6)?));
        }
        if languages.is_empty() {
            return Err(format!("script {tag}: empty script cannot be retained"));
        }
        let mut language_tags = BTreeSet::new();
        for (language, lang) in languages {
            if !language_tags.insert(language.clone()) {
                return Err("duplicate language record".into());
            }
            if lang.u16(0)? != 0 {
                return Err("reserved LangSys lookupOrder is not zero".into());
            }
            let required = lang.u16(2)?;
            let reference_count = lang.u16(4)? as usize;
            if reference_count > MAX_LOOKUPS
                || result.len().saturating_add(reference_count + 1) > super::binary::MAX_RULES
            {
                return Err("language feature references exceed resource limit".into());
            }
            let mut ids = lang.array(6, reference_count)?;
            if required != u16::MAX {
                ids.insert(0, required);
            }
            if ids.is_empty() {
                return Err(format!(
                    "script {tag}, language {language}: empty language system cannot be retained"
                ));
            }
            let mut language_features = BTreeSet::new();
            for id in ids {
                let (feature, lookups) = features
                    .get(id as usize)
                    .ok_or("language references missing feature")?;
                if !language_features.insert(feature.clone()) {
                    return Err("duplicate feature tag in one language system".into());
                }
                registration_references = registration_references.saturating_add(lookups.len());
                if registration_references > super::binary::MAX_RULES {
                    return Err("registered lookup references exceed resource limit".into());
                }
                used.insert(id);
                result.push(Registration {
                    script: tag.clone(),
                    language: language.clone(),
                    feature: feature.clone(),
                    required: id == required,
                    lookups: lookups.clone(),
                });
            }
        }
    }
    if used.len() != features.len() {
        return Err("unreachable feature would be discarded by compilation".into());
    }
    result.sort();
    Ok(result)
}
