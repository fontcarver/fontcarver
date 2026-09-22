use super::{error, MAX_TABLE_BYTES};
use crate::Error;
use fc_model::{
    GlyphName, MathAssembly, MathConstants, MathConstruction, MathDevice, MathGlyphInfo, MathKern,
    MathTable, MathValue,
};
use std::collections::HashMap;
use write_fonts::{
    validate::{Validate, ValidationCtx},
    FontWrite, TableWriter,
};

// Each node is a parent-relative offset origin. Records are written inline in
// their parent, including MathValueRecord and MathKernInfoRecord offsets.
#[derive(Default)]
struct Node {
    bytes: Vec<u8>,
    children: Vec<(usize, Node)>,
}

impl Node {
    fn u16(&mut self, value: u16) {
        self.bytes.extend(value.to_be_bytes());
    }
    fn i16(&mut self, value: i16) {
        self.bytes.extend(value.to_be_bytes());
    }
    fn unsigned(&mut self, value: f64) {
        self.u16(value.round() as u16);
    }
    fn offset(&mut self, child: Option<Node>) {
        if let Some(child) = child {
            self.children.push((self.bytes.len(), child));
        }
        self.u16(0);
    }
    fn value(&mut self, value: &MathValue) {
        self.i16(value.value.round() as i16);
        self.offset(value.device.as_ref().map(device));
    }
}

impl FontWrite for Node {
    fn write_into(&self, writer: &mut TableWriter) {
        let mut start = 0;
        for (at, child) in &self.children {
            writer.write_slice(&self.bytes[start..*at]);
            writer.write_offset(child, 2);
            start = at + 2;
        }
        writer.write_slice(&self.bytes[start..]);
    }
}

impl Validate for Node {
    fn validate_impl(&self, _: &mut ValidationCtx) {
        // This private tree is constructed only after MathTable::validate.
    }
}

fn device(device: &MathDevice) -> Node {
    let (format, bits) = if device.deltas.iter().all(|v| (-2..=1).contains(v)) {
        (1, 2)
    } else if device.deltas.iter().all(|v| (-8..=7).contains(v)) {
        (2, 4)
    } else {
        (3, 8)
    };
    let mut node = Node::default();
    node.u16(device.start_size);
    node.u16((usize::from(device.start_size) + device.deltas.len() - 1) as u16);
    node.u16(format);
    for chunk in device.deltas.chunks(16 / bits) {
        let mut word = 0u16;
        for (i, delta) in chunk.iter().enumerate() {
            word |= ((*delta as u16) & ((1 << bits) - 1)) << (16 - bits - i * bits);
        }
        node.u16(word);
    }
    node
}

fn constants(constants: &MathConstants) -> Node {
    let mut node = Node::default();
    node.i16(constants.script_percent_scale_down);
    node.i16(constants.script_script_percent_scale_down);
    node.unsigned(constants.delimited_sub_formula_min_height);
    node.unsigned(constants.display_operator_min_height);
    for value in constants.values() {
        node.value(value);
    }
    node.i16(constants.radical_degree_bottom_raise_percent);
    node
}

fn coverage(ids: impl IntoIterator<Item = u16>) -> Node {
    let ids: Vec<_> = ids.into_iter().collect();
    let mut node = Node::default();
    node.u16(1);
    node.u16(ids.len() as u16);
    for id in ids {
        node.u16(id);
    }
    node
}

fn values(records: Vec<(u16, &MathValue)>) -> Option<Node> {
    if records.is_empty() {
        return None;
    }
    let mut node = Node::default();
    node.offset(Some(coverage(records.iter().map(|(id, _)| *id))));
    node.u16(records.len() as u16);
    for (_, value) in records {
        node.value(value);
    }
    Some(node)
}

fn kern(kern: &MathKern) -> Node {
    let mut node = Node::default();
    node.u16(kern.heights.len() as u16);
    for value in kern.heights.iter().chain(&kern.values) {
        node.value(value);
    }
    node
}

fn glyph_info(records: &[(u16, &MathGlyphInfo)]) -> Node {
    let mut node = Node::default();
    node.offset(values(
        records
            .iter()
            .filter_map(|(id, info)| info.italic_correction.as_ref().map(|v| (*id, v)))
            .collect(),
    ));
    node.offset(values(
        records
            .iter()
            .filter_map(|(id, info)| info.top_accent.as_ref().map(|v| (*id, v)))
            .collect(),
    ));
    let extended: Vec<_> = records
        .iter()
        .filter(|(_, info)| info.extended_shape)
        .map(|(id, _)| *id)
        .collect();
    node.offset((!extended.is_empty()).then(|| coverage(extended)));
    let records: Vec<_> = records
        .iter()
        .filter(|(_, info)| info.kern.corners().iter().any(|corner| corner.is_some()))
        .collect();
    let kerns = if records.is_empty() {
        None
    } else {
        let mut node = Node::default();
        node.offset(Some(coverage(records.iter().map(|(id, _)| *id))));
        node.u16(records.len() as u16);
        for (_, info) in records {
            for corner in info.kern.corners() {
                node.offset(corner.as_ref().map(kern));
            }
        }
        Some(node)
    };
    node.offset(kerns);
    node
}

struct Writer<'a> {
    ids: HashMap<&'a GlyphName, u16>,
}

impl Writer<'_> {
    fn id(&self, name: &GlyphName) -> Result<u16, Error> {
        self.ids
            .get(name)
            .copied()
            .ok_or_else(|| error(format!("missing exported glyph `{name}`")))
    }
    fn construction(&self, construction: &MathConstruction) -> Result<Node, Error> {
        let mut node = Node::default();
        node.offset(
            construction
                .assembly
                .as_ref()
                .map(|a| self.assembly(a))
                .transpose()?,
        );
        node.u16(construction.variants.len() as u16);
        for variant in &construction.variants {
            node.u16(self.id(&variant.glyph)?);
            node.unsigned(variant.advance);
        }
        Ok(node)
    }
    fn assembly(&self, assembly: &MathAssembly) -> Result<Node, Error> {
        let mut node = Node::default();
        node.value(&assembly.italic_correction);
        node.u16(assembly.parts.len() as u16);
        for part in &assembly.parts {
            node.u16(self.id(&part.glyph)?);
            node.unsigned(part.start_connector);
            node.unsigned(part.end_connector);
            node.unsigned(part.full_advance);
            node.u16(u16::from(part.extender));
        }
        Ok(node)
    }
    fn variants(&self, math: &MathTable, records: &[(u16, &MathGlyphInfo)]) -> Result<Node, Error> {
        let vertical: Vec<_> = records
            .iter()
            .filter_map(|(id, info)| info.vertical.as_ref().map(|v| (*id, v)))
            .collect();
        let horizontal: Vec<_> = records
            .iter()
            .filter_map(|(id, info)| info.horizontal.as_ref().map(|v| (*id, v)))
            .collect();
        let mut node = Node::default();
        node.unsigned(math.min_connector_overlap);
        node.offset((!vertical.is_empty()).then(|| coverage(vertical.iter().map(|(id, _)| *id))));
        node.offset(
            (!horizontal.is_empty()).then(|| coverage(horizontal.iter().map(|(id, _)| *id))),
        );
        node.u16(vertical.len() as u16);
        node.u16(horizontal.len() as u16);
        for (_, construction) in vertical.into_iter().chain(horizontal) {
            node.offset(Some(self.construction(construction)?));
        }
        Ok(node)
    }
}

/// Serialize editable MATH using the actual exported glyph order.
///
/// All numeric values and references are validated before rounding. The
/// write-fonts packer deduplicates subtables and resolves Offset16 constraints.
/// Invalid source or an unrepresentable table returns an error, never truncation.
pub fn write_table(math: &MathTable, glyph_order: &[GlyphName]) -> Result<Vec<u8>, Error> {
    math.validate(glyph_order)
        .map_err(|e| error(e.to_string()))?;
    let writer = Writer {
        ids: glyph_order
            .iter()
            .enumerate()
            .map(|(i, n)| (n, i as u16))
            .collect(),
    };
    let records: Vec<_> = glyph_order
        .iter()
        .enumerate()
        .filter_map(|(i, name)| math.glyphs.get(name).map(|info| (i as u16, info)))
        .collect();
    let mut root = Node::default();
    root.u16(1);
    root.u16(0);
    root.offset(math.constants.as_ref().map(constants));
    root.offset(Some(glyph_info(&records)));
    root.offset(Some(writer.variants(math, &records)?));
    let bytes =
        write_fonts::dump_table(&root).map_err(|e| error(format!("cannot pack table: {e}")))?;
    if bytes.len() > MAX_TABLE_BYTES {
        return Err(error("table exceeds 16 MiB"));
    }
    Ok(bytes)
}
