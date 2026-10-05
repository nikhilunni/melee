//! Snapshot live HSD material values without running a second animation clock.
use super::*;
use hsd_archive::visual::TextureDecoder;

impl Presentation {
    pub(super) fn prepare_materials(&mut self, game: &Match) -> Result<(), PresentationError> {
        let originals: Vec<_> = self
            .meshes
            .iter()
            .map(|m| Arc::clone(&m.material))
            .collect();
        let mut prepared = BTreeMap::<usize, Arc<Material>>::new();
        for ((mesh, part), original) in self.meshes.iter_mut().zip(&self.parts).zip(&originals) {
            let key = Arc::as_ptr(original) as usize;
            if let Some(material) = prepared.get(&key) {
                mesh.material = Arc::clone(material);
                self.materials.push((**material).clone());
                continue;
            }
            let mut material = (*mesh.material).clone();
            let model = &mut self.models[part.model];
            let live = live_material(&self.fighter_parts, &model.source, part, game);
            for (index, layer) in material.textures.iter().enumerate() {
                let mut bank = vec![Arc::clone(&layer.image)];
                if let Some(live) = live {
                    let descriptor = &live.textures[index].descriptor;
                    let mut images = vec![descriptor.image];
                    let mut palettes = vec![descriptor.palette];
                    let archive = match &model.source {
                        ModelSource::Stage {
                            key,
                            form,
                            template,
                            ..
                        } => {
                            for program in super::stage::animation(game, *key, template)
                                .material_programs(part.owner, part.display)
                            {
                                if let Some(tables) = program.texture_tables(descriptor.id) {
                                    images.extend(tables.images.iter().flatten().copied());
                                    palettes.extend(
                                        tables.palettes.iter().filter(|p| p.is_some()).copied(),
                                    );
                                }
                            }
                            Some(match form {
                                Some(index) => &self.assets.stage_forms[*index].archive,
                                None => &self.assets.stage,
                            })
                        }
                        ModelSource::Effect(id) => {
                            let texture = &live.textures[index];
                            images.extend(texture.image_variants().iter().flatten().copied());
                            palettes.extend(
                                texture
                                    .palette_variants()
                                    .iter()
                                    .filter(|p| p.is_some())
                                    .copied(),
                            );
                            let definition = self
                                .assets
                                .effect_resources
                                .visual_models()
                                .find(|m| m.descriptor == *id)
                                .unwrap();
                            Some(effects::archive(&self.assets, definition.bank))
                        }
                        ModelSource::Fighter(slot) => {
                            // ftAnim_80070200: costume texture animations.
                            let texture = &live.textures[index];
                            images.extend(texture.image_variants().iter().flatten().copied());
                            palettes.extend(
                                texture
                                    .palette_variants()
                                    .iter()
                                    .filter(|p| p.is_some())
                                    .copied(),
                            );
                            let costume = game.engine.state().fighters[*slot].0.player.costume;
                            Some(self.assets.characters[*slot].costume(costume))
                        }
                        ModelSource::Item(pose) => {
                            for texture in pose.texture_states(part.owner, part.display, index) {
                                images.push(texture.descriptor.image);
                                palettes.push(texture.descriptor.palette);
                                images.extend(texture.image_variants().iter().flatten().copied());
                                palettes.extend(
                                    texture
                                        .palette_variants()
                                        .iter()
                                        .filter(|p| p.is_some())
                                        .copied(),
                                );
                            }
                            Some(
                                self.assets
                                    .items
                                    .visual_models()
                                    .find(|(kind, _, _)| *kind == pose.kind)
                                    .unwrap()
                                    .1,
                            )
                        }
                        ModelSource::Article(article) | ModelSource::StageItem(article) => {
                            for texture in article.texture_states(part.owner, part.display, index) {
                                images.push(texture.descriptor.image);
                                palettes.push(texture.descriptor.palette);
                                images.extend(texture.image_variants().iter().flatten().copied());
                                palettes.extend(
                                    texture
                                        .palette_variants()
                                        .iter()
                                        .filter(|p| p.is_some())
                                        .copied(),
                                );
                            }
                            Some(if matches!(model.source, ModelSource::StageItem(_)) {
                                &self.assets.stage
                            } else {
                                self.assets
                                    .items
                                    .visual_models()
                                    .find(|(kind, _, _)| *kind == article.kind)
                                    .unwrap()
                                    .1
                            })
                        }
                    };
                    let Some(archive) = archive else {
                        material.texture_banks.push(bank.into());
                        continue;
                    };
                    images.sort_unstable();
                    images.dedup();
                    palettes.sort_unstable();
                    palettes.dedup();
                    let mut decoder =
                        TextureDecoder::with_images(archive, std::mem::take(&mut model.images));
                    let authored = (descriptor.image, descriptor.palette);
                    for image in images {
                        for &palette in &palettes {
                            // Texture tables pair each image with its own palette;
                            // a cross pairing may index past a smaller palette and
                            // is never selected (Pokemon Stadium's screen).
                            let decoded = match decoder.image(image, palette) {
                                Ok(decoded) => decoded,
                                Err(_) if (image, palette) != authored => continue,
                                Err(e) => return Err(error(e)),
                            };
                            if !bank.iter().any(|old| Arc::ptr_eq(old, &decoded)) {
                                bank.push(decoded);
                            }
                        }
                    }
                    model.images = decoder.into_images();
                }
                material.texture_banks.push(bank.into());
            }
            let material = Arc::new(material);
            self.materials.push((*material).clone());
            mesh.material = Arc::clone(&material);
            prepared.insert(key, material);
        }
        Ok(())
    }
    pub(super) fn capture_materials(&mut self, game: &Match) -> Result<(), PresentationError> {
        for (material, part) in self.materials.iter_mut().zip(&self.parts) {
            let model = &self.models[part.model];
            material.overlay = match &model.source {
                ModelSource::Stage { key, .. }
                    if super::stage::is_screen_feed(game, *key, part.owner) =>
                {
                    super::stage::SCREEN_OFF
                }
                ModelSource::Stage { key, template, .. } => {
                    let overlay = &super::stage::animation(game, *key, template).overlay;
                    if overlay.enabled {
                        overlay.color.map(|v| f32::from(v) / 255.0)
                    } else {
                        [0.0; 4]
                    }
                }
                ModelSource::Fighter(slot) => {
                    let joint = model.source.tree(game).get(part.owner).id;
                    self.fighter_parts[*slot]
                        .overlay(joint, part.display)
                        .unwrap_or([0.0; 4])
                }
                _ => [0.0; 4],
            };
            let Some(live) = live_material(&self.fighter_parts, &model.source, part, game) else {
                continue;
            };
            capture_material(material, live, &model.images)?;
            if let ModelSource::Item(pose) = &model.source {
                // it_80278574: the owner's colour on every MObj.
                let owner = pose.owner.filter(|_| items::takes_owner_color(pose.kind));
                let color = owner.and_then(|(player, secondary)| {
                    let slot = game.engine.state().fighters.iter().position(|f| {
                        f.0.player.id == player && f.0.player.secondary == secondary
                    })?;
                    self.fighter_parts.get(slot)?.body_color()
                });
                if let Some([r, g, b, _]) = color {
                    let diffuse = [r, g, b].map(|v| f32::from(v) / 255.0);
                    material.diffuse[..3].copy_from_slice(&diffuse);
                }
            }
        }
        Ok(())
    }
}

/// The display's current MObj: a fighter's comes from its costume material
/// state (ftAnim_80070308), which the simulation's skeleton does not carry.
fn live_material<'a>(
    fighter_parts: &'a [fighters::FighterParts],
    source: &'a ModelSource,
    part: &Part,
    game: &'a Match,
) -> Option<&'a hsd_anim::mobj::MObj> {
    let tree = source.tree(game);
    if let ModelSource::Fighter(slot) = *source {
        if let Some(live) = fighter_parts
            .get(slot)
            .and_then(|parts| parts.material(tree.get(part.owner).id, part.display))
        {
            return Some(live);
        }
    }
    tree.dobj(part.owner)
        .and_then(|ds| ds.get(part.display))
        .and_then(|d| d.mobj.as_ref())
}

pub(super) fn capture_material(
    material: &mut Material,
    live: &hsd_anim::mobj::MObj,
    images: &hsd_archive::visual::DecodedImages,
) -> Result<(), PresentationError> {
    let color = |c: hsd_anim::mobj::GxColor| [c.r, c.g, c.b].map(|v| f32::from(v) / 255.0);
    material.ambient = color(live.mat.ambient);
    material.specular = color(live.mat.specular);
    material.shininess = live.mat.shininess;
    let diffuse = color(live.mat.diffuse);
    material.diffuse = [diffuse[0], diffuse[1], diffuse[2], live.mat.alpha];
    if let Some(pe) = live.pe {
        material.pixel.alpha_reference = [pe.ref0, pe.ref1];
    }
    for (layer, texture) in material.textures.iter_mut().zip(&live.textures) {
        let d = &texture.descriptor;
        layer.lod = d.lod;
        layer.lod.bias = texture.lod_bias;
        layer.scale = d.scale;
        layer.rotation = d.rotation;
        layer.translation = d.translation;
        layer.blending = d.blending;
        layer.combiner = d.combiner;
        let image = images
            .get(&(d.image, d.palette))
            .ok_or_else(|| error("animated image was not prepared"))?;
        layer.image = Arc::clone(image);
    }
    Ok(())
}
