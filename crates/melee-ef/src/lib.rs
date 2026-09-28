//! Headless ef subsystem: effect descriptor loading, efAsync request dispatch, attached and
//! positional effect instances, lifetime/animation procs, and DPtcl routing.
//! Async kind 2 transforms a queued bone-local offset before generator dispatch;
//! running dust also uses kinds 5/6 for facing and floor-angle parameters.
//! The warp additionally needs the HSD spline/reference evaluator in `spline`.
//! No spawn schedules or captured matrices are runtime inputs.
mod dust;
mod egg_shell;
mod item_generators;
pub mod fixture_spawns;
mod pool;
mod resources;
mod visual;
pub use resources::{CharacterEffectFile, Resources, CHARACTER_EFFECT_FILES};
pub use visual::{VisualModel, VISUAL_CAPACITY};
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
use std::{collections::BTreeMap, sync::Arc};
use tables::*;

// Stable fighter-bone identities occupy a range beyond effect model joints.
const FIRST_FIGHTER_JOINT: usize = 1 << 24;
const FIGHTER_JOINT_STRIDE: usize = 256;
// EF_EffectDesc: lifetime plus four model/animation pointers (ef/types.h).

// Port-only diagnostic budget, drained by the scheduler each tick. Retail has
// no Rust draw log; fail explicitly rather than growing this observation buffer.
const DRAW_CAPACITY: usize = 4096;
// Stage joint identities occupy the low range; effects own monotonic IDs.
pub const FIRST_EFFECT_JOINT: usize = 1 << 16;
/// Particle bank of the stage's own programs (`id / 1000`).
const STAGE_BANK: u16 = 30;

#[derive(Clone)]
pub struct Effects {
    pub events: crate::fixture_spawns::EventSink,
    camera_quakes: FixedVec<(u16, Vec3), { request::REQUEST_CAPACITY }>,
    pub draws: DrawLog,
    /// Ordered direct efAsync/ftColl draws; diagnostics use fixed storage.
    pub direct_draws: FixedVec<u32, 64>,
    instances: FixedVec<Effect, INSTANCE_CAPACITY>,
    models: ModelPool,
    character_banks: resources::CharacterBanks,
    next_joint: usize,
    fighter_joints: [bool; 2 * FIGHTER_JOINT_STRIDE],
    /// Items whose JObj carries generators (item_generators.rs).
    item_joints: FixedVec<u32, 64>,
}
#[derive(Clone)]
struct Effect {
    visual: Arc<desc::effect_visual::EffectVisual>,
    descriptor: u32,
    bank: u8,
    velocity: Option<Vec3>,
    tree: JObjTree,
    root: JObjId,
    joints: Arc<[JObjId]>,
    attachment: Option<usize>,
    owner: Option<ModelOwner>,
    lifetime: u16,
    indefinite: bool,
    attachment_bone: Option<usize>,
    scale_attachment: bool,
    callback_rotation: Option<Vec3>,
    /// efLib_Cb_SetRotY_FromFighterDir (eflib.c:1142): root rotation Y set
    /// after each update's animation, never at creation.
    facing_rotation: Option<f32>,
    /// efLib_Cb_AccumOffset_FromParams (eflib.c:1502): after each update's
    /// animation, the root sits at (player, bone)'s world position plus a
    /// world-axis offset.
    follow_bone: Option<(usize, usize, Vec3)>,
    hitlag_pause: HitlagPause,
    joint_base: usize,
    paths: Arc<BTreeMap<usize, (JObjId, spline::Spline)>>,
}
/// efLib state_flags: bit0x80 bypasses the pause states in efLib_Update.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HitlagPause {
    Ignore,
    Active,
    Entering,
    Paused,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelOwner {
    Fighter(usize),
    Blaster(usize),
}
/// Retail efSync runs at the caller; efAsync drains at fighter link 9.
#[derive(Clone, Copy)]
pub enum EffectTiming {
    /// Motion-entry destruction precedes graphics from the newly installed script.
    BeforeGraphics,
    /// Requests a motion change sealed (efAsync_QueueFlush in
    /// Fighter_ChangeMotionState), ahead of that proc's later graphics.
    Sealed,
    Immediate,
    Deferred,
}
impl Effects {
    /// efAlt_Spawn(0x48E), efLib_Create_Attach_Pos(0xBBD): Fox table row 5.
    /// Loading is initialization-only; all 64 synchronous model slots are warm.
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
            resources::Banks {
                common: common_bank,
                characters: &self.character_banks,
            },
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
        )?;
        self.instances.push(effect);
        Ok(())
    }
    /// efAsync_Dispatch 0x446 (efasync.c:806-816) ->
    /// efLib_CreateGenerator_AppSRT_SetPos (eflib.c:1513): model 0 with a
    /// 0x16-tick lifetime, and generator 0x1B on its root through
    /// eflib_create_generator_add_appsrt. The root stays at the origin until
    /// the first update's callback moves it; model 0 has no animation to queue.
    fn spawn_following_generator<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        offset: Vec3,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        /// efAsync_Dispatch 0x446 overwrites the model's lifetime.
        const LIFETIME: u16 = 0x16;
        let mut effect = self.acquire(0, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        // efLib_Create leaves state_flags ACTIVE; no EF_STATE_ASYNC bit here.
        effect.hitlag_pause = HitlagPause::Active;
        effect.lifetime = LIFETIME;
        effect.indefinite = false;
        effect.follow_bone = Some((player, bone, offset));
        let joint = effect.joint_base + effect.root.0;
        let mut spawn = SpawnRequest::new(0, 0x1B, 0);
        spawn.joint = Some((joint, effect.matrix(effect.root)));
        // psAddGeneratorAppSRT_begin(generator, 0).
        spawn.application_transform = Some(Default::default());
        self.events.spawn(&spawn, false, false);
        if let Some(id) = spawn_particle::<T>(particles, common_bank, spawn, rng, &mut self.draws)?
        {
            self.events.flags(joint, 0x600, 0x800);
            let generator = particles.generator_mut(id).unwrap();
            generator.flags = (generator.flags & !0x600) | 0x800;
        }
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
            for &joint in effect.joints.iter() {
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
    /// Fighter_8006C80C at s_link 9 (fighter.c:2552-2557). `stage_bank` holds
    /// the stage's own programs (ids 30000-30999, bank 30), which terrain
    /// footsteps and landings request.
    #[allow(clippy::too_many_arguments)] // Fighter, effect assets and particle runtime stay in their own layers.
    pub fn flush<T: InverseTrig>(
        &mut self,
        timing: EffectTiming,
        player: usize,
        fighter: &mut impl EffectOwner,
        bank: &ParticleBank,
        stage_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        // efLib_CreateGenerator picks the bank from the id (id / 1000).
        let bank_of = |id: u16| {
            if id / 1000 == STAGE_BANK {
                stage_bank
            } else {
                bank
            }
        };
        // Most procs queue nothing; skip moving an empty fixed-capacity batch.
        if fighter.effect_queue().is_empty() {
            return Ok(());
        }
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
            if let EffectRequest::AttachedParameter {
                id: 0x490,
                bone,
                parameter,
            } = request
            {
                // efAlt_Spawn 0x490 (efalt.c:163-170): efLib_Create_Attach
                // 0xFA2 (no scale), params.z = the angle, and the update
                // callback efLib_Cb_SetRotYZ_FromParamZ_FighterDir.
                let mut effect = self.acquire(0xFA2, particles);
                effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
                self.next_joint += effect.tree.len();
                effect.owner = Some(ModelOwner::Fighter(player));
                effect.hitlag_pause = HitlagPause::Active;
                effect.attachment = Some(player);
                effect.attachment_bone = Some(bone);
                effect.scale_attachment = false;
                // eflib.c:1261-1276: M_PI_2_F by facing; Z negated facing right.
                effect.callback_rotation = Some(if fighter.effect_facing() < 0.0 {
                    Vec3::new(0.0, -std::f32::consts::FRAC_PI_2, parameter)
                } else {
                    Vec3::new(0.0, std::f32::consts::FRAC_PI_2, -parameter)
                });
                let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone)));
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                );
                effect.animate_banks::<T>(
                    resources::Banks {
                        common: bank,
                        characters: &self.character_banks,
                    },
                    particles,
                    rng,
                    &mut self.draws,
                    &mut self.events,
                )?;
                self.instances.push(effect);
                continue;
            }
            if let EffectRequest::SyncAttachedPair { id: 0x48F, bones } = request {
                // efAlt_Spawn 0x48F (efalt.c:153-161): two
                // efLib_Create_Attach_Scale_FacingDir models, 0xFA0 then 0xFA1.
                let mut pair = [0xFA0, 0xFA1].map(|model| {
                    let mut effect = self.acquire(model, particles);
                    effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
                    self.next_joint += effect.tree.len();
                    effect
                });
                for (effect, bone) in pair.iter_mut().zip(bones) {
                    effect.owner = Some(ModelOwner::Fighter(player));
                    // efAlt's attached character models retain state_flags=0.
                    effect.hitlag_pause = HitlagPause::Active;
                    effect.attachment = Some(player);
                    effect.attachment_bone = Some(bone);
                    effect.scale_attachment = false;
                    // HSD_JObjGetScale of the fighter root, broadcast from Y.
                    let mut scale = fighter.effect_scale();
                    scale.x = scale.y;
                    scale.z = scale.y;
                    effect.tree.set_scale(effect.root, &scale);
                    // efLib_Cb_SetRotY_FromFighterDir: an f64 +-M_PI_2, rounded.
                    effect.facing_rotation = Some(if fighter.effect_facing() < 0.0 {
                        -std::f64::consts::FRAC_PI_2 as f32
                    } else {
                        std::f64::consts::FRAC_PI_2 as f32
                    });
                    let matrix = fighter.effect_matrix(Some(bone));
                    effect.tree.set_translate(
                        effect.root,
                        &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                    );
                }
                // efAlt_Spawn drains efLib_AnimQueue newest first (efalt.c:534-542).
                for effect in pair.iter_mut().rev() {
                    effect.animate_banks::<T>(
                        resources::Banks {
                            common: bank,
                            characters: &self.character_banks,
                        },
                        particles,
                        rng,
                        &mut self.draws,
                        &mut self.events,
                    )?;
                }
                for effect in pair {
                    self.instances.push(effect);
                }
                continue;
            }
            if let EffectRequest::SyncAttached {
                id: id @ (0x488..=0x48C | 0x491..=0x493 | 0x4D6 | 0x4F2..=0x4F3),
                bone,
            } = request
            {
                let model = match id {
                    0x488..=0x48C => 0xBB8 + u32::from(id - 0x488),
                    // efalt.c:172-210: Raptor Boost's start and lunge models.
                    0x491 => 0xFA4,
                    0x492 => 0xFA3,
                    0x493 => 0xFA5,
                    // efsync.c:305-308: efLib_Create_Attach_Scale(0x2AF8).
                    0x4D6 => 0x2AF8,
                    0x4F2..=0x4F3 => 0x3E80 + u32::from(id - 0x4F2),
                    _ => unreachable!(),
                };
                // efLib_Create_Attach_Scale, and a root rotation Y from the
                // fighter's facing at creation (efAlt 0x492/0x493, 0x4F2/0x4F3).
                let scaled = matches!(id, 0x488..=0x48A | 0x492..=0x493 | 0x4D6 | 0x4F2..=0x4F3);
                let faces = matches!(id, 0x492..=0x493 | 0x4F2..=0x4F3);
                let mut effect = self.acquire(model, particles);
                effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
                self.next_joint += effect.tree.len();
                effect.owner = Some(ModelOwner::Fighter(player));
                // efAlt's attached character models retain state_flags=0.
                effect.hitlag_pause = HitlagPause::Active;
                effect.attachment = Some(player);
                effect.attachment_bone = Some(bone);
                effect.scale_attachment = false;
                if scaled {
                    // efLib_Create_Attach_Scale: the fighter root supplies uniform scale.
                    let mut scale = fighter.effect_scale();
                    scale.x = scale.y;
                    scale.z = scale.y;
                    effect.tree.set_scale(effect.root, &scale);
                }
                if faces {
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
                    resources::Banks {
                        common: bank,
                        characters: &self.character_banks,
                    },
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
                let bank = resources::character_bank(&self.character_banks, bank_id.into())?;
                if let Some(id) = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?
                {
                    let generator = particles.generator_mut(id).unwrap();
                    self.events.flags(joint_id, 0x600, 0x800);
                    generator.flags = (generator.flags & !0x600) | 0x800;
                }
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::SyncAttached {
                id: id @ (0x4BE | 0x4BF),
                bone,
            } = request
            {
                // efsync.c:107-114: hsd_8039EFAC(0, 7, 0x1B5C / 0x1B5D, jobj),
                // and for 0x4BF also the common bank's 0x5F on the same joint.
                let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                let matrix = fighter.effect_matrix(Some(bone));
                let kind = if id == 0x4BE { 0x1B5C } else { 0x1B5D };
                let mut spawn = SpawnRequest::new(7, kind, 0);
                spawn.joint = Some((joint_id, matrix));
                self.events.spawn(&spawn, false, false);
                let character = resources::character_bank(&self.character_banks, 7)?;
                spawn_particle::<T>(particles, character, spawn, rng, &mut self.draws)?;
                if id == 0x4BF {
                    let mut spawn = SpawnRequest::new(0, 0x5F, 0);
                    spawn.joint = Some((joint_id, matrix));
                    self.events.spawn(&spawn, false, false);
                    spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                }
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::Attached { id: 0x4C0, bone } = request {
                // efAsync kind 0 -> efSync_Spawn 0x4C0 (efsync.c:115-117):
                // efLib_Create_Attach(0x1B58) on the live joint, no scale
                // inheritance; the fighter owns it for efLib_PauseAll.
                let mut effect = self.acquire(0x1B58, particles);
                effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
                self.next_joint += effect.tree.len();
                effect.owner = Some(ModelOwner::Fighter(player));
                effect.hitlag_pause = HitlagPause::Active;
                effect.attachment = Some(player);
                effect.attachment_bone = Some(bone);
                effect.scale_attachment = false;
                let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone)));
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                );
                effect.animate_banks::<T>(
                    resources::Banks {
                        common: bank,
                        characters: &self.character_banks,
                    },
                    particles,
                    rng,
                    &mut self.draws,
                    &mut self.events,
                )?;
                self.instances.push(effect);
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
                let generator = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                if let Some(generator) = generator.filter(|_| ATTACHED_CLEARS_B10.contains(&id)) {
                    // PSAPPSRT_UNK_B10 (1 << 10).
                    self.events.flags(joint_id, 0x400, 0);
                    let generator = particles.generator_mut(generator).unwrap();
                    generator.flags &= !0x400;
                }
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::DizzyStars { bone, scale } = request {
                // efLib_CreateGenerator_AppSRT_SetScale (8005CE48).
                let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                let mut spawn = SpawnRequest::new(0, 0xCE, 0);
                spawn.joint = Some((
                    joint_id,
                    resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone))),
                ));
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    scale: Vec3::new(scale, scale, scale),
                    ..Default::default()
                });
                self.events.spawn(&spawn, false, false);
                if let Some(id) = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?
                {
                    self.events.flags(joint_id, 0x600, 0x800);
                    let generator = particles.generator_mut(id).unwrap();
                    generator.flags = (generator.flags & !0x600) | 0x800;
                }
                self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
                continue;
            }
            if let EffectRequest::FollowingGenerator {
                id: 0x446,
                bone,
                offset,
            } = request
            {
                self.spawn_following_generator::<T>(player, bone, offset, bank, particles, rng)?;
                continue;
            }
            if let EffectRequest::ShieldBreak { bone, scale } = request {
                // efasync.c:506-519 -> efLib_CreateGenerator_AddAppSRT(0x31).
                let matrix = resolved_matrix.unwrap_or(fighter.effect_matrix(Some(bone)));
                let mut spawn = SpawnRequest::new(0, 0x31, 0);
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    translation: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                    scale: Vec3::new(scale, scale, scale),
                    status: 1,
                    ..Default::default()
                });
                self.events.spawn(&spawn, false, false);
                spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                continue;
            }
            if let EffectRequest::PositionalGenerator { id: 0x4C3, position } = request {
                // efsync.c:136-144: efLib_CreateGenerator_AddAppSRT(0x24C)
                // with the point as the AppSRT's translation (Thunder's cloud).
                let mut spawn = SpawnRequest::new(0, 0x24C, 0);
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    translation: position,
                    status: 1,
                    ..Default::default()
                });
                self.events.spawn(&spawn, false, false);
                spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
                continue;
            }
            if let EffectRequest::PositionalGenerator { id, position } = request {
                self.spawn_dust_generator::<T>(
                    id,
                    position,
                    fighter.effect_facing(),
                    bank,
                    particles,
                    rng,
                )?;
                continue;
            }
            if let EffectRequest::PowershieldSpark { position } = request {
                self.spawn_dust_generator::<T>(
                    27,
                    position,
                    fighter.effect_facing(),
                    bank,
                    particles,
                    rng,
                )?;
                continue;
            }
            if let EffectRequest::HitSpark {
                position, element, ..
            } = request
            {
                // ftColl hit_effect_ids -> efAsync_Dispatch. These branches
                // create positional generators, with no model or direct RNG.
                let generator = match element {
                    melee_types::HitElement::Electric => Some(0x3E9),
                    melee_types::HitElement::Fire => Some(0x3EA),
                    _ => None,
                };
                if let Some(generator) = generator {
                    self.spawn_dust_generator::<T>(
                        generator,
                        position,
                        fighter.effect_facing(),
                        bank,
                        particles,
                        rng,
                    )?;
                    continue;
                }
            }
            if let EffectRequest::LedgeGrab { position }
            | EffectRequest::ShieldSpark { position }
            | EffectRequest::Clank { position } = request
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
            // ftCo_8009F834: landing dust and a terrain's stage-bank splash
            // (ids 30000-30999) are positional generators.
            let positional_landing = match request {
                EffectRequest::Landing { id, offset, .. }
                    if matches!(id, 0x407 | 0x42D) || id / 1000 == STAGE_BANK =>
                {
                    Some((id, offset))
                }
                _ => None,
            };
            if let Some((id, offset)) = positional_landing {
                let mut position = Vec3::ZERO;
                mtx_mult_vec(
                    &resolved_matrix.unwrap_or(fighter.effect_matrix(None)),
                    &offset,
                    &mut position,
                );
                self.spawn_dust_generator::<T>(
                    id,
                    position,
                    fighter.effect_facing(),
                    bank_of(id),
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
                    if matches!(id, 0x513..=0x515) {
                        // efAsync kind 8 -> Camera_RequestQuake(2/3/4), no particle spawn.
                        self.camera_quakes.push((id - 0x511, position));
                    } else {
                        self.spawn_dust_generator::<T>(
                            id,
                            position,
                            facing,
                            bank_of(id),
                            particles,
                            rng,
                        )?;
                    }
                    continue;
                }
            }
            let (id, attachment) = match request {
                EffectRequest::SurfaceRebound { .. } => (4, None),
                EffectRequest::Death { .. } => (0x19, None),
                EffectRequest::CaptureFlash { .. } | EffectRequest::WallJump { .. } => (0xF, None),
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
                EffectRequest::PositionalModel { id, .. } => {
                    let (_, model) = POSITIONAL_MODELS
                        .iter()
                        .find(|(request, _)| *request == id)
                        .context("unsupported positional model effect")?;
                    (*model, None)
                }
                EffectRequest::Graphics { id, .. }
                | EffectRequest::PositionalGraphics { id, .. }
                | EffectRequest::Shield { id, .. }
                | EffectRequest::EntryWarp { id, .. }
                | EffectRequest::Landing { id, .. } => {
                    let source = match request {
                        EffectRequest::Graphics { .. }
                        | EffectRequest::PositionalGraphics { .. } => ModelSource::Graphics,
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
            if matches!(request, EffectRequest::Shield { id: 0x41A, .. }) {
                // efLib_Create_Attach leaves state_flags ACTIVE (no ASYNC bit).
                // Hitlag eligibility is separate from the synchronous pool.
                effect.hitlag_pause = HitlagPause::Active;
            }
            effect.owner = if matches!(request, EffectRequest::HitSpark { .. }) {
                None
            } else {
                Some(ModelOwner::Fighter(player))
            };
            if let EffectRequest::Shield { bone, .. }
            | EffectRequest::Graphics {
                id: 0x423 | 0x424 | 0x4C1 | 0x4C2 | 0x4C4 | 0x4C5,
                bone,
                ..
            } = request
            {
                effect.attachment_bone = Some(bone);
            }
            let scaled_facing = matches!(
                request,
                EffectRequest::Graphics { id, .. } if scaled_facing_graphics(id)
            );
            if scaled_facing {
                // efLib_Create_Attach_Scale_FacingDir (eflib.c:606-628): the
                // fighter root's Y scale on all axes, no scale inheritance,
                // and efLib_Cb_SetRotY_FromFighterDir as the update callback
                // (an f64 +-M_PI_2, rounded). The fighter owns it for
                // efLib_PauseAll.
                let mut scale = fighter.effect_scale();
                scale.x = scale.y;
                scale.z = scale.y;
                effect.tree.set_scale(effect.root, &scale);
                effect.scale_attachment = false;
                effect.hitlag_pause = HitlagPause::Active;
                effect.facing_rotation = Some(if fighter.effect_facing() < 0.0 {
                    -std::f64::consts::FRAC_PI_2 as f32
                } else {
                    std::f64::consts::FRAC_PI_2 as f32
                });
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
                | EffectRequest::PositionalGenerator { .. }
                | EffectRequest::EggShell { .. }
                | EffectRequest::DamageTrail { .. }
                | EffectRequest::NormalSparkExtra { .. }
                | EffectRequest::DestroyOwned
                | EffectRequest::Attached { .. }
                | EffectRequest::AttachedParameter { .. }
                | EffectRequest::SyncAttached { .. }
                | EffectRequest::SyncAttachedPair { .. }
                | EffectRequest::FollowingGenerator { .. }
                | EffectRequest::LedgeGrab { .. }
                | EffectRequest::DizzyStars { .. }
                | EffectRequest::ShieldBreak { .. }
                | EffectRequest::PowershieldSpark { .. }
                | EffectRequest::ShieldSpark { .. }
                | EffectRequest::Clank { .. } => unreachable!(),
                // efLib_Create_Attach_Pos: HSD_JObjSetTranslate only.
                EffectRequest::PositionalModel {
                    position: origin, ..
                } => position = origin,
                EffectRequest::SurfaceRebound {
                    position: origin,
                    angle,
                } => {
                    // efAsync_Dispatch 0x406: model4, world position, rotation Z.
                    position = origin;
                    effect.tree.set_rotation_z(effect.root, angle);
                }
                EffectRequest::PositionalGraphics {
                    position: origin,
                    facing,
                    angle,
                    ..
                } => {
                    // efAsync_Dispatch 0x3FF: model 5 at the point, then
                    // efAsync_SetEffectFacingDir and the rotation Z.
                    position = origin;
                    effect.tree.set_rotation_y(
                        effect.root,
                        if facing < 0.0 {
                            -std::f32::consts::FRAC_PI_2
                        } else {
                            std::f32::consts::FRAC_PI_2
                        },
                    );
                    effect.tree.set_rotation_z(effect.root, angle);
                }
                EffectRequest::Death {
                    position: origin,
                    angle,
                    scale,
                } => {
                    // efasync.c:598: rotation Z, then the uniform scale.
                    position = origin;
                    effect.tree.set_rotation_z(effect.root, angle);
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
                EffectRequest::WallJump { position: origin } => {
                    position = origin;
                }
                EffectRequest::CaptureFlash { .. } | EffectRequest::Shield { .. } => {}
                EffectRequest::Graphics {
                    id,
                    offset,
                    facing,
                    floor_angle,
                    ..
                } => {
                    mtx_mult_vec(&matrix, &offset, &mut position);
                    if !matches!(
                        id,
                        0x3F6 | 0x3FA | 0x3FB | 0x3FC | 0x404 | 0x406 | 0x41D | 0x423 | 0x424
                    ) && !scaled_facing_graphics(id)
                    {
                        effect.tree.set_rotation_y(
                            effect.root,
                            if facing < 0.0 {
                                -std::f32::consts::FRAC_PI_2
                            } else {
                                std::f32::consts::FRAC_PI_2
                            },
                        );
                    }
                    // efasync.c:192-197: 0x3F5 takes the facing, no Z rotation;
                    // efasync.c:524-529: 0x41D sets no rotation at all.
                    if id != 0x3F5 && id != 0x41D {
                        effect.tree.set_rotation_z(effect.root, floor_angle);
                    }
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
            if scaled_facing {
                // efSync_Spawn's efLib_AnimQueue drain (efsync.c:654-660);
                // the model's particles come from its character bank.
                effect.animate_banks::<T>(
                    resources::Banks {
                        common: bank,
                        characters: &self.character_banks,
                    },
                    particles,
                    rng,
                    &mut self.draws,
                    &mut self.events,
                )?;
            } else {
                effect.animate::<T>(bank, particles, rng, &mut self.draws, &mut self.events)?;
            }
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
    /// efLib_PauseAll / ResumeAll: affect existing owned models only.
    pub fn set_owner_hitlag(&mut self, player: usize, paused: bool) {
        for effect in self.instances.iter_mut() {
            if effect.owner == Some(ModelOwner::Fighter(player))
                && effect.hitlag_pause != HitlagPause::Ignore
            {
                effect.hitlag_pause = if paused {
                    HitlagPause::Entering
                } else {
                    HitlagPause::Active
                };
            }
        }
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
        // The scene drains camera requests after each fighter proc
        // (`drain_camera_quakes`); bound any other owner's to its frame.
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
            // efLib_Update: the entering-pause call still advances once.
            match effect.hitlag_pause {
                HitlagPause::Paused => continue,
                HitlagPause::Entering => effect.hitlag_pause = HitlagPause::Paused,
                _ => {}
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
                resources::Banks {
                    common: bank,
                    characters: &self.character_banks,
                },
                particles,
                rng,
                &mut self.draws,
                &mut self.events,
            )?;
            // efLib_Update invokes the effect callback after JObj animation.
            if let Some((player, bone, offset)) = effect.follow_bone {
                let matrix = bone_matrix(player, Some(bone));
                // lb_8000B1CC, then three separate fadds.
                effect.tree.set_translate(
                    effect.root,
                    &Vec3::new(
                        matrix.0[0][3] + offset.x,
                        matrix.0[1][3] + offset.y,
                        matrix.0[2][3] + offset.z,
                    ),
                );
            }
            if effect.callback_rotation.is_some()
                || effect.facing_rotation.is_some()
                || effect.follow_bone.is_some()
            {
                if let Some(rotation) = effect.callback_rotation {
                    effect.tree.set_rotation_y(effect.root, rotation.y);
                    effect.tree.set_rotation_z(effect.root, rotation.z);
                }
                if let Some(rotation) = effect.facing_rotation {
                    effect.tree.set_rotation_y(effect.root, rotation);
                }
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
        let visual = Arc::new(desc::effect_visual::EffectVisual::read(
            archive, table, index,
        )?);
        let lifetime = visual.lifetime as u16;
        let descriptor = &visual.model;
        let animation = &visual.animation;
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&joint_spec(archive, descriptor)?);
        let ids: Vec<_> = tree.depth_first(root).collect();
        let descs = descriptor.descendants();
        let mut paths = BTreeMap::new();
        let mut references = BTreeMap::new();
        if let Some(animation) = &animation {
            attach(&mut tree, root, animation, &mut references)?;
        }
        if let Some(material) = &visual.material {
            let material = hsd_anim::load::material_animation(archive, material)?;
            tree.add_anim_all(root, None, Some(&material));
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
            visual,
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
            facing_rotation: None,
            follow_bone: None,
            hitlag_pause: HitlagPause::Ignore,
            attachment: None,
            owner: None,
            joint_base: 0,
            joints: ids.into(),
            paths: Arc::new(paths),
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
        const NO_CHARACTER_BANKS: resources::CharacterBanks =
            [const { None }; resources::CHARACTER_EFFECT_FILES.len()];
        let banks = resources::Banks {
            common: bank,
            characters: &NO_CHARACTER_BANKS,
        };
        self.animate_banks::<T>(banks, particles, rng, draws, sink)
    }
    fn animate_banks<T: InverseTrig>(
        &mut self,
        banks: resources::Banks<'_>,
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
                        let bank = banks.get(lo)?;
                        ensure!(
                            (lo == 0 && (PARTICLE_KINDS.contains(&hi) || hi == 0x102))
                                || (resources::is_character_bank(self.bank)
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
                        if matches!(hi, 0x16D..=0x170 | 0x7E2) {
                            // efLib_SpawnParticleEffect (8005D174), eflib.c:919-930:
                            // standalone in bank gfx_id / 1000 with an AppSRT (status
                            // 1) taking the root's rot.y and translation; no joint,
                            // so it outlives its owner.
                            let root = self.tree.get(self.root);
                            let mut request = SpawnRequest::new((hi / 1000) as u8, hi as u32, 0);
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    translation: self.tree.translation(self.root),
                                    rotation: Vec3::new(0.0, root.rotate.y, 0.0),
                                    status: 1,
                                    ..Default::default()
                                });
                            sink.spawn(&request, false, false);
                            spawn_particle::<T>(particles, bank, request, rng, draws)?;
                            continue;
                        }
                        let mut request = SpawnRequest::new(lo as u8, hi as u32, 0);
                        request.joint = Some((self.joint_base + jobj.0, self.matrix(jobj)));
                        if matches!(hi, 0x2D | 0x2E | 0x31) {
                            // eflib_create_generator_add_appsrt, eflib.c:64-83:
                            // the attached generator always owns an AppSRT.
                            request.application_transform = Some(Default::default());
                        }
                        if matches!(hi, 2 | 6 | 306 | 307) {
                            // efLib_SpawnParticleEffect (8005D174): inherit root scale.
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    scale: self.tree.get(self.root).scale,
                                    ..Default::default()
                                });
                        }
                        if hi == 0x127 {
                            // efLib_SpawnParticleEffect (8005D174), eflib.c:891-900:
                            // attached, with the root's rot.y and scale.
                            let root = self.tree.get(self.root);
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    rotation: Vec3::new(0.0, root.rotate.y, 0.0),
                                    scale: root.scale,
                                    ..Default::default()
                                });
                        }
                        sink.spawn(&request, false, false);
                        let id = spawn_particle::<T>(particles, bank, request, rng, draws)?;
                        if matches!(hi, 2 | 6 | 0x127 | 306 | 307) {
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
/// HSD_JObjLoadJoint: retain visual materials and the existing effect spline path.
fn joint_spec(archive: &Archive, d: &desc::JObjDesc) -> Result<JointSpec> {
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
        children: d
            .children()
            .map(|d| joint_spec(archive, d))
            .collect::<Result<_>>()?,
        dobj: hsd_anim::load::load_dobj_chain(archive, d.u.dobj())?,
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
