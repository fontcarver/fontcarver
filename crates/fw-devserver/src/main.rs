//! Font Cutter browser-development server entry point.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use clap::Parser;
use fw_api::{Api, DocSummary};
use fw_devserver::{Error, ServeOptions};
use fw_doc::Session;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Parser)]
#[command(about = "Font Cutter browser-development API server")]
struct Args {
    /// TCP port to listen on.
    #[arg(long, default_value_t = 8787)]
    port: u16,
    /// Font or Font Cutter document to open at startup.
    #[arg(long = "open", value_name = "PATH")]
    open: Vec<PathBuf>,
    /// Default tracing filter when RUST_LOG is unset.
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&args.log_level))
        .map_err(|error| Error::Tracing(error.to_string()))?;
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .map_err(|error| Error::Tracing(error.to_string()))?;

    let api = Arc::new(Api::new(Arc::new(Session::new())));
    if let Err(error) = api.start_autosave() {
        tracing::warn!(%error, "could not start crash-recovery autosave");
    }
    for path in args.open {
        let summary: DocSummary = serde_json::from_value(api.call(
            "session.open",
            serde_json::json!({ "path": path, "face_index": null }),
        )?)?;
        tracing::info!(doc_id = summary.doc.0, path = ?summary.path, "opened startup font");
    }

    let address = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = TcpListener::bind(address).await?;
    tracing::info!(%address, "Font Cutter development server listening");
    fw_devserver::serve(listener, api, ServeOptions::default()).await
}
