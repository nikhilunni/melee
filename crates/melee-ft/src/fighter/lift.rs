//! Carrying a heavy item (a barrel or crate): ftCo_Lift.c (80096D9C..
//! 8009750C) and the heavy throws' entry (ftCo_80094EA4, ftCo_800958FC).
//!
//! HeavyGet ends in LiftWait with the item over the head. From there the
//! stick walks (LiftWalk1 and LiftWalk2 alternate, one step each) or turns
//! (LiftTurn spins TransN half a turn), and A, B or the C-stick throws
//! (HeavyThrowF/B/Hi/Lw and their smash rows). Every one of these entries
//! installs ftCo_800974C4 as take_dmg_cb: a hit drops the item. Leaving
//! the ground drops it too and tumbles (ftCo_80096E68).
use super::{
    assets::{FighterAssets, Result},
    part_rotation::Axis,
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter, MotionData, MotionEntryFlags,
};
use crate::{anim::WaitChoice, collision::ground};
use gekko_math::{fma::fmadds, msl::fabsf};
use melee_types::{CommonMotionState as S, FtPart};

/// PlCo +228..+230 and ft_data->x4C_sfx->x2C.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiftParameters {
    /// PlCo +228: the forward stick that takes a step.
    pub walk_threshold: f32,
    /// PlCo +22C: the backward stick (negative) that turns.
    pub turn_threshold: f32,
    /// PlCo +230: the turn's length in frames.
    pub turn_frames: f32,
    /// ft_data->x4C_sfx->x2C: the lift's voice (ftpickupitem_800948A8).
    pub lift_sound: u32,
}

/// mv.co.lift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiftState {
    /// x0: the next step is LiftWalk2.
    pub second_step: bool,
    /// x4: the turn's remaining frames (mv+4 as the predecessor left it
    /// until a turn writes it; `None` where not modelled).
    pub turn_frames: Option<f32>,
    /// x8: the turn has flipped the facing.
    pub turned: bool,
}

/// Ft_MF_Unk24: LiftWalk's and LiftTurn's return to LiftWait.
const KEEP_GROUND_POSE: MotionEntryFlags = MotionEntryFlags::KEEP_GROUND_POSE;

fn lift(f: &mut Fighter) -> &mut LiftState {
    match &mut f.core.state_data {
        MotionData::Lift(lift) => lift,
        _ => panic!("mv.co.lift scratch"),
    }
}

impl Fighter {
    /// ftCo_80096D9C (80096D9C): LiftWait, the next step the first.
    pub(super) fn enter_lift_wait(&mut self, assets: &FighterAssets) -> Result<()> {
        let retained = self.inherited_scratch_word();
        self.change_motion_state(S::LiftWait.into(), assets)?;
        self.core.state_data = MotionData::Lift(LiftState {
            second_step: false,
            turn_frames: retained,
            turned: false,
        });
        self.core.heavy_item_damage_armed = true;
        Ok(())
    }

    /// LiftWalk_Anim's and LiftTurn_Anim's return to LiftWait
    /// (Ft_MF_Unk24, the scratch kept).
    fn return_to_lift_wait(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state_with_flags(
            S::LiftWait.into(),
            assets,
            KEEP_GROUND_POSE,
            0.0,
            1.0,
        )?;
        self.core.heavy_item_damage_armed = true;
        Ok(())
    }

    /// ftCo_80096EF8 (80096EF8): the stick forward past PlCo +228 takes a
    /// step (ftCo_80096F48).
    fn try_lift_step(&mut self, assets: &FighterAssets) -> Result<bool> {
        if self.core.input.current.stick.x * self.core.physics.facing < assets.lift.walk_threshold {
            return Ok(false);
        }
        // ftCo_80096F48: LiftWalk1 or 2 by x0, which flips; one step.
        let state = lift(self);
        let walk = if state.second_step {
            S::LiftWalk2
        } else {
            S::LiftWalk1
        };
        state.second_step = !state.second_step;
        self.change_motion_state(walk.into(), assets)?;
        self.step_animation(assets);
        self.core.heavy_item_damage_armed = true;
        Ok(true)
    }

    /// ftCo_800970E0 (800970E0): the stick back past PlCo +22C turns
    /// (ftCo_80097130).
    fn try_lift_turn(&mut self, assets: &FighterAssets) -> Result<bool> {
        if self.core.input.current.stick.x * self.core.physics.facing > assets.lift.turn_threshold {
            return Ok(false);
        }
        let state = lift(self);
        state.turn_frames = Some(assets.lift.turn_frames);
        state.turned = false;
        self.change_motion_state_with_flags(
            S::LiftTurn.into(),
            assets,
            KEEP_GROUND_POSE,
            0.0,
            1.0,
        )?;
        self.step_animation(assets);
        self.core.heavy_item_damage_armed = true;
        self.lift_turn_step(assets)?;
        Ok(true)
    }

    /// ftCo_LiftTurn_Anim (800971B0): TransN turns 180 / PlCo +230 degrees
    /// a frame; halfway the facing flips (TransN's angle mirrored, the
    /// ground speed negated); at the end, LiftWait.
    fn lift_turn_step(&mut self, assets: &FighterAssets) -> Result<()> {
        let frames = assets.lift.turn_frames;
        let part = usize::from(assets.parts.joint(FtPart::TransN).expect("TransN"));
        let joint = self.core.animation.parts[part].joint;
        let state = lift(self);
        let remaining = state.turn_frames.expect("LiftTurn's frames") - 1.0;
        state.turn_frames = Some(remaining);
        let turned = state.turned;
        // x2222_b6 is never set for a Vs fighter (fighter.c:785).
        let core = &mut self.core;
        // retail 80097238 fdivs, 8009723C fmadds (deg-to-rad, the step, the
        // current angle).
        let angle = fmadds(DEG_TO_RAD, 180.0 / frames, core.part_rotation_y(part));
        core.set_part_rotation(part, Axis::Y, angle);
        core.animation.blend_tree.set_rotation_y(joint, angle);
        // retail 800972F4: fmuls; the compare is <=.
        if !turned && remaining <= 0.5 * frames {
            let current = core.part_rotation_y(part);
            core.physics.facing = -core.physics.facing;
            // retail 80097330: double M_PI_2 times the facing, rounded once.
            let root = (std::f64::consts::FRAC_PI_2 * f64::from(core.physics.facing)) as f32;
            core.set_part_rotation(0, Axis::Y, root);
            core.set_part_rotation(part, Axis::Y, -current);
            core.animation.blend_tree.set_rotation_y(joint, -current);
            core.physics.ground_velocity = -core.physics.ground_velocity;
            lift(self).turned = true;
        }
        if remaining <= 0.0 {
            self.return_to_lift_wait(assets)?;
        }
        Ok(())
    }

    /// ftCo_80094EA4 (80094EA4): with the item in hand, A or B throws
    /// toward the left stick (its smash windows open), else the C-stick
    /// (smash windows at 0); a neutral A or B throws forward. Returns
    /// whether a throw began.
    fn try_heavy_throw(&mut self, assets: &FighterAssets) -> Result<bool> {
        use crate::input::Buttons;
        if self.core.held_item.is_none() {
            return Ok(false);
        }
        let input = &self.core.input;
        let common = &assets.input;
        let pressed = input.pressed.intersects(Buttons::A | Buttons::B);
        let (stick, horizontal_age, vertical_age) = if pressed {
            (
                input.current.stick,
                f32::from(input.horizontal.held),
                f32::from(input.vertical.held),
            )
        } else {
            (input.current.cstick, 0.0, 0.0)
        };
        // ftCo_GetLStickAngle / ftCo_GetCStickAngle: atan2f(y, |x|).
        let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
        let forward = stick.x * self.core.physics.facing >= 0.0;
        let state = if fabsf(stick.x) >= common.thresholds.dash_smash_stick_threshold
            && horizontal_age
                < common.thresholds.dash_smash_window as f32 + common.item_smash_window_extension
        {
            Some(if forward {
                S::HeavyThrowF4
            } else {
                S::HeavyThrowB4
            })
        } else if stick.y >= common.up_smash_threshold
            && vertical_age
                < common.up_smash_window + self.core.attributes.jumping.jump_startup_time
        {
            Some(S::HeavyThrowHi4)
        } else if stick.y <= common.down_smash_threshold && vertical_age < common.down_smash_window
        {
            Some(S::HeavyThrowLw4)
        } else if fabsf(stick.x) >= common.side_tilt_threshold && fabsf(angle) <= common.tilt_angle
        {
            Some(if forward {
                S::HeavyThrowF
            } else {
                S::HeavyThrowB
            })
        } else if stick.y >= common.up_tilt_threshold && angle > common.tilt_angle {
            Some(S::HeavyThrowHi)
        } else if stick.y <= common.down_tilt_threshold && angle < -common.tilt_angle {
            Some(S::HeavyThrowLw)
        } else if pressed {
            Some(S::HeavyThrowF)
        } else {
            None
        };
        // `msid != fp->motion_id`: no lift state is a throw row.
        let Some(state) = state else {
            return Ok(false);
        };
        self.enter_heavy_throw(state, assets)?;
        Ok(true)
    }

    /// ftCo_800958FC (800958FC): ftCo_800957F4's entry with take_dmg_cb
    /// ftCo_800974C4; a kind with x2222_b0 would take ftCo_800961D0 and
    /// ftCo_80096498 as its physics and collision.
    fn enter_heavy_throw(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        if self.carries_cargo() {
            unimplemented!("ftCo_800958FC (ftCo_ItemThrow.c:370): x2222_b0 heavy throw");
        }
        self.enter_item_throw_armed(state, assets, true)
    }

    /// ftCo_80096E68 (80096E68): off the ground, a held item drops where it
    /// is (Item_8026ABD8, no push, xC44 0) and the fighter tumbles
    /// (ftCo_80090780); an empty hand just falls.
    fn lift_leave_ground(&mut self, assets: &FighterAssets) -> Result<()> {
        if let Some(held) = self.core.held_item {
            self.core.drop_held_item(held, 0.0, assets);
            self.enter_damage_fall(assets)
        } else {
            // ftCo_Fall_Enter: airborne (ftCommon_8007D5D4), then Fall.
            self.leave_ground();
            self.change_motion_state(S::Fall.into(), assets)
        }
    }

    /// ft_8008403C (8008403C): ft_80082708, with ftCo_80096E68 off the floor.
    fn lift_ground_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        let c = &mut self.core;
        if ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ) != ground::WaitGroundResult::Supported
        {
            self.lift_leave_ground(assets)?;
        }
        Ok(())
    }

    /// ft_800841B8 (800841B8) with ftCo_80096E68: the heavy throws.
    fn heavy_throw_ground_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        let c = &mut self.core;
        if ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ) == ground::WaitGroundResult::EnterFall
        {
            self.lift_leave_ground(assets)?;
        }
        Ok(())
    }

    /// ftCo_800974C4 (800974C4), take_dmg_cb of HeavyGet, the lift states
    /// and the heavy throws: a held item drops (Item_8026ABD8, no push,
    /// xC44 0).
    pub(super) fn drop_lifted_item(&mut self, assets: &FighterAssets) {
        if let Some(held) = self.core.held_item {
            self.core.drop_held_item(held, 0.0, assets);
        }
    }
}

/// MTXDegToRad's factor (@338).
const DEG_TO_RAD: f32 = 0.017_453_292;

/// ftCo_LiftWait_Anim (80096E14) is empty.
pub fn wait_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftCo_LiftWait_IASA (80096E18): a throw, the turn, then a step.
pub fn wait_input(f: &mut Fighter, p: InputPhase<'_>) {
    let assets = p.assets;
    let entered = (|| -> Result<()> {
        if f.try_heavy_throw(assets)? || f.try_lift_turn(assets)? {
            return Ok(());
        }
        f.try_lift_step(assets)?;
        Ok(())
    })();
    entered.expect("lift assets");
}

/// ftCo_LiftWait_Coll / LiftWalk_Coll / LiftTurn_Coll: ft_8008403C with
/// ftCo_80096E68.
pub fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    f.lift_ground_collision(p.map, p.assets.expect("lift collision assets"))
}

/// ftCo_HeavyThrow_Coll (8009645C): ft_800841B8 with ftCo_80096E68.
pub fn throw_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    f.heavy_throw_ground_collision(p.map, p.assets.expect("heavy throw collision assets"))
}

/// ftCo_LiftWalk_Anim (80097014): at the step's end another step, else
/// LiftWait.
pub fn walk_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.core.animation.frames_remaining(&f.core.skeleton) && !f.try_lift_step(p.assets)? {
        f.return_to_lift_wait(p.assets)?;
    }
    Ok(None)
}

/// ftCo_LiftWalk_IASA / ftCo_LiftTurn_IASA: a throw only.
pub fn throw_input(f: &mut Fighter, p: InputPhase<'_>) {
    f.try_heavy_throw(p.assets).expect("lift assets");
}

/// ftCo_LiftWalk_Phys (800970BC) -> ft_80085088 -> ft_800850E0: with root
/// motion the ground speed is TransN's offset (the acceleration keeps its
/// last value), else ground friction; then ftCommon_ApplyGroundMovement.
pub fn walk_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    use crate::physics::grounded::{self, GroundedParameters};
    let c = &mut f.core;
    if c.animation.flags.contains(crate::anim::MotionFlags::ROOT_MOTION) {
        let offset = c
            .animation
            .root_motion
            .as_ref()
            .expect("root motion TransN")
            .primary_history
            .offset
            .z;
        // retail ft_800850E0: fmuls.
        c.physics.ground_velocity = offset * c.physics.facing;
    } else {
        c.physics.ground_acceleration = crate::physics::friction::friction_acceleration(
            c.physics.ground_velocity,
            c.attributes.ground.ground_friction,
        );
    }
    let terrain = grounded::ground_friction_multiplier(&c.physics, &c.collision.data);
    grounded::apply_ground_movement(&mut c.physics, c.collision.data.floor.normal, terrain);
    grounded::finish_ground_update(
        &mut c.physics,
        &c.collision.data,
        &GroundedParameters::from_attributes(&c.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// ftCo_LiftTurn_Anim (800971B0).
pub fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.lift_turn_step(p.assets)?;
    Ok(None)
}

/// The scratch's mv+4 for a following state that inherits it.
pub(super) fn retained_word(state: &LiftState) -> Option<f32> {
    state.turn_frames
}
