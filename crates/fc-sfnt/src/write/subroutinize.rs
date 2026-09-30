//! Whole-program CFF1 sharing, derived from Adobe Technical Note 5177 §2.3/4.4.

use std::collections::HashMap;

use super::{encoding::number, WriteError};

// Input is exclusively the native writer's complete CFF1 programs, with no
// existing subroutine calls. Sharing their entire bodies preserves operand and
// hint-mask state without needing to split or reinterpret charstring tokens.
pub(super) fn extract(programs: &mut [Vec<u8>]) -> Result<Vec<Vec<u8>>, WriteError> {
    let original_size: u64 = programs.iter().map(|program| program.len() as u64).sum();
    let original_index_size = index_size(programs.len(), original_size)?;
    let mut groups: HashMap<&[u8], Vec<usize>> = HashMap::new();
    for (index, program) in programs.iter().enumerate() {
        if program.last() != Some(&14) {
            return Err(WriteError(
                "subroutinization requires complete CFF1 charstrings".into(),
            ));
        }
        groups.entry(program).or_default().push(index);
    }
    let mut candidates: Vec<_> = groups
        .into_iter()
        .filter(|(program, positions)| {
            // A call costs at most five bytes; allow four bytes for the new
            // INDEX offset. The final check includes the entire INDEX overhead.
            positions.len() > 1
                && program.len() as u64 * (positions.len() - 1) as u64
                    > positions.len() as u64 * 5 + 4
        })
        .collect();
    // Stable glyph order, independent of the randomized hash iteration order.
    candidates.sort_unstable_by_key(|(_, positions)| positions[0]);
    let bias = match candidates.len() {
        0..1240 => 107,
        1240..33900 => 1131,
        _ => 32768,
    };
    let mut subroutines = Vec::with_capacity(candidates.len());
    let mut calls = Vec::new();
    let mut reduced_size = original_size;
    for (index, (program, positions)) in candidates.into_iter().enumerate() {
        let mut subroutine = program.to_vec();
        if let Some(last) = subroutine.last_mut() {
            *last = 11; // return; the caller retains endchar.
        }
        let mut call = Vec::new();
        // INDEX count limits guarantee this biased operand fits Type 2 int16.
        number(&mut call, index as f64 - f64::from(bias))?;
        call.extend([29, 14]); // callgsubr, endchar
        reduced_size -= (program.len() - call.len()) as u64 * positions.len() as u64;
        calls.extend(positions.into_iter().map(|index| (index, call.clone())));
        subroutines.push(subroutine);
    }
    let global_size = index_size(
        subroutines.len(),
        subroutines.iter().map(|program| program.len() as u64).sum(),
    )?;
    if index_size(programs.len(), reduced_size)? + global_size >= original_index_size + 2 {
        return Ok(Vec::new());
    }
    for (index, call) in calls {
        programs[index] = call;
    }
    Ok(subroutines)
}

fn index_size(count: usize, data_size: u64) -> Result<u64, WriteError> {
    if count > usize::from(u16::MAX) || data_size >= u64::from(u32::MAX) {
        return Err(WriteError(
            "subroutinized CFF INDEX exceeds format limits".into(),
        ));
    }
    if count == 0 {
        return Ok(2);
    }
    let offset_size = match data_size + 1 {
        0..=0xff => 1,
        0x100..=0xffff => 2,
        0x10000..=0xffffff => 3,
        _ => 4,
    };
    Ok(3 + (count as u64 + 1) * offset_size + data_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::encoding::{index, integer};

    fn program(width: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        integer(&mut bytes, width);
        bytes.extend([139, 139, 21]);
        for _ in 0..20 {
            bytes.extend([140, 140, 5]);
        }
        bytes.push(14);
        bytes
    }

    #[test]
    fn repeated_programs_become_smaller_global_calls() {
        let original = vec![program(500); 4];
        let mut programs = original.clone();
        let globals = extract(&mut programs).expect("extract");
        assert_eq!(globals.len(), 1);
        assert_eq!(globals[0].last(), Some(&11));
        assert!(programs.iter().all(|program| program == &[32, 29, 14]));
        assert!(
            index(&programs).unwrap().len() + index(&globals).unwrap().len()
                < index(&original).unwrap().len() + 2
        );
    }

    #[test]
    fn tiny_and_unique_programs_remain_unchanged() {
        for original in [vec![vec![139, 14]; 32], vec![program(500), program(501)]] {
            let mut programs = original.clone();
            assert!(extract(&mut programs).expect("extract").is_empty());
            assert_eq!(programs, original);
        }
    }

    #[test]
    fn subroutine_indices_use_bias_for_final_count() {
        for (count, bias) in [(1239, 107), (1240, 1131)] {
            let original: Vec<_> = (0..count)
                .flat_map(|value| [program(value), program(value)])
                .collect();
            let mut programs = original.clone();
            let globals = extract(&mut programs).expect("extract");
            assert_eq!(globals.len(), count as usize);
            for index in 0..count {
                let mut expected = Vec::new();
                integer(&mut expected, index - bias);
                expected.extend([29, 14]);
                assert_eq!(programs[index as usize * 2], expected);
            }
            let mut repeated = original;
            assert_eq!(extract(&mut repeated).unwrap(), globals);
            assert_eq!(repeated, programs);
        }
    }
}
