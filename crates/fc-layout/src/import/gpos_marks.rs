//! GPOS cursive and mark attachments with static coordinate anchors.

use fc_model::GlyphName;

use super::super::binary::{glyph, Reader, Result, Rules, MAX_RULES};

fn anchor(reader: Reader<'_>, field: usize) -> Result<String> {
    let offset = reader.u16(field)? as usize;
    if offset == 0 {
        return Ok("<anchor NULL>".into());
    }
    let anchor = reader.at(offset)?;
    match anchor.u16(0)? {
        1 => {}
        3 if anchor.u16(6)? == 0 && anchor.u16(8)? == 0 => {}
        2 => return Err("contour-point positioning anchors are unsupported".into()),
        3 => return Err("device or variation positioning anchors are unsupported".into()),
        _ => return Err("unsupported positioning anchor format".into()),
    }
    Ok(format!("<anchor {} {}>", anchor.i16(2)?, anchor.i16(4)?))
}

/// Preserve coordinate-based cursive entry and exit anchors.
pub(super) fn cursive(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    if reader.u16(0)? != 1 {
        return Err("unsupported cursive positioning format".into());
    }
    let coverage = reader.offset16(2)?.coverage(names)?;
    if reader.u16(4)? as usize != coverage.len() {
        return Err("cursive entry/exit count does not match coverage".into());
    }
    reader.bytes(6, coverage.len() * 4)?;
    let mut rules = Rules::default();
    for (index, gid) in coverage.into_iter().enumerate() {
        rules.push(
            format!(
                "pos cursive {} {} {};",
                glyph(names, gid)?,
                anchor(reader, 6 + index * 4)?,
                anchor(reader, 8 + index * 4)?
            ),
            vec![gid],
        )?;
    }
    Ok(rules)
}

/// Preserve mark-to-base, mark-to-ligature and mark-to-mark attachment matrices.
pub(super) fn attachment(
    reader: Reader<'_>,
    kind: u16,
    names: &[GlyphName],
    lookup_name: &str,
) -> Result<Rules> {
    if reader.u16(0)? != 1 {
        return Err("unsupported mark attachment format".into());
    }
    if !lookup_name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err("invalid generated mark class identifier".into());
    }
    let marks = reader.offset16(2)?.coverage(names)?;
    let bases = reader.offset16(4)?.coverage(names)?;
    let count = reader.u16(6)? as usize;
    if count == 0 || count > MAX_RULES {
        return Err("invalid mark attachment class count".into());
    }
    let mark_array = reader.offset16(8)?;
    if mark_array.u16(0)? as usize != marks.len() {
        return Err("mark array count does not match coverage".into());
    }
    mark_array.bytes(2, marks.len() * 4)?;
    let mut rules = Rules::default();
    let mut class_members = vec![Vec::new(); count];
    for (index, gid) in marks.into_iter().enumerate() {
        let class = mark_array.u16(2 + index * 4)? as usize;
        let members = class_members
            .get_mut(class)
            .ok_or("mark class index outside declared range")?;
        if mark_array.u16(4 + index * 4)? == 0 {
            return Err("mark anchor offset is null".into());
        }
        let position = anchor(mark_array, 4 + index * 4)?;
        rules.preamble.push(format!(
            "markClass {} {position} @fc_{lookup_name}_mark_{class:05};",
            glyph(names, gid)?
        ));
        members.push(gid);
    }
    if class_members.iter().any(Vec::is_empty) {
        return Err("empty stored mark attachment classes cannot be represented in FEA".into());
    }
    let base_array = reader.offset16(10)?;
    if base_array.u16(0)? as usize != bases.len() {
        return Err("attachment base count does not match coverage".into());
    }
    let mut slots = 0;
    let mut references = 0;
    let mark_glyphs: Vec<_> = class_members.into_iter().flatten().collect();
    let mut used_classes = vec![false; count];
    for (index, gid) in bases.into_iter().enumerate() {
        let (array, components, start) = if kind == 5 {
            let ligature = base_array.offset16(2 + index * 2)?;
            let components = ligature.u16(0)? as usize;
            if components == 0 {
                return Err("ligature attachment has no components".into());
            }
            (ligature, components, 2)
        } else {
            (base_array, 1, 2 + index * count * 2)
        };
        let size = components
            .checked_mul(count)
            .ok_or("mark attachment matrix overflow")?;
        slots += size;
        if slots > MAX_RULES {
            return Err("mark attachment anchor resource limit".into());
        }
        array.bytes(start, size * 2)?;
        let mut component_rules = Vec::new();
        for component in 0..components {
            let mut anchors = Vec::new();
            for (class, used) in used_classes.iter_mut().enumerate() {
                let field = start + (component * count + class) * 2;
                if array.u16(field)? == 0 {
                    continue;
                }
                let position = anchor(array, field)?;
                *used = true;
                anchors.push(format!("{position} mark @fc_{lookup_name}_mark_{class:05}"));
            }
            // fea-rs cannot compile NULL followed by a mark class. Omitted
            // class attachments become null offsets in its generated matrix;
            // an empty ligature component uses the standalone NULL syntax.
            if anchors.is_empty() {
                if kind != 5 {
                    return Err(
                        "all-null base or mark attachment rows cannot be represented in FEA".into(),
                    );
                }
                anchors.push("<anchor NULL>".into());
            }
            component_rules.push(anchors.join(" "));
        }
        let command = match kind {
            4 => "base",
            5 => "ligature",
            6 => "mark",
            _ => return Err("invalid mark attachment type".into()),
        };
        references += 1 + mark_glyphs.len();
        if references > super::MAX_GLYPH_REFERENCES {
            return Err("mark attachment glyph-reference resource limit".into());
        }
        rules.push(
            format!(
                "pos {command} {} {};",
                glyph(names, gid)?,
                component_rules.join(" ligComponent ")
            ),
            std::iter::once(gid)
                .chain(mark_glyphs.iter().copied())
                .collect(),
        )?;
    }
    if used_classes.iter().any(|used| !used) {
        return Err("mark attachment class has no compiler-representable base anchors".into());
    }
    Ok(rules)
}
