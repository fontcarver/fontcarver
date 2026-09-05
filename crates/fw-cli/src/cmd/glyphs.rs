use std::{path::PathBuf, str::FromStr};

use clap::Args as ClapArgs;
use fw_api::{GlyphFilter, GlyphListParams, GlyphListResult, GlyphSort};

use super::{api_call, open_document};
use fw_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or Font Cutter document whose glyphs should be listed.
    pub(crate) font: PathBuf,
    /// Filter expression: cat=, script=, block=, query=, or unencoded=true.
    #[arg(long = "filter")]
    filters: Vec<FilterArgument>,
    /// Maximum number of matching glyphs to print.
    #[arg(long, default_value_t = 100)]
    limit: u32,
}

#[derive(Clone, Debug)]
enum FilterArgument {
    Category(String),
    Script(String),
    Block(String),
    Query(String),
    Unencoded(bool),
}

impl FromStr for FilterArgument {
    type Err = Error;

    fn from_str(argument: &str) -> Result<Self, Self::Err> {
        let (key, value) = argument
            .split_once('=')
            .ok_or_else(|| Error::InvalidFilter(argument.to_owned()))?;
        match (key, value) {
            ("cat", value) if !value.is_empty() => Ok(Self::Category(value.to_owned())),
            ("script", value) if !value.is_empty() => Ok(Self::Script(value.to_owned())),
            ("block", value) if !value.is_empty() => Ok(Self::Block(value.to_owned())),
            ("query", value) => Ok(Self::Query(value.to_owned())),
            ("unencoded", value) => value
                .parse::<bool>()
                .map(Self::Unencoded)
                .map_err(|_| Error::InvalidFilter(argument.to_owned())),
            _ => Err(Error::InvalidFilter(argument.to_owned())),
        }
    }
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.font, None)?;
    let result: GlyphListResult = api_call(
        &opened.api,
        "glyphs.list",
        GlyphListParams {
            doc: opened.doc,
            master: None,
            filter: build_filter(args.filters),
            sort: GlyphSort::Order,
            offset: 0,
            limit: args.limit,
            include_paths: false,
        },
    )?;
    for glyph in result.items {
        let unicodes = glyph
            .unicodes
            .iter()
            .map(|unicode| format!("U+{unicode:04X}"))
            .collect::<Vec<_>>()
            .join(",");
        println!("{}\t{}\t{}", glyph.name, unicodes, glyph.width);
    }
    Ok(())
}

fn build_filter(arguments: Vec<FilterArgument>) -> GlyphFilter {
    let mut filter = GlyphFilter::default();
    for argument in arguments {
        match argument {
            FilterArgument::Category(value) => filter.categories.push(value),
            FilterArgument::Script(value) => filter.scripts.push(value),
            FilterArgument::Block(value) => filter.blocks.push(value),
            FilterArgument::Query(value) => filter.query = Some(value),
            FilterArgument::Unencoded(value) => filter.unencoded_only = value,
        }
    }
    filter
}
