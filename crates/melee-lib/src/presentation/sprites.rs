//! Read-only sprite capture. Pixel assets are immutable; the live buffer is
//! provisioned from the simulation's prepared particle capacities.
use super::{error, PresentationError, Texture};
use crate::Match;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug)]
pub enum SpriteShape {
    Texture,
    Shield { port: crate::Port },
}
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub previous_position: [f32; 3],
    pub trail_alpha: f32,
    pub alpha_compare: [u8; 2],
    pub alpha_mode: u8,
    pub position: [f32; 3],
    pub half_size: [f32; 2],
    pub rotation: f32,
    pub color: [u8; 4],
    pub environment: [u8; 4],
    pub texture: usize,
    pub flags: u32,
    pub shape: SpriteShape,
}
pub(super) struct Sprites {
    pub textures: Vec<Arc<Texture>>,
    pub live: Vec<Sprite>,
    indexed: BTreeSet<(u8, u16)>,
    lookup: BTreeMap<(u8, u16, u16, u16), usize>,
}
impl Sprites {
    pub fn new(game: &Match) -> Result<Self, PresentationError> {
        let assets = &game.assets.inner;
        let mut result = Self {
            textures: vec![Arc::new(Texture {
                mipmaps: Vec::new(),
                lod_range: [0.0; 2],
                width: 1,
                height: 1,
                rgba: vec![255; 4],
            })],
            live: Vec::with_capacity(
                game.engine
                    .state()
                    .particles
                    .particles
                    .iter()
                    .map(Vec::capacity)
                    .sum::<usize>()
                    + 2,
            ),
            lookup: BTreeMap::new(),
            indexed: BTreeSet::new(),
        };
        let stage = assets
            .stage
            .public("map_texg")
            .ok_or_else(|| error("stage particle texture bank missing"))?;
        result.add_bank(30, &assets.stage, stage)?;
        let characters = melee_ef::CHARACTER_EFFECT_FILES
            .iter()
            .zip(&assets.visual_effect_archives[1..])
            .map(|(file, archive)| (file.bank, archive, file.table));
        for (bank, archive, symbol) in
            std::iter::once((0, &assets.visual_effect_archives[0], "effCommonDataTable"))
                .chain(characters)
        {
            let table = archive
                .public(symbol)
                .ok_or_else(|| error("effect table missing"))?;
            let offset = archive
                .link(table + 4)
                .map_err(error)?
                .ok_or_else(|| error("effect texture bank missing"))?;
            result.add_bank(bank, archive, offset)?;
        }
        Ok(result)
    }
    fn add_bank(
        &mut self,
        bank: u8,
        archive: &hsd_archive::Archive,
        offset: u32,
    ) -> Result<(), PresentationError> {
        for image in hsd_archive::visual::read_particle_textures(archive, offset).map_err(error)? {
            if image.indexed {
                self.indexed.insert((bank, image.group));
            }
            let index = self.textures.len();
            self.textures.push(image.texture);
            self.lookup
                .insert((bank, image.group, image.image, image.palette), index);
        }
        Ok(())
    }
    pub fn capture(&mut self, game: &Match) -> Result<(), PresentationError> {
        self.live.clear();
        let mut identity = hsd_types::Mtx::default();
        hsd_anim::mtx::mtx_identity(&mut identity);
        for p in game.engine.state().particles.particles.iter().flatten() {
            if p.life == 0 || p.size <= 0.0 {
                continue;
            }
            let texture = if p.kind & hsd_particle::particle::TEXTURED != 0 {
                let palette = if !self.indexed.contains(&(p.bank, u16::from(p.texture_group))) {
                    0
                } else if p.palette != 255 {
                    u16::from(p.palette)
                } else if p.kind & (1 << 4) == 0 {
                    u16::from(p.pose)
                } else {
                    0
                };
                let key = (
                    p.bank,
                    u16::from(p.texture_group),
                    u16::from(p.pose),
                    palette,
                );
                let Some(&index) = self.lookup.get(&key) else {
                    return Err(error(format!("missing particle image {key:?}")));
                };
                index
            } else {
                0
            };
            let mut position = hsd_types::Vec3::from(p.position);
            let mut previous_position =
                hsd_types::Vec3::from(if p.kind & ((1 << 20) | (1 << 21)) != 0 {
                    let generator = p.generator_id.and_then(|id| {
                        game.engine
                            .state()
                            .particles
                            .generators
                            .iter()
                            .find(|g| g.id == id)
                    });
                    hsd_particle::display::previous_position(p, generator).map_err(error)?
                } else {
                    p.position
                });
            let mut half_size = [p.size; 2];
            if let Some(transform) = &p.application_transform {
                let mut scratch = (**transform).clone();
                // Force evaluation in private scratch; simulation cache/camera
                // stamps cannot establish validity for this independent view.
                scratch.frame_number = 1;
                if scratch.camera_facing != 0 {
                    scratch.rotation = hsd_types::Vec3::ZERO;
                    scratch.scale = hsd_types::Vec3::new(1.0, 1.0, 1.0);
                    scratch.status = 0;
                    scratch.camera_facing = 0;
                }
                scratch.prepare_display(&identity, 0).map_err(error)?;
                let previous = previous_position;
                hsd_anim::mtx::mtx_mult_vec(
                    &scratch.model_matrix,
                    &previous,
                    &mut previous_position,
                );
                let source = position;
                hsd_anim::mtx::mtx_mult_vec(&scratch.model_matrix, &source, &mut position);
                half_size = [
                    p.size * scratch.axis_scale[0],
                    p.size * scratch.axis_scale[1],
                ];
            }
            self.push(Sprite {
                previous_position: [
                    previous_position.x,
                    previous_position.y,
                    previous_position.z,
                ],
                trail_alpha: p.trail,
                alpha_compare: p.alpha_compare.display_values(),
                alpha_mode: p.alpha_compare_mode,
                position: [position.x, position.y, position.z],
                half_size,
                rotation: p.rotation,
                color: p.primary.display_color(),
                environment: if p.kind & (1 << 7) != 0 {
                    p.environment.display_color()
                } else {
                    [0; 4]
                },
                texture,
                flags: p.kind,
                shape: SpriteShape::Texture,
            })?;
        }
        Ok(())
    }
    pub fn push(&mut self, sprite: Sprite) -> Result<(), PresentationError> {
        if self.live.len() == self.live.capacity() {
            return Err(error("presentation sprite capacity exceeded"));
        }
        self.live.push(sprite);
        Ok(())
    }
}
