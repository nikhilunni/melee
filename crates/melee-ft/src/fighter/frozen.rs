//! Frozen in a block of ice: ftCo_DamageIce.c (80090984..80091A2C).
//!
//! An Ice hit at knockback level 2 or 3 takes the ordinary damage entry in
//! DamageFlyTop and then freezes the victim (DamageIce): the launch becomes
//! the block's own velocity, the body's capsules give way to one round
//! capsule on XRotN and the map sees a square box of the same size. The block
//! falls under reduced gravity, bounces off walls and ceilings (or shatters
//! on them when fast enough), and slides once it lands. A timer started from
//! the hit's damage runs out, faster under mashing, and the fighter hops out
//! in DamageIceJump. A hit while frozen launches the block again; further
//! damage shortens the timer and fire ends it.
use super::{
    assets::{FighterAssets, Result},
    caches::bone_position,
    ledge::GrabExclusions,
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter, MotionData, MotionEntryFlags,
};
use crate::{
    anim::WaitChoice,
    collision::{air, ground},
};
use gekko_math::fma::{fmadds, fnmsubs};
use gekko_math::msl::sqrtf;
use hsd_types::{Vec2, Vec3};
use melee_ef::request::EffectRequest;
use melee_types::{
    combat::HurtStatus,
    mp::{collide as env, FtCollisionBox},
    CommonMotionState as S, FtPart, GroundOrAir, HitElement,
};

/// PlCo +714 and +77C..+7A4.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +714: the share of a hit's damage a frozen fighter takes.
    pub damage_scale: f32,
    /// +77C: the block's share of the fighter's gravity.
    pub gravity_scale: f32,
    /// +780: above this speed the block shatters on a wall or ceiling.
    pub shatter_speed: f32,
    /// +784: what a bounce keeps of the mirrored velocity.
    pub bounce_scale: f32,
    /// +788 / +78C: the range the block's spin per frame is drawn from.
    pub spin_min: f32,
    pub spin_max: f32,
    /// +790: frames frozen per percent of the freezing hit.
    pub frames_per_damage: f32,
    /// +794: what each frame takes off the timer.
    pub decrement: f32,
    /// +798: what each mash input takes off.
    pub mash_decrement: f32,
    /// +79C: what each percent of a later hit takes off.
    pub hit_decrement: f32,
    /// +7A0: the block size the model's scale is relative to.
    pub model_size: f32,
    /// +7A4: frames of DamageIceJump before Fall.
    pub jump_frames: f32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            damage_scale: r.f32(base + 0x714)?,
            gravity_scale: r.f32(base + 0x77C)?,
            shatter_speed: r.f32(base + 0x780)?,
            bounce_scale: r.f32(base + 0x784)?,
            spin_min: r.f32(base + 0x788)?,
            spin_max: r.f32(base + 0x78C)?,
            frames_per_damage: r.f32(base + 0x790)?,
            decrement: r.f32(base + 0x794)?,
            mash_decrement: r.f32(base + 0x798)?,
            hit_decrement: r.f32(base + 0x79C)?,
            model_size: r.f32(base + 0x7A0)?,
            jump_frames: r.f32(base + 0x7A4)?,
        })
    }
}

/// The surface the block last bounced off (mv.co.damageice.wall_hit_dir):
/// it does not bounce off the same one twice in a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bounce {
    RightWall = 1,
    LeftWall = 2,
    Ceiling = 3,
}

/// grab_timer and its mash latches with mv.co.damageice.
#[derive(Clone, Debug)]
pub struct FrozenState {
    pub timer: f32,
    stick_directions: [i8; 2],
    /// x0.
    pub last_bounce: Option<Bounce>,
    /// x4: XRotN's turn per airborne frame.
    pub spin: f32,
    /// x8: the block as the map sees it, relative to TopN.
    pub collision_box: FtCollisionBox,
    /// The entry's map pass (ftCo_800909D0) is still to run.
    unplaced: bool,
    /// PlCo +714: what hit detection multiplies a DamageIce victim's
    /// damage by (ftcoll.c:199, 576, 1155).
    pub damage_scale: f32,
}

/// ftCo_800886D8(fp, 0x122, 127, 64): freezing.
const FREEZE_SOUND: u32 = 0x122;
/// ft_PlaySFX(fp, 0x123, 127, 64): the block cracking or shattering.
const CRACK_SOUND: u32 = 0x123;
/// ftCommon_8007EBAC ids: freezing, and a bounce.
const FREEZE_RUMBLE: u16 = 1;
const BOUNCE_RUMBLE: u16 = 7;
/// InAirUpdate: the block cannot land before this frame. A frozen fighter
/// has no animation and its frame stays -1, so it never does.
const LANDING_FRAME: f32 = 3.0;

impl Fighter {
    /// ftCo_8008DCE0's tail (ftCo_Damage.c:530-538): the launch froze the
    /// victim unless it already was (the hit-while-frozen caller refreezes).
    pub(super) fn freezes(&self, element: HitElement, base_level: usize) -> bool {
        self.core.motion_state.id != S::DamageIce && base_level >= 2 && element == HitElement::Ice
    }

    /// ftCo_DamageIce_Init (80090B60).
    pub(super) fn enter_frozen(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        self.release_for_freeze();
        self.core.status.frozen = true;
        self.change_motion_state_with_flags(
            S::DamageIce.into(),
            assets,
            MotionEntryFlags::UNK06,
            0.0,
            1.0,
        )?;
        // retail 80090BD8: fmuls.
        let timer = self.core.combat.frame_damage * assets.frozen.frames_per_damage;
        self.sink_into_block(assets);
        self.freeze(timer, assets, rng);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: FREEZE_SOUND,
                volume: 127,
                pan: 64,
            });
        Ok(())
    }

    /// ftCo_DamageIce_HitWhileFrozen (80091030), after ftCo_8008DCE0 has
    /// launched the block in DamageIce: the same entry without the timer,
    /// the body's offset or the freezing sound. The first entry dropped the
    /// ice block's model with the motion; this one makes it again.
    pub(super) fn refreeze(
        &mut self,
        timer: f32,
        stick_directions: [i8; 2],
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        self.interrupt_actions(assets);
        self.release_for_freeze();
        self.core.status.frozen = true;
        self.change_motion_state_with_flags(
            S::DamageIce.into(),
            assets,
            MotionEntryFlags(MotionEntryFlags::KEEP_GFX.0 | MotionEntryFlags::UNK06.0),
            0.0,
            1.0,
        )?;
        self.freeze(timer, assets, rng);
        let MotionData::Frozen(frozen) = &mut self.core.state_data else {
            unreachable!()
        };
        frozen.stick_directions = stick_directions;
        Ok(())
    }

    /// ftCo_8009750C drops a heavy item; ftCo_800DD168 releases a grab pair.
    fn release_for_freeze(&mut self) {
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C: frozen with a heavy item");
        }
        assert!(
            self.core.combat.grab.is_none(),
            "ftCo_800DD168: frozen in a grab pair"
        );
    }

    /// ftCo_DamageIce_Init (80090BE0..80090E34): YRotN moves by the
    /// translation of T(0, +154, +150) * Ry^T * Rx^T, which is the offset
    /// itself: both transposed rotations carry no translation.
    fn sink_into_block(&mut self, assets: &FighterAssets) {
        let bone = usize::from(assets.parts.joint(FtPart::YRotN).expect("YRotN"));
        let joint = self.core.animation.parts[bone].joint;
        let ice = &self.core.attributes.ice;
        let (y, z) = (ice.unknown_154, ice.unknown_150);
        let mut translation = self.core.skeleton.translation(joint);
        // retail 80090D60 / 80090DDC: fadds.
        translation.y += y;
        translation.z += z;
        self.core.skeleton.set_translate(joint, &translation);
    }

    /// The part ftCo_DamageIce_Init and ftCo_DamageIce_HitWhileFrozen share
    /// after their motion change.
    fn freeze(&mut self, timer: f32, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        // ftCo_8009E140(fp, 0): every dynamic bone back to the animation.
        self.core.release_dynamics(assets);
        // ftCommon_8007F824.
        (self.character.table().knockback_enter)(self, assets);
        let core = &mut self.core;
        core.status.unconditional_top_exit = true;
        let physics = &mut core.physics;
        if physics.ground_or_air == GroundOrAir::Air {
            physics.self_velocity = physics.knockback_velocity;
            physics.knockback_velocity = Vec3::ZERO;
        } else {
            physics.ground_velocity = physics.ground_knockback_velocity;
            physics.ground_knockback_velocity = 0.0;
        }
        core.status.grab_exclusions = GrabExclusions::ALL;
        let p = &assets.frozen;
        // retail 80090ECC fsubs, 80090ED0 fmadds.
        let spin = fmadds(p.spin_max - p.spin_min, rng.randf(), p.spin_min);
        let collision_box = self.block_box(assets);
        let core = &mut self.core;
        core.state_data = MotionData::Frozen(FrozenState {
            timer,
            stick_directions: [0; 2],
            last_bounce: None,
            spin,
            collision_box,
            unplaced: true,
            damage_scale: p.damage_scale,
        });
        let bone = usize::from(assets.parts.joint(FtPart::XRotN).expect("XRotN"));
        core.effects.push(EffectRequest::IceBlock {
            bone,
            scale: core.block_model_scale(assets),
        });
        core.effect_state.destroy_on_state_change = true;
        core.set_hurt_capsules(HurtStatus::Intangible);
        let capsule = melee_coll::hurtbox::HurtCapsule {
            height: melee_coll::hurtbox::HurtHeight::Middle,
            grabbable: false,
            bone,
            offsets: [Vec3::ZERO; 2],
            radius: core.attributes.ice.damageice_ice_size,
            positions: [Vec3::ZERO; 2],
            cached: false,
        };
        core.replace_hurt_capsule(0, capsule);
        core.commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: FREEZE_RUMBLE,
                duration: 0,
            });
    }

    /// ftCo_800909D0 (800909D0): the block is a square of the fighter's ice
    /// size around XRotN, relative to TopN.
    fn block_box(&mut self, assets: &FighterAssets) -> FtCollisionBox {
        let core = &mut self.core;
        // retail 800909F4: fmuls.
        let radius = core.player.scale * core.attributes.ice.damageice_ice_size;
        let root = core.animation.root;
        let top = bone_position(&mut core.skeleton, root, 0, Vec3::ZERO);
        let bone = usize::from(assets.parts.joint(FtPart::XRotN).expect("XRotN"));
        let center = bone_position(&mut core.skeleton, root, bone, Vec3::ZERO);
        // lbVector_Sub.
        let offset = Vec3::new(center.x - top.x, center.y - top.y, center.z - top.z);
        FtCollisionBox {
            top: radius + offset.y,
            bottom: -radius + offset.y,
            right: Vec2::new(radius + offset.x, 0.0),
            left: Vec2::new(-radius + offset.x, 0.0),
        }
    }

    /// ftCo_800909D0's tail: the ECB unlocked (ftCommon_UnlockECB) and one
    /// pass with the new box, which never lands (ft_80082638) or leaves the
    /// floor (ft_80082888) by itself. The freeze happens inside
    /// Fighter_ProcessHit, whose port has no map: the proc runs this once
    /// the reaction returns, before anything else reads the position.
    pub(super) fn place_frozen_block(&mut self, map: &mut melee_mp::CollMap) {
        let MotionData::Frozen(frozen) = &mut self.core.state_data else {
            return;
        };
        if !std::mem::take(&mut frozen.unplaced) {
            return;
        }
        let collision_box = frozen.collision_box;
        let c = &mut self.core;
        c.collision.lock_frames = 0;
        c.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
        let root = c.animation.root;
        if c.physics.ground_or_air == GroundOrAir::Air {
            air::collide_stay_box(
                &mut c.physics,
                &mut c.collision,
                map,
                &mut c.skeleton,
                root,
                collision_box,
            );
        } else {
            ground::collide_box(
                &mut c.physics,
                &mut c.collision,
                map,
                &mut c.skeleton,
                root,
                collision_box,
                false,
            );
        }
    }

    /// ftCo_80091854 (80091854): the block breaks and the fighter hops out,
    /// steered by the stick.
    fn break_out(&mut self, assets: &FighterAssets) -> Result<()> {
        // ftCo_800C5240: holding the Hammer.
        let hammer = melee_types::ItemKind::Hammer;
        if self.core.held_item.as_ref().is_some_and(|held| held.kind == hammer) {
            unimplemented!("ftCo_DamageIce.c:471-472: thawing with a hammer (ftCo_800C5A98)");
        }
        let MotionData::Frozen(frozen) = &self.core.state_data else {
            panic!("frozen scratch missing")
        };
        let retained_word = frozen.spin;
        self.leave_ground();
        self.core.status.frozen = false;
        self.change_motion_state_with_flags(
            S::DamageIceJump.into(),
            assets,
            MotionEntryFlags::UNK06,
            0.0,
            0.0,
        )?;
        self.crack();
        let bone = usize::from(assets.parts.joint(FtPart::XRotN).expect("XRotN"));
        let core = &mut self.core;
        core.effects.push(EffectRequest::IceShatter {
            bone,
            scale: core.block_model_scale(assets),
        });
        let ice = &core.attributes.ice;
        // retail 80091954: fmuls.
        core.physics.self_velocity.x = core.input.current.stick.x * ice.damageicejump_vel_x_mult;
        core.physics.self_velocity.y = ice.damageicejump_vel_y;
        core.state_data = MotionData::FrozenJump {
            frames_left: assets.frozen.jump_frames,
            retained_word,
        };
        Ok(())
    }

    fn crack(&mut self) {
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: CRACK_SOUND,
                volume: 127,
                pan: 64,
            });
    }

    /// ftCo_DamageIce_Collide (80091620): at a wall or ceiling the block
    /// shatters into DamageFall when faster than PlCo +780, else bounces.
    fn hit_surface(
        &mut self,
        normal: Vec3,
        offset: Vec3,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        let p = self.core.physics.position;
        // retail 8009164C..8009166C: fadds.
        let contact = Vec3::new(p.x + offset.x, p.y + offset.y, p.z + offset.z);
        self.core.effects.push(EffectRequest::QueuedRebound {
            position: contact,
            angle: melee_lb::trigf::atan2f(-normal.x, normal.y),
        });
        self.core.quake_request = Some(melee_cm::QuakeKind::Small);
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: BOUNCE_RUMBLE,
                duration: 0,
            });
        self.crack();
        let v = self.core.physics.self_velocity;
        // retail 800916E8..80091700: three fmuls, x^2 + y^2, then z^2 + that.
        let squared = v.z * v.z + (v.x * v.x + v.y * v.y);
        if sqrtf(squared) > assets.frozen.shatter_speed {
            self.crack();
            self.core.physics.position = contact;
            self.core.status.frozen = false;
            return self.enter_damage_fall(assets);
        }
        let mirrored = melee_lb::vector::mirror(v, normal);
        let scale = assets.frozen.bounce_scale;
        // retail 800917C8 / 800917D4: fmuls.
        self.core.physics.self_velocity = Vec3::new(mirrored.x * scale, mirrored.y * scale, mirrored.z);
        // x68C_transNPos: a frozen fighter has no animation, so this is
        // TransN as the launch's first frame left it.
        let trans = self
            .core
            .animation
            .root_motion
            .as_ref()
            .map_or(Vec3::ZERO, |root| root.primary_history.position);
        let flags = self.core.collision.data.env_flags as u32;
        // Retail tests Collide_RightWallHug twice (ftCo_DamageIce.c:436-437),
        // so a left wall takes the ceiling's adjustment.
        if flags & env::RIGHT_WALL_HUG != 0 {
            // retail 800917F8 fneg, 80091800 fadds, 80091804 fnmsubs.
            self.core.physics.position.x =
                fnmsubs(trans.z, -self.core.physics.facing, p.x + offset.x);
        } else {
            // retail 8009181C / 80091820: fadds.
            self.core.physics.position.y = trans.y + (p.y + offset.y);
        }
        let c = &mut self.core;
        if air::collide_air_dodge(
            &mut c.physics,
            &mut c.collision,
            map,
            &mut c.skeleton,
            c.animation.root,
        ) {
            c.land();
        }
        Ok(())
    }
}

impl super::FighterCore {
    /// The ice block model's scale: x34_scale.y * co_attrs ice size over
    /// PlCo +7A0 (fmuls, fdivs).
    fn block_model_scale(&self, assets: &FighterAssets) -> f32 {
        self.player.scale * self.attributes.ice.damageice_ice_size / assets.frozen.model_size
    }

    /// ftCo_DamageIce_OnHit2 (80091274), take_dmg_2_cb while frozen: the
    /// frame's damage shortens the timer and fire ends it.
    pub(super) fn frozen_hit_taken(&mut self, element: HitElement, assets: &FighterAssets) {
        let damage = self.combat.frame_damage;
        let MotionData::Frozen(frozen) = &mut self.state_data else {
            return;
        };
        // retail 80091288: fnmsubs.
        frozen.timer = fnmsubs(damage, assets.frozen.hit_decrement, frozen.timer);
        if element == HitElement::Fire {
            frozen.timer = 0.0;
        }
    }

    pub(super) fn frozen_block_unplaced(&self) -> bool {
        matches!(&self.state_data, MotionData::Frozen(frozen) if frozen.unplaced)
    }

    /// ftCo_8009E140(fp, false) (8009E140): the dynamic bones back to the
    /// animation (ftCo_8009CB40 with no solver).
    pub(super) fn release_dynamics(&mut self, assets: &FighterAssets) {
        let first_only =
            super::assets::CommonBehavior::for_kind(assets.kind).frozen_first_dynamics_only;
        let sets = if first_only { 1 } else { self.dynamics.len() };
        for index in 0..sets.min(self.dynamics.len()) {
            self.dynamics_first_bone[index] = 0x100;
            crate::dynamics::select(
                &mut self.dynamics[index],
                &mut self.skeleton,
                &mut self.animation.parts,
                false,
                0,
            );
        }
    }

    /// The timer and mash latches a hit while frozen carries across its two
    /// motion changes (grab_timer, x1A50/x1A51 are not motion scratch).
    pub(super) fn frozen_timer(&self) -> Option<(f32, [i8; 2])> {
        match &self.state_data {
            MotionData::Frozen(frozen) => Some((frozen.timer, frozen.stick_directions)),
            _ => None,
        }
    }
}

/// ftCo_DamageIce_Anim (800912A8).
pub fn frozen_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let threshold = p.assets.grab_escape.stick_threshold;
    let parameters = &p.assets.frozen;
    let input = f.core.input.clone();
    let airborne = f.core.physics.ground_or_air == GroundOrAir::Air;
    let MotionData::Frozen(frozen) = &mut f.core.state_data else {
        panic!("frozen scratch missing")
    };
    if airborne {
        // HSD_JObjAddRotationX on the part's own joint.
        let bone = usize::from(p.assets.parts.joint(FtPart::XRotN).expect("XRotN"));
        let joint = f.core.animation.parts[bone].joint;
        let rotation = f.core.skeleton.rotation_x(joint) + frozen.spin;
        f.core.skeleton.set_rotation_x(joint, rotation);
    }
    frozen.timer -= parameters.decrement;
    super::capture_yoshi::grab_mash(
        &mut frozen.timer,
        &mut frozen.stick_directions,
        &input,
        threshold,
        parameters.mash_decrement,
    );
    if frozen.timer <= 0.0 {
        f.break_out(p.assets)?;
    }
    Ok(None)
}

/// ftCo_DamageIce_IASA, ftCo_DamageIceJump_IASA: empty.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCo_DamageIce_Phys (8009138C): in the air, friction and PlCo +77C of
/// the fighter's gravity; on the ground, ft_80084F3C.
pub fn frozen_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.core.physics.ground_or_air == GroundOrAir::Ground {
        return super::state::callbacks::physics::guard_on(f, p);
    }
    use crate::physics::airborne;
    let core = &mut f.core;
    let air = &core.attributes.air;
    // ftCommon_8007CEF4, then ftCommon_Fall with a scaled gravity (fmuls).
    core.physics.animation_velocity.x =
        airborne::drift_acceleration(core.physics.self_velocity.x, 0.0, 0.0, air);
    core.physics.self_velocity.y = airborne::gravity(
        core.physics.self_velocity.y,
        air.gravity * p.assets.frozen.gravity_scale,
        air.terminal_velocity,
    );
    core.finish_air_update(p.assets, p.wind);
}

/// ftCo_DamageIce_Coll (80091410).
pub fn frozen_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("frozen collision assets");
    let MotionData::Frozen(frozen) = &f.core.state_data else {
        panic!("frozen scratch missing")
    };
    let (collision_box, last_bounce) = (frozen.collision_box, frozen.last_bounce);
    let c = &mut f.core;
    let root = c.animation.root;
    air::begin_map(&c.physics, &mut c.collision, &mut c.skeleton, root);
    if c.physics.ground_or_air == GroundOrAir::Ground {
        if !ground::collide_box(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            root,
            collision_box,
            false,
        ) {
            f.leave_ground();
        }
        return Ok(());
    }
    // ftCo_DamageIce_InAirUpdate (80091478).
    let landed = if c.animation.frame <= LANDING_FRAME {
        air::collide_stay_box(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            root,
            collision_box,
        )
    } else {
        air::collide_box(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            root,
            collision_box,
        )
    };
    let cd = &c.collision.data;
    let flags = cd.env_flags as u32;
    // ftKb_SpecialN_800F1F1C at each contact is Kirby's alone.
    let (bounce, normal, offset) = if flags & env::RIGHT_WALL_HUG != 0
        && last_bounce != Some(Bounce::RightWall)
    {
        (
            Bounce::RightWall,
            cd.right_facing_wall.normal,
            Vec3::new(cd.ecb.left.x, cd.ecb.left.y, 0.0),
        )
    } else if flags & env::LEFT_WALL_HUG != 0 && last_bounce != Some(Bounce::LeftWall) {
        (
            Bounce::LeftWall,
            cd.left_facing_wall.normal,
            Vec3::new(cd.ecb.right.x, cd.ecb.right.y, 0.0),
        )
    } else if flags & env::CEILING_HUG != 0 && last_bounce != Some(Bounce::Ceiling) {
        (
            Bounce::Ceiling,
            cd.ceiling.normal,
            Vec3::new(0.0, cd.ecb.top.y, 0.0),
        )
    } else {
        if landed {
            f.core.land();
        }
        return Ok(());
    };
    f.hit_surface(normal, offset, assets, p.map)?;
    // The shatter left the state; a bounce records its surface.
    if let MotionData::Frozen(frozen) = &mut f.core.state_data {
        frozen.last_bounce = Some(bounce);
    }
    Ok(())
}

/// ftCo_DamageIceJump_Anim (800919A4): Fall when the countdown ends.
pub fn frozen_jump_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let MotionData::FrozenJump { frames_left, .. } = &mut f.core.state_data else {
        panic!("frozen jump scratch missing")
    };
    if *frames_left > 0.0 {
        *frames_left -= 1.0;
        if *frames_left <= 0.0 {
            // ftCo_Fall_Enter.
            f.change_motion_state(S::Fall.into(), p.assets)?;
        }
    }
    Ok(None)
}
