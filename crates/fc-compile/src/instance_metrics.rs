//! Materialize missing metric overrides using the compiler's per-master policy.
use fc_model::MasterMetrics;

pub(crate) fn defaults(metrics: &MasterMetrics, upem: u16) -> MasterMetrics {
    let mut result = metrics.clone();
    let upem = f64::from(upem);
    let gap = metrics
        .typo_line_gap
        .unwrap_or((upem * 1.2 + metrics.descender - metrics.ascender).max(0.));
    result.hhea_ascender.get_or_insert(metrics.ascender + gap);
    result.hhea_descender.get_or_insert(metrics.descender);
    result.hhea_line_gap.get_or_insert(0.);
    result.typo_ascender.get_or_insert(metrics.ascender);
    result.typo_descender.get_or_insert(metrics.descender);
    result.typo_line_gap.get_or_insert(gap);
    result.win_ascent.get_or_insert(metrics.ascender + gap);
    result.win_descent.get_or_insert(metrics.descender.abs());
    result.vhea_ascender.get_or_insert(0.);
    result.vhea_descender.get_or_insert(0.);
    result.vhea_line_gap.get_or_insert(0.);
    result
        .default_vertical_origin
        .get_or_insert(metrics.ascender);
    result.default_vertical_advance.get_or_insert(upem);
    result
}

/// Resolve registered-axis metadata without retaining the default instance's style bits.
pub(crate) fn apply_style(
    font: &mut fc_model::Font,
    metrics: &mut MasterMetrics,
    location: &fc_model::DesignLocation,
) {
    let user = |tag: [u8; 4]| {
        font.axes
            .iter()
            .find(|axis| axis.tag.0 == tag)
            .map(|axis| axis.design_to_user(location[&axis.tag]))
    };
    let weight = user(*b"wght");
    let width = user(*b"wdth");
    let italic_axis = user(*b"ital");
    let slant = user(*b"slnt");
    if let Some(weight) = weight {
        font.info.open_type_os2_weight_class = Some(weight.round().clamp(1., 1000.) as u16);
    }
    if let Some(width) = width {
        let mapping = fc_model::Axis {
            map: [50., 62.5, 75., 87.5, 100., 112.5, 125., 150., 200.]
                .into_iter()
                .enumerate()
                .map(|(i, percentage)| (percentage, (i + 1) as f64))
                .collect(),
            ..Default::default()
        };
        font.info.open_type_os2_width_class =
            Some(mapping.user_to_design(width.clamp(50., 200.)).round() as u16);
    }
    if let Some(slant) = slant {
        metrics.italic_angle = slant;
    }
    let bits = &mut font.info.open_type_os2_selection;
    let bold = weight.map_or(bits.contains(&5), |weight| weight >= 700.);
    let italic = italic_axis.map_or(bits.contains(&0), |italic| italic >= 0.5);
    let oblique = !italic && slant.map_or(bits.contains(&9), |slant| slant != 0.);
    bits.retain(|bit| ![0, 5, 6, 9].contains(bit));
    if italic {
        bits.push(0);
    }
    if bold {
        bits.push(5);
    }
    if oblique {
        bits.push(9);
    }
    if !italic && !bold && !oblique {
        bits.push(6);
    }
    bits.sort_unstable();
}
