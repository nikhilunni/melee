//! Jab entry and callbacks, ftCo_Attack1.c.
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
    /// checkAttack11 (8008ABC0); item pickup is excluded by the scene contract.
    pub(super) fn enter_jab(&mut self, assets: &FighterAssets) -> Result<()> {
        // Attack predicates share a transition enum; reject non-jab entries.
        let context = WaitContext {
            facing: self.physics.facing,
            ..WaitContext::default()
        };
        if self.first_ground_transition(
            assets,
            &context,
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
            unimplemented!("ftCo_Attack1.c:139-149: tilt/smash attack entry");
        }
        self.character.jab_variant();
        self.commands.jab_followup = false;
        self.change_motion_state(S::Attack11, assets)?;
        self.step_animation(assets);
        self.status.interaction = super::Interaction::Attack;
        self.state_data = MotionData::Jab(JabState {
            followup_window: self.attributes.combat.jab_2_input_window,
            followup_pressed: false,
        });
        Ok(())
    }
    /// ftCo_Attack11_Anim (8008AC9C).
    pub(super) fn jab_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.animation.frames_remaining(&self.skeleton) {
            self.change_motion_state(S::Wait, assets)?;
        }
        Ok(())
    }
    /// ftCo_Attack11_IASA (8008ACD8), checkAttack12 (8008AF0C).
    pub(super) fn jab_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.commands.allow_interrupt
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
        let MotionData::Jab(jab) = &mut self.state_data else {
            panic!("jab scratch missing")
        };
        if jab.followup_window > 0.0 {
            jab.followup_window -= 1.0;
            if self.input.pressed.intersects(Buttons::A) {
                jab.followup_pressed = true;
            }
        }
        if jab.followup_pressed && self.commands.jab_followup {
            unimplemented!("ftCo_Attack1.c:219-220: Attack12 entry");
        }
        if self.commands.allow_interrupt {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[P::Jump, P::Dash, P::Squat, P::Turn, P::Walk],
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
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
