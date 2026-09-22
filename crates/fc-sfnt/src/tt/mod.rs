//! Compiled TrueType point identities for safe instruction authoring.

mod basis;
mod points;
mod span;
mod write;

pub use write::{with_instructions, GlyphProgram};

use crate::Error;

/// A stored TrueType point, including off-curve controls and expanded components.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphPoint {
    /// Unscaled position in font units, with the vertical axis pointing up.
    pub position: kurbo::Point,
    /// Whether this point lies on the outline.
    pub on_curve: bool,
    /// Zero-based contour number after component expansion.
    pub contour: u32,
}

/// The point numbering and metrics to which a glyph program is bound.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphBasis {
    /// Versioned digest of stored point order, components, transforms and metrics.
    /// Hint bytecode is deliberately excluded.
    pub fingerprint: String,
    /// Points in the exact order used by TrueType instructions.
    pub points: Vec<GlyphPoint>,
    /// Index of the first of four phantom points following the outline points.
    pub phantom_start: u32,
    /// Horizontal phantom x positions implied by the glyph's hmtx metrics.
    /// Components using USE_MY_METRICS can replace these during interpretation.
    pub horizontal_phantoms: [f64; 2],
    /// Vertical phantom y positions when explicit vertical metrics exist.
    pub vertical_phantoms: Option<[f64; 2]>,
    /// Whether a component supplies metrics during TrueType interpretation.
    pub component_metrics: bool,
}

/// Reads stored point indices and a stable binding from a static TrueType sfnt.
///
/// This accepts compiled sfnt bytes, not a WOFF container. Recursive components,
/// truncated point streams and excessive expansion return errors.
pub fn point_basis(bytes: &[u8], glyph_id: u16) -> Result<GlyphBasis, Error> {
    let mut source = basis::Source::new(bytes)?;
    let fingerprint = source.fingerprint(glyph_id)?;
    let outline = points::expand(&source, glyph_id)?;
    let bbox = source.bounds(glyph_id)?;
    let horizontal = source
        .metric(glyph_id, false)?
        .ok_or_else(|| invalid("missing horizontal metrics"))?;
    let vertical_phantoms = source.metric(glyph_id, true)?.map(|(advance, bearing)| {
        let top = f64::from(bbox[3]) + f64::from(bearing);
        [top, top - f64::from(advance)]
    });
    let left = f64::from(bbox[0]) - f64::from(horizontal.1);
    Ok(GlyphBasis {
        fingerprint,
        phantom_start: outline.points.len() as u32,
        points: outline.points,
        horizontal_phantoms: [left, left + f64::from(horizontal.0)],
        vertical_phantoms,
        component_metrics: outline.component_metrics,
    })
}

/// Computes bindings with shared component caching and bounded total work.
pub fn fingerprints(bytes: &[u8], glyph_ids: &[u16]) -> Result<Vec<String>, Error> {
    if glyph_ids.len() > 65535 {
        return Err(invalid("too many glyph binding requests"));
    }
    let mut source = basis::Source::new(bytes)?;
    glyph_ids.iter().map(|id| source.fingerprint(*id)).collect()
}

fn invalid(reason: impl std::fmt::Display) -> Error {
    Error::parse("TrueType instruction binding", reason)
}
