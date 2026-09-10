//! Shared jab and up-tilt entry/callbacks, ftCo_Attack1.c / ftCo_AttackHi3.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::{pad::Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use melee_types::CommonMotionState as S;

#[derive(Clone, Debug)]
pub struct JabState {
    pub followup_window: f32,
    pub followup_pressed: bool,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// Grounded attack priority; checkAttack11 (8008ABC0), AttackHi3 doEnter (8008BA38).
    pub(super) fn enter_ground_attack(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.combat.has_recorded_hit {
            unimplemented!("ft_80089228: stale history across attack instances");
        }
        // Attack predicates share a transition enum; preserve the retail priority.
        let context = WaitContext {
            facing: self.core.physics.facing,
            ..WaitContext::default()
        };
        if self.first_ground_transition(assets, &context, &[P::SmashSide]) != T::None {
            return self.enter_forward_smash(assets);
        }
        if self.first_ground_transition(
            assets,
            &context,
            &[P::SmashSide, P::SmashUp, P::SmashDown, P::TiltSide],
        ) != T::None
        {
            unimplemented!("ftCo_Attack1.c:139-149: tilt/smash attack entry");
        }
        if self.first_ground_transition(assets, &context, &[P::TiltUp]) != T::None {
            self.change_motion_state(S::AttackHi3, assets)?;
            self.step_animation(assets);
            self.core.status.interaction = super::Interaction::Attack;
            self.core.state_data = MotionData::Tilt;
            return Ok(());
        }
        if self.first_ground_transition(assets, &context, &[P::TiltDown]) != T::None {
            unimplemented!("ftCo_AttackLw3: down tilt entry");
        }
        self.character.jab_variant();
        self.core.commands.jab_followup = false;
        self.core.commands.jab_combo = false;
        self.change_motion_state(S::Attack11, assets)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = MotionData::Jab(JabState {
            followup_window: self.core.attributes.combat.jab_2_input_window,
            followup_pressed: false,
        });
        Ok(())
    }
    /// ftCo_Attack11_Anim (8008AC9C).
    pub(super) fn jab_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Wait, assets)?;
        }
        Ok(())
    }
    /// ftCo_AttackHi3_IASA (8008BAD4): Wait predicates after script unlock.
    pub(super) fn tilt_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.core.commands.allow_interrupt {
            let transition = crate::input::wait_iasa(&self.core.input, &assets.input, context);
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
    /// ftCo_Attack11_IASA (8008ACD8), checkAttack12 (8008AF0C).
    pub(super) fn jab_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.core.commands.allow_interrupt
            && self.first_ground_transition(
                assets,
                context,
                &[
                    P::SmashSide,
                    P::SmashUp,
                    P::SmashDown,
                    P::TiltSide,
                    P::TiltUp,
                    P::TiltDown,
                ],
            ) != T::None
        {
            unimplemented!("ftCo_Attack1.c:143-148: jab interrupt attack");
        }
        let MotionData::Jab(jab) = &mut self.core.state_data else {
            panic!("jab scratch missing")
        };
        if jab.followup_window > 0.0 {
            jab.followup_window -= 1.0;
            if self.core.input.pressed.intersects(Buttons::A) {
                jab.followup_pressed = true;
            }
        }
        if jab.followup_pressed && self.core.commands.jab_followup {
            unimplemented!("ftCo_Attack1.c:219-220: Attack12 entry");
        }
        if self.core.commands.allow_interrupt {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[P::Jump, P::Dash, P::Squat, P::Turn, P::Walk],
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_Attack11_Phys (8008ADF0) -> ft_80084FA8 (80084FA8).
    pub(super) fn jab_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: hsd_types::Vec3,
    ) {
        use crate::physics::grounded::{self, GroundedParameters};
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("jab TransN")
                .primary_history
                .offset
                .z;
            // retail ft_80085030, 8008505C: fmsubs.
            self.physics.ground_acceleration =
                gekko_math::fma::fmsubs(offset, self.physics.facing, self.physics.ground_velocity);
            grounded::apply_ground_movement(
                &mut self.physics,
                self.collision.data.floor.normal,
                map.floor_speed_scale(&self.collision.data),
            );
        } else {
            grounded::friction_physics(
                &mut self.physics,
                &params,
                self.collision.data.floor.normal,
                map.floor_speed_scale(&self.collision.data),
            );
        }
        grounded::finish_ground_update(&mut self.physics, &self.collision.data, &params, map, wind);
    }
}
