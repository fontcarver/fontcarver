use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod build_unihan;

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn main() -> BuildResult<()> {
    for input in [
        "data/aglfn.txt",
        "data/Blocks.txt",
        "data/GB2312.TXT",
        "data/BIG5.TXT",
        "data/JIS0208.TXT",
        "data/KSX1001.TXT",
        "data/GB18030-2022-Level1.txt",
    ] {
        println!("cargo:rerun-if-changed={input}");
    }

    let output_dir = PathBuf::from(env::var("OUT_DIR")?);
    build_unihan::generate(&output_dir)?;
    generate_aglfn(&output_dir.join("aglfn.rs"))?;
    generate_unicode_tables(&output_dir.join("unicode_tables.rs"))?;
    Ok(())
}

fn generate_aglfn(output: &Path) -> BuildResult<()> {
    let input = read_data("data/aglfn.txt")?;
    let mut by_codepoint = Vec::new();

    for (line_index, line) in input.lines().enumerate() {
        let line = data_part(line);
        if line.is_empty() {
            continue;
        }

        let mut fields = line.split(';');
        let cp = parse_hex(required_field(fields.next(), "code point", line_index)?)?;
        let name = required_field(fields.next(), "glyph name", line_index)?;
        if name.is_empty() {
            return Err(invalid_data(line_index, "empty glyph name").into());
        }
        by_codepoint.push((cp, name));
    }

    by_codepoint.sort_unstable_by_key(|entry| entry.0);
    ensure_unique_by(
        &by_codepoint,
        |left, right| left.0 == right.0,
        "AGLFN code point",
    )?;

    let mut by_name = by_codepoint.clone();
    by_name.sort_unstable_by(|left, right| left.1.cmp(right.1));
    ensure_unique_by(
        &by_name,
        |left, right| left.1 == right.1,
        "AGLFN glyph name",
    )?;

    let mut generated = generated_header();
    generated.push_str("pub(super) static AGLFN_BY_CODEPOINT: &[(u32, &str)] = &[\n");
    for (cp, name) in by_codepoint {
        writeln!(generated, "    (0x{cp:X}, {name:?}),")?;
    }
    generated.push_str("];\n\n");

    generated.push_str("pub(super) static AGLFN_BY_NAME: &[(&str, u32)] = &[\n");
    for (cp, name) in by_name {
        writeln!(generated, "    ({name:?}, 0x{cp:X}),")?;
    }
    generated.push_str("];\n");
    fs::write(output, generated)?;
    Ok(())
}

fn generate_unicode_tables(output: &Path) -> BuildResult<()> {
    let blocks = parse_blocks("data/Blocks.txt")?;
    let gb2312 = parse_mapping_codepoints("data/GB2312.TXT", 1)?;
    let big5 = parse_mapping_codepoints("data/BIG5.TXT", 1)?;

    let mut generated = generated_header();
    generated.push_str("pub(super) static BLOCKS: &[(u32, u32, &str)] = &[\n");
    for (start, end, name) in &blocks {
        writeln!(generated, "    (0x{start:X}, 0x{end:X}, {name:?}),")?;
    }
    generated.push_str("];\n\n");

    generated.push_str("pub(super) static BLOCK_NAMES: &[&str] = &[\n    \"No Block\",\n");
    for (_, _, name) in &blocks {
        writeln!(generated, "    {name:?},")?;
    }
    generated.push_str("];\n\n");

    write_codepoint_array(&mut generated, "GB2312_CODEPOINTS", &gb2312)?;
    generated.push('\n');
    write_codepoint_array(&mut generated, "BIG5_CODEPOINTS", &big5)?;
    for (path, name, column) in [
        ("data/JIS0208.TXT", "JIS0208_CODEPOINTS", 2),
        ("data/KSX1001.TXT", "KSX1001_CODEPOINTS", 1),
    ] {
        write_codepoint_array(
            &mut generated,
            name,
            &parse_mapping_codepoints(path, column)?,
        )?;
    }
    let gb = read_data("data/GB18030-2022-Level1.txt")?;
    let gb: Vec<u32> = gb
        .lines()
        .map(data_part)
        .filter(|line| !line.is_empty())
        .map(parse_hex)
        .collect::<BuildResult<_>>()?;
    write_codepoint_array(&mut generated, "GB18030_LEVEL1_CODEPOINTS", &gb)?;
    fs::write(output, generated)?;
    Ok(())
}

fn parse_blocks(path: &str) -> BuildResult<Vec<(u32, u32, String)>> {
    let input = read_data(path)?;
    let mut blocks = Vec::new();

    for (line_index, line) in input.lines().enumerate() {
        let line = data_part(line);
        if line.is_empty() {
            continue;
        }

        let (range, name) = line
            .split_once(';')
            .ok_or_else(|| invalid_data(line_index, "block row has no semicolon"))?;
        let (start, end) = range
            .trim()
            .split_once("..")
            .ok_or_else(|| invalid_data(line_index, "block row has no range"))?;
        let start = parse_hex(start.trim())?;
        let end = parse_hex(end.trim())?;
        if start > end {
            return Err(invalid_data(line_index, "block range is reversed").into());
        }
        blocks.push((start, end, name.trim().to_owned()));
    }

    blocks.sort_unstable_by_key(|entry| entry.0);
    for pair in blocks.windows(2) {
        if let [left, right] = pair {
            if left.1 >= right.0 {
                return Err(
                    io::Error::new(io::ErrorKind::InvalidData, "overlapping blocks").into(),
                );
            }
        }
    }
    Ok(blocks)
}

fn parse_mapping_codepoints(path: &str, column: usize) -> BuildResult<Vec<u32>> {
    let input = read_data(path)?;
    let mut codepoints = Vec::new();

    for (line_index, line) in input.lines().enumerate() {
        let line = data_part(line);
        if line.is_empty() {
            continue;
        }

        let unicode = line
            .split_whitespace()
            .nth(column)
            .ok_or_else(|| invalid_data(line_index, "mapping row has no Unicode value"))?;
        codepoints.push(parse_prefixed_hex(unicode)?);
    }

    codepoints.sort_unstable();
    codepoints.dedup();
    Ok(codepoints)
}

fn write_codepoint_array(output: &mut String, name: &str, values: &[u32]) -> BuildResult<()> {
    writeln!(output, "pub(super) static {name}: &[u32] = &[")?;
    for chunk in values.chunks(12) {
        output.push_str("    ");
        for value in chunk {
            write!(output, "0x{value:X}, ")?;
        }
        output.push('\n');
    }
    output.push_str("];\n");
    Ok(())
}

fn data_part(line: &str) -> &str {
    line.split('#').next().unwrap_or_default().trim()
}

fn required_field<'a>(
    field: Option<&'a str>,
    label: &str,
    line_index: usize,
) -> BuildResult<&'a str> {
    field
        .map(str::trim)
        .ok_or_else(|| invalid_data(line_index, &format!("missing {label}")).into())
}

fn parse_prefixed_hex(value: &str) -> BuildResult<u32> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("U+"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing hexadecimal prefix"))?;
    parse_hex(digits)
}

fn parse_hex(value: &str) -> BuildResult<u32> {
    Ok(u32::from_str_radix(value, 16)?)
}

fn ensure_unique_by<T>(
    values: &[T],
    equal: impl Fn(&T, &T) -> bool,
    label: &str,
) -> BuildResult<()> {
    if values.windows(2).any(|pair| match pair {
        [left, right] => equal(left, right),
        _ => false,
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("duplicate {label} in generated table"),
        )
        .into());
    }
    Ok(())
}

fn invalid_data(line_index: usize, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("data line {}: {message}", line_index + 1),
    )
}

fn read_data(path: &str) -> BuildResult<String> {
    Ok(fs::read_to_string(path)?)
}

fn generated_header() -> String {
    "// @generated by build.rs from vendored data; do not edit.\n\n".to_owned()
}
