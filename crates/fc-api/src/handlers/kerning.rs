//! Read-only, bounded views of stored horizontal kerning.
use crate::{
    Api, ApiError, DocParams, GlyphKerning, GlyphKerningParams, KerningBoundaries,
    KerningBoundariesParams, KerningBoundary, KerningGroupSummary, KerningGroupsParams,
    KerningGroupsResult, KerningMembers, KerningMembersParams, KerningPairRecord, KerningPairs,
    KerningPairsParams, KerningResolution, KerningResolveParams, KerningStatus,
};
use fc_doc::{
    kerning::{
        check_horizontal_editing, check_vertical_editing, group_membership,
        resolve_stored_horizontal, resolve_stored_vertical, validate_group,
        validate_horizontal_record, KerningGroupSide as Side,
    },
    Document,
};
use fc_model::{GlyphName, KernPair, KernSide};

fn current_status(document: &Document) -> KerningStatus {
    direction_status(document, false)
}
fn direction_status(document: &Document, vertical: bool) -> KerningStatus {
    let reason = if vertical {
        check_vertical_editing(&document.font)
    } else {
        check_horizontal_editing(&document.font)
    }
    .err();
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
pub(crate) fn status_vertical(api: &Api, params: DocParams) -> Result<KerningStatus, ApiError> {
    api.session
        .with_doc(params.doc, |document| direction_status(document, true))
        .map_err(ApiError::from)
}
pub(crate) fn resolve_vertical(
    api: &Api,
    params: KerningResolveParams,
) -> Result<KerningResolution, ApiError> {
    resolve_direction(api, params, true)
}
pub(crate) fn pairs_vertical(
    api: &Api,
    params: KerningPairsParams,
) -> Result<KerningPairs, ApiError> {
    pairs_direction(api, params, true)
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
    resolve_direction(api, params, false)
}
fn resolve_direction(
    api: &Api,
    params: KerningResolveParams,
    vertical: bool,
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
            let status = direction_status(document, vertical);
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
                exception: false,
            };
            if !result.status.editable {
                return Ok(result);
            }
            result.left_group = group_membership(font, &left, Side::Side1)?.map(str::to_owned);
            result.right_group = group_membership(font, &right, Side::Side2)?.map(str::to_owned);
            result.rule = if vertical {
                resolve_stored_vertical(font, &left, &right, &result.master)
            } else {
                resolve_stored_horizontal(font, &left, &right, &result.master)
            }?;
            let kerning = if vertical {
                &master.kerning_v
            } else {
                &master.kerning_h
            };
            result.value = Some(result.rule.as_ref().map_or(0.0, |pair| pair.value));
            result.exception = result.rule.as_ref().is_some_and(|rule| {
                is_exception(
                    &rule.left,
                    &rule.right,
                    result.left_group.as_deref(),
                    result.right_group.as_deref(),
                )
            });
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
                    let value = *kerning.0.get(&(left.clone(), right.clone()))?;
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
/// Whether a rule departs from the class rule that would otherwise apply.
///
/// A side is an exception when the rule names a concrete glyph there and that
/// glyph is a member of a kerning group on the same side, so a broader class
/// rule could have covered it. This is the same notion the kerning table marks
/// and the notion a held-modifier edit creates.
fn is_exception(
    left: &KernSide,
    right: &KernSide,
    left_group: Option<&str>,
    right_group: Option<&str>,
) -> bool {
    (matches!(left, KernSide::Glyph(_)) && left_group.is_some())
        || (matches!(right, KernSide::Glyph(_)) && right_group.is_some())
}

/// Exception flag for one stored record, resolving membership per side.
fn record_exception(font: &fc_model::Font, left: &KernSide, right: &KernSide) -> bool {
    let membership = |value: &KernSide, side: Side| match value {
        KernSide::Glyph(name) => group_membership(font, name, side)
            .ok()
            .flatten()
            .map(str::to_owned),
        KernSide::Group(_) => None,
    };
    is_exception(
        left,
        right,
        membership(left, Side::Side1).as_deref(),
        membership(right, Side::Side2).as_deref(),
    )
}

/// Exception flags for many boundaries of one run, in one bounded request.
///
/// The canvas marks every exception, not only the selected pair, so it needs an
/// answer per boundary. Asking `kerning.resolve` once per boundary would be one
/// request per letter on every keystroke; this answers the whole visible run
/// at one generation. A boundary whose glyphs or groups cannot be read is
/// reported as a null value rather than failing the run.
pub(crate) fn boundaries(
    api: &Api,
    params: KerningBoundariesParams,
) -> Result<KerningBoundaries, ApiError> {
    if params.pairs.len() > crate::KERNING_BOUNDARY_LIMIT {
        return Err(ApiError::invalid_params(format!(
            "a kerning boundary request carries at most {} pairs",
            crate::KERNING_BOUNDARY_LIMIT
        )));
    }
    api.session
        .with_doc(params.doc, |document| {
            let font = &document.font;
            if font.master(&params.master).is_none() {
                return Err(ApiError::not_found(format!(
                    "master `{}` does not exist",
                    params.master
                )));
            }
            let status = direction_status(document, params.vertical);
            let items = params
                .pairs
                .iter()
                .map(|[left, right]| {
                    if !status.editable {
                        return KerningBoundary {
                            value: None,
                            exception: false,
                        };
                    }
                    let left = GlyphName::from(left.clone());
                    let right = GlyphName::from(right.clone());
                    let rule = if params.vertical {
                        resolve_stored_vertical(font, &left, &right, &params.master)
                    } else {
                        resolve_stored_horizontal(font, &left, &right, &params.master)
                    };
                    let Ok(rule) = rule else {
                        return KerningBoundary {
                            value: None,
                            exception: false,
                        };
                    };
                    let membership = |name: &GlyphName, side: Side| {
                        group_membership(font, name, side)
                            .ok()
                            .flatten()
                            .map(str::to_owned)
                    };
                    KerningBoundary {
                        value: Some(rule.as_ref().map_or(0.0, |pair| pair.value)),
                        exception: rule.as_ref().is_some_and(|pair| {
                            is_exception(
                                &pair.left,
                                &pair.right,
                                membership(&left, Side::Side1).as_deref(),
                                membership(&right, Side::Side2).as_deref(),
                            )
                        }),
                    }
                })
                .collect();
            Ok(KerningBoundaries { status, items })
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
    pairs_direction(api, params, false)
}
fn pairs_direction(
    api: &Api,
    params: KerningPairsParams,
    vertical: bool,
) -> Result<KerningPairs, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            page(document, params.generation, params.offset, params.limit)?;
            let master = document.font.master(&params.master).ok_or_else(|| {
                ApiError::not_found(format!("master `{}` does not exist", params.master))
            })?;
            let kerning = if vertical {
                &master.kerning_v
            } else {
                &master.kerning_h
            };
            let items = kerning
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
                        exception: record_exception(&document.font, left, right),
                        left: left.clone(),
                        right: right.clone(),
                        value: value.is_finite().then_some(*value),
                        diagnostic,
                    }
                })
                .collect();
            Ok(KerningPairs {
                status: direction_status(document, vertical),
                total: count(kerning.0.len()),
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
                        .flat_map(|master| {
                            master.kerning_h.0.keys().chain(master.kerning_v.0.keys())
                        })
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
