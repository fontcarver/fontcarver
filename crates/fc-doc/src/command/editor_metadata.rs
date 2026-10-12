//! Persistent editor annotations and glyph/contour edit protection.

use std::collections::{BTreeSet, HashSet};

use fc_model::{Contour, Font, GlyphName, Layer, LayerId};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

use super::{apply, BitmapCommand, Command};
use crate::Error;

/// Namespaced object inside `Layer.user_data` reserved for editor metadata.
pub const EDITOR_METADATA_KEY: &str = "com.fontcarver.editor";

/// Visual shape of a non-exported layer annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum AnnotationKind {
    /// A text label at the start position.
    Text,
    /// An arrow from the start to the end position.
    Arrow,
    /// An ellipse bounded by the start and end positions.
    Circle,
    /// A rectangle bounded by the start and end positions.
    Rectangle,
    /// A plus-shaped marker bounded by the start and end positions.
    Plus,
    /// A minus-shaped marker bounded by the start and end positions.
    Minus,
}

/// One persistent annotation in y-up font coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct LayerAnnotation {
    /// Stable, nonempty identifier unique within the layer.
    pub id: String,
    /// Shape drawn by the editor, without affecting exported outlines.
    pub kind: AnnotationKind,
    /// Start horizontal coordinate.
    pub x: f64,
    /// Start vertical coordinate.
    pub y: f64,
    /// End horizontal coordinate.
    pub end_x: f64,
    /// End vertical coordinate.
    pub end_y: f64,
    /// Plain text label; empty text is valid for geometric annotations.
    pub text: String,
}

/// Undoable editor metadata mutations independent of exported geometry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum EditorMetadataCommand {
    /// Explicitly lock or unlock a glyph, including an already locked glyph.
    GlyphLock {
        /// Existing glyph.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// New editing lock.
        locked: bool,
    },
    /// Update selected contour locks, assigning persistent IDs when needed.
    PathLocks {
        /// Existing glyph, which must be unlocked.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Existing source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Current contour indexes; stored locks refer to persistent IDs.
        contours: Vec<usize>,
        /// Whether to protect or release the selected contours.
        locked: bool,
    },
    /// Replace the complete annotation list without changing other metadata.
    Annotations {
        /// Existing glyph, which must be unlocked.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Existing source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Validated replacement list.
        annotations: Vec<LayerAnnotation>,
    },
}

pub(super) fn apply(font: &mut Font, command: &EditorMetadataCommand) -> Result<(), Error> {
    match command {
        EditorMetadataCommand::GlyphLock { glyph, locked } => {
            apply::glyph_mut(font, glyph)?.locked = *locked;
        }
        EditorMetadataCommand::PathLocks {
            glyph,
            layer,
            contours,
            locked,
        } => {
            let target = apply::layer_mut(font, glyph, layer)?;
            // Work on a candidate so malformed metadata or indexes cannot leave IDs behind.
            let mut candidate = target.clone();
            let selected: BTreeSet<_> = contours.iter().copied().collect();
            if selected
                .iter()
                .any(|index| *index >= candidate.contours.len())
            {
                return Err(invalid("path lock index is outside the layer"));
            }
            let mut locks = locked_ids(&candidate)?;
            for index in selected {
                let current = candidate.contours[index].id.clone();
                if *locked {
                    let unique = current.as_ref().is_some_and(|id| {
                        !id.is_empty()
                            && candidate
                                .contours
                                .iter()
                                .filter(|c| c.id.as_ref() == Some(id))
                                .count()
                                == 1
                    });
                    let id = if unique {
                        current.unwrap_or_default()
                    } else {
                        let id = fresh_contour_id(&candidate);
                        candidate.contours[index].id = Some(id.clone());
                        id
                    };
                    locks.insert(id);
                } else if let Some(id) = current {
                    locks.remove(&id);
                }
            }
            metadata_mut(&mut candidate)?.insert(
                "locked_paths".into(),
                Value::Array(locks.into_iter().map(Value::String).collect()),
            );
            *target = candidate;
        }
        EditorMetadataCommand::Annotations {
            glyph,
            layer,
            annotations,
        } => {
            validate_annotations(annotations)?;
            let value = serde_json::to_value(annotations)?;
            let target = apply::layer_mut(font, glyph, layer)?;
            metadata_mut(target)?.insert("annotations".into(), value);
        }
    }
    Ok(())
}

fn fresh_contour_id(layer: &Layer) -> String {
    loop {
        let id = LayerId::new().to_string();
        if !layer
            .contours
            .iter()
            .any(|contour| contour.id.as_ref() == Some(&id))
        {
            return id;
        }
    }
}

fn validate_annotations(annotations: &[LayerAnnotation]) -> Result<(), Error> {
    if annotations.len() > 10_000 {
        return Err(invalid("a layer may contain at most 10000 annotations"));
    }
    let mut ids = HashSet::new();
    for annotation in annotations {
        if annotation.id.trim().is_empty()
            || annotation.id.len() > 256
            || !ids.insert(&annotation.id)
        {
            return Err(invalid(
                "annotation IDs must be nonempty, unique and at most 256 bytes",
            ));
        }
        if ![
            annotation.x,
            annotation.y,
            annotation.end_x,
            annotation.end_y,
        ]
        .iter()
        .all(|coordinate| coordinate.is_finite())
        {
            return Err(invalid("annotation coordinates must be finite"));
        }
        if annotation.text.len() > 16_384 {
            return Err(invalid("annotation text exceeds 16384 bytes"));
        }
    }
    Ok(())
}

fn metadata_mut(layer: &mut Layer) -> Result<&mut Map<String, Value>, Error> {
    // Preserve unknown fields and reject incompatible legacy values instead of erasing them.
    if !layer.user_data.is_null() && !layer.user_data.is_object() {
        return Err(invalid(
            "layer user data must be an object to store editor metadata",
        ));
    }
    if let Some(value) = layer.user_data.get(EDITOR_METADATA_KEY) {
        if !value.is_null() && !value.is_object() {
            return Err(invalid("editor metadata must be an object"));
        }
    }
    if layer.user_data.is_null() {
        layer.user_data = Value::Object(Map::new());
    }
    let data = layer
        .user_data
        .as_object_mut()
        .ok_or_else(|| invalid("layer user data must be an object"))?;
    let value = data
        .entry(EDITOR_METADATA_KEY)
        .or_insert_with(|| Value::Object(Map::new()));
    if value.is_null() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .ok_or_else(|| invalid("editor metadata must be an object"))
}

fn locked_ids(layer: &Layer) -> Result<BTreeSet<String>, Error> {
    let Some(value) = layer
        .user_data
        .get(EDITOR_METADATA_KEY)
        .and_then(|value| value.get("locked_paths"))
    else {
        return Ok(BTreeSet::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| invalid("locked paths must be an array of IDs"))?;
    array
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid("locked path IDs must be nonempty strings"))
        })
        .collect()
}

/// Protected contours captured before a command, independently of their array indexes.
pub(super) struct LockedLayer {
    glyph: GlyphName,
    layer: LayerId,
    paths: Vec<Contour>,
    ids: BTreeSet<String>,
    effects: Vec<fc_model::LayerEffect>,
}

/// Check glyph locks and capture only protected paths in directly affected glyphs.
pub(super) fn preflight(font: &Font, command: &Command) -> Result<Vec<LockedLayer>, Error> {
    if matches!(
        command,
        Command::EditorMetadata(EditorMetadataCommand::GlyphLock { .. })
    ) {
        return Ok(vec![]);
    }
    let affected = apply::impact(font, command);
    for name in &affected.glyphs {
        require_unlocked(font, name)?;
    }
    check_bitmap_locks(font, command)?;
    if matches!(
        command,
        Command::EditorMetadata(EditorMetadataCommand::PathLocks { .. })
    ) {
        return Ok(vec![]);
    }
    let mut protected = Vec::new();
    for name in &affected.glyphs {
        let Some(glyph) = font.glyph(name) else {
            continue;
        };
        for layer in &glyph.layers {
            let ids = locked_ids(layer)?;
            if ids.is_empty() {
                continue;
            }
            let paths = layer
                .contours
                .iter()
                .filter(|contour| contour.id.as_ref().is_some_and(|id| ids.contains(id)))
                .cloned()
                .collect::<Vec<_>>();
            if paths.len() != ids.len()
                || paths
                    .iter()
                    .filter_map(|path| path.id.as_ref())
                    .collect::<HashSet<_>>()
                    .len()
                    != ids.len()
            {
                return Err(invalid(
                    "locked contour IDs must identify exactly one path each",
                ));
            }
            let destination = match command {
                Command::RenameGlyph { from, to } if from == name => to.clone(),
                _ => name.clone(),
            };
            protected.push(LockedLayer {
                glyph: destination,
                layer: layer.id.clone(),
                paths,
                ids,
                effects: layer.effects.clone(),
            });
        }
    }
    Ok(protected)
}

fn require_unlocked(font: &Font, glyph: &GlyphName) -> Result<(), Error> {
    if font.glyph(glyph).is_some_and(|glyph| glyph.locked) {
        return Err(invalid(format!(
            "glyph `{glyph}` is locked; unlock it before editing"
        )));
    }
    Ok(())
}

fn check_bitmap_locks(font: &Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::Bitmap(BitmapCommand::SetGlyph { glyph, .. }) => require_unlocked(font, glyph),
        // A batch replaces exactly the images it names and keeps the rest, so
        // only those glyphs need to be unlocked. Its own `apply` reaches the
        // strike arm below the preflight, which is why it is checked here.
        Command::Bitmap(BitmapCommand::SetGlyphs { glyphs, .. }) => {
            for glyph in glyphs.keys() {
                require_unlocked(font, glyph)?;
            }
            Ok(())
        }
        // A whole-strike write also carries the strike's metrics and its format
        // properties, and the property editor sends one for every such change.
        // Only the images it actually adds, removes or alters need an unlocked
        // glyph: a lock protects a glyph's pixels, not the strike it sits in.
        Command::Bitmap(BitmapCommand::SetStrike { strike }) => {
            let current = font
                .bitmap_strikes
                .iter()
                .find(|current| (current.ppem, current.depth) == (strike.ppem, strike.depth));
            for (name, value) in &strike.glyphs {
                if current.is_none_or(|current| current.glyphs.get(name) != Some(value)) {
                    require_unlocked(font, name)?;
                }
            }
            for name in current.iter().flat_map(|current| current.glyphs.keys()) {
                if !strike.glyphs.contains_key(name) {
                    require_unlocked(font, name)?;
                }
            }
            Ok(())
        }
        Command::Bitmap(BitmapCommand::RemoveStrike { ppem, depth }) => {
            for strike in font
                .bitmap_strikes
                .iter()
                .filter(|strike| (strike.ppem, strike.depth) == (*ppem, *depth))
            {
                for glyph in strike.glyphs.keys() {
                    require_unlocked(font, glyph)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Reject mutation, deletion, duplicated IDs, layer removal or discarded lock metadata.
pub(super) fn validate_locks(font: &Font, protected: &[LockedLayer]) -> Result<(), Error> {
    for before in protected {
        let after = font
            .glyph(&before.glyph)
            .and_then(|glyph| glyph.layer(&before.layer))
            .ok_or_else(|| invalid("a layer with locked paths cannot be removed"))?;
        if locked_ids(after)? != before.ids {
            return Err(invalid(
                "path locks must be changed through the lock command",
            ));
        }
        if after.effects != before.effects {
            return Err(invalid(
                "layer effects cannot change while paths are locked",
            ));
        }
        for path in &before.paths {
            let mut matches = after
                .contours
                .iter()
                .filter(|contour| contour.id == path.id);
            if matches.next() != Some(path) || matches.next().is_some() {
                return Err(invalid(
                    "a locked path cannot be edited, removed or duplicated; unlock it first",
                ));
            }
        }
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidCommand(message.into())
}
