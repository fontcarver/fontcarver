use fc_model::{MathTable, MathValue};
use std::mem::size_of;

fn value_size(value: &MathValue) -> usize {
    size_of::<MathValue>()
        + value
            .device
            .as_ref()
            .map_or(0, |device| device.deltas.len())
}

pub(super) fn estimated_bytes(math: &MathTable) -> usize {
    let constants = math.constants.as_ref().map_or(0, |c| {
        c.values().into_iter().map(value_size).sum::<usize>() + 64
    });
    size_of::<MathTable>()
        + constants
        + math
            .glyphs
            .iter()
            .map(|(name, info)| {
                let values = [&info.italic_correction, &info.top_accent]
                    .into_iter()
                    .flatten()
                    .map(value_size)
                    .sum::<usize>();
                let kerns = info
                    .kern
                    .corners()
                    .into_iter()
                    .flatten()
                    .map(|kern| {
                        64 + kern
                            .heights
                            .iter()
                            .chain(&kern.values)
                            .map(value_size)
                            .sum::<usize>()
                    })
                    .sum::<usize>();
                let constructions = [&info.vertical, &info.horizontal]
                    .into_iter()
                    .flatten()
                    .map(|c| {
                        64 + c.variants.iter().map(|v| v.glyph.len() + 48).sum::<usize>()
                            + c.assembly.as_ref().map_or(0, |a| {
                                value_size(&a.italic_correction)
                                    + a.parts.iter().map(|p| p.glyph.len() + 80).sum::<usize>()
                            })
                    })
                    .sum::<usize>();
                name.len() + 512 + values + kerns + constructions
            })
            .sum::<usize>()
}
