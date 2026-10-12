//! Script transactions and read-only reference documents at the document level.

use std::{fs, path::PathBuf, time::Duration};

/// Long enough that no test in this file reaches it by elapsed time.
const LIFETIME: Duration = Duration::from_secs(60);

use fc_doc::{Command, Session};
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::{Rect, Shape};

/// A unique scratch directory under the test runner's temporary directory.
///
/// `fc-doc` has no `tempfile` development dependency, so these two tests manage
/// their own directory and remove it before returning.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "fontcarver-script-transaction-{}-{name}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("scratch directory");
    directory
}

fn font() -> Font {
    let mut font = Font::new_empty("Transactions", 1000);
    for name in ["A", "B"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.width = 600.;
        layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 400., 700.).to_path(0.01));
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

fn set_width(document: &fc_doc::Document, name: &str, width: f64) -> Command {
    Command::SetWidth {
        glyph: name.into(),
        layer: document.font.glyph(name).unwrap().layers[0].id.clone(),
        width,
    }
}

#[test]
fn plain_applies_join_a_live_script_transaction_as_one_entry() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let before = document.font.clone();
            let token = document.begin_transaction("Script run", LIFETIME).unwrap();
            let a = set_width(document, "A", 700.);
            let b = set_width(document, "B", 800.);
            document.apply(a, "ignored label").unwrap();
            document.apply(b, "ignored label").unwrap();
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.undo_label(), Some("Script run"));
            assert_eq!(document.end_transaction(token), Some(true));

            document.undo().unwrap();
            assert_eq!(document.font, before);
            document.redo().unwrap();
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 700.);
            assert_eq!(document.font.glyph("B").unwrap().layers[0].width, 800.);
        })
        .unwrap();
}

#[test]
fn history_navigation_inside_a_transaction_is_rejected() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document.begin_transaction("Script run", LIFETIME).unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            assert!(document.undo().is_err());
            assert!(document.redo().is_err());
            // The rejected navigation left the transaction and the entry intact.
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.end_transaction(token), Some(true));
            document.undo().unwrap();
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 600.);
        })
        .unwrap();
}

#[test]
fn an_editor_gesture_supersedes_a_transaction_without_losing_its_entry() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document.begin_transaction("Script run", LIFETIME).unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            let gesture = document.begin_edit("Drag");
            assert_eq!(document.end_transaction(token), None);
            let command = set_width(document, "B", 900.);
            document.apply_in_edit(&gesture, command).unwrap();
            document.end_edit(gesture);
            assert_eq!(document.undo_len(), 2);
            assert_eq!(document.undo_label(), Some("Drag"));
        })
        .unwrap();
}

#[test]
fn a_saved_document_keeps_its_open_transaction() {
    let directory = scratch("save");
    let path = directory.join("transaction.fcf");
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document.begin_transaction("Script run", LIFETIME).unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            document.save(&path).unwrap();
            let command = set_width(document, "B", 800.);
            document.apply(command, "ignored").unwrap();
            assert_eq!(document.undo_len(), 1, "saving must not split the run");
            assert_eq!(document.end_transaction(token), Some(true));
        })
        .unwrap();
    assert!(path.is_file());
    fs::remove_dir_all(&directory).expect("remove the scratch directory");
}

#[test]
fn a_transaction_without_changes_reports_that_it_recorded_nothing() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document.begin_transaction("Script run", LIFETIME).unwrap();
            assert_eq!(document.end_transaction(token), Some(false));
            assert_eq!(document.undo_len(), 0);
            assert_eq!(document.end_transaction(token), None);
        })
        .unwrap();
}

#[test]
fn a_read_only_document_rejects_every_mutation_and_can_be_restored() {
    let directory = scratch("read-only");
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let command = set_width(document, "A", 700.);
            document.apply(command, "Before").unwrap();
            document.set_read_only(true);
            assert!(document.read_only());
            let command = set_width(document, "A", 800.);
            let error = document.apply(command, "Rejected").unwrap_err();
            assert!(error.to_string().contains("read-only reference"));
            assert!(document.undo().is_err());
            assert!(document.redo().is_err());
            assert!(document.begin_transaction("Script run", LIFETIME).is_err());
            assert!(document.save(&directory.join("copy.fcf")).is_err());
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 700.);

            document.set_read_only(false);
            let command = set_width(document, "A", 900.);
            document.apply(command, "After").unwrap();
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 900.);
        })
        .unwrap();
    assert!(!directory.join("copy.fcf").exists());
    fs::remove_dir_all(&directory).expect("remove the scratch directory");
}

#[test]
fn an_expired_transaction_is_reaped_by_the_next_edit_and_by_history() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document
                .begin_transaction("Abandoned run", Duration::from_millis(1))
                .unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            std::thread::sleep(Duration::from_millis(5));

            // A later edit no longer joins the abandoned entry.
            let command = set_width(document, "B", 800.);
            document.apply(command, "Later edit").unwrap();
            assert_eq!(document.undo_len(), 2);
            assert_eq!(document.undo_label(), Some("Later edit"));
            // History navigation is no longer blocked either.
            document.undo().unwrap();
            assert_eq!(document.font.glyph("B").unwrap().layers[0].width, 600.);
            assert_eq!(document.end_transaction(token), None);
        })
        .unwrap();
}

#[test]
fn history_and_save_reap_an_expired_transaction_before_acting() {
    let directory = scratch("expired");
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            document
                .begin_transaction("Abandoned run", Duration::from_millis(1))
                .unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            std::thread::sleep(Duration::from_millis(5));

            document.undo().expect("undo must not be blocked");
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 600.);
            document.redo().expect("redo must not be blocked");
            document.save(&directory.join("reaped.fcf")).unwrap();
            assert!(!document.finalize_transaction());
        })
        .unwrap();
    fs::remove_dir_all(&directory).expect("remove the scratch directory");
}

#[test]
fn the_host_can_close_a_live_transaction_it_no_longer_owns() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let token = document.begin_transaction("Lost run", LIFETIME).unwrap();
            let command = set_width(document, "A", 700.);
            document.apply(command, "ignored").unwrap();
            assert!(!document.finalize_expired_transaction(), "still live");

            assert!(document.finalize_transaction());
            assert!(!document.finalize_transaction());
            assert_eq!(document.end_transaction(token), None);
            // The entry it recorded is intact and undoable.
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.undo_label(), Some("Lost run"));
            document.undo().unwrap();
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 600.);
        })
        .unwrap();
}

#[test]
fn a_zero_lifetime_transaction_is_rejected() {
    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            assert!(document
                .begin_transaction("Script run", Duration::ZERO)
                .is_err());
            assert!(!document.finalize_transaction());
        })
        .unwrap();
}

#[test]
fn a_read_only_document_rejects_the_prepared_feature_commit() {
    use fc_doc::PreparedFeatureChange;

    let session = Session::new();
    let doc = session.open_font(font(), None);
    session
        .with_doc_mut(doc, |document| {
            let before = document.font.clone();
            document.set_read_only(true);
            let prepared = PreparedFeatureChange::new(
                document.font.features.clone(),
                document.font.feature_settings.clone(),
            );
            let error = document
                .commit_prepared_features(
                    document.generation,
                    document.compile_revision(),
                    prepared,
                    "Activate features",
                )
                .expect_err("a reference must reject a prepared feature commit");
            assert!(error.to_string().contains("read-only reference"));
            assert_eq!(document.font, before);
            assert_eq!(document.undo_len(), 0);
        })
        .unwrap();
}
