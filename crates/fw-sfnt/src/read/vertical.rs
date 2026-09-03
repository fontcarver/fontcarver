use std::collections::HashMap;

use fw_model::Layer;
use read_fonts::{tables::vmtx::Vmtx, FontRef, TableProvider};

use crate::Error;

pub(crate) struct VerticalMetrics<'a> {
    vmtx: Option<Vmtx<'a>>,
    default_origin: Option<i16>,
    origins: HashMap<u16, i16>,
}

impl<'a> VerticalMetrics<'a> {
    pub(crate) fn new(face: &FontRef<'a>) -> Result<Self, Error> {
        let vmtx = optional_table(face, read_fonts::types::Tag::new(b"vmtx"), || face.vmtx())
            .map_err(|error| Error::parse("vmtx", error))?;
        let vorg = optional_table(face, read_fonts::types::Tag::new(b"VORG"), || face.vorg())
            .map_err(|error| Error::parse("VORG", error))?;
        let default_origin = vorg.as_ref().map(|table| table.default_vert_origin_y());
        let origins = vorg
            .map(|table| {
                table
                    .vert_origin_y_metrics()
                    .iter()
                    .map(|metric| (metric.glyph_index().to_u16(), metric.vert_origin_y()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            vmtx,
            default_origin,
            origins,
        })
    }

    pub(crate) fn apply(&self, gid: u16, layer: &mut Layer) {
        let raw_gid = read_fonts::types::GlyphId::new(u32::from(gid));
        layer.vertical_width = self
            .vmtx
            .as_ref()
            .and_then(|metrics| metrics.advance(raw_gid))
            .map(f64::from);
        layer.vertical_origin = self
            .origins
            .get(&gid)
            .copied()
            .or(self.default_origin)
            .map(f64::from);
    }
}

fn optional_table<'a, T>(
    face: &FontRef<'a>,
    tag: read_fonts::types::Tag,
    read: impl FnOnce() -> Result<T, read_fonts::ReadError>,
) -> Result<Option<T>, read_fonts::ReadError> {
    if face.table_data(tag).is_some() {
        read().map(Some)
    } else {
        Ok(None)
    }
}
