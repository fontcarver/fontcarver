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
    /// Glyph name for a single-glyph EPS output.
    #[arg(long, value_name = "NAME")]
    pub(crate) glyph: Option<String>,
    /// Insert missing curve extrema in a compiled static export copy.
    #[arg(long)]
    pub(crate) add_extrema: bool,
    /// Round coordinates and metrics in a compiled static export copy.
    #[arg(long)]
    pub(crate) round_coordinates: bool,
    /// Expand components into contours in a compiled static export copy.
    #[arg(long)]
    pub(crate) decompose_components: bool,
    /// Use supplied production glyph names in compiled output.
    #[arg(long)]
    pub(crate) production_names: bool,
    /// Remove overlaps before compiling static output.
    #[arg(long)]
    pub(crate) remove_overlap: bool,
    /// Automatically hint compiled static output.
    #[arg(long)]
    pub(crate) autohint: bool,
    /// Share repeated charstrings in static OTF or bare CFF output.
    #[arg(long)]
    pub(crate) subroutinize: bool,
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
        glyph: args.glyph.map(Into::into),
        add_extrema: args.add_extrema,
        round_coordinates: args.round_coordinates,
        decompose_components: args.decompose_components,
        production_names: args.production_names,
        remove_overlap: args.remove_overlap,
        autohint: args.autohint,
        subroutinize: args.subroutinize,
        master: master_id(&opened, args.master.as_deref())?,
        ..ExportOptions::default()
    };
    let report = export(&opened, &args.output, options)?;
    print_report(&report);
    Ok(())
}

fn output_format(path: &Path) -> Result<ExportFormat, Error> {
    if is_fcf_path(path) {
        return Ok(ExportFormat::Fcf);
    }
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
        "svg" => Ok(ExportFormat::Svg),
        "eps" => Ok(ExportFormat::Eps),
        "afm" => Ok(ExportFormat::Afm),
        "pfm" => Ok(ExportFormat::Pfm),
        "ttx" => Ok(ExportFormat::Ttx),
        "ttc" => Ok(ExportFormat::Ttc),
        "cff" => Ok(ExportFormat::Cff),
        "t42" | "type42" => Ok(ExportFormat::Type42),
        "ufo" => Ok(ExportFormat::Ufo),
        "designspace" => Ok(ExportFormat::Designspace),
        "glyphs" => Ok(ExportFormat::Glyphs),
        "sfd" => Ok(ExportFormat::Sfd),
        "pfa" => Ok(ExportFormat::Pfa),
        "pfb" => Ok(ExportFormat::Pfb),
        extension => Err(Error::UnsupportedOutput {
            extension: extension.to_owned(),
            supported:
                "bdf, pcf, psf, fnt, fon, otb, fcf, fcf.gz, ttf, otf, woff, woff2, ufo, designspace, glyphs, sfd, pfa, pfb, svg, eps, afm, pfm, ttx, ttc, cff, t42, fcfp",
        }),
    }
}

fn is_fcf_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".fcfp")
        || name.ends_with(".fcf")
        || name.ends_with(".fcf.gz")
        // Documents written before the `.fwf` extension was renamed.
        || name.ends_with(".fwf")
        || name.ends_with(".fwf.gz")
}
