//! Read-only, bounded views of stored horizontal kerning.
use crate::{
    Api, ApiError, DocParams, GlyphKerning, GlyphKerningParams, KerningGroupSummary,
    KerningGroupsParams, KerningGroupsResult, KerningMembers, KerningMembersParams,
    KerningPairRecord, KerningPairs, KerningPairsParams, KerningResolution, KerningResolveParams,
    KerningStatus,
};
use fc_doc::{
    kerning::{
        check_horizontal_editing, group_membership, resolve_stored_horizontal, validate_group,
        validate_horizontal_record, KerningGroupSide as Side,
    },
    Document,
};
use fc_model::{GlyphName, KernPair, KernSide};

fn current_status(document: &Document) -> KerningStatus {
    let reason = check_horizontal_editing(&document.font).err();
    KerningStatus {
        generation: document.generation,
        editable: reason.is_none(),
        reason,
    }
}
pub(crate) fn status(api: &Api, params: DocParams) -> Result<KerningStatus, ApiError> {
    api.session
        .with_doc(params.doc, current_status)
        .map_err(ApiError::from)
}
pub(crate) fn glyph(api: &Api, params: GlyphKerningParams) -> Result<GlyphKerning, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let name = GlyphName::from(params.glyph);
            if document.font.glyph(&name).is_none() {
                return Err(ApiError::not_found(format!(
                    "glyph `{name}` does not exist"
                )));
            }
            let mut diagnostics = Vec::new();
            let mut membership = |side| match group_membership(&document.font, &name, side) {
                Ok(group) => {
                    if let Some(name) = group {
                        if let Err(error) = validate_group(&document.font, side, name) {
                            diagnostics.push(error.to_string());
                        }
                    }
                    group.map(str::to_owned)
                }
                Err(error) => {
                    diagnostics.push(error.to_string());
                    None
                }
            };
            let side1 = membership(Side::Side1);
            let side2 = membership(Side::Side2);
            Ok(GlyphKerning {
                status: current_status(document),
                side1,
                side2,
                diagnostics,
            })
        })
        .map_err(ApiError::from)?
}
pub(crate) fn resolve(
    api: &Api,
    params: KerningResolveParams,
) -> Result<KerningResolution, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let font = &document.font;
            let master = font.master(&params.master).ok_or_else(|| {
                ApiError::not_found(format!("master `{}` does not exist", params.master))
            })?;
            let left = GlyphName::from(params.left.clone());
            let right = GlyphName::from(params.right.clone());
            for name in [&left, &right] {
                if font.glyph(name).is_none() {
                    return Err(ApiError::not_found(format!(
                        "glyph `{name}` does not exist"
                    )));
                }
            }
            let status = current_status(document);
            let mut result = KerningResolution {
                status,
                master: params.master,
                left: params.left,
                right: params.right,
                left_group: None,
                right_group: None,
                value: None,
                rule: None,
                records: Vec::new(),
            };
            if !result.status.editable {
                return Ok(result);
            }
            result.left_group = group_membership(font, &left, Side::Side1)?.map(str::to_owned);
            result.right_group = group_membership(font, &right, Side::Side2)?.map(str::to_owned);
            result.rule = resolve_stored_horizontal(font, &left, &right, &result.master)?;
            result.value = Some(result.rule.as_ref().map_or(0.0, |pair| pair.value));
            let l = KernSide::Glyph(left);
            let r = KernSide::Glyph(right);
            let candidates = [
                Some((l.clone(), r.clone())),
                result
                    .right_group
                    .as_ref()
                    .map(|name| (l, KernSide::Group(name.clone()))),
                result
                    .left_group
                    .as_ref()
                    .map(|name| (KernSide::Group(name.clone()), r)),
                result
                    .left_group
                    .as_ref()
                    .zip(result.right_group.as_ref())
                    .map(|(l, r)| (KernSide::Group(l.clone()), KernSide::Group(r.clone()))),
            ];
            result.records = candidates
                .into_iter()
                .flatten()
                .filter_map(|(left, right)| {
                    let value = *master.kerning_h.0.get(&(left.clone(), right.clone()))?;
                    let record = KernPair { left, right, value };
                    validate_horizontal_record(font, &record)
                        .is_ok()
                        .then_some(record)
                })
                .collect();
            Ok(result)
        })
        .map_err(ApiError::from)?
}
fn page(
    document: &Document,
    generation: Option<u64>,
    offset: u32,
    limit: u32,
) -> Result<(), ApiError> {
    if limit == 0 || limit > 200 {
        return Err(ApiError::invalid_params(
            "kerning page limit must be between 1 and 200",
        ));
    }
    if generation.is_some_and(|generation| generation != document.generation) {
        return Err(ApiError {
            code: "stale_generation".into(),
            message: "document changed; restart kerning pagination".into(),
        });
    }
    if offset > 0 && generation.is_none() {
        return Err(ApiError::invalid_params(
            "continuation pages require a document generation",
        ));
    }
    Ok(())
}
pub(crate) fn pairs(api: &Api, params: KerningPairsParams) -> Result<KerningPairs, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            page(document, params.generation, params.offset, params.limit)?;
            let master = document.font.master(&params.master).ok_or_else(|| {
                ApiError::not_found(format!("master `{}` does not exist", params.master))
            })?;
            let items = master
                .kerning_h
                .0
                .iter()
                .skip(params.offset as usize)
                .take(params.limit as usize)
                .map(|((left, right), value)| {
                    let diagnostic = validate_horizontal_record(
                        &document.font,
                        &KernPair {
                            left: left.clone(),
                            right: right.clone(),
                            value: *value,
                        },
                    )
                    .err()
                    .map(|error| error.to_string());
                    KerningPairRecord {
                        left: left.clone(),
                        right: right.clone(),
                        value: value.is_finite().then_some(*value),
                        diagnostic,
                    }
                })
                .collect();
            Ok(KerningPairs {
                status: current_status(document),
                total: count(master.kerning_h.0.len()),
                items,
            })
        })
        .map_err(ApiError::from)?
}
pub(crate) fn groups(
    api: &Api,
    params: KerningGroupsParams,
) -> Result<KerningGroupsResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            page(document, params.generation, params.offset, params.limit)?;
            let matches = || {
                params
                    .side
                    .groups(&document.font)
                    .iter()
                    .filter(|(name, _)| {
                        params
                            .query
                            .as_ref()
                            .is_none_or(|query| name.contains(query))
                    })
            };
            let total = count(matches().count());
            let items = matches()
                .skip(params.offset as usize)
                .take(params.limit as usize)
                .map(|(name, members)| {
                    let pair_count = document
                        .font
                        .masters
                        .iter()
                        .flat_map(|master| master.kerning_h.0.keys())
                        .filter(|(left, right)| {
                            let value = match params.side {
                                Side::Side1 => left,
                                Side::Side2 => right,
                            };
                            matches!(value, KernSide::Group(group) if group == name)
                        })
                        .count();
                    KerningGroupSummary {
                        name: name.clone(),
                        member_count: count(members.len()),
                        pair_count: count(pair_count),
                        sample_members: members.iter().take(8).map(ToString::to_string).collect(),
                        diagnostic: validate_group(&document.font, params.side, name)
                            .err()
                            .map(|error| error.to_string()),
                    }
                })
                .collect();
            Ok(KerningGroupsResult {
                status: current_status(document),
                total,
                items,
            })
        })
        .map_err(ApiError::from)?
}
pub(crate) fn members(api: &Api, params: KerningMembersParams) -> Result<KerningMembers, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            page(document, params.generation, params.offset, params.limit)?;
            let members = params
                .side
                .groups(&document.font)
                .get(&params.name)
                .ok_or_else(|| {
                    ApiError::not_found(format!("kerning group `{}` does not exist", params.name))
                })?;
            Ok(KerningMembers {
                status: current_status(document),
                total: count(members.len()),
                items: members
                    .iter()
                    .skip(params.offset as usize)
                    .take(params.limit as usize)
                    .map(ToString::to_string)
                    .collect(),
            })
        })
        .map_err(ApiError::from)?
}
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
