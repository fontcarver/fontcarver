//! Browser-development transport for the FontCarver kernel.

#![warn(missing_docs)]

pub mod error;
mod server;

pub use error::Error;
pub use server::{router, serve, ServeOptions};
