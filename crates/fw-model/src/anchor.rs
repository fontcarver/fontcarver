//! Named attachment anchors and derived anchor roles.

use serde::{Deserialize, Serialize};

/// Role inferred from an anchor's conventional name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AnchorKind {
    /// A mark attachment anchor whose stored name begins with `_`.
    Mark(String),
    /// A base attachment anchor.
    Base(String),
    /// A numbered ligature attachment anchor.
    Ligature {
        /// Unnumbered anchor name.
        name: String,
        /// One-based component index.
        index: u16,
    },
    /// A cursive entry anchor.
    CursiveEntry,
    /// A cursive exit anchor.
    CursiveExit,
    /// A horizontal ligature caret.
    Caret(u16),
    /// A vertical ligature caret.
    VCaret(u16),
}

/// A named attachment point in font-space coordinates.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Anchor {
    /// Conventional anchor name.
    #[serde(default)]
    pub name: String,
    /// Horizontal coordinate.
    #[serde(default)]
    pub x: f64,
    /// Vertical coordinate.
    #[serde(default)]
    pub y: f64,
}

impl Anchor {
    /// Infers the attachment role from the anchor name.
    ///
    /// Mark syntax takes precedence over numbered ligature syntax, so `_top_2` is
    /// `Mark("top_2")`, not a ligature mark.
    pub fn kind(&self) -> AnchorKind {
        if self.name == "entry" {
            return AnchorKind::CursiveEntry;
        }
        if self.name == "exit" {
            return AnchorKind::CursiveExit;
        }
        if let Some(index) = numeric_suffix(&self.name, "caret_") {
            return AnchorKind::Caret(index);
        }
        if let Some(index) = numeric_suffix(&self.name, "vcaret_") {
            return AnchorKind::VCaret(index);
        }
        if let Some(mark_name) = self.name.strip_prefix('_') {
            return AnchorKind::Mark(mark_name.to_owned());
        }
        if let Some((name, index)) = ligature_name(&self.name) {
            return AnchorKind::Ligature { name, index };
        }
        AnchorKind::Base(self.name.clone())
    }
}

fn numeric_suffix(name: &str, prefix: &str) -> Option<u16> {
    name.strip_prefix(prefix)?.parse().ok()
}

fn ligature_name(name: &str) -> Option<(String, u16)> {
    let (base, suffix) = name.rsplit_once('_')?;
    let index = suffix.parse().ok()?;
    Some((base.to_owned(), index))
}
