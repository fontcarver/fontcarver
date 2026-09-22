use read_fonts::{types::Tag, FontRef, TableProvider};
use skrifa::MetadataProvider;
use write_fonts::FontBuilder;

fn fixture() -> (Vec<u8>, u16) {
    let bytes = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap();
    let face = skrifa::FontRef::new(&bytes).unwrap();
    let gid = face.charmap().map('A').unwrap().to_u32() as u16;
    (bytes, gid)
}

fn replace(bytes: &[u8], tag: Tag, replacement: Vec<u8>) -> Vec<u8> {
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

#[test]
fn compiled_point_identity_includes_metrics_but_excludes_hint_programs() {
    let (bytes, gid) = fixture();
    let basis = fc_sfnt::tt::point_basis(&bytes, gid).unwrap();
    assert!(!basis.points.is_empty());
    assert!(basis.fingerprint.starts_with("tt-points-v1:"));
    assert_eq!(basis.phantom_start as usize, basis.points.len());
    assert!(basis.points.iter().any(|point| !point.on_curve) || basis.points.len() > 5);

    let face = FontRef::new(&bytes).unwrap();
    let mut hmtx = face
        .table_data(Tag::new(b"hmtx"))
        .unwrap()
        .as_bytes()
        .to_vec();
    let offset = usize::from(gid) * 4;
    assert!(usize::from(gid) < face.hhea().unwrap().number_of_h_metrics() as usize);
    hmtx[offset + 1] ^= 1;
    let changed = replace(&bytes, Tag::new(b"hmtx"), hmtx);
    let other = fc_sfnt::tt::point_basis(&changed, gid).unwrap();
    assert_eq!(basis.points, other.points);
    assert_ne!(basis.fingerprint, other.fingerprint);

    let changed = replace(&bytes, Tag::new(b"prep"), vec![0x00]);
    assert_eq!(
        basis.fingerprint,
        fc_sfnt::tt::point_basis(&changed, gid).unwrap().fingerprint
    );
}

#[test]
fn empty_glyph_has_four_phantom_indices_and_invalid_input_is_rejected() {
    let (bytes, _) = fixture();
    let face = skrifa::FontRef::new(&bytes).unwrap();
    let gid = face.charmap().map(' ').unwrap().to_u32() as u16;
    let basis = fc_sfnt::tt::point_basis(&bytes, gid).unwrap();
    assert!(basis.points.is_empty());
    assert_eq!(basis.phantom_start, 0);
    assert!(basis.horizontal_phantoms[1] > basis.horizontal_phantoms[0]);
    assert!(fc_sfnt::tt::point_basis(b"invalid", 0).is_err());
    assert!(fc_sfnt::tt::point_basis(&bytes, u16::MAX).is_err());
}

#[test]
fn real_font_bindings_share_component_work_within_an_interactive_budget() {
    let (bytes, _) = fixture();
    let count = FontRef::new(&bytes).unwrap().maxp().unwrap().num_glyphs();
    let ids: Vec<_> = (0..count).collect();
    let start = std::time::Instant::now();
    let all = fc_sfnt::tt::fingerprints(&bytes, &ids).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(all.len(), usize::from(count));
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "binding {count} glyphs took {elapsed:?}"
    );
    for id in [0, count / 2, count - 1] {
        assert_eq!(
            all[usize::from(id)],
            fc_sfnt::tt::point_basis(&bytes, id).unwrap().fingerprint
        );
    }
    eprintln!("Bound {count} glyphs in {elapsed:?}");
}
