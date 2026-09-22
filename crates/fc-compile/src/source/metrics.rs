//! Master metrics supplied to the compiler.
use super::*;

#[derive(Debug)]
pub(super) struct GlobalMetricsWork(pub(super) Arc<SourceData>);

impl Work<Context, WorkId, IrError> for GlobalMetricsWork {
    fn id(&self) -> WorkId {
        WorkId::GlobalMetrics
    }

    fn read_access(&self) -> Access<WorkId> {
        Access::Variant(WorkId::StaticMetadata)
    }

    fn exec(&self, context: &Context) -> Result<(), IrError> {
        let static_metadata = context.static_metadata.get();
        let info = &self.0.font.info;
        let mut builder = GlobalMetricsBuilder::new();
        for (master, location) in self.0.master_locations() {
            let metrics = &master.metrics;
            for (metric, value) in [
                (GlobalMetric::Ascender, Some(metrics.ascender)),
                (GlobalMetric::Descender, Some(metrics.descender)),
                (GlobalMetric::HheaAscender, metrics.hhea_ascender),
                (GlobalMetric::HheaDescender, metrics.hhea_descender),
                (GlobalMetric::HheaLineGap, metrics.hhea_line_gap),
                (GlobalMetric::Os2TypoAscender, metrics.typo_ascender),
                (GlobalMetric::Os2TypoDescender, metrics.typo_descender),
                (GlobalMetric::Os2TypoLineGap, metrics.typo_line_gap),
                (GlobalMetric::Os2WinAscent, metrics.win_ascent),
                (GlobalMetric::Os2WinDescent, metrics.win_descent),
                (GlobalMetric::CapHeight, Some(metrics.cap_height)),
                (GlobalMetric::XHeight, Some(metrics.x_height)),
                (GlobalMetric::VheaAscender, metrics.vhea_ascender),
                (GlobalMetric::VheaDescender, metrics.vhea_descender),
                (GlobalMetric::VheaLineGap, metrics.vhea_line_gap),
                (
                    GlobalMetric::CaretSlopeRise,
                    info.open_type_hhea_caret_slope_rise.map(f64::from),
                ),
                (
                    GlobalMetric::CaretSlopeRun,
                    info.open_type_hhea_caret_slope_run.map(f64::from),
                ),
                (
                    GlobalMetric::CaretOffset,
                    info.open_type_hhea_caret_offset.map(f64::from),
                ),
                (
                    GlobalMetric::VheaCaretSlopeRise,
                    info.open_type_vhea_caret_slope_rise.map(f64::from),
                ),
                (
                    GlobalMetric::VheaCaretSlopeRun,
                    info.open_type_vhea_caret_slope_run.map(f64::from),
                ),
                (
                    GlobalMetric::VheaCaretOffset,
                    info.open_type_vhea_caret_offset.map(f64::from),
                ),
                (
                    GlobalMetric::UnderlineThickness,
                    info.postscript_underline_thickness,
                ),
                (
                    GlobalMetric::UnderlinePosition,
                    info.postscript_underline_position,
                ),
                (
                    GlobalMetric::SubscriptXSize,
                    info.open_type_os2_subscript_x_size,
                ),
                (
                    GlobalMetric::SubscriptYSize,
                    info.open_type_os2_subscript_y_size,
                ),
                (
                    GlobalMetric::SubscriptXOffset,
                    info.open_type_os2_subscript_x_offset,
                ),
                (
                    GlobalMetric::SubscriptYOffset,
                    info.open_type_os2_subscript_y_offset,
                ),
                (
                    GlobalMetric::SuperscriptXSize,
                    info.open_type_os2_superscript_x_size,
                ),
                (
                    GlobalMetric::SuperscriptYSize,
                    info.open_type_os2_superscript_y_size,
                ),
                (
                    GlobalMetric::SuperscriptXOffset,
                    info.open_type_os2_superscript_x_offset,
                ),
                (
                    GlobalMetric::SuperscriptYOffset,
                    info.open_type_os2_superscript_y_offset,
                ),
                (
                    GlobalMetric::StrikeoutSize,
                    info.open_type_os2_strikeout_size,
                ),
                (
                    GlobalMetric::StrikeoutPosition,
                    info.open_type_os2_strikeout_position,
                ),
            ] {
                builder.set_if_some(metric, location.clone(), value);
            }
            builder.populate_defaults(
                &location,
                static_metadata.units_per_em,
                Some(metrics.x_height),
                Some(metrics.ascender),
                Some(metrics.descender),
                Some(metrics.italic_angle),
            );
        }
        context
            .global_metrics
            .set(builder.build(&static_metadata.axes)?);
        Ok(())
    }
}
