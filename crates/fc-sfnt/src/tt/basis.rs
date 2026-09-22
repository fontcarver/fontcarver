use std::collections::HashMap;

use read_fonts::{
    tables::{
        glyf::{Anchor, CompositeGlyphFlags, Glyf, Glyph},
        loca::Loca,
    },
    types::{GlyphId, Tag},
    FontRef, TableProvider,
};
use sha2::{Digest, Sha256};

use super::{invalid, span};
use crate::Error;

pub(super) struct Source<'a> {
    face: FontRef<'a>,
    glyf: Glyf<'a>,
    loca: Loca<'a>,
    count: u16,
    global: Vec<u8>,
    hashes: HashMap<u16, Identity>,
    budget: usize,
}

#[derive(Clone, Copy)]
struct Identity {
    hash: [u8; 32],
    points: usize,
    contours: usize,
}

impl<'a> Source<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > 256 * 1024 * 1024 {
            return Err(invalid("font size limit exceeded"));
        }
        let face = FontRef::new(bytes).map_err(invalid)?;
        if face.table_data(Tag::new(b"fvar")).is_some() {
            return Err(invalid(
                "instruction authoring currently requires a static font",
            ));
        }
        let head = face.head().map_err(invalid)?;
        if head.units_per_em() == 0 || !matches!(head.index_to_loc_format(), 0 | 1) {
            return Err(invalid("invalid head values"));
        }
        let count = face.maxp().map_err(invalid)?.num_glyphs();
        let glyf = face.glyf().map_err(invalid)?;
        let loca = face
            .loca(head.index_to_loc_format() == 1)
            .map_err(invalid)?;
        if loca.len() < usize::from(count)
            || !loca.all_offsets_are_ascending()
            || loca
                .get_raw(usize::from(count))
                .is_none_or(|end| end as usize > glyf.offset_data().len())
        {
            return Err(invalid("invalid loca offsets"));
        }
        let mut global = head.units_per_em().to_be_bytes().to_vec();
        // These metrics can supply vertical phantom defaults when vmtx is absent.
        let hhea = face.hhea().map_err(invalid)?;
        for value in [
            hhea.ascender().to_i16(),
            hhea.descender().to_i16(),
            hhea.line_gap().to_i16(),
        ] {
            global.extend(value.to_be_bytes());
        }
        if let Some(os2) = face.table_data(Tag::new(b"OS/2")) {
            let bytes = os2.as_bytes();
            global.extend_from_slice(
                bytes
                    .get(62..64)
                    .ok_or_else(|| invalid("truncated OS/2 metrics"))?,
            );
            global.extend_from_slice(
                bytes
                    .get(68..78)
                    .ok_or_else(|| invalid("truncated OS/2 metrics"))?,
            );
        }
        Ok(Self {
            face,
            glyf,
            loca,
            count,
            global,
            hashes: HashMap::new(),
            budget: 8_000_000,
        })
    }

    pub(super) fn glyph(&self, id: u16) -> Result<Option<Glyph<'a>>, Error> {
        let raw = self.raw_glyph(id)?;
        span::locate(raw)?;
        if raw.len() == 10 && span::word(raw, 0)? == 0 {
            return Ok(None);
        }
        self.loca
            .get_glyf(GlyphId::new(u32::from(id)), &self.glyf)
            .map_err(invalid)
    }

    pub(super) fn bounds(&self, id: u16) -> Result<[i16; 4], Error> {
        let raw = self.raw_glyph(id)?;
        if raw.is_empty() {
            return Ok([0; 4]);
        }
        Ok([
            span::word(raw, 2)? as i16,
            span::word(raw, 4)? as i16,
            span::word(raw, 6)? as i16,
            span::word(raw, 8)? as i16,
        ])
    }

    pub(super) fn raw_glyph(&self, id: u16) -> Result<&'a [u8], Error> {
        if id >= self.count {
            return Err(invalid("glyph index outside maxp"));
        }
        let start = self
            .loca
            .get_raw(usize::from(id))
            .ok_or_else(|| invalid("missing loca entry"))? as usize;
        let end = self
            .loca
            .get_raw(usize::from(id) + 1)
            .ok_or_else(|| invalid("missing loca entry"))? as usize;
        self.glyf
            .offset_data()
            .as_bytes()
            .get(start..end)
            .ok_or_else(|| invalid("invalid glyph range"))
    }

    pub(super) fn metric(&self, id: u16, vertical: bool) -> Result<Option<(u16, i16)>, Error> {
        if id >= self.count {
            return Err(invalid("metric glyph index outside maxp"));
        }
        let tag = Tag::new(if vertical { b"vmtx" } else { b"hmtx" });
        let Some(table) = self.face.table_data(tag) else {
            return if vertical {
                Ok(None)
            } else {
                Err(invalid("missing hmtx"))
            };
        };
        let count = usize::from(if vertical {
            self.face
                .vhea()
                .map_err(invalid)?
                .number_of_long_ver_metrics()
        } else {
            self.face.hhea().map_err(invalid)?.number_of_h_metrics()
        });
        if count == 0 || count > usize::from(self.count) {
            return Err(invalid("invalid long metric count"));
        }
        let id = usize::from(id);
        let bytes = table.as_bytes();
        let value = |offset| -> Result<[u8; 2], Error> {
            bytes
                .get(offset..offset + 2)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| invalid("truncated glyph metrics"))
        };
        let advance = u16::from_be_bytes(value(id.min(count - 1) * 4)?);
        let offset = if id < count {
            id * 4 + 2
        } else {
            count * 4 + (id - count) * 2
        };
        Ok(Some((advance, i16::from_be_bytes(value(offset)?))))
    }

    pub(super) fn fingerprint(&mut self, id: u16) -> Result<String, Error> {
        let digest = self.hash(id, &mut Vec::new())?.hash;
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!("tt-points-v1:{hex}"))
    }

    fn hash(&mut self, id: u16, active: &mut Vec<u16>) -> Result<Identity, Error> {
        if let Some(hash) = self.hashes.get(&id) {
            return Ok(*hash);
        }
        if active.len() >= 64 || active.contains(&id) {
            return Err(invalid("component cycle or depth limit"));
        }
        active.push(id);
        let mut hash = Sha256::new();
        let mut points = 0;
        let mut contours = 0;
        hash.update(b"fontcarver-tt-points-v1");
        hash.update((self.global.len() as u32).to_be_bytes());
        hash.update(&self.global);
        for vertical in [false, true] {
            if let Some((advance, bearing)) = self.metric(id, vertical)? {
                hash.update([1]);
                hash.update(advance.to_be_bytes());
                hash.update(bearing.to_be_bytes());
            } else {
                hash.update([0]);
            }
        }
        for coordinate in self.bounds(id)? {
            hash.update(coordinate.to_be_bytes());
        }
        match self.glyph(id)? {
            Some(Glyph::Simple(simple)) => {
                if simple.num_points() > 65535 {
                    return Err(invalid("point count exceeds maxp capacity"));
                }
                if simple
                    .end_pts_of_contours()
                    .windows(2)
                    .any(|ends| ends[0].get() >= ends[1].get())
                {
                    return Err(invalid("contour endpoints are not increasing"));
                }
                hash.update([0]);
                hash.update(simple.number_of_contours().to_be_bytes());
                for end in simple.end_pts_of_contours() {
                    hash.update(end.get().to_be_bytes());
                }
                let mut count = 0;
                for point in simple.points() {
                    self.spend(1)?;
                    hash.update(point.x.to_be_bytes());
                    hash.update(point.y.to_be_bytes());
                    hash.update([u8::from(point.on_curve)]);
                    count += 1;
                }
                if count != simple.num_points() {
                    return Err(invalid("truncated simple point stream"));
                }
                points = count;
                contours = simple.end_pts_of_contours().len();
            }
            Some(Glyph::Composite(composite)) => {
                hash.update([1]);
                let mut last = None;
                for component in composite.components() {
                    self.spend(1)?;
                    last = Some(component.flags);
                    let metric_flags = CompositeGlyphFlags::ROUND_XY_TO_GRID
                        | CompositeGlyphFlags::USE_MY_METRICS
                        | CompositeGlyphFlags::SCALED_COMPONENT_OFFSET
                        | CompositeGlyphFlags::UNSCALED_COMPONENT_OFFSET;
                    let both_offsets = CompositeGlyphFlags::SCALED_COMPONENT_OFFSET
                        | CompositeGlyphFlags::UNSCALED_COMPONENT_OFFSET;
                    if component.flags.contains(both_offsets) {
                        return Err(invalid("contradictory component offset flags"));
                    }
                    let flags = (component.flags & metric_flags).bits();
                    hash.update(flags.to_be_bytes());
                    match component.anchor {
                        Anchor::Offset { x, y } => {
                            hash.update([0]);
                            hash.update(x.to_be_bytes());
                            hash.update(y.to_be_bytes());
                        }
                        Anchor::Point { base, component } => {
                            hash.update([1]);
                            hash.update(base.to_be_bytes());
                            hash.update(component.to_be_bytes());
                        }
                    }
                    let t = component.transform;
                    for value in [t.xx, t.xy, t.yx, t.yy] {
                        hash.update(value.to_bits().to_be_bytes());
                    }
                    let child = self.hash(component.glyph.to_u16(), active)?;
                    if let Anchor::Point { base, component } = component.anchor {
                        if usize::from(base) >= points + 4
                            || usize::from(component) >= child.points + 4
                        {
                            return Err(invalid(
                                "component point anchor outside outline and phantom points",
                            ));
                        }
                    }
                    points += child.points;
                    contours += child.contours;
                    if points > 65535 || contours > 65535 {
                        return Err(invalid("expanded component size exceeds maxp capacity"));
                    }
                    hash.update(child.hash);
                }
                if last.is_none_or(|flags| flags.contains(CompositeGlyphFlags::MORE_COMPONENTS)) {
                    return Err(invalid("truncated component stream"));
                }
            }
            None => {
                hash.update([0]);
                hash.update(0i16.to_be_bytes());
            }
        }
        active.pop();
        let result = Identity {
            hash: hash.finalize().into(),
            points,
            contours,
        };
        self.hashes.insert(id, result);
        Ok(result)
    }

    fn spend(&mut self, amount: usize) -> Result<(), Error> {
        self.budget = self
            .budget
            .checked_sub(amount)
            .ok_or_else(|| invalid("point identity work limit exceeded"))?;
        Ok(())
    }
}
