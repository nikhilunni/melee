//! Ledge catch, wait and jump: ftcliffcommon.c / ftCo_CliffWait.c /
//! ftCo_CliffJump.c. Ledge positions come from map geometry and animated TransN.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::Buttons;
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::collide, CommonMotionState as S};

#[derive(Clone, Copy, Debug)]
pub struct LedgeParameters {
    pub grab_down_threshold: f32,
    pub slow_damage: i32,
    pub wait_frames: [f32; 2],
    pub option_threshold: f32,
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
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCliffCommon_80081298 (80081298), ftcliffcommon.c:23-53.
    /// The caller must mark fighter interactions; occupied-ledge arbitration
    /// (ft_80082E3C) is outside the isolated Fox / idle-opponent slice.
    pub(super) fn try_grab_ledge(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<bool> {
        if self.input.current.stick.y <= -assets.ledge.grab_down_threshold
            || self.status.ledge_grab_disabled
            || self.collision.data.env_flags as u32 & collide::LEDGE_GRAB_MASK == 0
        {
            return Ok(false);
        }
        self.enter_cliff_catch(assets, map)?;
        Ok(true)
    }
    /// ftCliffCommon_80081370 (80081370), ftcliffcommon.c:55-114.
    fn enter_cliff_catch(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<()> {
        self.physics.facing =
            if self.collision.data.env_flags as u32 & collide::LEFT_LEDGE_GRAB != 0 {
                1.0
            } else {
                -1.0
            };
        let ledge_id = if self.physics.facing > 0.0 {
            self.collision.data.ledge_id_left
        } else {
            self.collision.data.ledge_id_right
        };
        self.leave_ground();
        self.change_motion_state(S::CliffCatch, assets)?;
        self.step_animation(assets);
        self.leave_ground();
        self.status.name_tag_timer = assets.name_tag_duration;
        // ftCommon_8007E2FC (8007E2FC): clear every movement source.
        self.physics.ground_acceleration = 0.0;
        self.physics.secondary_ground_acceleration = 0.0;
        self.physics.animation_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.ground_knockback_velocity = 0.0;
        self.physics.knockback_velocity = Vec3::ZERO;
        self.physics.ground_shield_knockback_velocity = 0.0;
        self.physics.shield_knockback_velocity = Vec3::ZERO;
        self.status.on_ledge = true;
        // Only ledge_id is written by catch. The other scratch words are
        // initialized by CliffWait before they are read.
        self.state_data = MotionData::Cliff(CliffState {
            ledge_id,
            wait_frames: 0.0,
            neutral_seen: false,
        });
        self.ledge_physics(assets, map)?;
        self.effects.push(super::effects::EffectRequest::LedgeGrab {
            position: self.ledge_position(map, ledge_id),
        });
        Ok(())
    }
    fn ledge_position(&self, map: &CollMap, ledge_id: i32) -> Vec3 {
        if self.physics.facing > 0.0 {
            map.floor_chain_left_end_id0(ledge_id)
        } else {
            map.floor_chain_right_end_id0(ledge_id)
        }
    }
    /// ftCo_CliffCatch_Phys (80081544); also CliffWait/CliffJump1 physics.
    pub(super) fn ledge_physics(&mut self, assets: &FighterAssets, map: &CollMap) -> Result<()> {
        let MotionData::Cliff(cliff) = &self.state_data else {
            panic!("cliff scratch missing")
        };
        if !map.line_is_active(cliff.ledge_id) {
            return self.change_motion_state(S::Fall, assets);
        }
        let edge = self.ledge_position(map, cliff.ledge_id);
        let translation = self
            .animation
            .root_motion
            .as_ref()
            .expect("ledge TransN")
            .primary_history
            .position;
        // Retail 800815A8 fmadds, then separate 800815B8 fadds.
        self.physics.position.x = fmadds(translation.z, self.physics.facing, edge.x);
        self.physics.position.y = edge.y + translation.y;
        Ok(())
    }
    /// CliffCatch_Anim (80081504), CliffWait_Anim (8009A8D8),
    /// CliffJump1_Anim (8009B278), ftCo_8009B2F8 (8009B2F8).
    pub(super) fn ledge_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.motion_state.id == S::CliffWait {
            let MotionData::Cliff(cliff) = &mut self.state_data else {
                panic!("cliff scratch missing")
            };
            if cliff.wait_frames > 0.0 {
                cliff.wait_frames -= 1.0;
            }
            return Ok(());
        }
        if self.animation.frames_remaining(&self.skeleton) {
            return Ok(());
        }
        match self.motion_state.id {
            S::CliffCatch => {
                // ftCo_8009A804 (8009A804), ftCo_CliffWait.c:22-40.
                self.change_motion_state(S::CliffWait, assets)?;
                self.status.on_ledge = true;
                let MotionData::Cliff(cliff) = &mut self.state_data else {
                    panic!("cliff scratch missing")
                };
                cliff.neutral_seen = false;
                cliff.wait_frames = assets.ledge.wait_frames
                    [usize::from(self.physics.percent >= assets.ledge.slow_damage as f32)];
                self.status.ledge_intangibility = self
                    .status
                    .ledge_intangibility
                    .max(assets.ledge.intangible_frames);
            }
            S::CliffJumpQuick1 | S::CliffJumpSlow1 => {
                let MotionData::Cliff(cliff) = &self.state_data else {
                    panic!("cliff scratch missing")
                };
                let retained_wait_frames = cliff.wait_frames;
                let next = if self.motion_state.id == S::CliffJumpQuick1 {
                    S::CliffJumpQuick2
                } else {
                    S::CliffJumpSlow2
                };
                self.change_motion_state(next, assets)?;
                self.step_animation(assets);
                self.state_data = MotionData::CliffJump(CliffJumpState {
                    physics_started: false,
                    retained_wait_frames,
                });
                // Retail 8009B368 fmadds: preserve any prior horizontal momentum.
                self.physics.self_velocity.x = fmadds(
                    self.physics.facing,
                    self.attributes.ledge.ledge_jump_horizontal_velocity,
                    self.physics.self_velocity.x,
                );
                self.physics.self_velocity.y = self.attributes.ledge.ledge_jump_vertical_velocity;
            }
            S::CliffJumpQuick2 | S::CliffJumpSlow2 => self.change_motion_state(S::Fall, assets)?,
            _ => unreachable!(),
        }
        Ok(())
    }
    /// ftCo_CliffWait_IASA (8009A8FC): attack, escape, jump, stick option, timeout.
    pub(super) fn ledge_input(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.input.current.cstick != crate::input::Stick::default() {
            unimplemented!("ftCo_CliffWait.c:53-56 / ft_0DF1.c:130-159: C-stick ledge options");
        }
        if self.input.pressed.intersects(Buttons::A | Buttons::B) {
            unimplemented!("ftCo_CliffAttack.c:29-49: CliffAttack entry");
        }
        if self.input.pressed.intersects(Buttons::SHIELD) {
            unimplemented!(
                "ftCo_CliffAttack.c:78-84 / ftCo_CliffEscape.c:14-27: CliffEscape entry"
            );
        }
        let jump = self.input.pressed.intersects(Buttons::XY)
            || (self.input.current.stick.y >= assets.input.thresholds.tap_jump_threshold
                && i32::from(self.input.vertical.tilt) < assets.input.thresholds.tap_jump_window);
        if jump {
            let next = if self.physics.percent < assets.ledge.slow_damage as f32 {
                S::CliffJumpQuick1
            } else {
                S::CliffJumpSlow1
            };
            self.change_motion_state(next, assets)?;
            self.step_animation(assets);
            self.status.on_ledge = true;
            return Ok(());
        }
        let MotionData::Cliff(cliff) = &mut self.state_data else {
            panic!("cliff scratch missing")
        };
        let stick = self.input.current.stick;
        if fabsf(stick.x) >= assets.ledge.option_threshold
            || fabsf(stick.y) >= assets.ledge.option_threshold
        {
            let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
            let climb = angle > assets.ledge.option_angle
                || (angle > -assets.ledge.option_angle && stick.x * self.physics.facing >= 0.0);
            if cliff.neutral_seen {
                if climb {
                    unimplemented!("ftCo_CliffClimb.c:83-94: CliffClimb entry");
                }
                self.status.ledge_cooldown = assets.ledge.cooldown;
                self.change_motion_state(S::Fall, assets)?;
                return Ok(());
            }
        } else {
            cliff.neutral_seen = true;
        }
        let MotionData::Cliff(cliff) = &self.state_data else {
            unreachable!()
        };
        if cliff.wait_frames <= 0.0 {
            self.status.ledge_cooldown = assets.ledge.cooldown;
            unimplemented!("ftCo_CliffWait.c:74-79: ledge timeout -> DamageFall");
        }
        Ok(())
    }
    /// ftCo_CliffJump2_Phys (8009B464): skip gravity only on the launch tick.
    pub(super) fn ledge_jump_physics(&mut self, assets: &FighterAssets) {
        let MotionData::CliffJump(jump) = &mut self.state_data else {
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
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            unimplemented!("ftCo_CliffClimb.c:155-156: grounded ledge-option collision");
        }
        air::begin_map(
            &self.physics,
            &mut self.collision,
            &mut self.skeleton,
            self.animation.root,
        );
        let cd = &mut self.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = self.physics.position;
        let pose = EcbPose::read(&mut self.skeleton, self.animation.root, cd);
        let landed = map.air_collide_ecb10(cd, Some(&|i| pose.position(i)));
        self.physics.position = cd.cur_pos;
        if landed {
            if matches!(self.motion_state.id, S::CliffJumpQuick1 | S::CliffJumpSlow1) {
                self.land();
            } else if self.physics.self_velocity.y > assets.soft_landing_speed {
                self.land();
                self.change_motion_state(S::Wait, assets)?;
            } else {
                self.enter_landing(assets)?;
            }
        } else if cd.env_flags as u32 & collide::CEILING_HUG != 0 {
            unimplemented!("ftcliffcommon.c:153-156 / ftCo_StopCeil.c:16-22: ledge ceiling impact");
        }
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        Ok(())
    }
}
