//! One open document, its stable handles, undo history, and derived caches.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use fc_model::{Font, GlyphId, GlyphName, KernSide, Kerning, MasterId};
use kurbo::BezPath;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ts_rs::TS;

use crate::{
    cache::{DocumentCache, FlattenedPathKey},
    command::{self, Impact},
    fcf,
    session::DocId,
    undo::{self, UndoEntry, UndoStack},
    Change, Command, Error,
};

mod features;
pub use features::PreparedFeatureChange;

/// Token identifying a live coalesced editing gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct EditToken(#[ts(type = "number")] pub u64);

/// Broadcast payload describing a committed document state change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DocumentEvent {
    /// Document that changed.
    pub doc: DocId,
    /// Monotonic document generation after the change.
    #[ts(type = "number")]
    pub generation: u64,
    /// Precise invalidation information.
    pub change: Change,
    /// Label of the next available undo operation.
    pub undo_label: Option<String>,
    /// Label of the next available redo operation.
    pub redo_label: Option<String>,
    /// Whether the current state differs from the last explicit save.
    pub dirty: bool,
}

struct ActiveEdit {
    token: EditToken,
    label: String,
    has_entry: bool,
    /// Structurally shared history before the first mutation, including redo.
    /// Cancellation restores it even if the live edit exceeded the undo budget.
    history_before: Option<UndoStack>,
    /// A script transaction also absorbs plain [`Document::apply`] commands.
    ///
    /// It carries its own deadline: an abandoned run — a reloaded page, a lost
    /// transport — cannot hold history open beyond it, because every later
    /// gesture, apply, undo, redo or save reaps an expired transaction first.
    sticky: Option<Instant>,
}

impl ActiveEdit {
    fn expired(&self, now: Instant) -> bool {
        self.sticky.is_some_and(|deadline| now >= deadline)
    }
}

/// An open editable font and its transient session state.
pub struct Document {
    /// Persistent font data.
    pub font: Font,
    /// Current save path or imported provenance.
    pub path: Option<PathBuf>,
    /// Whether the current state differs from the last explicit save.
    pub dirty: bool,
    /// Monotonic mutation generation.
    pub generation: u64,
    compile_revision: u64,
    doc_id: DocId,
    unicode_index: fc_model::UnicodeIndex,
    name_to_id: HashMap<GlyphName, GlyphId>,
    id_to_name: Vec<Option<GlyphName>>,
    glyph_generations: Vec<u64>,
    next_glyph_id: u32,
    undo: UndoStack,
    active_edit: Option<ActiveEdit>,
    read_only: bool,
    next_edit_token: u64,
    current_state: u64,
    saved_state: u64,
    next_state: u64,
    cache: Mutex<DocumentCache>,
    component_generation: u64,
    dependents_generation: Option<u64>,
    dependents_cache: HashMap<GlyphName, Vec<GlyphName>>,
    events: broadcast::Sender<DocumentEvent>,
}

impl Document {
    pub(crate) fn attached(
        doc_id: DocId,
        mut font: Font,
        path: Option<PathBuf>,
        events: broadcast::Sender<DocumentEvent>,
    ) -> Self {
        // Old or hand-edited native documents can store mismatched counters.
        // Keep the greatest revision so reopening never accepts an older write.
        if let Some(draft) = &mut font.feature_draft {
            font.feature_revision = font.feature_revision.max(draft.revision);
            draft.revision = font.feature_revision;
        }
        let mut document = Self {
            unicode_index: fc_model::UnicodeIndex::new(&font),
            font,
            path,
            dirty: false,
            generation: 0,
            compile_revision: 0,
            doc_id,
            name_to_id: HashMap::new(),
            id_to_name: Vec::new(),
            glyph_generations: Vec::new(),
            next_glyph_id: 0,
            undo: UndoStack::default(),
            active_edit: None,
            read_only: false,
            next_edit_token: 1,
            current_state: 0,
            saved_state: 0,
            next_state: 1,
            cache: Mutex::new(DocumentCache::default()),
            component_generation: 0,
            dependents_generation: None,
            dependents_cache: HashMap::new(),
            events,
        };
        let names: Vec<_> = document.font.glyph_order().cloned().collect();
        for name in names {
            document.allocate_handle(name);
        }
        document
    }

    /// Monotonic revision of effective compilation inputs, excluding drafts.
    pub fn compile_revision(&self) -> u64 {
        self.compile_revision
    }

    /// Returns the stable session handle for a glyph name.
    pub fn glyph_id(&self, name: &str) -> Option<GlyphId> {
        self.name_to_id.get(name).copied()
    }

    /// Resolves a stable session handle to its current glyph name.
    pub fn glyph_name(&self, id: GlyphId) -> Option<&GlyphName> {
        self.id_to_name.get(id.0 as usize)?.as_ref()
    }

    /// Returns the current invalidation generation of a glyph handle.
    ///
    /// Unknown handles report generation zero; removed handles retain their last
    /// invalidation generation so undo and redo can reuse the same handle.
    pub fn glyph_generation(&self, id: GlyphId) -> u64 {
        self.glyph_generations
            .get(id.0 as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Applies one labelled command and records a distinct undo entry.
    ///
    /// A live coalesced edit is finalized before this command is applied. A live
    /// script transaction ([`Document::begin_transaction`]) instead absorbs the
    /// command, so a whole script run remains one undo entry, and the
    /// transaction's own label is kept.
    pub fn apply(&mut self, command: Command, label: impl Into<String>) -> Result<Change, Error> {
        self.check_writable()?;
        // Only a live script transaction survives this; an expired one, and any
        // gesture, is finalized here.
        self.finalize_active_edit();
        if let Some(token) = self.active_edit.as_ref().map(|active| active.token) {
            return self.apply_in_edit(&token, command);
        }
        self.apply_command(command, label.into(), false)
    }

    /// Whether this document rejects every mutation, as an opened reference does.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// Marks the document as an unmodifiable reference, or restores editing.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
        if read_only {
            self.finalize_active_edit();
        }
    }

    fn check_writable(&self) -> Result<(), Error> {
        if self.read_only {
            return Err(Error::InvalidCommand(
                "this document is opened as a read-only reference and cannot be modified".into(),
            ));
        }
        Ok(())
    }

    /// Starts a script transaction whose commands share one undo entry.
    ///
    /// Unlike [`Document::begin_edit`], plain [`Document::apply`] calls join the
    /// transaction, so a script that mixes low-level and high-level operations
    /// still produces a single history entry. A gesture, history navigation or
    /// `lifetime` elapsing closes it, so an abandoned run — a reloaded page, a
    /// lost transport — can never hold history open indefinitely.
    pub fn begin_transaction(
        &mut self,
        label: impl Into<String>,
        lifetime: Duration,
    ) -> Result<EditToken, Error> {
        self.check_writable()?;
        if lifetime.is_zero() {
            return Err(Error::InvalidCommand(
                "a script transaction needs a nonzero lifetime".into(),
            ));
        }
        let token = self.begin_edit(label);
        if let Some(active) = &mut self.active_edit {
            active.sticky = Some(Instant::now() + lifetime);
        }
        Ok(token)
    }

    /// Closes a live script transaction whoever opened it, leaving a gesture
    /// alone. The automation host calls this when a new run supersedes an
    /// abandoned one. Returns whether a transaction was closed.
    pub fn finalize_transaction(&mut self) -> bool {
        if self
            .active_edit
            .as_ref()
            .is_some_and(|active| active.sticky.is_some())
        {
            self.active_edit = None;
            return true;
        }
        false
    }

    /// Closes a script transaction that has outlived its deadline.
    ///
    /// Safe to call at any time: a live transaction and a gesture are both left
    /// alone. Returns whether one was reaped.
    pub fn finalize_expired_transaction(&mut self) -> bool {
        let now = Instant::now();
        if self
            .active_edit
            .as_ref()
            .is_some_and(|active| active.expired(now))
        {
            self.active_edit = None;
            return true;
        }
        false
    }

    /// Finalizes a matching script transaction.
    ///
    /// Returns `None` when this token is not the live transaction — it already
    /// ended, or a gesture, undo, redo or save finalized it — and otherwise
    /// whether the transaction recorded a history entry.
    pub fn end_transaction(&mut self, token: EditToken) -> Option<bool> {
        let recorded = self
            .active_edit
            .as_ref()
            .filter(|active| active.token == token && active.sticky.is_some())
            .map(|active| active.has_entry)?;
        self.active_edit = None;
        Some(recorded)
    }

    /// Starts a coalesced editing gesture.
    ///
    /// Starting a gesture while another is live finalizes the previous gesture:
    /// its changes remain in history as one undo entry and its token becomes
    /// invalid.
    pub fn begin_edit(&mut self, label: impl Into<String>) -> EditToken {
        // An editor gesture supersedes any live edit, including a script
        // transaction: its recorded entry stays valid and its token expires.
        self.active_edit.take();
        let token = EditToken(self.next_edit_token);
        self.next_edit_token = self.next_edit_token.saturating_add(1);
        self.active_edit = Some(ActiveEdit {
            token,
            label: label.into(),
            has_entry: false,
            history_before: None,
            sticky: None,
        });
        token
    }

    /// Applies a command and merges it into the active gesture's undo entry.
    pub fn apply_in_edit(&mut self, token: &EditToken, command: Command) -> Result<Change, Error> {
        self.check_writable()?;
        let Some(active) = &self.active_edit else {
            return Err(Error::InvalidEditToken);
        };
        if active.token != *token {
            return Err(Error::InvalidEditToken);
        }
        let label = active.label.clone();
        let coalesce = active.has_entry;
        let generation = self.generation;
        let history_before = (!coalesce).then(|| self.undo.clone());
        let change = self.apply_command(command, label, coalesce)?;
        if let Some(active) = &mut self.active_edit {
            if active.token == *token && !active.has_entry && self.generation != generation {
                active.history_before = history_before;
                active.has_entry = true;
            }
        }
        Ok(change)
    }

    /// Finalizes a matching coalesced editing gesture.
    pub fn end_edit(&mut self, token: EditToken) {
        if self
            .active_edit
            .as_ref()
            .is_some_and(|active| active.token == token)
        {
            self.active_edit = None;
        }
    }

    /// Cancels only the matching live gesture, restoring its changes atomically.
    ///
    /// A stale token never ends or undoes another gesture. The cancelled entry is
    /// discarded, so redo cannot reintroduce a failed partial repair. Prior
    /// history, including its redo branch, is restored with the original state.
    pub fn cancel_edit(&mut self, token: EditToken) -> Result<Option<Change>, Error> {
        self.check_writable()?;
        let Some(active) = self
            .active_edit
            .as_ref()
            .filter(|active| active.token == token)
        else {
            return Err(Error::InvalidEditToken);
        };
        if !active.has_entry {
            self.active_edit = None;
            return Ok(None);
        }
        let entry = self
            .undo
            .entries()
            .last()
            .cloned()
            .ok_or_else(|| Error::Internal("live gesture lost its undo entry".into()))?;
        let Some(mut active) = self.active_edit.take() else {
            return Err(Error::InvalidEditToken);
        };
        let Some(mut previous) = active.history_before.take() else {
            self.active_edit = Some(active);
            return Err(Error::Internal(
                "live gesture lost its history snapshot".into(),
            ));
        };
        // Install the restored history before emitting the restoration event,
        // so observers see the correct undo/redo labels in that same event.
        std::mem::swap(&mut self.undo, &mut previous);
        if let Err(error) = self.restore_entry(&entry, false) {
            std::mem::swap(&mut self.undo, &mut previous);
            active.history_before = Some(previous);
            self.active_edit = Some(active);
            return Err(error);
        }
        Ok(Some(entry.change))
    }

    /// Restores the preceding undo state, if one is available.
    pub fn undo(&mut self) -> Result<Option<Change>, Error> {
        self.check_writable()?;
        self.finalize_active_edit();
        self.reject_open_transaction()?;
        let Some(entry) = self.undo.take_undo() else {
            return Ok(None);
        };
        if let Err(error) = self.restore_entry(&entry, false) {
            let _ = self.undo.take_redo();
            return Err(error);
        }
        Ok(Some(entry.change))
    }

    /// Reapplies the next redo state, if one is available.
    pub fn redo(&mut self) -> Result<Option<Change>, Error> {
        self.check_writable()?;
        self.finalize_active_edit();
        self.reject_open_transaction()?;
        let Some(entry) = self.undo.take_redo() else {
            return Ok(None);
        };
        if let Err(error) = self.restore_entry(&entry, true) {
            let _ = self.undo.take_undo();
            return Err(error);
        }
        Ok(Some(entry.change))
    }

    /// Returns the label of the operation that would be undone next.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.undo_label()
    }

    /// Returns the label of the operation that would be redone next.
    pub fn redo_label(&self) -> Option<&str> {
        self.undo.redo_label()
    }

    /// Returns the number of retained entries in the undo timeline.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Read-only access to the undo timeline, for the History surface.
    pub fn history(&self) -> &UndoStack {
        &self.undo
    }

    /// Sets the estimated byte budget for undo and redo history.
    ///
    /// If the retained history exceeds `bytes`, the oldest entries are evicted
    /// immediately. The newest entry is always retained so the latest mutation
    /// remains undoable even when its estimate alone exceeds the budget.
    pub fn set_undo_budget(&mut self, bytes: usize) {
        self.undo.set_memory_budget(bytes);
    }

    /// Reverse character map updated by commands and history without rescanning the font.
    pub fn unicode_index(&self) -> &fc_model::UnicodeIndex {
        &self.unicode_index
    }

    /// Resolves and caches a glyph's component-flattened path for one master.
    pub fn flattened_path(&self, id: GlyphId, master: &MasterId) -> Result<Arc<BezPath>, Error> {
        let Some(name) = self.glyph_name(id) else {
            return Err(Error::MissingGlyph(GlyphName::from(format!(
                "glyph-id-{}",
                id.0
            ))));
        };
        let key = FlattenedPathKey {
            glyph: id,
            master: master.clone(),
            generation: self.glyph_generation(id),
        };
        {
            let cache = self.cache.lock().map_err(|_| Error::LockPoisoned)?;
            if let Some(path) = cache.get(&key) {
                return Ok(path);
            }
        }
        let path = Arc::new(fc_geom::flatten_layer(&self.font, name, master)?);
        self.cache
            .lock()
            .map_err(|_| Error::LockPoisoned)?
            .insert(key, Arc::clone(&path));
        Ok(path)
    }

    /// Saves the document as a `.fcf` or `.fcf.gz` file and marks it clean.
    pub fn save(&mut self, path: &Path) -> Result<(), Error> {
        self.check_writable()?;
        let relocated = if crate::image_paths::relocates_images(self.path.as_deref(), path)? {
            let mut font = self.font.clone();
            let mut undo = self.undo.clone();
            crate::image_paths::rebase_image_paths(&mut font, self.path.as_deref(), path)?;
            undo.rebase_image_paths(self.path.as_deref(), path)?;
            Some((font, undo))
        } else {
            None
        };
        fcf::write(
            path,
            relocated.as_ref().map_or(&self.font, |(font, _)| font),
        )?;
        self.finalize_active_edit();
        if let Some((font, undo)) = relocated {
            self.font = font;
            self.undo = undo;
            // Relative-link metadata changed; reject editors captured before Save As.
            self.generation = self.generation.saturating_add(1);
        }
        self.path = Some(path.to_path_buf());
        self.saved_state = self.current_state;
        self.dirty = false;
        self.emit(Change::default());
        Ok(())
    }

    fn finalize_active_edit(&mut self) {
        // A *live* script transaction outlives gesture finalization: only its
        // own end, a new gesture, history navigation or its deadline closes it.
        let now = Instant::now();
        if self
            .active_edit
            .as_ref()
            .is_some_and(|active| active.sticky.is_some() && !active.expired(now))
        {
            return;
        }
        self.active_edit.take();
    }

    /// History navigation inside a live script transaction is rejected, not
    /// silently reinterpreted: the pending entry covers the whole run, so
    /// undoing it from inside the run would discard changes the script has not
    /// finished making. An expired transaction has already been reaped.
    fn reject_open_transaction(&self) -> Result<(), Error> {
        if self
            .active_edit
            .as_ref()
            .is_some_and(|active| active.sticky.is_some())
        {
            return Err(Error::InvalidCommand(
                "a script transaction is open; end the script run before undoing or redoing".into(),
            ));
        }
        Ok(())
    }

    fn apply_command(
        &mut self,
        command: Command,
        label: String,
        coalesce: bool,
    ) -> Result<Change, Error> {
        crate::kerning::validate_generation(&command, self.generation)?;
        let mut impact = command::impact(&self.font, &command);
        if !impact.renames.is_empty() {
            let dependent_names: Vec<_> = {
                let dependents = self.dependents();
                impact
                    .renames
                    .iter()
                    .filter_map(|(from, _)| dependents.get(from))
                    .flatten()
                    .cloned()
                    .collect()
            };
            for name in &dependent_names {
                impact.glyph(name);
            }
        }
        let affected_before = if impact.affects_dependents {
            let dependents = self.dependents();
            affected_names(&impact, Some(dependents), None)
        } else {
            affected_names(&impact, None, None)
        };
        // Dependent hints describe resolved geometry and must participate in undo
        // when a component base changes, even if their own points stay unchanged.
        if impact.affects_dependents {
            for name in &affected_before {
                if self
                    .font
                    .glyph(name)
                    .is_some_and(|g| g.layers.iter().any(|l| !l.hints.is_empty()))
                {
                    impact.glyph(name);
                }
            }
        }
        let inverse = undo::capture(&self.font, &impact);
        let inverse_handles = self.capture_handles(&impact.glyphs);
        let before_state = self.current_state;

        command::apply_to_font(&mut self.font, &command, &inverse)?;
        // Reject edits that would make an enabled filled effect unevaluable.
        // Checking dependents also catches a component edit under a parent mask.
        if impact.affects_dependents {
            let validation = affected_before
                .iter()
                .chain(&impact.glyphs)
                .try_for_each(|name| {
                    let Some(glyph) = self.font.glyph(name) else {
                        return Ok(());
                    };
                    for layer in &glyph.layers {
                        if layer.has_live_geometry() {
                            fc_geom::evaluate_layer(&self.font, name, &layer.id)?;
                        }
                    }
                    Ok::<_, fc_model::Error>(())
                });
            if let Err(error) = validation {
                for (from, to) in impact.renames.iter().rev() {
                    rename_references(&mut self.font, to, from);
                }
                undo::restore(&mut self.font, &inverse);
                return Err(Error::InvalidCommand(format!(
                    "{error}; remove or repair the live corner or layer effect before this source edit"
                )));
            }
        }
        command::invalidate_stale_ps_hints(&mut self.font, &inverse);
        if (command::is_kerning(&command) || command::is_draft(&command))
            && inverse == undo::capture(&self.font, &impact)
        {
            return Ok(Change::default());
        }
        self.apply_handle_effects(&command)?;
        for name in &impact.glyphs {
            self.unicode_index.update(name, self.font.glyph(name));
        }
        if impact.component_graph_changed {
            self.bump_component_generation();
        }

        let forward = undo::capture(&self.font, &impact);
        let forward_handles = self.capture_handles(&impact.glyphs);
        let affected_names = if impact.affects_dependents && impact.component_graph_changed {
            let dependents = self.dependents();
            merge_names(
                affected_before,
                affected_names(&impact, None, Some(dependents)),
            )
        } else {
            affected_before
        };
        let change = self.make_change(&impact, &affected_names, &inverse_handles, &forward_handles);
        self.bump_glyphs(&change.glyphs)?;

        let after_state = self.next_state;
        self.next_state = self.next_state.saturating_add(1);
        self.current_state = after_state;
        self.dirty = self.current_state != self.saved_state;
        let entry = UndoEntry::new(
            label,
            inverse,
            forward,
            inverse_handles,
            forward_handles,
            impact.renames,
            change.clone(),
            impact.component_graph_changed,
            before_state,
            after_state,
        );
        if coalesce {
            self.undo.coalesce(entry);
        } else {
            self.undo.push(entry);
        }
        self.generation = self.generation.saturating_add(1);
        if change.affects_compile() {
            self.compile_revision = self.compile_revision.saturating_add(1);
        }
        self.emit(change.clone());
        Ok(change)
    }

    fn restore_entry(&mut self, entry: &UndoEntry, forward: bool) -> Result<(), Error> {
        let previous_feature_revision = self.font.feature_revision;
        if entry.change.feature_source && previous_feature_revision == u64::MAX {
            return Err(Error::InvalidCommand(
                "feature revision is exhausted; cannot safely restore history".into(),
            ));
        }
        if forward {
            self.apply_history_renames(&entry.renames, true);
            undo::restore(&mut self.font, &entry.forward);
            self.restore_handles(&entry.forward_handles);
            self.current_state = entry.after_state;
        } else {
            self.apply_history_renames(&entry.renames, false);
            undo::restore(&mut self.font, &entry.inverse);
            self.restore_handles(&entry.inverse_handles);
            self.current_state = entry.before_state;
        }
        for (name, _) in entry.inverse_handles.iter().chain(&entry.forward_handles) {
            self.unicode_index.update(name, self.font.glyph(name));
        }
        if entry.change.feature_source {
            self.font.feature_revision = previous_feature_revision.max(self.font.feature_revision);
            command::bump_feature_revision(&mut self.font)?;
        }
        if entry.change.affects_compile() {
            self.compile_revision = self.compile_revision.saturating_add(1);
        }
        if entry.component_graph_changed {
            self.bump_component_generation();
        }
        self.dirty = self.current_state != self.saved_state;
        self.bump_glyphs(&entry.change.glyphs)?;
        self.generation = self.generation.saturating_add(1);
        self.emit(entry.change.clone());
        Ok(())
    }

    fn apply_history_renames(&mut self, renames: &[(GlyphName, GlyphName)], forward: bool) {
        if forward {
            for (from, to) in renames {
                rename_references(&mut self.font, from, to);
            }
        } else {
            for (from, to) in renames.iter().rev() {
                rename_references(&mut self.font, to, from);
            }
        }
    }

    fn apply_handle_effects(&mut self, command: &Command) -> Result<(), Error> {
        match command {
            Command::AddGlyph { glyph } => {
                self.allocate_handle(glyph.name.clone());
            }
            Command::RemoveGlyphs { names } => {
                for name in names {
                    self.remove_handle(name);
                }
            }
            Command::RenameGlyph { from, to } if from != to => {
                let Some(id) = self.name_to_id.remove(from) else {
                    return Err(Error::Internal(format!(
                        "glyph handle for `{from}` disappeared during rename"
                    )));
                };
                self.name_to_id.insert(to.clone(), id);
                if let Some(slot) = self.id_to_name.get_mut(id.0 as usize) {
                    *slot = Some(to.clone());
                }
            }
            Command::DuplicateGlyph { name, .. } => {
                self.allocate_handle(name.clone());
            }
            Command::Batch(commands) => {
                for command in commands {
                    self.apply_handle_effects(command)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn allocate_handle(&mut self, name: GlyphName) -> GlyphId {
        let id = GlyphId(self.next_glyph_id);
        self.next_glyph_id = self.next_glyph_id.saturating_add(1);
        self.name_to_id.insert(name.clone(), id);
        self.id_to_name.push(Some(name));
        self.glyph_generations.push(0);
        id
    }

    fn remove_handle(&mut self, name: &GlyphName) {
        if let Some(id) = self.name_to_id.remove(name) {
            if let Some(slot) = self.id_to_name.get_mut(id.0 as usize) {
                *slot = None;
            }
        }
    }

    fn capture_handles(&self, names: &[GlyphName]) -> Vec<(GlyphName, Option<GlyphId>)> {
        names
            .iter()
            .map(|name| (name.clone(), self.name_to_id.get(name).copied()))
            .collect()
    }

    fn restore_handles(&mut self, snapshots: &[(GlyphName, Option<GlyphId>)]) {
        for (name, _) in snapshots {
            if let Some(id) = self.name_to_id.remove(name) {
                if self.id_to_name.get(id.0 as usize).and_then(Option::as_ref) == Some(name) {
                    self.id_to_name[id.0 as usize] = None;
                }
            }
        }
        for (name, id) in snapshots {
            let Some(id) = id else {
                continue;
            };
            let index = id.0 as usize;
            if self.id_to_name.len() <= index {
                self.id_to_name.resize(index + 1, None);
                self.glyph_generations.resize(index + 1, 0);
            }
            if let Some(displaced) = self.id_to_name[index].take() {
                self.name_to_id.remove(&displaced);
            }
            self.id_to_name[index] = Some(name.clone());
            self.name_to_id.insert(name.clone(), *id);
        }
    }

    fn make_change(
        &self,
        impact: &Impact,
        names: &[GlyphName],
        inverse_handles: &[(GlyphName, Option<GlyphId>)],
        forward_handles: &[(GlyphName, Option<GlyphId>)],
    ) -> Change {
        let mut glyphs = Vec::new();
        for name in names {
            let id = self
                .name_to_id
                .get(name)
                .copied()
                .or_else(|| handle_for(inverse_handles, name))
                .or_else(|| handle_for(forward_handles, name));
            if let Some(id) = id {
                glyphs.push(id);
            }
        }
        glyphs.sort_by_key(|id| id.0);
        glyphs.dedup();
        Change {
            bitmaps: impact.bitmap_strikes || !impact.bitmap_glyphs.is_empty(),
            math: impact.math,
            color: impact.color || impact.svg,
            feature_source: impact.feature_source,
            layout: impact.layout,
            glyphs,
            font_info: impact.font_info,
            masters: impact.design_space || !impact.masters.is_empty(),
            glyph_order: impact.glyph_order,
            metrics: impact.metrics,
            layers: impact.layers,
            guides: impact.guides,
        }
    }

    fn bump_glyphs(&mut self, glyphs: &[GlyphId]) -> Result<(), Error> {
        for id in glyphs {
            if let Some(generation) = self.glyph_generations.get_mut(id.0 as usize) {
                *generation = generation.saturating_add(1);
            }
        }
        self.cache
            .lock()
            .map_err(|_| Error::LockPoisoned)?
            .invalidate(glyphs);
        Ok(())
    }

    fn dependents(&mut self) -> &HashMap<GlyphName, Vec<GlyphName>> {
        let stale = self
            .dependents_generation
            .is_none_or(|generation| generation != self.component_generation);
        if stale {
            self.dependents_cache = fc_model::dependents(&self.font);
            self.dependents_generation = Some(self.component_generation);
        }
        &self.dependents_cache
    }

    fn bump_component_generation(&mut self) {
        self.component_generation = self.component_generation.saturating_add(1);
    }

    fn emit(&self, change: Change) {
        let _ = self.events.send(DocumentEvent {
            doc: self.doc_id,
            generation: self.generation,
            change,
            undo_label: self.undo_label().map(str::to_owned),
            redo_label: self.redo_label().map(str::to_owned),
            dirty: self.dirty,
        });
    }
}

fn handle_for(snapshots: &[(GlyphName, Option<GlyphId>)], name: &GlyphName) -> Option<GlyphId> {
    snapshots
        .iter()
        .find(|(candidate, _)| candidate == name)
        .and_then(|(_, id)| *id)
}

fn affected_names(
    impact: &Impact,
    before: Option<&HashMap<GlyphName, Vec<GlyphName>>>,
    after: Option<&HashMap<GlyphName, Vec<GlyphName>>>,
) -> Vec<GlyphName> {
    let mut affected = HashSet::new();
    let mut queue = VecDeque::new();
    for name in &impact.glyphs {
        if affected.insert(name.clone()) {
            queue.push_back(name.clone());
        }
    }
    while let Some(name) = queue.pop_front() {
        for dependent in before
            .into_iter()
            .chain(after)
            .filter_map(|map| map.get(&name))
            .flatten()
        {
            if affected.insert(dependent.clone()) {
                queue.push_back(dependent.clone());
            }
        }
    }
    let mut affected: Vec<_> = affected.into_iter().collect();
    affected.sort();
    affected
}

fn merge_names(mut left: Vec<GlyphName>, right: Vec<GlyphName>) -> Vec<GlyphName> {
    for name in right {
        if !left.contains(&name) {
            left.push(name);
        }
    }
    left.sort();
    left
}

pub(crate) fn rename_references(font: &mut Font, from: &GlyphName, to: &GlyphName) {
    // Metrics formulas, feature source and named glyph lists are text and
    // metadata rather than model references; they follow a rename here too.
    crate::command::follow_rename(font, from, to);
    for glyph in font.glyphs.values_mut() {
        for construction in glyph
            .layers
            .iter_mut()
            .flat_map(|layer| &mut layer.constructions)
        {
            rename_name(&mut construction.source, from, to);
        }
        for component in glyph.layers.iter_mut().flat_map(|layer| {
            layer.components.iter_mut().chain(
                layer
                    .color_layers
                    .iter_mut()
                    .flatten()
                    .flat_map(|color_layer| color_layer.components.iter_mut()),
            )
        }) {
            rename_name(&mut component.base, from, to);
        }
    }
    for master in &mut font.masters {
        rename_kerning(&mut master.kerning_h, from, to);
        rename_kerning(&mut master.kerning_v, from, to);
    }
    for groups in [
        &mut font.kerning_groups.side1,
        &mut font.kerning_groups.side2,
    ] {
        for members in groups.values_mut() {
            if members.remove(from) {
                members.insert(to.clone());
            }
        }
    }
    for reference in font
        .bitmap_strikes
        .iter_mut()
        .flat_map(|strike| strike.glyphs.values_mut())
        .flat_map(|glyph| glyph.refs.iter_mut())
    {
        rename_name(&mut reference.base, from, to);
    }
    if let Some(cid) = &mut font.cid {
        for name in cid.cid_map.values_mut() {
            rename_name(name, from, to);
        }
    }
}

fn rename_kerning(kerning: &mut Kerning, from: &GlyphName, to: &GlyphName) {
    kerning.0 = std::mem::take(&mut kerning.0)
        .into_iter()
        .map(|((left, right), value)| {
            (
                (
                    rename_kern_side(left, from, to),
                    rename_kern_side(right, from, to),
                ),
                value,
            )
        })
        .collect();
}

fn rename_kern_side(side: KernSide, from: &GlyphName, to: &GlyphName) -> KernSide {
    match side {
        KernSide::Glyph(name) if name == *from => KernSide::Glyph(to.clone()),
        other => other,
    }
}

fn rename_name(name: &mut GlyphName, from: &GlyphName, to: &GlyphName) {
    if name == from {
        *name = to.clone();
    }
}
