//! Capture timer, mash input, pummel entry and paired escape.
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, PhysicsPhase},
    Fighter, MotionData,
};
use crate::{
    anim::WaitChoice,
    input::{Buttons, FighterInput},
};
use hsd_archive::Archive;
use melee_types::{CommonMotionState as S, GroundOrAir};

pub struct Parameters {
    pub stick_threshold: f32,
    pub base_timer: f32,
    pub handicap_scale: f32,
    pub handicap_origin: f32,
    pub rank_scale: f32,
    pub rank_origin: f32,
    pub percent_scale: f32,
    pub decrement: f32,
    pub mash_decrement: f32,
    pub fast_frames: f32,
    pub fast_rate: f32,
    pub escape_speed: f32,
    pub escape_friction: f32,
    pub lift_threshold: f32,
    pub air_escape_speed: f32,
    pub air_escape_vertical_speed: f32,
    pub jump_buffer_window: f32,
    pub jump_interrupt_delay: f32,
    pub horizontal_release_distance: f32,
    pub vertical_release_distance: f32,
    /// PlCo +3BC: how far above a floor under a thrown fighter's release
    /// point it may start and still land there (ftCo_800DC920).
    pub release_floor_reach: f32,
    /// PlCo +380 (lbColl_80008D30's record): the hit a captor takes when a
    /// launch frees its victim and leaves it unhit (ftCo_800DE2F0).
    pub captor_release_hit: melee_types::combat::ThrowHitbox,
    /// PlCo +348: the thrower's invincibility at the start of a throw
    /// (ftCo_800DD398 -> ftColl_8007B7A4).
    pub throw_invincible_frames: i32,
}
impl Parameters {
    /// ftCommonData: ftCo_CapturePulled/Wait/Cut and ftCommon_GrabMash.
    pub fn read(archive: &Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            stick_threshold: r.f32(base + 0x308)?,
            base_timer: r.f32(base + 0x354)?,
            handicap_scale: r.f32(base + 0x358)?,
            handicap_origin: r.f32(base + 0x35C)?,
            rank_scale: r.f32(base + 0x360)?,
            rank_origin: r.f32(base + 0x364)?,
            percent_scale: r.f32(base + 0x368)?,
            escape_friction: r.f32(base + 0x36C)?,
            escape_speed: r.f32(base + 0x370)?,
            lift_threshold: r.f32(base + 0x3C4)?,
            air_escape_speed: r.f32(base + 0x374)?,
            air_escape_vertical_speed: r.f32(base + 0x378)?,
            jump_buffer_window: r.f32(base + 0x3AC)?,
            jump_interrupt_delay: r.f32(base + 0x3B8)?,
            horizontal_release_distance: r.f32(base + 0x34C)?,
            vertical_release_distance: r.f32(base + 0x350)?,
            release_floor_reach: r.f32(base + 0x3BC)?,
            captor_release_hit: read_release_hit(archive, base + 0x380)?,
            throw_invincible_frames: r.s32(base + 0x348)?,
            decrement: r.f32(base + 0x3A4)?,
            mash_decrement: r.f32(base + 0x3A8)?,
            fast_frames: r.f32(base + 0x3B0)?,
            fast_rate: r.f32(base + 0x3B4)?,
        })
    }
}

/// lbColl_80008D30_arg1: state, damage, angle, growth, weight-set and base
/// knockback, element and sound words; lbColl_80008D30 converts the damage
/// to a float.
fn read_release_hit(archive: &Archive, at: u32) -> Result<melee_types::combat::ThrowHitbox> {
    let r = archive.reader();
    let word = |offset| r.u32(at + offset);
    Ok(melee_types::combat::ThrowHitbox {
        damage: word(0x4)? as f32,
        angle: word(0x8)? as u16,
        growth: word(0xC)? as u16,
        weight_knockback: word(0x10)? as u16,
        base_knockback: word(0x14)? as u16,
        element: melee_types::HitElement::try_from(word(0x18)? as i32)
            .expect("PlCo +398: a hit element"),
        sound_severity: word(0x1C)? as u8,
        sound_kind: word(0x20)? as u8,
    })
}

#[derive(Clone, Debug)]
pub struct CaptureState {
    pub timer: f32,
    pub elapsed: f32,
    /// Paired scene Map ran before the single-fighter row consumes it.
    pub map_prepared: bool,
    /// mv.capturewait.xC: latched XY during the initial capture interval.
    pub jump_requested: bool,
    /// CaptureWait's Anim found the timer expired this tick; the scene then
    /// releases both fighters (ftCo_800DA698). Only that callback checks the
    /// timer: entering CaptureWait from CaptureDamage never releases at once.
    pub release_requested: bool,
    /// The captor's [`super::CharacterCallbacks::mouth_capture_scale`]:
    /// this victim is held in its captor's mouth.
    pub mouth_scale: Option<f32>,

    fast_remaining: f32,
    stick_directions: [i8; 2],
}
impl CaptureState {
    /// fn_800DA8E4 (800DA8E4): handicap/rank-adjusted initial grab timer.
    pub fn new(percent: f32, handicap: u8, rank: u8, p: &Parameters) -> Self {
        let rank_term = p.rank_scale * (p.rank_origin - (f32::from(rank) + 1.0));
        // Retail 800DA9CC/800DA9D4: fmadds, with separate intervening fadds.
        let base = gekko_math::fma::fmadds(
            p.handicap_scale,
            p.handicap_origin - f32::from(handicap),
            p.base_timer,
        );
        Self {
            timer: gekko_math::fma::fmadds(percent, p.percent_scale, base + rank_term),
            elapsed: 0.0,
            map_prepared: false,
            jump_requested: false,
            release_requested: false,
            mouth_scale: None,
            fast_remaining: 0.0,
            stick_directions: [0; 2],
        }
    }

    /// ftCommon_GrabMash (8007DC08): one button decrement and one stick decrement.
    /// Neutral retains each axis's last non-neutral direction.
    fn mash(&mut self, input: &FighterInput, p: &Parameters) -> bool {
        super::capture_yoshi::grab_mash(
            &mut self.timer,
            &mut self.stick_directions,
            input,
            p.stick_threshold,
            p.mash_decrement,
        )
    }
}

/// ftCo_CaptureWaitLw_Anim -> ftCo_CaptureWaitHi_Anim (800DBE24 / 800DB908).
/// The scene performs the reciprocal escape immediately after this animation.
pub fn capture_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let p = &phase.assets.grab_escape;
    let input = f.input.clone();
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    // Retail increments elapsed in double precision, then rounds to single.
    capture.elapsed = (f64::from(capture.elapsed) + 1.0) as f32;
    capture.timer -= p.decrement;
    let mashed = capture.mash(&input, p);
    // Retail paired release returns before playback-rate maintenance.
    if capture.timer <= 0.0 {
        capture.release_requested = true;
        return Ok(None);
    }
    let mut rate = None;
    if capture.fast_remaining != 0.0 {
        capture.fast_remaining -= 1.0;
        if capture.fast_remaining <= 0.0 && !mashed {
            capture.fast_remaining = 0.0;
            rate = Some(1.0);
        }
    }
    if capture.fast_remaining <= 0.0 && mashed {
        capture.fast_remaining = p.fast_frames;
        rate = Some(p.fast_rate);
    }
    if let Some(rate) = rate {
        f.core.animation.set_rate(&mut f.core.skeleton, rate, false);
    }
    Ok(None)
}

/// ftCo_8008EC90 / ftCo_800DC3A4: damage without releasing or launching the victim.
pub(super) fn capture_damage(
    f: &mut Fighter,
    hit: &melee_coll::damage::ReceivedHit,
    assets: &FighterAssets,
) -> Result<i32> {
    // CaptureCaptain (Falcon Dive's victim) is neither 0xE0/0xE1 nor
    // 0xE3/0xE4: like a throw it keeps its motion.
    let thrown = matches!(
        f.motion_state.id,
        S::ThrownF | S::ThrownB | S::ThrownHi | S::ThrownLw | S::CaptureCaptain
    );
    if !thrown
        && !matches!(
            f.motion_state.id,
            S::CaptureWaitLw | S::CaptureDamageLw | S::CaptureWaitHi | S::CaptureDamageHi
        )
    {
        unimplemented!("ftCo_8008EC90: captured damage outside low capture or throw");
    }
    f.core.physics.percent += hit.percent_damage;
    f.core.input.pressed = Buttons::default();
    f.core.input.released = Buttons::default();
    // ftCo_Damage_CalcKnockback has already run: kb_applied is modified.
    let knockback = f.core.modified_knockback(hit.knockback, assets);
    // ftCo_8008EC90 inlineB2, 8008ECD4..ED84: thrown states keep their
    // borrowed animation, pose and link while sharing the captor hitlag.
    if thrown {
        // Fighter_ProcessHit (fighter.c:2851-2852) reaches ftCo_8008EC90 only
        // with dmg.kb_applied set; a knockback-free captor hit (Falcon Dive's
        // catch hitbox on CaptureCaptain) takes damage without the flash.
        if hit.knockback != 0.0 {
            f.core.unlaunched_damage_flash(knockback, hit, assets);
        }
        return Ok(gekko_math::msl::fctiwz(hit.descriptor.damage).max(1));
    }
    let state = if matches!(f.motion_state.id, S::CaptureWaitHi | S::CaptureDamageHi) {
        S::CaptureDamageHi
    } else {
        S::CaptureDamageLw
    };
    f.change_motion_state(state.into(), assets)?;
    super::grab::hold_in_mouth(&mut f.core, assets);
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    capture.fast_remaining = 0.0;
    f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    // inlineB2: ftCo_8008DA4C after the capture-damage entry.
    f.core.unlaunched_damage_flash(knockback, hit, assets);
    Ok(gekko_math::msl::fctiwz(hit.descriptor.damage).max(1))
}

/// ftCo_CaptureDamageLw_Anim (800DC470): count down without the wait state's escape check.
pub fn capture_damage_animation(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    // fn_800DB8A4: increment in f64, then subtract the f32 timer decrement and mash.
    capture.elapsed = (f64::from(capture.elapsed) + 1.0) as f32;
    capture.timer -= phase.assets.grab_escape.decrement;
    capture.mash(&f.core.input, &phase.assets.grab_escape);
    if !f.animation.frames_remaining(&f.skeleton) {
        super::grab::capture_wait(f, phase.assets)?;
    }
    Ok(None)
}

/// fn_800DA4C0 / fn_800DA4FC: pummel has priority over all four throws.
pub fn pummel_input(f: &mut Fighter, phase: super::state::InputPhase<'_>) {
    if f.input.pressed.intersects(Buttons::A) {
        f.core.physics.ground_velocity = 0.0;
        f.change_motion_state(S::CatchAttack.into(), phase.assets)
            .expect("pummel entry");
        f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    }
}

/// ftCo_CatchAttack_Anim (800DA56C), fn_800DA2B0: return without a new grab flash.
pub fn pummel_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::CatchWait.into(), phase.assets)?;
        f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    }
    Ok(None)
}

/// ftCo_800DA698 / ftCo_CaptureCut_Enter (800DA698 / 800DC750).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseCause {
    TimerExpired,
    CaptorSeparation,
}

pub fn release(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
    cause: ReleaseCause,
) -> Result<()> {
    let (jump, retained_drop_timer) = {
        let MotionData::Capture(capture) = &victim.state_data else {
            unreachable!()
        };
        (
            cause == ReleaseCause::TimerExpired
                && (capture.jump_requested
                    || victim.input.current.stick.y >= va.common.input.tap_jump_threshold),
            capture.fast_remaining,
        )
    };
    if attacker.physics.ground_or_air == GroundOrAir::Air {
        attacker.physics.self_velocity.x =
            -attacker.physics.facing * aa.grab_escape.air_escape_speed;
        attacker.physics.self_velocity.y = aa.grab_escape.air_escape_vertical_speed;
    } else {
        attacker.physics.ground_velocity = -attacker.physics.facing * aa.grab_escape.escape_speed;
    }
    attacker.change_motion_state(S::CatchCut.into(), aa)?;
    if jump {
        victim.leave_ground();
        victim.physics.self_velocity.x = -victim.physics.facing * va.grab_escape.air_escape_speed;
        victim.physics.self_velocity.y = va.grab_escape.air_escape_vertical_speed;
    }
    // ftCo_800DC920: both links go; a victim pinned in Yoshi's mouth
    // (x2226_b2) is first set down where its XRotN points. lb_8000B1CC
    // rebuilds XRotN's matrix, whose constraint reads the captor's TransN2
    // in its new CatchCut pose (ftCo_800DA698 ran first).
    attacker.combat.grab = None;
    if victim.combat.thrown_pose.is_some() {
        super::grab_throw::update_constraint(&mut victim.core, &mut attacker.core, va, aa);
        super::grab_damage::release_thrown(attacker, victim, va, map);
    }
    victim.combat.grab = None;
    if !jump {
        let velocity = -victim.physics.facing * va.grab_escape.escape_speed;
        if victim.physics.ground_or_air == GroundOrAir::Ground {
            victim.physics.ground_velocity = velocity;
        } else {
            victim.physics.self_velocity.x = velocity;
        }
    }
    victim.change_motion_state(
        (if jump { S::CaptureJump } else { S::CaptureCut }).into(),
        va,
    )?;
    if jump {
        victim.state_data = MotionData::CaptureJump(CaptureJumpState {
            frames: 0.0,
            retained_drop_timer,
        });
    }
    Ok(())
}

/// ftCo_CatchCut_Anim / ftCo_CaptureCut_Anim: ground -> Wait, air -> Fall.
pub fn cut_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            (if f.physics.ground_or_air == GroundOrAir::Ground {
                S::Wait
            } else {
                S::Fall
            })
            .into(),
            phase.assets,
        )?;
    }
    Ok(None)
}

/// ftCo_CaptureCut_Phys: captured fighter's independent common friction multiplier.
pub fn cut_physics(f: &mut Fighter, phase: PhysicsPhase<'_>) {
    use crate::physics::{friction::friction_acceleration, grounded};
    if f.physics.ground_or_air == GroundOrAir::Air {
        super::state::callbacks::physics::fall(f, phase);
        return;
    }
    f.core.physics.ground_acceleration = friction_acceleration(
        f.physics.ground_velocity,
        phase.assets.grab_escape.escape_friction * f.attributes.ground.ground_friction,
    );
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        phase.map.floor_speed_scale(&core.collision.data),
    );
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &phase.assets.common),
        phase.map,
        phase.wind,
    );
}

/// fn_800DC014: XY can request jump release only before PlCo+3AC elapsed.
pub fn capture_input(f: &mut Fighter, phase: super::state::InputPhase<'_>) {
    let pressed = f.input.pressed.intersects(Buttons::XY);
    let MotionData::Capture(capture) = &mut f.state_data else {
        unreachable!()
    };
    if capture.elapsed < phase.assets.grab_escape.jump_buffer_window && pressed {
        capture.jump_requested = true;
    }
}

#[derive(Clone, Debug)]
pub struct CaptureJumpState {
    pub frames: f32,
    /// CaptureJump writes only scratch x0; Landing inherits CaptureWait x4.
    pub retained_drop_timer: f32,
}

/// ftCo_CaptureJump_Anim: f32 counter, then ordinary Fall on completion.
pub fn jump_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let MotionData::CaptureJump(jump) = &mut f.state_data else {
        unreachable!()
    };
    jump.frames += 1.0;
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::Fall.into(), phase.assets)?;
    }
    Ok(None)
}

pub fn jump_input(f: &mut Fighter, phase: super::state::InputPhase<'_>) {
    let MotionData::CaptureJump(jump) = &f.state_data else {
        unreachable!()
    };
    let ready = jump.frames >= phase.assets.grab_escape.jump_interrupt_delay;
    if !ready {
        return;
    }
    let assets = phase.assets;
    if f.input.pressed.intersects(Buttons::B) {
        f.enter_buffered_special(assets, true);
        return;
    }
    // ftCo_CaptureJump_IASA: ftCo_800D7100 after the special check.
    if f.try_aerial_item_catch(assets) {
        return;
    }
    f.character.air_dodge_tether();
    if f.input.pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        f.enter_air_dodge(assets).expect("capture jump air dodge");
    } else if super::attack::aerial::requested(&f.input, &assets.input) {
        (f.character.table().enter_aerial)(f, assets).expect("capture jump aerial");
    } else if f.aerial_jump_requested(assets) {
        f.enter_aerial_jump(assets)
            .expect("capture jump aerial jump");
    }
    // ftCo_800D705C cannot succeed after the pressed-A aerial predicate failed.
}

/// ftCo_CaptureJump_Phys: gravity and drift, with no fast-fall check.
pub fn jump_physics(f: &mut Fighter, phase: PhysicsPhase<'_>) {
    f.physics.self_velocity.y = crate::physics::airborne::gravity(
        f.physics.self_velocity.y,
        f.attributes.air.gravity,
        f.attributes.air.terminal_velocity,
    );
    f.physics.animation_velocity.x = crate::physics::airborne::drift(
        f.physics.self_velocity.x,
        f.input.current.stick.x,
        &f.attributes.air,
    );
    f.core.finish_air_update(phase.assets, phase.wind);
}

pub fn catch_cut_physics(f: &mut Fighter, phase: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Air {
        super::state::callbacks::physics::fall(f, phase);
    } else {
        let c = &mut f.core;
        c.physics.ground_acceleration = crate::physics::friction::friction_acceleration(
            c.physics.ground_velocity,
            phase.assets.grab_friction_multiplier * c.attributes.ground.ground_friction,
        );
        crate::physics::grounded::apply_ground_movement(
            &mut c.physics,
            c.collision.data.floor.normal,
            phase.map.floor_speed_scale(&c.collision.data),
        );
        crate::physics::grounded::finish_ground_update(
            &mut c.physics,
            &c.collision.data,
            &crate::physics::grounded::GroundedParameters::from_attributes(
                &c.attributes,
                &phase.assets.common,
            ),
            phase.map,
            phase.wind,
        );
    }
}

/// ftCo_CatchCut_Coll: stop at ground edge, ordinary airborne landing.
pub fn catch_cut_collision(f: &mut Fighter, phase: super::state::CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return super::state::callbacks::collision::escape(f, phase);
    }
    cut_air_collision(f, phase, false)
}

/// ftCo_CaptureCut_Coll: leave support or land without replacing the motion.
pub fn cut_collision(f: &mut Fighter, phase: super::state::CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        let c = &mut f.core;
        let support = crate::collision::ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            phase.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        );
        if support == crate::collision::ground::WaitGroundResult::EnterFall {
            f.leave_ground();
        }
        return Ok(());
    }
    cut_air_collision(f, phase, true)
}

fn cut_air_collision(
    f: &mut Fighter,
    phase: super::state::CollisionPhase<'_>,
    captured: bool,
) -> Result<()> {
    let c = &mut f.core;
    crate::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = crate::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let position = |i| pose.position(i);
    let landed = if c.status.ledge_cooldown == 0 && !c.status.ledge_grab_disabled {
        melee_mp::set_facing_dir(cd, if c.physics.facing < 0.0 { -1 } else { 1 });
        if captured {
            phase.map.air_collide_ledge_ecb5(cd, Some(&position))
        } else {
            phase.map.air_collide_ledge(cd, Some(&position))
        }
    } else if captured {
        phase.map.air_collide_ecb5(cd, Some(&position))
    } else {
        phase.map.air_collide_pass(cd, Some(&position))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    let assets = phase.assets.expect("capture cut map assets");
    if landed {
        if captured {
            f.land();
        } else if f.physics.self_velocity.y > assets.soft_landing_speed {
            f.land();
            f.change_motion_state(S::Wait.into(), assets)?;
        } else {
            f.enter_landing(assets)?;
        }
    } else if !f.try_wall_jump(assets, phase.map)? {
        f.try_grab_ledge(assets, phase.map)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Stick;

    fn parameters() -> Parameters {
        Parameters {
            stick_threshold: 0.5,
            base_timer: 30.0,
            handicap_scale: 8.0,
            handicap_origin: 9.0,
            rank_scale: 15.0,
            rank_origin: 4.0,
            percent_scale: 1.6,
            decrement: 1.0,
            mash_decrement: 6.0,
            fast_frames: 10.0,
            fast_rate: 2.0,
            escape_speed: 1.0,
            escape_friction: 1.0,
            lift_threshold: 1.0,
            air_escape_speed: 1.0,
            air_escape_vertical_speed: 1.0,
            jump_buffer_window: 1.0,
            jump_interrupt_delay: 1.0,
            horizontal_release_distance: 1.0,
            vertical_release_distance: 1.0,
            release_floor_reach: -3.0,
            captor_release_hit: melee_types::combat::ThrowHitbox {
                damage: 0.0,
                angle: 0,
                growth: 0,
                weight_knockback: 0,
                base_knockback: 0,
                element: melee_types::HitElement::Normal,
                sound_severity: 0,
                sound_kind: 0,
            },
            throw_invincible_frames: 8,
        }
    }

    #[test]
    fn grab_timer_uses_percent_handicap_and_tied_rank() {
        let p = parameters();
        assert_eq!(CaptureState::new(0.0, 9, 0, &p).timer, 75.0);
        assert_eq!(CaptureState::new(10.0, 8, 1, &p).timer, 84.0);
    }

    #[test]
    fn mash_counts_buttons_and_stick_separately_and_retains_direction_through_neutral() {
        let p = parameters();
        let mut capture = CaptureState::new(0.0, 9, 0, &p);
        let mut input = FighterInput {
            pressed: Buttons::A | Buttons::B,
            ..FighterInput::default()
        };
        input.current.stick = Stick { x: 0.8, y: -0.8 };
        assert!(capture.mash(&input, &p));
        assert_eq!(capture.timer, 63.0); // Two buttons and two axes still cost 6 + 6.
        input.pressed = Buttons::default();
        input.current.stick = Stick::default();
        assert!(!capture.mash(&input, &p));
        input.current.stick = Stick { x: 0.8, y: -0.8 };
        assert!(!capture.mash(&input, &p));
        input.current.stick.x = -0.5; // Strict crossing: equality does not count.
        assert!(!capture.mash(&input, &p));
        input.current.stick.x = -0.8;
        assert!(capture.mash(&input, &p));
        assert_eq!(capture.timer, 57.0);
    }
}
