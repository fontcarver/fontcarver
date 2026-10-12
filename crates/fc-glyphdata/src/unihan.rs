//! Offline Unicode 17.0 Unihan radical, stroke and reading lookups.

const INDEX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/unihan.index"));
const TEXT: &str = include_str!(concat!(env!("OUT_DIR"), "/unihan.txt"));
/// Version of the reviewed Unihan source data embedded in this build.
pub const UNIHAN_VERSION: &str = "17.0.0";

/// Iterate the recorded Unicode scalars in ascending order.
pub fn unihan_codepoints() -> impl Iterator<Item = u32> {
    INDEX
        .chunks_exact(12)
        .map(|row| u32::from_le_bytes([row[0], row[1], row[2], row[3]]))
}

/// Sorted IICore repertoire, built once from the pinned Unihan properties.
pub fn iicore_codepoints() -> &'static [u32] {
    static VALUES: std::sync::LazyLock<Vec<u32>> = std::sync::LazyLock::new(|| {
        unihan_codepoints()
            .filter(|cp| unihan(*cp).is_some_and(|record| !record.iicore.is_empty()))
            .collect()
    });
    &VALUES
}

/// An original kRSUnicode value, including radical form and residual strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadicalStroke {
    /// Kangxi radical number, from 1 through 214.
    pub radical: u16,
    /// Zero for traditional; one through three identify simplified forms.
    pub simplified: u8,
    /// Remaining strokes; Unicode permits negative residual counts.
    pub residual: i16,
}

/// Borrowed source properties; empty strings mean the property is unassigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnihanRecord {
    /// Unicode scalar value.
    pub unicode: u32,
    /// Original kRSUnicode values, preserving multiple radical assignments.
    pub radical_strokes: &'static str,
    /// Original kTotalStrokes values, including regional alternatives.
    pub total_strokes: &'static str,
    /// Standard Mandarin pinyin, preserving tone marks.
    pub mandarin: &'static str,
    /// Cantonese Jyutping.
    pub cantonese: &'static str,
    /// Japanese on readings.
    pub japanese_on: &'static str,
    /// Japanese kun readings.
    pub japanese_kun: &'static str,
    /// Korean romanized readings.
    pub korean: &'static str,
    /// Original kIICore priority and source indicators.
    pub iicore: &'static str,
}

impl UnihanRecord {
    /// Iterate every radical-stroke assignment without allocating.
    pub fn radicals(&self) -> impl Iterator<Item = RadicalStroke> + '_ {
        self.radical_strokes.split_whitespace().filter_map(|value| {
            let (radical, residual) = value.split_once('.')?;
            Some(RadicalStroke {
                radical: radical.trim_end_matches('\'').parse().ok()?,
                simplified: radical.chars().filter(|c| *c == '\'').count() as u8,
                residual: residual.parse().ok()?,
            })
        })
    }
    /// Iterate every recorded total stroke count, retaining regional alternatives.
    pub fn strokes(&self) -> impl Iterator<Item = u16> + '_ {
        self.total_strokes
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
    }
    /// Match a normalized reading prefix in one language, or all supported languages.
    /// Recognized language keys are mandarin, cantonese, japanese, korean and any.
    pub fn matches_reading(&self, query: &str, language: &str) -> bool {
        let values: &[&str] = match language {
            "mandarin" => &[self.mandarin],
            "cantonese" => &[self.cantonese],
            "japanese" => &[self.japanese_on, self.japanese_kun],
            "korean" => &[self.korean],
            "any" => &[
                self.mandarin,
                self.cantonese,
                self.japanese_on,
                self.japanese_kun,
                self.korean,
            ],
            _ => return false,
        };
        let query = normalize_reading(query);
        !query.is_empty()
            && values.iter().any(|value| {
                value
                    .split_whitespace()
                    .any(|word| normalize_reading(word).starts_with(&query))
            })
    }
}

/// Look up a Unicode scalar using the generated sorted index, in logarithmic time.
pub fn unihan(unicode: u32) -> Option<UnihanRecord> {
    let read = |offset: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            INDEX.get(offset..offset + 4)?.try_into().ok()?,
        ))
    };
    let (mut low, mut high) = (0, INDEX.len() / 12);
    while low < high {
        let mid = low + (high - low) / 2;
        match read(mid * 12)?.cmp(&unicode) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => {
                let offset = read(mid * 12 + 4)? as usize;
                let len = read(mid * 12 + 8)? as usize;
                let mut fields = TEXT.get(offset..offset + len)?.split('\t');
                return Some(UnihanRecord {
                    unicode,
                    radical_strokes: fields.next()?,
                    total_strokes: fields.next()?,
                    mandarin: fields.next()?,
                    cantonese: fields.next()?,
                    japanese_on: fields.next()?,
                    japanese_kun: fields.next()?,
                    korean: fields.next()?,
                    iicore: fields.next()?,
                });
            }
        }
    }
    None
}

/// Lowercase reading text, dropping Mandarin tone marks and trailing tone digits.
/// The u-umlaut is represented by `v`; `u:` is accepted as the same spelling.
pub fn normalize_reading(text: &str) -> String {
    text.to_lowercase()
        .replace("u:", "v")
        .replace("u\u{0308}", "v")
        .chars()
        .filter_map(|c| {
            Some(match c {
                'ā' | 'á' | 'ǎ' | 'à' => 'a',
                'ē' | 'é' | 'ě' | 'è' => 'e',
                'ī' | 'í' | 'ǐ' | 'ì' => 'i',
                'ō' | 'ó' | 'ǒ' | 'ò' => 'o',
                'ū' | 'ú' | 'ǔ' | 'ù' => 'u',
                'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => 'v',
                'ń' | 'ň' | 'ǹ' => 'n',
                'ḿ' => 'm',
                '0'..='9' | '\u{0300}'..='\u{036f}' => return None,
                c => c,
            })
        })
        .collect()
}

/// Number of Kangxi radicals, which is also the length of [`kangxi_radicals`].
pub const KANGXI_RADICAL_COUNT: u16 = 214;

/// First character of the Unicode Kangxi Radicals block, which is radical 1.
///
/// The block assigns its 214 characters in radical order, so radical `n` is
/// `KANGXI_BLOCK_START + n - 1`. `kangxi_block_is_radical_ordered` pins that
/// layout against the shipped `Blocks.txt` and the compiled assignment data.
const KANGXI_BLOCK_START: u32 = 0x2F00;

/// One ideographic form of a Kangxi radical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadicalForm {
    /// Unicode scalar value of the ideograph that writes the radical.
    pub unicode: u32,
    /// Smallest recorded `kTotalStrokes` value, or `None` when unrecorded.
    pub strokes: Option<u16>,
}

/// One Kangxi radical: its block character and the ideographs that write it.
///
/// The forms are exactly the Basic Multilingual Plane ideographs whose
/// `kRSUnicode` assigns this radical with no residual strokes, which is how the
/// pinned source spells "this character *is* the radical". Traditional and
/// simplified radical indicators are both kept, so `forms` of radical 149 holds
/// 言, 訁 and 讠. Compatibility ideographs are excluded because they only repeat
/// characters already listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Radical {
    /// Kangxi radical number, from 1 through 214.
    pub number: u16,
    /// Character of the Kangxi Radicals block for this radical number.
    pub kangxi: u32,
    /// Ideographs that write the radical, in ascending scalar order.
    pub forms: Vec<RadicalForm>,
}

/// The 214 Kangxi radicals in radical order, derived once from the pinned data.
pub fn kangxi_radicals() -> &'static [Radical] {
    static VALUES: std::sync::LazyLock<Vec<Radical>> = std::sync::LazyLock::new(|| {
        let mut forms = vec![Vec::new(); usize::from(KANGXI_RADICAL_COUNT)];
        for unicode in unihan_codepoints() {
            // Radical forms are BMP ideographs; the CJK Compatibility
            // Ideographs block repeats characters that are already listed.
            if unicode > 0xffff || (0xf900..=0xfaff).contains(&unicode) {
                continue;
            }
            let Some(record) = unihan(unicode) else {
                continue;
            };
            let strokes = record.strokes().min();
            let mut numbers: Vec<u16> = record
                .radicals()
                .filter(|value| value.residual == 0)
                .map(|value| value.radical)
                .collect();
            numbers.sort_unstable();
            numbers.dedup();
            for number in numbers {
                if let Some(slot) = forms.get_mut(usize::from(number).wrapping_sub(1)) {
                    slot.push(RadicalForm { unicode, strokes });
                }
            }
        }
        forms
            .into_iter()
            .enumerate()
            .map(|(index, forms)| {
                let number = index as u16 + 1;
                Radical {
                    number,
                    kangxi: KANGXI_BLOCK_START + u32::from(number) - 1,
                    forms,
                }
            })
            .collect()
    });
    &VALUES
}

/// One Kangxi radical by number, or `None` outside 1 through 214.
pub fn kangxi_radical(number: u16) -> Option<&'static Radical> {
    kangxi_radicals().get(usize::from(number).wrapping_sub(1))
}

#[cfg(test)]
mod radical_tests {
    use super::*;

    #[test]
    fn every_radical_number_has_its_block_character() {
        let radicals = kangxi_radicals();
        assert_eq!(radicals.len(), usize::from(KANGXI_RADICAL_COUNT));
        for (index, radical) in radicals.iter().enumerate() {
            assert_eq!(radical.number, index as u16 + 1);
            assert_eq!(radical.kangxi, 0x2f00 + index as u32);
        }
        assert_eq!(kangxi_radical(1).unwrap().kangxi, 0x2f00);
        assert_eq!(kangxi_radical(214).unwrap().kangxi, 0x2fd5);
        assert!(kangxi_radical(0).is_none());
        assert!(kangxi_radical(215).is_none());
    }

    #[test]
    fn kangxi_block_is_radical_ordered() {
        for number in 1..=KANGXI_RADICAL_COUNT {
            let scalar = 0x2f00 + u32::from(number) - 1;
            assert_eq!(crate::block_name(scalar), "Kangxi Radicals");
            assert!(crate::is_assigned(scalar), "U+{scalar:04X}");
        }
        // The block allocates to U+2FDF but assigns only the 214 radicals.
        assert!(!crate::is_assigned(0x2fd6));
    }

    #[test]
    fn forms_hold_the_radical_and_its_variant_writings() {
        let scalars = |number: u16| -> Vec<u32> {
            kangxi_radical(number)
                .unwrap()
                .forms
                .iter()
                .map(|form| form.unicode)
                .collect()
        };
        // 人 with 亻, 心 with 忄, 水 with 氵 and 氺, 言 with 訁 and 讠.
        assert_eq!(scalars(9), [0x4eba, 0x4ebb]);
        assert_eq!(scalars(61), [0x5fc3, 0x5fc4]);
        assert_eq!(scalars(85), [0x6c34, 0x6c35, 0x6c3a]);
        assert_eq!(scalars(149), [0x8a00, 0x8a01, 0x8ba0]);
        // One stroke, one form.
        assert_eq!(scalars(1), [0x4e00]);
    }

    #[test]
    fn forms_are_ascending_and_carry_their_own_stroke_counts() {
        for radical in kangxi_radicals() {
            assert!(
                !radical.forms.is_empty(),
                "radical {} has no form",
                radical.number
            );
            assert!(
                radical
                    .forms
                    .windows(2)
                    .all(|pair| pair[0].unicode < pair[1].unicode),
                "radical {} is unsorted or repeats a scalar",
                radical.number
            );
            for form in &radical.forms {
                assert!(form.unicode <= 0xffff);
                assert!(!(0xf900..=0xfaff).contains(&form.unicode));
                assert_eq!(
                    form.strokes,
                    unihan(form.unicode).and_then(|record| record.strokes().min()),
                );
            }
        }
        assert_eq!(
            kangxi_radical(214).unwrap().forms[0],
            RadicalForm {
                unicode: 0x9fa0,
                strokes: Some(17)
            }
        );
    }
}
