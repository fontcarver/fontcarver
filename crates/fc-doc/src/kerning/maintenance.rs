//! Conservative, atomic edits of a complete stored kerning table.

use fc_model::{Font, GlyphName, KernPair, KernSide, Kerning, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{group_membership, validate_horizontal_record, KerningGroupSide};
use crate::{Command, Error};

/// One explicitly requested bulk operation on a master's stored kerning table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum KerningMaintenanceOperation {
    /// Remove non-finite records and records with invalid glyph or group references.
    CleanupInvalid,
    /// Remove redundant exceptions while preserving every concrete resolved value.
    Compress,
    /// Paste records whose sides must already exist in the destination font.
    Paste {
        /// Clipboard records; duplicate keys and invalid values are rejected.
        #[ts(type = "Array<KernPair>")]
        records: Vec<KernPair>,
        /// Replace the complete table; false merges and overwrites incoming keys.
        replace: bool,
    },
}

pub(crate) fn validate_generation(command: &Command, generation: u64) -> Result<(), Error> {
    match command {
        Command::KerningMaintenance {
            expected_generation,
            ..
        } if *expected_generation != generation => Err(Error::InvalidCommand(format!(
            "stale kerning maintenance generation: expected {expected_generation}, current {generation}"
        ))),
        Command::Batch(commands) => commands
            .iter()
            .try_for_each(|command| validate_generation(command, generation)),
        _ => Ok(()),
    }
}

pub(crate) fn apply(
    font: &mut Font,
    master: &MasterId,
    vertical: bool,
    operation: &KerningMaintenanceOperation,
) -> Result<(), Error> {
    let source = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?;
    let mut table = if vertical {
        source.kerning_v.clone()
    } else {
        source.kerning_h.clone()
    };
    match operation {
        KerningMaintenanceOperation::CleanupInvalid => table.0.retain(|(left, right), value| {
            validate_horizontal_record(
                font,
                &KernPair {
                    left: left.clone(),
                    right: right.clone(),
                    value: *value,
                },
            )
            .is_ok()
        }),
        KerningMaintenanceOperation::Compress => compress(font, &mut table),
        KerningMaintenanceOperation::Paste { records, replace } => {
            let mut incoming = Kerning::default();
            for record in records {
                validate_horizontal_record(font, record)?;
                if incoming
                    .0
                    .insert((record.left.clone(), record.right.clone()), record.value)
                    .is_some()
                {
                    return Err(Error::InvalidCommand(
                        "duplicate pasted kerning pair".into(),
                    ));
                }
            }
            if *replace {
                table = incoming;
            } else {
                table.0.extend(incoming.0);
            }
        }
    }
    let target = font
        .masters
        .iter_mut()
        .find(|candidate| candidate.id == *master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?;
    if vertical {
        target.kerning_v = table;
    } else {
        target.kerning_h = table;
    }
    Ok(())
}

fn compress(font: &Font, table: &mut Kerning) {
    let mut candidates: Vec<_> = table.0.keys().cloned().collect();
    // Lower-priority exceptions can be completely hidden by higher-priority
    // ones. Remove those first so their removal cannot leave an earlier checked
    // exception newly redundant, and a single pass reaches a stable result.
    candidates.sort_by_key(|key| match key {
        (KernSide::Group(_), KernSide::Glyph(_)) => 0,
        (KernSide::Glyph(_), KernSide::Group(_)) => 1,
        (KernSide::Glyph(_), KernSide::Glyph(_)) => 2,
        (KernSide::Group(_), KernSide::Group(_)) => 3,
    });
    for key in candidates {
        // Roots define the table's baseline and empty groups express future
        // membership intent. Neither is an exception we can safely remove.
        if matches!(&key, (KernSide::Group(_), KernSide::Group(_))) {
            continue;
        }
        let Some(&value) = table.0.get(&key) else {
            continue;
        };
        let record = KernPair {
            left: key.0.clone(),
            right: key.1.clone(),
            value,
        };
        if validate_horizontal_record(font, &record).is_err() {
            continue;
        }
        let first = concrete_members(font, &key.0, KerningGroupSide::Side1);
        let second = concrete_members(font, &key.1, KerningGroupSide::Side2);
        if first.is_empty() || second.is_empty() {
            continue;
        }
        let redundant = first.iter().all(|first| {
            second.iter().all(|second| {
                match (
                    resolved_value(font, table, first, second, None),
                    resolved_value(font, table, first, second, Some(&key)),
                ) {
                    (Ok(Some(before)), Ok(Some(after))) => before == after,
                    _ => false,
                }
            })
        });
        if redundant {
            // Apply sequentially: another exception may depend on this fallback.
            table.0.remove(&key);
        }
    }
}

fn concrete_members<'a>(
    font: &'a Font,
    value: &'a KernSide,
    side: KerningGroupSide,
) -> Vec<&'a GlyphName> {
    match value {
        KernSide::Glyph(name) => vec![name],
        KernSide::Group(name) => side
            .groups(font)
            .get(name)
            .map(|members| members.iter().collect())
            .unwrap_or_default(),
    }
}

fn resolved_value(
    font: &Font,
    table: &Kerning,
    first: &GlyphName,
    second: &GlyphName,
    omitted: Option<&(KernSide, KernSide)>,
) -> Result<Option<f64>, Error> {
    let first_group = group_membership(font, first, KerningGroupSide::Side1)?;
    let second_group = group_membership(font, second, KerningGroupSide::Side2)?;
    let first = KernSide::Glyph(first.clone());
    let second = KernSide::Glyph(second.clone());
    let candidates = [
        Some((first.clone(), second.clone())),
        second_group.map(|group| (first, KernSide::Group(group.into()))),
        first_group.map(|group| (KernSide::Group(group.into()), second)),
        first_group.zip(second_group).map(|(first, second)| {
            (
                KernSide::Group(first.into()),
                KernSide::Group(second.into()),
            )
        }),
    ];
    for key in candidates.into_iter().flatten() {
        if omitted == Some(&key) {
            continue;
        }
        if let Some(&value) = table.0.get(&key) {
            validate_horizontal_record(
                font,
                &KernPair {
                    left: key.0,
                    right: key.1,
                    value,
                },
            )?;
            return Ok(Some(value));
        }
    }
    Ok(None)
}
