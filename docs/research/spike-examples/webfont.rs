//! WOFF1/WOFF2 encoder+decoder crate evaluation.
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string()
    });
    let ttf = std::fs::read(&path).expect("read font");
    println!("input: {path} ({} bytes)\n", ttf.len());

    // ---------- woofwoof 1.0.2 : compress + decompress ----------
    // transform=false variant first, to test byte-exact glyf preservation
    if let Some(nt) = woofwoof::compress(&ttf, String::new(), 11, false) {
        println!("woofwoof::compress(transform=false) -> {} bytes ({:.1}%)", nt.len(), nt.len() as f64/ttf.len() as f64*100.0);
        if let Some(d) = woofwoof::decompress(&nt) { check_font("woofwoof no-transform roundtrip", &d, &ttf); }
    }
    let t = Instant::now();
    let w2 = woofwoof::compress(&ttf, String::new(), 11, true).expect("woofwoof::compress");
    let enc_ms = t.elapsed().as_millis();
    println!(
        "woofwoof::compress(q=11, transform=true) -> {} bytes ({:.1}% of orig) in {enc_ms} ms; magic={:?}",
        w2.len(),
        w2.len() as f64 / ttf.len() as f64 * 100.0,
        std::str::from_utf8(&w2[0..4])
    );

    let t = Instant::now();
    let back = woofwoof::decompress(&w2).expect("woofwoof::decompress");
    println!(
        "woofwoof::decompress -> {} bytes in {} ms",
        back.len(),
        t.elapsed().as_millis()
    );
    check_font("woofwoof roundtrip", &back, &ttf);

    // ---------- woff2-patched 0.4.0 : decompress only ----------
    match woff2_patched::convert_woff2_to_ttf(&mut &w2[..]) {
        Ok(v) => { println!("\nwoff2_patched::convert_woff2_to_ttf -> {} bytes", v.len());
                   check_font("woff2-patched decode", &v, &ttf); }
        Err(e) => println!("\nwoff2-patched decode FAILED: {e:?}"),
    }

    // ---------- wuff 0.2.9 : decompress only ----------
    match wuff::decompress_woff2(&w2) {
        Ok(v) => { println!("\nwuff::decompress_woff2 -> {} bytes", v.len());
                   check_font("wuff decode", &v, &ttf); }
        Err(e) => println!("\nwuff decode FAILED: {e:?}"),
    }

    // ---------- woff (bodoni) 0.6.3 ----------
    // version1::compress / version2::compress ?
    let t = Instant::now();
    match woff::version2::compress(&ttf, String::new(), 11, true) {
        Some(v) => println!("\nwoff::version2::compress -> {} bytes in {} ms", v.len(), t.elapsed().as_millis()),
        None => println!("\nwoff::version2::compress returned None"),
    }
    let t = Instant::now();
    match woff::version1::compress(&ttf, 1, 0) {
        Some(v) => {
            println!("woff::version1::compress -> {} bytes ({:.1}%) in {} ms; magic={:?}",
                v.len(), v.len() as f64/ttf.len() as f64*100.0, t.elapsed().as_millis(),
                std::str::from_utf8(&v[0..4]));
            match woff::version1::decompress(&v) {
                Some(d) => { println!("woff::version1::decompress -> {} bytes", d.len());
                             check_font("woff1 roundtrip", &d, &ttf); }
                None => println!("woff::version1::decompress returned None"),
            }
            match wuff::decompress_woff1(&v) {
                Ok(d) => { println!("wuff::decompress_woff1 -> {} bytes", d.len());
                           check_font("wuff woff1 decode", &d, &ttf); }
                Err(e) => println!("wuff::decompress_woff1 FAILED: {e:?}"),
            }
        }
        None => println!("woff::version1::compress returned None"),
    }
}

fn check_font(label: &str, bytes: &[u8], orig: &[u8]) {
    use read_fonts::{FontRef, TableProvider};
    match FontRef::new(bytes) {
        Ok(f) => {
            let n = f.table_directory().table_records().len();
            let tags: Vec<String> = f
                .table_directory()
                .table_records()
                .iter()
                .map(|r| r.tag().to_string())
                .collect();
            let o = FontRef::new(orig).unwrap();
            let otags: Vec<String> = o
                .table_directory()
                .table_records()
                .iter()
                .map(|r| r.tag().to_string())
                .collect();
            let ng = f.maxp().map(|m| m.num_glyphs()).unwrap_or(0);
            let og = o.maxp().map(|m| m.num_glyphs()).unwrap_or(0);
            // compare glyf bytes exactly
            let same_glyf = f.table_data(read_fonts::types::Tag::new(b"glyf")).map(|d| d.as_bytes().to_vec())
                == o.table_data(read_fonts::types::Tag::new(b"glyf")).map(|d| d.as_bytes().to_vec());
            let mut missing: Vec<&String> = otags.iter().filter(|t| !tags.contains(t)).collect();
            missing.sort();
            println!(
                "  [{label}] parses OK: {n} tables (orig {}), num_glyphs {ng} (orig {og}), glyf bytes identical: {same_glyf}, tables lost: {missing:?}",
                otags.len()
            );
        }
        Err(e) => println!("  [{label}] FontRef::new FAILED: {e:?}"),
    }
}
