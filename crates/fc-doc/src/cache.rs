//! Derived outline caches keyed by stable handles and generations.

use std::{collections::HashMap, sync::Arc};

use fc_model::{GlyphId, MasterId};
use kurbo::BezPath;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FlattenedPathKey {
    pub(crate) glyph: GlyphId,
    pub(crate) master: MasterId,
    pub(crate) generation: u64,
}

#[derive(Default)]
pub(crate) struct DocumentCache {
    flattened_paths: HashMap<FlattenedPathKey, Arc<BezPath>>,
}

impl DocumentCache {
    pub(crate) fn get(&self, key: &FlattenedPathKey) -> Option<Arc<BezPath>> {
        self.flattened_paths.get(key).cloned()
    }

    pub(crate) fn insert(&mut self, key: FlattenedPathKey, path: Arc<BezPath>) {
        self.flattened_paths.insert(key, path);
    }

    pub(crate) fn invalidate(&mut self, glyphs: &[GlyphId]) {
        self.flattened_paths
            .retain(|key, _| !glyphs.contains(&key.glyph));
    }
}
