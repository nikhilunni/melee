//! Running reversal, ftCommon/ftCo_TurnRun.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::fma::{fmadds, fnmsubs};
use melee_types::CommonMotionState as S;

/// mv.co.turnrun: facing at entry (+234C) and paused-animation latch (+2354).
#[derive(Clone, Debug)]
pub struct TurnRunState {
    pub entry_facing: f32,
    pub animation_paused: bool,
}
// The skid pose releases once velocity toward the old facing is this small.
const TURN_RELEASE_SPEED: f32 = 0.01;
impl Fighter {
    /// fn_800C9CEC / fn_800C9D40 (800C9CEC / 800C9D40):
    /// RunBrake retains its phase; Run supplies frame zero.
    pub(super) fn try_turn_run(&mut self, assets: &FighterAssets, start: f32) -> Result<bool> {
        if self.core.input.current.stick.x * self.core.physics.facing
            <= assets.running.turn_threshold
        {
            self.enter_turn_run(assets, start)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// ftCo_TurnRun_Enter (800C9D94), ftCo_TurnRun.c:44-56.
    fn enter_turn_run(&mut self, assets: &FighterAssets, start: f32) -> Result<()> {
        self.core.commands.variables[1] = 0;
        self.core.state_data = MotionData::TurnRun(TurnRunState {
            entry_facing: self.core.physics.facing,
            animation_paused: false,
        });
        // Ft_MF_SkipAnimVel (fighter.c:1318-1324): retain ground momentum
        // when RunBrake supplies a nonzero phase.
        self.change_motion_state_keeping_velocity(S::TurnRun.into(), assets, start)
    }
    /// ftCo_TurnRun_Anim (800C9E10), ftCo_TurnRun.c:58-80.
    pub(super) fn turn_run_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::TurnRun(turn) = &mut self.core.state_data else {
            panic!("turn-run scratch missing")
        };
        if self.core.commands.variables[1] != 0 {
            if !turn.animation_paused {
                self.core
                    .animation
                    .set_rate(&mut self.core.skeleton, 0.0, false);
                turn.animation_paused = true;
            } else if turn.entry_facing * self.core.physics.ground_velocity <= TURN_RELEASE_SPEED {
                self.core
                    .animation
                    .set_rate(&mut self.core.skeleton, 1.0, false);
                self.core.commands.variables[1] = 0;
                self.core.physics.facing = -self.core.physics.facing;
            }
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            // fn_800CA644 (800CA644): restarting Run installs PlCo.x430.
            if self.core.input.current.stick.x * self.core.physics.facing
                >= assets.running.run_threshold
            {
                self.enter_run(assets)?;
                let MotionData::Run(run) = &mut self.core.state_data else {
                    unreachable!()
                };
                run.interrupt_delay = assets.running.turn_exit_interrupt_delay;
            } else {
                self.change_motion_state(S::Wait.into(), assets)?;
            }
        }
        Ok(())
    }
    /// ftCo_TurnRun_Coll (800CA024), ftCo_TurnRun.c:126-139.
    pub(super) fn turn_run_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
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
        } else if self.core.collision.data.env_flags as u32
            & (melee_types::mp::collide::LEFT_EDGE | melee_types::mp::collide::RIGHT_EDGE)
            != 0
        {
            // ftCo_TurnRun_Coll: Collide_LeftEdge | Collide_RightEdge ->
            // ftCommon_8007E2FC stops the fighter at the edge.
            self.clear_movement();
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_TurnRun_Phys (800C9EFC), ftCo_TurnRun.c:87-124.
    pub(super) fn turn_run_physics(&mut self, assets: &FighterAssets) {
        let MotionData::TurnRun(turn) = &self.state_data else {
            panic!("turn-run scratch missing")
        };
        let stick = self.input.current.stick.x;
        let attrs = &self.attributes.running;
        // Retail 800C9F28 fmuls, 800C9F44 fadds: do not contract.
        let mut acceleration = stick * attrs.dash_accel_mul;
        acceleration += if stick > 0.0 {
            attrs.dash_accel_base
        } else {
            -attrs.dash_accel_base
        };
        let target = stick * attrs.dash_max_velocity;
        let velocity = self.physics.ground_velocity;
        let friction = self.attributes.ground.ground_friction;
        let multiplier = assets.running.friction_multiplier;
        if target != 0.0 && turn.entry_facing * acceleration < 0.0 {
            if acceleration > 0.0 {
                if velocity + acceleration > target {
                    // Retail 800C9FA4 fnmsubs: acceleration - friction * multiplier.
                    acceleration = fnmsubs(friction, multiplier, acceleration);
                    if velocity + acceleration < target {
                        acceleration = target - velocity;
                    }
                }
            } else if velocity + acceleration < target {
                // Retail 800C9FD8 fmadds.
                acceleration = fmadds(friction, multiplier, acceleration);
                if velocity + acceleration > target {
                    acceleration = target - velocity;
                }
            }
            self.physics.ground_acceleration = acceleration;
        } else {
            self.physics.ground_acceleration =
                crate::physics::friction::friction_acceleration(velocity, friction * multiplier);
        }
    }
}
