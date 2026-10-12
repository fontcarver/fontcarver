//! Bounded geometric PostScript hint analysis.
//!
//! The analyser proposes stems from facing outline edges, ghost hints for
//! alignment edges a stem cannot describe, point-bound hint replacement where
//! stems conflict, `hstem3`/`vstem3` counter groups where an axis holds a valid
//! triple, and the private-dictionary values the result implies. It never
//! mutates a document: every result is a proposal the caller applies.

mod replace;
mod stems;

use std::collections::BTreeMap;

use fc_model::{Font, GlyphName, Layer, MasterId, PsHints, PsPrivate, StemHint};
use kurbo::{BezPath, PathEl, Point};

use crate::Error;

/// One analysis of a captured selection, with nothing applied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    /// Proposed hint set for each selected glyph, in selection order.
    pub hints: Vec<(GlyphName, PsHints)>,
    /// Proposed values for the default private dictionary.
    pub private: PsPrivate,
    /// Proposed values for each additional CID private dictionary the selection
    /// uses, by its one-based index. Dictionaries no glyph uses are untouched.
    pub cid_private: Vec<(u8, PsPrivate)>,
}

/// Analyzes a captured selection without mutating any source or partially applying hints.
///
/// Glyph layers assigned to an additional CID private dictionary are analysed
/// against that dictionary's own alignment data, and each dictionary the
/// selection uses receives its own inferred values.
pub fn analyze(font: &Font, master: &MasterId, names: &[GlyphName]) -> Result<Analysis, Error> {
    if names.len() > 65_535 {
        return Err(Error::Invalid(
            "PS autohint selection exceeds glyph limit".into(),
        ));
    }
    let source = font
        .master(master)
        .ok_or_else(|| Error::Invalid("missing hint master".into()))?;
    let mut unique = std::collections::HashSet::new();
    let mut hints = Vec::new();
    let mut by_dictionary: BTreeMap<u8, Vec<PsHints>> = BTreeMap::new();
    let mut points = 0usize;
    for name in names {
        if !unique.insert(name) {
            continue;
        }
        let layer = font
            .glyph(name)
            .and_then(|g| g.master_layer(master))
            .ok_or_else(|| Error::Invalid(format!("missing master layer for {name}")))?;
        let dictionary = layer.cff_private_dict_index.unwrap_or(0);
        if dictionary > 0 && usize::from(dictionary) > source.cff_private_dicts.len() {
            return Err(Error::Invalid(format!(
                "{name} is assigned to private dictionary {dictionary}, which this master does not define"
            )));
        }
        points =
            points.saturating_add(layer.contours.iter().map(|c| c.points.len()).sum::<usize>());
        if points > 8_000_000 {
            return Err(Error::Invalid(
                "PS autohint selection exceeds point budget".into(),
            ));
        }
        let detected = detect(font, name, master)?;
        by_dictionary
            .entry(dictionary)
            .or_default()
            .push(detected.clone());
        hints.push((name.clone(), detected));
    }
    let private = infer_private_for(
        font,
        master,
        0,
        by_dictionary.get(&0).map(Vec::as_slice).unwrap_or_default(),
    )?;
    let cid_private = by_dictionary
        .iter()
        .filter(|(index, _)| **index > 0)
        .map(|(index, hints)| Ok((*index, infer_private_for(font, master, *index, hints)?)))
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Analysis {
        hints,
        private,
        cid_private,
    })
}

/// Detects one layer's complete proposed hint set.
///
/// Candidate stems must overlap and have filled ink between them. The highest
/// scoring compatible stems stay active for the whole glyph; a conflicting stem
/// becomes a replacement stem bound to the outline points it governs. Alignment
/// edges no stem constrains become ghost hints, and an axis holding exactly one
/// valid triple also receives a counter group. Master stem widths and alignment
/// zones bias the geometric score.
pub fn detect(font: &Font, glyph: &GlyphName, master: &MasterId) -> Result<PsHints, Error> {
    let source = font
        .master(master)
        .ok_or_else(|| Error::Invalid("missing hint master".into()))?;
    let layer = font
        .glyph(glyph)
        .and_then(|g| g.master_layer(master))
        .ok_or_else(|| Error::Invalid(format!("missing master layer for {glyph}")))?;
    let path =
        fc_model::flatten_layer(font, glyph, master).map_err(|e| Error::Invalid(e.to_string()))?;
    check_outline(&path)?;
    let upem = f64::from(font.info.units_per_em);
    let zones = alignment_zones(font, master, layer.cff_private_dict_index.unwrap_or(0));
    let lines = stems::edges(&path)?;
    let mut work = 4_000_000usize;
    let mut axes = Vec::new();
    for horizontal in [true, false] {
        let preferred: Vec<_> = source
            .stems
            .iter()
            .filter(|s| s.horizontal == horizontal && s.width.is_finite() && s.width > 0.0)
            .map(|s| s.width)
            .collect();
        axes.push(stems::axis(
            &path, &lines, horizontal, upem, &preferred, &zones, &mut work,
        )?);
    }
    let (horizontal_axis, vertical_axis) = (&axes[0], &axes[1]);
    let used = horizontal_axis.primary.len()
        + horizontal_axis.replacement.len()
        + vertical_axis.primary.len()
        + vertical_axis.replacement.len();
    let ghosts = replace::ghosts(
        &path,
        &lines,
        &zones,
        upem,
        &horizontal_axis.primary,
        96usize.saturating_sub(used).min(16),
    );
    assemble(layer, horizontal_axis, vertical_axis, &ghosts, upem)
}

/// Orders the emitted stems, binds replacement masks and proposes counter groups.
///
/// Primary stems and ghost hints come first in coordinate order, then the
/// replacement stems, so a glyph with no conflicts emits exactly the sorted
/// compatible set it emitted before replacement existed.
fn assemble(
    layer: &Layer,
    horizontal: &stems::Axis,
    vertical: &stems::Axis,
    ghosts: &[StemHint],
    upem: f64,
) -> Result<PsHints, Error> {
    let maskable = replace::maskable(layer);
    let mut bounds = Vec::new();
    // Flex hints, dot sections and an unresolved imported flex marker describe
    // the outline, not the stem analysis, so analysis keeps them. Dropping them
    // would silently lose authored data the moment a glyph is analysed.
    let mut result = PsHints {
        flex: layer.hints.flex,
        flex_hints: layer.hints.flex_hints.clone(),
        dot_sections: layer.hints.dot_sections.clone(),
        ..PsHints::default()
    };
    let mut primary_lengths = [0usize; 2];
    for (index, (axis, horizontal_axis)) in [(horizontal, true), (vertical, false)]
        .into_iter()
        .enumerate()
    {
        let mut primary: Vec<_> = axis
            .primary
            .iter()
            .map(|stem| {
                (
                    StemHint {
                        start: stem.start,
                        width: stem.width,
                        ranges: Vec::new(),
                    },
                    replace::Bound::new(horizontal_axis, stem, false),
                )
            })
            .collect();
        if horizontal_axis {
            primary.extend(
                ghosts
                    .iter()
                    .map(|ghost| (ghost.clone(), replace::Bound::ghost(ghost))),
            );
            primary.sort_by(|a, b| ghost_edge(&a.0).total_cmp(&ghost_edge(&b.0)));
        }
        primary_lengths[index] = primary.len();
        let replacements = axis.replacement.iter().map(|stem| {
            (
                StemHint {
                    start: stem.start,
                    width: stem.width,
                    ranges: Vec::new(),
                },
                replace::Bound::new(horizontal_axis, stem, true),
            )
        });
        let target = if horizontal_axis {
            &mut result.h_stems
        } else {
            &mut result.v_stems
        };
        for (stem, bound) in primary.into_iter().chain(replacements) {
            // A replacement stem without bindable points cannot be expressed.
            if bound.replacement && !maskable {
                continue;
            }
            target.push(stem);
            bounds.push(bound);
        }
    }
    if result.h_stems.len() + result.v_stems.len() > 96 {
        result.v_stems.truncate(96 - result.h_stems.len());
        bounds.truncate(result.h_stems.len() + result.v_stems.len());
    }
    if bounds.iter().any(|bound| bound.replacement) {
        // A mask from a flex hint's joint onwards cannot be activated, because
        // one operator draws both of its curves.
        let reserved = replace::flex_points(&result, &layer.contours);
        match replace::masks(&layer.contours, &bounds, upem, &reserved) {
            Some(masks) => result.hint_masks = masks,
            None => return assemble_primary_only(layer, horizontal, vertical, ghosts, upem),
        }
    } else {
        propose_counters(&mut result, layer, primary_lengths);
    }
    result
        .validate(&layer.contours)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(result)
}

/// Falls back to the compatible stem set when replacement would need too many masks.
fn assemble_primary_only(
    layer: &Layer,
    horizontal: &stems::Axis,
    vertical: &stems::Axis,
    ghosts: &[StemHint],
    upem: f64,
) -> Result<PsHints, Error> {
    let strip = |axis: &stems::Axis| stems::Axis {
        primary: axis.primary.clone(),
        replacement: Vec::new(),
    };
    assemble(layer, &strip(horizontal), &strip(vertical), ghosts, upem)
}

/// Adds a counter group when exactly one axis holds a valid, complete triple.
///
/// `hstem3`/`vstem3` requires the axis to hold the triple and nothing else, so a
/// group is proposed only for an axis whose whole primary set is that triple.
fn propose_counters(hints: &mut PsHints, layer: &Layer, primary_lengths: [usize; 2]) {
    let count = hints.h_stems.len() + hints.v_stems.len();
    let axes = [
        (0usize, &hints.h_stems, primary_lengths[0]),
        (hints.h_stems.len(), &hints.v_stems, primary_lengths[1]),
    ];
    let mut masks = Vec::new();
    for (offset, stems, length) in axes {
        if length != stems.len() || !replace::counter_triple(stems) {
            continue;
        }
        if let Some(mask) = replace::counter_mask(&layer.contours, offset, length, count) {
            masks.push(mask);
        }
    }
    // Type 1 accepts at most one counter group per axis, declared together.
    hints.hint_masks = masks;
}

fn ghost_edge(stem: &StemHint) -> f64 {
    if stem.width == -21.0 {
        stem.start - 21.0
    } else {
        stem.start
    }
}

fn check_outline(path: &BezPath) -> Result<(), Error> {
    if path.elements().len() > 16_384
        || !path.is_finite()
        || path.elements().iter().any(|el| {
            let valid = |p: &Point| p.x.abs() <= 32767.0 && p.y.abs() <= 32767.0;
            match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => !valid(p),
                PathEl::QuadTo(a, b) => !valid(a) || !valid(b),
                PathEl::CurveTo(a, b, c) => !valid(a) || !valid(b) || !valid(c),
                PathEl::ClosePath => false,
            }
        })
    {
        return Err(Error::Invalid(
            "PS autohint outline is invalid or exceeds the coordinate or analysis budget".into(),
        ));
    }
    Ok(())
}

/// The alignment intervals one private dictionary aligns outlines to.
///
/// Explicit blue values win; otherwise the master's alignment zones, and finally
/// the master metrics, supply the intervals. Every interval is returned as a
/// `(low, high)` pair in font units.
fn alignment_zones(font: &Font, master: &MasterId, dictionary: u8) -> Vec<(f64, f64)> {
    let Some(source) = font.master(master) else {
        return Vec::new();
    };
    let private = if dictionary == 0 {
        Some(&source.private_dict)
    } else {
        source.cff_private_dicts.get(usize::from(dictionary) - 1)
    };
    let mut zones = Vec::new();
    if let Some(private) = private {
        let fuzz = private.blue_fuzz.unwrap_or(1.0).clamp(0.0, 32.0);
        for array in [&private.blue_values, &private.other_blues] {
            for pair in array.chunks_exact(2) {
                zones.push((pair[0] - fuzz, pair[1] + fuzz));
            }
        }
    }
    if zones.is_empty() {
        for zone in &source.alignment_zones {
            zones.push((
                zone.position.min(zone.position + zone.size),
                zone.position.max(zone.position + zone.size),
            ));
        }
    }
    if zones.is_empty() {
        let overshoot = (f64::from(font.info.units_per_em) / 100.0).max(1.0);
        zones.extend([
            (-overshoot, 0.0),
            (source.metrics.x_height, source.metrics.x_height + overshoot),
            (
                source.metrics.cap_height,
                source.metrics.cap_height + overshoot,
            ),
        ]);
        if source.metrics.descender < -overshoot {
            zones.push((
                source.metrics.descender - overshoot,
                source.metrics.descender,
            ));
        }
    }
    zones.retain(|(low, high)| low.is_finite() && high.is_finite() && low <= high);
    zones.sort_by(|a, b| a.0.total_cmp(&b.0));
    zones
}

/// Infers missing private values for the default dictionary from analysed hints.
/// Existing private values remain authoritative and are validated on return.
pub fn infer_private(
    font: &Font,
    master: &MasterId,
    hints: &[PsHints],
) -> Result<PsPrivate, Error> {
    infer_private_for(font, master, 0, hints)
}

/// Infers missing values for one private dictionary from the hints that use it.
///
/// Alignment zones and named master stems describe the default dictionary, so an
/// additional CID dictionary keeps its own explicit values and learns only the
/// stem statistics of the glyphs assigned to it.
pub fn infer_private_for(
    font: &Font,
    master: &MasterId,
    dictionary: u8,
    hints: &[PsHints],
) -> Result<PsPrivate, Error> {
    let source = font
        .master(master)
        .ok_or_else(|| Error::Invalid("missing hint master".into()))?;
    let mut private = if dictionary == 0 {
        source.private_dict.clone()
    } else {
        source
            .cff_private_dicts
            .get(usize::from(dictionary) - 1)
            .cloned()
            .ok_or_else(|| Error::Invalid("missing CID private dictionary".into()))?
    };
    let overshoot = (f64::from(font.info.units_per_em) / 100.0).max(1.0);
    if private.blue_values.is_empty() {
        let mut zones: Vec<(f64, f64)> = if source.alignment_zones.is_empty() {
            vec![
                (-overshoot, 0.0),
                (source.metrics.x_height, source.metrics.x_height + overshoot),
                (
                    source.metrics.cap_height,
                    source.metrics.cap_height + overshoot,
                ),
            ]
        } else {
            source
                .alignment_zones
                .iter()
                .filter(|z| z.size >= 0.0 || z.position == 0.0)
                .map(|z| {
                    (
                        z.position.min(z.position + z.size),
                        z.position.max(z.position + z.size),
                    )
                })
                .collect()
        };
        zones.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = f64::NEG_INFINITY;
        for (low, high) in zones {
            if low <= end {
                continue;
            }
            private.blue_values.extend([low, high]);
            end = high;
            if private.blue_values.len() == 14 {
                break;
            }
        }
    }
    if private.other_blues.is_empty() {
        if source.alignment_zones.is_empty() && source.metrics.descender < -overshoot {
            private.other_blues = vec![
                source.metrics.descender - overshoot,
                source.metrics.descender,
            ];
        } else {
            let mut zones: Vec<_> = source
                .alignment_zones
                .iter()
                .filter(|z| z.size < 0.0 && z.position != 0.0)
                .collect();
            zones.sort_by(|a, b| a.position.total_cmp(&b.position));
            for zone in zones.iter().take(5) {
                private
                    .other_blues
                    .extend([zone.position + zone.size, zone.position]);
            }
        }
    }
    for horizontal in [true, false] {
        let mut widths = BTreeMap::<i32, usize>::new();
        for hint in hints {
            for stem in if horizontal {
                &hint.h_stems
            } else {
                &hint.v_stems
            } {
                if stem.width > 0.0 {
                    *widths.entry(stem.width.round() as i32).or_default() += 1;
                }
            }
        }
        if dictionary == 0 {
            for stem in source.stems.iter().filter(|s| s.horizontal == horizontal) {
                if !stem.width.is_finite() || stem.width <= 0.0 || stem.width > 32767.0 {
                    return Err(Error::Invalid("invalid master stem width".into()));
                }
                *widths.entry(stem.width.round() as i32).or_default() += hints.len().max(1);
            }
        }
        let mut ranked: Vec<_> = widths.into_iter().collect();
        ranked.sort_by_key(|(width, count)| (std::cmp::Reverse(*count), *width));
        let (standard, snaps) = if horizontal {
            (&mut private.std_hw, &mut private.stem_snap_h)
        } else {
            (&mut private.std_vw, &mut private.stem_snap_v)
        };
        if standard.is_none() {
            *standard = ranked.first().map(|(width, _)| f64::from(*width));
        }
        if snaps.is_empty() {
            *snaps = ranked
                .iter()
                .take(12)
                .map(|(width, _)| f64::from(*width))
                .collect();
            snaps.sort_by(f64::total_cmp);
        }
    }
    private
        .validate()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(private)
}
