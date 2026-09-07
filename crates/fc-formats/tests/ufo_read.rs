use fc_formats::ufo::read_ufo;
use fc_model::{FeaturesSource, KernSide, LayerKind};

#[test]
fn reads_minimal_ufo_semantics() {
    let font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();

    assert_eq!(font.info.family_name, "Minimal");
    assert_eq!(font.info.style_name, "Regular");
    assert_eq!(font.info.units_per_em, 1_000);
    assert_eq!(
        font.glyphs
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["A", "B", "space"]
    );
    let master = font.default_master();
    let a = font.glyph("A").unwrap().master_layer(&master.id).unwrap();
    assert_eq!(a.contours.len(), 2);
    assert!(a.contours.iter().all(|contour| contour.closed));
    let b = font.glyph("B").unwrap().master_layer(&master.id).unwrap();
    assert_eq!(b.components[0].base.as_ref(), "A");
    assert_eq!(b.anchors[0].name, "top");
    assert_eq!(font.features.source_of_truth, FeaturesSource::Fea);
    assert!(font.features.features[0].code.contains("feature kern"));
    assert!(font.kerning_groups.side1["public.kern1.A"].contains("A"));
    assert_eq!(
        master.kerning_h.0[&(
            KernSide::Group("public.kern1.A".to_owned()),
            KernSide::Group("public.kern2.B".to_owned())
        )],
        -80.0
    );
    assert!(matches!(a.kind, LayerKind::Master(_)));
    assert!(font.extra.lib.contains_key("ufo.metainfo"));
}
