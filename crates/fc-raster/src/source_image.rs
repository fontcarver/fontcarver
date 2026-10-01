//! Bounded static PNG source images, polygon tracing and curve-fitted autotrace.

use crate::{autotrace::FitOptions, invalid, Error};
use fc_model::{Bitmap, Contour};
use kurbo::{flatten, Affine, Point};

/// Maximum compressed size of a source PNG, in bytes.
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_PIXELS: u32 = 1_000_000;
const MAX_DIMENSION: u32 = 4096;
const MAX_SIMPLIFY_POINTS: usize = 4096;
/// Largest minimum-area filter, in square source pixels.
const MAX_MIN_AREA: f64 = 1_048_576.0;

/// Everything one autotrace run may be asked to do.
///
/// `threshold` is the minimum alpha-composited darkness that counts as ink, 1
/// through 255. `tolerance` is the largest permitted deviation from the pixel
/// ring in source pixels, 0 through 16, where zero keeps the exact pixel edges.
/// `curves` chooses between cubic curve fitting and straight-line simplification.
/// `corner_angle` is the turn angle in degrees at or above which a structural
/// vertex stays a corner. `smoothing` low-pass filters the ring before fitting,
/// in source pixels. `min_area` drops rings smaller than that many square source
/// pixels, which removes scanner speckle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceOptions {
    /// Minimum alpha-composited dark coverage that counts as ink, 1..=255.
    pub threshold: u8,
    /// Largest permitted deviation from the pixel ring, in source pixels.
    pub tolerance: f64,
    /// Fit cubic curves; false simplifies to straight lines only.
    pub curves: bool,
    /// Turn angle in degrees at or above which a vertex stays a corner, 0..=180.
    pub corner_angle: f64,
    /// Low-pass radius applied before fitting, in source pixels, 0..=8.
    pub smoothing: f64,
    /// Smallest retained ring area in square source pixels, 0..=1048576.
    pub min_area: f64,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            threshold: 128,
            tolerance: 1.0,
            curves: true,
            corner_angle: 35.0,
            smoothing: 0.0,
            min_area: 0.0,
        }
    }
}

/// Traced geometry and the quality information a review needs before applying it.
#[derive(Clone, Debug)]
pub struct TraceOutcome {
    /// Placed contours, in source-ring order.
    pub contours: Vec<Contour>,
    /// Rings the minimum-area filter removed.
    pub dropped: u32,
    /// Largest fitted deviation from the pixel ring, in source pixels.
    pub max_error: f64,
    /// Whether exact pixel edges were kept because a fit changed the topology.
    pub fallback: bool,
}

/// Validate a complete static PNG and return its dimensions.
///
/// Source files are limited to 16 MiB, 4096 pixels per side and one million
/// pixels. Header bounds are checked before allocating decoder or pixel buffers.
pub fn image_dimensions(data: &[u8]) -> Result<(u32, u32), Error> {
    let image = decode(data)?;
    Ok((image.width, image.height))
}

/// Trace dark pixels into closed polygons in y-up coordinates, then place them.
///
/// Pixels are composited over white; ink has darkness coverage at least
/// `threshold`, which must be in 1..=255. `tolerance` is a
/// finite distance in source pixels, from zero (exact edges) through 16. Optional
/// polygon simplification preserves ring orientation and containment; ambiguous
/// topology and paths above the simplification work budget retain exact edges.
pub fn trace_image(
    data: &[u8],
    transform: Affine,
    threshold: u8,
    tolerance: f64,
) -> Result<Vec<Contour>, Error> {
    trace_image_with_options(
        data,
        transform,
        &TraceOptions {
            threshold,
            tolerance,
            curves: false,
            ..Default::default()
        },
    )
    .map(|outcome| outcome.contours)
}

/// Trace dark pixels with curve fitting or polygon simplification, then place them.
///
/// Rings keep the orientation and containment of the exact pixel boundaries:
/// outside contours clockwise in font coordinates, holes counterclockwise. When a
/// fitted or simplified result would change that relationship or cross itself,
/// the exact pixel edges are kept and the outcome says so. Work is bounded by the
/// decoder limits, the pixel-edge budget, 4096 compared polygon points and the
/// per-ring fitting budget.
pub fn trace_image_with_options(
    data: &[u8],
    transform: Affine,
    options: &TraceOptions,
) -> Result<TraceOutcome, Error> {
    if options.threshold == 0 {
        return Err(invalid("image ink threshold must be at least 1"));
    }
    if !options.tolerance.is_finite() || !(0.0..=16.0).contains(&options.tolerance) {
        return Err(invalid("image simplification tolerance must be in 0..=16"));
    }
    if !options.corner_angle.is_finite() || !(0.0..=180.0).contains(&options.corner_angle) {
        return Err(invalid("image corner angle must be in 0..=180 degrees"));
    }
    if !options.smoothing.is_finite() || !(0.0..=8.0).contains(&options.smoothing) {
        return Err(invalid("image smoothing must be in 0..=8 source pixels"));
    }
    if !options.min_area.is_finite() || !(0.0..=MAX_MIN_AREA).contains(&options.min_area) {
        return Err(invalid(
            "image minimum area must be in 0..=1048576 square source pixels",
        ));
    }
    if transform.as_coeffs().iter().any(|value| !value.is_finite())
        || !transform.determinant().is_finite()
        || transform.determinant() == 0.0
    {
        return Err(invalid("image transform must be finite and invertible"));
    }
    let threshold = options.threshold;
    let tolerance = options.tolerance;
    let image = decode(data)?;
    let mut bitmap = Bitmap::new(image.width as u16, image.height as u16, 1)?;
    let channels = image.color.samples();
    for (index, pixel) in image.pixels.chunks_exact(channels).enumerate() {
        let (red, green, blue, alpha) = match image.color {
            png::ColorType::Grayscale => (pixel[0], pixel[0], pixel[0], 255),
            png::ColorType::GrayscaleAlpha => (pixel[0], pixel[0], pixel[0], pixel[1]),
            png::ColorType::Rgb => (pixel[0], pixel[1], pixel[2], 255),
            png::ColorType::Rgba => (pixel[0], pixel[1], pixel[2], pixel[3]),
            png::ColorType::Indexed => return Err(invalid("PNG palette was not expanded")),
        };
        // Integer Rec.709 luminance, followed by straight-alpha over white.
        let luminance =
            (54 * u32::from(red) + 183 * u32::from(green) + 19 * u32::from(blue) + 128) / 256;
        let composited =
            (luminance * u32::from(alpha) + 255 * (255 - u32::from(alpha)) + 127) / 255;
        if 255 - composited >= u32::from(threshold) {
            bitmap.set_pixel(
                (index % image.width as usize) as u16,
                (index / image.width as usize) as u16,
                1,
            )?;
        }
    }
    let mut exact = crate::trace_bitmap(&bitmap, 1, 1, 1)?;
    let before = exact.len();
    if options.min_area > 0.0 {
        exact.retain(|contour| signed_area(&model_ring(contour)).abs() >= options.min_area);
    }
    let dropped = (before - exact.len()) as u32;
    let (traced, max_error, fallback) = if options.curves && tolerance > 0.0 {
        fit_curves(&exact, options)
    } else {
        (simplify(&exact, tolerance), 0.0, false)
    };
    let placed = fc_geom::transform(&traced, transform);
    if placed
        .iter()
        .flat_map(|contour| &contour.points)
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(invalid("image transform coordinates overflow"));
    }
    Ok(TraceOutcome {
        contours: placed,
        dropped,
        max_error,
        fallback,
    })
}

/// Fit every exact ring with curves, keeping the pixel edges if topology changes.
///
/// The fitted rings are compared as polylines against the exact rings. A
/// comparison that would need more than [`MAX_SIMPLIFY_POINTS`] points is
/// flattened more coarsely rather than skipped, so the orientation, containment
/// and crossing checks always run.
fn fit_curves(exact: &[Contour], options: &TraceOptions) -> (Vec<Contour>, f64, bool) {
    let fit = FitOptions {
        tolerance: options.tolerance,
        corner_angle: options.corner_angle,
        smoothing: options.smoothing,
    };
    let original: Vec<Vec<Point>> = exact.iter().map(model_ring).collect();
    let fits: Vec<_> = original
        .iter()
        .map(|ring| crate::autotrace::fit_ring(ring, &fit))
        .collect();
    let max_error = fits.iter().fold(0.0_f64, |worst, f| worst.max(f.error));
    let fallback = fits.iter().any(|f| f.fallback);
    let fitted: Vec<Contour> = fits.into_iter().map(|f| f.contour).collect();
    let candidates: Vec<Vec<Point>> = fitted.iter().map(flattened_ring).collect();
    // Containment compares against the exact rings, which is only affordable
    // inside the same budget the straight-line simplification uses. Beyond it the
    // orientation and crossing checks still run over the fitted polylines.
    let containment = original.iter().map(Vec::len).sum::<usize>() <= MAX_SIMPLIFY_POINTS;
    if preserves_topology(&original, &candidates, containment) {
        return (fitted, max_error, fallback);
    }
    // A fit that would merge, invert or cross a ring is never offered; the
    // straight-line simplification is tried next and exact edges last.
    (simplify(exact, options.tolerance), 0.0, true)
}

/// Point list of a contour, without interpreting its curve segments.
fn model_ring(contour: &Contour) -> Vec<Point> {
    contour
        .points
        .iter()
        .map(|p| Point::new(p.x, p.y))
        .collect()
}

/// Polyline approximation of a possibly curved contour, inside the compare budget.
fn flattened_ring(contour: &Contour) -> Vec<Point> {
    let path = contour.to_bezpath();
    for tolerance in [0.25_f64, 1.0, 4.0, 16.0] {
        let mut ring = Vec::new();
        flatten(path.iter(), tolerance, |element| match element {
            kurbo::PathEl::MoveTo(point) | kurbo::PathEl::LineTo(point) => ring.push(point),
            _ => {}
        });
        if ring.len() <= MAX_SIMPLIFY_POINTS {
            // A closed path repeats its start as the last line; drop the repeat.
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            return ring;
        }
    }
    model_ring(contour)
}

struct DecodedImage {
    width: u32,
    height: u32,
    color: png::ColorType,
    pixels: Vec<u8>,
}

fn decode(data: &[u8]) -> Result<DecodedImage, Error> {
    if data.len() > MAX_IMAGE_BYTES {
        return Err(invalid("source PNG exceeds the byte limit"));
    }
    if data.len() < 33
        || &data[..8] != b"\x89PNG\r\n\x1a\n"
        || data[8..12] != 13u32.to_be_bytes()
        || &data[12..16] != b"IHDR"
    {
        return Err(invalid("invalid PNG header"));
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > u64::from(MAX_PIXELS)
    {
        return Err(invalid("source PNG dimensions exceed the image limits"));
    }
    let mut decoder = png::Decoder::new_with_limits(
        data,
        png::Limits {
            bytes: MAX_IMAGE_BYTES,
        },
    );
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(png_error)?;
    if reader.info().animation_control.is_some() {
        return Err(invalid("animated PNG source images are unsupported"));
    }
    let buffer_size = reader.output_buffer_size();
    if buffer_size > MAX_PIXELS as usize * 4 {
        return Err(invalid("source PNG decoded buffer exceeds the image limit"));
    }
    let mut pixels = vec![0; buffer_size];
    let info = reader.next_frame(&mut pixels).map_err(png_error)?;
    reader.finish().map_err(png_error)?;
    if info.width != width || info.height != height || info.bit_depth != png::BitDepth::Eight {
        return Err(invalid("unexpected PNG frame dimensions or depth"));
    }
    pixels.truncate(info.buffer_size());
    Ok(DecodedImage {
        width,
        height,
        color: info.color_type,
        pixels,
    })
}

fn png_error(error: png::DecodingError) -> Error {
    Error::Invalid(format!("invalid source PNG: {error}"))
}

fn simplify(exact: &[Contour], tolerance: f64) -> Vec<Contour> {
    if tolerance == 0.0
        || exact
            .iter()
            .map(|contour| contour.points.len())
            .sum::<usize>()
            > MAX_SIMPLIFY_POINTS
    {
        return exact.to_vec();
    }
    let original: Vec<Vec<Point>> = exact
        .iter()
        .map(|contour| {
            contour
                .points
                .iter()
                .map(|p| Point::new(p.x, p.y))
                .collect()
        })
        .collect();
    let candidates: Vec<_> = original
        .iter()
        .map(|ring| simplify_ring(ring, tolerance))
        .collect();
    if !preserves_topology(&original, &original, true)
        || !preserves_topology(&original, &candidates, true)
    {
        return exact.to_vec();
    }
    exact
        .iter()
        .zip(candidates)
        .map(|(contour, ring)| Contour {
            points: ring
                .into_iter()
                .map(|point| fc_model::Point {
                    x: point.x,
                    y: point.y,
                    ..Default::default()
                })
                .collect(),
            ..contour.clone()
        })
        .collect()
}

fn simplify_ring(ring: &[Point], tolerance: f64) -> Vec<Point> {
    let split = (1..ring.len())
        .max_by(|&a, &b| {
            ring[a]
                .distance_squared(ring[0])
                .total_cmp(&ring[b].distance_squared(ring[0]))
        })
        .unwrap_or(0);
    let mut keep = vec![false; ring.len()];
    keep[0] = true;
    keep[split] = true;
    let mut ranges = vec![(0, split), (split, ring.len())];
    while let Some((start, end)) = ranges.pop() {
        let mut farthest = None;
        let mut max_distance = tolerance * tolerance;
        let a = ring[start];
        let b = ring[end % ring.len()];
        let direction = b - a;
        for (index, &point) in ring.iter().enumerate().take(end).skip(start + 1) {
            let projection = ((point - a).dot(direction) / direction.hypot2()).clamp(0., 1.);
            let distance = point.distance_squared(a + direction * projection);
            if distance > max_distance {
                farthest = Some(index);
                max_distance = distance;
            }
        }
        if let Some(index) = farthest {
            keep[index] = true;
            ranges.push((start, index));
            ranges.push((index, end));
        }
    }
    ring.iter()
        .zip(keep)
        .filter_map(|(&p, keep)| keep.then_some(p))
        .collect()
}

fn signed_area(ring: &[Point]) -> f64 {
    (0..ring.len())
        .map(|i| {
            ring[i]
                .to_vec2()
                .cross(ring[(i + 1) % ring.len()].to_vec2())
        })
        .sum::<f64>()
        / 2.0
}

fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    if a.x.max(b.x) < c.x.min(d.x)
        || c.x.max(d.x) < a.x.min(b.x)
        || a.y.max(b.y) < c.y.min(d.y)
        || c.y.max(d.y) < a.y.min(b.y)
    {
        return false;
    }
    let ab = b - a;
    let cd = d - c;
    ab.cross(c - a) * ab.cross(d - a) <= 0.0 && cd.cross(a - c) * cd.cross(b - c) <= 0.0
}

fn contains(ring: &[Point], point: Point) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    inside
}

/// Whether candidate rings keep the orientation, nesting and simplicity of the originals.
///
/// `containment` enables the pairwise inside/outside comparison against the
/// original rings; the orientation, spike and crossing checks always run.
fn preserves_topology(
    original: &[Vec<Point>],
    candidates: &[Vec<Point>],
    containment: bool,
) -> bool {
    if original.len() != candidates.len() {
        return false;
    }
    for (index, ring) in candidates.iter().enumerate() {
        if ring.len() < 3 || signed_area(ring) * signed_area(&original[index]) <= 0.0 {
            return false;
        }
        for i in 0..ring.len() {
            let incoming = ring[i] - ring[(i + ring.len() - 1) % ring.len()];
            let outgoing = ring[(i + 1) % ring.len()] - ring[i];
            if incoming.cross(outgoing) == 0.0 && incoming.dot(outgoing) <= 0.0 {
                return false;
            }
        }
        for (other_index, other) in candidates.iter().enumerate() {
            if containment
                && index != other_index
                && contains(&original[index], original[other_index][0]) != contains(ring, other[0])
            {
                return false;
            }
            if other_index < index {
                continue;
            }
            for i in 0..ring.len() {
                for j in 0..other.len() {
                    if index == other_index
                        && (j <= i || j == i + 1 || (i == 0 && j == ring.len() - 1))
                    {
                        continue;
                    }
                    if intersects(
                        ring[i],
                        ring[(i + 1) % ring.len()],
                        other[j],
                        other[(j + 1) % other.len()],
                    ) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape;

    fn png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut result, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        result
    }

    fn mask(rows: &[&str]) -> Vec<u8> {
        let pixels: Vec<u8> = rows
            .iter()
            .flat_map(|row| row.bytes())
            .flat_map(|pixel| {
                if pixel == b'#' {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                }
            })
            .collect();
        png(rows[0].len() as u32, rows.len() as u32, &pixels)
    }

    #[test]
    fn dimensions_validate_the_whole_png() {
        let data = mask(&["##", ".#"]);
        assert_eq!(image_dimensions(&data).unwrap(), (2, 2));
        assert!(image_dimensions(b"not a png").is_err());
        assert!(image_dimensions(&data[..data.len() / 2]).is_err());
        assert!(image_dimensions(&data[..data.len() - 12]).is_err());
        let mut corrupt = data.clone();
        corrupt[29] ^= 1;
        assert!(image_dimensions(&corrupt).is_err());
        let mut corrupt_iend = data.clone();
        *corrupt_iend.last_mut().unwrap() ^= 1;
        assert!(image_dimensions(&corrupt_iend).is_err());
        assert!(image_dimensions(&vec![0; MAX_IMAGE_BYTES + 1]).is_err());
    }

    #[test]
    fn oversized_dimensions_are_rejected_from_the_header() {
        for (width, height) in [(4097u32, 1u32), (1, 4097), (1024, 1024), (0, 1)] {
            let mut data = mask(&["#"]);
            data[16..20].copy_from_slice(&width.to_be_bytes());
            data[20..24].copy_from_slice(&height.to_be_bytes());
            let error = image_dimensions(&data).unwrap_err().to_string();
            assert!(error.contains("dimensions"), "{error}");
        }
    }

    #[test]
    fn alpha_is_composited_over_white() {
        let data = png(
            4,
            1,
            &[0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 200, 0, 0, 0, 255],
        );
        let contours = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(
            contours[0].to_bezpath().bounding_box(),
            kurbo::Rect::new(2., 0., 4., 1.)
        );
    }

    #[test]
    fn threshold_extremes_are_defined_and_white_remains_background() {
        let data = png(
            3,
            1,
            &[0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255],
        );
        assert!(trace_image(&data, Affine::IDENTITY, 0, 0.0).is_err());
        for (threshold, width) in [(1, 2.), (127, 2.), (128, 1.), (255, 1.)] {
            let contours = trace_image(&data, Affine::IDENTITY, threshold, 0.0).unwrap();
            assert_eq!(contours[0].to_bezpath().bounding_box().width(), width);
        }
    }

    #[test]
    fn holes_are_retained_with_opposite_winding() {
        let data = mask(&["#####", "#...#", "#.#.#", "#...#", "#####"]);
        for tolerance in [0.0, 0.7, 16.0] {
            let contours = trace_image(&data, Affine::IDENTITY, 128, tolerance).unwrap();
            assert_eq!(contours.len(), 3);
            let signs: Vec<_> = contours
                .iter()
                .map(|c| c.to_bezpath().area().signum())
                .collect();
            assert_eq!(signs.iter().filter(|&&sign| sign == signs[0]).count(), 2);
            let winding: i32 = contours
                .iter()
                .map(|c| c.to_bezpath().winding(kurbo::Point::new(1.5, 1.5)))
                .sum();
            assert_eq!(winding, 0, "hole filled at tolerance {tolerance}");
        }
    }

    #[test]
    fn affine_placement_matches_exact_bitmap_tracing() {
        let data = mask(&["#.", "##"]);
        let original = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        let transform = Affine::new([2., 0.5, -0.25, -3., 10., 20.]);
        assert_eq!(
            trace_image(&data, transform, 128, 0.0).unwrap(),
            fc_geom::transform(&original, transform)
        );
        assert_eq!(
            original[0]
                .to_bezpath()
                .winding(kurbo::Point::new(1.5, 1.5)),
            0
        );
        assert_ne!(
            original[0]
                .to_bezpath()
                .winding(kurbo::Point::new(0.5, 1.5)),
            0
        );
    }

    #[test]
    fn simplification_removes_stair_steps_but_keeps_closed_polygons() {
        let data = mask(&["#....", "##...", "###..", "####.", "#####"]);
        let exact = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        let simplified = trace_image(&data, Affine::IDENTITY, 128, 0.8).unwrap();
        assert!(simplified[0].points.len() < exact[0].points.len());
        assert!(simplified[0].closed);
        assert!(simplified[0].points.len() >= 3);
    }

    #[test]
    fn invalid_geometry_parameters_are_rejected() {
        let data = mask(&["#"]);
        for tolerance in [-1., 16.1, f64::NAN, f64::INFINITY] {
            assert!(trace_image(&data, Affine::IDENTITY, 128, tolerance).is_err());
        }
        for transform in [
            Affine::scale(0.),
            Affine::translate((f64::NAN, 0.)),
            Affine::scale(f64::MAX),
        ] {
            assert!(trace_image(&data, transform, 128, 0.0).is_err());
        }
        assert!(trace_image(
            &mask(&["##"]),
            Affine::new([1e308, 0., 0., 1e-308, 0., 0.]),
            128,
            0.0,
        )
        .is_err());
    }

    #[test]
    fn palette_transparency_and_sub_byte_grayscale_are_decoded() {
        let mut data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut data, 3, 1);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::One);
            encoder.set_palette(vec![0, 0, 0, 0, 0, 0]);
            encoder.set_trns(vec![0]);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[0b0100_0000])
                .unwrap();
        }
        let contours = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(
            contours[0].to_bezpath().bounding_box(),
            kurbo::Rect::new(1., 0., 2., 1.)
        );
        data.clear();
        {
            let mut encoder = png::Encoder::new(&mut data, 3, 1);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Two);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[0b0001_1100])
                .unwrap();
        }
        let contours = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(
            contours[0].to_bezpath().bounding_box(),
            kurbo::Rect::new(0., 0., 2., 1.)
        );
    }

    #[test]
    fn sixteen_bit_alpha_is_supported_and_animation_is_rejected() {
        let mut data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut data, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Sixteen);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255])
                .unwrap();
        }
        let contours = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(
            contours[0].to_bezpath().bounding_box(),
            kurbo::Rect::new(1., 0., 2., 1.)
        );
        data.clear();
        {
            let mut encoder = png::Encoder::new(&mut data, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(1, 0).unwrap();
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[0, 0, 0, 255])
                .unwrap();
        }
        assert!(image_dimensions(&data)
            .unwrap_err()
            .to_string()
            .contains("animated"));
    }

    #[test]
    fn simplification_does_not_merge_diagonal_contacts() {
        let data = mask(&["#.", ".#"]);
        let exact = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(exact.len(), 2);
        assert_eq!(
            trace_image(&data, Affine::IDENTITY, 128, 16.0).unwrap(),
            exact
        );
    }

    /// A filled disc PNG of `size` pixels, used for the curve-fitting checks.
    fn disc(size: u32, radius: f64) -> Vec<u8> {
        let centre = f64::from(size) / 2.;
        let pixels: Vec<u8> = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let inside =
                    (f64::from(x) + 0.5 - centre).hypot(f64::from(y) + 0.5 - centre) <= radius;
                if inside {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                }
            })
            .collect();
        png(size, size, &pixels)
    }

    fn node_count(contours: &[Contour]) -> usize {
        contours
            .iter()
            .flat_map(|c| &c.points)
            .filter(|p| p.kind != fc_model::PointKind::OffCurve)
            .count()
    }

    #[test]
    fn curve_fitting_beats_the_pixel_edge_result_on_a_clean_disc() {
        let data = disc(400, 160.);
        let exact = trace_image(&data, Affine::IDENTITY, 128, 0.0).expect("exact");
        let polygon = trace_image(&data, Affine::IDENTITY, 128, 1.0).expect("polygon");
        let curved = trace_image_with_options(
            &data,
            Affine::IDENTITY,
            &TraceOptions {
                tolerance: 1.0,
                ..Default::default()
            },
        )
        .expect("curved");
        assert_eq!(curved.contours.len(), 1);
        assert_eq!(curved.dropped, 0);
        assert!(!curved.fallback);
        let exact_nodes = node_count(&exact);
        let polygon_nodes = node_count(&polygon);
        let curved_nodes = node_count(&curved.contours);
        assert!(
            curved_nodes * 10 < exact_nodes,
            "curve fit {curved_nodes} against {exact_nodes} pixel edges"
        );
        assert!(
            curved_nodes < polygon_nodes,
            "curve fit {curved_nodes} against {polygon_nodes} simplified edges"
        );
        assert!(
            curved.contours[0]
                .points
                .iter()
                .any(|p| p.kind == fc_model::PointKind::OffCurve),
            "a disc must be fitted with curves"
        );
        // Both results describe the same disc; compare them where it matters.
        let bounds = |contours: &[Contour]| {
            let mut path = kurbo::BezPath::new();
            for contour in contours {
                path.extend(contour.to_bezpath().iter());
            }
            path.bounding_box()
        };
        let a = bounds(&exact);
        let b = bounds(&curved.contours);
        assert!(
            (a.x0 - b.x0).abs() < 2.
                && (a.y0 - b.y0).abs() < 2.
                && (a.x1 - b.x1).abs() < 2.
                && (a.y1 - b.y1).abs() < 2.,
            "curve fit bounds {b:?} against pixel edges {a:?}"
        );
        assert!(
            curved.max_error < 2.0,
            "reported deviation {}",
            curved.max_error
        );
    }

    #[test]
    fn curve_fitting_keeps_holes_and_their_opposite_winding() {
        let size = 200u32;
        let centre = f64::from(size) / 2.;
        let pixels: Vec<u8> = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let distance = (f64::from(x) + 0.5 - centre).hypot(f64::from(y) + 0.5 - centre);
                if (40. ..80.).contains(&distance) {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                }
            })
            .collect();
        let data = png(size, size, &pixels);
        let traced = trace_image_with_options(&data, Affine::IDENTITY, &TraceOptions::default())
            .expect("ring");
        assert_eq!(traced.contours.len(), 2);
        assert!(!traced.fallback);
        let signs: Vec<f64> = traced
            .contours
            .iter()
            .map(|c| c.to_bezpath().area().signum())
            .collect();
        assert_ne!(signs[0], signs[1], "a hole must wind the other way");
        let mut path = kurbo::BezPath::new();
        for contour in &traced.contours {
            path.extend(contour.to_bezpath().iter());
        }
        assert_eq!(path.winding(Point::new(centre, centre)), 0, "hole filled");
        assert_ne!(
            path.winding(Point::new(centre + 60., centre)),
            0,
            "ring body empty"
        );
    }

    #[test]
    fn the_minimum_area_filter_removes_speckle_and_keeps_the_drawing() {
        let size = 120u32;
        let pixels: Vec<u8> = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let body = (20..100).contains(&x) && (20..100).contains(&y);
                let speck = (110..112).contains(&x) && (110..112).contains(&y);
                if body || speck {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                }
            })
            .collect();
        let data = png(size, size, &pixels);
        let all = trace_image_with_options(&data, Affine::IDENTITY, &TraceOptions::default())
            .expect("all");
        assert_eq!(all.contours.len(), 2);
        assert_eq!(all.dropped, 0);
        let filtered = trace_image_with_options(
            &data,
            Affine::IDENTITY,
            &TraceOptions {
                min_area: 16.,
                ..Default::default()
            },
        )
        .expect("filtered");
        assert_eq!(filtered.contours.len(), 1);
        assert_eq!(filtered.dropped, 1);
        assert_eq!(
            node_count(&filtered.contours),
            4,
            "the square stays a square"
        );
    }

    #[test]
    fn invalid_autotrace_options_are_rejected_before_decoding() {
        let data = mask(&["#"]);
        for options in [
            TraceOptions {
                threshold: 0,
                ..Default::default()
            },
            TraceOptions {
                tolerance: 17.,
                ..Default::default()
            },
            TraceOptions {
                corner_angle: -1.,
                ..Default::default()
            },
            TraceOptions {
                corner_angle: 181.,
                ..Default::default()
            },
            TraceOptions {
                smoothing: 9.,
                ..Default::default()
            },
            TraceOptions {
                smoothing: f64::NAN,
                ..Default::default()
            },
            TraceOptions {
                min_area: -1.,
                ..Default::default()
            },
            TraceOptions {
                min_area: f64::INFINITY,
                ..Default::default()
            },
        ] {
            assert!(
                trace_image_with_options(&data, Affine::IDENTITY, &options).is_err(),
                "{options:?}"
            );
        }
    }

    #[test]
    fn curve_fitting_places_through_the_same_transform_as_exact_tracing() {
        let data = disc(100, 40.);
        let transform = Affine::new([2., 0.5, -0.25, -3., 10., 20.]);
        let placed = trace_image_with_options(
            &data,
            transform,
            &TraceOptions {
                tolerance: 1.5,
                ..Default::default()
            },
        )
        .expect("placed");
        let origin = trace_image_with_options(
            &data,
            Affine::IDENTITY,
            &TraceOptions {
                tolerance: 1.5,
                ..Default::default()
            },
        )
        .expect("origin");
        assert_eq!(
            placed.contours,
            fc_geom::transform(&origin.contours, transform)
        );
        assert!(
            trace_image_with_options(&data, Affine::scale(0.), &TraceOptions::default()).is_err()
        );
    }

    #[test]
    fn diagonal_contacts_are_not_merged_by_curve_fitting() {
        let data = mask(&["#.", ".#"]);
        let traced = trace_image_with_options(&data, Affine::IDENTITY, &TraceOptions::default())
            .expect("contacts");
        assert_eq!(traced.contours.len(), 2);
        for contour in &traced.contours {
            assert_eq!(contour.points.len(), 4, "a single pixel stays a square");
        }
    }

    #[test]
    fn largest_supported_solid_image_has_bounded_tracing_cost() {
        let data = png(1000, 1000, &[0, 0, 0, 255].repeat(1_000_000));
        let started = std::time::Instant::now();
        let contours = trace_image(&data, Affine::IDENTITY, 128, 0.0).unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].points.len(), 4);
        assert!(started.elapsed() < std::time::Duration::from_secs(30));
    }
}
