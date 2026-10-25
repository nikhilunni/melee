//! A throw that leaves the floor from its script: the thrower's airborne
//! arms (ftCo_Throw.c:191-492) and what they ask of the fighter it holds
//! (ftCo_Thrown.c:61-86, 206-224).
//!
//! The script's SetAirborne lifts the thrower, whose velocity is then its
//! TransN's. On the script's flag (cmd var 0) both fighters stop animating
//! and the thrower keeps the velocity it had; when it lands, both resume and
//! the throw plays out on the floor. Kirby's forward, back and up throws are
//! the only scripts that do any of it.
use super::{
    assets::{FighterAssets, Result},
    grab_escape::CaptureJumpState,
    state::CollisionPhase,
    Fighter, FighterCore, MotionData,
};
use hsd_types::Vec3;
use melee_types::{mp::line_flag, CommonMotionState as S, GroundOrAir};

/// What ftCommon asks a thrower's kind about one of its throws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThrowTraits {
    /// ftCo_800DE3FC (ftCo_Thrown.c:36): the thrown fighter keeps its grab
    /// timer and can mash out of the throw.
    pub victim_keeps_grab_timer: bool,
    /// ftCo_800DD398 (ftCo_Throw.c:146-150): neither fighter is tested
    /// against the blast zone (x2219_b2) until the script's flag
    /// (ftCo_Throw.c:287-292), and neither camera box follows its fighter
    /// (ftCo_ThrowHi_Cam, ftCo_ThrownHi_Cam).
    pub leaves_the_screen: bool,
}

/// Every kind's throws but Kirby's.
pub fn ordinary_throw(_throw: S) -> ThrowTraits {
    ThrowTraits::default()
}

/// mv.co.throw (fp+2344..2354), cleared by ftCo_800DD398.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThrowFlight {
    /// x4: stopped on the script's flag, until the landing.
    pub frozen: bool,
    /// x8: the throw has landed once; a later flight cannot land.
    pub landed: bool,
    /// xC: TransN's offset on the flag's frame, the velocity while frozen.
    pub offset: Vec3,
}

/// What a thrower's callback asked of the fighter it holds this proc; the
/// scene runs it on the pair, in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThrownRequest {
    /// ftCo_800DE920 with the thrower's animation frame.
    pub freeze_at: Option<f32>,
    /// ftCo_Throw.c:289-291: the held fighter's x2219_b2 clears too.
    pub clear_blast_exemption: bool,
    /// ftCo_800DE974.
    pub resume: bool,
}

/// How a thrown fighter leaves a throw it mashed out of (ftCo_Thrown.c:79-83).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    /// fn_800DC070: CaptureJump.
    Jump,
    /// ftCo_CaptureCut_Enter.
    Cut,
}

/// mv.co.thrown and the fighter fields a thrown fighter's callback reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThrownHold {
    /// mv.co.thrown.unk_bool: the thrower froze; stop at `freeze_frame`.
    pub held: bool,
    /// mv.co.thrown.anim_timer (mv+4): the frame to stop at, 0 once
    /// stopped. Until a freeze it is the word CaptureWait left there.
    pub freeze_frame: f32,
    /// fp->grab_timer (+1A4C): 0 unless the thrower's kind keeps it.
    pub timer: f32,
    /// x1A50, x1A51: the mash's last stick directions.
    pub stick_directions: [i8; 2],
    /// mv.co.thrown.xC, the byte CaptureWait latched a jump press in.
    pub jump_requested: bool,
    /// The thrower's [`ThrowTraits::leaves_the_screen`].
    pub leaves_the_screen: bool,
    /// The animation callback found the timer spent; the scene parts the
    /// pair (ftCo_800DC920 reads the thrower).
    pub escape: Option<Escape>,
}

impl ThrownHold {
    /// ftCo_800DE3FC (800DE3FC): what the thrown fighter carries out of its
    /// capture state `previous`.
    pub(super) fn entering(previous: &MotionData, traits: ThrowTraits) -> Self {
        let kept = match previous {
            MotionData::Capture(capture) => Some(capture.carried_into_throw()),
            _ => None,
        };
        let mut hold = Self {
            leaves_the_screen: traits.leaves_the_screen,
            ..Self::default()
        };
        if let Some(kept) = kept {
            hold.freeze_frame = kept.fast_remaining;
            hold.stick_directions = kept.stick_directions;
            hold.jump_requested = kept.jump_requested;
            if traits.victim_keeps_grab_timer {
                hold.timer = kept.timer;
            }
        } else if traits.victim_keeps_grab_timer {
            unimplemented!(
                "ftCo_800DE3FC (ftCo_Thrown.c:36): a kept grab timer out of a state that is \
                 not a capture wait"
            );
        }
        hold
    }
}

impl Fighter {
    /// ftCo_800DD724's tail (retail 0x800DD804, ftCo_Throw.c:279-293): the
    /// script's flag stops the thrower once per flight.
    pub(super) fn freeze_throw_on_flag(&mut self) {
        let leaves_the_screen =
            (self.character.table().throw_traits)(self.motion_state.id).leaves_the_screen;
        let core = &mut self.core;
        if core.commands.variables[0] == 0 || core.combat.throw_flight.frozen {
            return;
        }
        core.combat.throw_flight.frozen = true;
        core.commands.variables[0] = 0;
        core.combat.throw_flight.offset = core.root_motion_offset();
        core.animation.set_rate(&mut core.skeleton, 0.0, false);
        core.combat.thrown_request.freeze_at = Some(core.animation.frame);
        if leaves_the_screen {
            core.status.blast_zone_exempt = false;
            core.combat.thrown_request.clear_blast_exemption = true;
        }
    }

    /// ftCo_ThrowF_Coll (800DD990) and its siblings in the air.
    pub(super) fn throw_air_collision(&mut self, phase: CollisionPhase<'_>) -> Result<()> {
        let CollisionPhase { assets, map } = phase;
        let assets = assets.expect("an airborne throw's map assets");
        let flight = self.core.combat.throw_flight;
        if flight.frozen {
            // ft_80083C00(fn_800DD568).
            if self.core.stay_airborne(map, false) {
                self.land_frozen_throw(assets)?;
            }
        } else if !flight.landed {
            // ft_80083CE4(fn_800DD6E4, fn_800DD5EC).
            if self.core.stay_airborne(map, true) {
                self.land_throw_before_flag(assets)?;
            }
        } else {
            // ft_80083B68.
            self.core.stay_airborne(map, false);
        }
        Ok(())
    }

    /// fn_800DD568 (800DD568): the frozen thrower touched a floor. It
    /// animates again from the flag's frame, runs its animation callback at
    /// once and stands.
    fn land_frozen_throw(&mut self, assets: &FighterAssets) -> Result<()> {
        let core = &mut self.core;
        core.animation.set_rate(&mut core.skeleton, 1.0, false);
        self.step_animation(assets);
        self.run_throw_callback_in_landing(assets)?;
        self.finish_throw_landing();
        Ok(())
    }

    /// fn_800DD5EC (800DD5EC): the thrower touched a floor before the
    /// script's flag. Retail runs its animation forward to the flag, where
    /// its callback freezes both fighters, and resumes them at once. No
    /// flight over level ground comes down before its flag, and no scene or
    /// replay has reached this.
    fn land_throw_before_flag(&mut self, _assets: &FighterAssets) -> Result<()> {
        unimplemented!(
            "fn_800DD5EC (ftCo_Throw.c:207-223): an airborne throw landing before its \
             script's flag (the animation fast-forwarded to it)"
        )
    }

    /// `fp->anim_cb(gobj)` inside the landing: ftCo_ThrowF_Anim or a sibling.
    fn run_throw_callback_in_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        let state = self.motion_state.id;
        super::state::callbacks::animation::throw_callback(self, assets)?;
        if self.core.commands.grab_release {
            unimplemented!(
                "fn_800DD568 / fn_800DD5EC (ftCo_Throw.c:196-198, 213-215): the release \
                 from the callback a landing runs, before ftCommon_8007D7FC"
            );
        }
        if self.motion_state.id != state {
            unimplemented!(
                "fn_800DD568 / fn_800DD5EC (ftCo_Throw.c:196-198, 213-215): a throw \
                 ending in the callback a landing runs"
            );
        }
        Ok(())
    }

    /// The landings' shared tail: mv.co.throw.x4 = 0, x8 = 1,
    /// ftCo_800DE974 on a fighter still held, ftCommon_8007D7FC.
    fn finish_throw_landing(&mut self) {
        let core = &mut self.core;
        core.combat.throw_flight.frozen = false;
        core.combat.throw_flight.landed = true;
        core.combat.thrown_request.resume = true;
        core.land();
    }
}

impl FighterCore {
    /// x6A4_transNOffset.
    fn root_motion_offset(&self) -> Vec3 {
        self.animation
            .root_motion
            .as_ref()
            .expect("a throw's TransN")
            .primary_history
            .offset
    }

    /// ftCo_ThrowF_Phys (800DD930) and its siblings in the air: TransN's
    /// velocity (ft_80085134), or the frozen flight's. Retail 0x800DD960
    /// and 0x80085140: fmuls by the facing.
    pub(super) fn throw_air_velocity(&mut self) {
        let flight = self.combat.throw_flight;
        let offset = if flight.frozen {
            flight.offset
        } else {
            self.root_motion_offset()
        };
        self.physics.self_velocity.x = offset.z * self.physics.facing;
        self.physics.self_velocity.y = offset.y;
    }

    /// ft_80082578 (80082578) -> mpColl_800477E0, or ft_80083CE4's pass
    /// (mpColl_800476B4 with fn_800DD6E4, which refuses platforms): an
    /// airborne collision that never lands. True when it touched a floor
    /// (Collide_FloorMask). The item landing (ft_80081A00) is not modelled.
    fn stay_airborne(&mut self, map: &mut melee_mp::CollMap, refuse_platforms: bool) -> bool {
        crate::collision::air::begin_map(
            &self.physics,
            &mut self.collision,
            &mut self.skeleton,
            self.animation.root,
        );
        let cd = &mut self.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = self.physics.position;
        let pose =
            crate::collision::ecb::EcbPose::read(&mut self.skeleton, self.animation.root, cd);
        let bones = |bone| pose.position(bone);
        if refuse_platforms {
            map.air_collide_stay_pass(cd, Some(&mut not_a_platform), Some(&bones));
        } else {
            map.air_collide_stay(cd, Some(&bones));
        }
        self.physics.position = cd.cur_pos;
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        self.collision.data.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK != 0
    }

    /// ftCo_800DE920 (800DE920): the thrower froze on `frame`. A fighter
    /// already there stops now; one behind stops when it arrives.
    pub fn freeze_thrown(&mut self, frame: f32) {
        let hold = &mut self.combat.thrown_hold;
        hold.held = true;
        if self.animation.frame == frame {
            self.animation.set_rate(&mut self.skeleton, 0.0, false);
            hold.freeze_frame = 0.0;
        } else {
            hold.freeze_frame = frame;
        }
    }

    /// ftCo_800DE5A4 (800DE5A4), after the animation step: the hip follows
    /// the borrowed animation unless the thrower froze, and a kept grab
    /// timer runs down, faster for each mash input (PlCo +3C8).
    pub(super) fn thrown_hold_animation(&mut self, assets: &FighterAssets) {
        let hold = self.combat.thrown_hold;
        if !hold.held {
            self.thrown_animation(assets);
        } else if hold.freeze_frame != 0.0 && hold.freeze_frame == self.animation.frame {
            self.animation.set_rate(&mut self.skeleton, 0.0, false);
            self.combat.thrown_hold.freeze_frame = 0.0;
            self.thrown_animation(assets);
        }
        if hold.timer == 0.0 {
            return;
        }
        let p = &assets.grab_escape;
        let hold = &mut self.combat.thrown_hold;
        hold.timer -= p.decrement;
        super::capture_yoshi::grab_mash(
            &mut hold.timer,
            &mut hold.stick_directions,
            &self.input,
            p.stick_threshold,
            p.thrown_mash_decrement,
        );
        if hold.timer <= 0.0 {
            // fn_800DC044: the stick at the tap-jump height.
            let jump = hold.jump_requested
                || self.input.current.stick.y >= assets.common.input.tap_jump_threshold;
            hold.escape = Some(if jump { Escape::Jump } else { Escape::Cut });
        }
    }
}

/// fn_800DD6E4 (800DD6E4): a floor the early landing may take.
fn not_a_platform(map: &melee_mp::CollMap, line: i32) -> bool {
    line != -1 && map.line_get_flags(line) & line_flag::PLATFORM == 0
}

/// ftCo_800DE974 (800DE974): the thrower landed. The held fighter animates
/// again, one step at once.
pub fn resume_thrown(victim: &mut Fighter, assets: &FighterAssets) {
    victim.core.combat.thrown_hold.held = false;
    let core = &mut victim.core;
    core.animation.set_rate(&mut core.skeleton, 1.0, false);
    victim.step_animation(assets);
}

/// fn_800DC070 (800DC070) or ftCo_CaptureCut_Enter (800DC750) from
/// ftCo_800DE5A4: the thrown fighter mashed out. ftCo_800DC920 sets it down
/// where its XRotN points; the thrower stays in its throw with nobody.
pub fn escape_thrown(
    victim: &mut Fighter,
    thrower: &mut Fighter,
    va: &FighterAssets,
    thrower_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    escape: Escape,
) -> Result<()> {
    let p = &va.grab_escape;
    if escape == Escape::Jump {
        victim.leave_ground();
        victim.physics.self_velocity.x = -victim.physics.facing * p.air_escape_speed;
        victim.physics.self_velocity.y = p.air_escape_vertical_speed;
    }
    thrower.combat.grab = None;
    // ftCo_800DC920 reads the live RObj target here, after this animation
    // callback changed the owner. A clean owner still keeps its cached matrix.
    super::grab_throw::refresh_constraint(victim, thrower, va, thrower_assets);
    super::grab_damage::release_thrown(thrower, victim, va, map);
    victim.combat.grab = None;
    match escape {
        Escape::Jump => {
            let retained_drop_timer = victim.combat.thrown_hold.freeze_frame;
            victim.change_motion_state(S::CaptureJump.into(), va)?;
            victim.state_data = MotionData::CaptureJump(CaptureJumpState {
                frames: 0.0,
                retained_drop_timer,
            });
        }
        Escape::Cut => {
            let velocity = -victim.physics.facing * p.escape_speed;
            if victim.physics.ground_or_air == GroundOrAir::Ground {
                victim.physics.ground_velocity = velocity;
            } else {
                victim.physics.self_velocity.x = velocity;
            }
            victim.change_motion_state(S::CaptureCut.into(), va)?;
        }
    }
    Ok(())
}
