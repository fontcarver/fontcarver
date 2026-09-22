use read_fonts::{types::Tag, FontRef};
use write_fonts::FontBuilder;

pub fn replace(bytes: &[u8], tag: Tag, replacement: Vec<u8>) -> Vec<u8> {
    let face = FontRef::new(bytes).unwrap();
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if record.tag() != tag {
            builder.add_raw(
                record.tag(),
                face.table_data(record.tag()).unwrap().as_bytes(),
            );
        }
    }
    builder.add_raw(tag, replacement);
    builder.build()
}

pub fn font(glyphs: &[Vec<u8>]) -> Vec<u8> {
    let mut builder = FontBuilder::new();
    let mut head = vec![0; 54];
    head[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5f0f_3cf5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut hhea = vec![0; 36];
    hhea[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
    hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
    hhea[34..36].copy_from_slice(&(glyphs.len() as u16).to_be_bytes());
    let mut maxp = vec![0; 32];
    maxp[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&(glyphs.len() as u16).to_be_bytes());
    maxp[14..16].copy_from_slice(&2u16.to_be_bytes());
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    let mut hmtx = Vec::new();
    for glyph in glyphs {
        loca.extend((glyf.len() as u32).to_be_bytes());
        glyf.extend(glyph);
        hmtx.extend(600u16.to_be_bytes());
        hmtx.extend(0i16.to_be_bytes());
    }
    loca.extend((glyf.len() as u32).to_be_bytes());
    for (tag, bytes) in [
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"maxp", maxp),
        (*b"glyf", glyf),
        (*b"loca", loca),
        (*b"hmtx", hmtx),
    ] {
        builder.add_raw(Tag::new(&tag), bytes);
    }
    builder.build()
}

pub fn simple(points: &[(i16, i16, bool)], program: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(i16::from(!points.is_empty()).to_be_bytes());
    for value in [
        points.iter().map(|p| p.0).min().unwrap_or(0),
        points.iter().map(|p| p.1).min().unwrap_or(0),
        points.iter().map(|p| p.0).max().unwrap_or(0),
        points.iter().map(|p| p.1).max().unwrap_or(0),
    ] {
        bytes.extend(value.to_be_bytes());
    }
    if !points.is_empty() {
        bytes.extend(((points.len() - 1) as u16).to_be_bytes());
    }
    bytes.extend((program.len() as u16).to_be_bytes());
    bytes.extend(program);
    bytes.extend(points.iter().map(|p| u8::from(p.2)));
    for axis in [false, true] {
        let mut previous = 0i16;
        for point in points {
            let value = if axis { point.1 } else { point.0 };
            bytes.extend(value.wrapping_sub(previous).to_be_bytes());
            previous = value;
        }
    }
    bytes
}

pub fn composite(components: &[(u16, u16, i16, i16, &[i16])]) -> Vec<u8> {
    let mut bytes = vec![0; 10];
    bytes[..2].copy_from_slice(&(-1i16).to_be_bytes());
    for (index, &(flags, gid, a, b, transform)) in components.iter().enumerate() {
        let flags = flags
            | 1
            | if index + 1 < components.len() {
                0x20
            } else {
                0
            };
        bytes.extend(flags.to_be_bytes());
        bytes.extend(gid.to_be_bytes());
        bytes.extend(a.to_be_bytes());
        bytes.extend(b.to_be_bytes());
        for value in transform {
            bytes.extend(value.to_be_bytes());
        }
    }
    bytes
}
