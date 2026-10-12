//! Compile reviewed Unihan text shards into a compact sorted index and UTF-8 pool.
use std::{collections::BTreeMap, fs, io, path::Path};

pub(super) fn generate(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut records = BTreeMap::new();
    for shard in [
        "data/Unihan-17-plane0.tsv",
        "data/Unihan-17-plane2.tsv",
        "data/Unihan-17-plane3.tsv",
    ] {
        println!("cargo:rerun-if-changed={shard}");
        let input = fs::read_to_string(shard)?;
        for line in input
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let (cp, data) = line
                .split_once('\t')
                .ok_or_else(|| invalid("missing Unicode scalar"))?;
            let cp = u32::from_str_radix(cp, 16)?;
            if char::from_u32(cp).is_none()
                || data.split('\t').count() != 8
                || records.insert(cp, data.to_owned()).is_some()
            {
                return Err(invalid("invalid or duplicate Unihan row").into());
            }
        }
    }
    let mut index = Vec::new();
    let mut text = String::new();
    for (cp, data) in records {
        index.extend_from_slice(&cp.to_le_bytes());
        index.extend_from_slice(&u32::try_from(text.len())?.to_le_bytes());
        index.extend_from_slice(&u32::try_from(data.len())?.to_le_bytes());
        text.push_str(&data);
        text.push('\n');
    }
    fs::write(output.join("unihan.index"), index)?;
    fs::write(output.join("unihan.txt"), text)?;
    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
