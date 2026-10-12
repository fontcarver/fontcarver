use fc_api::{Api, ApiError, DocId};
use fc_doc::Session;
use fc_model::{Component, Contour, Font, Glyph, Layer, LayerId, LayerKind, SmartAxis};
use kurbo::{Affine, Rect, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

struct Fixture {
    session: Arc<Session>,
    api: Api,
    doc: DocId,
    layer: LayerId,
}

impl Fixture {
    fn new() -> Self {
        let mut font = Font::new_empty("Smart component editing", 1000);
        let master = font.default_master().id.clone();
        let mut base = Glyph::new("part");
        base.smart_axes = ["Width", "Height"]
            .map(|name| SmartAxis {
                name: name.into(),
                min: 0.,
                default: 0.,
                max: 100.,
            })
            .into();
        for (width, height) in [(0., 0.), (100., 0.), (0., 100.), (100., 100.)] {
            let mut layer = Layer::new_master(master.clone());
            layer.contours =
                Contour::from_bezpath(&Rect::new(0., 0., 100. + width, 400. + height).to_path(0.1));
            if width != 0. || height != 0. {
                layer.kind = LayerKind::Smart {
                    master: master.clone(),
                    location: [("Width".into(), width), ("Height".into(), height)].into(),
                };
            }
            base.layers.push(layer);
        }
        font.insert_glyph(base).unwrap();
        let mut composite = Glyph::new("A");
        let mut layer = Layer::new_master(master);
        let layer_id = layer.id.clone();
        for (width, height, translation) in [(25., 75., (20., 30.)), (75., 25., (300., 0.))] {
            layer.components.push(Component {
                base: "part".into(),
                transform: Affine::translate(translation),
                smart: Some([("Width".into(), width), ("Height".into(), height)].into()),
                ..Default::default()
            });
        }
        composite.layers.push(layer);
        font.insert_glyph(composite).unwrap();
        let session = Arc::new(Session::new());
        let doc = session.open_font(font, None);
        let api = Api::new(session.clone());
        Self {
            session,
            api,
            doc,
            layer: layer_id,
        }
    }

    fn font(&self) -> Font {
        self.session
            .with_doc(self.doc, |document| document.font.clone())
            .unwrap()
    }

    fn outline(&self) -> Value {
        let gid = self
            .session
            .with_doc(self.doc, |document| document.glyph_id("A").unwrap())
            .unwrap();
        self.api
            .call(
                "glyphs.outline",
                json!({"doc":self.doc,"gid":gid,"layer":self.layer}),
            )
            .unwrap()
    }

    fn set_values(&self, index: usize, values: Value) -> Result<Value, ApiError> {
        let mut component = self.outline()["components"][index]["component"].clone();
        component["smart"] = values;
        self.api.call(
            "doc.apply",
            json!({
                "doc":self.doc,
                "label":"Smart parameters",
                "command":{"SetComponent":{
                    "glyph":"A","layer":self.layer,"index":index,"component":component
                }}
            }),
        )
    }

    fn history(&self, method: &str) {
        self.api.call(method, json!({"doc":self.doc})).unwrap();
    }
}

#[test]
fn local_parameters_edit_independent_instances_and_accept_axis_endpoints() {
    let fixture = Fixture::new();
    let source = fixture.font().glyph("part").unwrap().clone();
    let initial = fixture.outline();
    assert_eq!(
        initial["components"][0]["bbox"],
        json!([20., 30., 145., 505.])
    );
    assert_eq!(
        initial["components"][1]["bbox"],
        json!([300., 0., 475., 425.])
    );
    assert_eq!(
        initial["components"][0]["smart_axes"],
        json!([
            {"name":"Width","min":0.,"default":0.,"max":100.},
            {"name":"Height","min":0.,"default":0.,"max":100.}
        ])
    );

    fixture
        .set_values(0, json!({"Width":60.,"Height":75.}))
        .unwrap();
    let first_edit = fixture.outline();
    assert_eq!(
        first_edit["components"][0]["bbox"],
        json!([20., 30., 180., 505.])
    );
    assert_eq!(first_edit["components"][1], initial["components"][1]);
    fixture
        .set_values(1, json!({"Width":75.,"Height":50.}))
        .unwrap();
    let second_edit = fixture.outline();
    assert_eq!(second_edit["components"][0], first_edit["components"][0]);
    assert_eq!(
        second_edit["components"][1]["bbox"],
        json!([300., 0., 475., 450.])
    );
    assert_eq!(
        second_edit["components"][0]["component"]["smart"],
        json!({"Width":60.,"Height":75.})
    );

    fixture
        .set_values(0, json!({"Width":100.,"Height":100.}))
        .unwrap();
    assert_eq!(
        fixture.outline()["components"][0]["bbox"],
        json!([20., 30., 220., 530.])
    );
    fixture
        .set_values(0, json!({"Width":0.,"Height":0.}))
        .unwrap();
    let reset = fixture.outline();
    assert_eq!(
        reset["components"][0]["bbox"],
        json!([20., 30., 120., 430.])
    );
    assert_eq!(reset["components"][1], second_edit["components"][1]);
    assert_eq!(fixture.font().glyph("part"), Some(&source));
}

#[test]
fn invalid_local_parameters_preserve_document_and_redo_history_atomically() {
    let fixture = Fixture::new();
    fixture
        .set_values(0, json!({"Width":60.,"Height":35.}))
        .unwrap();
    let edited = fixture.font();
    fixture.history("doc.undo");
    let before = fixture.font();
    let outline_before = fixture.outline();
    let summary_before = fixture
        .api
        .call("doc.summary", json!({"doc":fixture.doc}))
        .unwrap();
    let state_before = fixture
        .session
        .with_doc(fixture.doc, |document| {
            (
                document.generation,
                document.glyph_generation(document.glyph_id("A").unwrap()),
                document.undo_len(),
            )
        })
        .unwrap();
    for (values, message) in [
        (json!({"Width":-1.,"Height":35.}), "outside its range"),
        (json!({"Width":101.,"Height":35.}), "outside its range"),
        (json!({"Width":60.,"Height":-1.}), "outside its range"),
        (json!({"Width":60.,"Height":101.}), "outside its range"),
        (
            json!({"Width":60.,"Height":35.,"Depth":50.}),
            "unknown smart-component axis",
        ),
    ] {
        let error = fixture.set_values(0, values).unwrap_err();
        assert_eq!(error.code, "invalid_params");
        assert!(error.message.contains(message), "{error}");
        assert_eq!(fixture.font(), before);
        assert_eq!(fixture.outline(), outline_before);
        assert_eq!(
            fixture
                .api
                .call("doc.summary", json!({"doc":fixture.doc}))
                .unwrap(),
            summary_before
        );
        fixture
            .session
            .with_doc(fixture.doc, |document| {
                assert_eq!(
                    (
                        document.generation,
                        document.glyph_generation(document.glyph_id("A").unwrap()),
                        document.undo_len(),
                    ),
                    state_before
                );
            })
            .unwrap();
    }
    fixture.history("doc.redo");
    assert_eq!(fixture.font(), edited);
}

#[test]
fn local_parameter_edit_undo_redo_and_native_reopen_preserve_both_instances() {
    let mut fixture = Fixture::new();
    let initial = fixture.font();
    fixture
        .set_values(0, json!({"Width":60.,"Height":35.}))
        .unwrap();
    let edited = fixture.font();
    let edited_components = fixture.outline()["components"].clone();
    fixture.history("doc.undo");
    assert_eq!(fixture.font(), initial);
    fixture.history("doc.redo");
    assert_eq!(fixture.font(), edited);
    assert_eq!(fixture.outline()["components"], edited_components);

    let temp = tempfile::tempdir().unwrap();
    fixture
        .api
        .configure_config_dir(temp.path().join("config"))
        .unwrap();
    let path = temp.path().join("smart-components.fcf");
    let saved = fixture
        .api
        .call("session.save", json!({"doc":fixture.doc,"path":path}))
        .unwrap();
    assert_eq!(saved["dirty"], false);
    fixture
        .api
        .call("session.close", json!({"doc":fixture.doc}))
        .unwrap();
    let reopened = fixture
        .api
        .call("session.open", json!({"path":path}))
        .unwrap();
    fixture.doc = serde_json::from_value(reopened["doc"].clone()).unwrap();
    assert_eq!(fixture.font(), edited);
    assert_eq!(fixture.outline()["components"], edited_components);
    assert_eq!(reopened["dirty"], false);
    fixture.history("doc.undo");
    assert_eq!(fixture.font(), edited);
}
