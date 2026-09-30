//! Multiple-document collection export through the shared kernel endpoint.

use std::{path::PathBuf, sync::Arc};

use clap::Args as ClapArgs;
use fc_api::{Api, ExportFormat, ExportOptions, ExportParams, ExportReport};
use fc_cli::Error;
use fc_doc::Session;

use super::{api_call, build::print_report, open_in, CliFontOpener};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Destination .ttc or .otc collection.
    #[arg(short, long)]
    pub(crate) output: PathBuf,
    /// Input fonts, source documents or native documents, in face order.
    #[arg(required = true, num_args = 1..)]
    pub(crate) inputs: Vec<PathBuf>,
    /// Compile without preserving imported binary tables and instructions.
    #[arg(long)]
    pub(crate) no_preserve: bool,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let session = Arc::new(Session::with_opener(Arc::new(CliFontOpener::default())));
    let mut documents = Vec::new();
    for path in &args.inputs {
        if matches!(super::source_extension(path).as_str(), "ttc" | "otc") {
            let names: Vec<String> = api_call(
                &Api::new(Arc::clone(&session)),
                "session.faces",
                fc_api::FacesParams {
                    path: path.to_string_lossy().into_owned(),
                },
            )?;
            for index in 0..names.len() {
                documents.push(open_in(&session, path, Some(index as u32))?);
            }
        } else {
            documents.push(open_in(&session, path, None)?);
        }
    }
    // clap enforces at least one path; keep library-facing failure explicit.
    if documents.is_empty() {
        return Err(Error::UnsupportedOutput {
            extension: "collection requires at least one input".into(),
            supported: "ttc, otc",
        });
    }
    let doc = documents.remove(0);
    let api = Api::new(session);
    let report: ExportReport = api_call(
        &api,
        "session.export",
        ExportParams {
            doc,
            path: args.output.to_string_lossy().into_owned(),
            collection_docs: documents,
            options: ExportOptions {
                format: ExportFormat::Ttc,
                keep_tt_instructions: !args.no_preserve,
                preserve_untouched_tables: !args.no_preserve,
                ..ExportOptions::default()
            },
        },
    )?;
    print_report(&report);
    Ok(())
}
