//! Lossless binary-table TTX interchange compatible with fontTools raw tables.
//!
//! This intentionally supports `<table raw="True"><hexdata>…</hexdata></table>`
//! rather than the table-specific decompiled XML dialect. Unsupported XML is
//! rejected explicitly; no external resources, DTDs or code are evaluated.

use std::{collections::BTreeSet, fmt::Write as _, io::Read as _, path::Path};

use read_fonts::types::Tag;
use write_fonts::FontBuilder;

use crate::{collection, Error, ReadOptions, ReadOutcome};

/// Maximum XML file size, including hexadecimal expansion and whitespace.
pub const MAX_XML_BYTES: usize = 288 * 1024 * 1024;

/// Serializes one complete sfnt/collection face using fontTools raw-table XML.
///
/// Table bytes are retained, apart from the regenerated `head` checksum on later
/// decoding. DSIG signatures and unusual tags without a reversible fontTools XML
/// name are rejected explicitly.
pub fn encode(bytes: &[u8], face_index: u32) -> Result<String, Error> {
    let face = collection::checked_face(bytes, face_index, false)?;
    let version = if face.table_data(Tag::new(b"CFF ")).is_some()
        || face.table_data(Tag::new(b"CFF2")).is_some()
    {
        "OTTO"
    } else {
        r"\x00\x01\x00\x00"
    };
    let mut output =
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ttFont sfntVersion=\"{version}\">\n");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for record in face.table_directory().table_records() {
        let name = tag_to_xml(record.tag())?;
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("TTX", "invalid table range"))?;
        let _ = writeln!(output, "  <{name} raw=\"True\">\n    <hexdata>");
        for chunk in data.as_bytes().chunks(32) {
            output.push_str("      ");
            for byte in chunk {
                output.push(HEX[usize::from(byte >> 4)] as char);
                output.push(HEX[usize::from(byte & 15)] as char);
            }
            output.push('\n');
        }
        let _ = writeln!(output, "    </hexdata>\n  </{name}>");
    }
    output.push_str("</ttFont>\n");
    Ok(output)
}

/// Parses bounded raw-table TTX into a standalone sfnt with rebuilt checksums.
pub fn decode(xml: &str) -> Result<Vec<u8>, Error> {
    if xml.len() > MAX_XML_BYTES {
        return Err(Error::parse("TTX", "XML exceeds 288 MiB"));
    }
    let document = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 32768,
            ..Default::default()
        },
    )
    .map_err(|error| Error::parse("TTX", error))?;
    let root = document.root_element();
    if root.tag_name().name() != "ttFont" || root.tag_name().namespace().is_some() {
        return Err(Error::parse("TTX", "expected ttFont root"));
    }
    check_attributes(root, &["sfntVersion", "ttLibVersion"])?;
    let version = root.attribute("sfntVersion").unwrap_or(r"\x00\x01\x00\x00");
    if ![r"\x00\x01\x00\x00", "OTTO", "true"].contains(&version) {
        return Err(Error::parse("TTX", "unsupported sfntVersion"));
    }
    let mut tags = BTreeSet::new();
    let mut builder = FontBuilder::new();
    let mut total = 12usize;
    for table in root.children() {
        if table.is_text() && !table.text().unwrap_or_default().trim().is_empty() {
            return Err(Error::parse("TTX", "unexpected text in ttFont"));
        }
        if !table.is_element() {
            continue;
        }
        check_attributes(table, &["raw"])?;
        if table.attribute("raw") != Some("True") && table.attribute("raw") != Some("1") {
            return Err(Error::parse(
                "TTX",
                "only raw=True binary tables are supported; decompiled TTX requires fontTools",
            ));
        }
        let tag = xml_to_tag(table.tag_name().name())?;
        if !tags.insert(tag) || tags.len() > collection::MAX_TABLES {
            return Err(Error::parse(
                "TTX",
                "duplicate table or table count exceeds 4095",
            ));
        }
        if tag == Tag::new(b"DSIG") {
            return Err(Error::parse(
                "TTX",
                "DSIG signature would be invalidated by rebuilding",
            ));
        }
        let mut hex = None;
        for child in table.children() {
            if child.is_element() {
                if child.tag_name().name() != "hexdata" || hex.is_some() {
                    return Err(Error::parse(
                        "TTX",
                        "expected exactly one hexdata element per table",
                    ));
                }
                check_attributes(child, &[])?;
                hex = Some(child);
            } else if child.is_text() && !child.text().unwrap_or_default().trim().is_empty() {
                return Err(Error::parse("TTX", "unexpected text in raw table"));
            }
        }
        let hex = hex.ok_or_else(|| Error::parse("TTX", "missing hexdata"))?;
        let mut data = Vec::new();
        let mut high = None;
        for part in hex.children() {
            if part.is_element() {
                return Err(Error::parse(
                    "TTX",
                    "nested hexdata elements are unsupported",
                ));
            }
            if !part.is_text() {
                continue;
            }
            for byte in part.text().unwrap_or_default().bytes() {
                if byte.is_ascii_whitespace() {
                    continue;
                }
                let digit = (byte as char)
                    .to_digit(16)
                    .ok_or_else(|| Error::parse("TTX", "invalid hexadecimal data"))?
                    as u8;
                if let Some(upper) = high.take() {
                    if total + data.len() >= collection::MAX_BYTES {
                        return Err(Error::parse("TTX", "decoded font exceeds 128 MiB"));
                    }
                    data.push(upper * 16 + digit);
                } else {
                    high = Some(digit);
                }
            }
        }
        if high.is_some() {
            return Err(Error::parse("TTX", "odd hexadecimal digit count"));
        }
        if tag == Tag::new(b"head") && data.len() < 54 {
            return Err(Error::parse("TTX", "truncated head table"));
        }
        total += 16 + ((data.len() + 3) & !3);
        if total > collection::MAX_BYTES {
            return Err(Error::parse("TTX", "decoded font exceeds 128 MiB"));
        }
        builder.add_raw(tag, data);
    }
    if tags.is_empty() {
        return Err(Error::parse("TTX", "no binary tables"));
    }
    let cff = tags.contains(&Tag::new(b"CFF ")) || tags.contains(&Tag::new(b"CFF2"));
    if cff != (version == "OTTO") {
        return Err(Error::parse(
            "TTX",
            "sfntVersion disagrees with outline tables",
        ));
    }
    Ok(builder.build())
}

/// Opens raw-table TTX using the standard sfnt model decoder and warnings.
pub fn read_path(path: &Path, options: &ReadOptions) -> Result<ReadOutcome, Error> {
    let file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;
    let mut xml = String::new();
    file.take(MAX_XML_BYTES as u64 + 1)
        .read_to_string(&mut xml)
        .map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
    let bytes = decode(&xml)?;
    let mut outcome = crate::read_bytes(&bytes, options)?;
    for font in &mut outcome.fonts {
        if let Some(source) = &mut font.extra.source {
            source.path = Some(path.to_string_lossy().into_owned());
            source.format = "ttx".into();
        }
    }
    Ok(outcome)
}

fn check_attributes(node: roxmltree::Node<'_, '_>, allowed: &[&str]) -> Result<(), Error> {
    if node.tag_name().namespace().is_some()
        || node.attributes().any(|attribute| {
            attribute.namespace().is_some() || !allowed.contains(&attribute.name())
        })
    {
        return Err(Error::parse(
            "TTX",
            "unsupported attribute, namespace or external reference",
        ));
    }
    Ok(())
}

fn tag_to_xml(tag: Tag) -> Result<String, Error> {
    if tag == Tag::new(b"OS/2") {
        return Ok("OS_2".into());
    }
    if tag == Tag::new(b"OS_2") {
        return Err(Error::parse(
            "TTX",
            "OS_2 tag collides with the OS/2 XML name",
        ));
    }
    let bytes = tag.to_be_bytes();
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| Error::parse("TTX", "non-ASCII table tag"))?
        .trim_end_matches(' ');
    if !text.is_empty()
        && text.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
    {
        return Ok(text.into());
    }
    Err(Error::parse(
        "TTX",
        format!("table tag {tag} has no supported reversible XML name"),
    ))
}

fn xml_to_tag(name: &str) -> Result<Tag, Error> {
    if name == "OS_2" {
        return Ok(Tag::new(b"OS/2"));
    }
    if name.is_empty()
        || name.len() > 4
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
    {
        return Err(Error::parse("TTX", "unsupported table XML name"));
    }
    let mut tag = [b' '; 4];
    tag[..name.len()].copy_from_slice(name.as_bytes());
    Ok(Tag::new(&tag))
}
