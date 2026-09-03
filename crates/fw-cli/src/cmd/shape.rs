//! Shaping of a text run with the document's own compiled font.

use std::path::PathBuf;

use clap::Args as ClapArgs;
use fw_api::{ShapeParams, ShapedRun};

use super::{api_call, build::master_id, open_document};
use fw_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontWorkshop document to shape with.
    pub(crate) input: PathBuf,
    /// Text to shape.
    pub(crate) text: String,
    /// Comma-separated feature tags; prefix a tag with `-` to switch it off.
    #[arg(long, value_name = "TAGS", allow_hyphen_values = true)]
    pub(crate) features: Option<String>,
    /// Master to shape with, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
    /// ISO 15924 script code.
    #[arg(long, value_name = "SCRIPT")]
    pub(crate) script: Option<String>,
    /// BCP 47 language tag.
    #[arg(long, value_name = "LANGUAGE")]
    pub(crate) language: Option<String>,
    /// Text direction: `ltr`, `rtl`, `ttb`, or `btt`.
    #[arg(long, value_name = "DIRECTION")]
    pub(crate) direction: Option<String>,
    /// Rendering size; positions stay in font units when omitted.
    #[arg(long, value_name = "SIZE")]
    pub(crate) size: Option<f64>,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.input, None)?;
    let run: ShapedRun = api_call(
        &opened.api,
        "text.shape",
        ShapeParams {
            doc: opened.doc,
            text: args.text,
            master: master_id(&opened, args.master.as_deref())?,
            features: features(args.features.as_deref()),
            script: args.script,
            language: args.language,
            direction: args.direction,
            size: args.size,
        },
    )?;
    for warning in &run.warnings {
        eprintln!("warning: {warning}");
    }
    if run.stale {
        eprintln!("warning: the shaping font is still being rebuilt");
    }
    for (glyph, name) in run.glyphs.iter().zip(&run.names) {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            glyph.gid,
            name.as_deref().unwrap_or_default(),
            glyph.x_advance,
            glyph.y_advance,
            glyph.x_offset,
            glyph.y_offset,
            glyph.cluster
        );
    }
    Ok(())
}

/// Parses `kern,-liga` into explicit feature switches.
fn features(list: Option<&str>) -> Vec<(String, bool)> {
    list.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| match tag.strip_prefix('-') {
            Some(tag) => (tag.to_owned(), false),
            None => (tag.to_owned(), true),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::features;

    #[test]
    fn parses_enabled_and_disabled_feature_tags() {
        assert_eq!(
            features(Some("kern, -liga ,")),
            vec![("kern".to_owned(), true), ("liga".to_owned(), false)]
        );
        assert!(features(None).is_empty());
    }
}
