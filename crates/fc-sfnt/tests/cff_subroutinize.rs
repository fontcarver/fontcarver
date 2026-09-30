use fc_model::{CidInfo, Contour, Font, Glyph, GlyphName, HintMask, Layer, PsPrivate, StemHint};
use fc_sfnt::write::{write_cff, write_cff_with_subroutines};
use kurbo::BezPath;
use read_fonts::{
    ps::{cff::CffFontRef, cs::CommandSink},
    types::{Fixed, GlyphId},
};

#[derive(Debug, PartialEq)]
enum Event {
    Move(Fixed, Fixed),
    Line(Fixed, Fixed),
    Curve([Fixed; 6]),
    Close,
    Stem(bool, Fixed, Fixed),
    Mask(bool, Vec<u8>),
}

#[derive(Default)]
struct RecordingSink(Vec<Event>);

impl CommandSink for RecordingSink {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.0.push(Event::Move(x, y));
    }
    fn line_to(&mut self, x: Fixed, y: Fixed) {
        self.0.push(Event::Line(x, y));
    }
    fn curve_to(&mut self, a: Fixed, b: Fixed, c: Fixed, d: Fixed, x: Fixed, y: Fixed) {
        self.0.push(Event::Curve([a, b, c, d, x, y]));
    }
    fn close(&mut self) {
        self.0.push(Event::Close);
    }
    fn hstem(&mut self, y: Fixed, dy: Fixed) {
        self.0.push(Event::Stem(false, y, dy));
    }
    fn vstem(&mut self, x: Fixed, dx: Fixed) {
        self.0.push(Event::Stem(true, x, dx));
    }
    fn hint_mask(&mut self, mask: &[u8]) {
        self.0.push(Event::Mask(false, mask.to_vec()));
    }
    fn counter_mask(&mut self, mask: &[u8]) {
        self.0.push(Event::Mask(true, mask.to_vec()));
    }
}

fn duplicate_font(cid: bool) -> (Font, Vec<GlyphName>) {
    let mut font = Font::new_empty("Subroutine Test", 1000);
    let master = font.default_master().id.clone();
    let mut path = BezPath::new();
    path.move_to((20.0, 0.0));
    path.line_to((20.0, 600.0));
    path.curve_to((40.25, 730.0), (260.5, 740.0), (300.0, 600.0));
    path.line_to((300.0, 0.0));
    path.close_path();
    let mut order = vec![GlyphName::from(".notdef")];
    for (index, name) in ["A", "B", "C", "D", "E", "F", "G", "H"].iter().enumerate() {
        let mut glyph = Glyph::new(*name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = if index == 7 { 501.0 } else { 500.0 };
        layer.contours = Contour::from_bezpath(&path);
        layer.hints.h_stems.push(StemHint {
            start: 600.0,
            width: 100.0,
            ranges: vec![],
        });
        layer.hints.v_stems.push(StemHint {
            start: 20.0,
            width: 80.0,
            ranges: vec![],
        });
        layer.hints.hint_masks = vec![
            HintMask {
                contour: 0,
                point: 0,
                mask: vec![0xc0],
                ..Default::default()
            },
            HintMask {
                contour: 0,
                point: 4,
                mask: vec![0x40],
                ..Default::default()
            },
        ];
        layer.cff_private_dict_index = cid.then_some((index % 2) as u8);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
        order.push((*name).into());
    }
    if cid {
        font.masters[0].cff_private_dicts.push(PsPrivate {
            blue_values: vec![-10.0, 0.0, 700.0, 710.0],
            ..Default::default()
        });
        font.cid = Some(CidInfo {
            registry: "Adobe".into(),
            ordering: "Identity".into(),
            supplement: 0,
            cid_map: order
                .iter()
                .enumerate()
                .map(|(index, name)| (index as u32 * 4, name.clone()))
                .collect(),
        });
    }
    (font, order)
}

#[test]
fn global_subroutines_preserve_widths_outlines_hints_and_cid_selections() {
    let started = std::time::Instant::now();
    for cid in [false, true] {
        let (font, order) = duplicate_font(cid);
        let original = font.clone();
        let master = &font.default_master().id;
        let plain = write_cff(&font, master, &order, false).expect("plain");
        let shared =
            write_cff_with_subroutines(&font, master, &order, false, true).expect("shared");
        assert_eq!(font, original);
        assert!(shared.bytes.len() < plain.bytes.len());
        assert_eq!(shared.bounds, plain.bounds);
        assert_eq!(shared.advances, plain.advances);
        assert_eq!(
            shared.bytes,
            write_cff_with_subroutines(&font, master, &order, false, true)
                .unwrap()
                .bytes
        );
        let plain = CffFontRef::new_cff(&plain.bytes, 0, Some(1000)).expect("plain CFF");
        let shared = CffFontRef::new_cff(&shared.bytes, 0, Some(1000)).expect("shared CFF");
        assert_eq!(shared.global_subrs().count(), 1);
        assert_eq!(plain.global_subrs().count(), 0);
        for index in 0..order.len() {
            let gid = GlyphId::new(index as u32);
            let mut before = RecordingSink::default();
            let mut after = RecordingSink::default();
            let fd = plain.subfont_index(gid).unwrap_or_default();
            assert_eq!(shared.subfont_index(gid), plain.subfont_index(gid));
            let plain_subfont = plain.subfont(fd, &[]).expect("plain subfont");
            let shared_subfont = shared.subfont(fd, &[]).expect("shared subfont");
            let before_width = plain
                .evaluate_charstring(&plain_subfont, gid, &[], &mut before)
                .expect("plain program");
            let after_width = shared
                .evaluate_charstring(&shared_subfont, gid, &[], &mut after)
                .expect("shared program");
            assert_eq!(before_width, after_width, "glyph {index}");
            assert_eq!(before.0, after.0, "glyph {index}");
            if index > 0 {
                assert!(after
                    .0
                    .iter()
                    .any(|event| matches!(event, Event::Mask(false, _))));
                assert_eq!(
                    after_width.unwrap().to_f64(),
                    if index == 8 { 501.0 } else { 500.0 }
                );
            }
        }
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn unique_charstrings_keep_original_cff_bytes() {
    let (font, _) = duplicate_font(false);
    let order = vec![".notdef".into(), "A".into(), "H".into()];
    let plain = write_cff(&font, &font.default_master().id, &order, false).expect("plain");
    let shared = write_cff_with_subroutines(&font, &font.default_master().id, &order, false, true)
        .expect("shared");
    assert_eq!(plain.bytes, shared.bytes);
}
