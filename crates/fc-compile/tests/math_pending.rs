use fc_compile::{export, ExportOptions};
use fc_model::{Font, MathTable};

#[test]
fn unfinished_math_integration_never_silently_drops_authored_data() {
    for math in [Some(MathTable::default()), None] {
        let font = Font {
            math,
            math_table_owned: true,
            ..Default::default()
        };
        let error = export(&font, &|_| 0, &ExportOptions::default()).unwrap_err();
        assert!(error.to_string().contains("editable MATH"), "{error}");
    }
}
