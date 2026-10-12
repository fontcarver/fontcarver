//! Adapt model conditional sources to fontc feature variations.
use crate::Error;
use fc_model::Font;
use fontdrasil::{
    coords::DesignCoord,
    types::{GlyphName, Tag},
};
use fontir::ir::{Condition, Rule, Substitution, VariableFeature};

pub(crate) fn prepare(font: &Font) -> Result<(Font, Option<VariableFeature>), Error> {
    let (result, rules) =
        fc_model::query::expand_brackets(font).map_err(|e| Error::InvalidSource(e.to_string()))?;
    if rules.is_empty() {
        return Ok((result, None));
    }
    let rules = rules
        .into_iter()
        .map(|rule| Rule {
            // One condition set per rule: a model condition set is a
            // conjunction, which is exactly an OpenType `ConditionSet`.
            conditions: vec![rule
                .conditions
                .iter()
                .map(|condition| {
                    Condition::new(
                        Tag::new(&condition.axis.0),
                        condition.min.map(DesignCoord::new),
                        condition.max.map(DesignCoord::new),
                    )
                })
                .collect()],
            substitutions: rule
                .substitutions
                .into_iter()
                .filter(|(base, _)| font.glyph(base).is_some_and(|g| g.export))
                .map(|(base, alias)| Substitution {
                    replace: GlyphName::new(base.as_ref()),
                    with: GlyphName::new(alias.as_ref()),
                })
                .collect(),
        })
        .collect();
    Ok((
        result,
        Some(VariableFeature {
            features: vec![Tag::new(b"rvrn")],
            rules,
        }),
    ))
}
