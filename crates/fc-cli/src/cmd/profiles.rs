//! Inspection of the export configurations stored in a project.

use std::path::PathBuf;

use clap::Args as ClapArgs;
use fc_cli::Error;

use super::{build::stored_profiles, open_document};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontCarver document whose stored export configurations to list.
    pub(crate) input: PathBuf,
    /// Emit the stored configurations as JSON.
    #[arg(long)]
    pub(crate) json: bool,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.input, None)?;
    let stored = stored_profiles(&opened)?;
    if args.json {
        println!("{}", serde_json::to_string(&stored)?);
        return Ok(());
    }
    if stored.profiles.is_empty() {
        println!("no export profiles are stored in this document");
    }
    for profile in &stored.profiles {
        let mut parts = vec![format!("format={:?}", profile.options.format)];
        if !profile.subfolder.is_empty() {
            parts.push(format!("subfolder={}", profile.subfolder));
        }
        if !profile.file_name.is_empty() {
            parts.push(format!("file={}", profile.file_name));
        }
        for (label, enabled) in [
            ("variable", profile.options.variable),
            ("remove-overlap", profile.options.remove_overlap),
            ("add-extrema", profile.options.add_extrema),
            ("round-coordinates", profile.options.round_coordinates),
            ("decompose-components", profile.options.decompose_components),
            ("subroutinize", profile.options.subroutinize),
            ("production-names", profile.options.production_names),
            ("autohint", profile.options.autohint),
        ] {
            if enabled {
                parts.push(label.to_owned());
            }
        }
        println!("profile {}: {}", profile.name, parts.join(" "));
    }
    for config in &stored.variable_configs {
        let mut parts = Vec::new();
        if !config.file_name.is_empty() {
            parts.push(format!("file={}", config.file_name));
        }
        parts.push(format!(
            "masters={}",
            if config.masters.is_empty() {
                "all".to_owned()
            } else {
                config
                    .masters
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            }
        ));
        parts.push(format!(
            "axes={}",
            if config.axes.is_empty() {
                "all".to_owned()
            } else {
                config.axes.join(",")
            }
        ));
        parts.push(format!(
            "instances={}",
            if config.instances.is_empty() {
                "all".to_owned()
            } else {
                config.instances.join(",")
            }
        ));
        println!("variable-config {}: {}", config.name, parts.join(" "));
    }
    Ok(())
}
