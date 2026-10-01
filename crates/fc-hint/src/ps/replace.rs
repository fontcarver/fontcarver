//! Ghost hints at alignment zones, point-bound hint replacement and counter groups.

use std::collections::HashSet;

use fc_model::{Contour, HintMask, PointKind, PsHints, StemHint};
use kurbo::{BezPath, Point};

use super::stems::{edge_side, flat_edges, Stem};

/// Largest number of replacement masks one glyph may carry before the analyser
/// falls back to a single compatible stem set.
const MAX_MASKS: usize = 256;

/// One emitted stem with the geometry needed to bind it to outline points.
///
/// Edges are stored as the normalised closed interval the rasteriser constrains,
/// so a ghost hint (`-20`/`-21`) collapses to its single real edge exactly as
/// `fc_model::query::path_hint_masks` interprets it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bound {
    /// Whether this stem constrains the vertical axis (a `hstem`).
    pub(super) horizontal: bool,
    /// Lower real edge coordinate.
    low: f64,
    /// Upper real edge coordinate; equal to `low` for a ghost hint.
    high: f64,
    /// Overlap start along the stem's direction.
    from: f64,
    /// Overlap end along the stem's direction.
    to: f64,
    /// Whether this stem is active only where a mask selects it.
    pub(super) replacement: bool,
}

impl Bound {
    pub(super) fn new(horizontal: bool, stem: &Stem, replacement: bool) -> Self {
        Self {
            horizontal,
            low: stem.start,
            high: stem.start + stem.width,
            from: stem.from,
            to: stem.to,
            replacement,
        }
    }

    /// An always-active ghost hint, which has one edge and no overlap span.
    pub(super) fn ghost(stem: &StemHint) -> Self {
        let edge = if stem.width == -21.0 {
            stem.start - 21.0
        } else {
            stem.start
        };
        Self {
            horizontal: true,
            low: edge,
            high: edge,
            from: f64::NEG_INFINITY,
            to: f64::INFINITY,
            replacement: false,
        }
    }

    /// Whether `point` lies on one of this stem's edges, inside its overlap.
    fn governs(&self, point: &fc_model::Point, upem: f64) -> bool {
        let edge_tolerance = (upem / 500.0).max(1.0);
        let span_tolerance = upem / 100.0;
        let (cross, along) = if self.horizontal {
            (point.y, point.x)
        } else {
            (point.x, point.y)
        };
        let near = (cross - self.low).abs().min((cross - self.high).abs());
        near <= edge_tolerance
            && along >= self.from - span_tolerance
            && along <= self.to + span_tolerance
    }

    fn conflicts(&self, other: &Self) -> bool {
        self.horizontal == other.horizontal
            && self.low < other.high + 0.5
            && other.low < self.high + 0.5
    }
}

/// Proposes ghost hints for alignment edges that no stem already constrains.
///
/// Type 1 ghost hints mark a single alignment edge: width `-20` stores the top
/// edge directly, while width `-21` stores the edge plus 21, so the real edge is
/// `start - 21` (the convention real fonts use, verified against Nimbus Sans).
/// Only edges inside an alignment zone are proposed, because a ghost outside a
/// zone constrains nothing the rasteriser would otherwise align.
pub(super) fn ghosts(
    path: &BezPath,
    lines: &[(Point, Point)],
    zones: &[(f64, f64)],
    upem: f64,
    existing: &[Stem],
    room: usize,
) -> Vec<StemHint> {
    if zones.is_empty() || room == 0 {
        return Vec::new();
    }
    let mut widest: Vec<super::stems::Edge> = Vec::new();
    for edge in flat_edges(lines, true, upem) {
        if edge.to - edge.from < upem / 100.0
            || !zones
                .iter()
                .any(|(bottom, top)| edge.position >= *bottom && edge.position <= *top)
            || existing.iter().any(|stem| stem.covers(edge.position))
        {
            continue;
        }
        match widest
            .iter_mut()
            .find(|other| (other.position - edge.position).abs() <= 1.0)
        {
            Some(other) => {
                if edge.to - edge.from > other.to - other.from {
                    *other = edge;
                }
            }
            None => widest.push(edge),
        }
        if widest.len() >= room * 4 {
            break;
        }
    }
    widest.sort_by(|a, b| a.position.total_cmp(&b.position));
    let mut result = Vec::new();
    for edge in widest {
        let Some(top) = edge_side(path, &edge, upem) else {
            continue;
        };
        result.push(if top {
            StemHint {
                start: edge.position,
                width: -20.0,
                ranges: Vec::new(),
            }
        } else {
            StemHint {
                start: edge.position + 21.0,
                width: -21.0,
                ranges: Vec::new(),
            }
        });
        if result.len() == room {
            break;
        }
    }
    result
}

/// Builds the point-bound masks that activate each replacement stem.
///
/// The primary stems stay active everywhere except where a replacement stem they
/// conflict with is active. A mask is emitted at the first on-curve point of the
/// first contour and wherever the active set changes, which is exactly the
/// `hintmask`/`OtherSubrs 3` semantics. Returns `None` when the glyph would need
/// more masks than the analyser is willing to author, so the caller can fall back
/// to the compatible primary set instead of emitting an unbounded program.
/// The stored points a mask may not bind to because a flex hint covers them.
///
/// A flex hint's joint is still available: its mask is emitted before the flex
/// operator. Every point from there to the pair's end belongs to the operator.
pub(super) fn flex_points(hints: &PsHints, contours: &[Contour]) -> HashSet<(usize, usize)> {
    let mut result = HashSet::new();
    for flex in &hints.flex_hints {
        let Some(count) = contours.get(flex.contour).map(|c| c.points.len()) else {
            continue;
        };
        if count == 0 {
            continue;
        }
        for offset in 4..=6 {
            result.insert((flex.contour, (flex.point + offset) % count));
        }
    }
    result
}

pub(super) fn masks(
    contours: &[Contour],
    bounds: &[Bound],
    upem: f64,
    reserved: &HashSet<(usize, usize)>,
) -> Option<Vec<HintMask>> {
    let count = bounds.len();
    if count == 0 || !bounds.iter().any(|bound| bound.replacement) {
        return Some(Vec::new());
    }
    let bytes = count.div_ceil(8);
    let blocked: Vec<Vec<usize>> = bounds
        .iter()
        .map(|bound| {
            if !bound.replacement {
                return Vec::new();
            }
            bounds
                .iter()
                .enumerate()
                .filter(|(_, other)| !other.replacement && bound.conflicts(other))
                .map(|(index, _)| index)
                .collect()
        })
        .collect();
    let mut result: Vec<HintMask> = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    for (contour_index, contour) in contours.iter().enumerate() {
        let start = contour
            .points
            .iter()
            .position(|point| point.kind != PointKind::OffCurve)?;
        for offset in 0..contour.points.len() {
            let index = (start + offset) % contour.points.len();
            let point = &contour.points[index];
            if point.kind == PointKind::OffCurve {
                continue;
            }
            let mut active = vec![true; count];
            for (position, bound) in bounds.iter().enumerate() {
                if bound.replacement {
                    active[position] = bound.governs(point, upem);
                }
            }
            for (position, bound) in bounds.iter().enumerate() {
                if bound.replacement && active[position] {
                    for conflict in &blocked[position] {
                        active[*conflict] = false;
                    }
                }
            }
            let mut mask = vec![0u8; bytes];
            for (position, enabled) in active.iter().enumerate() {
                if *enabled {
                    mask[position / 8] |= 0x80 >> (position % 8);
                }
            }
            let first = contour_index == 0 && offset == 0;
            // Leaving the previous mask in place keeps the active set unchanged
            // across a point a flex operator owns; the next point emits again.
            if reserved.contains(&(contour_index, index)) {
                continue;
            }
            if !first && previous.as_ref() == Some(&mask) {
                continue;
            }
            if result.len() == MAX_MASKS {
                return None;
            }
            previous = Some(mask.clone());
            result.push(HintMask {
                counter: false,
                closing: false,
                contour: contour_index,
                point: index,
                mask,
            });
        }
    }
    // Overlapping stems need an explicit active set before the first drawing
    // operator; without a mask at the first point the whole set would be active.
    result
        .first()
        .is_some_and(|mask| mask.contour == 0)
        .then_some(result)
}

/// Type 1 section 6.4: positive widths, equal outer widths, equal centre spacing.
///
/// `hstem3`/`vstem3` is the only counter-control form every PostScript
/// interpreter understands, so the analyser proposes it exactly when the axis
/// holds a valid triple and nothing else.
pub(super) fn counter_triple(stems: &[StemHint]) -> bool {
    stems.len() == 3
        && stems.iter().all(|stem| stem.width > 0.0)
        && stems
            .windows(2)
            .all(|pair| pair[0].start + pair[0].width <= pair[1].start)
        && (stems[0].width - stems[2].width).abs() <= 1.0 / 65536.0
        && ((stems[0].start + stems[0].width / 2.0) - 2.0 * (stems[1].start + stems[1].width / 2.0)
            + (stems[2].start + stems[2].width / 2.0))
            .abs()
            <= 1.0 / 65536.0
}

/// The counter mask that selects one axis's complete stem set.
pub(super) fn counter_mask(
    contours: &[Contour],
    offset: usize,
    length: usize,
    count: usize,
) -> Option<HintMask> {
    let contour = contours.first()?;
    let point = contour
        .points
        .iter()
        .position(|point| point.kind != PointKind::OffCurve)?;
    let mut mask = vec![0u8; count.div_ceil(8)];
    for index in offset..offset + length {
        mask[index / 8] |= 0x80 >> (index % 8);
    }
    Some(HintMask {
        counter: true,
        closing: false,
        contour: 0,
        point,
        mask,
    })
}

/// Whether the resolved outline is the layer's own geometry, point for point.
///
/// Hint masks address stored contour points, so a layer whose outline comes from
/// components has no point identity of its own to bind a mask to.
pub(super) fn maskable(layer: &fc_model::Layer) -> bool {
    layer.components.is_empty() && !layer.contours.is_empty()
}
