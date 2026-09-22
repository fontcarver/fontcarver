//! Preserve evaluated CFF stem identities and remap masks into model ordering.

use fc_model::{HintMask, PsHints, StemHint};
use kurbo::Affine;

#[derive(Default)]
pub(super) struct Hints {
    pub(super) values: PsHints,
    pub(super) lost: bool,
    order: Vec<(bool, usize)>,
    pending: Vec<(bool, Vec<u8>)>,
}

impl Hints {
    pub(super) fn stem(&mut self, horizontal: bool, start: f64, width: f64, transform: Affine) {
        let [xx, yx, xy, yy, dx, dy] = transform.as_coeffs();
        if xy != 0.0 || yx != 0.0 || xx <= 0.0 || yy <= 0.0 || self.order.len() == 96 {
            self.lost = true;
            return;
        }
        let (scale, offset) = if horizontal { (yy, dy) } else { (xx, dx) };
        let stems = if horizontal {
            &mut self.values.h_stems
        } else {
            &mut self.values.v_stems
        };
        self.order.push((horizontal, stems.len()));
        stems.push(StemHint {
            start: start * scale + offset,
            width: if width == -20.0 || width == -21.0 {
                width
            } else {
                width * scale
            },
            ranges: Vec::new(),
        });
    }

    pub(super) fn mask(&mut self, counter: bool, bytes: &[u8]) {
        if self.pending.len() + self.values.hint_masks.len() >= 65_535 {
            self.lost = true;
            return;
        }
        self.pending.push((counter, bytes.to_vec()));
    }

    pub(super) fn point(&mut self, contour: usize, point: usize) {
        for (counter, mask) in self.pending.drain(..) {
            if !counter {
                // Only the last active mask before a drawing operator takes effect.
                self.values
                    .hint_masks
                    .retain(|m| m.counter || m.contour != contour || m.point != point);
            }
            self.values.hint_masks.push(HintMask {
                counter,
                closing: false,
                contour,
                point,
                mask,
            });
        }
    }

    pub(super) fn close(&mut self, contour: usize, last: usize) {
        for mask in self
            .values
            .hint_masks
            .iter_mut()
            .filter(|m| m.contour == contour && m.point == last)
        {
            mask.point = 0;
            mask.closing = true;
        }
    }

    pub(super) fn finish(&mut self) {
        let count = self.order.len();
        for mask in &mut self.values.hint_masks {
            let mut reordered = vec![0; count.div_ceil(8)];
            for (old, (horizontal, index)) in self.order.iter().enumerate() {
                if mask
                    .mask
                    .get(old / 8)
                    .is_some_and(|byte| byte & (0x80 >> (old % 8)) != 0)
                {
                    let index = if *horizontal {
                        *index
                    } else {
                        self.values.h_stems.len() + index
                    };
                    reordered[index / 8] |= 0x80 >> (index % 8);
                }
            }
            mask.mask = reordered;
        }
        if self.lost {
            self.values = PsHints::default();
        }
    }

    pub(super) fn clear(&mut self) {
        // seac component replacement cannot be represented as one global stem set.
        self.lost |= !self.order.is_empty();
        self.order.clear();
        self.values = PsHints::default();
        self.pending.clear();
    }
}
