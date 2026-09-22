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
            conditions: vec![[Condition::new(
                Tag::new(&rule.axis.0),
                rule.min.map(DesignCoord::new),
                rule.max.map(DesignCoord::new),
            )]
            .into_iter()
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
