//! Bounded declarative EPS glyph outlines; imported PostScript is never executed.

use std::path::Path;

use fc_model::{Contour, Font, Glyph, GlyphName, Layer, MasterId};
use kurbo::{BezPath, PathEl, Point, Rect, Shape};

use crate::Error;

const LIMIT: f64 = 1_000_000_000.0;
const MAX_OPERATIONS: usize = 1_000_000;
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_NAME_BYTES: usize = 100;

fn error(reason: impl Into<String>) -> Error {
    super::source_error("EPS", reason)
}

/// Reads a literal outline subset without invoking a PostScript interpreter.
pub(super) fn read(path: &Path) -> Result<Font, Error> {
    let source = super::read_text(path, "EPS")?;
    let mut lines = source.split(['\r', '\n']);
    let first = lines.next().unwrap_or_default();
    let mut header = first.split_ascii_whitespace();
    if !header.next().is_some_and(|s| s.starts_with("%!PS-Adobe-"))
        || header.next() != Some("EPSF-3.0")
        || header.next().is_some()
    {
        return Err(error("expected an EPSF-3.0 header"));
    }
    let mut metadata = Metadata::default();
    let mut parser = Parser::default();
    let mut body = false;
    let mut ended = false;
    for line in lines {
        let line = line.trim();
        if line == "%%EOF" {
            ended = true;
            continue;
        }
        if line.starts_with('%') {
            metadata.comment(line, body)?;
            continue;
        }
        let code = line.split('%').next().unwrap_or_default();
        for token in code.split_ascii_whitespace() {
            if ended {
                return Err(error("executable data follows the EPS trailer"));
            }
            body = true;
            parser.token(token)?;
        }
    }
    if !parser.operands.is_empty() || (!parser.painted && !parser.path.is_empty()) {
        return Err(error("unfinished operands or an unpainted outline"));
    }
    let bbox = metadata.bbox.ok_or_else(|| error("missing BoundingBox"))?;
    let mut font = Font::new_empty("EPS Import", metadata.upem.unwrap_or(1000));
    let mut glyph = Glyph::new(metadata.name.unwrap_or_else(|| "glyph".into()));
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = metadata.width.unwrap_or(bbox.width());
    if layer.width > LIMIT {
        return Err(error("advance width exceeds the numeric limit"));
    }
    layer.contours = Contour::from_bezpath(&parser.path);
    glyph.layers.push(layer);
    font.insert_glyph(glyph).map_err(|e| error(e.to_string()))?;
    Ok(font)
}

#[derive(Default)]
struct Metadata {
    bbox: Option<Rect>,
    name: Option<GlyphName>,
    width: Option<f64>,
    upem: Option<u16>,
}

impl Metadata {
    fn comment(&mut self, line: &str, body: bool) -> Result<(), Error> {
        if let Some(value) = line.strip_prefix("%%BoundingBox:") {
            if body || self.bbox.is_some() {
                return Err(error("duplicate or non-header BoundingBox"));
            }
            let numbers = value
                .split_ascii_whitespace()
                .take(5)
                .map(number)
                .collect::<Result<Vec<_>, _>>()?;
            if numbers.len() != 4 || numbers[0] > numbers[2] || numbers[1] > numbers[3] {
                return Err(error("invalid BoundingBox"));
            }
            self.bbox = Some(Rect::new(numbers[0], numbers[1], numbers[2], numbers[3]));
        } else if let Some(value) = line.strip_prefix("%%FontCarverGlyphName:") {
            if body || self.name.is_some() {
                return Err(error("duplicate or non-header glyph name"));
            }
            let value = value.trim();
            if value.is_empty() || value.len() > MAX_NAME_BYTES * 2 || value.len() % 2 != 0 {
                return Err(error("invalid hexadecimal glyph name"));
            }
            let bytes = value
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| {
                    let high = char::from(pair[0]).to_digit(16);
                    let low = char::from(pair[1]).to_digit(16);
                    high.zip(low)
                        .map(|(a, b)| (a * 16 + b) as u8)
                        .ok_or_else(|| error("invalid hexadecimal glyph name"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let name = String::from_utf8(bytes).map_err(|_| error("glyph name is not UTF-8"))?;
            if name.chars().any(char::is_control) {
                return Err(error("glyph name contains control characters"));
            }
            self.name = Some(name.into());
        } else if let Some(value) = line.strip_prefix("%%FontCarverAdvanceWidth:") {
            if body || self.width.is_some() {
                return Err(error("duplicate or non-header advance width"));
            }
            let width = number(value.trim())?;
            if width < 0.0 {
                return Err(error("advance width must be nonnegative"));
            }
            self.width = Some(width);
        } else if let Some(value) = line.strip_prefix("%%FontCarverUnitsPerEm:") {
            if body || self.upem.is_some() {
                return Err(error("duplicate or non-header UPEM"));
            }
            let upem = value
                .trim()
                .parse::<u16>()
                .map_err(|_| error("invalid UPEM"))?;
            if upem == 0 {
                return Err(error("UPEM must be positive"));
            }
            self.upem = Some(upem);
        }
        Ok(())
    }
}

fn number(token: &str) -> Result<f64, Error> {
    let value = token
        .parse::<f64>()
        .map_err(|_| error("invalid numeric operand"))?;
    if !value.is_finite() || value.abs() > LIMIT {
        return Err(error("numeric operand exceeds the finite coordinate limit"));
    }
    Ok(value)
}

#[derive(Default)]
struct Parser {
    path: BezPath,
    operands: Vec<f64>,
    operations: usize,
    open: bool,
    painted: bool,
}

impl Parser {
    fn token(&mut self, token: &str) -> Result<(), Error> {
        if token.parse::<f64>().is_ok() {
            if self.operands.len() == 6 {
                return Err(error("too many numeric operands"));
            }
            self.operands.push(number(token)?);
            return Ok(());
        }
        self.operations += 1;
        if self.operations > MAX_OPERATIONS {
            return Err(error("outline exceeds the operation limit"));
        }
        if self.painted {
            return Err(error("only one fill operation is supported"));
        }
        match token {
            "newpath" => {
                self.arity(0)?;
                self.path = BezPath::new();
                self.open = false;
            }
            "moveto" => {
                self.arity(2)?;
                self.close();
                self.path.move_to((self.operands[0], self.operands[1]));
                self.open = true;
            }
            "lineto" => {
                self.arity(2)?;
                self.require_open()?;
                self.path.line_to((self.operands[0], self.operands[1]));
            }
            "curveto" => {
                self.arity(6)?;
                self.require_open()?;
                self.path.curve_to(
                    (self.operands[0], self.operands[1]),
                    (self.operands[2], self.operands[3]),
                    (self.operands[4], self.operands[5]),
                );
            }
            "closepath" => {
                self.arity(0)?;
                self.require_open()?;
                self.close();
            }
            "fill" => {
                self.arity(0)?;
                self.close();
                self.painted = true;
            }
            _ => {
                return Err(error(
                    "unsupported PostScript operator or executable syntax",
                ))
            }
        }
        if self.path.elements().len() > MAX_OPERATIONS {
            return Err(error("outline exceeds the path element limit"));
        }
        self.operands.clear();
        Ok(())
    }

    fn arity(&self, count: usize) -> Result<(), Error> {
        if self.operands.len() != count {
            return Err(error("incorrect operand count for outline operator"));
        }
        Ok(())
    }

    fn require_open(&self) -> Result<(), Error> {
        if !self.open {
            return Err(error("outline operator requires a preceding moveto"));
        }
        Ok(())
    }

    fn close(&mut self) {
        if self.open {
            self.path.close_path();
            self.open = false;
        }
    }
}

/// Writes one validated glyph as a standalone, declarative EPS outline.
pub(super) fn write(
    font: &Font,
    master: &MasterId,
    glyph: &GlyphName,
    path: &Path,
) -> Result<(), Error> {
    let outline = super::glyph_path(font, master, glyph, "EPS")?;
    let layer = font
        .glyph(glyph)
        .and_then(|g| g.master_layer(master))
        .ok_or_else(|| error("selected glyph or master layer does not exist"))?;
    let name: &str = glyph.as_ref();
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(error(
            "glyph name must contain 1–100 UTF-8 bytes without controls",
        ));
    }
    if font.info.units_per_em == 0 || !layer.width.is_finite() || layer.width.abs() > LIMIT {
        return Err(error("invalid UPEM or advance width"));
    }
    if outline.elements().len() + 2 > MAX_OPERATIONS {
        return Err(error("outline exceeds the operation limit"));
    }
    let bbox = outline.bounding_box();
    let hex: String = name.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let mut output = format!(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%Creator: FontCarver\n%%BoundingBox: {:.0} {:.0} {:.0} {:.0}\n%%HiResBoundingBox: {:e} {:e} {:e} {:e}\n%%FontCarverGlyphName: {hex}\n%%FontCarverAdvanceWidth: {:e}\n%%FontCarverUnitsPerEm: {}\n%%EndComments\nnewpath\n",
        bbox.x0.floor(), bbox.y0.floor(), bbox.x1.ceil(), bbox.y1.ceil(),
        bbox.x0, bbox.y0, bbox.x1, bbox.y1, layer.width, font.info.units_per_em,
    );
    let mut current = Point::ZERO;
    for element in outline.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                point(&mut output, p)?;
                output.push_str("moveto\n");
                current = p;
            }
            PathEl::LineTo(p) => {
                point(&mut output, p)?;
                output.push_str("lineto\n");
                current = p;
            }
            PathEl::QuadTo(control, p) => {
                point(&mut output, current + (control - current) * (2.0 / 3.0))?;
                point(&mut output, p + (control - p) * (2.0 / 3.0))?;
                point(&mut output, p)?;
                output.push_str("curveto\n");
                current = p;
            }
            PathEl::CurveTo(a, b, p) => {
                point(&mut output, a)?;
                point(&mut output, b)?;
                point(&mut output, p)?;
                output.push_str("curveto\n");
                current = p;
            }
            PathEl::ClosePath => output.push_str("closepath\n"),
        }
        if output.len() > MAX_BYTES - 16 {
            return Err(error("serialized outline exceeds the byte limit"));
        }
    }
    output.push_str("fill\n%%EOF\n");
    super::atomic_write(path, output.as_bytes())
}

fn point(output: &mut String, point: Point) -> Result<(), Error> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > LIMIT
        || point.y.abs() > LIMIT
    {
        return Err(error("outline exceeds the finite coordinate limit"));
    }
    output.push_str(&format!("{:e} {:e} ", point.x, point.y));
    Ok(())
}

#[cfg(test)]
mod tests {
    use fc_model::{Contour, Glyph, Layer};
    use kurbo::{BezPath, Shape};

    use super::*;

    fn fixture() -> (Font, GlyphName) {
        let mut font = Font::new_empty("EPS Test", 2048);
        let name = GlyphName::from("A test/字");
        let mut glyph = Glyph::new(name.clone());
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.width = 645.5;
        let mut path = BezPath::new();
        path.move_to((-25.5, -12.0));
        path.line_to((230.0, 0.0));
        path.curve_to((210.0, 400.0), (10.0, 390.0), (-25.5, -12.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
        (font, name)
    }

    #[test]
    fn cubic_outline_and_metadata_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("glyph.eps");
        let (font, name) = fixture();
        write(&font, &font.default_master().id, &name, &path).unwrap();
        let restored = read(&path).unwrap();
        assert_eq!(restored.info.units_per_em, 2048);
        let before = font.glyph(&name).unwrap().layers[0].contours[0].to_bezpath();
        let layer = &restored.glyph(&name).unwrap().layers[0];
        assert_eq!(layer.width, 645.5);
        assert_eq!(layer.contours[0].to_bezpath(), before);
    }

    #[test]
    fn reads_literal_eps_and_implicitly_closes_fill() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plain.eps");
        std::fs::write(&path, "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: -10 0 250 400\n%%EndComments\nnewpath\n-10 0 moveto\n250 0 lineto\n250 400 lineto\nfill\n%%EOF\n").unwrap();
        let font = read(&path).unwrap();
        let layer = &font.glyph("glyph").unwrap().layers[0];
        assert_eq!(layer.width, 260.0);
        assert!(layer.contours[0].closed);
        assert_eq!(layer.contours[0].to_bezpath().bounding_box().y1, 400.0);
    }

    #[test]
    fn rejects_unsafe_and_unsupported_programs() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.eps");
        for program in [
            "(secret) (r) file",
            "{ 1 } exec",
            "0 0 moveto 10 10 lineto stroke",
            "NaN 0 moveto",
            "1e300 0 moveto",
            "0 0 1 moveto",
            "10 20 lineto",
            "0 0 moveto 10 20 lineto",
            "0 0 moveto 1 1 lineto fill 7",
        ] {
            std::fs::write(
                &path,
                format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 10 20\n{program}\n"),
            )
            .unwrap();
            assert!(read(&path).is_err(), "accepted {program}");
        }
    }

    #[test]
    fn invalid_export_preserves_existing_destination() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("existing.eps");
        std::fs::write(&path, "preserve").unwrap();
        let (mut font, name) = fixture();
        font.glyph_mut(&name).unwrap().layers[0].width = f64::NAN;
        assert!(write(&font, &font.default_master().id, &name, &path).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "preserve");
    }

    #[test]
    fn rejects_invalid_metadata_and_multiple_paints() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("metadata.eps");
        for source in [
            "%%BoundingBox: 0 0 1 1 2\n",
            "%%BoundingBox: 2 0 1 1\n",
            "%%BoundingBox: 0 0 1 1\n%%FontCarverUnitsPerEm: 0\n",
            "%%BoundingBox: 0 0 1 1\n%%FontCarverAdvanceWidth: -1\n",
            "%%BoundingBox: 0 0 1 1\n%%FontCarverGlyphName: 0a\n",
            "%%BoundingBox: 0 0 1 1\n%%FontCarverGlyphName: ff\n",
            "%%BoundingBox: 0 0 1 1\nfill fill\n",
            "%%BoundingBox: 0 0 1 1\n%%EOF\n0 0 moveto fill\n",
        ] {
            std::fs::write(&path, format!("%!PS-Adobe-3.0 EPSF-3.0\n{source}")).unwrap();
            assert!(read(&path).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn bounds_operation_and_operand_counts() {
        let mut parser = Parser {
            operations: MAX_OPERATIONS,
            ..Parser::default()
        };
        assert!(parser.token("newpath").is_err());
        let mut parser = Parser::default();
        for _ in 0..6 {
            parser.token("1").unwrap();
        }
        assert!(parser.token("1").is_err());
    }

    #[test]
    fn exports_quadratics_and_multiple_contours() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("quadratic.eps");
        let (mut font, name) = fixture();
        let points = [
            (0.0, 0.0, fc_model::PointKind::Line),
            (50.0, 100.0, fc_model::PointKind::OffCurve),
            (100.0, 0.0, fc_model::PointKind::QCurve),
        ]
        .into_iter()
        .map(|(x, y, kind)| fc_model::Point {
            x,
            y,
            kind,
            ..Default::default()
        })
        .collect();
        let quadratic = Contour {
            closed: true,
            points,
            ..Default::default()
        };
        let area = quadratic.to_bezpath().area();
        font.glyph_mut(&name).unwrap().layers[0]
            .contours
            .push(quadratic);
        write(&font, &font.default_master().id, &name, &path).unwrap();
        let restored = read(&path).unwrap();
        let contours = &restored.glyph(&name).unwrap().layers[0].contours;
        assert_eq!(contours.len(), 2);
        assert!((contours[1].to_bezpath().area() - area).abs() < 1e-9);
    }

    #[test]
    fn ghostscript_accepts_our_known_safe_output_when_available() {
        let version = std::process::Command::new("gs").arg("--version").output();
        if matches!(&version, Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
            eprintln!("Ghostscript unavailable; skipping independent EPS parser check");
            return;
        }
        assert!(version.unwrap().status.success());
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("generated.eps");
        let (font, name) = fixture();
        write(&font, &font.default_master().id, &name, &path).unwrap();
        // This interpreter only receives bytes produced above from a fixed trusted fixture.
        let result = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=nullpage"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
