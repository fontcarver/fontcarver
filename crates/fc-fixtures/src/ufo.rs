//! Deterministic UFO fixtures.

use std::path::Path;

use norad::{
    AffineTransform, Anchor, Component, Contour, ContourPoint, Font, Glyph, Name, PointType,
};
use plist::Value;

use crate::Error;

/// Writes the deterministic three-glyph `Minimal.ufo` fixture to `dir`.
///
/// This deletes and recreates `dir`, matching [`norad::Font::save`] semantics.
pub fn write_minimal(dir: &Path) -> Result<(), Error> {
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut font = Font::new();
    font.font_info.family_name = Some("Minimal".to_owned());
    font.font_info.style_name = Some("Regular".to_owned());
    font.font_info.units_per_em = Some(1_000_u32.into());
    font.font_info.ascender = Some(800.0);
    font.font_info.descender = Some(-200.0);

    let mut glyph_a = Glyph::new("A");
    glyph_a.width = 600.0;
    glyph_a.codepoints.insert('A');
    glyph_a.contours = vec![
        closed_contour(&[(50.0, 0.0), (300.0, 700.0), (550.0, 0.0)]),
        closed_contour(&[(220.0, 240.0), (300.0, 470.0), (380.0, 240.0)]),
    ];

    let mut glyph_b = Glyph::new("B");
    glyph_b.width = 620.0;
    glyph_b.codepoints.insert('B');
    glyph_b.components.push(Component::new(
        Name::new("A").map_err(|error| Error::Internal(error.to_string()))?,
        AffineTransform {
            x_scale: 0.8,
            y_scale: 0.8,
            x_offset: 70.0,
            y_offset: 0.0,
            ..AffineTransform::default()
        },
        None,
    ));
    glyph_b.anchors.push(Anchor::new(
        310.0,
        720.0,
        Some(Name::new("top").map_err(|error| Error::Internal(error.to_string()))?),
        None,
        None,
    ));

    let mut space = Glyph::new("space");
    space.width = 300.0;
    space.codepoints.insert(' ');

    let layer = font.default_layer_mut();
    layer.insert_glyph(glyph_a);
    layer.insert_glyph(glyph_b);
    layer.insert_glyph(space);

    font.groups.insert(
        Name::new("public.kern1.A").map_err(|error| Error::Internal(error.to_string()))?,
        vec![Name::new("A").map_err(|error| Error::Internal(error.to_string()))?],
    );
    font.groups.insert(
        Name::new("public.kern2.B").map_err(|error| Error::Internal(error.to_string()))?,
        vec![Name::new("B").map_err(|error| Error::Internal(error.to_string()))?],
    );
    font.kerning.insert(
        Name::new("public.kern1.A").map_err(|error| Error::Internal(error.to_string()))?,
        [(
            Name::new("public.kern2.B").map_err(|error| Error::Internal(error.to_string()))?,
            -80.0,
        )]
        .into_iter()
        .collect(),
    );
    font.features = "feature kern { pos A B -80; } kern;\n".to_owned();
    font.lib.insert(
        "public.glyphOrder".to_owned(),
        Value::Array(
            ["A", "B", "space"]
                .map(|name| Value::String(name.to_owned()))
                .into(),
        ),
    );
    font.lib
        .insert("com.fontcarver.fixture".to_owned(), Value::Boolean(true));

    font.save(dir)?;
    Ok(())
}

fn closed_contour(points: &[(f64, f64)]) -> Contour {
    Contour::new(
        points
            .iter()
            .map(|&(x, y)| ContourPoint::new(x, y, PointType::Line, false, None, None))
            .collect(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::Path};

    use super::write_minimal;

    #[test]
    fn regeneration_is_byte_identical() {
        let temp = tempfile::tempdir().unwrap();
        let generated = temp.path().join("Minimal.ufo");
        write_minimal(&generated).unwrap();
        let committed = crate::workspace_root().join("fixtures/ufo/Minimal.ufo");
        assert_eq!(directory_bytes(&generated), directory_bytes(&committed));
    }

    fn directory_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    visit(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }
}
