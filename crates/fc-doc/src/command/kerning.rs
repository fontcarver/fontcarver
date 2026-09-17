use crate::{
    kerning::{
        check_horizontal_editing, group_membership, validate_group, validate_pair_side,
        KerningGroupSide as Side,
    },
    Command, Error,
};
use fc_model::{Font, GlyphName, KernSide};
use std::collections::BTreeSet;

pub(crate) fn is_kerning(command: &Command) -> bool {
    match command {
        Command::SetHorizontalKerning { .. }
        | Command::CreateKerningGroup { .. }
        | Command::SetKerningGroup { .. }
        | Command::AssignKerningGroup { .. }
        | Command::RenameKerningGroup { .. }
        | Command::RemoveKerningGroup { .. } => true,
        Command::Batch(commands) => !commands.is_empty() && commands.iter().all(is_kerning),
        _ => false,
    }
}

pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    check_horizontal_editing(font)?;
    match command {
        Command::SetHorizontalKerning {
            master,
            left,
            right,
            value,
        } => {
            if font.master(master).is_none() {
                return Err(Error::MissingMaster(master.clone()));
            }
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(Error::InvalidCommand("kerning must be finite".into()));
            }
            validate_pair_side(font, left, Side::Side1)?;
            validate_pair_side(font, right, Side::Side2)?;
            let target = font
                .masters
                .iter_mut()
                .find(|candidate| candidate.id == *master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            let key = (left.clone(), right.clone());
            match value {
                Some(value) => {
                    target.kerning_h.0.insert(key, *value);
                }
                None => {
                    target.kerning_h.0.remove(&key);
                }
            }
            Ok(())
        }
        Command::CreateKerningGroup {
            side,
            name,
            members,
        } => {
            if side.groups(font).contains_key(name) {
                return Err(Error::InvalidCommand(format!(
                    "kerning group `{name}` already exists"
                )));
            }
            set_group(font, *side, name, members)
        }
        Command::SetKerningGroup {
            side,
            name,
            members,
        } => set_group(font, *side, name, members),
        Command::AssignKerningGroup { side, glyph, group } => {
            assign(font, *side, glyph, group.as_deref())
        }
        Command::RenameKerningGroup { side, from, to } => rename(font, *side, from, to),
        Command::RemoveKerningGroup {
            side,
            name,
            remove_pairs,
        } => remove(font, *side, name, *remove_pairs),
        _ => Err(Error::Internal(
            "non-kerning command routed to kerning handler".into(),
        )),
    }
}

fn set_group(font: &mut Font, side: Side, name: &str, members: &[GlyphName]) -> Result<(), Error> {
    side.validate_name(name)?;
    let unique: BTreeSet<_> = members.iter().cloned().collect();
    if unique.len() != members.len() {
        return Err(Error::InvalidCommand("duplicate group member".into()));
    }
    for member in members {
        if group_membership(font, member, side)?.is_some_and(|current| current != name) {
            return Err(Error::InvalidCommand(format!(
                "glyph `{member}` already belongs to another {} group",
                side.prefix()
            )));
        }
    }
    if side.groups(font).contains_key(name) {
        validate_group(font, side, name)?;
    }
    let previous = side
        .groups_mut(font)
        .insert(name.into(), unique)
        .unwrap_or_default();
    for member in previous {
        side.set_membership(font, &member, None);
    }
    for member in members {
        side.set_membership(font, member, Some(name.into()));
    }
    Ok(())
}

fn assign(
    font: &mut Font,
    side: Side,
    glyph: &GlyphName,
    group: Option<&str>,
) -> Result<(), Error> {
    group_membership(font, glyph, side)?;
    if let Some(group) = group {
        validate_group(font, side, group)?;
    }
    for members in side.groups_mut(font).values_mut() {
        members.remove(glyph);
    }
    if let Some(group) = group {
        if let Some(members) = side.groups_mut(font).get_mut(group) {
            members.insert(glyph.clone());
        }
    }
    side.set_membership(font, glyph, group.map(str::to_owned));
    Ok(())
}

fn rename(font: &mut Font, side: Side, from: &str, to: &str) -> Result<(), Error> {
    validate_group(font, side, from)?;
    side.validate_name(to)?;
    if from == to {
        return Ok(());
    }
    if side.groups(font).contains_key(to) {
        return Err(Error::InvalidCommand(format!(
            "kerning group `{to}` already exists"
        )));
    }
    // A dangling destination pair is still user data: never overwrite it.
    for master in &font.masters {
        if master
            .kerning_h
            .0
            .keys()
            .any(|key| references(key, side, to))
        {
            return Err(Error::InvalidCommand(format!(
                "kerning pairs already reference `{to}`"
            )));
        }
    }
    let members = side
        .groups_mut(font)
        .remove(from)
        .ok_or_else(|| Error::InvalidCommand(format!("kerning group `{from}` does not exist")))?;
    for member in &members {
        side.set_membership(font, member, Some(to.into()));
    }
    side.groups_mut(font).insert(to.into(), members);
    for master in &mut font.masters {
        master.kerning_h.0 = std::mem::take(&mut master.kerning_h.0)
            .into_iter()
            .map(|(mut key, value)| {
                if references(&key, side, from) {
                    match side {
                        Side::Side1 => key.0 = KernSide::Group(to.into()),
                        Side::Side2 => key.1 = KernSide::Group(to.into()),
                    }
                }
                (key, value)
            })
            .collect();
    }
    Ok(())
}

fn remove(font: &mut Font, side: Side, name: &str, remove_pairs: bool) -> Result<(), Error> {
    validate_group(font, side, name)?;
    if !remove_pairs
        && font.masters.iter().any(|master| {
            master
                .kerning_h
                .0
                .keys()
                .any(|key| references(key, side, name))
        })
    {
        return Err(Error::InvalidCommand(format!(
            "kerning group `{name}` is used by pairs; explicit pair removal is required"
        )));
    }
    let members = side.groups_mut(font).remove(name).unwrap_or_default();
    for member in members {
        side.set_membership(font, &member, None);
    }
    for master in &mut font.masters {
        master
            .kerning_h
            .0
            .retain(|key, _| !references(key, side, name));
    }
    Ok(())
}

fn references(key: &(KernSide, KernSide), side: Side, name: &str) -> bool {
    let value = match side {
        Side::Side1 => &key.0,
        Side::Side2 => &key.1,
    };
    matches!(value, KernSide::Group(group) if group == name)
}
