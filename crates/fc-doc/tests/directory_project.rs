mod common;

use std::{fs, path::PathBuf, time::Instant};

use common::{synthetic_font, temp_dir, test_font};
use fc_doc::{fcf, Session};
use serde_json::Value;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(temp_dir("directory-project"))
    }
    fn project(&self) -> PathBuf {
        self.0.join("font.fcfp")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest(path: &std::path::Path) -> Value {
    serde_json::from_slice(&fs::read(path.join("project.json")).unwrap()).unwrap()
}
fn replace_manifest(path: &std::path::Path, value: &Value) {
    fs::write(
        path.join("project.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

#[test]
fn directory_project_roundtrips_and_sessions_detect_it() {
    let scratch = Scratch::new();
    let path = scratch.project();
    let font = test_font();
    fcf::write(&path, &font).unwrap();
    assert!(path.is_dir());
    assert_eq!(fcf::read(&path).unwrap(), font);
    assert_eq!(manifest(&path)["font"]["glyphs"], serde_json::json!({}));
    assert_eq!(manifest(&path)["glyph_files"].as_array().unwrap().len(), 2);
    let session = Session::new();
    let id = session.open_path(&path, None).unwrap();
    session
        .with_doc(id, |doc| {
            assert_eq!(doc.font, font);
            assert!(!doc.dirty);
        })
        .unwrap();
}

#[test]
fn editing_one_glyph_preserves_other_files_and_user_data() {
    let scratch = Scratch::new();
    let path = scratch.project();
    let mut font = test_font();
    fcf::write(&path, &font).unwrap();
    let old = manifest(&path);
    let unchanged = old["glyph_files"][1]["file"].as_str().unwrap();
    let unchanged_bytes = fs::read(path.join("glyphs").join(unchanged)).unwrap();
    fs::create_dir(path.join(".git")).unwrap();
    fs::write(path.join(".git/config"), "user-owned Git data").unwrap();
    fs::write(path.join("glyphs/notes.txt"), "user note").unwrap();
    font.glyphs.get_mut("base").unwrap().note = "one change".into();
    fcf::write(&path, &font).unwrap();
    let new = manifest(&path);
    assert_ne!(old["glyph_files"][0], new["glyph_files"][0]);
    assert_eq!(old["glyph_files"][1], new["glyph_files"][1]);
    assert_eq!(
        fs::read(path.join("glyphs").join(unchanged)).unwrap(),
        unchanged_bytes
    );
    assert_eq!(
        fs::read_to_string(path.join(".git/config")).unwrap(),
        "user-owned Git data"
    );
    assert_eq!(
        fs::read_to_string(path.join("glyphs/notes.txt")).unwrap(),
        "user note"
    );
    assert_eq!(fs::read_dir(path.join("glyphs")).unwrap().count(), 3);
    assert_eq!(fcf::read(&path).unwrap(), font);
    let bytes = fs::read(path.join("project.json")).unwrap();
    fcf::write(&path, &font).unwrap();
    assert_eq!(fs::read(path.join("project.json")).unwrap(), bytes);
}

#[test]
fn corrupted_or_unsafe_references_fail_reopen() {
    for kind in [
        "missing",
        "corrupt",
        "traversal",
        "duplicate",
        "wrong-name",
        "version",
    ] {
        let scratch = Scratch::new();
        let path = scratch.project();
        fcf::write(&path, &test_font()).unwrap();
        let mut index = manifest(&path);
        let glyph_path = path
            .join("glyphs")
            .join(index["glyph_files"][0]["file"].as_str().unwrap());
        match kind {
            "missing" => fs::remove_file(glyph_path).unwrap(),
            "corrupt" => fs::write(glyph_path, "{").unwrap(),
            "traversal" => index["glyph_files"][0]["file"] = "../../foreign.json".into(),
            "duplicate" => {
                let entry = index["glyph_files"][0].clone();
                index["glyph_files"].as_array_mut().unwrap().push(entry);
            }
            "wrong-name" => {
                let mut glyph: Value =
                    serde_json::from_slice(&fs::read(&glyph_path).unwrap()).unwrap();
                glyph["name"] = "foreign".into();
                fs::write(glyph_path, serde_json::to_vec(&glyph).unwrap()).unwrap();
            }
            "version" => index["version"] = 999.into(),
            _ => unreachable!(),
        }
        replace_manifest(&path, &index);
        assert!(fcf::read(&path).is_err(), "{kind}");
    }
}

#[test]
fn interrupted_unreferenced_writes_and_failed_save_preserve_committed_font() {
    let scratch = Scratch::new();
    let path = scratch.project();
    let mut font = test_font();
    fcf::write(&path, &font).unwrap();
    fs::write(path.join(".fontcarver-interrupted.tmp"), "{partial").unwrap();
    fs::write(path.join("glyphs/unreferenced.json"), "{partial").unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
    let expected = font.clone();
    font.masters[0].kerning_h.0.insert(
        (
            fc_model::KernSide::Glyph("base".into()),
            fc_model::KernSide::Glyph("base".into()),
        ),
        f64::NAN,
    );
    assert!(fcf::write(&path, &font).is_err());
    assert_eq!(fcf::read(&path).unwrap(), expected);
}

#[cfg(unix)]
#[test]
fn symlinked_glyph_and_storage_directory_are_rejected() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let path = scratch.project();
    let font = test_font();
    fcf::write(&path, &font).unwrap();
    let index = manifest(&path);
    let glyph_path = path
        .join("glyphs")
        .join(index["glyph_files"][0]["file"].as_str().unwrap());
    let foreign = scratch.0.join("foreign.json");
    fs::rename(&glyph_path, &foreign).unwrap();
    symlink(&foreign, &glyph_path).unwrap();
    assert!(fcf::read(&path).is_err());
    assert!(fcf::write(&path, &font).is_err());
    assert!(foreign.exists());
    fs::remove_file(glyph_path).unwrap();
    fs::rename(path.join("glyphs"), scratch.0.join("foreign-glyphs")).unwrap();
    symlink(scratch.0.join("foreign-glyphs"), path.join("glyphs")).unwrap();
    assert!(fcf::read(&path).is_err());
    assert!(fcf::write(&path, &font).is_err());
}

#[test]
fn moderately_sized_directory_project_reopens_within_budget() {
    let scratch = Scratch::new();
    let path = scratch.project();
    let font = synthetic_font(512);
    fcf::write(&path, &font).unwrap();
    let start = Instant::now();
    assert_eq!(fcf::read(&path).unwrap(), font);
    assert!(start.elapsed().as_secs() < 10);
}

#[test]
fn failed_revision_publication_keeps_the_previous_manifest_readable() {
    use sha1::{Digest, Sha1};
    let scratch = Scratch::new();
    let path = scratch.project();
    let font = test_font();
    fcf::write(&path, &font).unwrap();
    let original_manifest = fs::read(path.join("project.json")).unwrap();
    let mut changed = font.clone();
    changed.glyphs.get_mut("base").unwrap().note = "new first glyph revision".into();
    changed.glyphs.get_mut("composite").unwrap().note = "new second glyph revision".into();
    let mut bytes = serde_json::to_vec_pretty(&changed.glyphs["composite"]).unwrap();
    bytes.push(b'\n');
    let occupied = format!(
        "{:x}-{:x}.json",
        Sha1::digest(b"composite"),
        Sha1::digest(&bytes)
    );
    fs::write(path.join("glyphs").join(occupied), "conflicting revision").unwrap();
    assert!(fcf::write(&path, &changed).is_err());
    assert_eq!(
        fs::read(path.join("project.json")).unwrap(),
        original_manifest
    );
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn manually_edited_glyph_text_can_be_reopened_and_saved() {
    let scratch = Scratch::new();
    let path = scratch.project();
    fcf::write(&path, &test_font()).unwrap();
    let index = manifest(&path);
    let glyph_path = path
        .join("glyphs")
        .join(index["glyph_files"][0]["file"].as_str().unwrap());
    let mut glyph: Value = serde_json::from_slice(&fs::read(&glyph_path).unwrap()).unwrap();
    glyph["note"] = "edited in a text editor".into();
    fs::write(glyph_path, serde_json::to_vec_pretty(&glyph).unwrap()).unwrap();
    let font = fcf::read(&path).unwrap();
    assert_eq!(font.glyphs["base"].note, "edited in a text editor");
    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
}
