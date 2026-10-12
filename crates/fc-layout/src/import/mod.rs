//! Checked, bounded static OpenType layout inventory and source conversion.
mod base;
pub(crate) mod binary;
mod compare;
mod context;
mod gdef;
mod gpos;
pub(crate) mod graph;
mod gsub;
mod writer;

use crate::Error;
use binary::Reader;
pub use compare::equivalent_layout;
use fc_model::{FeaturesSource, Font, GlyphName, Tag};
use graph::Graph;
use read_fonts::{FontRef, TableProvider};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const OWNED: [[u8; 4]; 4] = [*b"GSUB", *b"GPOS", *b"GDEF", *b"BASE"];
const COMPETING: [[u8; 4]; 5] = [*b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF"];
const MAX_TABLE_BYTES: usize = 32 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 1024 * 1024;

/// A precise reason the whole imported layout cannot be converted safely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutBlocker {
    /// Table or authority involved in the rejection.
    pub table: String,
    /// Lookup index, when the rejection is specific to one lookup.
    pub lookup: Option<u16>,
    /// Human-readable explanation of the rejected structure.
    pub reason: String,
}
/// Bounded table inventory, including preserved competing layout authorities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutTableSummary {
    /// Four-byte sfnt table tag.
    pub tag: String,
    /// Size of the preserved table in bytes.
    pub byte_length: usize,
    /// Number of lookup headers available for inspection.
    pub lookup_count: usize,
}
/// One bounded rendering of an imported rule for the read-only inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutRule {
    /// FEA-style rendering, limited to 4096 Unicode scalar values.
    pub text: String,
    /// At most 64 referenced glyph names for glyph previews.
    pub glyphs: Vec<GlyphName>,
    /// More text or glyph references exist in this rule.
    pub truncated: bool,
}
/// An ordered imported lookup and its bounded inspector rule records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutLookup {
    /// GSUB or GPOS.
    pub table: String,
    /// Original zero-based lookup index.
    pub index: u16,
    /// Effective lookup kind, unwrapping extensions for supported lookups.
    pub kind: u16,
    /// Exact original lookup flags.
    pub flags: u16,
    /// Ordered contextual lookup dependencies.
    pub referenced_lookups: Vec<u16>,
    /// Bounded rule previews; callers must page this vector.
    pub rules: Vec<LayoutRule>,
}
/// Runtime-only parsed import, rebuilt from native raw tables and provenance.
#[derive(Clone, Debug, Default)]
pub struct LayoutImport {
    /// SHA-256 of owned/competing layout tables and imported glyph provenance.
    pub fingerprint: String,
    /// All original owned/competing tables, even when conversion is blocked.
    pub tables: Vec<LayoutTableSummary>,
    /// Original ordered lookups; API consumers page records before serialization.
    pub lookups: Vec<LayoutLookup>,
    /// Any unsupported, malformed or resource-limited data blocks conversion.
    pub blockers: Vec<LayoutBlocker>,
    glyph_order: Vec<GlyphName>,
    graphs: Vec<Graph>,
    inspection_graphs: Vec<Graph>,
    source: String,
    shared_source: String,
    gdef_source: String,
    base_source: String,
    mark_attach: BTreeMap<u16, Vec<u16>>,
    mark_sets: Vec<Vec<u16>>,
}
impl LayoutImport {
    /// Hash preserved layout and original glyph order without parsing table contents.
    pub fn fingerprint_for_font(font: &Font) -> String {
        let names = font
            .extra
            .source
            .as_ref()
            .map(|source| source.glyph_order.as_slice())
            .unwrap_or_default();
        fingerprint(&font.extra.raw_tables, names)
    }
    /// Inspect preserved layout without mutating the font or changing authority.
    pub fn from_font(font: &Font) -> Self {
        let order = font
            .extra
            .source
            .as_ref()
            .map(|s| s.glyph_order.clone())
            .unwrap_or_default();
        let mut result = Self::from_tables(&font.extra.raw_tables, &order);
        let recorded_conversion = font.layout_conversion.as_ref().is_some_and(|conversion| {
            conversion.baseline_fingerprint == result.fingerprint
                && conversion.original_glyph_order == order
                && conversion.original_features.source_of_truth == FeaturesSource::ImportedTables
        });
        if !result.tables.is_empty()
            && font.features.source_of_truth != FeaturesSource::ImportedTables
            && !recorded_conversion
        {
            result.block("authority", None, "source authority contradicts preserved imported layout; explicit provenance is required".into());
        }
        if !result.tables.is_empty() {
            if order.is_empty() {
                result.block("provenance", None, "original glyph order is missing".into());
            }
            let current: Vec<_> = font.glyph_order().cloned().collect();
            if order != current {
                result.block(
                    "provenance",
                    None,
                    "current glyph order differs from the original layout glyph order".into(),
                );
            }
        }
        result
    }
    /// Inspect a compiled binary using its verified glyph-ID to name mapping.
    pub fn from_bytes(bytes: &[u8], glyph_order: &[GlyphName]) -> Result<Self, Error> {
        let font =
            FontRef::from_index(bytes, 0).map_err(|error| Error::Parse(error.to_string()))?;
        let glyph_count = font
            .maxp()
            .map_err(|error| Error::Parse(error.to_string()))?
            .num_glyphs() as usize;
        if glyph_count != glyph_order.len() {
            return Err(Error::LayoutConversion(
                "compiled glyph-order mapping length differs from maxp".into(),
            ));
        }
        let mut tables = BTreeMap::new();
        for tag in OWNED.iter().chain(&COMPETING) {
            if let Some(table) = font.table_data(read_fonts::types::Tag::new(tag)) {
                tables.insert(Tag(*tag), table.as_bytes().to_vec());
            }
        }
        Ok(Self::from_tables(&tables, glyph_order))
    }
    /// Full checked glyph references for one rule; callers must page before sending.
    pub fn rule_glyphs(&self, table: &str, lookup: u16, rule: usize) -> Option<Vec<GlyphName>> {
        let graph = self
            .graphs
            .iter()
            .chain(&self.inspection_graphs)
            .find(|graph| graph.tag == table)?;
        let gids = graph.lookups.get(lookup as usize)?.rules.glyphs.get(rule)?;
        Some(
            gids.iter()
                .filter_map(|gid| self.glyph_order.get(*gid as usize).cloned())
                .collect(),
        )
    }
    pub(crate) fn graphs(&self) -> impl Iterator<Item = &graph::Graph> {
        self.graphs.iter()
    }
    pub(crate) fn gpos_graph(&self) -> Option<&graph::Graph> {
        self.graphs.iter().find(|graph| graph.tag == "GPOS")
    }
    /// Render a candidate omitting whole GPOS lookups for independently verified lifting.
    /// This is not an equivalence proof; callers must compile and compare the result.
    pub fn decompile_excluding_gpos(&self, indices: &[u16]) -> Result<String, Error> {
        self.decompile()?;
        let excluded: BTreeSet<_> = indices.iter().copied().collect();
        let Some(gpos) = self.gpos_graph() else {
            return Ok(self.source.clone());
        };
        if excluded
            .iter()
            .any(|index| *index as usize >= gpos.lookups.len())
            || gpos.lookups.iter().enumerate().any(|(index, lookup)| {
                !excluded.contains(&(index as u16))
                    && lookup
                        .rules
                        .references
                        .iter()
                        .any(|id| excluded.contains(id))
            })
        {
            return Err(Error::LayoutConversion(
                "cannot lift a missing or contextually referenced lookup".into(),
            ));
        }
        Ok(self.render_source(&excluded))
    }
    /// Whether every stored layout structure has a source representation.
    /// Actual candidate compilation and equivalence are still required.
    pub fn can_convert(&self) -> bool {
        self.blockers.is_empty()
    }
    /// Return deterministic complete FEA, refusing any partial conversion.
    pub fn decompile(&self) -> Result<String, Error> {
        if !self.can_convert() {
            return Err(Error::LayoutConversion(
                self.blockers
                    .iter()
                    .map(|b| format!("{}: {}", b.table, b.reason))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        Ok(self.source.clone())
    }
    /// Original glyph names in the order used to interpret binary glyph IDs.
    pub fn glyph_order(&self) -> &[GlyphName] {
        &self.glyph_order
    }
    fn block(&mut self, table: &str, lookup: Option<u16>, reason: String) {
        self.blockers.push(LayoutBlocker {
            table: table.into(),
            lookup,
            reason,
        });
    }
    fn from_tables(tables: &BTreeMap<Tag, Vec<u8>>, names: &[GlyphName]) -> Self {
        let mut result = Self {
            glyph_order: names.to_vec(),
            ..Self::default()
        };
        for (tag, bytes) in tables
            .iter()
            .filter(|(tag, _)| OWNED.contains(&tag.0) || COMPETING.contains(&tag.0))
        {
            result.tables.push(LayoutTableSummary {
                tag: tag.to_string(),
                byte_length: bytes.len(),
                lookup_count: 0,
            });
        }
        result.fingerprint = fingerprint(tables, names);
        if names.len() > binary::MAX_GLYPHS
            || names.iter().collect::<BTreeSet<_>>().len() != names.len()
        {
            result.block(
                "provenance",
                None,
                "invalid or excessive glyph order".into(),
            );
            return result;
        }
        for tag in COMPETING {
            if tables.contains_key(&Tag(tag)) {
                result.block(
                    &Tag(tag).to_string(),
                    None,
                    "competing layout authority or lookup-index dependency is not convertible"
                        .into(),
                );
            }
        }
        for tag in OWNED {
            let Some(bytes) = tables.get(&Tag(tag)) else {
                continue;
            };
            let name = Tag(tag).to_string();
            if bytes.len() > MAX_TABLE_BYTES {
                result.block(&name, None, "table byte limit exceeded".into());
                continue;
            }
            let reader = Reader::new(bytes);
            match &tag {
                b"GSUB" | b"GPOS" => match Graph::inspect(&name, reader, names) {
                    Ok((graph, blocker)) => {
                        for (index, lookup) in graph.lookups.iter().enumerate() {
                            let rules = lookup
                                .rules
                                .statements
                                .iter()
                                .zip(&lookup.rules.glyphs)
                                .map(|(text, gids)| LayoutRule {
                                    text: text.chars().take(4096).collect(),
                                    glyphs: gids
                                        .iter()
                                        .take(64)
                                        .filter_map(|gid| names.get(*gid as usize).cloned())
                                        .collect(),
                                    truncated: text.chars().count() > 4096 || gids.len() > 64,
                                })
                                .collect();
                            result.lookups.push(LayoutLookup {
                                table: name.clone(),
                                index: index as u16,
                                kind: lookup.kind,
                                flags: lookup.flags,
                                referenced_lookups: lookup.rules.references.clone(),
                                rules,
                            });
                        }
                        if let Some(summary) = result.tables.iter_mut().find(|t| t.tag == name) {
                            summary.lookup_count = graph.lookups.len();
                        }
                        if let Some(error) = blocker {
                            result.block(&name, None, error);
                            result.inspection_graphs.push(graph);
                        } else {
                            result.graphs.push(graph);
                        }
                    }
                    Err(error) => {
                        let lookup = error
                            .strip_prefix("lookup ")
                            .and_then(|message| message.split([',', ':']).next())
                            .and_then(|index| index.parse().ok());
                        result.block(&name, lookup, error);
                        result.inspect_headers(&name, reader);
                    }
                },
                b"GDEF" => match gdef::parse(reader, names) {
                    Ok(data) => {
                        result.gdef_source = data.table_source;
                        result.mark_attach = data.mark_attach;
                        result.mark_sets = data.mark_sets;
                        for line in data.preamble {
                            result.shared_source.push_str(&line);
                            result.shared_source.push('\n');
                        }
                    }
                    Err(error) => result.block(&name, None, error),
                },
                b"BASE" => match base::parse(reader, names) {
                    Ok(source) => result.base_source = source,
                    Err(error) => result.block(&name, None, error),
                },
                _ => (),
            }
        }
        result.check_filters();
        let source = result.render_source(&BTreeSet::new());
        if source.len() > MAX_SOURCE_BYTES {
            result.block(
                "source",
                None,
                "decompiled source byte limit exceeded".into(),
            );
        } else if result.blockers.is_empty() {
            if let Err(error) = fc_compile::check_feature_source_budget(&source, names) {
                result.block("source", None, error);
            }
        }
        result.source = source;
        result
    }
    fn render_source(&self, excluded_gpos: &BTreeSet<u16>) -> String {
        let mut languages = BTreeSet::new();
        for graph in &self.graphs {
            for registration in &graph.registrations {
                if graph.tag == "GPOS"
                    && registration
                        .lookups
                        .iter()
                        .all(|id| excluded_gpos.contains(id))
                {
                    continue;
                }
                languages.insert((registration.script.clone(), registration.language.clone()));
            }
        }
        let mut source = String::new();
        for (script, language) in languages {
            source.push_str(&format!("languagesystem {script} {language};\n"));
        }
        source.push_str(&self.shared_source);
        source.push_str(&self.gdef_source);
        source.push_str(&self.base_source);
        for graph in &self.graphs {
            if graph.tag == "GPOS" {
                source.push_str(&graph.source_excluding(excluded_gpos));
            } else {
                source.push_str(&graph.source());
            }
        }
        writer::compact_classes(&source)
    }
    fn inspect_headers(&mut self, tag: &str, reader: Reader<'_>) {
        let Ok(list) = reader.offset16(8) else {
            return;
        };
        let Ok(count) = list.u16(0) else {
            return;
        };
        if let Some(summary) = self.tables.iter_mut().find(|t| t.tag == tag) {
            summary.lookup_count = count as usize;
        }
        for index in 0..usize::from(count).min(graph::MAX_LOOKUPS) {
            let Ok(lookup) = list.offset16(2 + index * 2) else {
                continue;
            };
            let (Ok(kind), Ok(flags)) = (lookup.u16(0), lookup.u16(2)) else {
                continue;
            };
            self.lookups.push(LayoutLookup {
                table: tag.into(),
                index: index as u16,
                kind,
                flags,
                referenced_lookups: Vec::new(),
                rules: Vec::new(),
            });
        }
    }
    fn check_filters(&mut self) {
        let mut attach_used = BTreeSet::new();
        let mut sets_used = BTreeSet::new();
        let mut errors = Vec::new();
        for graph in self.graphs.iter().chain(&self.inspection_graphs) {
            for (index, lookup) in graph.lookups.iter().enumerate() {
                let attach = lookup.flags >> 8;
                if attach != 0 {
                    attach_used.insert(attach);
                    if !self.mark_attach.contains_key(&attach) {
                        errors.push((
                            graph.tag.clone(),
                            index,
                            "missing GDEF mark attachment class",
                        ));
                    }
                }
                if let Some(filter) = lookup.filter {
                    sets_used.insert(filter);
                    if filter as usize >= self.mark_sets.len() {
                        errors.push((graph.tag.clone(), index, "missing GDEF mark filtering set"));
                    }
                }
            }
        }
        for (table, index, message) in errors {
            self.block(&table, Some(index as u16), message.into());
        }
        if attach_used.len() != self.mark_attach.len() {
            self.block(
                "GDEF",
                None,
                "unused mark attachment classes cannot be retained by compiler".into(),
            );
        }
        if sets_used.len() != self.mark_sets.len() {
            self.block(
                "GDEF",
                None,
                "unused mark filtering sets cannot be retained by compiler".into(),
            );
        }
    }
}

fn fingerprint(tables: &BTreeMap<Tag, Vec<u8>>, names: &[GlyphName]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"fontcarver-layout-v1");
    hash.update((names.len() as u64).to_be_bytes());
    for name in names {
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
    }
    for (tag, bytes) in tables
        .iter()
        .filter(|(tag, _)| OWNED.contains(&tag.0) || COMPETING.contains(&tag.0))
    {
        hash.update(tag.0);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    format!("{:x}", hash.finalize())
}
