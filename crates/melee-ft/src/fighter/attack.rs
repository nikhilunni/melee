//! Shared ground attack entry/callbacks, ftCo_Attack1.c / ftCo_AttackHi3.c.
pub mod aerial;
pub mod combo;
pub mod stale;
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{pad::Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use melee_types::CommonMotionState as S;

#[derive(Clone, Debug)]
pub struct JabState {
    pub followup_window: f32,
    pub followup_pressed: bool,
    pub rapid_edges: i32,
}
impl Fighter {
    /// Grounded attack priority; checkAttack11 (8008ABC0), AttackHi3 doEnter (8008BA38).
    pub(super) fn enter_ground_attack(&mut self, assets: &FighterAssets) -> Result<()> {
        // Attack predicates share a transition enum; preserve the retail priority.
        let context = WaitContext {
            facing: self.core.physics.facing,
            ..WaitContext::default()
        };
        if self.first_ground_transition(assets, &context, &[P::SmashSide]) != T::None {
            return self.enter_forward_smash(assets);
        }
        for (predicate, state) in [
            (P::SmashUp, S::AttackHi4),
            (P::SmashDown, S::AttackLw4),
            (P::TiltSide, S::AttackS3S),
            (P::TiltUp, S::AttackHi3),
            (P::TiltDown, S::AttackLw3),
        ] {
            if self.first_ground_transition(assets, &context, &[predicate]) != T::None {
                let state = if state == S::AttackS3S {
                    self.side_tilt_state(assets)
                } else {
                    state
                };
                return self.enter_simple_attack(state, assets);
            }
        }
        self.enter_jab(assets)
    }
    /// checkAttack11 (8008ABC0), also used by a looping jab combo.
    fn enter_jab(&mut self, assets: &FighterAssets) -> Result<()> {
        self.character.jab_variant();
        self.core.commands.jab_followup = false;
        self.core.commands.rapid_jab = false;
        self.change_motion_state(S::Attack11.into(), assets)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = MotionData::Jab(JabState {
            followup_window: self.core.attributes.combat.jab_2_input_window,
            followup_pressed: false,
            rapid_edges: 0,
        });
        Ok(())
    }
    /// ftCo_AttackHi4 doEnter (8008CA38), AttackLw4 (8008CC5C),
    /// AttackHi3 (8008BA38), AttackLw3 (8008BC70), AttackDash (8008B4D4).
    pub(super) fn enter_simple_attack(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        self.core.commands.allow_interrupt = false;
        self.core.commands.variables[0] = 0;
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = if state == S::AttackLw3 {
            MotionData::DownTilt {
                repeat_pressed: false,
            }
        } else {
            MotionData::Tilt
        };
        Ok(())
    }
    /// ftCo_AttackLw3_Anim (8008BCFC): repeat latch, then SquatWait on completion.
    pub(super) fn down_tilt_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::DownTilt { repeat_pressed } = self.core.state_data else {
            panic!("down tilt scratch")
        };
        if self.core.commands.variables[0] != 0 && repeat_pressed {
            return self.enter_simple_attack(S::AttackLw3, assets);
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::SquatWait.into(), assets)?;
            self.core.state_data = MotionData::Squat(super::squat::SquatState::default());
        }
        Ok(())
    }
    /// ftCo_AttackLw3_IASA (8008BD80): latch A before ordinary movement interrupts.
    pub(super) fn down_tilt_input(
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
                ],
            ) != T::None
        {
            return self.enter_ground_attack(assets);
        }
        if self.core.input.pressed.intersects(Buttons::A) {
            if self.core.commands.variables[0] != 0 {
                return self.enter_simple_attack(S::AttackLw3, assets);
            }
            let MotionData::DownTilt { repeat_pressed } = &mut self.core.state_data else {
                panic!("down tilt scratch")
            };
            *repeat_pressed = true;
        }
        if self.core.commands.allow_interrupt {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[
                    P::TiltDown,
                    P::Jab,
                    P::Jump,
                    P::Dash,
                    // ftCo_AttackLw3_IASA calls the pure check: down held
                    // keeps the tilt and skips Turn/Walk.
                    P::SquatHeld,
                    P::Turn,
                    P::Walk,
                ],
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
    /// ftCo_Attack100Start_Anim (800D6C0C): loop entry does not advance animation.
    pub(super) fn rapid_start_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Attack100Loop.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_Attack100Loop_Anim (800D6D48): evaluate the script's end flag before IASA.
    pub(super) fn rapid_loop_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::RapidJab(rapid) = &mut self.core.state_data else {
            panic!("rapid jab scratch")
        };
        if self.core.animation.frame >= 0.0 && self.core.animation.frame < self.core.animation.speed
        {
            rapid.loop_started = true;
            self.core.combat.stale.new_instance();
        }
        if std::mem::take(&mut self.core.commands.rapid_jab_loop_end) {
            if rapid.loop_started && !rapid.edge_pressed {
                self.change_motion_state(S::Attack100End.into(), assets)?;
            } else {
                rapid.edge_pressed = false;
            }
        }
        Ok(())
    }
    /// ftCo_Attack11_Anim (8008AC9C).
    pub(super) fn jab_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Wait.into(), assets)?;
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
            return self.enter_ground_attack(assets);
        }
        let MotionData::Jab(jab) = &mut self.core.state_data else {
            panic!("jab scratch missing")
        };
        if (self.core.input.pressed | self.core.input.released).intersects(Buttons::A) {
            jab.rapid_edges += 1;
        }
        if self.core.commands.rapid_jab
            && jab.rapid_edges >= self.core.attributes.combat.rapid_jab_window
        {
            self.core.commands.rapid_jab_loop_end = false;
            self.change_motion_state(S::Attack100Start.into(), assets)?;
            self.step_animation(assets);
            self.core.state_data = MotionData::RapidJab(RapidJabState::default());
            return Ok(());
        }
        if jab.followup_window > 0.0 {
            jab.followup_window -= 1.0;
            if self.core.input.pressed.intersects(Buttons::A) {
                jab.followup_pressed = true;
            }
        }
        if jab.followup_pressed && self.core.commands.jab_followup {
            let edges = jab.rapid_edges;
            let state = if self.core.motion_state.id == S::Attack11 {
                S::Attack12
            } else {
                self.character.third_jab_state()
            };
            if state == S::Attack11 {
                // doAttack13 -> doAttack12Rapid -> checkAttack11: restart entry
                // includes ftAnim_8006EBA4 and the jab-2 window, unlike jab 2/3.
                return self.enter_jab(assets);
            }
            self.core.commands.jab_followup = false;
            self.change_motion_state(state.into(), assets)?;
            self.core.state_data = MotionData::Jab(JabState {
                followup_window: self.core.attributes.combat.jab_3_input_window,
                followup_pressed: false,
                rapid_edges: edges,
            });
            return Ok(());
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
    /// ftCo_AttackS3 decideAngle (8008B788): absent animation data rejects angles.
    fn side_tilt_state(&self, assets: &FighterAssets) -> S {
        let stick = self.input.current.stick;
        let angle = melee_lb::trigf::atan2f(stick.y, gekko_math::msl::fabsf(stick.x));
        let [high, high_middle, low_middle, low] = assets.attacks.tilt_angles;
        for (selected, state) in [
            (angle > high, S::AttackS3Hi),
            (angle > high_middle, S::AttackS3HiS),
            (angle < low, S::AttackS3Lw),
            (angle < low_middle, S::AttackS3LwS),
        ] {
            if selected && assets.motions.contains_key(&(state as i32 + 2)) {
                return state;
            }
        }
        S::AttackS3S
    }
    /// ftCo_AttackDash_Phys (8008B600): fixed friction, root-motion path shared with jab.
    pub(super) fn dash_attack_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: hsd_types::Vec3,
    ) {
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            return self.jab_physics(assets, map, wind);
        }
        use crate::physics::{
            friction::friction_acceleration,
            grounded::{self, GroundedParameters},
        };
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        // retail ftCo_AttackDash_Phys: fmuls, no fused instructions.
        let friction =
            assets.attacks.dash_friction_multiplier * self.attributes.ground.ground_friction;
        self.physics.ground_acceleration =
            friction_acceleration(self.physics.ground_velocity, friction);
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(&mut self.physics, &self.collision.data, &params, map, wind);
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

/// PlCo direction thresholds and dash-attack friction (ftCo_AttackS3/AttackDash).
pub struct AttackParameters {
    pub tilt_angles: [f32; 4],
    pub smash_angles: [f32; 4],
    pub dash_friction_multiplier: f32,
    pub charge_sound_frame: f32,
}
impl AttackParameters {
    pub fn read(archive: &hsd_archive::Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            tilt_angles: [
                r.f32(offset + 0x9C)?,
                r.f32(offset + 0xA0)?,
                r.f32(offset + 0xA4)?,
                r.f32(offset + 0xA8)?,
            ],
            smash_angles: [
                r.f32(offset + 0xB8)?,
                r.f32(offset + 0xBC)?,
                r.f32(offset + 0xC0)?,
                r.f32(offset + 0xC4)?,
            ],
            dash_friction_multiplier: r.f32(offset + 0x50)?,
            charge_sound_frame: r.f32(offset + 0x7C8)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RapidJabState {
    pub loop_started: bool,
    pub edge_pressed: bool,
}
