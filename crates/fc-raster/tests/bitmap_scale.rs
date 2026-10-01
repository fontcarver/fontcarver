//! Cross-strike pixel rescaling: both filters, depth conversion and limits.

use fc_model::Bitmap;
use fc_raster::{scale_bitmap, BitmapScaleFilter};

fn mask(rows: &[&str], depth: u8) -> Bitmap {
    let height = rows.len() as u16;
    let width = rows.first().map_or(0, |row| row.len()) as u16;
    let mut bitmap = Bitmap::new(width, height, depth).expect("bitmap");
    let max = (1u32 << depth) - 1;
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.bytes().enumerate() {
            if cell == b'#' {
                bitmap
                    .set_pixel(x as u16, y as u16, max)
                    .expect("set pixel");
            }
        }
    }
    bitmap
}

fn coverages(bitmap: &Bitmap) -> Vec<Vec<u8>> {
    (0..bitmap.height)
        .map(|y| (0..bitmap.width).map(|x| bitmap.coverage(x, y)).collect())
        .collect()
}

#[test]
fn nearest_doubling_repeats_every_source_pixel_exactly() {
    let source = mask(&["#.", ".#"], 1);
    let scaled = scale_bitmap(&source, 4, 4, 1, BitmapScaleFilter::Nearest).expect("scale");
    assert_eq!(scaled.depth, 1);
    assert_eq!((scaled.width, scaled.height), (4, 4));
    assert_eq!(
        coverages(&scaled),
        vec![
            vec![255, 255, 0, 0],
            vec![255, 255, 0, 0],
            vec![0, 0, 255, 255],
            vec![0, 0, 255, 255],
        ]
    );
}

#[test]
fn nearest_halving_samples_destination_pixel_centres() {
    // Each destination pixel centre falls on the odd source column/row.
    let source = mask(&["....", ".#.#", "....", ".#.#"], 1);
    let scaled = scale_bitmap(&source, 2, 2, 1, BitmapScaleFilter::Nearest).expect("scale");
    assert_eq!(coverages(&scaled), vec![vec![255, 255], vec![255, 255]]);
}

#[test]
fn area_average_halving_averages_source_coverage() {
    let source = mask(&["##..", "##..", "#...", "#..."], 1);
    let scaled = scale_bitmap(&source, 2, 2, 8, BitmapScaleFilter::AreaAverage).expect("scale");
    assert_eq!(scaled.depth, 8);
    assert_eq!(coverages(&scaled), vec![vec![255, 0], vec![128, 0]]);
}

#[test]
fn area_average_of_half_covered_cell_rounds_to_mid_coverage() {
    let source = mask(&["#."], 1);
    let scaled = scale_bitmap(&source, 1, 1, 8, BitmapScaleFilter::AreaAverage).expect("scale");
    assert_eq!(coverages(&scaled), vec![vec![128]]);
    // The same average thresholds to ink in a monochrome destination.
    let mono = scale_bitmap(&source, 1, 1, 1, BitmapScaleFilter::AreaAverage).expect("scale");
    assert_eq!(mono.pixel(0, 0), 1);
    // Just under half coverage stays blank in monochrome.
    let thin = mask(&["#..."], 1);
    let blank = scale_bitmap(&thin, 1, 1, 1, BitmapScaleFilter::AreaAverage).expect("scale");
    assert_eq!(blank.pixel(0, 0), 0);
}

#[test]
fn area_average_upscaling_replicates_whole_pixels() {
    let source = mask(&["#"], 1);
    for filter in [BitmapScaleFilter::Nearest, BitmapScaleFilter::AreaAverage] {
        let scaled = scale_bitmap(&source, 3, 3, 8, filter).expect("scale");
        assert_eq!(coverages(&scaled), vec![vec![255; 3]; 3], "{filter:?}");
    }
}

#[test]
fn coverage_to_rgba_becomes_black_with_the_source_alpha() {
    let mut source = Bitmap::new(2, 1, 4).expect("bitmap");
    source.set_pixel(0, 0, 15).expect("set");
    source.set_pixel(1, 0, 5).expect("set");
    for filter in [BitmapScaleFilter::Nearest, BitmapScaleFilter::AreaAverage] {
        let scaled = scale_bitmap(&source, 2, 1, 32, filter).expect("scale");
        assert_eq!(scaled.pixel(0, 0), 0x0000_00ff, "{filter:?}");
        assert_eq!(scaled.pixel(1, 0), u32::from(source.coverage(1, 0)));
    }
}

#[test]
fn rgba_area_average_uses_straight_alpha_without_darkening() {
    let mut source = Bitmap::new(2, 1, 32).expect("bitmap");
    source.set_pixel(0, 0, 0xff00_00ff).expect("set");
    source.set_pixel(1, 0, 0xff00_0000).expect("set");
    let scaled = scale_bitmap(&source, 1, 1, 32, BitmapScaleFilter::AreaAverage).expect("scale");
    let pixel = scaled.pixel(0, 0).to_be_bytes();
    // A fully transparent neighbour halves the alpha and keeps the hue.
    assert_eq!(pixel[3], 128);
    assert_eq!([pixel[0], pixel[1], pixel[2]], [255, 0, 0]);
}

#[test]
fn rgba_to_coverage_uses_alpha_only() {
    let mut source = Bitmap::new(1, 1, 32).expect("bitmap");
    source.set_pixel(0, 0, 0x00ff_0080).expect("set");
    let scaled = scale_bitmap(&source, 1, 1, 8, BitmapScaleFilter::Nearest).expect("scale");
    assert_eq!(scaled.pixel(0, 0), 128);
}

#[test]
fn empty_sources_and_sizes_produce_empty_bitmaps() {
    let source = mask(&["#"], 1);
    for filter in [BitmapScaleFilter::Nearest, BitmapScaleFilter::AreaAverage] {
        let empty = scale_bitmap(&source, 0, 4, 1, filter).expect("scale");
        assert_eq!((empty.width, empty.height, empty.rows.len()), (0, 4, 0));
        let none = Bitmap::new(0, 0, 1).expect("bitmap");
        let grown = scale_bitmap(&none, 2, 2, 8, filter).expect("scale");
        assert_eq!(coverages(&grown), vec![vec![0, 0], vec![0, 0]]);
    }
}

#[test]
fn invalid_depth_dimensions_and_malformed_sources_are_rejected() {
    let source = mask(&["#"], 1);
    for filter in [BitmapScaleFilter::Nearest, BitmapScaleFilter::AreaAverage] {
        assert!(
            scale_bitmap(&source, 2, 2, 3, filter).is_err(),
            "{filter:?}"
        );
        assert!(scale_bitmap(&source, 5000, 2, 1, filter).is_err());
        assert!(scale_bitmap(&source, 4096, 4096, 32, filter).is_err());
        let mut broken = source.clone();
        broken.rows.clear();
        assert!(scale_bitmap(&broken, 2, 2, 1, filter).is_err());
    }
}
