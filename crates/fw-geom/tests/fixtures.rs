#![allow(dead_code)]

use fw_model::{Contour, Point, PointKind, TtIndex};

pub fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        smooth: false,
        name: None,
        tt_index: TtIndex::Unnumbered,
    }
}

pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        closed: true,
        points: vec![
            point(x0, y0, PointKind::Line),
            point(x1, y0, PointKind::Line),
            point(x1, y1, PointKind::Line),
            point(x0, y1, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

pub fn open_line(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        closed: false,
        points: vec![
            point(x0, y0, PointKind::Move),
            point(x1, y1, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}
