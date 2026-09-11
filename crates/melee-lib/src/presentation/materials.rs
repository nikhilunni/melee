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
            let live = model
                .source
                .tree(game)
                .dobj(part.owner)
                .and_then(|ds| ds.get(part.display))
                .and_then(|d| d.mobj.as_ref());
            for (index, layer) in material.textures.iter().enumerate() {
                let mut bank = vec![Arc::clone(&layer.image)];
                if let (ModelSource::Stage(map), Some(live)) = (&model.source, live) {
                    let descriptor = &live.textures[index].descriptor;
                    let mut images = vec![descriptor.image];
                    let mut palettes = vec![descriptor.palette];
                    for program in game.engine.state().stage_animations[map]
                        .material_programs(part.owner, part.display)
                    {
                        if let Some(tables) = program.texture_tables(descriptor.id) {
                            images.extend(tables.images.iter().flatten().copied());
                            palettes
                                .extend(tables.palettes.iter().filter(|p| p.is_some()).copied());
                        }
                    }
                    images.sort_unstable();
                    images.dedup();
                    palettes.sort_unstable();
                    palettes.dedup();
                    let mut decoder = TextureDecoder::with_images(
                        &self.assets.stage,
                        std::mem::take(&mut model.images),
                    );
                    for image in images {
                        for &palette in &palettes {
                            let decoded = decoder.image(image, palette).map_err(error)?;
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
            let Some(live) = model
                .source
                .tree(game)
                .dobj(part.owner)
                .and_then(|ds| ds.get(part.display))
                .and_then(|d| d.mobj.as_ref())
            else {
                continue;
            };
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
                layer.scale = d.scale;
                layer.rotation = d.rotation;
                layer.translation = d.translation;
                layer.blending = d.blending;
                layer.combiner = d.combiner;
                let image = model
                    .images
                    .get(&(d.image, d.palette))
                    .ok_or_else(|| error("animated image was not prepared"))?;
                layer.image = Arc::clone(image);
            }
        }
        Ok(())
    }
}
