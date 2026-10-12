//! Optional model ownership transfers, accepted only after actual compilation.

#[path = "lifting_parse.rs"]
mod parse;

use fc_model::{
    FeatureAnchorState, FeatureBlock, FeatureGenerationSettings, FeatureGlyphMembership,
    FeatureKerningState, FeatureMasterKerning, Features, Font,
};

use crate::{equivalent_layout, LayoutImport};

/// A source candidate and the independently proven model transfers it supports.
///
/// The original font is never mutated. Rejected transfers leave their complete
/// lookups in source, with their model writer disabled.
#[derive(Clone, Debug)]
pub struct LiftResult {
    /// Source and model settings suitable for the atomic conversion transaction.
    pub candidate: Font,
    /// Pre-conversion horizontal data, only when replaced by a proven lift.
    pub original_kerning: Option<FeatureKerningState>,
    /// Replacement horizontal data for the prepared document transaction.
    pub new_kerning: Option<FeatureKerningState>,
    /// Pre-conversion anchor lists of affected layers, without outlines.
    pub original_anchors: Vec<FeatureAnchorState>,
    /// Replacement anchor lists for the prepared document transaction.
    pub new_anchors: Vec<FeatureAnchorState>,
    /// Whether all source-owned horizontal kern rules transferred to the model.
    pub kerning_lifted: bool,
    /// Whether the selected complete mark/mkmk lookups transferred to anchors.
    pub anchors_lifted: bool,
    /// Accepted transfers and reasons an optional transfer stayed source-owned.
    pub report: Vec<String>,
}

/// Try bounded whole-lookup lifts after baseline source conversion has succeeded.
///
/// Only the exact decompiled baseline is eligible: edited source must first be
/// applied as source and cannot borrow an imported table's lifting proof. Every
/// accepted trial is compiled through fontc for every master and compared with
/// the original layout's complete canonical table/feature graph.
pub fn lift_candidate(original: &Font, source: &Features) -> LiftResult {
    let mut candidate = original.clone();
    candidate.features = source.clone();
    candidate.feature_draft = None;
    candidate.feature_settings = FeatureGenerationSettings::default();
    candidate.feature_settings.writer_policy.kerning = false;
    candidate.feature_settings.writer_policy.anchors = false;
    let mut result = LiftResult {
        candidate,
        original_kerning: None,
        new_kerning: None,
        original_anchors: Vec::new(),
        new_anchors: Vec::new(),
        kerning_lifted: false,
        anchors_lifted: false,
        report: Vec::new(),
    };
    let import = LayoutImport::from_font(original);
    match import.decompile() {
        Ok(code) if code.trim() == source.to_fea().trim() => (),
        Ok(_) => {
            result
                .report
                .push("Model lifting requires the unchanged decompiled baseline".into());
            return result;
        }
        Err(error) => {
            result.report.push(error.to_string());
            return result;
        }
    }
    let Some(graph) = import.gpos_graph() else {
        return result;
    };
    let has_kerning = graph
        .registrations
        .iter()
        .any(|registration| registration.feature == "kern");
    let has_anchors = graph
        .registrations
        .iter()
        .any(|registration| matches!(registration.feature.as_str(), "mark" | "mkmk"));
    // fontc orders source and generated lookups separately. A partial transfer
    // can therefore reorder lookups even when transferring both writers
    // together exactly reproduces the original generated order.
    if has_kerning && has_anchors {
        match try_joint(original, &result.candidate, &import, source) {
            Ok((candidate, before, after)) => {
                result.original_kerning = Some(capture_kerning(original));
                result.new_kerning = Some(capture_kerning(&candidate));
                result.original_anchors = before;
                result.new_anchors = after;
                result.candidate = candidate;
                result.kerning_lifted = true;
                result.anchors_lifted = true;
                result.report.push("Complete kern and mark/mkmk lookups transferred together after compiler equivalence".into());
                return result;
            }
            Err(error) => result
                .report
                .push(format!("Combined model transfer was not proven: {error}")),
        }
    }
    let mut excluded = Vec::new();
    if has_kerning {
        let trial = stage_kerning(original, &result.candidate, &import, source, &excluded)
            .and_then(|(candidate, indices)| {
                prove(&import, &candidate)?;
                Ok((candidate, indices))
            });
        match trial {
            Ok((candidate, indices)) => {
                result.original_kerning = Some(capture_kerning(original));
                result.new_kerning = Some(capture_kerning(&candidate));
                result.candidate = candidate;
                result.kerning_lifted = true;
                excluded.extend(indices);
                result.report.push(
                    "Complete kern lookup transferred to model pairs after compiler equivalence"
                        .into(),
                );
            }
            Err(error) => result
                .report
                .push(format!("Kerning remains source-owned: {error}")),
        }
    }
    if has_anchors {
        let trial = stage_anchors(original, &result.candidate, &import, source, &excluded)
            .and_then(|(candidate, before, after)| {
                prove(&import, &candidate)?;
                Ok((candidate, before, after))
            });
        match trial {
            Ok((candidate, before, after)) => {
                result.candidate = candidate;
                result.original_anchors = before;
                result.new_anchors = after;
                result.anchors_lifted = true;
                result.report.push("Complete mark/mkmk lookups transferred to model anchors after compiler equivalence".into());
            }
            Err(error) => result
                .report
                .push(format!("Anchors remain source-owned: {error}")),
        }
    }
    result
}

fn try_joint(
    original: &Font,
    current: &Font,
    import: &LayoutImport,
    source: &Features,
) -> Result<AnchorTrial, String> {
    let (kerning_candidate, indices) = stage_kerning(original, current, import, source, &[])?;
    let (candidate, before, after) =
        stage_anchors(original, &kerning_candidate, import, source, &indices)?;
    prove(import, &candidate)?;
    Ok((candidate, before, after))
}

fn stage_kerning(
    original: &Font,
    current: &Font,
    import: &LayoutImport,
    source: &Features,
    excluded: &[u16],
) -> Result<(Font, Vec<u16>), String> {
    let graph = import.gpos_graph().ok_or("missing GPOS graph")?;
    let (indices, pairs, groups) = parse::kerning(original, graph, import.glyph_order())?;
    let mut candidate = current.clone();
    candidate.kerning_groups = groups;
    for master in &mut candidate.masters {
        master.kerning_h = pairs.clone();
    }
    for glyph in candidate.glyphs.values_mut() {
        glyph.kern_group_side1 = candidate
            .kerning_groups
            .side1
            .iter()
            .find(|(_, members)| members.contains(&glyph.name))
            .map(|(name, _)| name.clone());
        glyph.kern_group_side2 = candidate
            .kerning_groups
            .side2
            .iter()
            .find(|(_, members)| members.contains(&glyph.name))
            .map(|(name, _)| name.clone());
    }
    candidate.feature_settings.writer_policy.kerning = true;
    candidate.features = remaining_source(import, source, &[excluded, &indices].concat())?;
    Ok((candidate, indices))
}

type AnchorTrial = (Font, Vec<FeatureAnchorState>, Vec<FeatureAnchorState>);

fn stage_anchors(
    original: &Font,
    current: &Font,
    import: &LayoutImport,
    source: &Features,
    excluded: &[u16],
) -> Result<AnchorTrial, String> {
    let graph = import.gpos_graph().ok_or("missing GPOS graph")?;
    let (indices, anchors) = parse::anchors(original, graph, import.glyph_order())?;
    let mut candidate = current.clone();
    let mut before = Vec::new();
    let mut after = Vec::new();
    let masters: Vec<_> = candidate
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    for (name, additions) in anchors {
        let glyph = candidate
            .glyph_mut(name.as_ref())
            .ok_or("anchor glyph no longer exists")?;
        for master in &masters {
            let layer = glyph
                .master_layer_mut(master)
                .ok_or("anchor glyph has no layer for a master")?;
            let old = FeatureAnchorState {
                glyph: name.clone(),
                layer: layer.id.clone(),
                anchors: layer.anchors.clone(),
            };
            for addition in &additions {
                if let Some(existing) = layer
                    .anchors
                    .iter()
                    .find(|anchor| anchor.name == addition.name)
                {
                    if existing != addition {
                        return Err(format!(
                            "conflicting existing anchor {} on {name}",
                            addition.name
                        ));
                    }
                } else {
                    layer.anchors.push(addition.clone());
                }
            }
            if old.anchors != layer.anchors {
                before.push(old);
                after.push(FeatureAnchorState {
                    glyph: name.clone(),
                    layer: layer.id.clone(),
                    anchors: layer.anchors.clone(),
                });
            }
        }
    }
    candidate.feature_settings.writer_policy.anchors = true;
    candidate.features = remaining_source(import, source, &[excluded, &indices].concat())?;
    Ok((candidate, before, after))
}

fn remaining_source(
    import: &LayoutImport,
    original: &Features,
    excluded: &[u16],
) -> Result<Features, String> {
    let code = import
        .decompile_excluding_gpos(excluded)
        .map_err(|error| error.to_string())?;
    let mut block = original
        .prefixes
        .first()
        .cloned()
        .unwrap_or_else(|| FeatureBlock {
            tag: "Imported layout".into(),
            ..FeatureBlock::default()
        });
    block.code = code;
    block.disabled = false;
    Ok(Features {
        prefixes: vec![block],
        ..Features::default()
    })
}

fn prove(import: &LayoutImport, candidate: &Font) -> Result<(), String> {
    if candidate.masters.is_empty() {
        return Err("font has no masters".into());
    }
    for master in &candidate.masters {
        let compiled =
            fc_compile::compile_static_with_metadata(candidate, &master.id, &Default::default())
                .map_err(|error| error.to_string())?;
        equivalent_layout(import, &compiled.bytes, &compiled.glyph_order)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn capture_kerning(font: &Font) -> FeatureKerningState {
    FeatureKerningState {
        masters: font
            .masters
            .iter()
            .map(|master| FeatureMasterKerning {
                master: master.id.clone(),
                kerning: master.kerning_h.clone(),
            })
            .collect(),
        groups: font.kerning_groups.clone(),
        memberships: font
            .glyphs
            .values()
            .map(|glyph| FeatureGlyphMembership {
                glyph: glyph.name.clone(),
                side1: glyph.kern_group_side1.clone(),
                side2: glyph.kern_group_side2.clone(),
            })
            .collect(),
    }
}
