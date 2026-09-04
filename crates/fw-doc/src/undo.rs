//! Snapshot-based undo storage with structural sharing.

use std::sync::Arc;

use fw_model::{Font, FontInfo, Glyph, GlyphId, GlyphName, Guide, Master, MasterId};

use crate::{command::Impact, Change};

/// Default maximum estimated memory retained by one undo stack.
pub const DEFAULT_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// A structurally shared value restored by undo or redo.
#[derive(Clone, Debug)]
pub enum Snapshot {
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
    for name in &impact.glyphs {
        snapshots.push(Snapshot::Glyph(
            name.clone(),
            font.glyph(name).cloned().map(Arc::new),
        ));
    }
    if impact.font_info {
        snapshots.push(Snapshot::FontInfo(Arc::new(font.info.clone())));
    }
    for master_id in &impact.masters {
        if let Some(master) = font.master(master_id) {
            snapshots.push(Snapshot::Master(
                master_id.clone(),
                Arc::new(master.clone()),
            ));
        }
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
            Snapshot::Glyph(_, _) | Snapshot::GlyphOrder(_) => {}
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
}

fn merge_earliest_snapshots(existing: &mut Vec<Snapshot>, newer: Vec<Snapshot>) {
    for snapshot in newer {
        if !existing
            .iter()
            .any(|candidate| same_snapshot_key(candidate, &snapshot))
        {
            existing.push(snapshot);
        }
    }
}

fn merge_latest_snapshots(existing: &mut Vec<Snapshot>, newer: Vec<Snapshot>) {
    for snapshot in newer {
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

fn same_snapshot_key(left: &Snapshot, right: &Snapshot) -> bool {
    match (left, right) {
        (Snapshot::Glyph(left, _), Snapshot::Glyph(right, _)) => left == right,
        (Snapshot::FontInfo(_), Snapshot::FontInfo(_))
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
        Snapshot::GlyphOrder(names) => names.iter().map(|name| name.len()).sum(),
        Snapshot::FontGuides(guides) => guides.len().saturating_mul(std::mem::size_of::<Guide>()),
    }
}
