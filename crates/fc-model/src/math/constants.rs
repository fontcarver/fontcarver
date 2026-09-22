//! Named OpenType mathematical typesetting constants.

use super::MathValue;
use serde::{Deserialize, Serialize};

/// All constants in MATH 1.0, in font units except the three percentages.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathConstants {
    /// Percentage scale of first-level scripts.
    #[serde(default)]
    pub script_percent_scale_down: i16,
    /// Percentage scale of second-level scripts.
    #[serde(default)]
    pub script_script_percent_scale_down: i16,
    /// Minimum height of a delimited subformula.
    #[serde(default)]
    pub delimited_sub_formula_min_height: f64,
    /// Minimum height of a display-style operator.
    #[serde(default)]
    pub display_operator_min_height: f64,
    /// Extra space between mathematical formulas.
    #[serde(default)]
    pub math_leading: MathValue,
    /// Height of the mathematical axis above the baseline.
    #[serde(default)]
    pub axis_height: MathValue,
    /// Base height above which an accent is raised.
    #[serde(default)]
    pub accent_base_height: MathValue,
    /// Base height above which an accent is flattened.
    #[serde(default)]
    pub flattened_accent_base_height: MathValue,
    /// Subscript shift down in font units.
    #[serde(default)]
    pub subscript_shift_down: MathValue,
    /// Subscript top max in font units.
    #[serde(default)]
    pub subscript_top_max: MathValue,
    /// Subscript baseline drop min in font units.
    #[serde(default)]
    pub subscript_baseline_drop_min: MathValue,
    /// Superscript shift up in font units.
    #[serde(default)]
    pub superscript_shift_up: MathValue,
    /// Superscript shift up cramped in font units.
    #[serde(default)]
    pub superscript_shift_up_cramped: MathValue,
    /// Superscript bottom min in font units.
    #[serde(default)]
    pub superscript_bottom_min: MathValue,
    /// Superscript baseline drop max in font units.
    #[serde(default)]
    pub superscript_baseline_drop_max: MathValue,
    /// Sub superscript gap min in font units.
    #[serde(default)]
    pub sub_superscript_gap_min: MathValue,
    /// Superscript bottom max with subscript in font units.
    #[serde(default)]
    pub superscript_bottom_max_with_subscript: MathValue,
    /// Space after script in font units.
    #[serde(default)]
    pub space_after_script: MathValue,
    /// Upper limit gap min in font units.
    #[serde(default)]
    pub upper_limit_gap_min: MathValue,
    /// Upper limit baseline rise min in font units.
    #[serde(default)]
    pub upper_limit_baseline_rise_min: MathValue,
    /// Lower limit gap min in font units.
    #[serde(default)]
    pub lower_limit_gap_min: MathValue,
    /// Lower limit baseline drop min in font units.
    #[serde(default)]
    pub lower_limit_baseline_drop_min: MathValue,
    /// Stack top shift up in font units.
    #[serde(default)]
    pub stack_top_shift_up: MathValue,
    /// Stack top display style shift up in font units.
    #[serde(default)]
    pub stack_top_display_style_shift_up: MathValue,
    /// Stack bottom shift down in font units.
    #[serde(default)]
    pub stack_bottom_shift_down: MathValue,
    /// Stack bottom display style shift down in font units.
    #[serde(default)]
    pub stack_bottom_display_style_shift_down: MathValue,
    /// Stack gap min in font units.
    #[serde(default)]
    pub stack_gap_min: MathValue,
    /// Stack display style gap min in font units.
    #[serde(default)]
    pub stack_display_style_gap_min: MathValue,
    /// Stretch stack top shift up in font units.
    #[serde(default)]
    pub stretch_stack_top_shift_up: MathValue,
    /// Stretch stack bottom shift down in font units.
    #[serde(default)]
    pub stretch_stack_bottom_shift_down: MathValue,
    /// Stretch stack gap above min in font units.
    #[serde(default)]
    pub stretch_stack_gap_above_min: MathValue,
    /// Stretch stack gap below min in font units.
    #[serde(default)]
    pub stretch_stack_gap_below_min: MathValue,
    /// Fraction numerator shift up in font units.
    #[serde(default)]
    pub fraction_numerator_shift_up: MathValue,
    /// Fraction numerator display style shift up in font units.
    #[serde(default)]
    pub fraction_numerator_display_style_shift_up: MathValue,
    /// Fraction denominator shift down in font units.
    #[serde(default)]
    pub fraction_denominator_shift_down: MathValue,
    /// Fraction denominator display style shift down in font units.
    #[serde(default)]
    pub fraction_denominator_display_style_shift_down: MathValue,
    /// Fraction numerator gap min in font units.
    #[serde(default)]
    pub fraction_numerator_gap_min: MathValue,
    /// Fraction num display style gap min in font units.
    #[serde(default)]
    pub fraction_num_display_style_gap_min: MathValue,
    /// Fraction rule thickness in font units.
    #[serde(default)]
    pub fraction_rule_thickness: MathValue,
    /// Fraction denominator gap min in font units.
    #[serde(default)]
    pub fraction_denominator_gap_min: MathValue,
    /// Fraction denom display style gap min in font units.
    #[serde(default)]
    pub fraction_denom_display_style_gap_min: MathValue,
    /// Skewed fraction horizontal gap in font units.
    #[serde(default)]
    pub skewed_fraction_horizontal_gap: MathValue,
    /// Skewed fraction vertical gap in font units.
    #[serde(default)]
    pub skewed_fraction_vertical_gap: MathValue,
    /// Overbar vertical gap in font units.
    #[serde(default)]
    pub overbar_vertical_gap: MathValue,
    /// Overbar rule thickness in font units.
    #[serde(default)]
    pub overbar_rule_thickness: MathValue,
    /// Overbar extra ascender in font units.
    #[serde(default)]
    pub overbar_extra_ascender: MathValue,
    /// Underbar vertical gap in font units.
    #[serde(default)]
    pub underbar_vertical_gap: MathValue,
    /// Underbar rule thickness in font units.
    #[serde(default)]
    pub underbar_rule_thickness: MathValue,
    /// Underbar extra descender in font units.
    #[serde(default)]
    pub underbar_extra_descender: MathValue,
    /// Radical vertical gap in font units.
    #[serde(default)]
    pub radical_vertical_gap: MathValue,
    /// Radical display style vertical gap in font units.
    #[serde(default)]
    pub radical_display_style_vertical_gap: MathValue,
    /// Radical rule thickness in font units.
    #[serde(default)]
    pub radical_rule_thickness: MathValue,
    /// Radical extra ascender in font units.
    #[serde(default)]
    pub radical_extra_ascender: MathValue,
    /// Radical kern before degree in font units.
    #[serde(default)]
    pub radical_kern_before_degree: MathValue,
    /// Horizontal spacing after a radical degree; may be negative.
    #[serde(default)]
    pub radical_kern_after_degree: MathValue,
    /// Bottom of a radical degree as a percentage of the radical height.
    #[serde(default)]
    pub radical_degree_bottom_raise_percent: i16,
}

impl MathConstants {
    /// MathValueRecord field names in the order defined by MATH 1.0.
    pub const VALUE_NAMES: [&'static str; 51] = [
        "math_leading",
        "axis_height",
        "accent_base_height",
        "flattened_accent_base_height",
        "subscript_shift_down",
        "subscript_top_max",
        "subscript_baseline_drop_min",
        "superscript_shift_up",
        "superscript_shift_up_cramped",
        "superscript_bottom_min",
        "superscript_baseline_drop_max",
        "sub_superscript_gap_min",
        "superscript_bottom_max_with_subscript",
        "space_after_script",
        "upper_limit_gap_min",
        "upper_limit_baseline_rise_min",
        "lower_limit_gap_min",
        "lower_limit_baseline_drop_min",
        "stack_top_shift_up",
        "stack_top_display_style_shift_up",
        "stack_bottom_shift_down",
        "stack_bottom_display_style_shift_down",
        "stack_gap_min",
        "stack_display_style_gap_min",
        "stretch_stack_top_shift_up",
        "stretch_stack_bottom_shift_down",
        "stretch_stack_gap_above_min",
        "stretch_stack_gap_below_min",
        "fraction_numerator_shift_up",
        "fraction_numerator_display_style_shift_up",
        "fraction_denominator_shift_down",
        "fraction_denominator_display_style_shift_down",
        "fraction_numerator_gap_min",
        "fraction_num_display_style_gap_min",
        "fraction_rule_thickness",
        "fraction_denominator_gap_min",
        "fraction_denom_display_style_gap_min",
        "skewed_fraction_horizontal_gap",
        "skewed_fraction_vertical_gap",
        "overbar_vertical_gap",
        "overbar_rule_thickness",
        "overbar_extra_ascender",
        "underbar_vertical_gap",
        "underbar_rule_thickness",
        "underbar_extra_descender",
        "radical_vertical_gap",
        "radical_display_style_vertical_gap",
        "radical_rule_thickness",
        "radical_extra_ascender",
        "radical_kern_before_degree",
        "radical_kern_after_degree",
    ];

    /// Device-adjustable constants in their binary field order.
    pub fn values(&self) -> [&MathValue; 51] {
        [
            &self.math_leading,
            &self.axis_height,
            &self.accent_base_height,
            &self.flattened_accent_base_height,
            &self.subscript_shift_down,
            &self.subscript_top_max,
            &self.subscript_baseline_drop_min,
            &self.superscript_shift_up,
            &self.superscript_shift_up_cramped,
            &self.superscript_bottom_min,
            &self.superscript_baseline_drop_max,
            &self.sub_superscript_gap_min,
            &self.superscript_bottom_max_with_subscript,
            &self.space_after_script,
            &self.upper_limit_gap_min,
            &self.upper_limit_baseline_rise_min,
            &self.lower_limit_gap_min,
            &self.lower_limit_baseline_drop_min,
            &self.stack_top_shift_up,
            &self.stack_top_display_style_shift_up,
            &self.stack_bottom_shift_down,
            &self.stack_bottom_display_style_shift_down,
            &self.stack_gap_min,
            &self.stack_display_style_gap_min,
            &self.stretch_stack_top_shift_up,
            &self.stretch_stack_bottom_shift_down,
            &self.stretch_stack_gap_above_min,
            &self.stretch_stack_gap_below_min,
            &self.fraction_numerator_shift_up,
            &self.fraction_numerator_display_style_shift_up,
            &self.fraction_denominator_shift_down,
            &self.fraction_denominator_display_style_shift_down,
            &self.fraction_numerator_gap_min,
            &self.fraction_num_display_style_gap_min,
            &self.fraction_rule_thickness,
            &self.fraction_denominator_gap_min,
            &self.fraction_denom_display_style_gap_min,
            &self.skewed_fraction_horizontal_gap,
            &self.skewed_fraction_vertical_gap,
            &self.overbar_vertical_gap,
            &self.overbar_rule_thickness,
            &self.overbar_extra_ascender,
            &self.underbar_vertical_gap,
            &self.underbar_rule_thickness,
            &self.underbar_extra_descender,
            &self.radical_vertical_gap,
            &self.radical_display_style_vertical_gap,
            &self.radical_rule_thickness,
            &self.radical_extra_ascender,
            &self.radical_kern_before_degree,
            &self.radical_kern_after_degree,
        ]
    }

    /// Mutable device-adjustable constants in their binary field order.
    pub fn values_mut(&mut self) -> [&mut MathValue; 51] {
        [
            &mut self.math_leading,
            &mut self.axis_height,
            &mut self.accent_base_height,
            &mut self.flattened_accent_base_height,
            &mut self.subscript_shift_down,
            &mut self.subscript_top_max,
            &mut self.subscript_baseline_drop_min,
            &mut self.superscript_shift_up,
            &mut self.superscript_shift_up_cramped,
            &mut self.superscript_bottom_min,
            &mut self.superscript_baseline_drop_max,
            &mut self.sub_superscript_gap_min,
            &mut self.superscript_bottom_max_with_subscript,
            &mut self.space_after_script,
            &mut self.upper_limit_gap_min,
            &mut self.upper_limit_baseline_rise_min,
            &mut self.lower_limit_gap_min,
            &mut self.lower_limit_baseline_drop_min,
            &mut self.stack_top_shift_up,
            &mut self.stack_top_display_style_shift_up,
            &mut self.stack_bottom_shift_down,
            &mut self.stack_bottom_display_style_shift_down,
            &mut self.stack_gap_min,
            &mut self.stack_display_style_gap_min,
            &mut self.stretch_stack_top_shift_up,
            &mut self.stretch_stack_bottom_shift_down,
            &mut self.stretch_stack_gap_above_min,
            &mut self.stretch_stack_gap_below_min,
            &mut self.fraction_numerator_shift_up,
            &mut self.fraction_numerator_display_style_shift_up,
            &mut self.fraction_denominator_shift_down,
            &mut self.fraction_denominator_display_style_shift_down,
            &mut self.fraction_numerator_gap_min,
            &mut self.fraction_num_display_style_gap_min,
            &mut self.fraction_rule_thickness,
            &mut self.fraction_denominator_gap_min,
            &mut self.fraction_denom_display_style_gap_min,
            &mut self.skewed_fraction_horizontal_gap,
            &mut self.skewed_fraction_vertical_gap,
            &mut self.overbar_vertical_gap,
            &mut self.overbar_rule_thickness,
            &mut self.overbar_extra_ascender,
            &mut self.underbar_vertical_gap,
            &mut self.underbar_rule_thickness,
            &mut self.underbar_extra_descender,
            &mut self.radical_vertical_gap,
            &mut self.radical_display_style_vertical_gap,
            &mut self.radical_rule_thickness,
            &mut self.radical_extra_ascender,
            &mut self.radical_kern_before_degree,
            &mut self.radical_kern_after_degree,
        ]
    }
}
