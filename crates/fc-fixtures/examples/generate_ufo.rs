//! Regenerates the committed minimal UFO fixture.

use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    fc_fixtures::ufo::write_minimal(Path::new("fixtures/ufo/Minimal.ufo"))?;
    Ok(())
}
