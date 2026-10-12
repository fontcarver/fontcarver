//! Unicode block, script, ideograph, and legacy CJK charset lookups.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::sync::LazyLock;

use icu_properties::props::{Script as IcuScript, UnifiedIdeograph};
use icu_properties::{CodePointMapData, CodePointSetData, PropertyNamesShort};

include!(concat!(env!("OUT_DIR"), "/unicode_tables.rs"));

/// Bit flags describing membership in legacy CJK character sets.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CjkCharsets(u8);

impl CjkCharsets {
    /// The GB 2312 repertoire.
    pub const GB2312: Self = Self(1 << 0);
    /// The GB 18030 level 1 repertoire.
    pub const GB18030_LEVEL1: Self = Self(1 << 1);
    /// The Big5 repertoire.
    pub const BIG5: Self = Self(1 << 2);
    /// The JIS X 0208 repertoire.
    pub const JIS_X_0208: Self = Self(1 << 3);
    /// The KS X 1001 repertoire.
    pub const KS_X_1001: Self = Self(1 << 4);

    /// Returns an empty set of charset flags.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns all currently defined charset flags.
    pub const fn all() -> Self {
        Self((1 << 5) - 1)
    }

    /// Returns the raw flag bits.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns whether no charset flags are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether all flags in `other` are present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is present.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for CjkCharsets {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CjkCharsets {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CjkCharsets {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CjkCharsets {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// Returns the Unicode block name for `cp`, or `"No Block"` when none exists.
pub fn block_name(cp: u32) -> &'static str {
    let insertion = BLOCKS.partition_point(|entry| entry.0 <= cp);
    let Some(index) = insertion.checked_sub(1) else {
        return "No Block";
    };
    let Some((_, end, name)) = BLOCKS.get(index) else {
        return "No Block";
    };

    if cp <= *end {
        name
    } else {
        "No Block"
    }
}

/// Returns all Unicode block names in code-point order, including `"No Block"`.
pub fn all_block_names() -> &'static [&'static str] {
    BLOCK_NAMES
}

/// Returns the ISO 15924 four-letter script code for `cp`.
pub fn script(cp: u32) -> &'static str {
    let script = script_value(cp);
    PropertyNamesShort::<IcuScript>::new()
        .get(script)
        .unwrap_or("Zzzz")
}

/// Returns all ISO 15924 codes used by the compiled Unicode Script property.
pub fn all_scripts() -> &'static [&'static str] {
    static SCRIPTS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let names = PropertyNamesShort::<IcuScript>::new();
        let mut scripts: Vec<_> = CodePointMapData::<IcuScript>::new()
            .iter_ranges()
            .filter_map(|range| names.get(range.value))
            .collect();
        scripts.sort_unstable();
        scripts.dedup();
        scripts
    });
    SCRIPTS.as_slice()
}

/// Returns whether `cp` has Unicode's `Unified_Ideograph` property.
pub fn is_cjk_ideograph(cp: u32) -> bool {
    is_scalar(cp) && CodePointSetData::new::<UnifiedIdeograph>().contains32(cp)
}

/// Returns the implemented legacy CJK charset memberships for `cp`.
pub fn cjk_charset_membership(cp: u32) -> CjkCharsets {
    let mut membership = CjkCharsets::empty();
    if GB2312_CODEPOINTS.binary_search(&cp).is_ok() {
        membership |= CjkCharsets::GB2312;
    }
    if BIG5_CODEPOINTS.binary_search(&cp).is_ok() {
        membership |= CjkCharsets::BIG5;
    }

    for (data, flag) in [
        (GB18030_LEVEL1_CODEPOINTS, CjkCharsets::GB18030_LEVEL1),
        (JIS0208_CODEPOINTS, CjkCharsets::JIS_X_0208),
        (KSX1001_CODEPOINTS, CjkCharsets::KS_X_1001),
    ] {
        if data.binary_search(&cp).is_ok() {
            membership |= flag;
        }
    }
    membership
}

/// Sorted scalar repertoire for exactly one charset flag; other flags yield no values.
pub fn cjk_charset_codepoints(charset: CjkCharsets) -> &'static [u32] {
    match charset {
        CjkCharsets::GB2312 => GB2312_CODEPOINTS,
        CjkCharsets::GB18030_LEVEL1 => GB18030_LEVEL1_CODEPOINTS,
        CjkCharsets::BIG5 => BIG5_CODEPOINTS,
        CjkCharsets::JIS_X_0208 => JIS0208_CODEPOINTS,
        CjkCharsets::KS_X_1001 => KSX1001_CODEPOINTS,
        _ => &[],
    }
}

pub(crate) fn script_value(cp: u32) -> IcuScript {
    if !is_scalar(cp) {
        return IcuScript::Unknown;
    }
    CodePointMapData::<IcuScript>::new().get32(cp)
}

fn is_scalar(cp: u32) -> bool {
    char::from_u32(cp).is_some()
}
