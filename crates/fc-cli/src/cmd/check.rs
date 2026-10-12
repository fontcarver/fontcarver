use std::path::PathBuf;

use clap::Args as ClapArgs;
use fc_api::{FindProblemsParams, FindProblemsResult};
use fc_cli::Error;

use super::{api_call, open_document, print_problem};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontCarver document to check without changing it.
    pub(crate) font: PathBuf,
    /// Zero-based face index for a font collection.
    #[arg(long)]
    pub(crate) face: Option<u32>,
    /// Maximum reported issues (1 to 2000).
    #[arg(long, default_value = "200", value_parser = clap::value_parser!(u16).range(1..=2000))]
    pub(crate) limit: u16,
    /// Emit the generation-bound report as JSON.
    #[arg(long)]
    pub(crate) json: bool,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.font, args.face)?;
    let report: FindProblemsResult = api_call(
        &opened.api,
        "doc.find_problems",
        FindProblemsParams {
            doc: opened.doc,
            limit: Some(args.limit.into()),
        },
    )?;
    if args.json {
        println!("{}", serde_json::to_string(&report)?);
    } else {
        for issue in &report.issues {
            print_problem(issue);
        }
        println!(
            "{} problems detected; {} shown{}",
            report.total,
            report.issues.len(),
            if report.truncated { " (truncated)" } else { "" }
        );
    }
    if report.has_errors {
        Err(Error::ProblemsFound)
    } else {
        Ok(())
    }
}
