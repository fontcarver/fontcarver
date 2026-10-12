//! Generated Adobe Glyph List For New Fonts lookup tables.

include!(concat!(env!("OUT_DIR"), "/aglfn.rs"));

pub(super) fn codepoint_for_name(name: &str) -> Option<u32> {
    let index = AGLFN_BY_NAME
        .binary_search_by(|entry| entry.0.cmp(name))
        .ok()?;
    AGLFN_BY_NAME.get(index).map(|entry| entry.1)
}

pub(super) fn name_for_codepoint(cp: u32) -> Option<&'static str> {
    let index = AGLFN_BY_CODEPOINT
        .binary_search_by_key(&cp, |entry| entry.0)
        .ok()?;
    AGLFN_BY_CODEPOINT.get(index).map(|entry| entry.1)
}
