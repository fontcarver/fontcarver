//! Coordinate conversion and design-space invariants, independent of compilation.
use crate::{Axis, DesignLocation, Error, Font, Master};
use std::collections::HashSet;

fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidDesignSpace(reason.into())
}

impl Axis {
    /// Validate the axis and its strictly increasing user-to-design mapping.
    pub fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() || !self.tag.0.iter().all(|b| (0x20..=0x7e).contains(b)) {
            return Err(invalid(
                "axis name and printable four-byte tag are required",
            ));
        }
        if [self.min, self.default, self.max]
            .iter()
            .any(|n| !n.is_finite() || *n < -32768.0 || *n >= 32768.0)
            || self.min > self.default
            || self.default > self.max
            || self.min == self.max
        {
            return Err(invalid(format!(
                "{} requires min <= default <= max and min < max",
                self.tag
            )));
        }
        if self.map.len() > 1024 {
            return Err(invalid("axis mapping exceeds 1024 points"));
        }
        for (i, (user, design)) in self.map.iter().enumerate() {
            if !user.is_finite()
                || !design.is_finite()
                || *user < self.min
                || *user > self.max
                || design.abs() > 1_000_000.0
                || (i > 0 && (self.map[i - 1].0 >= *user || self.map[i - 1].1 >= *design))
            {
                return Err(invalid(format!(
                    "{} mapping must increase strictly in both coordinates",
                    self.tag
                )));
            }
        }
        Ok(())
    }

    /// Map user coordinates into design coordinates, with unit-slope extrapolation.
    pub fn user_to_design(&self, value: f64) -> f64 {
        map_value(&self.map, value, false)
    }

    /// Invert a validated, strictly monotonic mapping.
    pub fn design_to_user(&self, value: f64) -> f64 {
        map_value(&self.map, value, true)
    }

    /// Normalize design coordinates to the axis' mapped minimum/default/maximum.
    pub fn normalize_design(&self, value: f64) -> f64 {
        let default = self.user_to_design(self.default);
        if value == default {
            0.0
        } else if value < default {
            let min = self.user_to_design(self.min);
            if min == default {
                -1.0
            } else {
                ((value - default) / (default - min)).clamp(-1.0, 0.0)
            }
        } else {
            let max = self.user_to_design(self.max);
            if max == default {
                1.0
            } else {
                ((value - default) / (max - default)).clamp(0.0, 1.0)
            }
        }
    }
}

fn map_value(points: &[(f64, f64)], value: f64, inverse: bool) -> f64 {
    let pair = |p: (f64, f64)| if inverse { (p.1, p.0) } else { p };
    let Some(first) = points.first().copied().map(pair) else {
        return value;
    };
    if value <= first.0 {
        return value + first.1 - first.0;
    }
    for window in points.windows(2) {
        let a = pair(window[0]);
        let b = pair(window[1]);
        if value <= b.0 {
            return a.1 + (value - a.0) / (b.0 - a.0) * (b.1 - a.1);
        }
    }
    let last = pair(points[points.len() - 1]);
    value + last.1 - last.0
}

impl Font {
    /// Resolve a brace source by inheriting coordinates from its associated master.
    pub fn intermediate_location(
        &self,
        master: &crate::MasterId,
        overrides: &DesignLocation,
    ) -> Result<DesignLocation, Error> {
        let source = self
            .master(master)
            .ok_or_else(|| invalid("intermediate layer master is missing"))?;
        let mut location = self.complete_location(&source.location)?;
        location.extend(overrides.iter().map(|(tag, value)| (*tag, *value)));
        self.complete_location(&location)
    }
    /// Return a full design location, filling unspecified axes at their mapped defaults.
    pub fn complete_location(&self, location: &DesignLocation) -> Result<DesignLocation, Error> {
        if location
            .keys()
            .any(|tag| !self.axes.iter().any(|a| a.tag == *tag))
        {
            return Err(invalid("location contains an unknown axis"));
        }
        self.axes
            .iter()
            .map(|axis| {
                axis.validate()?;
                let value = location
                    .get(&axis.tag)
                    .copied()
                    .unwrap_or_else(|| axis.user_to_design(axis.default));
                if !value.is_finite()
                    || value < axis.user_to_design(axis.min)
                    || value > axis.user_to_design(axis.max)
                {
                    return Err(invalid(format!(
                        "{} coordinate is outside its mapped range",
                        axis.tag
                    )));
                }
                Ok((axis.tag, value))
            })
            .collect()
    }

    /// Locate the actual default source rather than assuming insertion order.
    pub fn design_default_master(&self) -> Result<&Master, Error> {
        let default = self.complete_location(&DesignLocation::new())?;
        self.masters
            .iter()
            .find(|m| {
                self.complete_location(&m.location)
                    .is_ok_and(|p| p == default)
            })
            .ok_or_else(|| invalid("a master at the mapped default location is required"))
    }

    /// Validate source metadata before design-space editing or variable compilation.
    /// Missing non-default glyph layers are allowed as sparse sources.
    pub fn validate_designspace(&self) -> Result<(), Error> {
        if self.extra.lib.contains_key("ufo.designspace.rules") {
            return Err(invalid("preserved native designspace rules require conversion before interpolation or compilation"));
        }

        if self.axes.len() > 16
            || self.masters.is_empty()
            || self.masters.len() > 1024
            || self.instances.len() > 4096
            || self.virtual_masters.len() > 1024
        {
            return Err(invalid("design space exceeds axis/master/instance limits"));
        }
        let mut tags = HashSet::new();
        for axis in &self.axes {
            axis.validate()?;
            if !tags.insert(axis.tag) {
                return Err(invalid("duplicate axis tag"));
            }
        }
        let mut ids = HashSet::new();
        let mut positions = HashSet::new();
        for master in &self.masters {
            master.metrics.validate()?;
            if master.name.trim().is_empty() || !ids.insert(&master.id) {
                return Err(invalid("master names and unique IDs are required"));
            }
            let location = self.complete_location(&master.location)?;
            let key: Vec<_> = location
                .values()
                .map(|v| if *v == 0.0 { 0 } else { v.to_bits() })
                .collect();
            if !self.axes.is_empty() && !positions.insert(key) {
                return Err(invalid("masters cannot share a design location"));
            }
        }
        if !self.axes.is_empty() {
            self.design_default_master()?;
        }
        let mut virtual_names = HashSet::new();
        for master in &self.virtual_masters {
            if self.axes.is_empty()
                || master.name.trim().is_empty()
                || master.name.len() > 256
                || !virtual_names.insert(master.name.trim())
            {
                return Err(invalid(
                    "virtual masters require axes and unique nonempty names",
                ));
            }
            let location = self.complete_location(&master.location)?;
            let key: Vec<_> = location
                .values()
                .map(|value| if *value == 0.0 { 0 } else { value.to_bits() })
                .collect();
            if !positions.insert(key) {
                return Err(invalid(
                    "virtual master positions must be distinct from other masters",
                ));
            }
        }
        for instance in &self.instances {
            if instance.name.trim().is_empty() {
                return Err(invalid("instance name is required"));
            }
            self.complete_location(&instance.location)?;
        }
        for glyph in self.glyphs.values() {
            glyph.validate_smart_axes()?;
            let mut source_ids = HashSet::new();
            let mut source_locations = HashSet::new();
            for layer in &glyph.layers {
                if !self.axes.is_empty() {
                    let location = match &layer.kind {
                        crate::LayerKind::Master(id) => self
                            .master(id)
                            .map(|master| self.complete_location(&master.location))
                            .transpose()?,
                        crate::LayerKind::Brace { master, location } => {
                            Some(self.intermediate_location(master, location)?)
                        }
                        _ => None,
                    };
                    if let Some(location) = location {
                        let key: Vec<_> = location
                            .values()
                            .map(|value| if *value == 0.0 { 0 } else { value.to_bits() })
                            .collect();
                        if !source_locations.insert(key) {
                            return Err(invalid(format!(
                                "{} has duplicate source locations",
                                glyph.name
                            )));
                        }
                    }
                }
                match &layer.kind {
                    crate::LayerKind::Smart { master, .. } => {
                        if !ids.contains(master) {
                            return Err(invalid("smart source master is missing"));
                        }
                    }
                    crate::LayerKind::Master(master) => {
                        if !ids.contains(master) || !source_ids.insert(master) {
                            return Err(invalid(format!(
                                "{} has a missing or repeated master",
                                glyph.name
                            )));
                        }
                    }
                    crate::LayerKind::Brace { master, location } => {
                        if !ids.contains(master) {
                            return Err(invalid("intermediate layer master is missing"));
                        }
                        self.intermediate_location(master, location)?;
                    }
                    crate::LayerKind::Bracket {
                        master,
                        axis,
                        min,
                        max,
                    } => {
                        let a = self
                            .axes
                            .iter()
                            .find(|a| a.tag == *axis)
                            .ok_or_else(|| invalid("alternate layer axis is missing"))?;
                        if !ids.contains(master)
                            || min.is_none() && max.is_none()
                            || min.is_some_and(|n| {
                                !n.is_finite()
                                    || n < a.user_to_design(a.min)
                                    || n > a.user_to_design(a.max)
                            })
                            || max.is_some_and(|n| {
                                !n.is_finite()
                                    || n < a.user_to_design(a.min)
                                    || n > a.user_to_design(a.max)
                            })
                            || min.zip(*max).is_some_and(|(a, b)| a > b)
                        {
                            return Err(invalid("invalid alternate layer condition"));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
