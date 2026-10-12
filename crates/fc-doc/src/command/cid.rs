//! CID metadata authoring and reference validation.
use crate::Error;
use fc_model::{CidInfo, Font};
use std::collections::HashSet;

pub(super) fn set(font: &mut Font, cid: &Option<CidInfo>) -> Result<(), Error> {
    if let Some(info) = cid {
        for value in [&info.registry, &info.ordering] {
            if value.is_empty()
                || value.len() > 127
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && !b"()<>[]{}/%".contains(&b))
            {
                return Err(Error::InvalidCommand("CID registry and ordering must be nonempty PostScript names of at most 127 bytes".into()));
            }
        }
        if info.supplement > i32::MAX as u32 || info.cid_map.len() > 65_536 {
            return Err(Error::InvalidCommand(
                "CID supplement or mapping exceeds format limits".into(),
            ));
        }
        let mut seen = HashSet::new();
        for (cid, name) in &info.cid_map {
            if *cid > u16::MAX as u32
                || (*cid == 0) != (name.as_ref() == ".notdef")
                || !seen.insert(name)
            {
                return Err(Error::InvalidCommand(
                    "CIDs must be unique, at most 65,535, and reserve zero for .notdef".into(),
                ));
            }
            if font.glyph(name).is_none() {
                return Err(Error::MissingGlyph(name.clone()));
            }
        }
    }
    font.cid = cid.clone();
    Ok(())
}
