//! ftWallJump_8008169C and ftCo_PassiveWall: the wall jump and the wall tech
//! (PassiveWall / PassiveWallJump) share ftCo_800C1E64 and these states.
use super::state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase};
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::collision::{air, ecb::EcbPose};
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::collide, CommonMotionState as S};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub contact_window: f32,
    pub stick_threshold: f32,
    pub tilt_window: f32,
    pub freeze_frames: i32,
    /// PlCo +760: the wall tech's freeze before its push-off.
    pub tech_freeze_frames: i32,
    pub intangible_frames: i32,
    pub vertical_decay: f32,
    pub jump_buffer_window: f32,
}
impl Parameters {
    pub fn read(a: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            contact_window: r.f32(base + 0x768)?,
            stick_threshold: r.f32(base + 0x76C)?,
            tilt_window: r.f32(base + 0x770)?,
            freeze_frames: r.s32(base + 0x774)?,
            tech_freeze_frames: r.s32(base + 0x760)?,
            intangible_frames: r.s32(base + 0x764)?,
            vertical_decay: r.f32(base + 0x778)?,
            jump_buffer_window: r.f32(base + 0x250)?,
        })
    }
}
/// Persistent Fighter +1969/+210C/+2110, not motion scratch.
#[derive(Clone, Copy, Debug)]
pub struct WallJump {
    pub used: u8,
    pub contact_age: u8,
    pub side: f32,
}
impl Default for WallJump {
    fn default() -> Self {
        Self {
            used: 0,
            contact_age: 254,
            side: 0.0,
        }
    }
}
/// mv.co.passivewall +2340/+2344/+2348/+234C.
#[derive(Clone, Debug)]
pub struct State {
    pub freeze_frames: i32,
    pub retained_zero: i32,
    pub jump_buffered: bool,
    pub vertical_exponent: i32,
}
impl Fighter {
    /// Right-wall hug wins when both masks are set. Contact age increments
    /// only when this predicate is called, not in the global input scheduler.
    pub fn try_wall_jump(&mut self, assets: &FighterAssets, map: &mut CollMap) -> Result<bool> {
        if !self.core.capabilities.can_walljump {
            return Ok(false);
        }
        let cd = &self.core.collision.data;
        let flags = cd.env_flags as u32;
        if flags & (collide::RIGHT_WALL_HUG | collide::LEFT_WALL_HUG) == 0 {
            self.core.status.wall_jump.contact_age = 254;
            return Ok(false);
        }
        let right = flags & collide::RIGHT_WALL_HUG != 0;
        let side = if right { -1.0 } else { 1.0 };
        let previous = self.core.status.wall_jump;
        if previous.contact_age < 254 && previous.side == side {
            self.core.status.wall_jump.contact_age += 1;
        } else {
            let (ecb, line) = if right {
                (cd.ecb.left, cd.right_facing_wall.index)
            } else {
                (cd.ecb.right, cd.left_facing_wall.index)
            };
            let p = self.core.physics.position;
            let contact = Vec3::new(ecb.x + p.x, ecb.y + p.y, 0.0 + p.z);
            let wall_velocity = map.line_speed(line, &contact).map_or(0.0, |v| v.x);
            let relative = self.core.physics.position_delta.x - wall_velocity;
            let speed = if relative < 0.0 { -relative } else { relative };
            if speed > self.core.attributes.wall.wall_jump_min_approach_speed {
                self.core.status.wall_jump.side = side;
                self.core.status.wall_jump.contact_age = 0;
            }
        }
        let wall = self.core.status.wall_jump;
        let params = &assets.wall_jump;
        let stick = self.core.input.current.stick.x;
        if f32::from(wall.contact_age) < params.contact_window
            && ((wall.side == -1.0 && stick >= params.stick_threshold)
                || (wall.side == 1.0 && stick <= -params.stick_threshold))
            && f32::from(self.core.input.horizontal.tilt) < params.tilt_window
        {
            let freeze = assets.wall_jump.freeze_frames;
            let exponent = i32::from(wall.used);
            self.enter_wall_contact(assets, map, S::PassiveWallJump, freeze, wall.side, exponent)?;
            self.core.status.wall_jump.contact_age = 254;
            self.core.status.wall_jump.used = self.core.status.wall_jump.used.saturating_add(1);
            return Ok(true);
        }
        Ok(false)
    }

    /// ftCo_800C1D38 (800C1D38): a tumbling fighter against a wall inside
    /// the tech window techs it; a buffered jump (ftCo_800C1E0C) makes it the
    /// wall-tech jump. Returns whether it teched.
    pub(super) fn try_wall_tech(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<bool> {
        let flags = self.core.collision.data.env_flags as u32;
        if flags & (collide::RIGHT_WALL_HUG | collide::LEFT_WALL_HUG) == 0
            || !self.core.tech_window_open(assets)
        {
            return Ok(false);
        }
        let state = if self.core.wall_jump_buffered(assets) {
            S::PassiveWallJump
        } else {
            S::PassiveWall
        };
        let side = if flags & collide::RIGHT_WALL_HUG != 0 {
            -1.0
        } else {
            1.0
        };
        let freeze = assets.wall_jump.tech_freeze_frames;
        self.enter_wall_contact(assets, map, state, freeze, side, 0)?;
        Ok(true)
    }

    /// ftCo_800C1E64 (800C1E64): push off a wall, facing away from it, frozen
    /// for `freeze` frames; shared by the wall jump and the wall tech.
    fn enter_wall_contact(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        state: S,
        freeze: i32,
        side: f32,
        vertical_exponent: i32,
    ) -> Result<()> {
        self.core.commands.variables[0] = 1;
        self.core.physics.facing = -side;
        self.core.clear_movement();
        let cd = &self.core.collision.data;
        let offset = if cd.env_flags as u32 & collide::RIGHT_WALL_HUG != 0 {
            cd.ecb.left
        } else {
            cd.ecb.right
        };
        let timer = freeze;
        self.change_motion_state_with_rate(
            state.into(),
            assets,
            0.0,
            if timer != 0 { 0.0 } else { 1.0 },
        )?;
        self.core.input.horizontal.tilt = 254;
        self.core.input.vertical.tilt = 254;
        self.core.state_data = MotionData::WallJump(State {
            freeze_frames: timer,
            retained_zero: 0,
            jump_buffered: false,
            vertical_exponent,
        });
        let position = Vec3::new(
            self.core.physics.position.x + offset.x,
            self.core.physics.position.y + offset.y,
            self.core.physics.position.z,
        );
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::WallJump { position });
        let trans_z = self
            .core
            .animation
            .root_motion
            .as_ref()
            .expect("walljump TransN")
            .primary_history
            .position
            .z;
        // 800C1F9C fnmsubs f0,f1,f0,f31; f0 is the negated NEW facing.
        self.core.physics.position.x =
            gekko_math::fma::fnmsubs(trans_z, -self.core.physics.facing, position.x);
        self.wall_contact_map(assets, map);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Action,
                id: assets.wall_jump_sound,
                volume: 127,
                pan: 64,
            });
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 12,
                duration: 0,
            });
        if timer == 0 {
            self.core.shield_sound(3);
        }
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 120,
                duration: 0,
            });
        self.core.status.ledge_intangibility = self
            .core
            .status
            .ledge_intangibility
            .max(assets.wall_jump.intangible_frames);
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
        Ok(())
    }

    /// ft_80081F2C (80081F2C): the wall-contact air pass; returns whether the
    /// fighter landed. Entries after begin_map do not run it again: that
    /// would tick the ECB lock twice in the same Fighter_procMap visit.
    /// ft_80081A00's item landing is not in scope.
    pub(super) fn wall_contact_map(&mut self, assets: &FighterAssets, map: &mut CollMap) -> bool {
        let core = &mut self.core;
        core.skeleton
            .set_translate(core.animation.root, &core.physics.position);
        let cd = &mut core.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = core.physics.position;
        let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
        let position = |i| pose.position(i);
        let landed = if core.shield.allow_sdi {
            map.air_collide_stay_ecb10(cd, Some(&position))
        } else if core.status.ledge_cooldown != 0 {
            map.air_collide_ecb10(cd, Some(&position))
        } else {
            let old_height = cd.ledge_snap_height;
            cd.ledge_snap_height = old_height * assets.damage.ledge_height_scale;
            let landed = map.air_collide_ledge_ecb10(cd, Some(&position));
            cd.ledge_snap_height = old_height;
            landed
        };
        core.physics.position = cd.cur_pos;
        core.skeleton
            .set_translate(core.animation.root, &core.physics.position);
        landed
    }
}

impl Fighter {
    /// ft_80082084 (80082084): ft_80081F2C's pass with the ECB loaded at 0x12
    /// (mpColl_80048388 / 80048768 / 80048578); returns whether the fighter
    /// landed.
    pub(super) fn ceiling_contact_map(&mut self, assets: &FighterAssets, map: &mut CollMap) -> bool {
        let core = &mut self.core;
        core.skeleton
            .set_translate(core.animation.root, &core.physics.position);
        let cd = &mut core.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = core.physics.position;
        let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
        let position = |i| pose.position(i);
        let landed = if core.shield.allow_sdi {
            map.air_collide_stay_ecb18(cd, Some(&position))
        } else if core.status.ledge_cooldown != 0 {
            map.air_collide_ecb18(cd, Some(&position))
        } else {
            let old_height = cd.ledge_snap_height;
            cd.ledge_snap_height = old_height * assets.damage.ledge_height_scale;
            let landed = map.air_collide_ledge_ecb18(cd, Some(&position));
            cd.ledge_snap_height = old_height;
            landed
        };
        core.physics.position = cd.cur_pos;
        core.skeleton
            .set_translate(core.animation.root, &core.physics.position);
        landed
    }
}

impl super::FighterCore {
    /// ftCo_800C1E0C (800C1E0C): a jump pressed within PlCo +250 frames or a
    /// tap-jump stick.
    fn wall_jump_buffered(&self, assets: &FighterAssets) -> bool {
        f32::from(self.input.buttons.jump_button) < assets.wall_jump.jump_buffer_window
            || self.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
    }
}

pub fn animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    let assets = phase.assets;
    fighter.step_animation(assets);
    fighter.advance_smash_charge(assets);
    let MotionData::WallJump(state) = &mut fighter.core.state_data else {
        panic!("walljump scratch missing")
    };
    if state.freeze_frames != 0 {
        state.freeze_frames -= 1;
        if state.freeze_frames == 0 {
            let buffered = state.jump_buffered;
            let exponent = state.vertical_exponent;
            fighter.core.shield_sound(8);
            if buffered {
                let frame = fighter.core.animation.frame;
                fighter.change_motion_state_with_rate(
                    S::PassiveWallJump.into(),
                    assets,
                    frame,
                    1.0,
                )?;
                fighter.core.input.vertical.tilt = 254;
                let MotionData::WallJump(state) = &mut fighter.core.state_data else {
                    panic!("walljump scratch missing")
                };
                state.freeze_frames = 0;
            } else {
                fighter
                    .core
                    .animation
                    .set_rate(&mut fighter.core.skeleton, 1.0, false);
            }
            let wall = &fighter.core.attributes.wall;
            if fighter.core.motion_state.id == S::PassiveWall {
                fighter.core.physics.self_velocity.x =
                    fighter.core.physics.facing * wall.passivewall_vel_x;
            } else {
                fighter.core.physics.self_velocity.x =
                    fighter.core.physics.facing * wall.wall_jump_horizontal_velocity;
                fighter.core.physics.self_velocity.y = wall.wall_jump_vertical_velocity;
                if fighter.core.status.wall_jump.used != 0 {
                    fighter.core.physics.self_velocity.y *=
                        melee_lb::trigf::powf(assets.wall_jump.vertical_decay, exponent as f32);
                }
            }
        }
    }
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(S::Fall.into(), assets)?;
    }
    Ok(None)
}

pub fn input(fighter: &mut Fighter, phase: InputPhase<'_>) {
    let assets = phase.assets;
    let MotionData::WallJump(state) = &mut fighter.core.state_data else {
        panic!("walljump scratch missing")
    };
    if state.freeze_frames != 0 {
        if fighter.core.wall_jump_buffered(assets) {
            let MotionData::WallJump(state) = &mut fighter.core.state_data else {
                unreachable!()
            };
            state.jump_buffered = true;
        }
        return;
    }
    // Fox/Marth have no float, tether, parasol or held throwable-item owner.
    // The ordinary aerial dispatcher keeps Special -> Dodge -> Attack -> Jump.
    super::state::callbacks::input::aerial(fighter, phase);
}

pub fn physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let MotionData::WallJump(state) = &fighter.core.state_data else {
        panic!("walljump scratch missing")
    };
    if state.freeze_frames == 0 {
        fighter.core.apply_fall_gravity(phase.assets);
        fighter.core.physics.animation_velocity.x = crate::physics::airborne::drift_acceleration(
            fighter.core.physics.self_velocity.x,
            0.0,
            0.0,
            &fighter.core.attributes.air,
        );
    }
    fighter.decay_air_knockback(phase.assets);
    crate::physics::integrate::integrate_velocity(&mut fighter.core.physics);
    crate::physics::integrate::integrate_environment(&mut fighter.core.physics, None, phase.wind);
}

pub fn collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let assets = phase.assets.expect("walljump map needs assets");
    let stick_y = fighter.core.input.current.stick.y;
    let drop_threshold = assets.input.platform_drop_threshold;
    let mut accept_floor = air::platform_floor_filter(stick_y, drop_threshold);
    let MotionData::WallJump(state) = &fighter.core.state_data else {
        panic!("walljump scratch missing")
    };
    let frozen = state.freeze_frames != 0;
    let core = &mut fighter.core;
    air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    let can_grab = core.status.ledge_cooldown == 0;
    let landed = if frozen {
        let cd = &mut core.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = core.physics.position;
        melee_mp::set_facing_dir(cd, if core.physics.facing < 0.0 { -1 } else { 1 });
        let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
        let position = |i| pose.position(i);
        let landed = if can_grab {
            phase.map.air_collide_platform_pass_ledge_ecb10(
                cd,
                Some(&mut accept_floor),
                Some(&position),
            )
        } else {
            phase
                .map
                .air_collide_platform_pass_ecb10(cd, Some(&mut accept_floor), Some(&position))
        };
        core.physics.position = cd.cur_pos;
        core.skeleton
            .set_translate(core.animation.root, &core.physics.position);
        landed
    } else {
        air::collide_fall_filtered(
            &mut core.physics,
            &mut core.collision,
            phase.map,
            &mut core.skeleton,
            core.animation.root,
            can_grab,
            stick_y,
            drop_threshold,
        )
    };
    if landed {
        if fighter.core.physics.self_velocity.y > assets.soft_landing_speed {
            fighter.land();
            fighter.change_motion_state(S::Wait.into(), assets)?;
        } else {
            fighter.enter_landing(assets)?;
        }
    } else if !fighter.try_wall_jump(assets, phase.map)? {
        fighter.try_grab_ledge(assets, phase.map)?;
    }
    Ok(())
}
