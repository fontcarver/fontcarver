//! Read-only diagnostics with bounded output and linear component traversal.

mod geometry;
mod kerning;

use std::collections::BTreeMap;

use fc_model::{Font, GlyphName, Layer, LayerId, LayerKind, MasterId};

/// Maximum number of diagnostics retained by a scan.
pub const MAX_ISSUES: usize = 2000;

/// Significance of a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Invalid model data or an export blocker.
    Error,
    /// Editable data that deserves attention.
    Warning,
    /// Informational feedback.
    Info,
}

/// A stable diagnostic and its most precise available model location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Stable machine-readable category.
    pub code: String,
    /// Significance of the issue.
    pub severity: Severity,
    /// Human-readable detail.
    pub message: String,
    /// Affected glyph, when present in the font.
    pub glyph: Option<GlyphName>,
    /// Associated master.
    pub master: Option<MasterId>,
    /// Persistent layer identity.
    pub layer: Option<LayerId>,
    /// Color-layer index, if the issue concerns a color outline.
    pub color_layer: Option<usize>,
    /// Contour index within the ordinary or color layer.
    pub contour: Option<usize>,
    /// Point index within the contour.
    pub point: Option<usize>,
}

/// A deterministic report; output storage never exceeds [`MAX_ISSUES`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Retained diagnostics, in traversal order.
    pub issues: Vec<Issue>,
    /// Number of detected diagnostics, including omitted entries.
    pub total: usize,
    /// Whether the response omitted any diagnostics.
    pub truncated: bool,
    /// Whether any detected diagnostic is an error, even when omitted.
    pub has_errors: bool,
}

#[derive(Clone, Default)]
struct Location {
    glyph: Option<GlyphName>,
    master: Option<MasterId>,
    layer: Option<LayerId>,
    color_layer: Option<usize>,
    contour: Option<usize>,
    point: Option<usize>,
}

impl Location {
    fn glyph(name: &GlyphName) -> Self {
        Self {
            glyph: Some(name.clone()),
            ..Self::default()
        }
    }

    fn layer(font: &Font, name: &GlyphName, layer: &Layer) -> Self {
        Self {
            glyph: Some(name.clone()),
            master: layer_master(font, layer).cloned(),
            layer: Some(layer.id.clone()),
            ..Self::default()
        }
    }
}

struct Collector {
    report: Report,
    limit: usize,
    component_limits_only: bool,
}

impl Collector {
    fn push(&mut self, code: &str, severity: Severity, message: impl Into<String>, at: &Location) {
        if self.component_limits_only
            && !matches!(
                code,
                "component-cycle" | "component-depth-limit" | "component-expansion-limit"
            )
        {
            return;
        }
        self.report.total += 1;
        self.report.has_errors |= severity == Severity::Error;
        if self.report.issues.len() < self.limit {
            self.report.issues.push(Issue {
                code: code.into(),
                severity,
                message: message.into(),
                glyph: at.glyph.clone(),
                master: at.master.clone(),
                layer: at.layer.clone(),
                color_layer: at.color_layer,
                contour: at.contour,
                point: at.point,
            });
        } else {
            self.report.truncated = true;
        }
    }

    fn error(&mut self, code: &str, message: impl Into<String>, at: &Location) {
        self.push(code, Severity::Error, message, at);
    }
}

/// Scans all model records without modifying the font or expanding components.
///
/// Glyphs and layers retain document order, kerning uses its sorted key order,
/// and component traversal is iterative: even very deep cycles cannot recurse
/// or cause exponential outline expansion. Only the first `limit.min(2000)`
/// diagnostics are retained, while `total` counts every detected issue.
pub fn scan(font: &Font, limit: usize) -> Report {
    let mut out = Collector {
        report: Report {
            issues: Vec::new(),
            total: 0,
            truncated: false,
            has_errors: false,
        },
        limit: limit.min(MAX_ISSUES),
        component_limits_only: false,
    };
    geometry::metrics(font, &mut out);
    let mut unicode_owners = BTreeMap::new();
    for (name, glyph) in &font.glyphs {
        let at = Location::glyph(name);
        for unicode in &glyph.unicodes {
            if char::from_u32(*unicode).is_none() {
                out.error(
                    "invalid-unicode",
                    format!("U+{unicode:04X} is not a Unicode scalar value."),
                    &at,
                );
            }
            if let Some(owner) = unicode_owners.get(unicode) {
                out.error(
                    "duplicate-unicode",
                    format!("U+{unicode:04X} is also assigned to {owner}."),
                    &at,
                );
            } else {
                unicode_owners.insert(*unicode, name);
            }
        }
        for (base, selector) in &glyph.variation_selectors {
            if char::from_u32(*base).is_none()
                || !matches!(selector, 0xfe00..=0xfe0f | 0xe0100..=0xe01ef)
            {
                out.error(
                    "invalid-variation-selector",
                    format!("Invalid Unicode variation sequence U+{base:04X} U+{selector:04X}."),
                    &at,
                );
            }
        }
        for master in &font.masters {
            if glyph.master_layer(&master.id).is_none() {
                out.error(
                    "missing-master-layer",
                    format!("Glyph {name} has no layer for master {}.", master.name),
                    &Location {
                        master: Some(master.id.clone()),
                        ..at.clone()
                    },
                );
            }
        }
        for layer in &glyph.layers {
            geometry::layer(font, name, layer, &mut out);
        }
    }
    geometry::components(font, &mut out);
    kerning::scan(font, &mut out);
    out.report
}

/// Returns the first component cycle or expansion/depth budget violation.
///
/// Uses the scanner's iterative graph analysis without materializing outlines.
/// Call before full font compilation: small acyclic repeated-reference graphs
/// can otherwise cause exponential allocation. Unrelated model diagnostics
/// cannot displace the safety finding from this bounded one-issue result.
pub fn compilation_blocker(font: &Font) -> Option<Issue> {
    let mut out = Collector {
        report: Report {
            issues: Vec::new(),
            total: 0,
            truncated: false,
            has_errors: false,
        },
        limit: 1,
        component_limits_only: true,
    };
    geometry::components(font, &mut out);
    out.report.issues.into_iter().next()
}

fn layer_master<'a>(font: &'a Font, layer: &'a Layer) -> Option<&'a MasterId> {
    match &layer.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => Some(master),
        LayerKind::Backup { of, .. } => Some(of),
        LayerKind::Custom(_) => font.masters.first().map(|master| &master.id),
    }
}
