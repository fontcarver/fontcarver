#[path = "support/tt.rs"]
mod support;

use fc_sfnt::tt::point_basis;
use read_fonts::types::Tag;
use support::{composite, font, simple};

#[test]
fn expanded_indices_follow_transformed_components_and_matched_points() {
    let child = simple(&[(10, 20, true), (30, 40, false), (50, 20, true)], &[]);
    // Stored matrix order: xx, yx, xy, yy. The unequal cross terms catch transposition.
    let parent = composite(&[
        (2 | 0x80 | 0x800, 0, 8, 12, &[16384, 8192, 4096, 16384]),
        (0, 0, 1, 0, &[]),
    ]);
    let basis = point_basis(&font(&[child, parent]), 1).unwrap();
    let actual: Vec<_> = basis
        .points
        .iter()
        .map(|p| (p.position.x, p.position.y, p.on_curve, p.contour))
        .collect();
    assert_eq!(
        actual,
        vec![
            (26.0, 41.0, true, 0),
            (51.0, 71.0, false, 0),
            (66.0, 61.0, true, 0),
            (51.0, 71.0, true, 1),
            (71.0, 91.0, false, 1),
            (91.0, 71.0, true, 1),
        ]
    );
    assert_eq!(basis.phantom_start, 6);
}

#[test]
fn composite_binding_tracks_child_geometry_and_metric_selection() {
    let child = simple(&[(10, 20, true)], &[]);
    let parent = composite(&[(2, 0, 0, 0, &[])]);
    let original = point_basis(&font(&[child.clone(), parent.clone()]), 1).unwrap();
    let changed = simple(&[(11, 20, true)], &[]);
    assert_ne!(
        original.fingerprint,
        point_basis(&font(&[changed, parent]), 1)
            .unwrap()
            .fingerprint
    );
    let metric_parent = composite(&[(2 | 0x200, 0, 0, 0, &[])]);
    let metric_basis = point_basis(&font(&[child, metric_parent]), 1).unwrap();
    assert_eq!(original.points, metric_basis.points);
    assert_ne!(original.fingerprint, metric_basis.fingerprint);
    assert!(metric_basis.component_metrics);
}

#[test]
fn header_only_empty_glyph_is_valid_and_has_the_same_point_identity() {
    let with_length = font(&[simple(&[], &[])]);
    let header_only = font(&[vec![0; 10]]);
    let omitted = font(&[vec![]]);
    let original = point_basis(&with_length, 0).unwrap();
    assert_eq!(original, point_basis(&header_only, 0).unwrap());
    assert_eq!(original, point_basis(&omitted, 0).unwrap());
}

#[test]
fn cycles_truncated_components_and_conflicting_transforms_are_rejected() {
    let child = simple(&[(10, 20, true)], &[]);
    let cycle = font(&[composite(&[(2, 0, 0, 0, &[])])]);
    assert!(point_basis(&cycle, 0).is_err());
    let parent = composite(&[(2 | 0x80, 0, 0, 0, &[16384, 0, 0, 16384])]);
    for cut in 1..parent.len() {
        assert!(
            point_basis(&font(&[child.clone(), parent[..cut].to_vec()]), 1).is_err(),
            "cut {cut}"
        );
    }
    for flags in [0x08 | 0x40, 0x08 | 0x80, 0x40 | 0x80, 0x800 | 0x1000] {
        let parent = composite(&[(2 | flags, 0, 0, 0, &[16384, 0, 0, 16384])]);
        assert!(
            point_basis(&font(&[child.clone(), parent]), 1).is_err(),
            "flags {flags:x}"
        );
    }
}

#[test]
fn malformed_instructions_and_point_data_cannot_create_a_binding() {
    let child = simple(&[(10, 20, true)], &[0]);
    for cut in 1..child.len() {
        assert!(
            point_basis(&font(&[child[..cut].to_vec()]), 0).is_err(),
            "cut {cut}"
        );
    }
    let mut parent = composite(&[(2 | 0x100, 0, 0, 0, &[])]);
    parent.extend([0, 2, 0]); // Declares two instruction bytes but stores only one.
    assert!(point_basis(&font(&[child, parent]), 1).is_err());
}

#[test]
fn invalid_loca_end_and_malformed_variable_input_are_rejected() {
    let bytes = font(&[simple(&[], &[])]);
    let malformed = support::replace(
        &bytes,
        Tag::new(b"loca"),
        [0u32, u32::MAX]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect(),
    );
    assert!(point_basis(&malformed, 0).is_err());
    let variable = support::replace(&bytes, Tag::new(b"fvar"), vec![0; 16]);
    assert!(point_basis(&variable, 0).is_err());
}

#[test]
fn exponential_component_expansion_is_rejected_without_building_the_outline() {
    let mut glyphs = vec![simple(&[(10, 20, true), (30, 50, false)], &[])];
    for id in 1..18 {
        glyphs.push(composite(&[(2, id - 1, 0, 0, &[]), (2, id - 1, 0, 0, &[])]));
    }
    let bytes = font(&glyphs);
    assert!(fc_sfnt::tt::fingerprints(&bytes, &[17])
        .unwrap_err()
        .to_string()
        .contains("maxp capacity"));
}

#[test]
fn invalid_point_matches_and_unsupported_phantom_matches_are_distinguished() {
    let child = simple(&[(10, 20, true)], &[]);
    let invalid = font(&[
        child.clone(),
        composite(&[(2, 0, 0, 0, &[]), (0, 0, 5, 0, &[])]),
    ]);
    assert!(point_basis(&invalid, 1)
        .unwrap_err()
        .to_string()
        .contains("outside outline"));
    let phantom = font(&[child, composite(&[(2, 0, 0, 0, &[]), (0, 0, 1, 0, &[])])]);
    assert!(point_basis(&phantom, 1)
        .unwrap_err()
        .to_string()
        .contains("not yet supported"));
    assert!(fc_sfnt::tt::fingerprints(&phantom, &[1])
        .unwrap_err()
        .to_string()
        .contains("not yet supported"));
}
