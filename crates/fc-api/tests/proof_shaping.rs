//! Proofing behaviour the shaping and sheet-export surfaces depend on:
//! tri-state feature overrides and the embedded proof font.

use std::sync::Arc;

use fc_api::{Api, ProofFont, ShapedRun};
use fc_doc::Session;
use fc_model::{
    Axis, Contour, FeatureBlock, Font, Glyph, Instance, KernSide, Layer, MasterId, Tag,
};
use kurbo::BezPath;
use serde_json::json;

/// A font with one feature the shaper applies by default (`liga`), one it does
/// not (`dlig`), and a `kern` pair, so all three override states are distinct.
fn proof_font_source() -> Font {
    let mut font = Font::new_empty("Proof Shaping", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 600.0),
        ("f", Some(0x66), 300.0),
        ("i", Some(0x69), 280.0),
        ("f_i", None, 540.0),
        ("A", Some(0x41), 700.0),
        ("V", Some(0x56), 700.0),
        ("A_V", None, 1100.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        let mut path = BezPath::new();
        path.move_to((40.0, 0.0));
        path.line_to((40.0, 700.0));
        path.line_to((width - 40.0, 700.0));
        path.line_to((width - 40.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert proof glyph");
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".to_owned(),
        code: "sub f i by f_i;".to_owned(),
        ..FeatureBlock::default()
    });
    // Discretionary ligatures are off until a client asks for them, which is the
    // state a Boolean toggle cannot express.
    font.features.features.push(FeatureBlock {
        tag: "dlig".to_owned(),
        code: "sub A V by A_V;".to_owned(),
        ..FeatureBlock::default()
    });
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.,
    );
    font
}

fn shape(api: &Api, doc: fc_doc::DocId, text: &str, features: serde_json::Value) -> ShapedRun {
    serde_json::from_value(
        api.call(
            "text.shape",
            json!({"doc": doc, "text": text, "features": features}),
        )
        .expect("shape"),
    )
    .expect("shaped run")
}

#[test]
fn feature_override_states_are_default_on_and_off() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(proof_font_source(), None);
    let api = Api::new(session);

    // A feature the shaper applies by default. Default and On agree; Off is the
    // state that differs, and only an explicit Off produces it.
    let default_liga = shape(&api, doc, "fi", json!([]));
    assert_eq!(default_liga.names, vec![Some("f_i".to_owned())]);
    assert_eq!(
        shape(&api, doc, "fi", json!([["liga", true]])).names,
        default_liga.names
    );
    assert_eq!(
        shape(&api, doc, "fi", json!([["liga", false]])).names,
        vec![Some("f".to_owned()), Some("i".to_owned())],
        "an off override must suppress a feature the font applies by default"
    );

    // A feature the shaper does not apply by default. Here Default and Off
    // agree, and On is the state that differs — the opposite of `liga`, which
    // is what makes the three states distinguishable rather than two.
    let default_dlig = shape(&api, doc, "AV", json!([]));
    assert_eq!(
        default_dlig.names,
        vec![Some("A".to_owned()), Some("V".to_owned())],
        "a discretionary feature must stay off until it is asked for"
    );
    let on_dlig = shape(&api, doc, "AV", json!([["dlig", true]]));
    assert_eq!(
        on_dlig.names,
        vec![Some("A_V".to_owned())],
        "an on override must apply a feature the font leaves off by default"
    );
    assert_ne!(on_dlig.names, default_dlig.names);
    let off_dlig = shape(&api, doc, "AV", json!([["dlig", false]]));
    assert_eq!(off_dlig.names, default_dlig.names);
    // Taken together the two features separate all three states: no single
    // Boolean can reproduce both `liga` Default and `dlig` Default.
    assert_ne!(default_liga.names, on_dlig.names);

    // The same three states over a positioning feature.
    assert_eq!(shape(&api, doc, "AV", json!([])).glyphs[0].x_advance, 620.);
    assert_eq!(
        shape(&api, doc, "AV", json!([["kern", true]])).glyphs[0].x_advance,
        620.
    );
    assert_eq!(
        shape(&api, doc, "AV", json!([["kern", false]])).glyphs[0].x_advance,
        700.
    );
}

/// A two-master variable source with one named instance between them.
fn variable_source() -> Font {
    let mut font = proof_font_source();
    let tag = Tag(*b"wght");
    font.axes.push(Axis {
        tag,
        name: "Weight".into(),
        min: 100.,
        default: 100.,
        max: 900.,
        map: vec![(100., 0.), (900., 1000.)],
        ..Default::default()
    });
    font.masters[0].location.insert(tag, 0.);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(tag, 1000.);
    let heavy_id = heavy.id.clone();
    font.masters.push(heavy);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = fc_model::LayerId::new();
        layer.kind = fc_model::LayerKind::Master(heavy_id.clone());
        glyph.layers.push(layer);
    }
    font.instances.push(Instance {
        name: "Medium".into(),
        location: [(tag, 500.)].into(),
        active: true,
        ..Default::default()
    });
    font
}

#[test]
fn proof_font_compiles_the_default_master_as_woff2() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(proof_font_source(), None);
    let api = Api::new(session);

    let font: ProofFont = serde_json::from_value(
        api.call("text.proof_font", json!({"doc": doc}))
            .expect("compile"),
    )
    .expect("proof font");
    assert_eq!(font.generation, 0);
    assert_eq!(font.family_name, "Proof Shaping");
    assert_eq!(font.units_per_em, 1_000);
    let bytes = base64_decode(&font.woff2_base64);
    assert_eq!(&bytes[..4], b"wOF2", "the sheet embeds a WOFF 2 face");
    assert!(bytes.len() > 100, "compiled face is {} bytes", bytes.len());
    assert!(font
        .woff2_base64
        .chars()
        .all(|c| { c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' }));
}

#[test]
fn proof_font_compiles_a_named_instance_and_each_master() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(variable_source(), None);
    let masters = session
        .with_doc(doc, |document| {
            document
                .font
                .masters
                .iter()
                .map(|master| (master.id.to_string(), master.name.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap();
    let api = Api::new(session);

    let instance: ProofFont = serde_json::from_value(
        api.call("text.proof_font", json!({"doc": doc, "instance": 0}))
            .expect("instance"),
    )
    .expect("proof font");
    assert_eq!(instance.style_name, "Medium");
    assert_eq!(&base64_decode(&instance.woff2_base64)[..4], b"wOF2");

    for (id, name) in masters {
        let compiled: ProofFont = serde_json::from_value(
            api.call("text.proof_font", json!({"doc": doc, "master": id}))
                .expect("master"),
        )
        .expect("proof font");
        assert_eq!(compiled.style_name, name);
        assert_eq!(&base64_decode(&compiled.woff2_base64)[..4], b"wOF2");
    }
}

#[test]
fn proof_font_rejects_an_unknown_master_or_instance_without_touching_the_document() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(variable_source(), None);
    let before = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    let api = Api::new(session.clone());

    for params in [
        json!({"doc": doc, "instance": 1}),
        json!({"doc": doc, "instance": 4_294_967_295u32}),
        json!({"doc": doc, "master": "00000000000000000000000000000000"}),
    ] {
        assert!(
            api.call("text.proof_font", params.clone()).is_err(),
            "{params} should be refused"
        );
    }
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.clone())
            .unwrap(),
        before
    );
}

/// Minimal standard base64 decoder, so the test checks the transport encoding
/// rather than re-using the encoder under test.
fn base64_decode(encoded: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(encoded.len() / 4 * 3);
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in encoded.bytes() {
        if byte == b'=' {
            break;
        }
        let value = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .unwrap_or_else(|| panic!("not base64: {byte}")) as u32;
        accumulator = (accumulator << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    out
}
