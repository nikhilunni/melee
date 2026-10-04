//! The cargo carry: ftCo_CargoWait.c .. ftCo_CargoThrow.c and ftCo_09C4.c
//! for the carrier, ftCo_Shouldered.c for the fighter on its shoulder.
//!
//! A kind whose OnLoad sets x2222_b0 and x2CC (Donkey Kong) ends its forward
//! throw holding the victim (`Capabilities::cargo`). The carrier's rows live
//! in its own table at `CargoCarry::first_state`, in retail's order; their
//! callbacks are the common ones here. Each of the carrier's state changes
//! also moves the victim, which the scene applies right after the carrier's
//! proc (`CargoRequest`); the victim's mashing ends the carry from its own
//! animation callback (`shoulder_escape`).
use super::{
    assets::{FighterAssets, Result},
    grab_throw,
    jump::JumpInput,
    state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    ActionId, Fighter, FighterCore, MotionData, MotionEntryFlags,
};
use crate::{
    anim::WaitChoice,
    collision::{air, ground},
    input::Buttons,
};
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::{fabsf, fctiwz},
};
use melee_types::{CommonMotionState as S, GroundOrAir};

/// Fighter.x2CC as the cargo states read it (ftDonkeyAttributes), present
/// when the kind's OnLoad set x2222_b0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CargoCarry {
    /// x4_motion_state: the carrier's first cargo row (CargoWait); the rest
    /// follow at the `row` offsets.
    pub first_state: u16,
    /// The animations whose lengths OnLoad stores in x8, xC and x10 (the
    /// slow, middle and fast carry walks' phase lengths).
    pub walk_length_animations: [i32; 3],
    /// x14, x18, x1C: the ground speed each carry walk plays at rate 1.
    pub walk_rates: [f32; 3],
    /// cargo_hold.x20_TURN_SPEED: frames before the turn flips the facing.
    pub turn_frames: f32,
    /// cargo_hold.x24_JUMP_STARTUP_LAG: frames in the carry's jump squat.
    pub jump_squat_frames: f32,
    /// cargo_hold.x28_LANDING_LAG: frames in the carry's landing.
    pub landing_frames: f32,
}

/// Row offsets from `CargoCarry::first_state`, as retail adds them.
pub mod row {
    pub const WAIT: u16 = 0;
    pub const WALK: u16 = 1;
    pub const TURN: u16 = 4;
    pub const KNEE_BEND: u16 = 5;
    pub const FALL: u16 = 6;
    pub const JUMP: u16 = 7;
    pub const LANDING: u16 = 8;
    /// The four grounded throws; their aerial rows follow four later.
    pub const THROW: u16 = 10;
    pub const AIR_THROW_OFFSET: u16 = 4;
}

/// The carrier's motion scratch in the rows that have one of their own (the
/// walks use `MotionData::Walk`, the turn `MotionData::Turn`).
#[derive(Clone, Debug)]
pub enum CargoState {
    /// CargoWait and CargoFall write nothing.
    Carry,
    /// mv.co.cargokneebend: the hop decision, the jump's input and the
    /// frames left.
    KneeBend {
        short_hop: bool,
        input: JumpInput,
        frames: f32,
    },
    /// mv.co.jump: the hop decision kept from the squat, and whether the
    /// first physics frame has passed (ftCo_Jump_Phys_Inner).
    Jump {
        short_hop: bool,
        physics_started: bool,
    },
    /// The landing's frames left.
    Landing { frames: f32 },
    /// A cargo throw: facing_dir1, the facing the row was entered with.
    Throw { entry_facing: f32 },
}

/// What a carrier's state change does to the fighter it carries; the scene
/// applies it with both fighters (`apply`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CargoRequest {
    /// ftCo_8009C5A4: the shouldered row matching the carrier's. `begin`:
    /// ftCo_8009C640, the carry starts and with it the mash timer.
    Shoulder { state: S, begin: bool },
    /// ftCo_CargoTurn_Anim: the victim takes the carrier's new facing.
    Face,
    /// ftCo_8009C02C: ftCo_800DE3FC into ThrownFF + `direction`.
    Throw { direction: u16 },
}
pub type CargoRequests = melee_types::fixed::FixedVec<CargoRequest, 4>;

/// The shouldered fighter's state: the grab timer and mash latches
/// (grab_timer, x1A50/x1A51) and mv.co.shouldered.
#[derive(Clone, Debug)]
pub struct ShoulderedState {
    pub timer: f32,
    /// mv.co.shouldered.x0: frames left of the struggling animation rate.
    pub fast_remaining: f32,
    stick_directions: [i8; 2],
}

/// PlCo +4A0..+4AC: the carry's mash timer and decrements.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +4A0: timer frames per percent.
    pub percent_scale: f32,
    /// +4A4: the timer's base.
    pub base_timer: f32,
    /// +4A8: what each mash input takes off.
    pub mash_decrement: f32,
    /// +4AC: the decrement's scale while the carrier is airborne.
    pub air_mash_scale: f32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            percent_scale: r.f32(base + 0x4A0)?,
            base_timer: r.f32(base + 0x4A4)?,
            mash_decrement: r.f32(base + 0x4A8)?,
            air_mash_scale: r.f32(base + 0x4AC)?,
        })
    }
}

/// ftCo_SM_ShoulderedWait..ShoulderedTurn and ftCo_SM_ThrownFF..ThrownFLw
/// (ftData_MotionStateList[266..274]): the carrier's animations the victim
/// plays.
pub const VICTIM_MOTIONS: [i32; 9] = [267, 268, 269, 270, 271, 272, 273, 274, 275];

fn victim_motion(state: S) -> i32 {
    VICTIM_MOTIONS[state as usize - S::ShoulderedWait as usize]
}

fn carry(f: &Fighter) -> CargoCarry {
    f.core
        .capabilities
        .cargo
        .expect("a cargo row on a kind without x2222_b0")
}

fn cargo_row(f: &Fighter, offset: u16) -> ActionId {
    ActionId(carry(f).first_state + offset)
}

fn request(f: &mut Fighter, request: CargoRequest) {
    f.core.combat.cargo_requests.push(request);
}

fn shoulder(f: &mut Fighter, state: S) {
    request(
        f,
        CargoRequest::Shoulder {
            state,
            begin: false,
        },
    );
}

impl Fighter {
    /// ftCo_ThrowF_Anim (800DD7DC), at the forward throw's end: a kind with
    /// x2222_b0 keeps the victim on its shoulder (ftCo_8009B56C).
    pub(super) fn carries_cargo(&self) -> bool {
        self.core.capabilities.cargo.is_some()
    }

    /// ftCo_8009B518 (8009B518) / ftCo_8009B56C (8009B56C): CargoWait, with
    /// the victim in ShoulderedWait; `begin` starts its mash timer.
    pub(super) fn enter_cargo_wait(&mut self, assets: &FighterAssets, begin: bool) -> Result<()> {
        let state = cargo_row(self, row::WAIT);
        self.change_motion_state_with_flags(state, assets, MotionEntryFlags(0), 0.0, 1.0)?;
        self.core.state_data = MotionData::Cargo(CargoState::Carry);
        request(
            self,
            CargoRequest::Shoulder {
                state: S::ShoulderedWait,
                begin,
            },
        );
        Ok(())
    }

    /// ftCo_8009B6C8 (8009B6C8): ftWalkCommon_800DFCA4 on the carry walks
    /// with accel_mul 1, then the victim's row. Retail adds the walk type
    /// to ShoulderedWait, so the slow walk keeps the victim in
    /// ShoulderedWait and the fast one plays ShoulderedWalkMiddle.
    fn enter_cargo_walk(&mut self, assets: &FighterAssets, frame: f32) -> Result<()> {
        let base = carry(self).first_state + row::WALK;
        let tier = super::walk::walk_tier(
            self.core.physics.ground_velocity,
            self.core.attributes.walking.walk_max_vel,
            1.0,
            &assets.movement,
        );
        self.change_motion_state_with_flags(
            ActionId(base + tier),
            assets,
            MotionEntryFlags(0),
            frame,
            1.0,
        )?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Walk(super::walk::WalkState {
            slippery_animation_velocity: self.core.physics.ground_velocity,
            base_motion: i32::from(base),
            acceleration_multiplier: 1.0,
        });
        let victim = [
            S::ShoulderedWait,
            S::ShoulderedWalkSlow,
            S::ShoulderedWalkMiddle,
        ];
        shoulder(self, victim[usize::from(tier)]);
        Ok(())
    }

    /// ftCo_8009B860 (8009B860): ftCo_Turn_Enter on the carry turn with
    /// x20 frames before the flip.
    fn enter_cargo_turn(&mut self, assets: &FighterAssets) -> Result<()> {
        let carry = carry(self);
        self.core.state_data = MotionData::Turn(super::turn::TurnState {
            has_turned: false,
            just_turned: false,
            facing_after: -self.core.physics.facing,
            dash_direction: 0.0,
            frames_to_turn: carry.turn_frames,
            buffered_buttons: Buttons(0),
        });
        let turn = self.core.state_data.clone();
        self.change_motion_state_with_flags(
            ActionId(carry.first_state + row::TURN),
            assets,
            MotionEntryFlags(0),
            0.0,
            1.0,
        )?;
        self.core.state_data = turn;
        self.step_animation(assets);
        shoulder(self, S::ShoulderedTurn);
        Ok(())
    }

    /// ftCo_8009B9C8 (8009B9C8): the carry's jump squat, its animation held
    /// at frame 0 for x24 frames.
    fn enter_cargo_knee_bend(&mut self, assets: &FighterAssets, input: JumpInput) -> Result<()> {
        let carry = carry(self);
        self.change_motion_state_with_flags(
            ActionId(carry.first_state + row::KNEE_BEND),
            assets,
            MotionEntryFlags(0),
            0.0,
            1.0,
        )?;
        self.core.state_data = MotionData::Cargo(CargoState::KneeBend {
            short_hop: false,
            input,
            frames: carry.jump_squat_frames,
        });
        self.hold_animation();
        shoulder(self, S::ShoulderedWait);
        Ok(())
    }

    /// ftCo_8009BB64 (8009BB64): off the ground (ftCommon_8007D5D4), the
    /// carry jump held at frame 0, and ftCo_800CB110(gobj, true, 1.0)'s
    /// take-off velocity.
    fn enter_cargo_jump(&mut self, assets: &FighterAssets, short_hop: bool) -> Result<()> {
        self.leave_ground();
        let state = cargo_row(self, row::JUMP);
        self.change_motion_state_with_flags(state, assets, MotionEntryFlags(0), 0.0, 1.0)?;
        self.hold_animation();
        // ftCo_800CB110 with jump_mul 1 (retail 800CB140..800CB1B8: every
        // product and the sum rounded separately).
        let attrs = &self.core.attributes.jumping;
        let multiplier = 1.0;
        let momentum = self.core.physics.self_velocity.x
            * (attrs.ground_to_air_jump_momentum_multiplier * multiplier);
        let horizontal = momentum
            + multiplier * (self.core.input.current.stick.x * attrs.jump_h_initial_velocity);
        let maximum = attrs.jump_h_max_velocity * multiplier;
        let vertical = if short_hop {
            attrs.hop_v_initial_velocity
        } else {
            attrs.jump_v_initial_velocity
        } * multiplier;
        self.core.physics.self_velocity =
            hsd_types::Vec3::new(horizontal.clamp(-maximum, maximum), vertical, 0.0);
        self.core.input.vertical.tilt = 0xFE;
        self.core.state_data = MotionData::Cargo(CargoState::Jump {
            short_hop,
            physics_started: false,
        });
        shoulder(self, S::ShoulderedWait);
        Ok(())
    }

    /// ftCo_8009BC58 (8009BC58): the carry fall (Ft_MF_KeepFastFall), held
    /// at frame 0; a grounded carrier leaves the floor after the change.
    fn enter_cargo_fall(&mut self, assets: &FighterAssets) -> Result<()> {
        let state = cargo_row(self, row::FALL);
        self.change_motion_state_with_flags(
            state,
            assets,
            MotionEntryFlags::KEEP_FAST_FALL,
            0.0,
            1.0,
        )?;
        self.hold_animation();
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.leave_ground();
        }
        self.core.state_data = MotionData::Cargo(CargoState::Carry);
        shoulder(self, S::ShoulderedWait);
        Ok(())
    }

    /// ftCo_8009C540 (8009C540): ftCo_8009A184 drops through the platform
    /// into the carry fall.
    fn enter_cargo_pass(&mut self, assets: &FighterAssets) -> Result<()> {
        self.leave_ground();
        let maximum = self.core.attributes.air.air_drift_max;
        self.core.physics.self_velocity.x =
            self.core.physics.self_velocity.x.clamp(-maximum, maximum);
        self.core.physics.self_velocity.y = assets.movement.platform_drop_velocity;
        let state = cargo_row(self, row::FALL);
        self.change_motion_state_with_flags(state, assets, MotionEntryFlags(0), 0.0, 1.0)?;
        melee_mp::update_floor_skip(&mut self.core.collision.data);
        self.core.input.vertical.tilt = 0xFE;
        self.hold_animation();
        self.core.state_data = MotionData::Cargo(CargoState::Carry);
        shoulder(self, S::ShoulderedWait);
        Ok(())
    }

    /// ftCo_8009BD4C (8009BD4C): ftCo_Landing_Enter on the carry landing,
    /// held at frame 0 for x28 frames.
    fn enter_cargo_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        let carry = carry(self);
        self.land();
        self.change_motion_state_with_flags(
            ActionId(carry.first_state + row::LANDING),
            assets,
            MotionEntryFlags(0),
            0.0,
            1.0,
        )?;
        // ftCo_Landing_Enter's kind switch (allow_interrupt true).
        self.character.on_landing(true);
        (self.character.table().landing_articles)(self, true);
        self.hold_animation();
        self.core.state_data = MotionData::Cargo(CargoState::Landing {
            frames: carry.landing_frames,
        });
        shoulder(self, S::ShoulderedWait);
        Ok(())
    }

    /// ftAnim_SetAnimRate(gobj, 0).
    fn hold_animation(&mut self) {
        self.core
            .animation
            .set_rate(&mut self.core.skeleton, 0.0, false);
    }

    /// ftCo_8009BF3C (8009BF3C): A or B with a direction throws: past PlCo
    /// +98 sideways, forward or back by the facing; else up past +AC or
    /// down past +B0. A press with a neutral stick does nothing.
    fn try_cargo_throw(&mut self, assets: &FighterAssets) -> Result<bool> {
        if !self.core.input.pressed.intersects(Buttons::A | Buttons::B) {
            return Ok(false);
        }
        let stick = self.core.input.current.stick;
        let direction = if fabsf(stick.x) >= assets.input.side_tilt_threshold {
            if stick.x * self.core.physics.facing > 0.0 {
                0
            } else {
                1
            }
        } else if stick.y >= assets.input.up_tilt_threshold {
            2
        } else if stick.y <= assets.input.down_tilt_threshold {
            3
        } else {
            return Ok(false);
        };
        self.enter_cargo_throw(assets, direction)?;
        Ok(true)
    }

    /// ftCo_8009C02C (8009C02C): the throw's row (its aerial twin with
    /// Ft_MF_KeepFastFall in the air), every grab category excluded, and
    /// the victim into the matching ThrownF row.
    fn enter_cargo_throw(&mut self, assets: &FighterAssets, direction: u16) -> Result<()> {
        let airborne = self.core.physics.ground_or_air == GroundOrAir::Air;
        let (offset, flags) = if airborne {
            (row::AIR_THROW_OFFSET, MotionEntryFlags::KEEP_FAST_FALL)
        } else {
            (0, MotionEntryFlags(0))
        };
        let state = cargo_row(self, row::THROW + direction + offset);
        self.change_motion_state_with_flags(state, assets, flags, 0.0, 1.0)?;
        self.step_animation(assets);
        self.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        self.core.state_data = MotionData::Cargo(CargoState::Throw {
            entry_facing: self.core.physics.facing,
        });
        request(self, CargoRequest::Throw { direction });
        Ok(())
    }

    /// ftCo_8009BB1C (8009BB1C): ftCo_Jump_GetInput, then the carry's squat.
    fn try_cargo_jump(&mut self, assets: &FighterAssets) -> Result<bool> {
        let input = &self.core.input;
        let thresholds = &assets.input.thresholds;
        let jump = if input.current.stick.y >= thresholds.tap_jump_threshold
            && i32::from(input.vertical.tilt) < thresholds.tap_jump_window
        {
            JumpInput::Stick
        } else if input.pressed.intersects(Buttons::XY) {
            JumpInput::Buttons
        } else {
            return Ok(false);
        };
        self.enter_cargo_knee_bend(assets, jump)?;
        Ok(true)
    }

    /// ftCo_8009C4F8 (8009C4F8): ftCo_80099F1C, a flick down on a platform.
    fn try_cargo_pass(&mut self, assets: &FighterAssets) -> Result<bool> {
        let on_platform =
            self.core.collision.data.floor.flags & melee_types::mp::line_flag::PLATFORM != 0;
        if self.core.input.current.stick.y <= -assets.movement.platform_drop_threshold
            && f32::from(self.core.input.vertical.tilt) < assets.movement.platform_drop_window
            && on_platform
        {
            self.enter_cargo_pass(assets)?;
            return Ok(true);
        }
        Ok(false)
    }
}

/// ftCo_CargoWait_Anim (8009B5B8) is empty.
pub fn wait_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftCo_CargoWait_IASA (8009B5BC): a throw, the jump, a platform drop, the
/// turn (ftCo_800C97A8), the walk (ftWalkCommon_800DFC70).
pub fn wait_input(f: &mut Fighter, p: InputPhase<'_>) {
    let assets = p.assets;
    let entered = (|| -> Result<()> {
        if f.try_cargo_throw(assets)? || f.try_cargo_jump(assets)? || f.try_cargo_pass(assets)? {
            return Ok(());
        }
        let along = f.core.input.current.stick.x * f.core.physics.facing;
        let thresholds = &assets.input.thresholds;
        if along <= thresholds.turn_stick_threshold {
            f.enter_cargo_turn(assets)
        } else if along >= thresholds.walk_stick_threshold {
            f.enter_cargo_walk(assets, 0.0)
        } else {
            Ok(())
        }
    })();
    entered.expect("cargo carry assets");
}

/// ftCo_CargoWait_Coll (8009B664) and the other grounded rows: off the
/// floor (ft_8008403C -> ft_80082708), the carry fall.
pub fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !stays_grounded(f, p.map) {
        f.enter_cargo_fall(p.assets.expect("cargo carry collision assets"))?;
    }
    Ok(())
}

/// ft_80082708 (80082708): true while the floor holds.
fn stays_grounded(f: &mut Fighter, map: &mut melee_mp::CollMap) -> bool {
    let c = &mut f.core;
    ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_80081D0C (80081D0C): true on landing.
fn lands(f: &mut Fighter, map: &mut melee_mp::CollMap) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        map,
        &mut c.skeleton,
        c.animation.root,
    )
}

/// ftCo_CargoWalk_Anim (8009B768) -> ftWalkCommon_800DFDDC: the carry
/// walks' own speeds set the animation rate.
pub fn walk_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let carry = carry(f);
    let c = &mut f.core;
    let MotionData::Walk(walk) = &c.state_data else {
        panic!("cargo walk data missing")
    };
    let speed = if crate::physics::grounded::floor_friction(&c.collision.data) < 1.0 {
        walk.slippery_animation_velocity
    } else {
        c.physics.ground_velocity
    };
    let rate = if speed * c.physics.facing <= 0.0 {
        0.0
    } else {
        let tier = i32::from(c.motion_state.action.0) - walk.base_motion;
        fabsf(speed) / carry.walk_rates[tier as usize]
    };
    c.animation.set_rate(&mut c.skeleton, rate, false);
    Ok(None)
}

/// ftCo_CargoWalk_IASA (8009B788): a throw, the jump, a platform drop, the
/// wait (ft_8008A1FC), else ftWalkCommon_800DFEC8: a new speed tier
/// re-enters the walk at the same phase of the new animation.
pub fn walk_input(f: &mut Fighter, p: InputPhase<'_>) {
    let assets = p.assets;
    let entered = (|| -> Result<()> {
        if f.try_cargo_throw(assets)? || f.try_cargo_jump(assets)? || f.try_cargo_pass(assets)? {
            return Ok(());
        }
        let stick = f.core.input.current.stick.x;
        if stick * f.core.physics.facing < 0.0
            || fabsf(stick) < assets.input.thresholds.walk_stick_threshold
        {
            return f.enter_cargo_wait(assets, false);
        }
        let carry = carry(f);
        let MotionData::Walk(walk) = &f.core.state_data else {
            panic!("cargo walk data missing")
        };
        let tier = super::walk::walk_tier(
            f.core.physics.ground_velocity,
            f.core.attributes.walking.walk_max_vel,
            walk.acceleration_multiplier,
            &assets.movement,
        );
        if walk.base_motion + i32::from(tier) == i32::from(f.core.motion_state.action.0) {
            return Ok(());
        }
        let duration = assets.motions[&f.core.animation.motion_id].animation.frames;
        let target = assets.motions[&carry.walk_length_animations[usize::from(tier)]]
            .animation
            .frames;
        let quotient = fctiwz(f.core.animation.frame / duration) as f32;
        // retail 0x800E0010: fnmsubs.
        let phase = fnmsubs(duration, quotient, f.core.animation.frame);
        let frame = fctiwz(target * (phase / duration)) as f32;
        f.enter_cargo_walk(assets, frame)
    })();
    entered.expect("cargo carry assets");
}

/// ftCo_CargoTurn_Anim (8009B8D4): after x20 frames the facing flips, the
/// victim's with it; at the animation's end, the wait.
pub fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let MotionData::Turn(turn) = &mut f.core.state_data else {
        panic!("cargo turn data missing")
    };
    if turn.frames_to_turn > 0.0 {
        turn.frames_to_turn -= 1.0;
    } else if !turn.has_turned {
        turn.has_turned = true;
        f.core.physics.facing = -f.core.physics.facing;
        request(f, CargoRequest::Face);
    }
    if !f.core.animation.frames_remaining(&f.core.skeleton) {
        f.enter_cargo_wait(p.assets, false)?;
    }
    Ok(None)
}

/// ftCo_CargoTurn_IASA (8009B974): a throw, then the jump.
pub fn turn_input(f: &mut Fighter, p: InputPhase<'_>) {
    let assets = p.assets;
    let entered = (|| -> Result<()> {
        if !f.try_cargo_throw(assets)? {
            f.try_cargo_jump(assets)?;
        }
        Ok(())
    })();
    entered.expect("cargo carry assets");
}

/// ftCo_CargoKneebend_Anim (8009BA50): the jump once the frames run out
/// (checked before the decrement).
pub fn knee_bend_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let MotionData::Cargo(CargoState::KneeBend {
        short_hop, frames, ..
    }) = &mut f.core.state_data
    else {
        panic!("cargo jump squat data missing")
    };
    if *frames <= 0.0 {
        let short_hop = *short_hop;
        // The decrement after ftCo_8009BB64 lands in the jump's scratch
        // (mv+8 is mv.co.jump.jump_mul there, which only ftCo_800CB110
        // read, before it).
        return f.enter_cargo_jump(p.assets, short_hop).map(|()| None);
    }
    *frames -= 1.0;
    Ok(None)
}

/// ftCo_CargoKneebend_IASA (8009BA9C): a throw, else
/// ftCo_KneeBend_Check_ShortHop.
pub fn knee_bend_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.try_cargo_throw(p.assets).expect("cargo carry assets") {
        return;
    }
    let release = p.assets.jumping.release_threshold;
    let current = f.core.input.current;
    let MotionData::Cargo(CargoState::KneeBend {
        short_hop, input, ..
    }) = &mut f.core.state_data
    else {
        panic!("cargo jump squat data missing")
    };
    *short_hop |= match input {
        JumpInput::Buttons => !current.held.intersects(Buttons::XY),
        JumpInput::Stick => current.stick.y < release,
        JumpInput::CStick => current.cstick.y < release,
    };
}

/// ftCo_CargoJump_Anim (8009BBF4) and ftCo_CargoFall_Anim (8009BCE4) are
/// empty.
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// ftCo_CargoJump_IASA (8009BBF8) / ftCo_CargoFall_IASA (8009BCE8): a
/// throw.
pub fn air_input(f: &mut Fighter, p: InputPhase<'_>) {
    f.try_cargo_throw(p.assets).expect("cargo carry assets");
}

/// ftCo_CargoJump_Phys (8009BC1C) -> ftCo_Jump_Phys_Inner: nothing on the
/// first frame, then ft_80084DB0.
pub fn jump_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if let MotionData::Cargo(CargoState::Jump {
        physics_started, ..
    }) = &mut f.core.state_data
    {
        if !*physics_started {
            *physics_started = true;
            f.core.finish_air_update(p.assets, p.wind);
            return;
        }
    }
    callbacks::physics::pass(f, p);
}

/// ftCo_CargoFall_Coll (8009BD2C) / ftCo_CargoJump_Coll: landing
/// (ft_80082C74 -> ft_80081D0C) enters the carry landing.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if lands(f, p.map) {
        f.enter_cargo_landing(p.assets.expect("cargo carry collision assets"))?;
    }
    Ok(())
}

/// ftCo_CargoLanding_Anim (8009BDC4): the wait once the frames run out
/// (checked before the decrement).
pub fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let MotionData::Cargo(CargoState::Landing { frames }) = &mut f.core.state_data else {
        panic!("cargo landing data missing")
    };
    if *frames <= 0.0 {
        return f.enter_cargo_wait(p.assets, false).map(|()| None);
    }
    *frames -= 1.0;
    Ok(None)
}

/// The empty IASA callbacks (the landing's, the aerial throws').
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCo_CargoThrowF_Anim (8009C0EC) and its siblings: ftCo_800DD724 (the
/// script's facing reversal; the release itself is the scene's, right after
/// this callback), then ftCommon_8007D92C at the animation's end.
pub fn throw_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    if std::mem::take(&mut f.commands.throw_reverse) {
        f.physics.facing = -f.physics.facing;
    }
    if !f.core.animation.frames_remaining(&f.core.skeleton) {
        let state = if f.core.physics.ground_or_air == GroundOrAir::Air {
            S::Fall
        } else {
            S::Wait
        };
        f.change_motion_state(state.into(), p.assets)?;
    }
    Ok(None)
}

/// inlineB0 (ftCo_CargoThrow.c:162): the throw's ground/air twin at the
/// current frame with ftCommon_GroundAirColl_MF, entered with the facing
/// the throw began with (facing_dir1), and every grab category excluded
/// again.
fn change_throw_row(f: &mut Fighter, assets: &FighterAssets, state: ActionId) -> Result<()> {
    let MotionData::Cargo(CargoState::Throw { entry_facing }) = f.core.state_data.clone() else {
        panic!("cargo throw data missing")
    };
    let facing = f.core.physics.facing;
    f.core.physics.facing = entry_facing;
    let frame = f.core.animation.frame;
    f.change_motion_state_with_flags(state, assets, GROUND_AIR, frame, 1.0)?;
    f.core.physics.facing = facing;
    f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    f.core.state_data = MotionData::Cargo(CargoState::Throw { entry_facing });
    Ok(())
}

/// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12).
const GROUND_AIR: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);

/// ftCo_CargoThrowF_Coll (8009C208) and its siblings: off the floor
/// (ft_8008403C), ftCo_8009C170 continues in the aerial row.
pub fn throw_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !stays_grounded(f, p.map) {
        f.leave_ground();
        let state = ActionId(f.core.motion_state.action.0 + row::AIR_THROW_OFFSET);
        change_throw_row(f, p.assets.expect("cargo throw collision assets"), state)?;
    }
    Ok(())
}

/// ftCo_CargoThrowAir_Coll (8009C43C): landing (ft_80082C74),
/// ftCo_8009C45C continues in the grounded row.
pub fn air_throw_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if lands(f, p.map) {
        f.land();
        let state = ActionId(f.core.motion_state.action.0 - row::AIR_THROW_OFFSET);
        change_throw_row(f, p.assets.expect("cargo throw collision assets"), state)?;
    }
    Ok(())
}

impl FighterCore {
    /// The carrier's throw script may release the fighter it holds
    /// (ftCo_800DD724 from the cargo throws' animation callbacks).
    pub fn in_cargo_throw(&self) -> bool {
        matches!(self.state_data, MotionData::Cargo(CargoState::Throw { .. }))
    }

    /// One of the shouldered rows (ftCo_MS_ShoulderedWait..ShoulderedTurn).
    pub fn shouldered(&self) -> bool {
        matches!(
            self.motion_state.id,
            S::ShoulderedWait
                | S::ShoulderedWalkSlow
                | S::ShoulderedWalkMiddle
                | S::ShoulderedWalkFast
                | S::ShoulderedTurn
        )
    }
}

/// Apply one of the carrier's requests to the fighter it holds.
pub fn apply(
    request: CargoRequest,
    victim: &mut Fighter,
    carrier: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> Result<()> {
    match request {
        CargoRequest::Shoulder { state, begin } => {
            if begin {
                begin_carry(victim, va);
            }
            shoulder_victim(victim, state, va, aa)
        }
        CargoRequest::Face => {
            victim.core.physics.facing = carrier.core.physics.facing;
            Ok(())
        }
        CargoRequest::Throw { direction } => {
            let states = [S::ThrownFF, S::ThrownFB, S::ThrownFHi, S::ThrownFLw];
            let state = states[usize::from(direction)];
            grab_throw::enter_thrown(victim, carrier, va, aa, state, victim_motion(state), 1.0)
        }
    }
}

/// ftCo_8009C640 (8009C640): ftCommon_InitGrab with the timer
/// (int)(percent * PlCo +4A0 + +4A4) (retail 0x8009C684: fmadds, then
/// fctiwz), and the scratch cleared.
fn begin_carry(victim: &mut Fighter, va: &FighterAssets) {
    let p = &va.cargo;
    let timer = fctiwz(fmadds(
        victim.core.physics.percent,
        p.percent_scale,
        p.base_timer,
    )) as f32;
    victim.core.state_data = MotionData::Shouldered(ShoulderedState {
        timer,
        fast_remaining: 0.0,
        stick_directions: [0; 2],
    });
}

/// ftCo_8009C5A4 (8009C5A4): the shouldered row from the carrier's data,
/// unless the victim already waits there; every movement source cleared
/// (ftCommon_8007E2FC) and every grab category excluded. accessory1 stays
/// ftCo_800DB464 (the constraint the throw installed).
fn shoulder_victim(
    victim: &mut Fighter,
    state: S,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> Result<()> {
    if victim.core.motion_state.id == state && state == S::ShoulderedWait {
        return Ok(());
    }
    // The grab timer, the mash latches and mv.co.shouldered outlive the
    // motion change.
    let kept = std::mem::take(&mut victim.core.state_data);
    assert!(
        matches!(kept, MotionData::Shouldered(_)),
        "ftCo_8009C5A4: a shouldered fighter without its grab timer"
    );
    let source = grab_throw::throw_source(aa, va, victim_motion(state));
    victim.change_motion_state_with_source(state.into(), va, 0.0, 1.0, Some(source))?;
    victim.core.state_data = kept;
    victim.core.clear_movement();
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    Ok(())
}

/// ftCo_Shouldered_Anim (8009C830): each mash input takes PlCo +4A8 off the
/// timer (scaled by +4AC while the carrier is airborne); nothing else does.
/// At zero the scene ends the carry (`escape`). Mashing in ShoulderedWait
/// plays the animation at PlCo +3B4 for +3B0 frames at a time.
pub fn shouldered_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let cargo = &p.assets.cargo;
    let escape = &p.assets.grab_escape;
    // retail 0x8009C878: fmuls.
    let decrement = if f.core.combat.carrier_airborne {
        cargo.mash_decrement * cargo.air_mash_scale
    } else {
        cargo.mash_decrement
    };
    let waiting = f.core.motion_state.id == S::ShoulderedWait;
    let input = f.core.input.clone();
    let MotionData::Shouldered(state) = &mut f.core.state_data else {
        panic!("shouldered scratch missing")
    };
    let mashed = super::capture_yoshi::grab_mash(
        &mut state.timer,
        &mut state.stick_directions,
        &input,
        escape.stick_threshold,
        decrement,
    );
    if state.timer <= 0.0 {
        f.core.combat.shoulder_escape = true;
        return Ok(None);
    }
    let mut rate = None;
    if state.fast_remaining != 0.0 {
        state.fast_remaining -= 1.0;
        if state.fast_remaining <= 0.0 && !mashed {
            rate = Some(1.0);
            state.fast_remaining = 0.0;
        }
    }
    if state.fast_remaining <= 0.0 && mashed && waiting {
        state.fast_remaining = escape.fast_frames;
        rate = Some(escape.fast_rate);
    }
    if let Some(rate) = rate {
        f.core.animation.set_rate(&mut f.core.skeleton, rate, false);
    }
    Ok(None)
}

/// A throw record (xDF4[1]) the fighter's scripts never wrote reads as
/// zeroes.
fn second_throw_record(f: &Fighter) -> (melee_types::combat::HitboxDescriptor, u32) {
    match f.commands.throw_hitboxes[1].as_ref() {
        Some(record) => (
            grab_throw::throw_descriptor(record),
            f.commands.throw_damage_counts[1],
        ),
        None => unimplemented!(
            "ftCo_Shouldered.c:92: breaking out with a throw record 1 no script has written (zeroes in retail)"
        ),
    }
}

/// ftCo_Shouldered_Anim's breakout (8009C894..8009C958) and ftCo_8009C744
/// (8009C744): the carrier takes the victim's throw record 1 facing its
/// own way, then the pair parts (ftCo_800DC920) and the victim takes the
/// carrier's record 1 facing against it. Both knockbacks are
/// ftColl_80079C70's and both launch through ftCo_8008E908(gobj, 0).
pub fn escape(
    victim: &mut Fighter,
    carrier: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let (victim_record, victim_count) = second_throw_record(victim);
    let (carrier_record, carrier_count) = second_throw_record(carrier);
    // ftColl_80078710 only records the source for stats.
    let knockback = aa.damage.knockback_for_frame(
        &victim_record,
        carrier.physics.percent,
        carrier.combat.frame_damage,
        carrier.attributes.size.weight,
        victim_count,
    );
    let hit = melee_coll::damage::ReceivedHit {
        facing: carrier.physics.facing,
        facing_override: None,
        percent_damage: victim_record.damage,
        descriptor: victim_record,
        height: melee_coll::hurtbox::HurtHeight::Middle,
        knockback,
    };
    carrier.begin_damage_reaction(hit, None, None, None, false, aa, rng)?;
    // ftCo_800DC920's lb_8000B1CC rebuilds the victim's XRotN, whose
    // constraint reads the carrier's TransN2 in its new damage pose.
    grab_throw::update_constraint(&mut victim.core, &mut carrier.core, va, aa);
    super::grab_damage::release_pair(carrier, victim, va, map);
    let knockback = va.damage.knockback_for_frame(
        &carrier_record,
        victim.physics.percent,
        victim.combat.frame_damage,
        victim.attributes.size.weight,
        carrier_count,
    );
    let hit = melee_coll::damage::ReceivedHit {
        facing: -carrier.physics.facing,
        facing_override: None,
        percent_damage: carrier_record.damage,
        descriptor: carrier_record,
        height: melee_coll::hurtbox::HurtHeight::Middle,
        knockback,
    };
    victim.begin_damage_reaction(hit, None, None, None, false, va, rng)?;
    Ok(())
}
