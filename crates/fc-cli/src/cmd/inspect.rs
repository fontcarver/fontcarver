use std::path::PathBuf;

use clap::Args as ClapArgs;
use fc_api::{DocParams, DocSummary};
use serde::Serialize;

use super::{api_call, open_document};
use fc_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or Font Cutter document to inspect.
    pub(crate) font: PathBuf,
    /// Zero-based face index for a font collection.
    #[arg(long)]
    pub(crate) face: Option<u32>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Serialize)]
struct Output {
    summary: DocSummary,
    tables: Vec<Table>,
}

#[derive(Serialize)]
struct Table {
    tag: String,
    len: usize,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.font, args.face)?;
    let mut summary: DocSummary =
        api_call(&opened.api, "doc.summary", DocParams { doc: opened.doc })?;
    summary.warnings = opened.warnings;
    let tables = opened.session.with_doc(opened.doc, |document| {
        document
            .font
            .extra
            .raw_tables
            .iter()
            .map(|(tag, bytes)| Table {
                tag: tag.to_string(),
                len: bytes.len(),
            })
            .collect::<Vec<_>>()
    })?;
    let output = Output { summary, tables };

    if args.json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        print_text(&output);
    }
    Ok(())
}

fn print_text(output: &Output) {
    println!("family: {}", output.summary.family_name);
    println!("style: {}", output.summary.style_name);
    println!("upem: {}", output.summary.units_per_em);
    println!("glyphs: {}", output.summary.glyph_count);
    println!("masters: {}", output.summary.masters.len());
    println!("axes: {}", output.summary.axes.len());
    println!("format: {}", output.summary.format);
    if output.summary.warnings.is_empty() {
        println!("warnings: none");
    } else {
        println!(
            "warnings: {}",
            output
                .summary
                .warnings
                .iter()
                .map(|warning| format!("{}: {}", warning.code, warning.message))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    println!("tables:");
    for table in &output.tables {
        println!("  {}: {} bytes, preserved", table.tag, table.len);
    }
}
