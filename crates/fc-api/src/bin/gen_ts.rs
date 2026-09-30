//! Generates the TypeScript API contract consumed by the frontend.

use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use fc_api::*;
use ts_rs::{Config, TS};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/api/generated");
    // A Vite dev server watches the generated contract while it runs, so the
    // directory must never be observed empty or half-written: every file is
    // written into a staging directory beside it, and that directory is swapped
    // into place only once the whole contract is complete.
    let staging_dir = sibling(&output_dir, ".generated.staging");
    let replaced_dir = sibling(&output_dir, ".generated.replaced");
    for leftover in [&staging_dir, &replaced_dir] {
        if leftover.exists() {
            fs::remove_dir_all(leftover)?;
        }
    }
    fs::create_dir_all(&staging_dir)?;
    let result = generate(&staging_dir)
        .and_then(|()| keep_non_generated_files(&output_dir, &staging_dir).map_err(Into::into))
        .and_then(|()| {
            swap_into_place(&staging_dir, &output_dir, &replaced_dir).map_err(Into::into)
        });
    if result.is_err() {
        // Leave the previous contract in place for the next run to overwrite.
        let _ = fs::remove_dir_all(&staging_dir);
    }
    result
}

/// The path of the directory named `name` beside `output_dir`.
fn sibling(output_dir: &Path, name: &str) -> PathBuf {
    match output_dir.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

/// Copies the files the generator does not produce (`.gitkeep`) into `staging`,
/// so swapping it into place does not drop them.
fn keep_non_generated_files(output_dir: &Path, staging: &Path) -> Result<(), std::io::Error> {
    if !output_dir.is_dir() {
        fs::write(staging.join(".gitkeep"), "\n")?;
        return Ok(());
    }
    let mut kept = false;
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        let is_typescript = path.extension().is_some_and(|extension| extension == "ts");
        if path.is_file() && !is_typescript {
            if let Some(name) = path.file_name() {
                fs::copy(&path, staging.join(name))?;
                kept = true;
            }
        }
    }
    if !kept {
        fs::write(staging.join(".gitkeep"), "\n")?;
    }
    Ok(())
}

/// Moves `staging` onto `target` through two renames in the same directory,
/// keeping the directory it replaces until the move has succeeded so a failure
/// restores the previous contract instead of leaving nothing behind.
fn swap_into_place(staging: &Path, target: &Path, previous: &Path) -> Result<(), std::io::Error> {
    let had_target = target.exists();
    if had_target {
        fs::rename(target, previous)?;
    }
    match fs::rename(staging, target) {
        Ok(()) => {
            if had_target {
                fs::remove_dir_all(previous)?;
            }
            Ok(())
        }
        Err(error) => {
            if had_target {
                fs::rename(previous, target)?;
            }
            Err(error)
        }
    }
}

/// Writes the whole TypeScript contract into `output_dir`.
fn generate(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let output_dir = output_dir.to_path_buf();
    let config = Config::new()
        .with_out_dir(&output_dir)
        .with_large_int("number");
    export_api_types(&config)?;
    // ts-rs cannot infer imports referenced only by `#[ts(type = ...)]` overrides,
    // so repair the affected generated files explicitly.
    add_imports(
        &output_dir.join("Command.ts"),
        &[
            "CidInfo",
            "Anchor",
            "Component",
            "Contour",
            "FontInfo",
            "Glyph",
            "GlyphConstruction",
            "Guide",
            "Image",
            "Layer",
            "LayerEffect",
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
    add_imports(
        &output_dir.join("BitmapCommand.ts"),
        &["BitmapStrike", "BitmapGlyph"],
    )?;
    add_imports(&output_dir.join("BitmapStrike.ts"), &["BitmapGlyph"])?;
    add_imports(&output_dir.join("NodeLinkCommand.ts"), &["NodeAxis"])?;
    add_imports(
        &output_dir.join("HintCommand.ts"),
        &["PsHints", "PsPrivate", "Zone", "Stem", "TtMasterData"],
    )?;
    add_imports(
        &output_dir.join("DesignSpaceCommand.ts"),
        &[
            "Axis",
            "BracketCondition",
            "Master",
            "Instance",
            "LayerKind",
            "SmartAxis",
            "VirtualMaster",
        ],
    )?;
    add_imports(&output_dir.join("Kerning.ts"), &["KernPair"])?;
    add_imports(
        &output_dir.join("KerningMaintenanceOperation.ts"),
        &["KernPair"],
    )?;
    add_imports(
        &output_dir.join("MathCommand.ts"),
        &["MathTable", "MathConstants", "MathGlyphInfo"],
    )?;
    add_imports(&output_dir.join("SvgCommand.ts"), &["SvgTable"])?;
    add_imports(&output_dir.join("RescaleUpmResult.ts"), &["Change"])?;
    add_imports(&output_dir.join("ColorCommand.ts"), &["ColorTable"])?;
    write_methods(&output_dir)?;
    let declarations = write_scripting_declarations(&output_dir)?;
    write_scripting_manifest(&output_dir, &declarations)?;
    write_smart_filter_fields(&output_dir)?;
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
    VariableProofParams::export_all(config)?;
    VariableProof::export_all(config)?;
    SmartFiltersGetParams::export_all(config)?;
    SmartFiltersSetParams::export_all(config)?;
    SmartFiltersResult::export_all(config)?;
    SmartFilterParseParams::export_all(config)?;
    SmartFilterParseResult::export_all(config)?;
    FontCompareParams::export_all(config)?;
    FontCompareResult::export_all(config)?;
    FontCompareGlyphParams::export_all(config)?;
    FontCompareGlyphResult::export_all(config)?;
    FontCompareFamilyParams::export_all(config)?;
    FontCompareFamilyResult::export_all(config)?;
    FontReferenceParams::export_all(config)?;
    FontReferenceResult::export_all(config)?;
    FontMergeParams::export_all(config)?;
    FontMergeResult::export_all(config)?;
    GlyphDataStateParams::export_all(config)?;
    GlyphDataState::export_all(config)?;
    SetGlyphDataOverlayParams::export_all(config)?;
    GenerateGlyphsParams::export_all(config)?;
    GeneratedGlyph::export_all(config)?;
    GenerateGlyphsResult::export_all(config)?;
    UpdateGlyphInfoParams::export_all(config)?;
    PasteSpecialParams::export_all(config)?;
    PasteSpecialResult::export_all(config)?;
    RelinkComponentsParams::export_all(config)?;
    AutomationScriptListParams::export_all(config)?;
    AutomationScriptList::export_all(config)?;
    AutomationScriptReadParams::export_all(config)?;
    AutomationScriptSource::export_all(config)?;
    AutomationSettings::export_all(config)?;
    AutomationSettingsParams::export_all(config)?;
    AutomationTransactionParams::export_all(config)?;
    AutomationTransaction::export_all(config)?;
    AutomationTransactionEndParams::export_all(config)?;
    AutomationTransactionEnd::export_all(config)?;
    AutomationPythonParams::export_all(config)?;
    AutomationPythonResult::export_all(config)?;
    TtHintStateParams::export_all(config)?;
    TtHintState::export_all(config)?;
    TtHintApplyParams::export_all(config)?;
    TtHintPreviewParams::export_all(config)?;
    TtHintPreview::export_all(config)?;
    HintStateParams::export_all(config)?;
    HintState::export_all(config)?;
    HintApplyParams::export_all(config)?;
    HintAnalyzeParams::export_all(config)?;
    HintAnalysis::export_all(config)?;
    VariationState::export_all(config)?;
    VariationApplyParams::export_all(config)?;
    VariationPreviewParams::export_all(config)?;
    VariationPreview::export_all(config)?;
    VariationShape::export_all(config)?;
    RescaleUpmParams::export_all(config)?;
    RescaleUpmResult::export_all(config)?;
    FontEditorSettings::export_all(config)?;
    EditorSettingsView::export_all(config)?;
    SetEditorSettingsParams::export_all(config)?;
    VariationShapeStatus::export_all(config)?;
    BitmapRasterParams::export_all(config)?;
    BitmapTraceParams::export_all(config)?;
    BitmapTransferParams::export_all(config)?;
    BitmapPixelComponentsParams::export_all(config)?;
    BitmapDetectParams::export_all(config)?;
    BitmapPixelDetection::export_all(config)?;
    BitmapReconstructParams::export_all(config)?;
    BitmapPreviewParams::export_all(config)?;
    BitmapPreviewLine::export_all(config)?;
    BitmapConfigureParams::export_all(config)?;
    BitmapState::export_all(config)?;
    BitmapGlyphParams::export_all(config)?;
    BitmapGlyphView::export_all(config)?;
    BitmapApplyParams::export_all(config)?;
    fc_model::BitmapStrike::export_all(config)?;
    fc_model::BitmapGlyph::export_all(config)?;
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
    SpacingMatrixParams::export_all(config)?;
    SpacingMatrix::export_all(config)?;
    // Registered here for task a's in-flight history/workspace methods: the
    // method table already names these types, so `methods.ts` does not compile
    // without them. Move them into that task's own block when it lands.
    DocumentHistory::export_all(config)?;
    HistoryGoParams::export_all(config)?;
    DocumentWorkspaces::export_all(config)?;
    SetWorkspacesParams::export_all(config)?;
    KerningStatus::export_all(config)?;
    KerningBoundariesParams::export_all(config)?;
    KerningBoundaries::export_all(config)?;
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
    OpticalBatchParams::export_all(config)?;
    OpticalBatchResult::export_all(config)?;
    CjkBatchParams::export_all(config)?;
    CjkBatchResult::export_all(config)?;
    CjkBatchExample::export_all(config)?;
    LayerEffectsParams::export_all(config)?;
    ImageTarget::export_all(config)?;
    ImageState::export_all(config)?;
    ImageImportParams::export_all(config)?;
    ImageSetParams::export_all(config)?;
    ImageTraceParams::export_all(config)?;
    ImageTraceResult::export_all(config)?;
    LayerConstructionsParams::export_all(config)?;
    LayerEffectsResult::export_all(config)?;
    EffectMaskParams::export_all(config)?;
    StyleBatchParams::export_all(config)?;
    StyleBatchResult::export_all(config)?;
    CjkFilter::export_all(config)?;
    fc_model::CidInfo::export_all(config)?;
    CidState::export_all(config)?;
    SvgState::export_all(config)?;
    SvgApplyParams::export_all(config)?;
    SvgDecodeParams::export_all(config)?;
    ColorState::export_all(config)?;
    ColorApplyParams::export_all(config)?;
    ColorDecodeParams::export_all(config)?;
    ColorGraphPreviewParams::export_all(config)?;
    ColorGraphPreview::export_all(config)?;
    ColorPreviewParams::export_all(config)?;
    ColorPreview::export_all(config)?;
    MathState::export_all(config)?;
    MathApplyParams::export_all(config)?;
    MathDecodeParams::export_all(config)?;
    MathPreviewParams::export_all(config)?;
    MathPreview::export_all(config)?;
    fc_model::MathTable::export_all(config)?;
    CidApplyParams::export_all(config)?;
    UnihanParams::export_all(config)?;
    UnihanInfo::export_all(config)?;
    GlyphFilter::export_all(config)?;
    GlyphSort::export_all(config)?;
    GlyphListParams::export_all(config)?;
    GlyphCell::export_all(config)?;
    GlyphListResult::export_all(config)?;
    GlyphList::export_all(config)?;
    GlyphListView::export_all(config)?;
    GlyphListsResult::export_all(config)?;
    RepertoireKind::export_all(config)?;
    CoverageRow::export_all(config)?;
    InventoryCoverageParams::export_all(config)?;
    InventoryCoverage::export_all(config)?;
    MissingMembersParams::export_all(config)?;
    MissingMembers::export_all(config)?;
    SetGlyphListsParams::export_all(config)?;
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
    ExportProfile::export_all(config)?;
    VariableExportConfig::export_all(config)?;
    ExportProfilesResult::export_all(config)?;
    SetExportProfilesParams::export_all(config)?;
    ResolveExportProfileParams::export_all(config)?;
    ResolvedExportProfile::export_all(config)?;
    RecoveryEntry::export_all(config)?;
    RecoveryParams::export_all(config)?;
    fc_model::KernPair::export_all(config)?;
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

/// Methods the scripting facade deliberately withholds from macros.
fn scriptable(method: &str) -> bool {
    !method.starts_with("automation.")
        && !matches!(method, "doc.edit_begin" | "doc.edit_apply" | "doc.edit_end")
}

/// One parameter of a script-callable method, read back from the generated type.
struct ParamsShape {
    /// `false` when the generated type is not a plain object and cannot be checked.
    closed: bool,
    fields: Vec<(String, String, bool, String)>,
}

/// Reads a generated `ts-rs` parameter type back into a checkable field list.
///
/// The console uses this to complete parameter names and to reject unknown or
/// missing keys before a macro runs, without bundling a TypeScript compiler.
fn params_shape(output_dir: &Path, name: &str) -> ParamsShape {
    let Ok(source) = fs::read_to_string(output_dir.join(format!("{name}.ts"))) else {
        return ParamsShape {
            closed: false,
            fields: Vec::new(),
        };
    };
    let Some(alias) = source.find(&format!("export type {name} = ")) else {
        return ParamsShape {
            closed: false,
            fields: Vec::new(),
        };
    };
    let body = &source[alias..];
    // A parameterless method's generated alias is an empty record, which is a
    // closed shape with no accepted keys.
    if body.contains("Record<symbol, never>") {
        return ParamsShape {
            closed: true,
            fields: Vec::new(),
        };
    }
    let Some(open) = body.find("= {") else {
        return ParamsShape {
            closed: false,
            fields: Vec::new(),
        };
    };
    let mut fields = Vec::new();
    let mut depth = 0usize;
    let mut doc = String::new();
    let mut in_doc = false;
    for line in body[open + 2..].lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/**") {
            in_doc = true;
            doc.clear();
            continue;
        }
        if in_doc {
            if trimmed.starts_with("*/") {
                in_doc = false;
            } else {
                let text = trimmed.trim_start_matches('*').trim();
                if !text.is_empty() {
                    if !doc.is_empty() {
                        doc.push(' ');
                    }
                    doc.push_str(text);
                }
            }
            continue;
        }
        if depth == 1 {
            if let Some((head, rest)) = trimmed.split_once(':') {
                let optional = head.ends_with('?');
                let field = head.trim_end_matches('?').trim();
                if !field.is_empty() && field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    let ty = rest
                        .trim_end_matches(|c: char| {
                            matches!(c, ',' | ';' | '}') || c.is_whitespace()
                        })
                        .trim()
                        .to_owned();
                    // A nullable field is `Option<T>` with `#[serde(default)]`
                    // in this workspace, so omitting it is accepted.
                    let optional = optional || ty.contains("| null");
                    fields.push((field.to_owned(), ty, optional, std::mem::take(&mut doc)));
                }
            }
        }
        depth = depth
            .saturating_add(trimmed.matches('{').count())
            .saturating_sub(trimmed.matches('}').count());
    }
    ParamsShape {
        // Only a shape whose every field was recognised may reject unknown keys.
        closed: !fields.is_empty(),
        fields,
    }
}

fn ts_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Writes the runtime scripting manifest and the shipped declaration text.
fn write_scripting_manifest(output_dir: &Path, declarations: &str) -> Result<(), std::io::Error> {
    let mut contents = generated_header();
    contents.push_str(
        r#"
/** One accepted key of a script-callable method's parameter object. */
export interface ScriptParameterField {
  readonly name: string
  readonly type: string
  readonly optional: boolean
  readonly doc: string
}

/** One kernel method, as the macro console completes and checks it. */
export interface ScriptMethodInfo {
  readonly method: string
  readonly params: string
  readonly result: string
  /** False for the methods the scripting facade withholds from macros. */
  readonly scriptable: boolean
  /** True when every parameter key is known and unknown keys may be rejected. */
  readonly closed: boolean
  readonly fields: readonly ScriptParameterField[]
}

/** Every kernel method, sorted by name. */
export const SCRIPT_METHODS: readonly ScriptMethodInfo[] = [
"#,
    );
    let mut methods: Vec<_> = METHODS.iter().collect();
    methods.sort_by_key(|(method, _, _)| *method);
    for (method, params, result) in methods {
        let shape = params_shape(output_dir, params);
        contents.push_str(&format!(
            "  {{ method: {}, params: {}, result: {}, scriptable: {}, closed: {}, fields: [",
            ts_string(method),
            ts_string(params),
            ts_string(result),
            scriptable(method),
            shape.closed,
        ));
        for (name, ty, optional, doc) in &shape.fields {
            contents.push_str(&format!(
                "{{ name: {}, type: {}, optional: {optional}, doc: {} }}, ",
                ts_string(name),
                ts_string(ty),
                ts_string(doc),
            ));
        }
        contents.push_str("] },\n");
    }
    contents.push_str("]\n\n");
    contents.push_str(
        "/** Path of the shipped ambient declaration file, relative to the app package. */\nexport const SCRIPTING_DECLARATIONS_FILE = 'src/api/generated/fontcarver.d.ts'\n\n",
    );
    contents.push_str(
        "/** The exact shipped declaration text, so the console can show and export it. */\nexport const SCRIPTING_DECLARATIONS = ",
    );
    contents.push_str(&format!("{}\n", ts_template(declarations)));
    fs::write(output_dir.join("scriptingManifest.ts"), contents)
}

/// Writes the smart-filter vocabulary so the builder cannot drift from the parser.
fn write_smart_filter_fields(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut contents = String::from(
        "// This file is generated from `fc-api`'s SMART_FILTER_FIELDS. Do not edit it manually.\n\n         /** What kind of literal a smart-filter field compares against. */\n         export type SmartFilterFieldKind = 'text' | 'number' | 'scalar' | 'boolean'\n\n         /** The complete predicate vocabulary the kernel parses, in builder order. */\n         export const SMART_FILTER_FIELDS: readonly {\n  readonly field: string\n           readonly type: SmartFilterFieldKind\n}[] = [\n",
    );
    for entry in fc_api::SMART_FILTER_FIELDS {
        let kind = match entry.kind {
            fc_api::SmartFilterFieldKind::Text => "text",
            fc_api::SmartFilterFieldKind::Number => "number",
            fc_api::SmartFilterFieldKind::Scalar => "scalar",
            fc_api::SmartFilterFieldKind::Boolean => "boolean",
        };
        contents.push_str(&format!(
            "  {{ field: {}, type: {} }},\n",
            ts_string(entry.field),
            ts_string(kind),
        ));
    }
    contents.push_str("]\n");
    fs::write(output_dir.join("smartFilterFields.ts"), contents)?;
    Ok(())
}

fn ts_template(value: &str) -> String {
    format!(
        "`{}`",
        value
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("${", "\\${")
    )
}

fn write_scripting_declarations(output_dir: &Path) -> Result<String, std::io::Error> {
    let mut contents = generated_header();
    contents.push_str(
        r#"import type { ApiMethods } from './methods';
import type { DocId } from './DocId';
import type { MasterId } from './MasterId';
import type { GlyphId } from './GlyphId';

/** Context captured when the user explicitly starts a macro. */
export interface FontCarverScriptContext {
  readonly doc: DocId | null;
  readonly master: MasterId | null;
  readonly glyph: GlyphId | null;
}

/** Typed RPC facade in the macro worker. Calls preserve kernel validation/undo. */
export interface FontCarverScriptApi {
  readonly context: FontCarverScriptContext;
  call<M extends Exclude<keyof ApiMethods, `automation.${string}` | 'doc.edit_begin' | 'doc.edit_apply' | 'doc.edit_end'>>(method: M, params: ApiMethods[M]['params']): Promise<ApiMethods[M]['result']>;
}

declare global {
  const fc: FontCarverScriptApi;
  const fontcarver: FontCarverScriptApi;
}
"#,
    );
    fs::write(output_dir.join("fontcarver.d.ts"), &contents)?;
    Ok(contents)
}

fn write_index(output_dir: &Path) -> Result<(), std::io::Error> {
    let mut modules = Vec::new();
    for entry in fs::read_dir(output_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "ts") {
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                if stem != "index" && !stem.ends_with(".d") {
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

    use super::{
        add_imports, params_shape, repair_command_double_option, scriptable, write_index,
        write_scripting_declarations, write_scripting_manifest,
    };

    #[test]
    fn parameter_shapes_are_read_back_from_generated_types() {
        let directory = tempfile::TempDir::new().unwrap();
        std::fs::write(
            directory.path().join("SampleParams.ts"),
            "export type SampleParams = { \n/**\n * Open document.\n */\ndoc: DocId, \nlabel?: string | null, };\n",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("EmptyParams.ts"),
            "export type EmptyParams = Record<symbol, never>;\n",
        )
        .unwrap();

        let sample = params_shape(directory.path(), "SampleParams");
        assert!(sample.closed);
        assert_eq!(
            sample
                .fields
                .iter()
                .map(|(name, ty, optional, doc)| (
                    name.as_str(),
                    ty.as_str(),
                    *optional,
                    doc.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("doc", "DocId", false, "Open document."),
                ("label", "string | null", true, ""),
            ]
        );
        let empty = params_shape(directory.path(), "EmptyParams");
        assert!(empty.closed && empty.fields.is_empty());
        let missing = params_shape(directory.path(), "AbsentParams");
        assert!(!missing.closed && missing.fields.is_empty());
    }

    #[test]
    fn the_scripting_manifest_lists_every_method_and_ships_the_declarations() {
        let directory = tempfile::TempDir::new().unwrap();
        let declarations = write_scripting_declarations(directory.path()).unwrap();
        write_scripting_manifest(directory.path(), &declarations).unwrap();
        let manifest = fs::read_to_string(directory.path().join("scriptingManifest.ts")).unwrap();

        for (method, params, result) in fc_api::METHODS {
            assert!(
                manifest.contains(&format!(
                    "method: '{method}', params: '{params}', result: '{result}'"
                )),
                "manifest is missing {method}"
            );
        }
        assert!(manifest.contains("method: 'automation.python_run', params: 'AutomationPythonParams', result: 'AutomationPythonResult', scriptable: false"));
        assert!(manifest.contains(
            "method: 'doc.apply', params: 'ApplyParams', result: 'Change', scriptable: true"
        ));
        assert!(manifest.contains("export const SCRIPTING_DECLARATIONS = `"));
        assert!(manifest.contains("declare global"));
        assert!(!scriptable("doc.edit_begin"));
        assert!(scriptable("doc.undo"));
    }

    #[test]
    fn scripting_declarations_are_ambient_and_not_a_runtime_index_export() {
        let directory = tempfile::TempDir::new().unwrap();
        write_scripting_declarations(directory.path()).unwrap();
        fs::write(
            directory.path().join("methods.ts"),
            "export interface ApiMethods {}\n",
        )
        .unwrap();
        write_index(directory.path()).unwrap();
        let declarations = fs::read_to_string(directory.path().join("fontcarver.d.ts")).unwrap();
        assert!(declarations.contains("declare global"));
        assert!(declarations.contains("const fc: FontCarverScriptApi"));
        assert!(declarations.contains("const fontcarver: FontCarverScriptApi"));
        assert!(declarations.contains("Exclude<keyof ApiMethods, `automation.${string}` | 'doc.edit_begin' | 'doc.edit_apply' | 'doc.edit_end'>"));
        assert_eq!(
            fs::read_to_string(directory.path().join("index.ts")).unwrap(),
            "// This file is generated by fc-api. Do not edit it manually.\nexport * from './methods';\n",
        );
    }

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
