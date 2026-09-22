//! Writes a web container and an independently decoded sfnt for acceptance checks.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: wrap_woff2 INPUT.ttf OUTPUT.woff2".into());
    }
    let source = std::fs::read(&args[0])?;
    let output = fc_compile::wrap_woff2(&source)?;
    std::fs::write(&args[1], &output)?;
    let decoded =
        woofwoof::decompress(&output).ok_or("reference WOFF2 decoder rejected the output")?;
    std::fs::write(
        std::path::Path::new(&args[1]).with_extension("decoded.ttf"),
        decoded,
    )?;
    Ok(())
}
