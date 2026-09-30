use fc_sfnt::{collection, ttx};
use read_fonts::{types::Tag, FontRef};

fn fixture() -> Vec<u8> {
    std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap()
}

#[test]
fn collection_preserves_each_face_and_uses_absolute_aligned_offsets() {
    let original = fixture();
    let packed = collection::encode(&[&original, &original]).unwrap();
    assert_eq!(fc_sfnt::face_count(&packed).unwrap(), 2);
    for index in 0..2 {
        let face = FontRef::from_index(&packed, index).unwrap();
        for table in face.table_directory().table_records() {
            assert_eq!(table.offset() % 4, 0);
        }
        let extracted = collection::extract_face(&packed, index).unwrap();
        assert_tables_equal(&original, &extracted);
    }
    assert!(collection::extract_face(&packed, 2).is_err());
    assert!(collection::encode(&[]).is_err());
    assert!(collection::encode(&[&packed]).is_err());
}

#[test]
fn raw_ttx_preserves_tables_and_rejects_unsupported_xml() {
    let original = fixture();
    let xml = ttx::encode(&original, 0).unwrap();
    assert!(xml.contains("<OS_2 raw=\"True\">"));
    assert_tables_equal(&original, &ttx::decode(&xml).unwrap());
    for invalid in [
        "<!DOCTYPE ttFont [<!ENTITY x 'abcd'>]><ttFont>&x;</ttFont>",
        "<ttFont><head><unitsPerEm value='1000'/></head></ttFont>",
        "<ttFont><head src='/etc/passwd'/></ttFont>",
        "<ttFont><head raw='True'><hexdata>xyz</hexdata></head></ttFont>",
        "<ttFont><head raw='True'><hexdata>00</hexdata></head><head raw='True'><hexdata>00</hexdata></head></ttFont>",
    ] {
        assert!(ttx::decode(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn repacking_rejects_bad_ranges_and_invalidated_signatures() {
    let mut invalid = fixture();
    invalid[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(collection::encode(&[&invalid]).is_err());
    assert!(ttx::encode(&invalid, 0).is_err());
    let mut builder = write_fonts::FontBuilder::new();
    builder.add_raw(Tag::new(b"head"), vec![0; 54]);
    builder.add_raw(Tag::new(b"DSIG"), vec![0; 8]);
    let signed = builder.build();
    assert!(collection::encode(&[&signed]).is_err());
    assert!(ttx::encode(&signed, 0).is_err());
    let mut builder = write_fonts::FontBuilder::new();
    builder.add_raw(Tag::new(b"OS_2"), vec![0; 8]);
    assert!(ttx::encode(&builder.build(), 0).is_err());
}

fn assert_tables_equal(a: &[u8], b: &[u8]) {
    let a = FontRef::new(a).unwrap();
    let b = FontRef::new(b).unwrap();
    assert_eq!(
        a.table_directory().num_tables(),
        b.table_directory().num_tables()
    );
    for table in a.table_directory().table_records() {
        let tag = table.tag();
        let mut a = a.table_data(tag).unwrap().as_bytes().to_vec();
        let mut b = b.table_data(tag).unwrap().as_bytes().to_vec();
        if tag == Tag::new(b"head") {
            a[8..12].fill(0);
            b[8..12].fill(0);
        }
        assert_eq!(a, b, "{tag}");
    }
}

#[test]
fn fonttools_independently_reads_raw_ttx_and_collection() {
    let available = std::process::Command::new("python3")
        .args(["-c", "import fontTools"])
        .output();
    if !available.is_ok_and(|output| output.status.success()) {
        eprintln!("skipping optional independent decoder: Python fontTools unavailable");
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let original = fixture();
    std::fs::write(temp.path().join("font.ttf"), &original).unwrap();
    std::fs::write(
        temp.path().join("font.ttx"),
        ttx::encode(&original, 0).unwrap(),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("font.ttc"),
        collection::encode(&[&original, &original]).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            r#"
import sys
from pathlib import Path
from fontTools.ttLib import TTFont, TTCollection
p=Path(sys.argv[1])
source=TTFont(p/'font.ttf')
font=TTFont(recalcBBoxes=False, recalcTimestamp=False)
font.importXML(p/'font.ttx')
assert set(font.keys()) == set(source.keys())
for tag in source.keys():
    if tag == 'GlyphOrder': continue
    assert font.getTableData(tag) == source.getTableData(tag), tag
font.save(p/'roundtrip.ttf')
parsed=TTFont(p/'roundtrip.ttf')
assert parsed['maxp'].numGlyphs == source['maxp'].numGlyphs
collection=TTCollection(p/'font.ttc')
assert len(collection.fonts) == 2
for face in collection.fonts:
    assert face.getBestCmap() == source.getBestCmap()
    assert face['hmtx'].metrics == source['hmtx'].metrics
print('fontTools raw TTX and 2-face TTC independently decoded')
"#,
        ])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
}
