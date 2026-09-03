//! Browser-development transport for the FontWorkshop kernel.

pub mod error;
mod server;

pub use error::Error;
pub use server::{router, serve, ServeOptions};
