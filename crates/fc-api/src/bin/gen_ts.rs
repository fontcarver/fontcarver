//! Generates the TypeScript API contract consumed by the frontend.

use std::{collections::BTreeSet, error::Error, fs, path::Path};

use fc_api::*;
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/api/generated");
    fs::create_dir_all(&output_dir)?;
    remove_old_typescript(&output_dir)?;

    let config = Config::new()
        .with_out_dir(&output_dir)
        .with_large_int("number");
    export_api_types(&config)?;
    // ts-rs cannot infer imports referenced only by `#[ts(type = ...)]` overrides,
    // so repair the affected generated files explicitly.
    add_imports(
        &output_dir.join("Command.ts"),
        &[
            "Anchor",
            "Component",
            "Contour",
            "FontInfo",
            "Glyph",
            "Guide",
            "Layer",
            "LayerKind",
            "Master",
            "MasterMetrics",
            "MetricsKeys",
            "KernSide",
            "Features",
            "FeatureGenerationSettings",
        ],
    )?;
    repair_command_double_option(&output_dir.join("Command.ts"))?;
    add_imports(&output_dir.join("Kerning.ts"), &["KernPair"])?;
    write_methods(&output_dir)?;
    write_index(&output_dir)?;
    Ok(())
}

fn repair_command_double_option(path: &Path) -> Result<(), std::io::Error> {
    let contents = fs::read_to_string(path)?;
    let source = "color_label: number | null | null,";
    let replacement = "color_label?: number | null,";
    if !contents.contains(source) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "cannot find SetGlyphMeta color_label declaration in `{}`",
                path.display()
            ),
        ));
    }
    fs::write(path, contents.replacen(source, replacement, 1))
}

fn export_api_types(config: &Config) -> Result<(), ts_rs::ExportError> {
    FindProblemsParams::export_all(config)?;
    FindProblemsResult::export_all(config)?;
    FeatureState::export_all(config)?;
    SetFeatureDraftParams::export_all(config)?;
    DiscardFeatureDraftParams::export_all(config)?;
    FeatureValidationParams::export_all(config)?;
    FeatureValidationResult::export_all(config)?;
    FeatureApplyResult::export_all(config)?;
    FreezeFeatureParams::export_all(config)?;
    LayoutInspectParams::export_all(config)?;
    LayoutInspection::export_all(config)?;
    LayoutRulesParams::export_all(config)?;
    LayoutRules::export_all(config)?;
    LayoutRuleGlyphsParams::export_all(config)?;
    LayoutRuleGlyphs::export_all(config)?;
    PrepareLayoutConversionParams::export_all(config)?;
    LayoutConversionPreview::export_all(config)?;
    ApplyLayoutConversionParams::export_all(config)?;
    RevertOriginalLayoutParams::export_all(config)?;
    KerningStatus::export_all(config)?;
    GlyphKerningParams::export_all(config)?;
    GlyphKerning::export_all(config)?;
    KerningResolveParams::export_all(config)?;
    KerningResolution::export_all(config)?;
    KerningPairsParams::export_all(config)?;
    KerningPairs::export_all(config)?;
    KerningGroupsParams::export_all(config)?;
    KerningGroupsResult::export_all(config)?;
    KerningMembersParams::export_all(config)?;
    KerningMembers::export_all(config)?;
    ApiError::export_all(config)?;
    OpenParams::export_all(config)?;
    FacesParams::export_all(config)?;
    NewParams::export_all(config)?;
    CloseParams::export_all(config)?;
    SaveParams::export_all(config)?;
    EmptyParams::export_all(config)?;
    MasterSummary::export_all(config)?;
    AxisSummary::export_all(config)?;
    DocWarning::export_all(config)?;
    DocSummary::export_all(config)?;
    Prefs::export_all(config)?;
    SetPrefsParams::export_all(config)?;
    DocParams::export_all(config)?;
    TableInfo::export_all(config)?;
    FontInfoView::export_all(config)?;
    SidebarCounts::export_all(config)?;
    GlyphFilter::export_all(config)?;
    GlyphSort::export_all(config)?;
    GlyphListParams::export_all(config)?;
    GlyphCell::export_all(config)?;
    GlyphListResult::export_all(config)?;
    GlyphIndexParams::export_all(config)?;
    GlyphCellParams::export_all(config)?;
    LayerPath::export_all(config)?;
    GlyphDetail::export_all(config)?;
    GlyphDetailParams::export_all(config)?;
    GlyphSearchParams::export_all(config)?;
    ApplyParams::export_all(config)?;
    EditBeginParams::export_all(config)?;
    EditTokenId::export_all(config)?;
    EditApplyParams::export_all(config)?;
    EditEndParams::export_all(config)?;
    Command::export_all(config)?;
    Change::export_all(config)?;
    DocumentEvent::export_all(config)?;
    ProgressEvent::export_all(config)?;
    Event::export_all(config)?;
    SnapPrefs::export_all(config)?;
    LayerOutlineParams::export_all(config)?;
    ComponentView::export_all(config)?;
    LayerOutline::export_all(config)?;
    FlattenedComponents::export_all(config)?;
    AddGlyphsParams::export_all(config)?;
    AddGlyphsResult::export_all(config)?;
    GlyphIdsParams::export_all(config)?;
    MetricsResolveParams::export_all(config)?;
    ResolvedMetrics::export_all(config)?;
    ShapeParams::export_all(config)?;
    ShapedGlyph::export_all(config)?;
    ShapedRun::export_all(config)?;
    ExportFormat::export_all(config)?;
    ExportOptions::export_all(config)?;
    ExportParams::export_all(config)?;
    ExportWarning::export_all(config)?;
    ExportReport::export_all(config)?;
    RecoveryEntry::export_all(config)?;
    RecoveryParams::export_all(config)?;
    fc_model::KernPair::export_all(config)?;
    Ok(())
}

fn remove_old_typescript(output_dir: &Path) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "ts") {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn add_imports(path: &Path, names: &[&str]) -> Result<(), std::io::Error> {
    let contents = fs::read_to_string(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!(
                "cannot repair imports in missing generated file `{}`: {error}",
                path.display()
            ),
        )
    })?;
    let imports = names
        .iter()
        .map(|name| format!("import type {{ {name} }} from \"./{name}\";\n"))
        .collect::<String>();
    let insertion = contents.find('\n').map_or(0, |index| index + 1);
    let mut repaired = String::with_capacity(contents.len() + imports.len() + 1);
    repaired.push_str(&contents[..insertion]);
    repaired.push_str(&imports);
    repaired.push_str(&contents[insertion..]);
    fs::write(path, repaired)
}

fn write_methods(output_dir: &Path) -> Result<(), std::io::Error> {
    let mut imports = BTreeSet::new();
    for (_, params, result) in METHODS {
        collect_type_names(params, &mut imports);
        collect_type_names(result, &mut imports);
    }
    let mut contents = generated_header();
    for name in imports {
        contents.push_str(&format!("import type {{ {name} }} from \"./{name}\";\n"));
    }
    contents.push_str("\nexport interface ApiMethods {\n");
    for (method, params, result) in METHODS {
        contents.push_str(&format!(
            "  '{method}': {{ params: {params}; result: {result} }};\n"
        ));
    }
    contents.push_str("}\n");
    fs::write(output_dir.join("methods.ts"), contents)
}

fn collect_type_names(expression: &str, names: &mut BTreeSet<String>) {
    for token in expression
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
    {
        if !matches!(token, "Array" | "null" | "number" | "string") {
            names.insert(token.to_owned());
        }
    }
}

fn write_index(output_dir: &Path) -> Result<(), std::io::Error> {
    let mut modules = Vec::new();
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "ts") {
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                if stem != "index" {
                    modules.push(stem.to_owned());
                }
            }
        }
    }
    modules.sort();
    let mut contents = generated_header();
    for module in modules {
        contents.push_str(&format!("export * from './{module}';\n"));
    }
    fs::write(output_dir.join("index.ts"), contents)
}

fn generated_header() -> String {
    "// This file is generated by fc-api. Do not edit it manually.\n".to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{add_imports, repair_command_double_option};

    #[test]
    fn import_repair_error_names_the_missing_generated_file() {
        let path = std::env::temp_dir().join(format!(
            "fontcarver-missing-generated-file-{}.ts",
            std::process::id()
        ));
        let error = add_imports(&path, &["Glyph"]).expect_err("missing patched file must fail");

        assert!(error.to_string().contains(&path.display().to_string()));
    }

    #[test]
    fn repairs_double_option_as_optional_nullable_typescript() {
        let path = std::env::temp_dir().join(format!(
            "fontcarver-command-binding-{}.ts",
            std::process::id()
        ));
        fs::write(&path, "color_label: number | null | null,").unwrap();

        repair_command_double_option(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "color_label?: number | null,"
        );
        fs::remove_file(path).ok();
    }
}
