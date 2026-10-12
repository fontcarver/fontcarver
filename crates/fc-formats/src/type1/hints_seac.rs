//! Original Type 1 seac hint composition, with separate component active sets.
//!
//! Adobe Type 1 section 6.4 and Supplement 5015 section 6 define accent placement
//! relative to sidebearing points. This matches the existing read-fonts outline
//! evaluator: base at the origin, accent at parent_sbx + adx - asb, ady.

use read_fonts::ps::cs::CharstringContext;

use super::{Scan, Stem};

impl Scan {
    pub(super) fn seac(
        &mut self,
        context: &impl CharstringContext,
        depth: usize,
        budget: &mut usize,
    ) -> Result<(), &'static str> {
        self.operands(5)?;
        if self.in_seac || self.contour.is_some() || !self.values.is_empty() {
            return Err("nested seac or parent drawing/hints before seac");
        }
        let parent_sidebearing = self.sidebearing.ok_or("seac precedes sidebearing")?;
        let asb = self.stack[0];
        let offset = [parent_sidebearing + self.stack[1] - asb, self.stack[2]];
        let codes = [self.stack[3], self.stack[4]];
        if codes
            .iter()
            .any(|code| code.fract() != 0.0 || !(0.0..=255.0).contains(code))
        {
            return Err("invalid seac StandardEncoding code");
        }
        let programs = context
            .seac_components(codes[0] as i32, codes[1] as i32)
            .map_err(|_| "missing seac component")?;
        let mut contour_offset = 0;
        for (index, program) in programs.into_iter().enumerate() {
            let mut component = Scan {
                in_seac: true,
                ..Default::default()
            };
            if !component.walk(program, context, depth + 1, budget)?
                || !component.stack.is_empty()
                || component.result.is_some()
                || component.replacement_call
            {
                return Err("incomplete seac component program");
            }
            let sidebearing = component.sidebearing.ok_or("missing seac sidebearing")?;
            if index == 1 && sidebearing != asb {
                return Err("seac accent sidebearing does not match asb");
            }
            if !component.counters.is_empty() {
                return Err("seac component-scoped counter hints are unsupported");
            }
            // A move that is discarded by the outline evaluator cannot safely
            // supply the component boundary. The final count is checked against
            // the evaluated outline before any hint masks are bound.
            let count = component
                .contour
                .map(|last| last + 1)
                .ok_or("seac component has no bindable contour")?;
            self.append_component(
                component,
                contour_offset,
                if index == 0 { [0.0; 2] } else { offset },
            )?;
            contour_offset += count;
        }
        self.stack.clear();
        self.seac_contours = Some(contour_offset);
        Ok(())
    }

    fn append_component(
        &mut self,
        mut component: Scan,
        contour_offset: usize,
        offset: [f64; 2],
    ) -> Result<(), &'static str> {
        let mut mapping = [Vec::<usize>::new(), Vec::<usize>::new()];
        let mut count = self.values.h_stems.len() + self.values.v_stems.len();
        for (horizontal, stems, target, shift) in [
            (
                true,
                &mut component.values.h_stems,
                &mut self.values.h_stems,
                offset[1],
            ),
            (
                false,
                &mut component.values.v_stems,
                &mut self.values.v_stems,
                offset[0],
            ),
        ] {
            for stem in stems {
                // Translation moves a ghost's edge and encoded start equally;
                // the negative marker width must never change.
                stem.start += shift;
                let index = match target.iter().position(|existing| existing == stem) {
                    Some(index) => index,
                    None => {
                        if count >= 96 {
                            return Err("seac combined stem count limit exceeded");
                        }
                        count += 1;
                        target.push(stem.clone());
                        target.len() - 1
                    }
                };
                mapping[usize::from(horizontal)].push(index);
            }
        }
        // Every independently scanned first moveto supplies an event, including
        // an empty set. Keeping that event prevents the preceding component's
        // active hints from leaking across the component boundary.
        for mut event in component.events {
            event.contour += contour_offset;
            event.active = event
                .active
                .into_iter()
                .map(|(horizontal, index)| (horizontal, mapping[usize::from(horizontal)][index]))
                .collect::<Vec<Stem>>();
            self.events.push(event);
            if self.events.len() > 65535 {
                return Err("seac hint mask count limit exceeded");
            }
        }
        Ok(())
    }
}
