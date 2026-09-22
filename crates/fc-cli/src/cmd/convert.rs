//! Conversion of a font between the supported document and export formats.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use fc_api::{ExportFormat, ExportOptions};

use super::{
    build::{export, extension, master_id, print_report},
    open_document,
};
use fc_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font, UFO, designspace, or FontCarver document to read.
    pub(crate) input: PathBuf,
    /// Destination path; the extension selects the output format.
    pub(crate) output: PathBuf,
    /// Master to write, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
    /// Strike pixels per em for single-strike bitmap outputs.
    #[arg(long, requires = "depth")]
    pub(crate) ppem: Option<u16>,
    /// Strike depth (1, 2, 4, 8, 32), together with --ppem.
    #[arg(long, requires = "ppem")]
    pub(crate) depth: Option<u8>,
    /// Write legacy PSF1 instead of PSF2 for .psf outputs.
    #[arg(long)]
    pub(crate) psf1: bool,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.input, None)?;
    if is_fcf_path(&args.output) {
        return opened
            .session
            .with_doc_mut(opened.doc, |document| document.save(&args.output))
            .map_err(|source| Error::Save {
                path: args.output.clone(),
                source,
            })?
            .map_err(|source| Error::Save {
                path: args.output,
                source,
            });
    }
    let mut format = output_format(&args.output)?;
    if args.psf1 {
        if format != ExportFormat::Psf {
            return Err(Error::UnsupportedOutput {
                extension: "--psf1 requires a .psf output".into(),
                supported: "psf, psf.gz",
            });
        }
        format = ExportFormat::Psf1;
    }
    let options = ExportOptions {
        format,
        bitmap_strike: args.ppem.zip(args.depth),
        master: master_id(&opened, args.master.as_deref())?,
        ..ExportOptions::default()
    };
    let report = export(&opened, &args.output, options)?;
    print_report(&report);
    Ok(())
}

fn output_format(path: &Path) -> Result<ExportFormat, Error> {
    let inner = if extension(path) == "gz" {
        path.with_extension("")
    } else {
        path.to_path_buf()
    };
    match extension(&inner).as_str() {
        "pcf" => Ok(ExportFormat::Pcf),
        "psf" | "psfu" => Ok(ExportFormat::Psf),
        "fnt" => Ok(ExportFormat::Fnt),
        "fon" => Ok(ExportFormat::Fon),
        "otb" => Ok(ExportFormat::Otb),
        "bdf" => Ok(ExportFormat::Bdf),
        "ttf" => Ok(ExportFormat::Ttf),
        "otf" => Ok(ExportFormat::Otf),
        "woff" => Ok(ExportFormat::Woff),
        "woff2" => Ok(ExportFormat::Woff2),
        "ufo" => Ok(ExportFormat::Ufo),
        "designspace" => Ok(ExportFormat::Designspace),
        "glyphs" => Ok(ExportFormat::Glyphs),
        "sfd" => Ok(ExportFormat::Sfd),
        "pfa" => Ok(ExportFormat::Pfa),
        "pfb" => Ok(ExportFormat::Pfb),
        extension => Err(Error::UnsupportedOutput {
            extension: extension.to_owned(),
            supported:
                "bdf, pcf, psf, fnt, fon, otb, fcf, fcf.gz, ttf, otf, woff, woff2, ufo, designspace, glyphs, sfd, pfa, pfb",
        }),
    }
}

fn is_fcf_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".fcf")
        || name.ends_with(".fcf.gz")
        // Documents written before the `.fwf` extension was renamed.
        || name.ends_with(".fwf")
        || name.ends_with(".fwf.gz")
}
