//! The Warp Star ride, fighter side: ftCo_WarpStar.c (800C4724..800C4E2C).
//!
//! LightGet's end (or an air catch) with a Warp Star in hand takes over
//! (ftpickupitem_8009447C -> ftCo_800C4724): the rider loses its facing,
//! goes intangible and hidden from the display pass, and its root joint
//! plays one of the star's seven flight paths (it_80294364), whose
//! translation each frame is the rider's velocity. Above the blast zone's
//! top, or after 120 frames, it falls back from the top (WarpStarFall)
//! with the star's collision box and stick drift; landing leaves it in
//! JumpB while the star bursts (it_802947CC). A hit or a KO on the way
//! drops the star (ftCo_800C4858 / ftCo_800C48AC).
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter, MotionData,
};
use crate::anim::WaitChoice;
use hsd_anim::aobj::AObjEndCallback;
use hsd_types::Vec3;
use melee_types::{mp::FtCollisionBox, CommonMotionState as S};

/// PlCo +438 and +694..+6A8.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +438: what the landing first scales the vertical speed by (the jump's
    /// own speed replaces it).
    pub landing_vertical_scale: f32,
    /// +694 / +698: the fall's gravity and terminal speed.
    pub fall_gravity: f32,
    pub fall_terminal_speed: f32,
    /// +69C / +6A0 / +6A4: the drift's stick factor, base and top speed.
    pub drift_scaling: f32,
    pub drift_flat: f32,
    pub drift_max: f32,
    /// +6A8: the drift's friction.
    pub drift_friction: f32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            landing_vertical_scale: r.f32(base + 0x438)?,
            fall_gravity: r.f32(base + 0x694)?,
            fall_terminal_speed: r.f32(base + 0x698)?,
            drift_scaling: r.f32(base + 0x69C)?,
            drift_flat: r.f32(base + 0x6A0)?,
            drift_max: r.f32(base + 0x6A4)?,
            drift_friction: r.f32(base + 0x6A8)?,
        })
    }
}

/// mv.co.warpstar.
#[derive(Clone, Debug)]
pub struct WarpStarState {
    /// The facing the ride took away (restored by the landing or a hit).
    pub facing: f32,
    /// x1C: frames of flight left.
    pub timer: i32,
    /// cur_pos at the pickup: the fall starts above its x.
    pub start: Vec3,
    /// self_vel: the root's translation last frame.
    pub previous_translation: Vec3,
    /// ecb: the falling rider's box (it_802944AC).
    pub collision_box: FtCollisionBox,
}

/// ftCo_800C4724's flight frames (x1C).
const FLIGHT_FRAMES: i32 = 120;
/// ftColl_8007B6A0's colour animation: the invincibility flash.
const INTANGIBLE_FLASH: u8 = 9;

impl Fighter {
    /// ftpickupitem_8009447C's It_Kind_WStar case: whether the item just
    /// taken is a Warp Star, whose ride then begins.
    pub(super) fn ride_held_warp_star(&mut self, assets: &FighterAssets) -> Result<bool> {
        let Some(held) = self.core.held_item else {
            return Ok(false);
        };
        if held.kind != melee_types::ItemKind::WStar {
            return Ok(false);
        }
        self.enter_warp_star_jump(held.item, assets)?;
        Ok(true)
    }

    /// ftCo_800C4724 (800C4724): it_80294430 sizes the star to the rider
    /// (x2C4.y, its push half width scaled, and warp_star_hitbox_scale);
    /// the facing is kept and zeroed, the flight timer starts, the rider is
    /// airborne with no movement, in WarpStarJump, intangible
    /// (ftColl_8007B6A0), blast-zone exempt (x2219_b2) and hidden (x221E_b2);
    /// the star then draws the path its root plays (the scene's
    /// ItemRequest::WarpStarPath, `begin_warp_star_flight`).
    fn enter_warp_star_jump(&mut self, item: u32, assets: &FighterAssets) -> Result<()> {
        let half_width = assets.overlap.half_width * self.core.player.scale;
        self.core.item_requests.push(melee_it::ItemRequest::ControlInstance {
            item,
            control: melee_it::ItemControl::WarpStarRide {
                half_width,
                hitbox_scale: self.core.attributes.size.warp_star_hitbox_scale,
            },
        });
        let facing = self.core.physics.facing;
        self.core.physics.facing = 0.0;
        let start = self.core.physics.position;
        self.core.leave_ground();
        self.change_motion_state(S::WarpStarJump.into(), assets)?;
        self.become_item_intangible(assets);
        self.core.state_data = MotionData::WarpStar(WarpStarState {
            facing,
            timer: FLIGHT_FRAMES,
            start,
            previous_translation: Vec3::ZERO,
            collision_box: FtCollisionBox::default(),
        });
        self.core.status.blast_zone_exempt = true;
        self.core.clear_movement();
        self.core
            .item_requests
            .push(melee_it::ItemRequest::WarpStarPath { item });
        Ok(())
    }

    /// ftCo_800C4724's tail, once the star drew its path: HSD_JObjAddAnim on
    /// the root, ftAnim_80070734 (frame 0) and HSD_JObjAnim. `collision_box`
    /// is it_802944AC's box for the fall.
    pub fn begin_warp_star_flight(
        &mut self,
        path: &hsd_anim::jobj::AnimJoint,
        prepared: &hsd_anim::aobj::AObj,
        collision_box: FtCollisionBox,
    ) {
        let MotionData::WarpStar(ride) = &mut self.core.state_data else {
            panic!("ftCo_800C4724: a Warp Star path without the ride")
        };
        ride.collision_box = collision_box;
        let root = self.core.animation.root;
        let tree = &mut self.core.skeleton;
        tree.add_prepared_joint_anim(root, path, prepared);
        tree.req_anim_by_flags(root, 1, 0.0);
        tree.anim::<super::RetailTrig>(root, &mut AObjEndCallback::default());
    }

    /// ftColl_8007B6A0 (8007B6A0): x2221_b0, the timed intangibility gone
    /// (x1990 0, x198C 2) and the flash.
    fn become_item_intangible(&mut self, assets: &FighterAssets) {
        self.core.status.item_intangible = true;
        self.core.status.ledge_intangibility = 0;
        self.core.install_color_overlay_now(INTANGIBLE_FLASH, assets);
    }

    /// ftColl_8007B6EC (8007B6EC): x2221_b0 off; with no timed
    /// intangibility left the flash goes (ftCo_800C0200) if it holds the slot.
    fn end_item_intangible(&mut self, assets: &FighterAssets) {
        self.core.status.item_intangible = false;
        if self.core.status.ledge_intangibility == 0
            && self.core.combat.color_overlay.id == INTANGIBLE_FLASH
        {
            self.core.release_charge_color(INTANGIBLE_FLASH, assets);
        }
    }

    /// ftCo_800C4858 / ftCo_800C48AC (the ride's take_dmg_cb and
    /// death2_cb): hurt again, the facing back, and the star let go with
    /// no push (Item_8026ABD8(item, &zero, 1)).
    pub(super) fn drop_warp_star(&mut self, _assets: &FighterAssets) {
        // The rider is intangible (x2221_b0) through the whole ride, so no
        // hit reaches it.
        unimplemented!("ftCo_800C4858 (ftCo_WarpStar.c:79-91): a Warp Star rider hit")
    }

    /// ftCo_800C48AC (800C48AC), the ride's death2_cb (inlineA0):
    /// ftColl_8007B6EC, the facing back, and the star let go with no push
    /// (Item_8026ABD8(item, &zero, 1)).
    pub(super) fn release_warp_star(&mut self, assets: &FighterAssets) {
        let MotionData::WarpStar(ride) = &self.core.state_data else {
            return;
        };
        let facing = ride.facing;
        self.end_item_intangible(assets);
        self.core.physics.facing = facing;
        let held = self.core.held_item.expect("the ridden Warp Star is held");
        self.core.drop_held_item(held, 1.0, assets);
    }

    /// ftCo_800C4A38 (800C4A38): WarpStarFall from the blast zone's top
    /// above where the ride began, the root turned (pi/2, 0, 0), the star
    /// in its fall (it_802946B0).
    fn enter_warp_star_fall(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::WarpStar(ride) = self.core.state_data.clone() else {
            panic!("WarpStarFall without the ride")
        };
        self.change_motion_state(S::WarpStarFall.into(), assets)?;
        self.core.state_data = MotionData::WarpStar(ride.clone());
        self.core.clear_movement();
        self.core.physics.position = Vec3::new(ride.start.x, assets.stage_blast_top, 0.0);
        let root = self.core.animation.root;
        // HSD_JObjSetRotation(jobj, &{pi/2, 0, 0, 0}).
        self.core.skeleton.set_rotation(
            root,
            &hsd_anim::quat::Quaternion {
                x: std::f32::consts::FRAC_PI_2,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            },
        );
        let held = self.core.held_item.expect("the ridden Warp Star is held");
        self.core
            .item_requests
            .push(melee_it::ItemRequest::ControlInstance {
                item: held.item,
                control: melee_it::ItemControl::WarpStarFall,
            });
        melee_mp::set_position(&mut self.core.collision.data, &self.core.physics.position);
        Ok(())
    }

    /// ftCo_800C4C60 (800C4C60): the landing. The facing back, JumpB with
    /// a full jump's speed from the drift (retail 800C4CBC..800C4D68: fmuls
    /// only), hurt again, the star's burst (it_802947CC), effect 1030 at
    /// the box's bottom turned to the floor, a large quake and rumble.
    fn land_warp_star(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::WarpStar(ride) = self.core.state_data.clone() else {
            panic!("WarpStarFall landing without the ride")
        };
        self.core.physics.facing = ride.facing;
        self.change_motion_state(S::JumpB.into(), assets)?;
        self.end_item_intangible(assets);
        let multiplier = 1.0;
        self.core.state_data = MotionData::Jump(super::jump::JumpState {
            short_hop: false,
            physics_started: 0,
            multiplier,
        });
        let attrs = &self.core.attributes.jumping;
        let velocity = &mut self.core.physics.self_velocity;
        velocity.x *= attrs.ground_to_air_jump_momentum_multiplier * multiplier;
        velocity.y *= assets.warp_star.landing_vertical_scale;
        velocity.z = 0.0;
        let initial = attrs.jump_h_initial_velocity
            * (-self.core.physics.facing * assets.jumping.backward_threshold);
        velocity.y = attrs.jump_v_initial_velocity * multiplier;
        let horizontal = velocity.x + multiplier * initial;
        let maximum = attrs.jump_h_max_velocity * multiplier;
        let magnitude = if horizontal < 0.0 { -horizontal } else { horizontal };
        velocity.x = if magnitude > maximum {
            if horizontal < 0.0 {
                -maximum
            } else {
                maximum
            }
        } else {
            horizontal
        };
        let position = self.core.physics.position;
        let held = self.core.held_item.expect("the ridden Warp Star is held");
        let part = self.core.held_item_part(&held);
        let mut holder = self.core.item_holder(part, assets);
        let part_position = holder.part_position();
        let hand = holder.part_matrix();
        let (center, attack) = (holder.center, holder.attack);
        self.core
            .item_requests
            .push(melee_it::ItemRequest::WarpStarBlast {
                item: held.item,
                position,
                hand,
                part_position,
                center,
                attack,
            });
        self.core.release_held_item(held.item, assets);
        // vec = cur_pos + (0, ecb.bottom.y, 0): three fadds.
        let bottom = self.core.collision.data.ecb.bottom.y;
        let at = Vec3::new(0.0 + position.x, bottom + position.y, 0.0 + position.z);
        let normal = self.core.collision.data.floor.normal;
        let angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::CameraQuake {
                kind: melee_cm::QuakeKind::Large as u16,
                position: at,
            });
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::SurfaceRebound { position: at, angle });
        // ftCommon_8007EBAC(fp, 14, 0) is rumble.
        Ok(())
    }
}

fn ride(fighter: &mut Fighter) -> &mut WarpStarState {
    match &mut fighter.core.state_data {
        MotionData::WarpStar(ride) => ride,
        _ => panic!("Warp Star ride scratch missing"),
    }
}

/// ftCo_WarpStarJump_Anim (800C48F8): the flight ends above the blast
/// zone's top or when its timer runs out.
pub fn jump_anim(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    jump_anim_callback(fighter, phase)
}

/// ftCo_WarpStarJump_Anim alone, without ftAnim_8006EBA4's step before it
/// (Fighter_8006A360 already stepped the state a catch rode out of).
pub(super) fn jump_anim_callback(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    let y = fighter.core.physics.position.y;
    let ride = ride(fighter);
    ride.timer -= 1;
    if y > phase.assets.stage_blast_top || ride.timer == 0 {
        fighter.enter_warp_star_fall(phase.assets)?;
    }
    Ok(None)
}

/// ftCo_WarpStarJump_IASA / ftCo_WarpStarFall_IASA: empty;
/// ftCo_WarpStarFall_Anim: empty after the step.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}
pub fn fall_anim(fighter: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    Ok(None)
}

/// ftCo_WarpStarJump_Phys (800C4950): the root's translation's change is
/// the velocity (three fsubs); Fighter_procUpdate's airborne tail moves it.
pub fn jump_physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let translation = fighter
        .core
        .skeleton
        .translation(fighter.core.animation.root);
    let previous = ride(fighter).previous_translation;
    fighter.core.physics.self_velocity = Vec3::new(
        translation.x - previous.x,
        translation.y - previous.y,
        translation.z - previous.z,
    );
    ride(fighter).previous_translation = translation;
    fighter.core.free_flight_physics(phase.assets, phase.wind);
}

/// ftCo_WarpStarJump_Coll: empty; Fighter_procMap's own ECB unlock.
pub fn jump_collision(fighter: &mut Fighter, _: CollisionPhase<'_>) -> Result<()> {
    let core = &mut fighter.core;
    crate::collision::air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    Ok(())
}

/// ftCo_WarpStarFall_Phys (800C4BAC): ftCommon_Fall with PlCo +694/+698,
/// then the stick's drift (retail 800C4BE8..800C4C10: fmuls, fadds, fmuls)
/// through ftCommon_8007D174 with PlCo +6A8 as its friction.
pub fn fall_physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let p = &phase.assets.warp_star;
    let physics = &mut fighter.core.physics;
    physics.self_velocity.y =
        crate::physics::airborne::gravity(physics.self_velocity.y, p.fall_gravity, p.fall_terminal_speed);
    let stick = fighter.core.input.current.stick.x;
    let scaled = stick * p.drift_scaling;
    let flat = if stick > 0.0 { p.drift_flat } else { -p.drift_flat };
    let acceleration = scaled + flat;
    let target = stick * p.drift_max;
    let mut air = fighter.core.attributes.air.clone();
    air.aerial_friction = p.drift_friction;
    let physics = &mut fighter.core.physics;
    physics.animation_velocity.x = crate::physics::airborne::drift_acceleration(
        physics.self_velocity.x,
        acceleration,
        target,
        &air,
    );
    fighter.core.free_flight_physics(phase.assets, phase.wind);
}

/// ftCo_WarpStarFall_Coll (800C4C30): ft_80083E64 with the star's box;
/// touching down lands (ftCo_800C4C60).
pub fn fall_collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let CollisionPhase { assets, map } = phase;
    let assets = assets.expect("Warp Star fall collision needs proc_map_with_assets");
    let core = &mut fighter.core;
    crate::collision::air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    let collision_box = ride(fighter).collision_box;
    let core = &mut fighter.core;
    let landed = crate::collision::air::collide_entry(
        &mut core.physics,
        &mut core.collision,
        map,
        collision_box,
    );
    if landed {
        fighter.land_warp_star(assets)?;
    }
    Ok(())
}
