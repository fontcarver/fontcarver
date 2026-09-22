//! Snapshot-based undo storage with structural sharing.

use std::sync::Arc;

use fc_model::{Font, FontInfo, Glyph, GlyphId, GlyphName, Guide, Master, MasterId};

use crate::{command::Impact, Change};

mod features;
mod kerning;
mod math;
pub use features::FeatureSnapshot;
pub use kerning::HorizontalKerningSnapshot;

/// Default maximum estimated memory retained by one undo stack.
pub const DEFAULT_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// A structurally shared value restored by undo or redo.
#[derive(Clone, Debug, PartialEq)]
pub enum Snapshot {
    /// Editable mathematical typesetting data and raw-table ownership.
    Math(Option<Arc<fc_model::MathTable>>, bool),
    /// CID identity and numeric mappings, including absence.
    Cid(Option<Arc<fc_model::CidInfo>>),
    /// Axes, complete master list and instances, including source additions/removals.
    DesignSpace(Arc<(Vec<fc_model::Axis>, Vec<Master>, Vec<fc_model::Instance>)>),
    /// Complete strike collection for strike management and glyph structure edits.
    BitmapStrikes(Arc<Vec<fc_model::BitmapStrike>>, bool),
    /// One bitmap for bounded pixel-edit undo storage.
    BitmapGlyph(u16, u8, GlyphName, Option<Arc<fc_model::BitmapGlyph>>),
    /// Effective source, draft, policy and conversion metadata.
    Features(Arc<FeatureSnapshot>),
    /// One layer's anchor list, excluding outline data and touched state.
    FeatureAnchors(Arc<fc_model::FeatureAnchorState>),
    /// Horizontal kerning and redundant group membership fields.
    Kerning(Arc<HorizontalKerningSnapshot>),
    /// A glyph value, or absence of that glyph.
    Glyph(GlyphName, Option<Arc<Glyph>>),
    /// Complete font-wide metadata.
    FontInfo(Arc<FontInfo>),
    /// One interpolation master.
    Master(MasterId, Arc<Master>),
    /// Persistent glyph ordering.
    GlyphOrder(Vec<GlyphName>),
    /// Font-global guides.
    FontGuides(Arc<Vec<Guide>>),
}

/// One labelled atomic operation in the undo timeline.
#[derive(Clone, Debug)]
pub struct UndoEntry {
    /// User-facing operation label.
    pub label: String,
    /// Snapshots restoring the state before the operation.
    pub inverse: Vec<Snapshot>,
    /// Snapshots restoring the state after the operation.
    pub forward: Vec<Snapshot>,
    pub(crate) inverse_handles: Vec<(GlyphName, Option<GlyphId>)>,
    pub(crate) forward_handles: Vec<(GlyphName, Option<GlyphId>)>,
    pub(crate) renames: Vec<(GlyphName, GlyphName)>,
    pub(crate) change: Change,
    pub(crate) component_graph_changed: bool,
    pub(crate) before_state: u64,
    pub(crate) after_state: u64,
    estimated_bytes: usize,
}

impl UndoEntry {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        label: String,
        inverse: Vec<Snapshot>,
        forward: Vec<Snapshot>,
        inverse_handles: Vec<(GlyphName, Option<GlyphId>)>,
        forward_handles: Vec<(GlyphName, Option<GlyphId>)>,
        renames: Vec<(GlyphName, GlyphName)>,
        change: Change,
        component_graph_changed: bool,
        before_state: u64,
        after_state: u64,
    ) -> Self {
        let estimated_bytes = estimate_entry(&inverse, &forward);
        Self {
            label,
            inverse,
            forward,
            inverse_handles,
            forward_handles,
            renames,
            change,
            component_graph_changed,
            before_state,
            after_state,
            estimated_bytes,
        }
    }

    fn merge(&mut self, newer: Self) {
        merge_earliest_snapshots(&mut self.inverse, newer.inverse);
        merge_latest_snapshots(&mut self.forward, newer.forward);
        merge_earliest_handles(&mut self.inverse_handles, newer.inverse_handles);
        merge_latest_handles(&mut self.forward_handles, newer.forward_handles);
        self.renames.extend(newer.renames);
        self.change.merge(&newer.change);
        self.component_graph_changed |= newer.component_graph_changed;
        self.after_state = newer.after_state;
        self.estimated_bytes = estimate_entry(&self.inverse, &self.forward);
    }
}

/// Cursor-based undo and redo history bounded by an estimated memory budget.
///
/// Estimation deliberately avoids serialization on the editing hot path. Each
/// glyph snapshot counts `points * 32 + components * 64` bytes, plus a small
/// fixed entry cost; metadata containers use similarly cheap size estimates.
#[derive(Debug)]
pub struct UndoStack {
    entries: Vec<UndoEntry>,
    cursor: usize,
    memory_budget: usize,
    estimated_bytes: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(DEFAULT_MEMORY_BUDGET)
    }
}

impl UndoStack {
    /// Creates an empty stack with a byte budget based on cheap estimates.
    pub fn new(memory_budget: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            memory_budget,
            estimated_bytes: 0,
        }
    }

    /// Returns the number of retained undo entries, including redo entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no history entries are retained.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn set_memory_budget(&mut self, bytes: usize) {
        self.memory_budget = bytes;
        self.enforce_budget();
    }

    /// Returns the label of the next entry that undo would restore.
    pub fn undo_label(&self) -> Option<&str> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.label.as_str())
    }

    /// Returns the label of the next entry that redo would restore.
    pub fn redo_label(&self) -> Option<&str> {
        self.entries
            .get(self.cursor)
            .map(|entry| entry.label.as_str())
    }

    pub(crate) fn push(&mut self, entry: UndoEntry) {
        for discarded in self.entries.drain(self.cursor..) {
            self.estimated_bytes = self
                .estimated_bytes
                .saturating_sub(discarded.estimated_bytes);
        }
        self.estimated_bytes = self.estimated_bytes.saturating_add(entry.estimated_bytes);
        self.entries.push(entry);
        self.cursor = self.entries.len();
        self.enforce_budget();
    }

    pub(crate) fn coalesce(&mut self, entry: UndoEntry) {
        let Some(previous) = self.entries.last_mut() else {
            self.push(entry);
            return;
        };
        self.estimated_bytes = self
            .estimated_bytes
            .saturating_sub(previous.estimated_bytes);
        previous.merge(entry);
        self.estimated_bytes = self
            .estimated_bytes
            .saturating_add(previous.estimated_bytes);
        self.cursor = self.entries.len();
        self.enforce_budget();
    }

    pub(crate) fn take_undo(&mut self) -> Option<UndoEntry> {
        let index = self.cursor.checked_sub(1)?;
        self.cursor = index;
        self.entries.get(index).cloned()
    }

    pub(crate) fn take_redo(&mut self) -> Option<UndoEntry> {
        let entry = self.entries.get(self.cursor)?.clone();
        self.cursor += 1;
        Some(entry)
    }

    fn enforce_budget(&mut self) {
        while self.estimated_bytes > self.memory_budget && self.entries.len() > 1 {
            let removed = self.entries.remove(0);
            self.estimated_bytes = self.estimated_bytes.saturating_sub(removed.estimated_bytes);
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
}

pub(crate) fn capture(font: &Font, impact: &Impact) -> Vec<Snapshot> {
    let mut snapshots = Vec::new();
    if impact.math {
        snapshots.push(Snapshot::Math(
            font.math.clone().map(Arc::new),
            font.math_table_owned,
        ));
    }
    if impact.bitmap_strikes {
        snapshots.push(Snapshot::BitmapStrikes(
            Arc::new(font.bitmap_strikes.clone()),
            font.bitmap_tables_owned,
        ));
    } else {
        for (ppem, depth, name) in &impact.bitmap_glyphs {
            let value = font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (*ppem, *depth))
                .and_then(|s| s.glyphs.get(name))
                .cloned()
                .map(Arc::new);
            snapshots.push(Snapshot::BitmapGlyph(*ppem, *depth, name.clone(), value));
        }
    }
    if impact.feature_source {
        snapshots.push(Snapshot::Features(Arc::new(FeatureSnapshot::capture(font))));
    }
    for (glyph, layer) in &impact.feature_anchors {
        if let Some(value) = font.glyph(glyph).and_then(|value| value.layer(layer)) {
            snapshots.push(Snapshot::FeatureAnchors(Arc::new(
                fc_model::FeatureAnchorState {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    anchors: value.anchors.clone(),
                },
            )));
        }
    }
    if impact.layout || impact.design_space || !impact.masters.is_empty() {
        snapshots.push(Snapshot::Kerning(Arc::new(
            HorizontalKerningSnapshot::capture(font),
        )));
    }
    for name in &impact.glyphs {
        snapshots.push(Snapshot::Glyph(
            name.clone(),
            font.glyph(name).cloned().map(Arc::new),
        ));
    }
    if impact.cid {
        snapshots.push(Snapshot::Cid(font.cid.clone().map(Arc::new)));
    }
    if impact.font_info {
        snapshots.push(Snapshot::FontInfo(Arc::new(font.info.clone())));
    }
    if impact.design_space || !impact.masters.is_empty() {
        snapshots.push(Snapshot::DesignSpace(Arc::new((
            font.axes.clone(),
            font.masters.clone(),
            font.instances.clone(),
        ))));
    }
    if impact.glyph_order {
        snapshots.push(Snapshot::GlyphOrder(font.glyph_order().cloned().collect()));
    }
    if impact.font_guides {
        snapshots.push(Snapshot::FontGuides(Arc::new(font.guides.clone())));
    }
    snapshots
}

pub(crate) fn restore(font: &mut Font, snapshots: &[Snapshot]) {
    for snapshot in snapshots {
        match snapshot {
            Snapshot::Math(math, owned) => {
                font.math = math.as_ref().map(|value| (**value).clone());
                font.math_table_owned = *owned;
            }
            Snapshot::Cid(cid) => font.cid = cid.as_ref().map(|value| (**value).clone()),
            Snapshot::DesignSpace(space) => {
                font.axes = space.0.clone();
                font.masters = space.1.clone();
                font.instances = space.2.clone();
            }
            Snapshot::BitmapStrikes(strikes, owned) => {
                font.bitmap_strikes = (**strikes).clone();
                font.bitmap_tables_owned = *owned;
            }
            Snapshot::BitmapGlyph(ppem, depth, name, value) => {
                if let Some(strike) = font
                    .bitmap_strikes
                    .iter_mut()
                    .find(|s| (s.ppem, s.depth) == (*ppem, *depth))
                {
                    match value {
                        Some(value) => {
                            strike.glyphs.insert(name.clone(), (**value).clone());
                        }
                        None => {
                            strike.glyphs.remove(name);
                        }
                    }
                }
            }
            Snapshot::Features(features) => features.restore(font),
            Snapshot::FeatureAnchors(state) => {
                if let Some(layer) = font.glyph_mut(&state.glyph).and_then(|glyph| {
                    glyph
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == state.layer)
                }) {
                    layer.anchors = state.anchors.clone();
                }
            }
            Snapshot::FontInfo(info) => font.info = (**info).clone(),
            Snapshot::Master(id, master) => {
                if let Some(target) = font
                    .masters
                    .iter_mut()
                    .find(|candidate| candidate.id == *id)
                {
                    *target = (**master).clone();
                }
            }
            Snapshot::FontGuides(guides) => font.guides = (**guides).clone(),
            Snapshot::Glyph(_, _) | Snapshot::GlyphOrder(_) | Snapshot::Kerning(_) => {}
        }
    }
    for snapshot in snapshots {
        if let Snapshot::Glyph(name, None) = snapshot {
            font.glyphs.shift_remove(name);
        }
    }
    for snapshot in snapshots {
        if let Snapshot::Glyph(name, Some(glyph)) = snapshot {
            font.glyphs.insert(name.clone(), (**glyph).clone());
        }
    }
    for snapshot in snapshots {
        if let Snapshot::GlyphOrder(order) = snapshot {
            let mut target = 0;
            for name in order {
                if let Some(current) = font.glyphs.get_index_of(name) {
                    font.glyphs.move_index(current, target);
                    target += 1;
                }
            }
        }
    }
    for snapshot in snapshots {
        if let Snapshot::Kerning(kerning) = snapshot {
            kerning.restore(font);
        }
    }
}

fn merge_earliest_snapshots(existing: &mut Vec<Snapshot>, newer: Vec<Snapshot>) {
    for mut snapshot in newer {
        if merge_bitmap_snapshot(existing, &mut snapshot, false) {
            continue;
        }
        if !existing
            .iter()
            .any(|candidate| same_snapshot_key(candidate, &snapshot))
        {
            existing.push(snapshot);
        }
    }
}

fn merge_latest_snapshots(existing: &mut Vec<Snapshot>, newer: Vec<Snapshot>) {
    for mut snapshot in newer {
        if merge_bitmap_snapshot(existing, &mut snapshot, true) {
            continue;
        }
        if let Some(position) = existing
            .iter()
            .position(|candidate| same_snapshot_key(candidate, &snapshot))
        {
            existing[position] = snapshot;
        } else {
            existing.push(snapshot);
        }
    }
}

// Full-strike and single-image snapshots overlap. Fold their chronology into a
// single full snapshot when necessary so mixed gestures restore the true endpoints.
fn merge_bitmap_snapshot(
    existing: &mut Vec<Snapshot>,
    incoming: &mut Snapshot,
    latest: bool,
) -> bool {
    if !matches!(
        incoming,
        Snapshot::BitmapStrikes(..) | Snapshot::BitmapGlyph(..)
    ) {
        return false;
    }
    let full = existing
        .iter()
        .position(|s| matches!(s, Snapshot::BitmapStrikes(..)));
    if let Some(index) = full {
        if !latest {
            return true;
        }
        if let Snapshot::BitmapGlyph(ppem, depth, name, value) = incoming {
            if let Snapshot::BitmapStrikes(strikes, _) = &mut existing[index] {
                restore_bitmap_glyph(
                    Arc::make_mut(strikes).as_mut_slice(),
                    *ppem,
                    *depth,
                    name,
                    value,
                );
            }
            return true;
        }
    }
    if let Snapshot::BitmapStrikes(strikes, _) = incoming {
        if !latest {
            for old in existing.iter() {
                if let Snapshot::BitmapGlyph(ppem, depth, name, value) = old {
                    restore_bitmap_glyph(
                        Arc::make_mut(strikes).as_mut_slice(),
                        *ppem,
                        *depth,
                        name,
                        value,
                    );
                }
            }
        }
        existing.retain(|s| !matches!(s, Snapshot::BitmapStrikes(..) | Snapshot::BitmapGlyph(..)));
    }
    false
}
fn restore_bitmap_glyph(
    strikes: &mut [fc_model::BitmapStrike],
    ppem: u16,
    depth: u8,
    name: &GlyphName,
    value: &Option<Arc<fc_model::BitmapGlyph>>,
) {
    if let Some(strike) = strikes
        .iter_mut()
        .find(|s| (s.ppem, s.depth) == (ppem, depth))
    {
        match value {
            Some(g) => {
                strike.glyphs.insert(name.clone(), (**g).clone());
            }
            None => {
                strike.glyphs.remove(name);
            }
        }
    }
}

fn same_snapshot_key(left: &Snapshot, right: &Snapshot) -> bool {
    match (left, right) {
        (Snapshot::Math(..), Snapshot::Math(..)) => true,
        (Snapshot::Cid(_), Snapshot::Cid(_)) => true,
        (Snapshot::DesignSpace(_), Snapshot::DesignSpace(_)) => true,
        (Snapshot::BitmapStrikes(..), Snapshot::BitmapStrikes(..)) => true,
        (Snapshot::BitmapGlyph(a, b, c, _), Snapshot::BitmapGlyph(d, e, f, _)) => {
            (a, b, c) == (d, e, f)
        }
        (Snapshot::Glyph(left, _), Snapshot::Glyph(right, _)) => left == right,
        (Snapshot::FeatureAnchors(left), Snapshot::FeatureAnchors(right)) => {
            left.glyph == right.glyph && left.layer == right.layer
        }
        (Snapshot::Features(_), Snapshot::Features(_))
        | (Snapshot::FontInfo(_), Snapshot::FontInfo(_))
        | (Snapshot::Kerning(_), Snapshot::Kerning(_))
        | (Snapshot::GlyphOrder(_), Snapshot::GlyphOrder(_))
        | (Snapshot::FontGuides(_), Snapshot::FontGuides(_)) => true,
        (Snapshot::Master(left, _), Snapshot::Master(right, _)) => left == right,
        _ => false,
    }
}

fn merge_earliest_handles(
    existing: &mut Vec<(GlyphName, Option<GlyphId>)>,
    newer: Vec<(GlyphName, Option<GlyphId>)>,
) {
    for handle in newer {
        if !existing.iter().any(|candidate| candidate.0 == handle.0) {
            existing.push(handle);
        }
    }
}

fn merge_latest_handles(
    existing: &mut Vec<(GlyphName, Option<GlyphId>)>,
    newer: Vec<(GlyphName, Option<GlyphId>)>,
) {
    for handle in newer {
        if let Some(position) = existing
            .iter()
            .position(|candidate| candidate.0 == handle.0)
        {
            existing[position] = handle;
        } else {
            existing.push(handle);
        }
    }
}

fn estimate_entry(inverse: &[Snapshot], forward: &[Snapshot]) -> usize {
    128usize.saturating_add(
        inverse
            .iter()
            .chain(forward)
            .map(estimate_snapshot)
            .sum::<usize>(),
    )
}

fn estimate_snapshot(snapshot: &Snapshot) -> usize {
    match snapshot {
        Snapshot::Math(value, _) => value
            .as_ref()
            .map_or(8, |value| math::estimated_bytes(value)),
        Snapshot::Cid(cid) => cid.as_ref().map_or(8, |info| {
            info.registry.len()
                + info.ordering.len()
                + info
                    .cid_map
                    .values()
                    .map(|name| name.len() + 48)
                    .sum::<usize>()
        }),
        Snapshot::BitmapStrikes(strikes, _) => strikes
            .iter()
            .flat_map(|s| s.glyphs.values())
            .map(|g| g.bitmap.rows.len() + g.refs.len() * 64 + 128)
            .sum(),
        Snapshot::BitmapGlyph(_, _, _, value) => value
            .as_ref()
            .map_or(64, |g| g.bitmap.rows.len() + g.refs.len() * 64 + 128),
        Snapshot::Features(features) => features.estimated_bytes(),
        Snapshot::FeatureAnchors(state) => state.anchors.len().saturating_mul(128),
        Snapshot::Kerning(kerning) => kerning.estimated_bytes(),
        Snapshot::Glyph(_, Some(glyph)) => glyph
            .layers
            .iter()
            .map(|layer| {
                let outline_points = layer
                    .contours
                    .iter()
                    .map(|contour| contour.points.len())
                    .sum::<usize>();
                let color_points = layer
                    .color_layers
                    .iter()
                    .flatten()
                    .flat_map(|color_layer| &color_layer.contours)
                    .map(|contour| contour.points.len())
                    .sum::<usize>();
                let color_components = layer
                    .color_layers
                    .iter()
                    .flatten()
                    .map(|color_layer| color_layer.components.len())
                    .sum::<usize>();
                let points = outline_points.saturating_add(color_points);
                let components = layer.components.len().saturating_add(color_components);
                points
                    .saturating_mul(32)
                    .saturating_add(components.saturating_mul(64))
            })
            .sum(),
        Snapshot::Glyph(name, None) => name.len(),
        Snapshot::FontInfo(_) => std::mem::size_of::<FontInfo>(),
        Snapshot::Master(_, _) => std::mem::size_of::<Master>(),
        Snapshot::DesignSpace(space) => {
            space.0.len() * 128
                + space
                    .1
                    .iter()
                    .map(|m| {
                        std::mem::size_of::<Master>()
                            + (m.kerning_h.0.len() + m.kerning_v.0.len()) * 128
                    })
                    .sum::<usize>()
                + space.2.len() * 128
        }
        Snapshot::GlyphOrder(names) => names.iter().map(|name| name.len()).sum(),
        Snapshot::FontGuides(guides) => guides.len().saturating_mul(std::mem::size_of::<Guide>()),
    }
}
