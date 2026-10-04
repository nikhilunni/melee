//! A cape's turnaround (HitElement_Cape), ftCo_0C35.c: instead of a
//! reaction, the victim freezes for PlCo +648 frames (dmg.x1954, beside
//! ordinary hitlag) while its TransN spins half a turn, is pushed along the
//! hit's direction, and faces the other way at the end.
use super::{assets::FighterAssets, Fighter, FighterCore};
use melee_types::GroundOrAir;

/// PlCo +648..+654.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapeTurnParameters {
    /// +648: the turn's frames.
    pub frames: i32,
    /// +64C: the push speed of an airborne victim.
    pub air_speed: f32,
    /// +650: the push speed of a grounded victim.
    pub ground_speed: f32,
    /// +654: the push speed of a shielding victim.
    pub shield_speed: f32,
}

/// fp->x21F8: what the motion state does when a turn ends
/// (ftCo_800C37A0), installed by its entry and cleared by every motion
/// change (fighter.c:1388).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapeTurnEnd {
    /// ftCommon_8007F76C: ground and self X speed along the new facing.
    SpeedForward,
    /// ftCommon_8007F7B4: ground and self X speed against the new facing.
    SpeedBackward,
    /// A character's own function, through its `CAPE_TURN_END` hook.
    Character,
}

/// dmg.x18F4 / x1954 / x2220_b4: the turn in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapeTurn {
    /// dmg.x18F4: turn frames left.
    pub frames: i32,
    /// dmg.x1954: freeze frames left, counted like hitlag.
    pub freeze: f32,
    /// x2220_b4.
    pub turning: bool,
    /// PlCo +648 and the TransN part, captured when the turn starts.
    pub total: i32,
    pub trans_n: usize,
    /// fp->x21F8.
    pub on_end: Option<CapeTurnEnd>,
}

impl FighterCore {
    /// inlineA0 (ftCo_0C35.c:15-25): the turn and its freeze start, and
    /// Fighter_UnkRecursiveFunc_8006D044 enters hitlag (x2219_b5).
    fn begin_cape_turn(&mut self, parameters: &CapeTurnParameters, assets: &FighterAssets) {
        let turn = &mut self.combat.cape_turn;
        turn.total = parameters.frames;
        turn.trans_n = usize::from(
            assets
                .parts
                .joint(melee_types::FtPart::TransN)
                .expect("TransN part"),
        );
        turn.frames = parameters.frames;
        if turn.frames as f32 > turn.freeze {
            turn.freeze = turn.frames as f32;
        }
        turn.turning = true;
    }

    /// ftCo_800C3598 (800C3598): the turn, then the push along the hit's
    /// launch angle (ftCo_Damage_CalcVel), and ftCommon_800804FC: turned
    /// on the ground, the victim's KO credit goes.
    pub(super) fn cape_turn(
        &mut self,
        hit: &melee_coll::damage::ReceivedHit,
        knockback: f32,
        assets: &FighterAssets,
    ) {
        let parameters = assets.damage.cape_turn;
        self.begin_cape_turn(&parameters, assets);
        let airborne = self.physics.ground_or_air == GroundOrAir::Air;
        let speed = if airborne {
            parameters.air_speed
        } else {
            parameters.ground_speed
        };
        let angle =
            assets
                .damage
                .launch_angle(hit.descriptor.angle, knockback, self.physics.ground_or_air);
        // 800C3600..: separate fmuls.
        let x = speed * gekko_math::msl::cosf(angle);
        let y = speed * gekko_math::msl::sinf(angle);
        if airborne {
            self.combine_knockback(x * hit.facing, y, assets);
            self.physics.ground_knockback_velocity = 0.0;
        } else {
            let push = x * hit.facing;
            self.physics.ground_knockback_velocity = push;
            let normal = self.collision.data.floor.normal;
            self.combine_knockback(normal.y * push, -normal.x * push, assets);
        }
        self.combat
            .ko_source
            .clear_if_grounded(self.physics.ground_or_air);
    }

    /// ftCo_800C36DC (800C36DC): a cape against a shield turns the
    /// defender and pushes it along the shield hit's direction.
    pub(super) fn shield_cape_turn(&mut self, direction: f32, assets: &FighterAssets) {
        let parameters = assets.damage.cape_turn;
        self.begin_cape_turn(&parameters, assets);
        let push = parameters.shield_speed * -direction;
        self.physics.ground_knockback_velocity = push;
        let normal = self.collision.data.floor.normal;
        self.combine_knockback(normal.y * push, -normal.x * push, assets);
    }

    /// Fighter_ChangeMotionState, fighter.c:1016-1019: a motion change ends
    /// the turn (its freeze runs on).
    pub(super) fn cancel_cape_turn(&mut self) {
        let turn = &mut self.combat.cape_turn;
        if turn.frames != 0 {
            turn.frames = 0;
            turn.turning = false;
        }
    }

    /// Fighter_8006A1BC's x1954 countdown (fighter.c:1398-1405): true when
    /// the freeze ends this frame with no hitlag left.
    pub(super) fn tick_cape_freeze(&mut self) -> bool {
        let turn = &mut self.combat.cape_turn;
        if turn.freeze > 0.0 {
            turn.freeze -= 1.0;
            if turn.freeze <= 0.0 {
                turn.freeze = 0.0;
                return self.combat.hitlag_remaining <= 0.0;
            }
        }
        false
    }

    /// ftCo_800C37A0 (800C37A0): TransN turns pi * elapsed / frames about Y
    /// (800C383C..54: the int differences as singles, the product and
    /// quotient in double, rounded once); at the end the fighter faces the
    /// other way with TransN straight again. Returns whether the turn ended
    /// with a character's x21F8 to run.
    pub(super) fn step_cape_turn(&mut self) -> bool {
        let turn = &mut self.combat.cape_turn;
        let total = turn.total;
        if turn.frames == 0 {
            return false;
        }
        let elapsed = (total - turn.frames) as f32;
        let angle = ((std::f64::consts::PI * f64::from(elapsed)) / f64::from(total as f32)) as f32;
        turn.frames -= 1;
        let finished = turn.frames == 0;
        let part = turn.trans_n;
        self.set_part_rotation(part, super::part_rotation::Axis::Y, angle);
        let joint = self.animation.parts[part].joint;
        self.animation.blend_tree.set_rotation_y(joint, angle);
        if finished {
            self.combat.cape_turn.turning = false;
            self.physics.facing = -self.physics.facing;
            let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(self.physics.facing)) as f32;
            self.set_part_rotation(0, super::part_rotation::Axis::Y, rotation);
            self.set_part_rotation(part, super::part_rotation::Axis::Y, 0.0);
            self.animation.blend_tree.set_rotation_y(joint, 0.0);
            match self.combat.cape_turn.on_end {
                Some(CapeTurnEnd::SpeedForward) => self.orient_speed(self.physics.facing),
                Some(CapeTurnEnd::SpeedBackward) => self.orient_speed(-self.physics.facing),
                Some(CapeTurnEnd::Character) => return true,
                None => {}
            }
        }
        false
    }

    /// ftCommon_8007F76C / ftCommon_8007F7B4: gr_vel and self_vel.x keep
    /// their magnitudes and take `direction`'s sign.
    fn orient_speed(&mut self, direction: f32) {
        let physics = &mut self.physics;
        physics.ground_velocity = direction * physics.ground_velocity.abs();
        physics.self_velocity.x = direction * physics.self_velocity.x.abs();
    }

    /// Fighter_ChangeMotionState (fighter.c:1388): x21F8 = NULL.
    pub(super) fn clear_cape_turn_end(&mut self) {
        self.combat.cape_turn.on_end = None;
    }
}

impl Fighter {
    /// Fighter_8006A360's status step (`FighterCore::proc_status`), with a
    /// character's x21F8 when ftCo_800C37A0 ends a turn in its state.
    pub fn proc_status(&mut self) {
        if self.core.proc_status() {
            (self.character.table().cape_turn_end)(self);
        }
    }

    /// Installs fp->x21F8 after the motion change that cleared it.
    pub fn set_cape_turn_end(&mut self, end: CapeTurnEnd) {
        self.core.combat.cape_turn.on_end = Some(end);
    }

    /// ftCo_800C3538 (800C3538) returns false: Ft_MF bits 20-23
    /// (x2071_b0_3) of 9..11 (the cliff, catch/throw and capture rows) or
    /// x2222_b2 (the character's hook). The victim then takes the hit's
    /// ordinary reaction with its facing kept, and turns afterwards.
    pub(super) fn cape_turn_blocked(&mut self) -> bool {
        let action = self.motion_state.action.0;
        matches!(action, 212..=232 | 239..=243 | 252..=263 | 266..=339)
            || (self.character.table().cape_turn_blocked)(self)
    }
}
