//! Static COLRv1 paint graphs with named glyph references and bounded evaluation.
use crate::GlyphName;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

mod validate;

/// Maximum paint nodes and expanded evaluation work in an editable graph.
pub const MAX_COLR_PAINTS: usize = 65_535;

/// An indexed palette color at a gradient stop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColrColorStop {
    /// F2DOT14 offset in [-2, 2); source order resolves coincident stops.
    pub offset: f64,
    /// Palette entry, or null for the foreground color.
    pub palette_index: Option<u16>,
    /// Opacity multiplier in [0, 1].
    pub alpha: f64,
}

/// Gradient stops and their extend mode.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColrColorLine {
    /// 0 = pad, 1 = repeat, 2 = reflect.
    pub extend: u8,
    /// Stops retain source order. Empty lines paint transparent black.
    pub stops: Vec<ColrColorStop>,
}

/// An exact static transform, retaining the original parameter precision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ColrTransform {
    /// Six 16.16 coefficients in kurbo order: xx, yx, xy, yy, dx, dy.
    Affine {
        /// Matrix coefficients.
        matrix: [f64; 6],
    },
    /// Integer translation.
    Translate {
        /// Translation in font units.
        offset: [i16; 2],
    },
    /// Separate F2DOT14 scale factors, optionally about an integer center.
    Scale {
        /// Horizontal and vertical scale.
        scale: [f64; 2],
        /// Fixed center in font units.
        center: [i16; 2],
    },
    /// Rotation in degrees, optionally about an integer center.
    Rotate {
        /// Angle in degrees, stored as F2DOT14 fractions of 180 degrees.
        angle: f64,
        /// Fixed center in font units.
        center: [i16; 2],
    },
    /// Independent horizontal and vertical skews, in degrees.
    Skew {
        /// X and Y skew angles, stored as fractions of 180 degrees.
        angles: [f64; 2],
        /// Fixed center in font units.
        center: [i16; 2],
    },
}

/// One paint operation. Edges are zero-based indices into `ColrGraph.paints`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ColrPaint {
    /// Paint child operations in bottom-to-top order.
    Layers {
        /// Ordered paint indices (at most 255).
        paints: Vec<u32>,
    },
    /// Unbounded solid fill.
    Solid {
        /// Palette entry, or null for foreground.
        palette_index: Option<u16>,
        /// Opacity multiplier.
        alpha: f64,
    },
    /// Linear gradient: origin, end and perpendicular direction point.
    Linear {
        /// Gradient colors.
        color_line: ColrColorLine,
        /// Three font-unit positions.
        points: [[i16; 2]; 3],
    },
    /// Radial gradient between two circles.
    Radial {
        /// Gradient colors.
        color_line: ColrColorLine,
        /// Circle centers.
        centers: [[i16; 2]; 2],
        /// Unsigned circle radii.
        radii: [u16; 2],
    },
    /// Sweep gradient around a center.
    Sweep {
        /// Gradient colors.
        color_line: ColrColorLine,
        /// Center in font units.
        center: [i16; 2],
        /// Start and end angles in degrees.
        angles: [f64; 2],
    },
    /// Clip a paint to a monochrome outline.
    Glyph {
        /// Outline glyph name.
        glyph: GlyphName,
        /// Child paint index.
        paint: u32,
    },
    /// Reference another v1 color glyph's root.
    ColrGlyph {
        /// Base glyph in this graph's root map.
        glyph: GlyphName,
    },
    /// Transform a paint's coordinate system.
    Transform {
        /// Child paint index.
        paint: u32,
        /// Exact transform parameters.
        transform: ColrTransform,
    },
    /// Composite source over backdrop with an OpenType blend mode.
    Composite {
        /// Source paint index.
        source: u32,
        /// Backdrop paint index.
        backdrop: u32,
        /// OpenType CompositeMode, 0 through 27.
        mode: u8,
    },
}

/// Static COLRv1 roots, shared paints and optional clip boxes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColrGraph {
    /// Named color glyphs mapped to root paint indices.
    pub glyphs: BTreeMap<GlyphName, u32>,
    /// Shared paint node pool, including unreferenced nodes during editing.
    pub paints: Vec<ColrPaint>,
    /// Per-glyph [x_min, y_min, x_max, y_max] clip boxes.
    #[serde(default)]
    pub clips: BTreeMap<GlyphName, [i16; 4]>,
}

impl ColrPaint {
    /// Explicit paint indices referenced by this node (excluding named roots).
    pub fn children(&self) -> Vec<u32> {
        match self {
            Self::Layers { paints } => paints.clone(),
            Self::Glyph { paint, .. } | Self::Transform { paint, .. } => vec![*paint],
            Self::Composite {
                source, backdrop, ..
            } => vec![*source, *backdrop],
            _ => Vec::new(),
        }
    }
}

impl ColrGraph {
    /// Check fields, named references, graph cycles and expanded rendering work.
    pub fn validate(&self, order: &[GlyphName], entries: usize) -> Result<(), String> {
        validate::graph(self, order, entries)
    }

    /// Named outlines and color roots directly referenced by a root's paint DAG.
    /// Does not follow named roots; callers can compute transitive dependencies.
    pub fn dependencies(&self, root: u32) -> HashSet<GlyphName> {
        let mut result = HashSet::new();
        let mut seen = HashSet::new();
        let mut pending = vec![root];
        while let Some(index) = pending.pop() {
            if !seen.insert(index) {
                continue;
            }
            let Some(paint) = self.paints.get(index as usize) else {
                continue;
            };
            if let ColrPaint::Glyph { glyph, .. } | ColrPaint::ColrGlyph { glyph } = paint {
                result.insert(glyph.clone());
            }
            pending.extend(paint.children());
        }
        result
    }

    /// Update all root, outline and color-glyph references.
    pub fn rename_glyph(&mut self, from: &str, to: &GlyphName) {
        if let Some(paint) = self.glyphs.remove(from) {
            self.glyphs.insert(to.clone(), paint);
        }
        if let Some(clip) = self.clips.remove(from) {
            self.clips.insert(to.clone(), clip);
        }
        for paint in &mut self.paints {
            match paint {
                ColrPaint::Glyph { glyph, .. } | ColrPaint::ColrGlyph { glyph }
                    if glyph.as_ref() == from =>
                {
                    *glyph = to.clone()
                }
                _ => {}
            }
        }
    }

    /// Duplicate a root and its clip box without expanding the shared graph.
    pub fn duplicate_glyph(&mut self, source: &GlyphName, name: &GlyphName) {
        if let Some(paint) = self.glyphs.get(source).copied() {
            self.glyphs.insert(name.clone(), paint);
        }
        if let Some(clip) = self.clips.get(source).copied() {
            self.clips.insert(name.clone(), clip);
        }
    }

    /// Drop roots that depend on absent glyphs, and compact the remaining graph.
    /// Shared subgraphs are never partially painted after losing an outline.
    pub fn retain_glyphs(&mut self, mut keep: impl FnMut(&GlyphName) -> bool) -> usize {
        let original = self.glyphs.len();
        let absent: HashSet<_> = self
            .glyphs
            .keys()
            .filter(|name| !keep(name))
            .cloned()
            .collect();
        let mut parents = vec![Vec::new(); self.paints.len()];
        let mut bad = vec![false; self.paints.len()];
        let mut pending = Vec::new();
        for (i, paint) in self.paints.iter().enumerate() {
            let mut children = paint.children();
            let missing = match paint {
                ColrPaint::Glyph { glyph, .. } => !keep(glyph),
                ColrPaint::ColrGlyph { glyph } => {
                    if let Some(root) = self.glyphs.get(glyph) {
                        children.push(*root);
                    }
                    absent.contains(glyph) || !self.glyphs.contains_key(glyph)
                }
                _ => false,
            };
            if missing {
                bad[i] = true;
                pending.push(i);
            }
            for child in children {
                if let Some(parents) = parents.get_mut(child as usize) {
                    parents.push(i);
                } else {
                    bad[i] = true;
                    pending.push(i);
                }
            }
        }
        while let Some(index) = pending.pop() {
            for parent in &parents[index] {
                if !bad[*parent] {
                    bad[*parent] = true;
                    pending.push(*parent);
                }
            }
        }
        self.glyphs.retain(|name, index| {
            !absent.contains(name) && bad.get(*index as usize) == Some(&false)
        });
        self.clips.retain(|name, _| keep(name));
        let mut live = HashSet::new();
        let mut pending: Vec<_> = self.glyphs.values().copied().collect();
        while let Some(index) = pending.pop() {
            if !live.insert(index) {
                continue;
            }
            pending.extend(self.paints[index as usize].children());
        }
        let mapping: BTreeMap<_, _> = (0..self.paints.len() as u32)
            .filter(|i| live.contains(i))
            .enumerate()
            .map(|(to, from)| (from, to as u32))
            .collect();
        self.paints = self
            .paints
            .iter()
            .enumerate()
            .filter(|(i, _)| live.contains(&(*i as u32)))
            .map(|(_, p)| {
                let mut paint = p.clone();
                match &mut paint {
                    ColrPaint::Layers { paints } => {
                        for child in paints {
                            *child = mapping[child];
                        }
                    }
                    ColrPaint::Glyph { paint, .. } | ColrPaint::Transform { paint, .. } => {
                        *paint = mapping[paint]
                    }
                    ColrPaint::Composite {
                        source, backdrop, ..
                    } => {
                        *source = mapping[source];
                        *backdrop = mapping[backdrop];
                    }
                    _ => {}
                }
                paint
            })
            .collect();
        for root in self.glyphs.values_mut() {
            *root = mapping[root];
        }
        original - self.glyphs.len()
    }
}
