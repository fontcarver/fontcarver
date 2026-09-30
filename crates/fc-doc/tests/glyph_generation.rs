//! Database-driven glyph creation and the references a rename must follow.

use std::collections::BTreeMap;

use fc_doc::command::{composed_glyph, ComposedGlyph};
use fc_doc::{Command, Session};
use fc_glyphdata::{parse_overlay, GlyphDataOverlay};
use fc_model::{Anchor, Contour, Font, Glyph, GlyphName, Layer, Master, MasterId, MetricsKeys};
use kurbo::Shape;

/// A font with `A` (anchored) and `acutecomb` (with an `_top` entry anchor).
fn latin_font() -> Font {
    let mut font = Font::new_empty("Composition", 1_000);
    let master = font.default_master().id.clone();
    font.insert_glyph(letter_a(&master, 0.0)).unwrap();
    font.insert_glyph(acutecomb(&master, 0.0)).unwrap();
    font
}

fn letter_a(master: &MasterId, shift: f64) -> Glyph {
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0x0041];
    let mut layer = Layer::new_master(master.clone());
    layer.width = 600.0 + shift;
    layer.contours = Contour::from_bezpath(&kurbo::Rect::new(20.0, 0.0, 580.0, 700.0).to_path(0.1));
    layer.anchors.push(Anchor {
        name: "top".into(),
        x: 300.0 + shift,
        y: 700.0,
    });
    layer.anchors.push(Anchor {
        name: "bottom".into(),
        x: 300.0 + shift,
        y: 0.0,
    });
    glyph.layers.push(layer);
    glyph
}

fn acutecomb(master: &MasterId, shift: f64) -> Glyph {
    let mut glyph = Glyph::new("acutecomb");
    glyph.unicodes = vec![0x0301];
    let mut layer = Layer::new_master(master.clone());
    layer.width = 0.0;
    layer.contours =
        Contour::from_bezpath(&kurbo::Rect::new(-40.0, 620.0, 40.0, 760.0).to_path(0.1));
    // A mark sits on the origin and declares where it attaches.
    layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 0.0 + shift,
        y: 620.0,
    });
    layer.anchors.push(Anchor {
        name: "top".into(),
        x: 0.0 + shift,
        y: 780.0,
    });
    glyph.layers.push(layer);
    glyph
}

fn names(values: &[GlyphName]) -> Vec<&str> {
    values.iter().map(GlyphName::as_ref).collect()
}

fn compose(font: &Font, name: &str, unicodes: Vec<u32>) -> ComposedGlyph {
    composed_glyph(
        font,
        &BTreeMap::new(),
        &GlyphName::from(name),
        unicodes,
        &GlyphDataOverlay::default(),
    )
}

#[test]
fn composition_places_marks_on_matching_base_anchors() {
    let font = latin_font();
    let composed = compose(&font, "Aacute", vec![0x00C1]);
    assert_eq!(names(&composed.components), ["A", "acutecomb"]);
    assert!(composed.unavailable.is_empty());
    let layer = &composed.glyph.layers[0];
    assert_eq!(layer.components.len(), 2);
    assert_eq!(layer.width, 600.0);
    assert_eq!(layer.components[0].base.as_ref(), "A");
    assert_eq!(layer.components[0].transform, kurbo::Affine::IDENTITY);
    assert!(!layer.components[0].auto_align);
    // `_top` at (0, 620) must land on the base `top` at (300, 700).
    assert_eq!(
        layer.components[1].transform.translation(),
        kurbo::Vec2::new(300.0, 80.0)
    );
    assert_eq!(layer.components[1].anchor.as_deref(), Some("top"));
    assert!(layer.components[1].auto_align);
    // The mark's own `top` replaces the base's, so a second mark stacks above.
    let anchors: Vec<_> = layer
        .anchors
        .iter()
        .map(|anchor| (anchor.name.as_str(), anchor.x, anchor.y))
        .collect();
    assert!(anchors.contains(&("top", 300.0, 860.0)), "{anchors:?}");
    assert!(anchors.contains(&("bottom", 300.0, 0.0)), "{anchors:?}");
}

#[test]
fn a_member_that_is_not_a_mark_is_placed_at_the_pen() {
    let mut font = latin_font();
    // A second base: no entry anchor, so it follows the first one's advance.
    font.insert_glyph({
        let master = font.default_master().id.clone();
        let mut glyph = Glyph::new("f");
        glyph.unicodes = vec![0x0066];
        let mut layer = Layer::new_master(master);
        layer.width = 350.0;
        layer.contours =
            Contour::from_bezpath(&kurbo::Rect::new(10.0, 0.0, 340.0, 730.0).to_path(0.1));
        glyph.layers.push(layer);
        glyph
    })
    .unwrap();
    let composed = compose(&font, "f_f", Vec::new());
    assert_eq!(names(&composed.components), ["f", "f"]);
    let layer = &composed.glyph.layers[0];
    assert_eq!(
        layer.components[0].transform.translation(),
        kurbo::Vec2::new(0.0, 0.0)
    );
    assert_eq!(
        layer.components[1].transform.translation(),
        kurbo::Vec2::new(350.0, 0.0)
    );
    assert!(layer.components.iter().all(|part| !part.auto_align));
    assert_eq!(layer.width, 700.0, "the advance is the whole ligature");
}

#[test]
fn composition_is_built_per_master_and_stays_interpolatable() {
    let mut font = latin_font();
    let bold = Master {
        id: MasterId("bold".into()),
        ..Master::default()
    };
    font.add_master(bold.clone());
    // Give the bold master its own, wider drawing and anchor positions.
    *font.glyph_mut("A").unwrap() = {
        let mut glyph = font.glyph("A").unwrap().clone();
        glyph.layers.push({
            let mut layer = letter_a(&bold.id, 40.0).layers.remove(0);
            layer.kind = fc_model::LayerKind::Master(bold.id.clone());
            layer
        });
        glyph.layers.retain(|layer| !layer.is_empty());
        glyph
    };
    *font.glyph_mut("acutecomb").unwrap() = {
        let mut glyph = font.glyph("acutecomb").unwrap().clone();
        glyph.layers.push({
            let mut layer = acutecomb(&bold.id, 10.0).layers.remove(0);
            layer.kind = fc_model::LayerKind::Master(bold.id.clone());
            layer
        });
        glyph.layers.retain(|layer| !layer.is_empty());
        glyph
    };
    let composed = compose(&font, "Aacute", vec![0x00C1]);
    assert_eq!(composed.glyph.layers.len(), 2);
    let regular = composed.glyph.master_layer(&font.masters[0].id).unwrap();
    let heavy = composed.glyph.master_layer(&bold.id).unwrap();
    assert_eq!(
        regular.components[1].transform.translation(),
        kurbo::Vec2::new(300.0, 80.0)
    );
    assert_eq!(
        heavy.components[1].transform.translation(),
        kurbo::Vec2::new(330.0, 80.0)
    );
    assert_eq!(heavy.width, 640.0);
    let mut with_glyph = font.clone();
    with_glyph.insert_glyph(composed.glyph).unwrap();
    fc_model::masters_compatible(&with_glyph, "Aacute").expect("composite interpolates");
}

#[test]
fn a_missing_member_leaves_a_blank_glyph_and_is_reported() {
    let mut font = latin_font();
    font.remove_glyph("acutecomb");
    let composed = compose(&font, "Aacute", vec![0x00C1]);
    assert_eq!(composed.unavailable, [GlyphName::from("acutecomb")]);
    assert!(composed.components.is_empty());
    assert_eq!(composed.glyph.layers.len(), 1);
    assert!(composed.glyph.layers[0].is_empty());
    // A glyph with no composition record is simply created empty.
    let plain = compose(&font, "a.sc", Vec::new());
    assert!(plain.components.is_empty());
    assert!(plain.unavailable.is_empty());
    assert!(plain.glyph.layers[0].is_empty());
}

#[test]
fn an_overlay_composition_overrides_the_bundled_one() {
    let mut font = latin_font();
    font.insert_glyph({
        let master = font.default_master().id.clone();
        let mut glyph = Glyph::new("ring");
        let mut layer = Layer::new_master(master);
        layer.anchors.push(Anchor {
            name: "_top".into(),
            x: 10.0,
            y: 600.0,
        });
        glyph.layers.push(layer);
        glyph
    })
    .unwrap();
    let overlay =
        parse_overlay(r#"{"version":1,"glyphs":{"Aacute":{"composition":["A","ring"]}}}"#).unwrap();
    let composed = composed_glyph(
        &font,
        &BTreeMap::new(),
        &GlyphName::from("Aacute"),
        vec![0x00C1],
        &overlay,
    );
    assert_eq!(names(&composed.components), ["A", "ring"]);
    assert_eq!(
        composed.glyph.layers[0].components[1]
            .transform
            .translation(),
        kurbo::Vec2::new(290.0, 100.0)
    );
}

#[test]
fn staged_glyphs_are_usable_as_components_within_one_batch() {
    let mut font = Font::new_empty("Staged", 1_000);
    let master = font.default_master().id.clone();
    font.insert_glyph(acutecomb(&master, 0.0)).unwrap();
    let mut staged = BTreeMap::new();
    let base = letter_a(&master, 0.0);
    staged.insert(GlyphName::from("A"), base);
    let composed = composed_glyph(
        &font,
        &staged,
        &GlyphName::from("Aacute"),
        vec![0x00C1],
        &GlyphDataOverlay::default(),
    );
    assert_eq!(names(&composed.components), ["A", "acutecomb"]);
    assert_eq!(composed.glyph.layers[0].width, 600.0);
}

#[test]
fn renaming_follows_metrics_keys_feature_source_and_glyph_lists() {
    let mut font = latin_font();
    font.glyph_mut("acutecomb").unwrap().metrics_keys = MetricsKeys {
        left: Some("=|A+10".into()),
        width: Some("=A@500".into()),
        right: Some("=20".into()),
        ..MetricsKeys::default()
    };
    font.glyph_mut("A").unwrap().layers[0].metrics_keys = Some(MetricsKeys {
        left: Some("==acutecomb".into()),
        ..MetricsKeys::default()
    });
    font.features.classes.push(fc_model::FeatureClass {
        name: "uppercase".into(),
        code: "@uppercase = [A B C];".into(),
        ..fc_model::FeatureClass::default()
    });
    font.features.features.push(fc_model::FeatureBlock {
        tag: "ss01".into(),
        code: concat!(
            "# rename A here\n",
            "sub A by A.alt; # comment A\n",
            "sub B by @A;\n",
            "feature A;\n",
            "markClass A <anchor 0 0> @TOP;\n",
        )
        .into(),
        ..fc_model::FeatureBlock::default()
    });
    font.info.user_data = serde_json::json!({
        "com.fontcarver.inventory": {
            "glyph_lists": [{"id": "l1", "name": "Caps", "glyphs": ["A", "B"]}],
            "smart_filters": [{"name": "keep", "expression": "name:\"A\""}]
        }
    });

    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            document.apply(
                Command::RenameGlyph {
                    from: "A".into(),
                    to: "Alpha".into(),
                },
                "Rename",
            )
        })
        .unwrap()
        .unwrap();

    session
        .with_doc(doc, |document| {
            let keys = &document.font.glyph("acutecomb").unwrap().metrics_keys;
            assert_eq!(keys.left.as_deref(), Some("=|Alpha+10"));
            assert_eq!(keys.width.as_deref(), Some("=Alpha@500"));
            assert_eq!(keys.right.as_deref(), Some("=20"));
            let local = document.font.glyph("Alpha").unwrap().layers[0]
                .metrics_keys
                .as_ref()
                .unwrap();
            assert_eq!(local.left.as_deref(), Some("==acutecomb"));
            assert_eq!(
                document.font.features.classes[0].code,
                "@uppercase = [Alpha B C];"
            );
            assert_eq!(
                document.font.features.features[0].code,
                concat!(
                    "# rename A here\n",
                    "sub Alpha by A.alt; # comment A\n",
                    "sub B by @A;\n",
                    "feature A;\n",
                    // A mark class names a glyph, not a lookup.
                    "markClass Alpha <anchor 0 0> @TOP;\n",
                )
            );
            let lists = &document.font.info.user_data["com.fontcarver.inventory"]["glyph_lists"];
            assert_eq!(lists[0]["glyphs"], serde_json::json!(["Alpha", "B"]));
            // A saved query is text the user wrote, not a reference; it is left alone.
            let filters =
                &document.font.info.user_data["com.fontcarver.inventory"]["smart_filters"];
            assert_eq!(filters[0]["expression"], serde_json::json!("name:\"A\""));
        })
        .unwrap();

    session
        .with_doc_mut(doc, |document| document.undo())
        .unwrap()
        .unwrap();
    session
        .with_doc(doc, |document| {
            assert!(document.font.glyph("A").is_some());
            assert_eq!(
                document
                    .font
                    .glyph("acutecomb")
                    .unwrap()
                    .metrics_keys
                    .left
                    .as_deref(),
                Some("=|A+10")
            );
            assert_eq!(
                document.font.features.classes[0].code,
                "@uppercase = [A B C];"
            );
        })
        .unwrap();
}

/// `Glyph → Update Glyph Info` must never invent or drop a cmap entry.
///
/// The database speaks only for the names it lists. A suffixed private-use
/// glyph and a glyph whose AGLFN entry points somewhere else both keep their own
/// encodings; only a name the database lists exactly is refreshed from it.
#[test]
fn update_glyph_info_preserves_encodings_it_does_not_own() {
    let mut font = latin_font();
    let master = font.default_master().id.clone();
    for (name, unicodes) in [
        // A suffixed private-use glyph: `resolve_info` reports no encoding for
        // a suffix, which used to wipe the cmap entry.
        ("uni02E5.5", vec![0xF8FF_u32]),
        // AGLFN 1.7 maps the name `Omega` to U+2126 (OHM SIGN) on purpose; a
        // font encoding it U+03A9 must keep that.
        ("Omega", vec![0x03A9]),
        // An unsuffixed name the database does not list at all.
        ("uni02E5", vec![0xE000]),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicodes;
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    // A name the database does list is still refreshed from it.
    font.glyph_mut("A").unwrap().unicodes = vec![0x0041, 0x24B6];

    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            document.apply(
                Command::UpdateGlyphInfo {
                    glyphs: ["uni02E5.5", "Omega", "uni02E5", "A"]
                        .map(GlyphName::from)
                        .to_vec(),
                },
                "Update Glyph Info",
            )
        })
        .unwrap()
        .unwrap();

    session
        .with_doc(doc, |document| {
            let encodings = |name: &str| document.font.glyph(name).unwrap().unicodes.clone();
            assert_eq!(encodings("uni02E5.5"), [0xF8FF]);
            assert_eq!(encodings("Omega"), [0x03A9]);
            assert_eq!(encodings("uni02E5"), [0xE000]);
            assert_eq!(encodings("A"), [0x0041], "a listed name is refreshed");
            // Classification is still refreshed for every one of them.
            assert_eq!(
                document.font.glyph("Omega").unwrap().category.as_deref(),
                Some("Letter")
            );
            assert_eq!(
                document
                    .font
                    .glyph("uni02E5.5")
                    .unwrap()
                    .production_name
                    .as_deref(),
                Some("uni02E5.5")
            );
        })
        .unwrap();

    session
        .with_doc_mut(doc, |document| document.undo())
        .unwrap()
        .unwrap();
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyph("A").unwrap().unicodes, [0x0041, 0x24B6]);
        })
        .unwrap();
}

/// An overlay record for the exact name is the one thing that may re-encode.
#[test]
fn an_overlay_record_for_the_exact_name_re_encodes_a_glyph() {
    let mut font = latin_font();
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("uni02E5.5");
    glyph.unicodes = vec![0xF8FF];
    glyph.layers.push(Layer::new_master(master));
    font.insert_glyph(glyph).unwrap();
    font.info.user_data = serde_json::json!({
        "com.fontcarver.glyphData": {
            "version": 1,
            "glyphs": {
                "uni02E5.5": {"unicodes": [0xF8F0]},
                "Omega": {"unicodes": []}
            }
        }
    });
    font.insert_glyph({
        let mut omega = Glyph::new("Omega");
        omega.unicodes = vec![0x03A9];
        omega
    })
    .unwrap();

    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            document.apply(
                Command::UpdateGlyphInfo {
                    glyphs: ["uni02E5.5", "Omega"].map(GlyphName::from).to_vec(),
                },
                "Update Glyph Info",
            )
        })
        .unwrap()
        .unwrap();
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyph("uni02E5.5").unwrap().unicodes, [0xF8F0]);
            assert_eq!(
                document.font.glyph("Omega").unwrap().unicodes,
                [0x03A9],
                "an empty record never unencodes a glyph"
            );
        })
        .unwrap();
}
