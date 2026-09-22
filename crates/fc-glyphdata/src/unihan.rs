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
