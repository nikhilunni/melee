//! Headless ef subsystem: effect descriptor loading, efAsync request dispatch, attached and
//! positional effect instances, lifetime/animation procs, and DPtcl routing.
//! Async kind 2 transforms a queued bone-local offset before generator dispatch;
//! running dust also uses kinds 5/6 for facing and floor-angle parameters.
//! The warp additionally needs the HSD spline/reference evaluator in `spline`.
//! No spawn schedules or captured matrices are runtime inputs.
mod dust;
mod egg_shell;
pub mod fixture_spawns;
mod pool;
pub mod request;
mod spline;
mod tables;
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_anim::mtx::InverseTrig;
use hsd_anim::{
    aobj::AObjDesc,
    fobj::FObjDesc,
    jobj::{AnimJoint, JObjEvent, JObjId, JObjTree, JointSpec},
    mtx::mtx_mult_vec,
};
use hsd_archive::{desc, Archive};
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::{Mtx, Vec3};
use melee_types::fixed::FixedVec;
use pool::{ModelPool, INSTANCE_CAPACITY};
use request::{EffectOwner, EffectRequest};
use std::collections::BTreeMap;
use tables::*;

// Stable fighter-bone identities occupy a range beyond effect model joints.
const FIRST_FIGHTER_JOINT: usize = 1 << 24;
const FIGHTER_JOINT_STRIDE: usize = 256;
// EF_EffectDesc: lifetime plus four model/animation pointers (ef/types.h).
const EFFECT_DESCRIPTOR_SIZE: u32 = 20;
// Port-only diagnostic budget, drained by the scheduler each tick. Retail has
// no Rust draw log; fail explicitly rather than growing this observation buffer.
const DRAW_CAPACITY: usize = 4096;
// Stage joint identities occupy the low range; effects own monotonic IDs.
pub const FIRST_EFFECT_JOINT: usize = 1 << 16;

pub struct Effects {
    pub events: crate::fixture_spawns::EventSink,
    camera_quakes: FixedVec<(u16, Vec3), { request::REQUEST_CAPACITY }>,
    pub draws: DrawLog,
    /// Ordered direct efAsync/ftColl draws; diagnostics use fixed storage.
    pub direct_draws: FixedVec<u32, 64>,
    instances: FixedVec<Effect, INSTANCE_CAPACITY>,
    models: ModelPool,
    fox_bank: Option<ParticleBank>,
    mars_bank: Option<ParticleBank>,
    next_joint: usize,
    fighter_joints: [bool; 2 * FIGHTER_JOINT_STRIDE],
}
#[derive(Clone)]
struct Effect {
    descriptor: u32,
    bank: u8,
    velocity: Option<Vec3>,
    tree: JObjTree,
    root: JObjId,
    joints: Vec<JObjId>,
    attachment: Option<usize>,
    owner: Option<ModelOwner>,
    lifetime: u16,
    indefinite: bool,
    attachment_bone: Option<usize>,
    scale_attachment: bool,
    callback_rotation: Option<Vec3>,
    joint_base: usize,
    paths: BTreeMap<usize, (JObjId, spline::Spline)>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelOwner {
    Fighter(usize),
    Blaster(usize),
}
/// Retail efSync runs at the caller; efAsync drains at fighter link 9.
#[derive(Clone, Copy)]
pub enum EffectTiming {
    Immediate,
    Deferred,
}
impl Effects {
    /// efAlt_Spawn(0x48E), efLib_Create_Attach_Pos(0xBBD): Fox table row 5.
    /// Loading is initialization-only; all 64 synchronous model slots are warm.
    pub fn load_fox(&mut self, archive: &Archive) -> Result<()> {
        ensure!(self.fox_bank.is_none(), "Fox effects already loaded");
        let table = archive
            .public("effFoxDataTable")
            .context("Fox effect table")?;
        let commands = archive.link(table)?.context("Fox particle commands")? as usize;
        let textures = archive.link(table + 4)?.context("Fox particle textures")? as usize;
        self.fox_bank = Some(ParticleBank::from_bytes(
            &archive.data()[commands..textures],
            &archive.data()[textures..],
        )?);
        for (index, id) in [
            (0, 0xBB8),
            (1, 0xBB9),
            (2, 0xBBA),
            (3, 0xBBB),
            (4, 0xBBC),
            (5, 0xBBD),
        ] {
            self.models.add(Effect::load_table(
                archive,
                "effFoxDataTable",
                index,
                id,
                3,
            )?);
        }
        Ok(())
    }
    /// efSync 4F2/4F3: Marth effect models and bank 16, initialization only.
    pub fn load_mars(&mut self, archive: &Archive) -> Result<()> {
        ensure!(self.mars_bank.is_none(), "Marth effects already loaded");
        let table = archive
            .public("effMarsDataTable")
            .context("Marth effect table")?;
        let commands = archive.link(table)?.context("Marth particle commands")? as usize;
        let textures = archive
            .link(table + 4)?
            .context("Marth particle textures")? as usize;
        self.mars_bank = Some(ParticleBank::from_bytes(
            &archive.data()[commands..textures],
            &archive.data()[textures..],
        )?);
        for (index, id) in [(0, 0x3E80), (1, 0x3E81)] {
            self.models.add(Effect::load_table(
                archive,
                "effMarsDataTable",
                index,
                id,
                16,
            )?);
        }
        Ok(())
    }
    pub fn fox_particle_bank(&self) -> Option<&ParticleBank> {
        self.fox_bank.as_ref()
    }
    pub fn spawn_blaster_muzzle<T: InverseTrig>(
        &mut self,
        owner: usize,
        position: Vec3,
        angle: f32,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(0xBBD, particles);
        effect.owner = Some(ModelOwner::Blaster(owner));
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.tree.set_translate(effect.root, &position);
        effect
            .tree
            .set_rotation_y(effect.root, std::f32::consts::FRAC_PI_2);
        effect.tree.set_rotation_z(effect.root, angle);
        effect.animate_banks::<T>(
            (common_bank, self.fox_bank.as_ref(), self.mars_bank.as_ref()),
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
        )?;
        self.instances.push(effect);
        Ok(())
    }
    /// it_802AEAB4 -> efLib_DestroyAll(item): remove only this owner's muzzle models.
    pub fn destroy_blaster_muzzles(&mut self, owner: usize, particles: &mut ParticleSystem) {
        for effect in self
            .instances
            .iter()
            .filter(|e| e.owner == Some(ModelOwner::Blaster(owner)))
        {
            for &joint in &effect.joints {
                let id = effect.joint_base + joint.0;
                self.events.expire_joint(id);
                particles.expire_joint(id);
            }
        }
        self.recycle_where(|e| e.owner == Some(ModelOwner::Blaster(owner)));
    }
    /// Diagnostic matrices; intentionally allocates outside the tick path.
    pub fn matrices(&self) -> BTreeMap<usize, Mtx> {
        let mut matrices = BTreeMap::new();
        for effect in self.instances.iter() {
            let joints: Vec<_> = effect.tree.depth_first(effect.root).collect();
            for joint in joints {
                matrices.insert(effect.joint_base + joint.0, effect.tree.get(joint).mtx);
            }
        }
        matrices
    }
    /// efAsync_QueueProcessDeferred (efasync.c:1321-1381), drained by
    /// Fighter_8006C80C at s_link 9 (fighter.c:2552-2557).
    #[allow(clippy::too_many_arguments)] // Fighter, effect assets and particle runtime stay in their own layers.
    pub fn flush<T: InverseTrig>(
        &mut self,
        timing: EffectTiming,
        player: usize,
        fighter: &mut impl EffectOwner,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let requests = fighter.effect_queue().drain(timing);
        for queued in requests.iter() {
            let request = queued.request;
            let resolved_matrix = queued.matrix;
            if let EffectRequest::DamageTrail { trajectory } = request {
                let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(None));
                let mut spawn = SpawnRequest::new(0, 0x3E, 0);
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    translation: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                    rotation: Vec3::new(0.0, 0.0, trajectory),
                    status: 1,
                    ..Default::default()
                });
                self.events.spawn(&spawn, false, false);
                spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                continue;
            }
            if let EffectRequest::EggShell { bone, scale } = request {
                let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone)));
                self.spawn_egg_shell::<T>(bank, particles, rng, &matrix, scale)?;
                continue;
            }
            if let EffectRequest::HitSpark {
                position,
                large: true,
                element: melee_types::HitElement::Normal,
                ..
            } = request
            {
                self.spawn_dust_generator::<T>(0x3F3, position, 1.0, bank, particles, rng)?;
                continue;
            }
            if let EffectRequest::NormalSparkExtra {
                position,
                facing,
                variant,
                random_bound,
            } = request
            {
                let site = if variant == 0 { 0x800785CC } else { 0x800785FC };
                // ftColl_80078538+0x94: the draw is an external RNG input to the
                // particle pass and is listed among this tick's effect RNG sites.
                self.events.external_randi(site, random_bound);
                self.direct_draws.push(site);
                let selected = rng.randi(random_bound) == 0;
                if variant == 0 && selected {
                    self.spawn_dust_generator::<T>(0x3EF, position, facing, bank, particles, rng)?;
                }
                continue;
            }
            if matches!(request, EffectRequest::DestroyOwned) {
                for effect in self
                    .instances
                    .iter()
                    .filter(|effect| effect.owner == Some(ModelOwner::Fighter(player)))
                {
                    for joint in effect.tree.depth_first(effect.root) {
                        self.events.expire_joint(effect.joint_base + joint.0);
                        particles.expire_joint(effect.joint_base + joint.0);
                    }
                }
                self.recycle_where(|effect| effect.owner == Some(ModelOwner::Fighter(player)));
                // S3: efLib_DestroyAll also walks the parent fighter skeleton.
                for bone in 0..FIGHTER_JOINT_STRIDE {
                    if self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] {
                        let joint = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                        self.events.expire_joint(joint);
                        particles.expire_joint(joint);
                    }
                }
                continue;
            }
            // S3: efAlt 48B/48C use efLib_Create_Attach, with no scale inheritance.
            if let EffectRequest::OwnedRotation { model, rotation } = request {
                for effect in self.instances.iter_mut().filter(|effect| {
                    effect.owner == Some(ModelOwner::Fighter(player)) && effect.descriptor == model
                }) {
                    effect.callback_rotation = Some(rotation);
                }
                continue;
            }
            if let EffectRequest::SyncAttached {
                id: id @ (0x488..=0x48C | 0x4F2..=0x4F3),
                bone,
            } = request
            {
                let model = match id {
                    0x488..=0x48C => 0xBB8 + u32::from(id - 0x488),
                    0x4F2..=0x4F3 => 0x3E80 + u32::from(id - 0x4F2),
                    _ => unreachable!(),
                };
                let mut effect = self.acquire(model, particles);
                effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
                self.next_joint += effect.tree.len();
                effect.owner = Some(ModelOwner::Fighter(player));
                effect.attachment = Some(player);
                effect.attachment_bone = Some(bone);
                effect.scale_attachment = false;
                if id <= 0x48A || id >= 0x4F2 {
                    // efLib_Create_Attach_Scale: the fighter root supplies uniform scale.
                    let mut scale = fighter.effect_scale();
                    scale.x = scale.y;
                    scale.z = scale.y;
                    effect.tree.set_scale(effect.root, &scale);
                }
                if id >= 0x4F2 {
                    effect.tree.set_rotation_y(
                        effect.root,
                        std::f32::consts::FRAC_PI_2 * fighter.effect_facing(),
                    );
                }
                let matrix = fighter.effect_matrix(Some(bone));
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                );
                effect.animate_banks::<T>(
                    (bank, self.fox_bank.as_ref(), self.mars_bank.as_ref()),
                    particles,
                    rng,
                    &mut self.draws,
                    &mut self.events,
                )?;
                self.instances.push(effect);
                continue;
            }
            // S3: efAlt_Spawn 0x48D -> efLib_CreateGenerator_AppSRT_SetFacingDir.
            if let EffectRequest::SyncAttached {
                id: id @ (0x48D | 0x4F1),
                bone,
            } = request
            {
                let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                let (bank_id, kind) = if id == 0x48D {
                    (3, 0xBC0)
                } else {
                    (16, 0x3E80)
                };
                let mut spawn = SpawnRequest::new(bank_id, kind, 0);
                spawn.joint = Some((joint_id, fighter.effect_matrix(Some(bone))));
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    rotation: Vec3::new(
                        0.0,
                        if fighter.effect_facing() < 0.0 {
                            -std::f32::consts::FRAC_PI_2
                        } else {
                            std::f32::consts::FRAC_PI_2
                        },
                        0.0,
                    ),
                    ..Default::default()
                });
                self.events.spawn(&spawn, false, false);
                let bank = if id == 0x48D {
                    self.fox_bank.as_ref()
                } else {
                    self.mars_bank.as_ref()
                }
                .context("special particle bank")?;
                if let Some(id) = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?
                {
                    let generator = particles.generator_mut(id).unwrap();
                    self.events.flags(joint_id, 0x600, 0x800);
                    generator.flags = (generator.flags & !0x600) | 0x800;
                }
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::Attached { id, bone } = request {
                // efasync.c:282-287: kind 0, hsd_8039EFAC on the live bone.
                let kind = ATTACHED_SPAWNS
                    .iter()
                    .find(|&&(request, _)| request == id)
                    .map(|&(_, particle)| particle)
                    .with_context(|| format!("unsupported attached effect {id:#x}"))?;
                assert!(bone < FIGHTER_JOINT_STRIDE);
                let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                let mut spawn = SpawnRequest::new(0, kind, 0);
                spawn.joint = Some((
                    joint_id,
                    resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone))),
                ));
                self.events.spawn(&spawn, false, false);
                spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::LedgeGrab { position } | EffectRequest::ShieldSpark { position } =
                request
            {
                self.spawn_dust_generator::<T>(
                    0x41C,
                    position,
                    fighter.effect_facing(),
                    bank,
                    particles,
                    rng,
                )?;
                continue;
            }
            if let EffectRequest::Landing {
                id: 0x407, offset, ..
            } = request
            {
                let mut position = Vec3::ZERO;
                mtx_mult_vec(
                    &resolved_matrix.unwrap_or(fighter.effect_matrix(None)),
                    &offset,
                    &mut position,
                );
                self.spawn_dust_generator::<T>(
                    0x407,
                    position,
                    fighter.effect_facing(),
                    bank,
                    particles,
                    rng,
                )?;
                continue;
            }
            if let EffectRequest::Graphics {
                id,
                bone,
                offset,
                facing,
                ..
            } = request
            {
                if !MODEL_SPAWNS
                    .iter()
                    .any(|row| row.request == id && row.source == ModelSource::Graphics)
                {
                    let mut position = Vec3::ZERO;
                    mtx_mult_vec(
                        &resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone))),
                        &offset,
                        &mut position,
                    );
                    if id == 0x514 {
                        // efAsync kind 8 -> Camera_RequestQuake(3), no particle spawn.
                        self.camera_quakes.push((3, position));
                    } else {
                        self.spawn_dust_generator::<T>(id, position, facing, bank, particles, rng)?;
                    }
                    continue;
                }
            }
            let (id, attachment) = match request {
                EffectRequest::Death { .. } => (0x19, None),
                EffectRequest::CaptureFlash { .. } => (0xF, None),
                EffectRequest::HitSpark {
                    element: melee_types::HitElement::Normal,
                    ..
                } => {
                    self.direct_draws.push(0x8006_3990);
                    (if rng.randi(8) == 0 { 9 } else { 10 }, None)
                }
                EffectRequest::HitSpark {
                    element: melee_types::HitElement::Slash,
                    ..
                } => (8, None),
                EffectRequest::Graphics { id, .. }
                | EffectRequest::Shield { id, .. }
                | EffectRequest::EntryWarp { id, .. }
                | EffectRequest::Landing { id, .. } => {
                    let source = match request {
                        EffectRequest::Graphics { .. } => ModelSource::Graphics,
                        EffectRequest::Shield { .. } => ModelSource::Shield,
                        EffectRequest::Landing { .. } => ModelSource::Landing,
                        EffectRequest::EntryWarp { .. } => ModelSource::Entry,
                        _ => unreachable!(),
                    };
                    let row = MODEL_SPAWNS
                        .iter()
                        .chain(std::iter::once(&WARP_SPAWN))
                        .find(|row| row.request == id && row.source == source)
                        .context("unsupported model effect")?;
                    (row.model, row.attached.then_some(player))
                }
                _ => anyhow::bail!("unsupported fighter effect {request:?}"),
            };
            let mut effect = self.acquire(id, particles);
            // Reserve disjoint owned joint identities; stage IDs occupy 0..65536.
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect.attachment = attachment;
            effect.owner = if matches!(request, EffectRequest::HitSpark { .. }) {
                None
            } else {
                Some(ModelOwner::Fighter(player))
            };
            if let EffectRequest::Shield { bone, .. }
            | EffectRequest::Graphics {
                id: 0x423 | 0x424,
                bone,
                ..
            } = request
            {
                effect.attachment_bone = Some(bone);
            }
            let bone = if let EffectRequest::CaptureFlash { bone }
            | EffectRequest::Graphics { bone, .. }
            | EffectRequest::Shield { bone, .. } = request
            {
                Some(bone)
            } else {
                None
            };
            let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(bone));
            let mut position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
            match request {
                EffectRequest::OwnedRotation { .. }
                | EffectRequest::EggShell { .. }
                | EffectRequest::DamageTrail { .. }
                | EffectRequest::NormalSparkExtra { .. }
                | EffectRequest::DestroyOwned
                | EffectRequest::Attached { .. }
                | EffectRequest::SyncAttached { .. }
                | EffectRequest::LedgeGrab { .. }
                | EffectRequest::ShieldSpark { .. } => unreachable!(),
                EffectRequest::Death {
                    position: origin,
                    scale,
                } => {
                    position = origin;
                    effect
                        .tree
                        .set_scale(effect.root, &Vec3::new(scale, scale, scale));
                }
                EffectRequest::HitSpark {
                    position: contact,
                    element,
                    damage,
                    ..
                } => {
                    position = contact;
                    if element == melee_types::HitElement::Normal {
                        // efAsync_Dispatch, retail 80063A10: fmadds.
                        let scale = gekko_math::fma::fmadds(0.04, damage, 0.3).clamp(0.3, 1.5);
                        effect
                            .tree
                            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
                    } else {
                        self.events.external_randf(0x8006_3b70);
                        // efasync.c:34-36: M_TAU is double, multiply then round.
                        effect.tree.set_rotation_z(
                            effect.root,
                            (std::f64::consts::TAU * f64::from(rng.randf())) as f32,
                        );
                    }
                }
                // EF_SCALE_INHERIT is applied by efLib_Update, after creation.
                EffectRequest::CaptureFlash { .. } | EffectRequest::Shield { .. } => {}
                EffectRequest::Graphics {
                    id,
                    offset,
                    facing,
                    floor_angle,
                    ..
                } => {
                    mtx_mult_vec(&matrix, &offset, &mut position);
                    if !matches!(id, 0x3FA | 0x3FB | 0x406 | 0x423 | 0x424) {
                        effect.tree.set_rotation_y(
                            effect.root,
                            if facing < 0.0 {
                                -std::f32::consts::FRAC_PI_2
                            } else {
                                std::f32::consts::FRAC_PI_2
                            },
                        );
                    }
                    effect.tree.set_rotation_z(effect.root, floor_angle);
                }
                // efasync.c:750-756; efLib_Create_Attach, eflib.c:538-555.
                EffectRequest::EntryWarp { scale, .. } => {
                    effect.tree.set_scale(effect.root, &scale)
                }
                EffectRequest::Landing {
                    offset,
                    floor_angle,
                    ..
                } => {
                    // efasync.c:1350-1355: lb_8000B1CC transforms the queued
                    // offset by the fighter root, then 0x404 dispatches 0x18.
                    mtx_mult_vec(&matrix, &offset, &mut position);
                    effect.tree.set_rotation_z(effect.root, floor_angle);
                }
            }
            effect.tree.set_translate(effect.root, &position);
            // efasync.c:1122-1126 drains initial HSD_JObjAnimAll immediately.
            effect.animate::<T>(bank, particles, rng, &mut self.draws, &mut self.events)?;
            self.instances.push(effect);
        }
        Ok(())
    }
    /// efLib_Update (eflib.c:387-431), s_link 15/p_link 11/priority 0.
    pub fn tick<T: InverseTrig>(
        &mut self,
        bone_matrix: impl FnMut(usize, Option<usize>) -> Mtx,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.tick_with_pause::<T>(false, bone_matrix, bank, particles, rng)
    }
    /// gm_803DA888[4] pauses p_link 11; KO models on p_link 12 keep updating.
    pub fn tick_with_pause<T: InverseTrig>(
        &mut self,
        pause_async: bool,
        mut bone_matrix: impl FnMut(usize, Option<usize>) -> Mtx,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        // Camera requests have no simulation/RNG output. Bound their lifetime
        // to the owning frame; a renderer may drain them before this update.
        while self.camera_quakes.pop().is_some() {}
        for (index, active) in self.fighter_joints.iter_mut().enumerate() {
            if !*active {
                continue;
            }
            let id = FIRST_FIGHTER_JOINT + index;
            *active = particles
                .generators
                .iter()
                .any(|g| g.attachment_id == Some(id));
            if *active {
                let matrix = bone_matrix(
                    index / FIGHTER_JOINT_STRIDE,
                    Some(index % FIGHTER_JOINT_STRIDE),
                );
                self.events.update_joint(id, matrix);
                particles.update_joint(id, matrix);
            }
        }
        for effect in self.instances.iter_mut() {
            if pause_async && effect.descriptor != 0x19 {
                continue;
            }
            if !effect.indefinite && effect.lifetime != 0 {
                effect.lifetime -= 1;
                if effect.lifetime == 0 {
                    continue;
                }
            }
            if let Some(player) = effect.attachment {
                let matrix = bone_matrix(player, effect.attachment_bone);
                if effect.attachment_bone.is_some() && effect.scale_attachment {
                    // efLib_Update (8005BC50), eflib.c:406-425: world Y scale,
                    // broadcast to all three axes. HSD_MtxGetScale is audited.
                    let mut scale = Vec3::ZERO;
                    hsd_anim::mtx::hsd_mtx_get_scale(&matrix, &mut scale);
                    scale.x = scale.y;
                    scale.z = scale.y;
                    effect.tree.set_scale(effect.root, &scale);
                }
                // lb_8000C1C0's position constraint (0x90000001) resolves the
                // attachment's world translation; it does not inherit rotation.
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                );
            }
            effect.animate_banks::<T>(
                (bank, self.fox_bank.as_ref(), self.mars_bank.as_ref()),
                particles,
                rng,
                &mut self.draws,
                &mut self.events,
            )?;
            // efLib_Update invokes the effect callback after JObj animation.
            if let Some(rotation) = effect.callback_rotation {
                effect.tree.set_rotation_y(effect.root, rotation.y);
                effect.tree.set_rotation_z(effect.root, rotation.z);
                // The particle proc sees matrices dirtied by this post-animation callback.
                for index in 0..effect.joints.len() {
                    let joint = effect.joints[index];
                    let matrix = effect.matrix(joint);
                    self.events
                        .update_joint(effect.joint_base + joint.0, matrix);
                    particles.update_joint(effect.joint_base + joint.0, matrix);
                }
            }
            if let Some(velocity) = &mut effect.velocity {
                // efLib_Cb_SetOffset_FromParams (8005E950): separate fsubs/fadds.
                velocity.y -= 0.1;
                let position = effect.tree.get(effect.root).translate;
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(
                        position.x + velocity.x,
                        position.y + velocity.y,
                        position.z + velocity.z,
                    ),
                );
            }
        }
        self.recycle_where(|effect| !effect.indefinite && effect.lifetime == 0);
        Ok(())
    }
}
impl Effect {
    /// efLib_Create (eflib.c:433-536): table index, model, lifetime, animation.
    fn load(archive: &Archive, id: u32) -> Result<Self> {
        Self::load_table(archive, "effCommonDataTable", id, id, 0)
    }
    fn load_table(
        archive: &Archive,
        table_name: &str,
        index: u32,
        id: u32,
        bank: u8,
    ) -> Result<Self> {
        let table = archive.public(table_name).context("effect table")?;
        let offset = table + 8 + index * EFFECT_DESCRIPTOR_SIZE;
        let lifetime = archive.reader().f32(offset)? as u16;

        let descriptor =
            desc::JObjDesc::read(archive, archive.link(offset + 4)?.context("effect model")?)?;
        let animation = archive
            .link(offset + 8)?
            .map(|offset| desc::AnimJoint::read(archive, offset))
            .transpose()?;
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&joint_spec(&descriptor)?);
        let ids: Vec<_> = tree.depth_first(root).collect();
        let descs = descriptor.descendants();
        let mut paths = BTreeMap::new();
        let mut references = BTreeMap::new();
        if let Some(animation) = &animation {
            attach(&mut tree, root, animation, &mut references)?;
        }
        for &id in &ids {
            let reference = references.get(&id.0).copied().unwrap_or(0);
            if reference == 0 {
                continue;
            }
            let index = descs
                .iter()
                .position(|d| d.offset == reference)
                .context("unresolved spline joint")?;
            let desc::JObjUnion::Spline(Some(offset)) = descs[index].u else {
                anyhow::bail!("animation reference is not a spline");
            };
            paths.insert(id.0, (ids[index], spline::Spline::read(archive, offset)?));
        }
        // KEY packs can emit multiple callbacks in one step. Bound each joint
        // by its stream bytes plus endpoint callbacks, including an AObj rewind's
        // stop+interpret pair. The event buffer is drained after each joint.
        let event_capacity = ids
            .iter()
            .map(|&id| {
                tree.get(id).aobj.as_ref().map_or(0, |a| {
                    2 * a.fobj.iter().map(|f| f.stream().len() + 2).sum::<usize>()
                })
            })
            .max()
            .unwrap_or(0);
        tree.events.reserve_exact(event_capacity);
        tree.req_anim_all(root, 0.0);
        Ok(Self {
            descriptor: id,
            bank,
            velocity: None,
            tree,
            root,
            lifetime: lifetime + 1,
            indefinite: lifetime == 0,
            attachment_bone: None,
            scale_attachment: true,
            callback_rotation: None,
            attachment: None,
            owner: None,
            joint_base: 0,
            joints: ids,
            paths,
        })
    }
    fn matrix(&mut self, joint: JObjId) -> Mtx {
        self.tree.setup_matrix(joint);
        if let Some((reference, _)) = self.paths.get(&joint.0) {
            // JObjMakeMatrix, jobj.c:187-195: spline-reference translation
            // uses the referenced joint's world matrix after ordinary SRT.
            self.tree.setup_matrix(*reference);
            let mut position = Vec3::ZERO;
            mtx_mult_vec(
                &self.tree.get(*reference).mtx,
                &self.tree.get(joint).translate,
                &mut position,
            );
            let matrix = &mut self.tree.get_mut(joint).mtx.0;
            matrix[0][3] = position.x;
            matrix[1][3] = position.y;
            matrix[2][3] = position.z;
        }
        self.tree.get(joint).mtx
    }
    fn animate<T: InverseTrig>(
        &mut self,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
        sink: &mut crate::fixture_spawns::EventSink,
    ) -> Result<()> {
        self.animate_banks::<T>((bank, None, None), particles, rng, draws, sink)
    }
    fn animate_banks<T: InverseTrig>(
        &mut self,
        banks: (&ParticleBank, Option<&ParticleBank>, Option<&ParticleBank>),
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
        sink: &mut crate::fixture_spawns::EventSink,
    ) -> Result<()> {
        let mut cb = hsd_anim::aobj::AObjEndCallback::default();
        for index in 0..self.joints.len() {
            let joint = self.joints[index];
            self.tree.anim::<T>(joint, &mut cb);
            let mut events = std::mem::take(&mut self.tree.events);
            for event in events.drain(..) {
                match event {
                    JObjEvent::Path { jobj, t } => {
                        let (_, spline) = self.paths.get(&jobj.0).context("PATH without spline")?;
                        self.tree.set_translate(jobj, &spline.point(t));
                    }
                    JObjEvent::DPtcl { jobj, lo, hi } => {
                        // efLib_Cb_DPtcl -> efLib_SpawnParticleEffect,
                        // eflib.c:857-1013: these kinds take hsd_8039EFAC(0,...).
                        let bank = match lo {
                            0 => banks.0,
                            3 => banks.1.context("Fox particle bank not loaded")?,
                            16 => banks.2.context("Marth particle bank not loaded")?,
                            _ => anyhow::bail!("unsupported effect particle bank {lo}"),
                        };
                        ensure!(
                            (lo == 0 && (PARTICLE_KINDS.contains(&hi) || hi == 0x102))
                                || (matches!(self.bank, 3 | 16)
                                    && lo == i32::from(self.bank)
                                    && bank.descriptor(hi as u32).is_some()),
                            "unsupported ef particle {lo}/{hi}"
                        );
                        if hi == 0xD4 {
                            // efLib_SpawnParticleEffect (8005D174): standalone link 2,
                            // root SRT overrides; no live joint attachment.
                            let mut request = SpawnRequest::new(lo as u8, hi as u32, 2);
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    translation: self.tree.translation(self.root),
                                    rotation: Vec3::new(
                                        0.0,
                                        0.0,
                                        self.tree.get(self.root).rotate.z,
                                    ),
                                    scale: self.tree.scale(self.root),
                                    ..Default::default()
                                });
                            sink.spawn(&request, false, false);
                            spawn_particle::<T>(particles, bank, request, rng, draws)?;
                            continue;
                        }
                        let mut request = SpawnRequest::new(lo as u8, hi as u32, 0);
                        request.joint = Some((self.joint_base + jobj.0, self.matrix(jobj)));
                        if matches!(hi, 2 | 6 | 306 | 307) {
                            // efLib_SpawnParticleEffect (8005D174): inherit root scale.
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    scale: self.tree.get(self.root).scale,
                                    ..Default::default()
                                });
                        }
                        sink.spawn(&request, false, false);
                        let id = spawn_particle::<T>(particles, bank, request, rng, draws)?;
                        if matches!(hi, 2 | 6 | 306 | 307) {
                            if let Some(id) = id {
                                let generator = particles.generator_mut(id).unwrap();
                                sink.flags(self.joint_base + jobj.0, 0x600, 0x800);
                                generator.flags = (generator.flags & !0x600) | 0x800;
                            }
                        }
                        if matches!(hi, 0x2D | 0x2E | 0x31) {
                            // efLib_SpawnParticleEffect (8005D174), eflib.c:882-890.
                            if let Some(id) = id {
                                let generator = particles.generator_mut(id).unwrap();
                                sink.flags(self.joint_base + jobj.0, 0x600, 0x1800);
                                generator.flags = (generator.flags & !0x600) | 0x1800;
                            }
                        }
                    }
                    JObjEvent::JSound(_) => {} // Audio has no simulation output.
                    _ => anyhow::bail!("unsupported ef animation event {event:?}"),
                }
            }
            self.tree.events = events;
        }
        for index in 0..self.joints.len() {
            let joint = self.joints[index];
            let matrix = self.matrix(joint);
            sink.update_joint(self.joint_base + joint.0, matrix);
            particles.update_joint(self.joint_base + joint.0, matrix);
        }
        Ok(())
    }
}
fn vector(v: desc::Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}
/// Headless HSD_JObjLoadJoint: retain spline joints and reject constraints.
/// DObjs/materials only render; SRT, flags and all joint FObjs are retained.
fn joint_spec(d: &desc::JObjDesc) -> Result<JointSpec> {
    ensure!(
        d.robjdesc.is_none() && d.instance_of.is_none(),
        "unsupported effect constraint/instance"
    );
    Ok(JointSpec {
        flags: d.flags,
        id: d.offset,
        rotation: vector(d.rotation),
        scale: vector(d.scale),
        position: vector(d.position),
        mtx: d.mtx.map(Mtx),
        children: d.children().map(joint_spec).collect::<Result<_>>()?,
        ..JointSpec::new()
    })
}
fn attach(
    tree: &mut JObjTree,
    id: JObjId,
    node: &desc::AnimJoint,
    references: &mut BTreeMap<usize, u32>,
) -> Result<()> {
    ensure!(
        node.robj_anim.is_none(),
        "effect RObj animation unsupported"
    );
    let animation = AnimJoint {
        flags: node.flags,
        aobjdesc: node.aobjdesc.as_ref().map(|a| AObjDesc {
            flags: a.flags,
            end_frame: a.end_frame,
            obj_id: a.obj_id,
            fobjdesc: a
                .tracks()
                .map(|f| FObjDesc {
                    length: f.length,
                    startframe: f.startframe,
                    obj_type: f.type_,
                    frac_value: f.frac_value,
                    frac_slope: f.frac_slope,
                    ad: f.ad.clone(),
                })
                .collect(),
        }),
        children: vec![],
    };
    tree.add_anim(id, Some(&animation), None);
    if let Some(a) = &node.aobjdesc {
        references.insert(id.0, a.obj_id);
    }
    let mut child = tree.child(id);
    let mut anim = node.child.as_deref();
    while let (Some(id), Some(node)) = (child, anim) {
        attach(tree, id, node, references)?;
        child = tree.get(id).next;
        anim = node.next.as_deref();
    }
    ensure!(
        child.is_none() && anim.is_none(),
        "effect animation tree mismatch"
    );
    Ok(())
}

#[cfg(test)]
mod tests;

/// Generator creation (hsd_8039F05C) records at most its one initial emission
/// draw. Particle storage/texture/descriptor ownership remains in hsd-particle.
fn spawn_particle<T: InverseTrig>(
    particles: &mut ParticleSystem,
    bank: &ParticleBank,
    request: SpawnRequest,
    rng: &mut HsdRng,
    draws: &mut DrawLog,
) -> Result<Option<usize>> {
    assert!(
        draws.0.len() < DRAW_CAPACITY,
        "effect draw log capacity exhausted"
    );
    Ok(particles.spawn::<T>(bank, request, rng, draws)?)
}
