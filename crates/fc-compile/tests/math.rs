//! Editable MATH through the static font export pipeline.

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{Font, GlyphName, MathTable, MathValue, Tag};
use read_fonts::FontRef;

fn dejavu() -> Font {
    let bytes = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap();
    fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0)
}

/// Decode an exported MATH table and rename it back to source identities.
///
/// Exported names may be production names, so records are matched by the
/// position of each exported glyph in the source's exported glyph order.
fn exported_math(bytes: &[u8], font: &Font) -> Option<MathTable> {
    let sfnt = fc_sfnt::read_bytes(bytes, &Default::default()).unwrap();
    let output = sfnt.fonts.into_iter().next().unwrap();
    assert!(
        !output.extra.raw_tables.contains_key(&Tag(*b"MATH")),
        "exported MATH must decode"
    );
    let output_order: Vec<GlyphName> = output.glyph_order().cloned().collect();
    let mut math = output.math?;
    let source_order: Vec<GlyphName> = font
        .glyphs
        .values()
        .filter(|glyph| glyph.export)
        .map(|glyph| glyph.name.clone())
        .collect();
    assert_eq!(output_order.len(), source_order.len());
    // Two passes avoid collisions when production and source names overlap.
    for (index, name) in output_order.iter().enumerate() {
        math.rename_glyph(name, &format!("\u{1}{index}").into());
    }
    for (index, name) in source_order.iter().enumerate() {
        math.rename_glyph(&format!("\u{1}{index}"), name);
    }
    Some(math)
}

fn options(format: ExportFormat) -> ExportOptions {
    ExportOptions {
        format,
        // DejaVu's imported GPOS uses point-index anchors that CFF conversion
        // rejects. Editable MATH does not depend on raw-table preservation.
        preserve_untouched_tables: format != ExportFormat::Otf,
        ..Default::default()
    }
}

#[test]
fn imported_math_is_rewritten_semantically_in_every_container() {
    let font = dejavu();
    let source = font.math.clone().unwrap();
    for format in [
        ExportFormat::Ttf,
        ExportFormat::Otf,
        ExportFormat::Woff,
        ExportFormat::Woff2,
    ] {
        let report = export(&font, &|_| 0, &options(format)).unwrap();
        assert_eq!(
            exported_math(&report.bytes, &font).as_ref(),
            Some(&source),
            "{format:?}"
        );
        assert!(
            report.warnings.iter().all(|w| !w.message.contains("MATH")),
            "{format:?}: {:?}",
            report.warnings
        );
    }
}

#[test]
fn authored_values_and_production_names_are_exported() {
    let mut font = dejavu();
    let math = font.math.as_mut().unwrap();
    math.constants.as_mut().unwrap().axis_height = MathValue::from(321.);
    math.glyphs.entry("A".into()).or_default().top_accent = Some(MathValue::from(-12.));
    let expected = font.math.clone().unwrap();
    for production_names in [false, true] {
        let report = export(
            &font,
            &|_| 1,
            &ExportOptions {
                production_names,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(exported_math(&report.bytes, &font), Some(expected.clone()));
    }
}

#[test]
fn explicit_deletion_is_not_restored_from_import_bytes() {
    let mut font = dejavu();
    font.math = None;
    assert!(font.math_table_owned);
    let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .is_none());
    // The same holds when stale raw bytes are present in a legacy document.
    font.extra
        .raw_tables
        .insert(Tag(*b"MATH"), vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
    let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .is_none());
    assert!(report.warnings.iter().all(|w| !w.message.contains("MATH")));
}

#[test]
fn excluded_glyph_references_are_dropped_with_a_warning() {
    let mut font = dejavu();
    let math = font.math.as_ref().unwrap();
    // Exclude a glyph used as a variant or assembly part by another record.
    let excluded = math
        .glyphs
        .values()
        .flat_map(|info| [&info.vertical, &info.horizontal])
        .flatten()
        .flat_map(|construction| {
            construction
                .variants
                .iter()
                .map(|variant| variant.glyph.clone())
                .skip(1)
        })
        .next()
        .expect("DejaVu has a larger variant");
    font.glyph_mut(&excluded).unwrap().export = false;
    let mut expected = font.math.clone().unwrap();
    assert!(expected.retain_glyphs(|name| *name != excluded) > 0);
    let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    assert_eq!(exported_math(&report.bytes, &font), Some(expected));
    let warning = report
        .warnings
        .iter()
        .find(|w| w.code == "math-references-dropped")
        .expect("dropped reference warning");
    assert!(
        warning.message.contains(excluded.as_ref() as &str),
        "{}",
        warning.message
    );
    // Source data is untouched by export filtering.
    assert!(font
        .math
        .as_ref()
        .unwrap()
        .glyphs
        .values()
        .flat_map(|info| [&info.vertical, &info.horizontal])
        .flatten()
        .any(|c| c.variants.iter().any(|v| v.glyph == excluded)));
}

#[test]
fn invalid_authored_math_fails_instead_of_writing_a_partial_table() {
    let mut font = dejavu();
    font.math.as_mut().unwrap().min_connector_overlap = -1.;
    let error = export(&font, &|_| 0, &ExportOptions::default()).unwrap_err();
    assert!(error.to_string().contains("MATH"), "{error}");
}

#[test]
fn table_selection_can_omit_math() {
    let font = dejavu();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            table_subset: Some(Vec::new()),
            ..Default::default()
        },
    )
    .unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .is_none());
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            table_subset: Some(vec![Tag(*b"MATH")]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exported_math(&report.bytes, &font), font.math);
}

#[test]
fn renamed_glyphs_and_static_instances_keep_math() {
    let mut font = dejavu();
    let covered = font
        .math
        .as_ref()
        .unwrap()
        .glyphs
        .keys()
        .find(|name| {
            font.glyph(name)
                .is_some_and(|glyph| !glyph.unicodes.is_empty())
        })
        .cloned()
        .expect("covered encoded glyph");
    font.rename_glyph(&covered, "math.custom.name").unwrap();
    // Instances require editable layout; MATH does not depend on it.
    font.features = fc_model::Features::default();
    for tag in [b"GSUB", b"GPOS", b"GDEF", b"kern"] {
        font.extra.raw_tables.remove(&Tag(*tag));
    }
    assert!(font
        .math
        .as_ref()
        .unwrap()
        .glyphs
        .contains_key("math.custom.name"));
    font.instances.push(fc_model::Instance {
        name: "Book".into(),
        style_name: "Book".into(),
        location: font.default_master().location.clone(),
        active: true,
        ..Default::default()
    });
    let expected = font.math.clone();
    for (production_names, instance) in [(true, None), (false, Some(0)), (true, Some(0))] {
        let report = export(
            &font,
            &|_| 1,
            &ExportOptions {
                production_names,
                instance,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            exported_math(&report.bytes, &font),
            expected,
            "production names {production_names}, instance {instance:?}"
        );
    }
}
