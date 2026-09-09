//! Headless ef subset. This should become `melee-ef` once that workspace member
//! is added: effect descriptor loading, efAsync request dispatch, attached and
//! positional effect instances, lifetime/animation procs, and DPtcl routing.
//! Async kind 2 transforms a queued bone-local offset before generator dispatch;
//! running dust also uses kinds 5/6 for facing and floor-angle parameters.
//! The warp additionally needs the HSD spline/reference evaluator in `spline`.
//! No spawn schedules or captured matrices are runtime inputs.
mod dust;
mod egg_shell;
mod spline;
use crate::scene_fighter::SceneFighter;
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
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
use melee_ft::fighter::CharacterCallbacks;
use melee_ft::fighter::{effects::EffectRequest, Fighter, RetailTrig};
use std::collections::BTreeMap;

// efasync.c:288-293,750-756: request IDs select common effect descriptors.
const ENTRY_WARP_REQUEST: u16 = 0x43E;
const ENTRY_WARP_EFFECT: u32 = 0x24;
const LANDING_REQUEST: u16 = 0x404;
const LANDING_EFFECT: u32 = 0x18;
// efasync.c:262-268: stationary model effect, with facing and floor rotation.
const DASH_DUST_REQUEST: u16 = 0x3FF;
const DASH_DUST_EFFECT: u32 = 5;
// efasync.c:205-212: jump flash model, also with facing and floor rotation.
const JUMP_FLASH_REQUEST: u16 = 0x3F7;
const JUMP_FLASH_EFFECT: u32 = 0x12;
// efasync.c:282-287: direct joint attachment, without a model effect.
const JUMP_DUST_REQUEST: u16 = 0x402;
const AERIAL_JUMP_DUST_REQUEST: u16 = 0x403;
const JUMP_DUST_PARTICLE: u32 = 0x59;
const AERIAL_JUMP_DUST_PARTICLE: u32 = 0x5E;
// Stable fighter-bone identities occupy a range beyond effect model joints.
const FIRST_FIGHTER_JOINT: usize = 1 << 24;
const FIGHTER_JOINT_STRIDE: usize = 256;
// EF_EffectDesc: lifetime plus four model/animation pointers (ef/types.h).
const EFFECT_DESCRIPTOR_SIZE: u32 = 20;
// Stage joint identities occupy the low range; effects own monotonic IDs.
pub(crate) const FIRST_EFFECT_JOINT: usize = 1 << 16;
// EfCoData animation outputs supported by efLib_SpawnParticleEffect's ordinary branch.
const PARTICLE_KINDS: [i32; 13] = [2, 9, 10, 45, 212, 261, 267, 306, 307, 364, 445, 448, 449];

#[derive(Default)]
pub(crate) struct Effects {
    camera_quakes: Vec<(u16, Vec3)>,
    pub(crate) draws: DrawLog,
    instances: Vec<Effect>,
    next_joint: usize,
    fighter_joints: BTreeMap<usize, (usize, usize)>,
}
struct Effect {
    velocity: Option<Vec3>,
    tree: JObjTree,
    root: JObjId,
    attachment: Option<usize>,
    owner: Option<usize>,
    lifetime: u16,
    indefinite: bool,
    shield_bone: Option<usize>,
    joint_base: usize,
    paths: BTreeMap<usize, (JObjId, spline::Spline)>,
}
/// Retail efSync runs at the caller; efAsync drains at fighter link 9.
#[derive(Clone, Copy)]
pub enum EffectTiming {
    Immediate,
    Deferred,
}
impl Effects {
    #[cfg(test)]
    pub fn matrices(&self) -> BTreeMap<usize, Mtx> {
        let mut matrices = BTreeMap::new();
        for effect in &self.instances {
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
    pub fn flush<C: CharacterCallbacks>(
        &mut self,
        timing: EffectTiming,
        player: usize,
        fighter: &mut Fighter<C>,
        archive: &Archive,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut requests = Vec::new();
        match timing {
            EffectTiming::Immediate => fighter.drain_immediate_effects(&mut requests),
            EffectTiming::Deferred => {
                fighter.drain_effects(&mut requests);
                // efAsync_Spawn prepends to the pending list.
                requests.reverse();
            }
        }
        // Motion changes have already sealed earlier queues with their outgoing
        // transforms. Preserve those flushes relative to immediate destruction.
        let requests = requests.into_iter().flat_map(|request| match request {
            EffectRequest::FlushDeferred(batch) => batch
                .into_iter()
                .map(|resolved| (resolved.request, Some(resolved.matrix)))
                .collect::<Vec<_>>(),
            request => vec![(request, None)],
        });
        for (request, resolved_matrix) in requests {
            if let EffectRequest::EggShell { bone, scale } = request {
                let joint = fighter.animation.parts[bone].joint;
                fighter.skeleton.setup_matrix(joint);
                let matrix = resolved_matrix.unwrap_or(fighter.skeleton.get(joint).mtx);
                self.spawn_egg_shell(archive, bank, particles, rng, &matrix, scale)?;
                continue;
            }
            if matches!(request, EffectRequest::DestroyOwned) {
                for effect in self
                    .instances
                    .iter()
                    .filter(|effect| effect.owner == Some(player))
                {
                    for joint in effect.tree.depth_first(effect.root) {
                        particles.expire_joint(effect.joint_base + joint.0);
                    }
                }
                self.instances.retain(|effect| effect.owner != Some(player));
                continue;
            }
            if let EffectRequest::Attached { id, bone } = request {
                // efasync.c:282-287: kind 0, hsd_8039EFAC on the live bone.
                let kind = match id {
                    JUMP_DUST_REQUEST => JUMP_DUST_PARTICLE,
                    AERIAL_JUMP_DUST_REQUEST => AERIAL_JUMP_DUST_PARTICLE,
                    _ => anyhow::bail!("unsupported attached effect {id:#x}"),
                };
                assert!(bone < FIGHTER_JOINT_STRIDE);
                let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
                let joint = fighter.animation.parts[bone].joint;
                fighter.skeleton.setup_matrix(joint);
                let mut spawn = SpawnRequest::new(0, kind, 0);
                spawn.joint = Some((
                    joint_id,
                    resolved_matrix.unwrap_or(fighter.skeleton.get(joint).mtx),
                ));
                particles.spawn::<RetailTrig>(bank, spawn, rng, &mut self.draws)?;
                self.fighter_joints.insert(joint_id, (player, bone));
                continue;
            }
            if let EffectRequest::LedgeGrab { position } | EffectRequest::ShieldSpark { position } =
                request
            {
                self.spawn_dust_generator(
                    0x41C,
                    position,
                    fighter.physics.facing,
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
                let joint = fighter.animation.root;
                fighter.skeleton.setup_matrix(joint);
                let mut position = Vec3::ZERO;
                mtx_mult_vec(
                    &resolved_matrix.unwrap_or(fighter.skeleton.get(joint).mtx),
                    &offset,
                    &mut position,
                );
                self.spawn_dust_generator(
                    0x407,
                    position,
                    fighter.physics.facing,
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
                if !matches!(id, 0x3F8 | 0x406 | DASH_DUST_REQUEST | JUMP_FLASH_REQUEST) {
                    let joint = fighter.animation.parts[bone].joint;
                    fighter.skeleton.setup_matrix(joint);
                    let mut position = Vec3::ZERO;
                    mtx_mult_vec(
                        &resolved_matrix.unwrap_or(fighter.skeleton.get(joint).mtx),
                        &offset,
                        &mut position,
                    );
                    if id == 0x514 {
                        // efAsync kind 8 -> Camera_RequestQuake(3), no particle spawn.
                        self.camera_quakes.push((3, position));
                    } else {
                        self.spawn_dust_generator(id, position, facing, bank, particles, rng)?;
                    }
                    continue;
                }
            }
            let (id, attachment) = match request {
                EffectRequest::Death { .. } => (0x19, None),
                EffectRequest::CaptureFlash { .. } => (0xF, None),
                EffectRequest::Graphics { id: 0x3F8, .. } => (0x13, None),
                EffectRequest::Graphics { id: 0x406, .. } => (4, None),
                EffectRequest::HitSpark {
                    element: melee_types::HitElement::Normal,
                    ..
                } => (if rng.randi(8) == 0 { 9 } else { 10 }, None),
                EffectRequest::HitSpark {
                    element: melee_types::HitElement::Slash,
                    ..
                } => (8, None),
                EffectRequest::Shield { id: 0x417, .. } => (0xB, Some(player)),
                EffectRequest::Shield { id: 0x419, .. } => (0xD, Some(player)),
                EffectRequest::Shield { id: 0x418, .. } => (0xC, Some(player)),
                EffectRequest::EntryWarp {
                    id: ENTRY_WARP_REQUEST,
                    ..
                } => (ENTRY_WARP_EFFECT, Some(player)),
                EffectRequest::Landing {
                    id: LANDING_REQUEST,
                    ..
                } => (LANDING_EFFECT, None),
                EffectRequest::Graphics {
                    id: DASH_DUST_REQUEST,
                    ..
                } => (DASH_DUST_EFFECT, None),
                EffectRequest::Graphics {
                    id: JUMP_FLASH_REQUEST,
                    ..
                } => (JUMP_FLASH_EFFECT, None),
                _ => anyhow::bail!("unsupported fighter effect {request:?}"),
            };
            let mut effect = Effect::load(archive, id)?;
            // Reserve disjoint owned joint identities; stage IDs occupy 0..65536.
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect.attachment = attachment;
            effect.owner = if matches!(request, EffectRequest::HitSpark { .. }) {
                None
            } else {
                Some(player)
            };
            if let EffectRequest::Shield { bone, .. } = request {
                effect.shield_bone = Some(bone);
            }
            let root = if let EffectRequest::CaptureFlash { bone }
            | EffectRequest::Graphics { bone, .. }
            | EffectRequest::Shield { bone, .. } = request
            {
                fighter.animation.parts[bone].joint
            } else {
                fighter.animation.root
            };
            fighter.skeleton.setup_matrix(root);
            let matrix = resolved_matrix.unwrap_or(fighter.skeleton.get(root).mtx);
            let mut position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
            match request {
                EffectRequest::EggShell { .. }
                | EffectRequest::FlushDeferred(_)
                | EffectRequest::DestroyOwned
                | EffectRequest::Attached { .. }
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
                } => {
                    position = contact;
                    if element == melee_types::HitElement::Normal {
                        // efAsync_Dispatch, retail 80063A10: fmadds.
                        let scale = gekko_math::fma::fmadds(0.04, damage, 0.3).clamp(0.3, 1.5);
                        effect
                            .tree
                            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
                    } else {
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
                    if id != 0x406 {
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
            effect.animate(bank, particles, rng, &mut self.draws)?;
            self.instances.push(effect);
        }
        Ok(())
    }
    /// efLib_Update (eflib.c:387-431), s_link 15/p_link 11/priority 0.
    pub fn tick(
        &mut self,
        fighters: &mut [SceneFighter; 2],
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.fighter_joints.retain(|id, _| {
            particles
                .generators
                .iter()
                .any(|g| g.attachment_id == Some(*id))
        });
        for (&id, &(player, bone)) in &self.fighter_joints {
            let fighter = &mut fighters[player];
            particles.update_joint(id, fighter.bone_matrix(Some(bone)));
        }
        for effect in &mut self.instances {
            if !effect.indefinite && effect.lifetime != 0 {
                effect.lifetime -= 1;
                if effect.lifetime == 0 {
                    continue;
                }
            }
            if let Some(player) = effect.attachment {
                let fighter = &mut fighters[player];
                let matrix = fighter.bone_matrix(effect.shield_bone);
                if effect.shield_bone.is_some() {
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
            effect.animate(bank, particles, rng, &mut self.draws)?;
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
        self.instances
            .retain(|effect| effect.indefinite || effect.lifetime != 0);
        Ok(())
    }
}
impl Effect {
    /// efLib_Create (eflib.c:433-536): table index, model, lifetime, animation.
    fn load(archive: &Archive, id: u32) -> Result<Self> {
        let table = archive
            .public("effCommonDataTable")
            .context("effect table")?;
        let offset = table + 8 + id * EFFECT_DESCRIPTOR_SIZE;
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
        tree.req_anim_all(root, 0.0);
        Ok(Self {
            velocity: None,
            tree,
            root,
            lifetime: lifetime + 1,
            indefinite: lifetime == 0,
            shield_bone: None,
            attachment: None,
            owner: None,
            joint_base: 0,
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
    fn animate(
        &mut self,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<()> {
        let joints: Vec<_> = self.tree.depth_first(self.root).collect();
        let mut cb = hsd_anim::aobj::AObjEndCallback::default();
        for joint in &joints {
            self.tree.anim::<RetailTrig>(*joint, &mut cb);
            for event in std::mem::take(&mut self.tree.events) {
                match event {
                    JObjEvent::Path { jobj, t } => {
                        let (_, spline) = self.paths.get(&jobj.0).context("PATH without spline")?;
                        self.tree.set_translate(jobj, &spline.point(t));
                    }
                    JObjEvent::DPtcl { jobj, lo, hi } => {
                        // efLib_Cb_DPtcl -> efLib_SpawnParticleEffect,
                        // eflib.c:857-1013: these kinds take hsd_8039EFAC(0,...).
                        ensure!(
                            lo == 0 && PARTICLE_KINDS.contains(&hi),
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
                            particles.spawn::<RetailTrig>(bank, request, rng, draws)?;
                            continue;
                        }
                        let mut request = SpawnRequest::new(lo as u8, hi as u32, 0);
                        request.joint = Some((self.joint_base + jobj.0, self.matrix(jobj)));
                        if matches!(hi, 2 | 306 | 307) {
                            // efLib_SpawnParticleEffect (8005D174): inherit root scale.
                            request.application_transform =
                                Some(hsd_particle::generator::ApplicationTransform {
                                    scale: self.tree.get(self.root).scale,
                                    ..Default::default()
                                });
                        }
                        let id = particles.spawn::<RetailTrig>(bank, request, rng, draws)?;
                        if matches!(hi, 2 | 306 | 307) {
                            if let Some(id) = id {
                                let generator = particles.generator_mut(id).unwrap();
                                generator.flags = (generator.flags & !0x600) | 0x800;
                            }
                        }
                        if matches!(hi, 0x2D | 0x2E | 0x31) {
                            // efLib_SpawnParticleEffect (8005D174), eflib.c:882-890.
                            if let Some(id) = id {
                                let generator = particles.generator_mut(id).unwrap();
                                generator.flags = (generator.flags & !0x600) | 0x1800;
                            }
                        }
                    }
                    JObjEvent::JSound(_) => {} // Audio has no simulation output.
                    _ => anyhow::bail!("unsupported ef animation event {event:?}"),
                }
            }
        }
        for joint in joints {
            let matrix = self.matrix(joint);
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
