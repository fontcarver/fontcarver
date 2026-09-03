use std::{fs, path::PathBuf};

use fw_sfnt::{face_count, face_names, read_bytes, read_path, sniff, Container, ReadOptions};
use sha2::{Digest, Sha256};

#[test]
fn sniffs_supported_container_magics() {
    assert_eq!(sniff(b"\0\x01\0\0rest"), Some(Container::Sfnt));
    assert_eq!(sniff(b"OTTOrest"), Some(Container::Sfnt));
    assert_eq!(sniff(b"truerest"), Some(Container::Sfnt));
    assert_eq!(sniff(b"ttcfrest"), Some(Container::Ttc));
    assert_eq!(sniff(b"wOFFrest"), Some(Container::Woff));
    assert_eq!(sniff(b"wOF2rest"), Some(Container::Woff2));
    assert_eq!(sniff(b"short"), None);
}

#[test]
fn decodes_woff1_and_woff2() {
    let ttf = read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &ReadOptions::default(),
    )
    .expect("read TTF");
    let woff = read_path(
        &fw_fixtures::path("DejaVuSans.woff"),
        &ReadOptions::default(),
    )
    .expect("read WOFF");
    assert_eq!(woff.fonts[0].glyphs.len(), ttf.fonts[0].glyphs.len());
    let woff_source = woff.fonts[0].extra.source.as_ref().expect("WOFF source");
    assert_eq!(woff_source.format, "woff");
    let woff_bytes = fs::read(fw_fixtures::path("DejaVuSans.woff")).expect("read WOFF fixture");
    let decoded = wuff::decompress_woff1(&woff_bytes).expect("decode WOFF fixture");
    assert_eq!(
        woff_source.sha256,
        format!("{:x}", Sha256::digest(&decoded))
    );
    assert_eq!(woff_source.original_bytes.as_deref(), Some(&decoded));

    let woff2 = read_path(
        &fw_fixtures::path("CantarellMin.woff2"),
        &ReadOptions::default(),
    )
    .expect("read WOFF2");
    assert!(!woff2.fonts[0].glyphs.is_empty());
    assert_eq!(
        woff2.fonts[0]
            .extra
            .source
            .as_ref()
            .expect("WOFF2 source")
            .format,
        "woff2"
    );
}

#[test]
fn reads_every_face_of_an_available_ttc() {
    let Some(path) = first_ttc(PathBuf::from("/usr/share/fonts")) else {
        eprintln!("skipping test: no TTC under /usr/share/fonts");
        return;
    };
    let bytes = fs::read(&path).expect("read system TTC");
    let count = face_count(&bytes).expect("TTC face count");
    assert!(count > 0);
    assert_eq!(
        face_names(&bytes).expect("TTC face names").len(),
        count as usize
    );

    let all = read_bytes(&bytes, &ReadOptions::default()).expect("read all TTC faces");
    assert_eq!(all.fonts.len(), count as usize);

    let one = read_bytes(
        &bytes,
        &ReadOptions {
            font_index: Some(count - 1),
            ..ReadOptions::default()
        },
    )
    .expect("read selected TTC face");
    assert_eq!(one.fonts.len(), 1);
}

fn first_ttc(root: PathBuf) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = first_ttc(path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|extension| extension == "ttc") {
            return Some(path);
        }
    }
    None
}
