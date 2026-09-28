//! Effect meshes upload once; each active model gets a bounded pose/material slot.
use super::*;
const CAPACITY: usize = melee_ef::VISUAL_CAPACITY;
#[derive(Clone, Copy)]
pub struct EffectDraw {
    pub mesh: usize,
    pub slot: usize,
}
#[derive(Default)]
pub(super) struct Effects {
    draws: Vec<EffectDraw>,
    poses: Vec<Vec<[[f32; 4]; 4]>>,
    materials: Vec<Vec<Material>>,
    active: usize,
}
pub(super) fn archive(assets: &crate::assets::Assets, bank: u8) -> &Archive {
    let index = if bank == 0 {
        0
    } else {
        1 + melee_ef::CHARACTER_EFFECT_FILES
            .iter()
            .position(|file| file.bank == bank)
            .expect("registered effect bank")
    };
    &assets.visual_effect_archives[index]
}
impl Effects {
    pub fn new(scene: &Presentation) -> Self {
        let materials: Vec<_> = scene
            .parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                if matches!(scene.models[part.model].source, ModelSource::Effect(_)) {
                    (0..CAPACITY)
                        .map(|_| scene.materials[index].clone())
                        .collect()
                } else {
                    Vec::new()
                }
            })
            .collect();
        let max_parts = scene
            .models
            .iter()
            .enumerate()
            .filter(|(_, m)| matches!(m.source, ModelSource::Effect(_)))
            .map(|(i, _)| scene.parts.iter().filter(|p| p.model == i).count())
            .max()
            .unwrap_or(0);
        Self {
            draws: Vec::with_capacity(max_parts * CAPACITY),
            poses: (0..CAPACITY)
                .map(|_| vec![[[0.0; 4]; 4]; scene.matrices.len()])
                .collect(),
            materials,
            active: 0,
        }
    }
}
impl Presentation {
    pub fn effect_capacity(&self) -> usize {
        CAPACITY
    }
    pub fn effect_draws(&self) -> &[EffectDraw] {
        &self.effects.draws
    }
    pub fn effect_poses(&self) -> impl Iterator<Item = &[[[f32; 4]; 4]]> {
        self.effects.poses[..self.effects.active]
            .iter()
            .map(Vec::as_slice)
    }
    pub fn effect_material(&self, draw: EffectDraw) -> &Material {
        &self.effects.materials[draw.mesh][draw.slot]
    }
    pub fn is_effect_mesh(&self, mesh: usize) -> bool {
        !self.effects.materials[mesh].is_empty()
    }
    pub(super) fn capture_effects(&mut self, game: &Match) -> Result<(), PresentationError> {
        self.effects.draws.clear();
        self.effects.active = 0;
        for live in game.engine.state().effects.visual_models() {
            let Some(index) = self
                .models
                .iter()
                .position(|m| matches!(m.source, ModelSource::Effect(id) if id == live.descriptor))
            else {
                continue;
            };
            let slot = self.effects.active;
            self.effects.active += 1;
            let model = &mut self.models[index];
            if !model.pose.refresh(live.tree) {
                return Err(error("effect skeleton shape changed"));
            }
            for (mesh, part) in self
                .parts
                .iter()
                .enumerate()
                .filter(|(_, p)| p.model == index)
            {
                let mut ancestor = Some(part.owner);
                let mut hidden = self.meshes[mesh].culling == FaceCulling::Both;
                while let Some(id) = ancestor {
                    hidden |= live.tree.get(id).flags & JOBJ_HIDDEN != 0;
                    ancestor = live.tree.parent(id);
                }
                let display = live
                    .tree
                    .dobj(part.owner)
                    .and_then(|ds| ds.get(part.display));
                hidden |= display.is_some_and(|d| d.flags & 1 != 0);
                if hidden {
                    continue;
                }
                for (offset, binding) in part.bindings.iter().enumerate() {
                    let matrix = match binding {
                        Binding::Rigid(joint) => model.pose.matrix(*joint),
                        Binding::Envelope(weights) => model
                            .pose
                            .envelope_matrix(part.owner, weights)
                            .ok_or_else(|| error("incomplete effect envelope"))?,
                    };
                    self.effects.poses[slot][self.meshes[mesh].matrix_offset as usize + offset] =
                        columns(matrix);
                }
                if let Some(material) = display.and_then(|d| d.mobj.as_ref()) {
                    materials::capture_material(
                        &mut self.effects.materials[mesh][slot],
                        material,
                        &model.images,
                    )?;
                }
                self.effects.draws.push(EffectDraw { mesh, slot });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Buttons, Character, GameAssets, Inputs, MatchConfig, PlayerConfig, Port, Seed, Stage,
    };
    #[test]
    fn model_effects_use_live_poses_and_materials_without_a_render_clock() {
        let files =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
        if !melee_test_support::require_files([files.join("PlCo.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut frequent = Presentation::new(&game).unwrap();
        let mut sparse = Presentation::new(&game).unwrap();
        let mut saw_effect = false;
        let mut saw_multiple = false;
        for tick in 0..600 {
            let mut inputs = Inputs::default();
            if tick >= 220 && tick % 60 < 30 {
                inputs.0[0].buttons = Buttons::B;
            }
            game.step(&inputs).unwrap();
            frequent.capture(&game).unwrap();
            saw_effect |= !frequent.effect_draws().is_empty();
            saw_multiple |= frequent.effects.active > 1;
            if tick % 7 != 0 {
                continue;
            }
            sparse.capture(&game).unwrap();
            assert_eq!(frequent.effect_draws().len(), sparse.effect_draws().len());
            for (a, b) in frequent.effect_draws().iter().zip(sparse.effect_draws()) {
                assert_eq!((a.mesh, a.slot), (b.mesh, b.slot));
                let part = &frequent.parts[a.mesh];
                let start = frequent.meshes[a.mesh].matrix_offset as usize;
                let end = start + part.bindings.len();
                assert_eq!(
                    &frequent.effects.poses[a.slot][start..end],
                    &sparse.effects.poses[b.slot][start..end]
                );
                let (a, b) = (frequent.effect_material(*a), sparse.effect_material(*b));
                assert_eq!(a.diffuse, b.diffuse);
                for (a, b) in a.textures.iter().zip(&b.textures) {
                    assert_eq!(a.image.rgba, b.image.rgba);
                    assert_eq!(a.translation, b.translation);
                    assert_eq!(a.blending, b.blending);
                }
            }
        }
        assert!(saw_effect && saw_multiple);
        game.reset(Seed(42)).unwrap();
        frequent.capture(&game).unwrap();
        let fresh = Presentation::new(&game).unwrap();
        assert_eq!(frequent.effect_draws().len(), fresh.effect_draws().len());
    }
}
