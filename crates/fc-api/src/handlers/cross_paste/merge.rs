//! Conflict-resolved font merging in one atomic history transaction.

use std::collections::{BTreeMap, BTreeSet};

use fc_doc::{kerning::KerningGroupSide, Command, Document, Session};
use fc_model::{Font, GlyphId, GlyphName, KernSide, MasterId};

use crate::{
    Api, ApiError, FontMergeParams, FontMergeResult, MergeChoice, PasteContent, PasteDestination,
    PasteGlyphMapping, PasteSpecialParams,
};

use super::{check_generation, plan};

const MAX_GLYPHS: usize = 4096;
const MAX_KERN_COMMANDS: usize = 50_000;
const LABEL: &str = "Merge fonts";

fn master(font: &Font, requested: Option<&MasterId>) -> Result<MasterId, ApiError> {
    let id = requested.unwrap_or(&font.default_master().id);
    font.master(id)
        .map(|master| master.id.clone())
        .ok_or_else(|| ApiError::not_found("selected merge master does not exist"))
}

/// Whether a kerning side still resolves after the merge.
fn resolvable(side: &KernSide, glyphs: &BTreeSet<GlyphName>, groups: &BTreeSet<String>) -> bool {
    match side {
        KernSide::Glyph(name) => glyphs.contains(name),
        KernSide::Group(group) => groups.contains(group),
    }
}

/// Builds the group and record commands that adopt the reference's kerning.
///
/// Every destination group is emptied first: a glyph that moves between groups
/// would otherwise be rejected as a membership conflict.
fn kerning_commands(
    source: &Font,
    target: &Font,
    source_master: &MasterId,
    target_master: &MasterId,
    known: &BTreeSet<GlyphName>,
    commands: &mut Vec<Command>,
    warnings: &mut Vec<String>,
) -> Result<(), ApiError> {
    let mut adopted_groups = BTreeSet::new();
    let mut skipped_members = 0usize;
    let mut removals = Vec::new();
    for side in [KerningGroupSide::Side1, KerningGroupSide::Side2] {
        let source_groups = side.groups(source).clone();
        let target_groups = side.groups(target).clone();
        for name in target_groups.keys() {
            commands.push(Command::SetKerningGroup {
                side,
                name: name.clone(),
                members: Vec::new(),
            });
            if !source_groups.contains_key(name) {
                removals.push(Command::RemoveKerningGroup {
                    side,
                    name: name.clone(),
                    remove_pairs: true,
                });
            }
        }
        for (name, members) in &source_groups {
            let kept: Vec<GlyphName> = members
                .iter()
                .filter(|member| known.contains(*member))
                .cloned()
                .collect();
            skipped_members += members.len() - kept.len();
            adopted_groups.insert(name.clone());
            commands.push(Command::SetKerningGroup {
                side,
                name: name.clone(),
                members: kept,
            });
        }
    }
    commands.extend(removals);
    if skipped_members > 0 {
        warnings.push(format!(
            "{skipped_members} kerning group members were skipped because their glyphs are absent from the destination."
        ));
    }
    let mut records = 0usize;
    let mut skipped_records = 0usize;
    for vertical in [false, true] {
        let table = |font: &Font, id: &MasterId| {
            font.master(id).map(|master| {
                if vertical {
                    master.kerning_v.clone()
                } else {
                    master.kerning_h.clone()
                }
            })
        };
        let incoming = table(source, source_master)
            .ok_or_else(|| ApiError::not_found("merge reference master does not exist"))?;
        let existing = table(target, target_master)
            .ok_or_else(|| ApiError::not_found("merge destination master does not exist"))?;
        let mut wanted = BTreeMap::new();
        for (key, value) in &incoming.0 {
            if resolvable(&key.0, known, &adopted_groups)
                && resolvable(&key.1, known, &adopted_groups)
            {
                wanted.insert(key.clone(), *value);
            } else {
                skipped_records += 1;
            }
        }
        for (key, value) in &wanted {
            if existing.0.get(key) == Some(value) {
                continue;
            }
            records += 1;
            commands.push(record(target_master, key, Some(*value), vertical));
        }
        for key in existing.0.keys() {
            if !wanted.contains_key(key) {
                records += 1;
                commands.push(record(target_master, key, None, vertical));
            }
        }
    }
    if records > MAX_KERN_COMMANDS {
        return Err(ApiError::invalid_params(format!(
            "merging kerning needs {records} record changes, above the {MAX_KERN_COMMANDS} limit"
        )));
    }
    if skipped_records > 0 {
        warnings.push(format!(
            "{skipped_records} reference kerning records were skipped because a side is absent from the destination."
        ));
    }
    Ok(())
}

fn record(
    master: &MasterId,
    key: &(KernSide, KernSide),
    value: Option<f64>,
    vertical: bool,
) -> Command {
    let (left, right) = (key.0.clone(), key.1.clone());
    if vertical {
        Command::SetVerticalKerning {
            master: master.clone(),
            left,
            right,
            value,
        }
    } else {
        Command::SetHorizontalKerning {
            master: master.clone(),
            left,
            right,
            value,
        }
    }
}

struct Built {
    commands: Vec<Command>,
    replaced: Vec<String>,
    kept: Vec<String>,
    added_bases: Vec<String>,
    adopted: Vec<String>,
    warnings: Vec<String>,
}

/// Decisions resolved against the reference document before the merge is planned.
struct Decisions {
    taken: Vec<GlyphName>,
    handles: Vec<GlyphId>,
    kept: Vec<String>,
}

fn decisions(document: &Document, params: &FontMergeParams) -> Result<Decisions, ApiError> {
    let mut result = Decisions {
        taken: Vec::new(),
        handles: Vec::new(),
        kept: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for entry in &params.glyphs {
        let name = GlyphName::from(entry.name.as_str());
        if !seen.insert(name.clone()) {
            return Err(ApiError::invalid_params(format!(
                "glyph `{name}` appears twice in the merge decisions"
            )));
        }
        match entry.choice {
            MergeChoice::Target => result.kept.push(entry.name.clone()),
            MergeChoice::Source => {
                let handle = document.glyph_id(&name).ok_or_else(|| {
                    ApiError::not_found(format!(
                        "glyph `{name}` is absent from the reference document"
                    ))
                })?;
                result.taken.push(name);
                result.handles.push(handle);
            }
        }
    }
    Ok(result)
}

fn build(
    source: &Font,
    decisions: &Decisions,
    document: &Document,
    params: &FontMergeParams,
) -> Result<Built, ApiError> {
    let source_master = master(source, params.source_master.as_ref())?;
    let target_master = master(&document.font, params.target_master.as_ref())?;
    let mut commands = Vec::new();
    let mut replaced = Vec::new();
    let mut added_bases = Vec::new();
    let mut warnings = Vec::new();
    if !decisions.taken.is_empty() {
        // Reuse the reviewed same-name transfer so components, masters and CFF
        // resources follow exactly the rules Paste Special already enforces.
        let transfer = PasteSpecialParams {
            doc: params.doc,
            source_doc: params.source_doc,
            mappings: decisions
                .handles
                .iter()
                .map(|source| PasteGlyphMapping {
                    source: *source,
                    target: None,
                })
                .collect(),
            destination: PasteDestination::SameName,
            content: PasteContent::AllData,
            source_master: Some(source_master.clone()),
            target_master: Some(target_master.clone()),
            missing_bases: params.missing_bases,
            dry_run: true,
            expected_generation: None,
            expected_source_generation: None,
        };
        let built = plan::build(source, &decisions.taken, document, &transfer)?;
        commands.extend(built.commands);
        replaced = built.pasted;
        added_bases = built.added_bases;
        warnings.extend(built.warnings);
    }
    let mut adopted = Vec::new();
    if params.adopt.metadata {
        commands.push(Command::SetFontInfo {
            info: Box::new(source.info.clone()),
        });
        adopted.push("metadata".to_owned());
        if source.info.units_per_em != document.font.info.units_per_em {
            warnings.push(
                "Adopting the reference metadata changes units per em without scaling any outline."
                    .to_owned(),
            );
        }
    }
    if params.adopt.master_metrics {
        let metrics = source
            .master(&source_master)
            .ok_or_else(|| ApiError::not_found("merge reference master does not exist"))?
            .metrics
            .clone();
        commands.push(Command::SetMasterMetrics {
            master: target_master.clone(),
            metrics,
        });
        adopted.push("master_metrics".to_owned());
    }
    if params.adopt.kerning {
        let mut known: BTreeSet<GlyphName> = document.font.glyphs.keys().cloned().collect();
        known.extend(replaced.iter().map(|name| GlyphName::from(name.as_str())));
        known.extend(
            added_bases
                .iter()
                .map(|name| GlyphName::from(name.as_str())),
        );
        kerning_commands(
            source,
            &document.font,
            &source_master,
            &target_master,
            &known,
            &mut commands,
            &mut warnings,
        )?;
        adopted.push("kerning".to_owned());
    }
    warnings.push(
        "Merging does not adopt OpenType layout source, bitmap strikes, colour palettes or global hint programs."
            .to_owned(),
    );
    Ok(Built {
        commands,
        replaced,
        kept: decisions.kept.clone(),
        added_bases,
        adopted,
        warnings,
    })
}

/// Previews or commits a reviewed merge; every change lands in one undo entry.
pub(crate) fn merge(api: &Api, params: FontMergeParams) -> Result<FontMergeResult, ApiError> {
    if params.glyphs.len() > MAX_GLYPHS {
        return Err(ApiError::invalid_params(format!(
            "a merge accepts at most {MAX_GLYPHS} glyph decisions"
        )));
    }
    if params.glyphs.is_empty()
        && !params.adopt.metadata
        && !params.adopt.master_metrics
        && !params.adopt.kerning
    {
        return Err(ApiError::invalid_params(
            "choose at least one glyph decision or font-level resource to merge",
        ));
    }
    if params.source_doc == params.doc {
        return Err(ApiError::invalid_params(
            "a merge needs two different documents",
        ));
    }
    let (source, decided, source_generation) = api
        .session
        .with_doc(params.source_doc, |doc| {
            check_generation(doc.generation, params.expected_source_generation, "source")?;
            Ok::<_, ApiError>((doc.font.clone(), decisions(doc, &params)?, doc.generation))
        })
        .map_err(ApiError::from)??;
    let (built, target, generation) = api
        .session
        .with_doc(params.doc, |doc| {
            check_generation(doc.generation, params.expected_generation, "destination")?;
            let built = build(&source, &decided, doc, &params)?;
            Ok::<_, ApiError>((built, doc.font.clone(), doc.generation))
        })
        .map_err(ApiError::from)??;
    let batch = Command::Batch(built.commands);
    // Validate against the real command and undo validators before committing.
    let scratch = Session::new();
    let id = scratch.open_font(target, None);
    scratch
        .with_doc_mut(id, |candidate| candidate.apply(batch.clone(), LABEL))
        .map_err(ApiError::from)??;
    let (change, generation) = if params.dry_run {
        (None, generation)
    } else {
        api.session
            .with_doc_mut(params.doc, |doc| {
                check_generation(doc.generation, Some(generation), "destination")?;
                let change = doc.apply(batch, LABEL)?;
                Ok::<_, ApiError>((Some(change), doc.generation))
            })
            .map_err(ApiError::from)??
    };
    Ok(FontMergeResult {
        change,
        generation,
        source_generation,
        replaced: built.replaced,
        kept: built.kept,
        added_bases: built.added_bases,
        adopted: built.adopted,
        label: LABEL.to_owned(),
        warnings: built.warnings,
    })
}
