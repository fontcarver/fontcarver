//! Bounded smart-filter compilation and read-only model predicates.
//!
//! Geometry predicates inspect the selected layer; special-layer and compatibility
//! predicates inspect the whole glyph. Missing colors compare as -1. Supplemental
//! annotations use `layer.user_data["com.fontcarver.editor"].annotations`; plain
//! `annotations` arrays are accepted as a legacy fallback. Case and layer color
//! use `user_data.case` and `user_data.color_label`. Hangul keys come from
//! `glyph.user_data.is_hangul_key` or names in `font.extra.lib` under
//! `hangul_composition_groups`; Hangul Unicode membership alone is insufficient.

use fc_model::{Font, Glyph, Layer, LayerKind};
use regex::{Regex, RegexBuilder};
use serde_json::Value;

use crate::{ApiError, SmartFilterNode, SmartFilterOperator as Op};

const MAX_DEPTH: usize = 16;
const MAX_NODES: usize = 256;
const MAX_LITERAL: usize = 4096;

/// A validated expression whose regular expressions are compiled once per query.
pub(super) struct CompiledSmartFilter(CompiledNode);

enum CompiledNode {
    All(Vec<CompiledNode>),
    Any(Vec<CompiledNode>),
    None(Vec<CompiledNode>),
    Predicate(CompiledPredicate),
}

struct CompiledPredicate {
    field: &'static str,
    operator: Op,
    value: Operand,
}

enum Operand {
    Text(String),
    Pattern(Regex),
    Number(f64),
    Boolean(bool),
}

impl CompiledSmartFilter {
    /// Validates node/literal/regex limits and compiles an expression.
    pub(super) fn new(node: &SmartFilterNode) -> Result<Self, ApiError> {
        compile_node(node, 0, &mut 0).map(Self)
    }

    /// Evaluates the expression against a glyph and its selected master layer.
    pub(super) fn matches(
        &self,
        font: &Font,
        glyph: &Glyph,
        layer: Option<&Layer>,
        generation: u64,
    ) -> bool {
        self.0.matches(font, glyph, layer, generation)
    }
}

fn compile_node(
    node: &SmartFilterNode,
    depth: usize,
    count: &mut usize,
) -> Result<CompiledNode, ApiError> {
    *count += 1;
    if depth >= MAX_DEPTH || *count > MAX_NODES {
        return Err(ApiError::invalid_params(
            "smart filter exceeds 16 levels or 256 nodes",
        ));
    }
    let (children, constructor): (&Vec<_>, fn(Vec<CompiledNode>) -> CompiledNode) = match node {
        SmartFilterNode::All { children } => (children, CompiledNode::All),
        SmartFilterNode::Any { children } => (children, CompiledNode::Any),
        SmartFilterNode::None { children } => (children, CompiledNode::None),
        SmartFilterNode::Predicate {
            field,
            operator,
            value,
        } => {
            return compile_predicate(field, *operator, value).map(CompiledNode::Predicate);
        }
    };
    children
        .iter()
        .map(|child| compile_node(child, depth + 1, count))
        .collect::<Result<Vec<_>, _>>()
        .map(constructor)
}

/// Checks a single field, operator, literal, and bounded regular expression.
pub(super) fn validate_predicate(field: &str, operator: Op, value: &str) -> Result<(), ApiError> {
    compile_predicate(field, operator, value).map(|_| ())
}

fn compile_predicate(
    field: &str,
    operator: Op,
    value: &str,
) -> Result<CompiledPredicate, ApiError> {
    if value.len() > MAX_LITERAL {
        return Err(ApiError::invalid_params(
            "smart-filter literal exceeds 4096 bytes",
        ));
    }
    let (field, kind) = field_kind(field)
        .ok_or_else(|| ApiError::invalid_params(format!("unknown smart-filter field `{field}`")))?;
    let invalid = || {
        ApiError::invalid_params(format!(
            "invalid operator or value for smart-filter field `{field}`"
        ))
    };
    let value = match kind {
        'b' if matches!(operator, Op::Eq | Op::Ne) => Operand::Boolean(match value {
            "true" => true,
            "false" => false,
            _ => return Err(invalid()),
        }),
        'n' if matches!(
            operator,
            Op::Eq | Op::Ne | Op::Gt | Op::Gte | Op::Lt | Op::Lte
        ) =>
        {
            let number = if field == "unicode" {
                let scalar = value
                    .strip_prefix("U+")
                    .or_else(|| value.strip_prefix("u+"))
                    .map_or_else(
                        || value.parse::<u32>().ok(),
                        |digits| u32::from_str_radix(digits, 16).ok(),
                    )
                    .filter(|scalar| char::from_u32(*scalar).is_some())
                    .ok_or_else(invalid)?;
                f64::from(scalar)
            } else {
                value.parse::<f64>().map_err(|_| invalid())?
            };
            if !number.is_finite()
                || (matches!(field, "paths" | "components")
                    && (number < 0. || number.fract() != 0.))
                || (matches!(field, "color_label" | "layer_color_label")
                    && (!(-1. ..=255.).contains(&number) || number.fract() != 0.))
            {
                return Err(invalid());
            }
            Operand::Number(number)
        }
        's' if matches!(operator, Op::Eq | Op::Ne | Op::Contains) => {
            Operand::Text(value.to_owned())
        }
        's' if operator == Op::Regex => {
            if value.len() > 1024 {
                return Err(ApiError::invalid_params(
                    "smart-filter regex exceeds 1024 bytes",
                ));
            }
            Operand::Pattern(
                RegexBuilder::new(value)
                    .size_limit(1 << 20)
                    .dfa_size_limit(1 << 20)
                    .nest_limit(32)
                    .build()
                    .map_err(|error| {
                        ApiError::invalid_params(format!("invalid smart-filter regex: {error}"))
                    })?,
            )
        }
        _ => return Err(invalid()),
    };
    Ok(CompiledPredicate {
        field,
        operator,
        value,
    })
}

/// Resolves supported aliases to the visual builder's canonical field names.
pub(super) fn canonical_field(field: &str) -> Option<&'static str> {
    field_kind(field).map(|(name, _)| name)
}

fn field_kind(field: &str) -> Option<(&'static str, char)> {
    Some(match field {
        "name" | "glyph_name" => ("name", 's'),
        "paths" | "path_count" | "count_of_paths" => ("paths", 'n'),
        "components" | "component_count" | "count_of_components" => ("components", 'n'),
        "tags" => ("tags", 's'),
        "script" => ("script", 's'),
        "category" => ("category", 's'),
        "subcategory" | "sub_category" => ("subcategory", 's'),
        "case" => ("case", 's'),
        "contains_component" => ("contains_component", 's'),
        "note" => ("note", 's'),
        "block" => ("block", 's'),
        "unicode" => ("unicode", 'n'),
        "width" => ("width", 'n'),
        "color_label" => ("color_label", 'n'),
        "layer_color_label" => ("layer_color_label", 'n'),
        "master_compatible" => ("master_compatible", 'b'),
        "export" | "export_glyph" => ("export", 'b'),
        "has_unicode" => ("has_unicode", 'b'),
        "has_components" => ("has_components", 'b'),
        "has_hints" => ("has_hints", 'b'),
        "has_ps_hints" => ("has_ps_hints", 'b'),
        "has_tt_hints" => ("has_tt_hints", 'b'),
        "has_corners" => ("has_corners", 'b'),
        "has_anchors" => ("has_anchors", 'b'),
        "has_special_layers" => ("has_special_layers", 'b'),
        "has_custom_glyph_info" => ("has_custom_glyph_info", 'b'),
        "has_annotations" => ("has_annotations", 'b'),
        "is_auto_aligned" => ("is_auto_aligned", 'b'),
        "has_metrics_keys" => ("has_metrics_keys", 'b'),
        "metrics_out_of_sync" | "metrics_keys_out_of_sync" => ("metrics_out_of_sync", 'b'),
        "has_kerning_groups" => ("has_kerning_groups", 'b'),
        "is_hangul_key" => ("is_hangul_key", 'b'),
        "changed" => ("changed", 'b'),
        _ => return None,
    })
}

impl CompiledNode {
    fn matches(&self, font: &Font, glyph: &Glyph, layer: Option<&Layer>, generation: u64) -> bool {
        match self {
            Self::All(children) => children
                .iter()
                .all(|node| node.matches(font, glyph, layer, generation)),
            Self::Any(children) => children
                .iter()
                .any(|node| node.matches(font, glyph, layer, generation)),
            Self::None(children) => !children
                .iter()
                .any(|node| node.matches(font, glyph, layer, generation)),
            Self::Predicate(predicate) => predicate.matches(font, glyph, layer, generation),
        }
    }
}

impl CompiledPredicate {
    fn matches(&self, font: &Font, glyph: &Glyph, layer: Option<&Layer>, generation: u64) -> bool {
        let cp = glyph.unicodes.first().copied();
        match self.field {
            "name" => self.text(glyph.name.as_ref()),
            "note" => self.text(&glyph.note),
            "tags" => self.texts(glyph.tags.iter().map(String::as_str)),
            "contains_component" => self.texts(layer.into_iter().flat_map(|layer| {
                layer
                    .components
                    .iter()
                    .map(|component| component.base.as_ref())
            })),
            "script" => self.text(
                glyph
                    .script
                    .as_deref()
                    .unwrap_or_else(|| cp.map(fc_glyphdata::script).unwrap_or("Zzzz")),
            ),
            "category" => self.text(
                glyph
                    .category
                    .as_deref()
                    .unwrap_or_else(|| cp.map(category_name).unwrap_or("Unencoded")),
            ),
            "subcategory" => self.text(
                glyph
                    .sub_category
                    .as_deref()
                    .unwrap_or_else(|| cp.map(fc_glyphdata::sub_category).unwrap_or("Unencoded")),
            ),
            "case" => self.text(glyph_case(glyph)),
            "block" => self.texts(
                glyph
                    .unicodes
                    .iter()
                    .map(|cp| fc_glyphdata::block_name(*cp)),
            ),
            "unicode" => {
                let positive = glyph.unicodes.iter().any(|cp| {
                    self.number_with(
                        f64::from(*cp),
                        if self.operator == Op::Ne {
                            Op::Eq
                        } else {
                            self.operator
                        },
                    )
                });
                if self.operator == Op::Ne {
                    !positive
                } else {
                    positive
                }
            }
            "paths" => self.number(layer.map_or(0., |layer| layer.contours.len() as f64)),
            "components" => self.number(layer.map_or(0., |layer| layer.components.len() as f64)),
            "width" => self.number(layer.map_or(0., |layer| layer.width)),
            "color_label" => self.number(glyph.color_label.map_or(-1., f64::from)),
            "layer_color_label" => self.number(
                layer
                    .and_then(|layer| {
                        layer
                            .user_data
                            .get("color_label")
                            .or_else(|| layer.user_data.get("color"))
                    })
                    .and_then(Value::as_u64)
                    .filter(|n| *n <= 255)
                    .map_or(-1., |n| n as f64),
            ),
            field => {
                self.boolean(match field {
                    "export" => glyph.export,
                    "changed" => generation > 0,
                    "has_unicode" => !glyph.unicodes.is_empty(),
                    "master_compatible" => {
                        fc_model::masters_compatible(font, glyph.name.as_ref()).is_ok()
                    }
                    "has_components" => layer.is_some_and(|layer| !layer.components.is_empty()),
                    "has_hints" => layer.is_some_and(|layer| {
                        !layer.hints.is_empty()
                            || layer
                                .tt_instructions
                                .as_ref()
                                .is_some_and(|hints| !hints.is_empty())
                    }),
                    "has_ps_hints" => layer.is_some_and(|layer| !layer.hints.is_empty()),
                    "has_tt_hints" => layer.is_some_and(|layer| {
                        layer
                            .tt_instructions
                            .as_ref()
                            .is_some_and(|hints| !hints.is_empty())
                    }),
                    "has_corners" => layer.is_some_and(Layer::has_smart_corners),
                    "has_anchors" => layer.is_some_and(|layer| !layer.anchors.is_empty()),
                    "has_special_layers" => glyph.layers.iter().any(|layer| {
                        matches!(
                            layer.kind,
                            LayerKind::Brace { .. }
                                | LayerKind::Bracket { .. }
                                | LayerKind::Smart { .. }
                        )
                    }),
                    "has_custom_glyph_info" => {
                        glyph.category.is_some()
                            || glyph.sub_category.is_some()
                            || glyph.script.is_some()
                            || glyph.production_name.is_some()
                            || glyph.user_data.get("case").is_some()
                    }
                    "has_annotations" => layer.is_some_and(|layer| {
                        layer
                            .user_data
                            .get("com.fontcarver.editor")
                            .and_then(|data| data.get("annotations"))
                            .or_else(|| layer.user_data.get("annotations"))
                            .and_then(Value::as_array)
                            .is_some_and(|items| !items.is_empty())
                    }),
                    "is_auto_aligned" => layer.is_some_and(|layer| {
                        !layer.components.is_empty()
                            && layer
                                .components
                                .iter()
                                .all(|component| component.auto_align)
                    }),
                    "has_metrics_keys" => metric_keys(glyph).iter().any(Option::is_some),
                    "metrics_out_of_sync" => metrics_out_of_sync(font, glyph, layer),
                    "has_kerning_groups" => {
                        glyph
                            .kern_group_side1
                            .as_ref()
                            .is_some_and(|group| !group.is_empty())
                            || glyph
                                .kern_group_side2
                                .as_ref()
                                .is_some_and(|group| !group.is_empty())
                            || font
                                .kerning_groups
                                .side1
                                .values()
                                .chain(font.kerning_groups.side2.values())
                                .any(|group| group.contains(&glyph.name))
                    }
                    "is_hangul_key" => {
                        glyph
                            .user_data
                            .get("is_hangul_key")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                            || font.extra.lib.get("hangul_composition_groups").is_some_and(
                                |groups| {
                                    hangul_group_contains(groups, glyph.name.as_ref(), 0, &mut 0)
                                },
                            )
                    }
                    _ => false,
                })
            }
        }
    }

    fn text(&self, actual: &str) -> bool {
        self.texts(std::iter::once(actual))
    }
    fn texts<'a>(&self, values: impl Iterator<Item = &'a str>) -> bool {
        let mut values = values;
        let positive = values.any(|actual| match &self.value {
            Operand::Text(expected) if self.operator == Op::Contains => actual.contains(expected),
            Operand::Text(expected) => actual == expected,
            Operand::Pattern(regex) => regex.is_match(actual),
            _ => false,
        });
        if self.operator == Op::Ne {
            !positive
        } else {
            positive
        }
    }
    fn boolean(&self, actual: bool) -> bool {
        matches!(self.value, Operand::Boolean(expected) if (actual == expected) != (self.operator == Op::Ne))
    }
    fn number(&self, actual: f64) -> bool {
        self.number_with(actual, self.operator)
    }
    fn number_with(&self, actual: f64, operator: Op) -> bool {
        let Operand::Number(expected) = self.value else {
            return false;
        };
        match operator {
            Op::Eq => actual == expected,
            Op::Ne => actual != expected,
            Op::Gt => actual > expected,
            Op::Gte => actual >= expected,
            Op::Lt => actual < expected,
            Op::Lte => actual <= expected,
            _ => false,
        }
    }
}

fn category_name(cp: u32) -> &'static str {
    use fc_glyphdata::Category;
    match fc_glyphdata::category(cp) {
        Category::Letter => "Letter",
        Category::Number => "Number",
        Category::Punctuation => "Punctuation",
        Category::Symbol => "Symbol",
        Category::Separator => "Separator",
        Category::Mark => "Mark",
        Category::Format => "Format",
        Category::Private => "Private",
        Category::Other => "Other",
    }
}

fn glyph_case(glyph: &Glyph) -> &str {
    if let Some(case) = glyph.user_data.get("case").and_then(Value::as_str) {
        return case;
    }
    if glyph
        .name
        .split('.')
        .any(|suffix| matches!(suffix, "sc" | "smcp" | "c2sc"))
    {
        return "Smallcaps";
    }
    match glyph.sub_category.as_deref().or_else(|| {
        glyph
            .unicodes
            .first()
            .map(|cp| fc_glyphdata::sub_category(*cp))
    }) {
        Some("Uppercase") => "Uppercase",
        Some("Lowercase") => "Lowercase",
        Some("Smallcaps") => "Smallcaps",
        _ => "Uncased",
    }
}

fn hangul_group_contains(value: &Value, name: &str, depth: usize, nodes: &mut usize) -> bool {
    *nodes += 1;
    if depth > 8 || *nodes > 65_536 {
        return false;
    }
    match value {
        Value::String(value) => value == name,
        Value::Array(values) => hangul_values_contain(values.iter(), name, depth, nodes),
        Value::Object(values) => hangul_values_contain(values.values(), name, depth, nodes),
        _ => false,
    }
}

fn hangul_values_contain<'a>(
    values: impl Iterator<Item = &'a Value>,
    name: &str,
    depth: usize,
    nodes: &mut usize,
) -> bool {
    for value in values {
        if *nodes >= 65_536 {
            return false;
        }
        if hangul_group_contains(value, name, depth + 1, nodes) {
            return true;
        }
    }
    false
}

fn metric_keys(glyph: &Glyph) -> [Option<&str>; 5] {
    let keys = &glyph.metrics_keys;
    [
        keys.left.as_deref(),
        keys.right.as_deref(),
        keys.width.as_deref(),
        keys.vertical_origin.as_deref(),
        keys.vertical_width.as_deref(),
    ]
}

fn metrics_out_of_sync(font: &Font, glyph: &Glyph, layer: Option<&Layer>) -> bool {
    metric_keys(glyph).iter().enumerate().any(|(metric, key)| {
        if key.is_none() {
            return false;
        }
        let Some(layer) = layer else {
            return true;
        };
        let LayerKind::Master(master) = &layer.kind else {
            return true;
        };
        match (
            resolve_metric(font, glyph, master, metric, 0),
            explicit_metric(font, glyph, master, metric),
        ) {
            (Some(expected), Some(actual)) => {
                !expected.is_finite()
                    || !actual.is_finite()
                    || (expected - actual).abs() > 0.000_001
            }
            _ => true,
        }
    })
}

// Bounded recursion also covers vertical keys, which the horizontal metrics
// resolver does not expose. Broken references/cycles are reported as out of sync.
fn resolve_metric(
    font: &Font,
    glyph: &Glyph,
    master: &fc_model::MasterId,
    metric: usize,
    depth: usize,
) -> Option<f64> {
    if depth > 64 {
        return None;
    }
    let Some(key) = metric_keys(glyph)[metric] else {
        return explicit_metric(font, glyph, master, metric);
    };
    let body = key.trim().strip_prefix('=')?;
    if let Ok(number) = body.parse::<f64>() {
        return number.is_finite().then_some(number);
    }
    let (mirrored, body) = body
        .strip_prefix('|')
        .map_or((false, body), |body| (true, body));
    let (name, offset) = body
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            (index > 0 && matches!(character, '+' | '-'))
                .then(|| {
                    body[index..]
                        .parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite())
                        .map(|offset| (&body[..index], offset))
                })
                .flatten()
        })
        .unwrap_or((body, 0.));
    let metric = if mirrored && metric < 2 {
        1 - metric
    } else {
        metric
    };
    let value = resolve_metric(font, font.glyph(name)?, master, metric, depth + 1)? + offset;
    value.is_finite().then_some(value)
}

fn explicit_metric(
    font: &Font,
    glyph: &Glyph,
    master: &fc_model::MasterId,
    metric: usize,
) -> Option<f64> {
    let layer = glyph.master_layer(master)?;
    match metric {
        0 | 1 => {
            let bounds = fc_geom::layer_bbox(font, glyph.name.as_ref(), master).ok()?;
            Some(if metric == 0 {
                bounds.map_or(0., |bounds| bounds.x0)
            } else {
                bounds.map_or(layer.width, |bounds| layer.width - bounds.x1)
            })
        }
        2 => Some(layer.width),
        3 => layer.vertical_origin,
        4 => layer.vertical_width,
        _ => None,
    }
}

#[cfg(test)]
#[path = "smart_filter_eval_tests.rs"]
mod tests;
