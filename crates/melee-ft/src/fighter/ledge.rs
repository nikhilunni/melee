//! Ledge catch, wait, jump, climb and escape (ftcliffcommon.c / ftCo_Cliff*.c).
//! Ledge positions come from map geometry and animated TransN.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::Buttons;
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::collide, CommonMotionState as S};

/// ftCommon_8007E2F4 (8007E2F4): grab-category exclusion flags, tested
/// against the attacker's grab mask by ftcoll.c:1588.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrabExclusions(pub u16);
impl GrabExclusions {
    pub const NONE: Self = Self(0);
    /// CliffCatch/Wait exclude every retail grab category.
    pub const ALL: Self = Self(0x1FF);
    /// Climb, escape and the first ledge-jump phase exclude category bit 5.
    pub const LEDGE_OPTION: Self = Self(0x20);
}

#[derive(Clone, Copy, Debug)]
pub struct LedgeParameters {
    pub grab_down_threshold: f32,
    pub slow_damage: i32,
    pub wait_frames: [f32; 2],
    pub option_threshold: f32,
    pub cstick_attack_threshold: f32,
    pub cstick_escape_threshold: f32,
    pub option_angle: f32,
    pub cooldown: i32,
    pub intangible_frames: i32,
}
impl LedgeParameters {
    /// ftCommonData +20, +480..49C, read at the archive boundary.
    pub fn read(archive: &Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            grab_down_threshold: r.f32(base + 0x480)?,
            slow_damage: r.s32(base + 0x488)?,
            wait_frames: [r.f32(base + 0x48C)?, r.f32(base + 0x490)?],
            option_threshold: r.f32(base + 0x494)?,
            cstick_attack_threshold: r.f32(base + 0x7F8)?,
            cstick_escape_threshold: r.f32(base + 0x7FC)?,
            option_angle: r.f32(base + 0x20)?,
            cooldown: r.s32(base + 0x498)?,
            intangible_frames: r.s32(base + 0x49C)?,
        })
    }
}
/// mv.co.cliff +2340/+2344/+2348. The wait timer also survives into Jump2.
#[derive(Clone, Debug)]
pub struct CliffState {
    pub ledge_id: i32,
    pub wait_frames: f32,
    pub neutral_seen: bool,
}
/// mv.co.cliffjump: first-physics latch plus the retained inactive wait word.
#[derive(Clone, Debug)]
pub struct CliffJumpState {
    pub physics_started: bool,
    pub retained_wait_frames: f32,
}
impl Fighter {
    /// ftCliffCommon_80081298 (80081298), ftcliffcommon.c:23-53.
    /// The caller must mark fighter interactions; occupied-ledge arbitration
    /// (ft_80082E3C) is outside the isolated Fox / idle-opponent slice.
    pub fn try_grab_ledge(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<bool> {
        if self.core.input.current.stick.y <= -assets.ledge.grab_down_threshold
            || self.core.status.ledge_grab_disabled
            || self.core.collision.data.env_flags as u32 & collide::LEDGE_GRAB_MASK == 0
        {
            return Ok(false);
        }
        self.enter_cliff_catch(assets, map)?;
        Ok(true)
    }
    /// ftCliffCommon_80081370 (80081370), ftcliffcommon.c:55-114.
    fn enter_cliff_catch(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<()> {
        self.core.physics.facing =
            if self.core.collision.data.env_flags as u32 & collide::LEFT_LEDGE_GRAB != 0 {
                1.0
            } else {
                -1.0
            };
        let ledge_id = if self.core.physics.facing > 0.0 {
            self.core.collision.data.ledge_id_left
        } else {
            self.core.collision.data.ledge_id_right
        };
        self.leave_ground();
        self.change_motion_state(S::CliffCatch.into(), assets)?;
        self.step_animation(assets);
        self.leave_ground();
        self.core.status.name_tag_timer = assets.name_tag_duration;
        self.clear_movement();
        self.core.status.grab_exclusions = GrabExclusions::ALL;
        self.core.status.on_ledge = true;
        // Only ledge_id is written by catch. The other scratch words are
        // initialized by CliffWait before they are read.
        self.core.state_data = MotionData::Cliff(CliffState {
            ledge_id,
            wait_frames: 0.0,
            neutral_seen: false,
        });
        self.ledge_physics(assets, map)?;
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::LedgeGrab {
                position: self.ledge_position(map, ledge_id),
            });
        Ok(())
    }
    /// ftCo_CliffCatch_Phys (80081544); also CliffWait/CliffJump1 physics.
    pub(super) fn ledge_physics(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<()> {
        let MotionData::Cliff(cliff) = &self.core.state_data else {
            panic!("cliff scratch missing")
        };
        if !map.line_is_active(cliff.ledge_id) {
            return self.change_motion_state(S::Fall.into(), assets);
        }
        self.core.place_at_ledge(map, cliff.ledge_id);
        Ok(())
    }
    /// CliffCatch_Anim (80081504), CliffWait_Anim (8009A8D8),
    /// CliffJump1_Anim (8009B278), ftCo_8009B2F8 (8009B2F8).
    pub(super) fn ledge_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.motion_state.id == S::CliffWait {
            let MotionData::Cliff(cliff) = &mut self.core.state_data else {
                panic!("cliff scratch missing")
            };
            if cliff.wait_frames > 0.0 {
                cliff.wait_frames -= 1.0;
            }
            return Ok(());
        }
        if self.core.animation.frames_remaining(&self.core.skeleton) {
            return Ok(());
        }
        match self.core.motion_state.id {
            S::CliffCatch => {
                // ftCo_8009A804 (8009A804), ftCo_CliffWait.c:22-40.
                self.change_motion_state(S::CliffWait.into(), assets)?;
                self.core.status.grab_exclusions = GrabExclusions::ALL;
                self.core.status.on_ledge = true;
                let MotionData::Cliff(cliff) = &mut self.core.state_data else {
                    panic!("cliff scratch missing")
                };
                cliff.neutral_seen = false;
                cliff.wait_frames = assets.ledge.wait_frames
                    [usize::from(self.core.physics.percent >= assets.ledge.slow_damage as f32)];
                self.core.status.ledge_intangibility = self
                    .core
                    .status
                    .ledge_intangibility
                    .max(assets.ledge.intangible_frames);
            }
            S::CliffJumpQuick1 | S::CliffJumpSlow1 => {
                let MotionData::Cliff(cliff) = &self.core.state_data else {
                    panic!("cliff scratch missing")
                };
                let retained_wait_frames = cliff.wait_frames;
                let next = if self.core.motion_state.id == S::CliffJumpQuick1 {
                    S::CliffJumpQuick2
                } else {
                    S::CliffJumpSlow2
                };
                self.change_motion_state(next.into(), assets)?;
                self.step_animation(assets);
                self.core.state_data = MotionData::CliffJump(CliffJumpState {
                    physics_started: false,
                    retained_wait_frames,
                });
                // Retail 8009B368 fmadds: preserve any prior horizontal momentum.
                self.core.physics.self_velocity.x = fmadds(
                    self.core.physics.facing,
                    self.core.attributes.ledge.ledge_jump_horizontal_velocity,
                    self.core.physics.self_velocity.x,
                );
                self.core.physics.self_velocity.y =
                    self.core.attributes.ledge.ledge_jump_vertical_velocity;
            }
            S::CliffJumpQuick2 | S::CliffJumpSlow2 => {
                self.change_motion_state(S::Fall.into(), assets)?
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    /// ftCo_CliffWait_IASA (8009A8FC): attack, escape, jump, stick option, timeout.
    pub(super) fn ledge_input(&mut self, assets: &FighterAssets) -> Result<()> {
        // ftCo_800DF6F8 / 800DF72C: crossings, not held directions.
        // Retail ASM uses comparisons and separate facing products; no FMA.
        let current_cstick = self.core.input.current.cstick;
        let previous_cstick = self.core.input.previous.cstick;
        let cstick_attack = previous_cstick.y < assets.ledge.cstick_attack_threshold
            && current_cstick.y >= assets.ledge.cstick_attack_threshold;
        if self.core.input.pressed.intersects(Buttons::A | Buttons::B) || cstick_attack {
            return self.enter_cliff_option(assets, S::CliffAttackQuick);
        }
        // gm_8016B0FC is false in the supported versus-match rules.
        let cstick_escape = self.core.physics.facing * previous_cstick.x
            < assets.ledge.cstick_escape_threshold
            && self.core.physics.facing * current_cstick.x >= assets.ledge.cstick_escape_threshold;
        if self.core.input.pressed.intersects(Buttons::SHIELD) || cstick_escape {
            return self.enter_cliff_option(assets, S::CliffEscapeQuick);
        }
        let jump = self.core.input.pressed.intersects(Buttons::XY)
            || (self.core.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
                && i32::from(self.core.input.vertical.tilt)
                    < assets.input.thresholds.tap_jump_window);
        if jump {
            let next = if self.core.physics.percent < assets.ledge.slow_damage as f32 {
                S::CliffJumpQuick1
            } else {
                S::CliffJumpSlow1
            };
            self.change_motion_state(next.into(), assets)?;
            self.step_animation(assets);
            self.core.status.on_ledge = true;
            self.core.status.grab_exclusions = GrabExclusions::LEDGE_OPTION;
            return Ok(());
        }
        let MotionData::Cliff(cliff) = &mut self.core.state_data else {
            panic!("cliff scratch missing")
        };
        // ftCo_8009AA0C: left stick wins; a C-stick direction may drop,
        // but the climb branch requires the left-stick selector.
        let left = self.core.input.current.stick;
        let left_active = fabsf(left.x) >= assets.ledge.option_threshold
            || fabsf(left.y) >= assets.ledge.option_threshold;
        let stick = if left_active { left } else { current_cstick };
        if fabsf(stick.x) >= assets.ledge.option_threshold
            || fabsf(stick.y) >= assets.ledge.option_threshold
        {
            let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
            let climb = angle > assets.ledge.option_angle
                || (angle > -assets.ledge.option_angle
                    && stick.x * self.core.physics.facing >= 0.0);
            if cliff.neutral_seen {
                if climb {
                    if left_active {
                        return self.enter_cliff_option(assets, S::CliffClimbQuick);
                    }
                } else {
                    self.core.status.ledge_cooldown = assets.ledge.cooldown;
                    self.change_motion_state(S::Fall.into(), assets)?;
                    return Ok(());
                }
            }
        } else {
            cliff.neutral_seen = true;
        }
        let MotionData::Cliff(cliff) = &self.core.state_data else {
            unreachable!()
        };
        if cliff.wait_frames <= 0.0 {
            self.core.status.ledge_cooldown = assets.ledge.cooldown;
            self.core.status.ledge_timed_out = true;
            self.enter_damage_fall(assets)?;
        }
        Ok(())
    }
    /// ftCo_8009AB9C / ftCo_8009B040 (8009AB9C / 8009B040),
    /// ftCo_CliffClimb.c:73-85 / ftCo_CliffEscape.c:14-28.
    fn enter_cliff_option(&mut self, assets: &FighterAssets, state: S) -> Result<()> {
        let state = if self.core.physics.percent >= assets.ledge.slow_damage as f32 {
            match state {
                S::CliffClimbQuick => S::CliffClimbSlow,
                S::CliffAttackQuick => S::CliffAttackSlow,
                S::CliffEscapeQuick => S::CliffEscapeSlow,
                _ => unreachable!("normal ledge option entry"),
            }
        } else {
            state
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.status.grab_exclusions = GrabExclusions::LEDGE_OPTION;
        self.core.status.on_ledge = true;
        self.core.status.ignore_fighter_nudge = true;
        // The first Phys snaps the stepped TransN before ground detection.
        Ok(())
    }
    /// ftCo_CliffClimb_Anim / ftCo_CliffEscape_Anim (8009AC68 / 8009B10C),
    /// then ftCommon_8007D92C (8007D92C): ground -> Wait, air -> Fall.
    pub(super) fn cliff_climb_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(
                (if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
                    S::Fall
                } else {
                    S::Wait
                })
                .into(),
                assets,
            )?;
        }
        Ok(())
    }
    /// ftCo_CliffClimb_Phys (8009ACA8), reused by CliffEscape_Phys (8009B130).
    pub(super) fn cliff_climb_physics(
        &mut self,
        assets: &FighterAssets,
        map: &CollMap,
    ) -> Result<()> {
        if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
            self.ledge_physics(assets, map)?;
            if self.core.motion_state.id == S::Fall {
                return Ok(());
            }
            let translation = self
                .core
                .animation
                .root_motion
                .as_ref()
                .expect("ledge TransN")
                .primary_history
                .position;
            if translation.z >= 0.0 && translation.y >= 0.0 {
                let MotionData::Cliff(cliff) = &self.core.state_data else {
                    panic!("cliff scratch missing")
                };
                self.core.collision.data.floor.index = cliff.ledge_id;
                self.land();
            }
        } else {
            self.core.cliff_ground_physics(assets, map);
        }
        Ok(())
    }
    /// ftCo_CliffJump2_Phys (8009B464): skip gravity only on the launch tick.
    pub(super) fn ledge_jump_physics(&mut self, assets: &FighterAssets) {
        let MotionData::CliffJump(jump) = &mut self.core.state_data else {
            panic!("ledge jump scratch missing")
        };
        if !jump.physics_started {
            jump.physics_started = true;
        } else {
            self.airborne_physics(assets);
        }
    }
    /// ftCo_CliffCatch_Coll (800815E4), CliffClimb_Coll (8009ADA4).
    pub(super) fn ledge_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<()> {
        use crate::collision::{air, ecb::EcbPose};
        if self.core.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            let result = crate::collision::ground::map_escape(
                &mut self.core.physics,
                &mut self.core.collision,
                map,
                &mut self.core.skeleton,
                self.core.animation.root,
                self.core.input.current.stick.x,
            );
            if result == crate::collision::ground::WaitGroundResult::EnterFall {
                self.change_motion_state(S::Fall.into(), assets)?;
            }
            return Ok(());
        }
        air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        let cd = &mut self.core.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = self.core.physics.position;
        let pose = EcbPose::read(&mut self.core.skeleton, self.core.animation.root, cd);
        let landed = map.air_collide_ecb10(cd, Some(&|i| pose.position(i)));
        self.core.physics.position = cd.cur_pos;
        if landed {
            if matches!(
                self.core.motion_state.id,
                S::CliffClimbQuick
                    | S::CliffAttackQuick
                    | S::CliffEscapeQuick
                    | S::CliffJumpQuick1
                    | S::CliffJumpSlow1
            ) {
                self.land();
            } else if self.core.physics.self_velocity.y > assets.soft_landing_speed {
                self.land();
                self.change_motion_state(S::Wait.into(), assets)?;
            } else {
                self.enter_landing(assets)?;
            }
        } else if cd.env_flags as u32 & collide::CEILING_HUG != 0 {
            // CliffCatch/Wait impose cooldown after a ceiling exit; ledge
            // options use CliffClimb_Coll, which does not perform that write.
            let hanging = matches!(self.core.motion_state.id, S::CliffCatch | S::CliffWait);
            self.enter_stop_ceil(assets, map)?;
            if hanging {
                self.core.status.ledge_cooldown = assets.ledge.cooldown;
            }
        }
        self.core
            .skeleton
            .set_translate(self.core.animation.root, &self.core.physics.position);
        Ok(())
    }
}
impl FighterCore {
    /// ftCommon_8007E2FC (8007E2FC): clear every movement source.
    pub(super) fn clear_movement(&mut self) {
        self.physics.ground_acceleration = 0.0;
        self.physics.secondary_ground_acceleration = 0.0;
        self.physics.animation_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.ground_knockback_velocity = 0.0;
        self.physics.knockback_velocity = Vec3::ZERO;
        self.physics.ground_shield_knockback_velocity = 0.0;
        self.physics.shield_knockback_velocity = Vec3::ZERO;
    }
    pub(super) fn ledge_position(&self, map: &CollMap, ledge_id: i32) -> Vec3 {
        if self.physics.facing > 0.0 {
            map.floor_chain_left_end_id0(ledge_id)
        } else {
            map.floor_chain_right_end_id0(ledge_id)
        }
    }
}

impl FighterCore {
    /// ft_80084FA8 / ft_80085030 (80085030): grounded ledge root motion.
    fn cliff_ground_physics(&mut self, assets: &FighterAssets, map: &CollMap) {
        // ft_80084FA8 (80084FA8) -> ft_80085030 (80085030).
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("ledge TransN")
                .primary_history
                .offset
                .z;
            // Retail 8008505C fmsubs.
            self.physics.ground_acceleration =
                gekko_math::fma::fmsubs(offset, self.physics.facing, self.physics.ground_velocity);
        } else {
            let friction = crate::physics::friction::wait_friction(
                self.physics.ground_velocity,
                self.attributes.ground.ground_friction,
                self.attributes.walking.walk_max_vel,
                assets.common.friction_when_above_walk_speed,
            );
            self.physics.ground_acceleration = crate::physics::friction::friction_acceleration(
                self.physics.ground_velocity,
                friction,
            );
        }
        crate::physics::grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
    }
}

impl FighterCore {
    /// ftCo_CliffCatch_Phys (80081544): active ledge plus animated TransN.
    fn place_at_ledge(&mut self, map: &CollMap, ledge_id: i32) {
        let edge = self.ledge_position(map, ledge_id);
        let translation = self
            .animation
            .root_motion
            .as_ref()
            .expect("ledge TransN")
            .primary_history
            .position;
        // Retail 800815A8 / 8009AD1C fmadds, then separate 800815B8 / 8009AD2C fadds.
        self.physics.position.x = fmadds(translation.z, self.physics.facing, edge.x);
        self.physics.position.y = edge.y + translation.y;
    }
}

/// ftCo_CliffAttack_Anim (8009AF70): same completion as the ledge climb.
pub fn attack_animation(
    f: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    f.step_animation(phase.assets);
    f.cliff_climb_animation(phase.assets)?;
    Ok(None)
}
