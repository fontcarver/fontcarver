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
    /// Remove invalid records and every explicit zero whose removal changes nothing.
    ///
    /// A zero record that suppresses a broader rule is a real exception and is
    /// kept; a zero that only restates the value the pair would resolve to
    /// anyway is obsolete. This is the "Clean Up" the kerning table offers.
    CleanupObsolete,
    /// Fold pairs into class pairs where safe, then remove redundant exceptions.
    ///
    /// Promotion inserts a group/group record only when every concrete member
    /// combination of the two groups already resolves to that same nonzero
    /// value, so no pair's resolved kerning changes. The redundancy pass then
    /// removes the records the new class record covers, keeping true
    /// exceptions.
    Compress,
    /// Paste records whose sides must already exist in the destination font.
    Paste {
        /// Clipboard records; duplicate keys and invalid values are rejected.
        #[ts(type = "Array<KernPair>")]
        records: Vec<KernPair>,
        /// Replace the complete table; false merges and overwrites incoming keys.
        replace: bool,
    },
    /// Merge records, resolving every conflict in favour of the destination.
    ///
    /// This is the "keep existing" half of copy/paste conflict resolution;
    /// [`Self::Paste`] with `replace: false` is the "overwrite" half. Sides are
    /// validated against the destination font exactly as for a paste.
    PasteKeepingExisting {
        /// Clipboard records; duplicate keys and invalid values are rejected.
        #[ts(type = "Array<KernPair>")]
        records: Vec<KernPair>,
    },
}

/// Largest group product a single promotion candidate may examine.
///
/// Promotion is quadratic in the two groups' membership, and a CJK side group
/// can hold thousands of glyphs. A candidate above this bound is left alone
/// rather than making Compress unbounded; its records stay exactly as they are.
const MAX_PROMOTION_PAIRS: usize = 4_096;

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
        KerningMaintenanceOperation::CleanupInvalid => retain_valid(font, &mut table),
        KerningMaintenanceOperation::CleanupObsolete => {
            retain_valid(font, &mut table);
            remove_obsolete_zeros(font, &mut table);
        }
        KerningMaintenanceOperation::Compress => {
            promote(font, &mut table);
            compress(font, &mut table);
        }
        KerningMaintenanceOperation::Paste { records, replace } => {
            let incoming = validated_clipboard(font, records)?;
            if *replace {
                table = incoming;
            } else {
                table.0.extend(incoming.0);
            }
        }
        KerningMaintenanceOperation::PasteKeepingExisting { records } => {
            for (key, value) in validated_clipboard(font, records)?.0 {
                table.0.entry(key).or_insert(value);
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

/// Validates clipboard records against the destination and rejects duplicates.
fn validated_clipboard(font: &Font, records: &[KernPair]) -> Result<Kerning, Error> {
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
    Ok(incoming)
}

fn retain_valid(font: &Font, table: &mut Kerning) {
    table.0.retain(|(left, right), value| {
        validate_horizontal_record(
            font,
            &KernPair {
                left: left.clone(),
                right: right.clone(),
                value: *value,
            },
        )
        .is_ok()
    });
}

/// Keys in the order Compress and Clean Up must consider them.
///
/// Lower-priority exceptions can be completely hidden by higher-priority ones.
/// Removing those first means one pass reaches a stable result.
fn by_exception_priority(table: &Kerning) -> Vec<(KernSide, KernSide)> {
    let mut candidates: Vec<_> = table.0.keys().cloned().collect();
    candidates.sort_by_key(|key| match key {
        (KernSide::Group(_), KernSide::Glyph(_)) => 0,
        (KernSide::Glyph(_), KernSide::Group(_)) => 1,
        (KernSide::Glyph(_), KernSide::Glyph(_)) => 2,
        (KernSide::Group(_), KernSide::Group(_)) => 3,
    });
    candidates
}

/// Removes explicit zeros that only restate an already-zero resolved value.
fn remove_obsolete_zeros(font: &Font, table: &mut Kerning) {
    for key in by_exception_priority(table) {
        if table.0.get(&key) != Some(&0.0) {
            continue;
        }
        let first = concrete_members(font, &key.0, KerningGroupSide::Side1);
        let second = concrete_members(font, &key.1, KerningGroupSide::Side2);
        // An empty group states future membership intent; leave it alone.
        if first.is_empty() || second.is_empty() {
            continue;
        }
        let obsolete = first.iter().all(|first| {
            second.iter().all(|second| {
                matches!(
                    effective_value(font, table, first, second, Some(&key)),
                    Ok(value) if value == 0.0
                )
            })
        });
        if obsolete {
            table.0.remove(&key);
        }
    }
}

/// Inserts a class record wherever every member pair already agrees on a value.
fn promote(font: &Font, table: &mut Kerning) {
    let mut candidates = std::collections::BTreeSet::new();
    for (left, right) in table.0.keys() {
        if matches!((left, right), (KernSide::Group(_), KernSide::Group(_))) {
            continue;
        }
        let Ok(Some(first)) = side_group(font, left, KerningGroupSide::Side1) else {
            continue;
        };
        let Ok(Some(second)) = side_group(font, right, KerningGroupSide::Side2) else {
            continue;
        };
        candidates.insert((first, second));
    }
    for (first_group, second_group) in candidates {
        let key = (
            KernSide::Group(first_group.clone()),
            KernSide::Group(second_group.clone()),
        );
        if table.0.contains_key(&key) {
            continue;
        }
        let first = concrete_members(font, &key.0, KerningGroupSide::Side1);
        let second = concrete_members(font, &key.1, KerningGroupSide::Side2);
        if first.is_empty()
            || second.is_empty()
            || first.len().saturating_mul(second.len()) > MAX_PROMOTION_PAIRS
        {
            continue;
        }
        let mut agreed: Option<f64> = None;
        let mut foldable = true;
        for left in &first {
            for right in &second {
                match effective_value(font, table, left, right, None) {
                    Ok(value) if *agreed.get_or_insert(value) == value => {}
                    _ => foldable = false,
                }
                if !foldable {
                    break;
                }
            }
            if !foldable {
                break;
            }
        }
        // A zero agreement needs no record at all; Clean Up removes those.
        if let (true, Some(value)) = (foldable, agreed.filter(|value| *value != 0.0)) {
            table.0.insert(key, value);
        }
    }
}

/// The group a stored pair side belongs to on that side, if any.
fn side_group(
    font: &Font,
    value: &KernSide,
    side: KerningGroupSide,
) -> Result<Option<String>, Error> {
    Ok(match value {
        KernSide::Group(name) => Some(name.clone()),
        KernSide::Glyph(name) => group_membership(font, name, side)?.map(str::to_owned),
    })
}

/// Resolved kerning for a concrete pair, where "no rule" is zero.
fn effective_value(
    font: &Font,
    table: &Kerning,
    first: &GlyphName,
    second: &GlyphName,
    omitted: Option<&(KernSide, KernSide)>,
) -> Result<f64, Error> {
    Ok(resolved_value(font, table, first, second, omitted)?.unwrap_or(0.0))
}

fn compress(font: &Font, table: &mut Kerning) {
    let candidates = by_exception_priority(table);
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
