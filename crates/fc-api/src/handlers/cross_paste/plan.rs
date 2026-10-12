use std::collections::{BTreeMap, BTreeSet, HashSet};

use fc_doc::{
    kerning::{group_membership, KerningGroupSide},
    Command, Document,
};
use fc_model::{Component, Font, Glyph, GlyphName, Layer, LayerId, LayerKind, Master, MasterId};

use crate::{
    ApiError, MissingBasePolicy, PasteContent, PasteDestination, PastePlaceholder,
    PasteSpecialParams, PLACEHOLDER_TAG,
};

use super::{glyph_name, layer_master};

pub(super) struct Plan {
    pub(super) commands: Vec<Command>,
    pub(super) pasted: Vec<String>,
    pub(super) added_bases: Vec<String>,
    pub(super) placeholders: Vec<PastePlaceholder>,
    pub(super) warnings: Vec<String>,
}

struct Builder<'a> {
    source: &'a Font,
    target: &'a Font,
    params: &'a PasteSpecialParams,
    source_master: MasterId,
    target_master: MasterId,
    names: BTreeMap<GlyphName, GlyphName>,
    staged: BTreeMap<GlyphName, Glyph>,
    added_bases: Vec<String>,
    placeholders: Vec<PastePlaceholder>,
    warnings: Vec<String>,
    import_depth: usize,
    masters: BTreeMap<MasterId, Master>,
}

pub(super) fn build(
    source: &Font,
    names: &[GlyphName],
    doc: &Document,
    params: &PasteSpecialParams,
) -> Result<Plan, ApiError> {
    let source_master = master(source, params.source_master.as_ref())?;
    let target_master = master(&doc.font, params.target_master.as_ref())?;
    let mut builder = Builder {
        source,
        target: &doc.font,
        params,
        source_master,
        target_master,
        names: BTreeMap::new(),
        staged: BTreeMap::new(),
        added_bases: Vec::new(),
        placeholders: Vec::new(),
        warnings: Vec::new(),
        import_depth: 0,
        masters: BTreeMap::new(),
    };
    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    for (source, mapping) in names.iter().zip(&params.mappings) {
        let target = match params.destination {
            PasteDestination::SameName => {
                if mapping.target.is_some() {
                    return Err(ApiError::invalid_params(
                        "same-name mappings must omit target handles",
                    ));
                }
                source.clone()
            }
            _ => doc
                .glyph_name(mapping.target.ok_or_else(|| {
                    ApiError::invalid_params("selected mappings require target handles")
                })?)
                .cloned()
                .ok_or_else(|| ApiError::not_found("target glyph handle does not exist"))?,
        };
        if !seen.insert(target.clone()) {
            return Err(ApiError::invalid_params(
                "duplicate destination glyph mapping",
            ));
        }
        if doc.font.glyph(&target).is_some_and(|glyph| glyph.locked) {
            return Err(ApiError::invalid_params(format!(
                "glyph `{target}` is locked"
            )));
        }
        if params.destination != PasteDestination::Components
            && builder
                .names
                .insert(source.clone(), target.clone())
                .is_some()
        {
            return Err(ApiError::invalid_params("duplicate source glyph mapping"));
        }
        targets.push(target);
    }
    let drawing = matches!(params.content, PasteContent::AllData | PasteContent::Layer);
    // Reserve every output before resolving components so batch-local names can
    // refer to one another independently of the user's mapping order.
    for target in &targets {
        let glyph = doc
            .font
            .glyph(target)
            .cloned()
            .unwrap_or_else(|| empty_glyph(&doc.font, target));
        builder.staged.insert(target.clone(), glyph);
    }
    for (source_name, target_name) in names.iter().zip(&targets) {
        let source_glyph = source
            .glyph(source_name)
            .ok_or_else(|| ApiError::not_found("source glyph does not exist"))?;
        let mut glyph = builder
            .staged
            .get(target_name)
            .cloned()
            .ok_or_else(|| ApiError::internal("missing planned glyph"))?;
        if drawing && params.destination == PasteDestination::Components {
            builder.append_components(source_glyph, &mut glyph)?;
        } else {
            match params.content {
                PasteContent::AllData => {
                    glyph = builder.copy_glyph(source_glyph, target_name)?;
                }
                PasteContent::Layer => {
                    let original = source_glyph
                        .master_layer(&builder.source_master)
                        .ok_or_else(|| ApiError::not_found("source master layer does not exist"))?;
                    let mut layer = builder.copy_layer(source_name, original)?;
                    let target =
                        glyph
                            .master_layer_mut(&builder.target_master)
                            .ok_or_else(|| {
                                ApiError::not_found("destination master layer does not exist")
                            })?;
                    layer.id = target.id.clone();
                    layer.kind = target.kind.clone();
                    *target = layer;
                }
                PasteContent::MetricKeys => {
                    glyph.metrics_keys = source_glyph.metrics_keys.clone();
                    for source_layer in &source_glyph.layers {
                        let mut kind = source_layer.kind.clone();
                        builder.map_kind(&mut kind)?;
                        if let Some(layer) =
                            glyph.layers.iter_mut().find(|layer| layer.kind == kind)
                        {
                            layer.metrics_keys = source_layer.metrics_keys.clone();
                        }
                    }
                }
                PasteContent::KerningGroups => {}
            }
        }
        builder.staged.insert(target_name.clone(), glyph);
    }
    let mut commands = builder
        .masters
        .values()
        .cloned()
        .map(|master| Command::SetMaster { master })
        .collect::<Vec<_>>();
    let mut complete = BTreeSet::new();
    for name in builder.staged.keys() {
        order(
            name,
            &builder,
            &mut complete,
            &mut BTreeSet::new(),
            &mut commands,
        )?;
    }
    if matches!(
        params.content,
        PasteContent::AllData | PasteContent::KerningGroups
    ) && (params.destination != PasteDestination::Components
        || params.content == PasteContent::KerningGroups)
    {
        // Copying a reference retains target metadata. Kerning-only content is
        // still useful with any destination choice and explicitly copies groups.
        group_commands(source, &doc.font, names, &targets, &mut commands)?;
    }
    if source.info.units_per_em != doc.font.info.units_per_em && drawing {
        builder.warnings.push(
            "Source and destination units per em differ; coordinates are copied without scaling."
                .into(),
        );
    }
    if params.content == PasteContent::AllData && params.source_doc != params.doc {
        builder.warnings.push("All data transfers glyph records; font-wide layout, bitmap strikes, palettes and global hint programs are separate resources.".into());
    }
    Ok(Plan {
        commands,
        pasted: targets.iter().map(glyph_name).collect(),
        added_bases: builder.added_bases,
        placeholders: builder.placeholders,
        warnings: builder.warnings,
    })
}

impl Builder<'_> {
    fn mapped_master(&self, id: &MasterId) -> Result<MasterId, ApiError> {
        if id == &self.source_master {
            return Ok(self.target_master.clone());
        }
        if id != &self.target_master && self.target.master(id).is_some() {
            return Ok(id.clone());
        }
        let source = self
            .source
            .master(id)
            .ok_or_else(|| ApiError::not_found("source layer master does not exist"))?;
        let matching = self
            .target
            .masters
            .iter()
            .filter(|master| master.name == source.name)
            .collect::<Vec<_>>();
        if matching.len() == 1 && matching[0].id != self.target_master {
            return Ok(matching[0].id.clone());
        }
        Err(ApiError::invalid_params(format!(
            "no unique destination master for `{}`; use active-layer paste",
            source.name
        )))
    }

    fn copy_glyph(&mut self, source: &Glyph, name: &GlyphName) -> Result<Glyph, ApiError> {
        let mut glyph = source.clone();
        glyph.name = name.clone();
        glyph.kern_group_side1 = None;
        glyph.kern_group_side2 = None;
        glyph.layers = source
            .layers
            .iter()
            .map(|layer| self.copy_layer(&source.name, layer))
            .collect::<Result<_, _>>()?;
        if let Some(existing) = self.target.glyph(name) {
            let mut retained = HashSet::new();
            for layer in &mut glyph.layers {
                if let Some(current) = existing
                    .layers
                    .iter()
                    .find(|current| current.kind == layer.kind && !retained.contains(&current.id))
                {
                    layer.id = current.id.clone();
                    retained.insert(current.id.clone());
                }
            }
            // A source with fewer masters must not erase target-only masters.
            for layer in &existing.layers {
                if matches!(layer.kind, LayerKind::Master(_))
                    && !glyph
                        .layers
                        .iter()
                        .any(|candidate| candidate.kind == layer.kind)
                {
                    glyph.layers.push(layer.clone());
                }
            }
        }
        Ok(glyph)
    }

    fn copy_layer(&mut self, source_name: &GlyphName, source: &Layer) -> Result<Layer, ApiError> {
        let mut layer = source.clone();
        layer.id = LayerId::new();
        self.map_kind(&mut layer.kind)?;
        self.check_global_resources(source, &layer)?;
        self.map_private_dict(source, &mut layer)?;
        // Variable alternate coordinates only have meaning in the same axes.
        if matches!(
            layer.kind,
            LayerKind::Brace { .. } | LayerKind::Bracket { .. }
        ) && self.source.axes != self.target.axes
        {
            return Err(ApiError::invalid_params(
                "variable alternate layers require matching destination axes",
            ));
        }
        let master = layer_master(&source.kind)
            .unwrap_or(&self.source_master)
            .clone();
        if self.params.missing_bases == MissingBasePolicy::Decompose
            && !source.constructions.is_empty()
        {
            let path = fc_geom::flatten_layer_by_id(self.source, source_name, &source.id)
                .map_err(|error| ApiError::geometry(error.to_string()))?;
            layer.contours = fc_model::Contour::from_bezpath(&path);
            layer.components.clear();
            layer.constructions.clear();
            layer.effects.clear();
            layer.node_links.clear();
            layer.linked_guides.clear();
            layer.hints = Default::default();
            layer.tt_instructions = None;
            layer.tt_binding = None;
        } else {
            self.resolve_components(&mut layer.components, &mut layer.contours, &master)?;
            for construction in &mut layer.constructions {
                let original = construction.source.clone();
                construction.source = self
                    .names
                    .get(&original)
                    .cloned()
                    .unwrap_or_else(|| original.clone());
                self.ensure_base(&original, &construction.source)?;
            }
        }
        for color in layer.color_layers.iter_mut().flatten() {
            self.resolve_components(&mut color.components, &mut color.contours, &master)?;
        }
        Ok(layer)
    }

    fn check_global_resources(&self, source: &Layer, target: &Layer) -> Result<(), ApiError> {
        if self.params.source_doc == self.params.doc {
            return Ok(());
        }
        if source
            .tt_instructions
            .as_ref()
            .is_some_and(|program| !program.is_empty())
        {
            let source_master = self
                .source
                .master(&master(self.source, layer_master(&source.kind))?)
                .ok_or_else(|| ApiError::not_found("source hint master does not exist"))?;
            let target_master = self
                .target
                .master(&master(self.target, layer_master(&target.kind))?)
                .ok_or_else(|| ApiError::not_found("destination hint master does not exist"))?;
            if source_master.tt != target_master.tt
                || self.source.info.units_per_em != self.target.info.units_per_em
            {
                return Err(ApiError::invalid_params("TrueType instructions require matching destination global programs and units per em"));
            }
        }
        if source
            .color_layers
            .as_ref()
            .is_some_and(|layers| !layers.is_empty())
            && self.source.color.as_ref().map(|color| &color.palettes)
                != self.target.color.as_ref().map(|color| &color.palettes)
        {
            return Err(ApiError::invalid_params(
                "color layers require matching destination palettes",
            ));
        }
        Ok(())
    }

    fn map_kind(&self, kind: &mut LayerKind) -> Result<(), ApiError> {
        match kind {
            LayerKind::Master(master)
            | LayerKind::Background(master)
            | LayerKind::Smart { master, .. }
            | LayerKind::Brace { master, .. }
            | LayerKind::Bracket { master, .. } => *master = self.mapped_master(master)?,
            LayerKind::Backup { of, .. } => *of = self.mapped_master(of)?,
            LayerKind::Custom(_) => {}
        }
        Ok(())
    }

    fn map_private_dict(&mut self, source: &Layer, target: &mut Layer) -> Result<(), ApiError> {
        // An unhinted FD0 layer has no glyph-local dependency on the source's
        // private values; explicit assignments and PS hints do depend on them.
        if source.cff_private_dict_index.is_none() && source.hints.is_empty() {
            return Ok(());
        }
        let source_master = master(self.source, layer_master(&source.kind))?;
        let target_master = master(self.target, layer_master(&target.kind))?;
        let source_master = self
            .source
            .master(&source_master)
            .ok_or_else(|| ApiError::not_found("source CFF master does not exist"))?;
        let current = self
            .target
            .master(&target_master)
            .ok_or_else(|| ApiError::not_found("destination CFF master does not exist"))?;
        let target_master = self.masters.get(&target_master).unwrap_or(current);
        let index = usize::from(source.cff_private_dict_index.unwrap_or(0));
        let private = if index == 0 {
            Some(&source_master.private_dict)
        } else {
            source_master.cff_private_dicts.get(index - 1)
        }
        .ok_or_else(|| {
            ApiError::invalid_params("source layer has an invalid CFF private dictionary index")
        })?;
        private
            .validate()
            .map_err(|error| ApiError::invalid_params(error.to_string()))?;
        let mapped = match std::iter::once(&target_master.private_dict)
            .chain(&target_master.cff_private_dicts)
            .position(|candidate| candidate == private)
        {
            Some(index) => index,
            None => {
                if self.target.cid.is_none() {
                    return Err(ApiError::invalid_params("destination has no matching CFF private dictionary; a CID destination is required to import additional dictionaries"));
                }
                if target_master.cff_private_dicts.len() >= 255 {
                    return Err(ApiError::invalid_params(
                        "destination already has 256 CFF private dictionaries",
                    ));
                }
                let mut master = target_master.clone();
                master.cff_private_dicts.push(private.clone());
                let index = master.cff_private_dicts.len();
                self.masters.insert(master.id.clone(), master);
                self.warnings
                    .push(format!("Imported CFF private dictionary as FD {index}."));
                index
            }
        };
        target.cff_private_dict_index = if mapped == 0 {
            None
        } else {
            Some(u8::try_from(mapped).map_err(|_| {
                ApiError::invalid_params("destination CFF private dictionary index exceeds 255")
            })?)
        };
        Ok(())
    }

    fn resolve_components(
        &mut self,
        components: &mut Vec<Component>,
        contours: &mut Vec<fc_model::Contour>,
        master: &MasterId,
    ) -> Result<(), ApiError> {
        if self.params.missing_bases == MissingBasePolicy::Decompose {
            for component in components.iter() {
                let path = fc_geom::flatten_smart_component(
                    self.source,
                    &component.base,
                    master,
                    component.smart.as_ref().unwrap_or(&Default::default()),
                )
                .map_err(|error| ApiError::geometry(error.to_string()))?;
                let path = component.transform * path;
                if !path.is_finite() {
                    return Err(ApiError::invalid_params(
                        "component transform is not finite",
                    ));
                }
                contours.extend(fc_model::Contour::from_bezpath(&path));
            }
            components.clear();
        } else {
            for component in components.iter_mut() {
                let original = component.base.clone();
                component.base = self
                    .names
                    .get(&original)
                    .cloned()
                    .unwrap_or_else(|| original.clone());
                self.ensure_base(&original, &component.base.clone())?;
            }
        }
        Ok(())
    }

    fn ensure_base(
        &mut self,
        source_name: &GlyphName,
        target_name: &GlyphName,
    ) -> Result<(), ApiError> {
        if self.staged.contains_key(target_name) || self.target.glyph(target_name).is_some() {
            return Ok(());
        }
        if self.staged.len() >= 4096 {
            return Err(ApiError::invalid_params(
                "paste dependency limit of 4096 glyphs exceeded",
            ));
        }
        match self.params.missing_bases {
            MissingBasePolicy::Reject | MissingBasePolicy::Decompose => {
                return Err(ApiError::invalid_params(format!(
                    "missing component base `{target_name}`"
                )))
            }
            MissingBasePolicy::Placeholder => {
                let mut glyph = empty_glyph(self.target, target_name);
                glyph.export = false;
                glyph.tags.insert(PLACEHOLDER_TAG.into());
                glyph.note = format!("Missing component base `{source_name}`. Relink references or replace this placeholder with real geometry.");
                self.staged.insert(target_name.clone(), glyph);
                self.placeholders.push(PastePlaceholder {
                    name: target_name.to_string(),
                    source_base: source_name.to_string(),
                    tag: PLACEHOLDER_TAG.to_owned(),
                });
                self.warnings.push(format!(
                    "Created empty nonexporting placeholder `{target_name}`, tagged `{PLACEHOLDER_TAG}`."
                ));
            }
            MissingBasePolicy::Copy => {
                if self.import_depth >= 64 {
                    return Err(ApiError::invalid_params(
                        "component dependency depth exceeds 64",
                    ));
                }
                let source = self.source.glyph(source_name).ok_or_else(|| {
                    ApiError::not_found(format!(
                        "component base `{source_name}` is absent from the source"
                    ))
                })?;
                // Reservation bounds recursion and lets final graph validation
                // identify a source cycle without recursively overflowing.
                self.staged
                    .insert(target_name.clone(), empty_glyph(self.target, target_name));
                self.import_depth += 1;
                let copied = self.copy_glyph(source, target_name);
                self.import_depth -= 1;
                let glyph = copied?;
                self.staged.insert(target_name.clone(), glyph);
            }
        }
        self.added_bases.push(target_name.to_string());
        Ok(())
    }

    fn append_components(&mut self, source: &Glyph, target: &mut Glyph) -> Result<(), ApiError> {
        if self.params.missing_bases == MissingBasePolicy::Decompose {
            return Err(ApiError::invalid_params(
                "as-components paste requires reject, copy or placeholder base policy",
            ));
        }
        let mut base = source.name.clone();
        if base == target.name && self.params.source_doc != self.params.doc {
            if self.params.missing_bases != MissingBasePolicy::Copy {
                return Err(ApiError::invalid_params(
                    "a same-name component needs the copy policy to create a distinct base",
                ));
            }
            base = (1..=4096)
                .map(|index| GlyphName::from(format!("{}.{index:03}", source.name)))
                .find(|name| self.target.glyph(name).is_none() && !self.staged.contains_key(name))
                .ok_or_else(|| ApiError::invalid_params("no available component copy name"))?;
        }
        self.ensure_base(&source.name, &base)?;
        let masters = if self.params.content == PasteContent::AllData {
            self.source
                .masters
                .iter()
                .map(|master| self.mapped_master(&master.id))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![self.target_master.clone()]
        };
        for master in masters {
            let layer = target
                .master_layer_mut(&master)
                .ok_or_else(|| ApiError::not_found("destination master layer does not exist"))?;
            layer.components.push(Component {
                base: base.clone(),
                ..Default::default()
            });
        }
        Ok(())
    }
}

fn master(font: &Font, requested: Option<&MasterId>) -> Result<MasterId, ApiError> {
    let id = requested.unwrap_or(&font.default_master().id);
    font.master(id)
        .map(|master| master.id.clone())
        .ok_or_else(|| ApiError::not_found("selected master does not exist"))
}

fn empty_glyph(font: &Font, name: &GlyphName) -> Glyph {
    let mut glyph = Glyph::new(name.clone());
    glyph.layers = font
        .masters
        .iter()
        .map(|master| Layer::new_master(master.id.clone()))
        .collect();
    glyph
}

fn order(
    name: &GlyphName,
    builder: &Builder<'_>,
    complete: &mut BTreeSet<GlyphName>,
    visiting: &mut BTreeSet<GlyphName>,
    commands: &mut Vec<Command>,
) -> Result<(), ApiError> {
    if complete.contains(name) {
        return Ok(());
    }
    if visiting.len() >= 64 || !visiting.insert(name.clone()) {
        return Err(ApiError::invalid_params(format!(
            "component cycle or excessive depth at `{name}`"
        )));
    }
    let glyph = builder
        .staged
        .get(name)
        .ok_or_else(|| ApiError::internal("missing planned glyph"))?;
    for base in glyph.layers.iter().flat_map(|layer| {
        layer
            .components
            .iter()
            .map(|component| &component.base)
            .chain(
                layer
                    .color_layers
                    .iter()
                    .flatten()
                    .flat_map(|color| &color.components)
                    .map(|component| &component.base),
            )
            .chain(
                layer
                    .constructions
                    .iter()
                    .map(|construction| &construction.source),
            )
    }) {
        if builder.staged.contains_key(base) {
            order(base, builder, complete, visiting, commands)?;
        } else if builder.target.glyph(base).is_none() {
            return Err(ApiError::invalid_params(format!(
                "missing component base `{base}`"
            )));
        }
    }
    let command = if builder.target.glyph(name).is_some() {
        Command::ReplaceGlyph {
            glyph: glyph.clone(),
        }
    } else {
        Command::AddGlyph {
            glyph: glyph.clone(),
        }
    };
    commands.push(command);
    visiting.remove(name);
    complete.insert(name.clone());
    Ok(())
}

fn group_commands(
    source: &Font,
    target: &Font,
    sources: &[GlyphName],
    targets: &[GlyphName],
    commands: &mut Vec<Command>,
) -> Result<(), ApiError> {
    for side in [KerningGroupSide::Side1, KerningGroupSide::Side2] {
        let mut created = BTreeSet::new();
        for (source_name, target_name) in sources.iter().zip(targets) {
            let group = group_membership(source, source_name, side)?.map(str::to_owned);
            let current = if target.glyph(target_name).is_some() {
                group_membership(target, target_name, side)?
            } else {
                None
            };
            if current == group.as_deref() {
                continue;
            }
            if let Some(group) = &group {
                if !side.groups(target).contains_key(group) && created.insert(group.clone()) {
                    commands.push(Command::CreateKerningGroup {
                        side,
                        name: group.clone(),
                        members: Vec::new(),
                    });
                }
            }
            commands.push(Command::AssignKerningGroup {
                side,
                glyph: target_name.clone(),
                group,
            });
        }
    }
    Ok(())
}
