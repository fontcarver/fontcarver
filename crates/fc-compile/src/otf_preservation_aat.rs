//! Geometry-dependent AAT data retained during static CFF conversion.

use read_fonts::{
    tables::{
        aat::Lookup,
        kerx::{Subtable, Subtable4Actions, SubtableKind},
    },
    types::Tag,
    FontData, FontRead, FontRef, ReadError, TableProvider,
};

use crate::Error;

/// Keep coordinate AAT positioning, but never retain original contour indices.
/// Glyph-ID stability is checked by the preservation merge before this call.
pub(crate) fn check(sfnt: &[u8]) -> Result<(), Error> {
    let face = checked("OTF source sfnt", FontRef::new(sfnt))?;
    let glyph_count = checked("maxp", face.maxp())?.num_glyphs();
    for (tag, context) in [(*b"bsln", "bsln"), (*b"lcar", "lcar"), (*b"opbd", "opbd")] {
        let Some(data) = face.table_data(Tag::new(&tag)) else {
            continue;
        };
        checked(context, version(data, 0x00010000))?;
        let format = checked(context, data.read_at::<u16>(4))?;
        match (context, format) {
            ("bsln", 0 | 1) => {
                checked(context, span(data, 0, 72))?;
                if checked(context, data.read_at::<u16>(6))? >= 32 {
                    return Err(Error::parse(context, "invalid default baseline"));
                }
                if format == 1 {
                    let lookup = checked(context, tail(data, 72))?;
                    checked(
                        context,
                        visit_lookup(lookup, glyph_count, |value| {
                            if value < 32 {
                                Ok(())
                            } else {
                                Err(ReadError::MalformedData("invalid baseline value"))
                            }
                        }),
                    )?;
                }
            }
            ("bsln", 2 | 3) => {
                checked(context, span(data, 0, 74))?;
                return Err(point_layout(context, &format!("format {format}")));
            }
            ("lcar" | "opbd", 0) => {
                let lookup = checked(context, tail(data, 6))?;
                checked(
                    context,
                    visit_lookup(lookup, glyph_count, |offset| {
                        let offset = usize::from(offset);
                        if context == "lcar" {
                            let count = data.read_at::<u16>(offset)?;
                            span(data, offset + 2, usize::from(count) * 2)
                        } else {
                            span(data, offset, 8)
                        }
                    }),
                )?;
            }
            ("lcar" | "opbd", 1) => return Err(point_layout(context, "format 1")),
            _ => {
                return Err(Error::parse(
                    context,
                    ReadError::InvalidFormat(format.into()),
                ))
            }
        }
    }

    if let Some(data) = face.table_data(Tag::new(b"kerx")) {
        check_kerx(&face, data, glyph_count)?;
    }
    if let Some(data) = face.table_data(Tag::new(b"fmtx")) {
        checked("fmtx", version(data, 0x00020000))?;
        checked("fmtx", span(data, 0, 16))?;
        return Err(point_layout("fmtx", "font metric control points"));
    }
    if let Some(data) = face.table_data(Tag::new(b"JSTF")) {
        checked("JSTF", version(data, 0x00010000))?;
        let scripts = checked("JSTF", data.read_at::<u16>(4))?;
        checked("JSTF", span(data, 6, usize::from(scripts) * 6))?;
        // JstfMax holds independent GPOS lookups. Checking the font's GPOS
        // alone therefore cannot establish that this table is geometry-safe.
        return Err(Error::InvalidSource(
            "static OTF/CFF export cannot preserve JSTF because its embedded positioning \
             lookups have not been checked for outline point references; rebuild the \
             justification layout for CFF, or disable preservation of imported tables \
             before exporting"
                .into(),
        ));
    }
    Ok(())
}

fn check_kerx(face: &FontRef<'_>, data: FontData<'_>, glyph_count: u16) -> Result<(), Error> {
    let kerx = checked("kerx", face.kerx())?;
    if !(2..=4).contains(&kerx.version()) {
        return Err(Error::parse("kerx", "unsupported table version"));
    }
    // Do not use Kerx::subtables(): its iterator can silently stop at truncated
    // data. Bound every declared subtable independently before typed parsing.
    let mut offset = 8usize;
    let count = kerx.n_tables() as usize;
    if count > data.len().saturating_sub(offset) / Subtable::HEADER_LEN {
        return Err(Error::parse("kerx", ReadError::OutOfBounds));
    }
    for _ in 0..count {
        let length = checked("kerx", data.read_at::<u32>(offset))? as usize;
        if length < Subtable::HEADER_LEN {
            return Err(Error::parse(
                "kerx",
                "subtable length is smaller than its header",
            ));
        }
        let bytes = checked("kerx", range(data, offset, length))?;
        let subtable = checked("kerx", Subtable::read(bytes))?;
        let tuple_count = if kerx.version() >= 4 {
            subtable.tuple_count()
        } else {
            0
        };
        let kind = checked(
            "kerx",
            SubtableKind::read_with_args(
                FontData::new(subtable.data()),
                (subtable.coverage(), tuple_count),
            ),
        )?;
        // tupleCount does not affect format 4; version 2/3 ignore it in all
        // formats. Other version-4 vectors require variation instantiation.
        if subtable.is_variable()
            || (kerx.version() >= 4
                && subtable.tuple_count() != 0
                && !matches!(&kind, SubtableKind::Format4(_)))
        {
            return Err(Error::InvalidSource(
                "static OTF/CFF export cannot preserve kerx variation kerning before it is \
                 instantiated; rebuild static kerning, or disable preservation of imported \
                 tables before exporting"
                    .into(),
            ));
        }
        match kind {
            SubtableKind::Format4(subtable) => match subtable.actions {
                Subtable4Actions::ControlPoints(_) => {
                    return Err(point_layout("kerx", "format 4 control-point actions"));
                }
                Subtable4Actions::AnchorPoints(_) => {
                    // ankr entries are coordinates, not original outline points.
                    let anchors = checked("ankr", face.ankr())?;
                    let lookup = checked("ankr", anchors.lookup_table())?;
                    let base = anchors.glyph_data_table_offset() as usize;
                    checked(
                        "ankr",
                        visit_lookup(lookup.lookup.offset_data(), glyph_count, |offset| {
                            let start = base
                                .checked_add(usize::from(offset))
                                .ok_or(ReadError::OutOfBounds)?;
                            let count = anchors.offset_data().read_at::<u32>(start)? as usize;
                            let length = count.checked_mul(4).ok_or(ReadError::OutOfBounds)?;
                            span(anchors.offset_data(), start + 4, length)
                        }),
                    )?;
                }
                Subtable4Actions::ControlPointCoords(_) => {}
            },
            SubtableKind::Format0(subtable) => {
                // Generated array accessors return an empty slice on truncation.
                if subtable.pairs().len() != subtable.n_pairs() as usize {
                    return Err(Error::parse("kerx", ReadError::OutOfBounds));
                }
            }
            SubtableKind::Format1(_) | SubtableKind::Format2(_) | SubtableKind::Format6(_) => {}
        }
        offset = offset
            .checked_add(length)
            .ok_or_else(|| Error::parse("kerx", ReadError::OutOfBounds))?;
    }
    Ok(())
}

/// Traverse the five 16-bit AAT lookup formats used by bsln/lcar/opbd/ankr.
/// Bound arrays explicitly because read-fonts accessors can turn truncated
/// arrays into empty slices. Format 0 ends at numGlyphs, not at table EOF.
fn visit_lookup(
    data: FontData<'_>,
    glyph_count: u16,
    mut visit: impl FnMut(u16) -> Result<(), ReadError>,
) -> Result<(), ReadError> {
    let lookup = Lookup::read(data)?;
    match lookup {
        Lookup::Format0(_) => {
            span(data, 2, usize::from(glyph_count) * 2)?;
            for index in 0..usize::from(glyph_count) {
                visit(data.read_at(2 + index * 2)?)?;
            }
        }
        Lookup::Format2(_) | Lookup::Format4(_) | Lookup::Format6(_) => {
            let format = data.read_at::<u16>(0)?;
            let unit = usize::from(data.read_at::<u16>(2)?);
            let count = usize::from(data.read_at::<u16>(4)?);
            let minimum = if format == 6 { 4 } else { 6 };
            if unit < minimum {
                return Err(ReadError::MalformedData("invalid AAT lookup record size"));
            }
            span(data, 12, count * unit)?;
            for index in 0..count {
                let start = 12 + index * unit;
                let last = data.read_at::<u16>(start)?;
                if last == u16::MAX {
                    continue; // Search sentinel, not a glyph mapping.
                }
                if last >= glyph_count {
                    return Err(ReadError::MalformedData("AAT lookup glyph outside maxp"));
                }
                if format == 6 {
                    visit(data.read_at(start + 2)?)?;
                    continue;
                }
                let first = data.read_at::<u16>(start + 2)?;
                let length = usize::from(
                    last.checked_sub(first)
                        .ok_or(ReadError::MalformedData("invalid AAT lookup segment"))?,
                ) + 1;
                let value = data.read_at::<u16>(start + 4)?;
                if format == 2 {
                    visit(value)?;
                } else {
                    let base = usize::from(value);
                    span(data, base, length * 2)?;
                    for index in 0..length {
                        visit(data.read_at(base + index * 2)?)?;
                    }
                }
            }
        }
        Lookup::Format8(lookup) => {
            let count = usize::from(lookup.glyph_count());
            if usize::from(lookup.first_glyph()) + count > usize::from(glyph_count) {
                return Err(ReadError::MalformedData("AAT lookup glyph outside maxp"));
            }
            span(data, 6, count * 2)?;
            for index in 0..count {
                visit(data.read_at(6 + index * 2)?)?;
            }
        }
        Lookup::Format10(_) => return Err(ReadError::InvalidFormat(10)),
    }
    Ok(())
}

fn version(data: FontData<'_>, expected: u32) -> Result<(), ReadError> {
    if data.read_at::<u32>(0)? == expected {
        Ok(())
    } else {
        Err(ReadError::MalformedData("unsupported table version"))
    }
}

fn range(data: FontData<'_>, offset: usize, length: usize) -> Result<FontData<'_>, ReadError> {
    let end = offset.checked_add(length).ok_or(ReadError::OutOfBounds)?;
    data.slice(offset..end).ok_or(ReadError::OutOfBounds)
}

fn span(data: FontData<'_>, offset: usize, length: usize) -> Result<(), ReadError> {
    range(data, offset, length).map(|_| ())
}

fn tail(data: FontData<'_>, offset: usize) -> Result<FontData<'_>, ReadError> {
    data.split_off(offset).ok_or(ReadError::OutOfBounds)
}

fn checked<T>(context: &'static str, result: Result<T, ReadError>) -> Result<T, Error> {
    result.map_err(|error| Error::parse(context, error))
}

fn point_layout(table: &str, structure: &str) -> Error {
    Error::InvalidSource(format!(
        "static OTF/CFF export cannot preserve {table} {structure} because it references \
         outline point indices that change during CFF conversion; rebuild positioning \
         using coordinates, or disable preservation of imported tables before exporting"
    ))
}
