//! Belay, ftpopospecialhi.c (80120E68..8012280C): Popo's rows 347..356.
//!
//! Popo swings his hammer up (the start, 347 / 352). When the script sets
//! cmd_vars[2], Nana joins if she is near and free (ftNn_Init_8012300C,
//! `partner`); otherwise Popo belays alone (350 / 355, then 351 / 356:
//! aloft, a small hop). Joined, the throw (348 / 353) flings Nana up
//! (her 361 then 365) and, when the script sets cmd_vars[1] while she is
//! in flight, Popo climbs after her (354, ftPp_SpecialHi_8012280C).
//!
//! A frame counter (mv.pp.unk_80123954.x0) runs through the start, the
//! throw and the climb; at set frames it makes the rope in Popo's left
//! hand and moves it along (ftPp_SpecialS_80120FE0, `rope`).
pub mod partner;
pub mod rope;

use crate::climber::{self, attributes, vars};
use gekko_math::{fma::fnmsubs, msl::sqrtf};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::friction,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{mp::collide, FtPart, ItemKind};

/// ftPp_MS_SpecialHiStart_0 .. ftPp_MS_SpecialAirHiThrow_1.
pub const START: ActionId = ActionId(347);
pub const THROW: ActionId = ActionId(348);
pub const CLIMB: ActionId = ActionId(349);
pub const SOLO_START: ActionId = ActionId(350);
pub const SOLO_THROW: ActionId = ActionId(351);
pub const AIR_START: ActionId = ActionId(352);
pub const AIR_THROW: ActionId = ActionId(353);
pub const AIR_CLIMB: ActionId = ActionId(354);
pub const AIR_SOLO_START: ActionId = ActionId(355);
pub const AIR_SOLO_THROW: ActionId = ActionId(356);
/// The rope item lives while its owner is in one of these
/// (itClimbersstring_UnkMotion3_Anim).
pub const BELAY: core::ops::RangeInclusive<u16> = START.0..=AIR_SOLO_THROW.0;

/// ftPp_MF_SpecialHi_Coll (0x0C4C508A): ftCommon_GroundAirColl_MF with
/// KeepGfx and SkipHit, the ground/air counterparts.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_508A);
/// ftPp_SpecialS_80120FE0: the counter makes the rope at 8 and takes it
/// away at 0x53.
const ROPE_SPAWN_FRAME: i32 = 8;
const ROPE_REMOVE_FRAME: i32 = 0x53;
/// fp->parts[FtPart_L4thNb] / [FtPart_R4thNb] / [FtPart_XRotN]: the
/// FtPart values index the parts table directly (retail 801210F0: +0x1D0;
/// 80123160: +0x2F0; 80123174: +0x20), no ftParts_GetBoneIndex.
pub(crate) const LEFT_HAND: usize = FtPart::L4thNb as usize;
pub(crate) const RIGHT_HAND: usize = FtPart::R4thNb as usize;
pub(crate) const HIPS: usize = FtPart::XRotN as usize;
/// ftPp_SpecialS_80120E68: Popo aims 3 behind Nana's facing and 5 above
/// her (retail @197, @198).
const CLIMB_AIM_BEHIND: f32 = 3.0;
const CLIMB_AIM_ABOVE: f32 = 5.0;

/// Popo's Belay scratch: mv.pp.unk_80123954 and u.pp's rope words.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BelayVars {
    /// mv.pp.unk_80123954.x0: the Belay's frame counter, 1 at entry.
    pub frame: i32,
    /// x2222_b2 once Nana joined the start (kept across its ground/air
    /// changes, Ft_MF_Unk19): a cape does not turn Popo.
    pub partner_joined: bool,
    /// u.pp.x2238 != NULL: the rope is in Popo's hand.
    pub rope_out: bool,
    /// take_dmg_cb = ftPp_Init_8011F060, installed with the rope; every
    /// motion change removes it.
    pub rope_take_damage: bool,
    /// death3_cb = ftPp_Init_8011F060, installed with the rope; motion
    /// changes keep it.
    pub rope_death: bool,
    /// u.pp.x2240: Nana's left hand, where the rope's tail hangs (zero
    /// when she is not belaying).
    pub anchor: Vec3,
}

fn belay(f: &mut Fighter) -> &mut BelayVars {
    &mut vars(f).belay
}
fn rope(f: &mut Fighter) -> &mut rope::Rope {
    &mut f.character.get_mut::<crate::init::IceClimber>().rope
}
fn attrs(f: &Fighter) -> &crate::attributes::BelayAttributes {
    &attributes(f).belay
}
fn is_air(f: &Fighter) -> bool {
    f.physics.ground_or_air == melee_types::GroundOrAir::Air
}

/// ftPp_Init_MotionStateTable rows 347..356.
pub const ROWS: [MotionRow; 10] = [
    climber::row(
        START.0,
        start_anim::<false>,
        start_input,
        ground_physics::<true>,
        start_ground_collision,
    ),
    climber::row(
        THROW.0,
        throw_anim::<false>,
        climber::no_input,
        ground_physics::<true>,
        throw_ground_collision,
    ),
    climber::row(
        CLIMB.0,
        climb_anim::<false>,
        climber::no_input,
        ground_physics::<true>,
        climb_ground_collision,
    ),
    climber::row(
        SOLO_START.0,
        solo_start_anim::<false>,
        climber::no_input,
        ground_physics::<false>,
        solo_start_ground_collision,
    ),
    climber::row(
        SOLO_THROW.0,
        solo_throw_anim::<false>,
        climber::no_input,
        ground_physics::<false>,
        solo_throw_ground_collision,
    ),
    climber::row(
        AIR_START.0,
        start_anim::<true>,
        start_input,
        throw_air_physics,
        start_air_collision,
    ),
    climber::row(
        AIR_THROW.0,
        throw_anim::<true>,
        climber::no_input,
        throw_air_physics,
        throw_air_collision,
    ),
    climber::row(
        AIR_CLIMB.0,
        climb_anim::<true>,
        climber::no_input,
        climb_air_physics,
        climb_air_collision,
    ),
    climber::row(
        AIR_SOLO_START.0,
        solo_start_anim::<true>,
        climber::no_input,
        solo_air_physics,
        solo_start_air_collision,
    ),
    climber::row(
        AIR_SOLO_THROW.0,
        solo_throw_anim::<true>,
        climber::no_input,
        solo_air_physics,
        solo_throw_air_collision,
    ),
];

/// ftPp_SpecialHi_Enter (80121164..) / ftPp_SpecialAirHi_Enter: the entry
/// speed divided down, the start's first frame (ftAnim_8006EBA4), then
/// the scratch.
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    let (divisor_x, divisor_y) = {
        let a = attrs(f);
        (a.entry_speed_divisor_x, a.entry_speed_divisor_y)
    };
    if airborne {
        f.physics.self_velocity.x /= divisor_x;
        f.physics.self_velocity.y /= divisor_y;
    } else {
        f.physics.ground_velocity /= divisor_x;
    }
    let state = if airborne { AIR_START } else { START };
    f.change_motion_state(state, assets).expect("Belay assets");
    f.step_animation(assets);
    if airborne {
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
    f.commands.variables[2] = 0;
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    let b = belay(f);
    b.frame = 1;
    b.partner_joined = false;
    b.anchor = Vec3::ZERO;
}

/// ftPp_SpecialHiStart_0_Anim (801212C4) / ftPp_SpecialAirHiStart_0_Anim
/// (801213CC): at the script's cmd_vars[2], Nana joins or Popo belays
/// alone at the current frame; at the end, the throw.
fn start_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[2] != 0 {
        f.commands.variables[2] = 0;
        if !partner::try_join(f) {
            let frame = f.animation.frame;
            let solo = if AIR { AIR_SOLO_START } else { SOLO_START };
            f.change_motion_state_at(solo, p.assets, frame)?;
            return Ok(None);
        }
        belay(f).partner_joined = true;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let throw = if AIR { AIR_THROW } else { THROW };
        f.change_motion_state(throw, p.assets)?;
        return Ok(None);
    }
    advance(f);
    Ok(None)
}

/// ftPp_SpecialHiStart_0_IASA (801214D4) / ftPp_SpecialAirHiStart_0_IASA:
/// at the script's cmd_vars[0], a stick past x80 either way turns Popo
/// (ftCommon_UpdateFacing, then ftPartSetRotY(fp, 0, M_PI_2 * facing) in
/// double, rounded once).
fn start_input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[0] == 0 {
        return;
    }
    f.commands.variables[0] = 0;
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    if magnitude > attrs(f).stick_threshold {
        turn_toward_stick(f);
    }
}

/// ftCommon_UpdateFacing, then ftPartSetRotY(fp, 0, M_PI_2 * facing).
fn turn_toward_stick(f: &mut Fighter) {
    f.physics.facing = if f.input.current.stick.x >= 0.0 {
        1.0
    } else {
        -1.0
    };
    face_model(f);
}

/// ftPartSetRotY(fp, 0, M_PI_2 * facing): fmul in double, frsp.
pub(crate) fn face_model(f: &mut Fighter) {
    let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
    f.core.set_part_rotation(0, Axis::Y, rotation);
}

/// The throw (ftPp_SpecialHiThrow_0_Anim 80121944, ftPp_SpecialAirHiThrow_0_Anim
/// 801219F4): at its end Wait or the special fall, though the rest still
/// runs; at the script's cmd_vars[1], Popo climbs after a flying Nana
/// (ftNn_Init_8012309C).
fn throw_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        finish::<AIR>(f, p.assets)?;
    }
    if f.commands.variables[1] != 0 {
        f.commands.variables[1] = 0;
        if partner::in_flight(f) {
            climb(f, p.assets)?;
            return Ok(None);
        }
    }
    advance(f);
    Ok(None)
}

/// ftPp_SpecialHiThrow2_Anim (801223B8) / ftPp_SpecialAirHiThrow2_Anim
/// (80122410).
fn climb_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        finish::<AIR>(f, p.assets)?;
    } else {
        advance(f);
    }
    Ok(None)
}

/// ftPp_SpecialHiStart_1_Anim (80121E10) / ftPp_SpecialAirHiStart_1_Anim
/// (80121E4C): aloft, the script's cmd_vars[2] lifts Popo (xA4); at the
/// end, the solo throw.
fn solo_start_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if AIR && f.commands.variables[2] != 0 {
        f.commands.variables[2] = 0;
        f.physics.self_velocity.y = attrs(f).solo_lift;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let throw = if AIR { AIR_SOLO_THROW } else { SOLO_THROW };
        f.change_motion_state(throw, p.assets)?;
    }
    Ok(None)
}

/// ftPp_SpecialHiThrow_1_Anim (80122110) / ftPp_SpecialAirHiThrow_1_Anim
/// (8012214C).
fn solo_throw_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        finish::<AIR>(f, p.assets)?;
    }
    Ok(None)
}

/// ft_8008A2BC on the ground; ftCo_80096900(gobj, 0, 1, false, x74, x78)
/// in the air.
fn finish<const AIR: bool>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if AIR {
        let (mobility, lag) = {
            let a = attrs(f);
            (a.fall_mobility, a.landing_lag)
        };
        f.enter_special_fall(assets, false, true, false, mobility, lag)
    } else {
        climber::finish(f, assets, false)
    }
}

/// The counter's step (incrementMvAndCheck): one more frame, then the
/// rope's events (ftPp_SpecialS_80120FE0).
fn advance(f: &mut Fighter) {
    belay(f).frame += 1;
    rope_events(f);
}

/// ftPp_SpecialS_80120FE0 (80120FE0): at 8 the rope appears in Popo's
/// hand; with it out, the article's x18 / x1C / x20 frames pay it out,
/// hang it and reel it in (it_802C3950, it_802C3810, it_802C3864), and
/// 0x53 takes it away (it_802C2750).
fn rope_events(f: &mut Fighter) {
    let frame = belay(f).frame;
    if frame > ROPE_SPAWN_FRAME && frame <= ROPE_REMOVE_FRAME {
        if !belay(f).rope_out {
            return;
        }
        let a = rope(f).attributes;
        if frame == a.pay_out_frame {
            let hand = hand_position(f);
            rope(f).start_paying_out(hand);
            set_rope_phase(f, rope::phase::PAYING_OUT);
        } else if frame == a.hang_frame {
            set_rope_phase(f, rope::phase::HANGING);
        } else if frame == a.reel_frame {
            set_rope_phase(f, rope::phase::REELING);
        }
        if frame == ROPE_REMOVE_FRAME {
            remove_rope(f);
        }
    } else if frame == ROPE_SPAWN_FRAME {
        spawn_rope(f);
    }
}

/// lb_8000B1CC(fp->parts[FtPart_L4thNb].joint, NULL, &pos): Popo's left
/// hand, where the rope's handle hangs.
fn hand_position(f: &mut Fighter) -> Vec3 {
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, LEFT_HAND, Vec3::ZERO)
}

fn set_rope_phase(f: &mut Fighter, phase: u16) {
    rope(f).phase = phase;
    rope_request(f, ItemControl::Motion(phase));
}

fn rope_request(f: &mut Fighter, control: ItemControl) {
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::IceClimberGumStrings,
        control,
    });
}

/// ftPp_SpecialS_801210C8 (801210C8): the rope at the left hand
/// (it_802C27D4: Item_80268B18 without initial collision, its links laid
/// out there by it_802C248C, then Item_8026AB54 on L4thNb); take_dmg_cb
/// and death3_cb become ftPp_Init_8011F060. x1984_heldItemSpec has no
/// port reader.
fn spawn_rope(f: &mut Fighter) {
    let hand = hand_position(f);
    let facing = f.physics.facing;
    let spawn = SpawnItem {
        initial_collision: false,
        ..SpawnItem::attached(ItemKind::IceClimberGumStrings, f.player.id, hand, facing)
    };
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: LEFT_HAND as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    let attributes = rope(f).attributes;
    rope(f).lay_out(attributes, hand);
    let b = belay(f);
    b.rope_out = true;
    b.rope_take_damage = true;
    b.rope_death = true;
}

/// it_802C2750 (802C2750) from Popo's side: the rope goes, and with it
/// his pointer and callbacks (ftPp_SpecialS_8012114C).
fn remove_rope(f: &mut Fighter) {
    rope_request(f, ItemControl::Remove);
    rope_gone(f);
}

/// ftPp_SpecialS_8012114C (8012114C): x2238, death3_cb and take_dmg_cb
/// cleared.
pub fn rope_gone(f: &mut Fighter) {
    let b = belay(f);
    b.rope_out = false;
    b.rope_take_damage = false;
    b.rope_death = false;
}

/// ftPp_SpecialS_80121164 (80121164), from ftPp_Init_8011F060: a rope
/// still out goes.
pub fn drop_rope(f: &mut Fighter) {
    if belay(f).rope_out {
        remove_rope(f);
    }
}

/// The rope's on_accessory for its motion state (fn_802C28B8, fn_802C28DC,
/// fn_802C29E8, fn_802C2AF4), each at Popo's left hand as it is posed now
/// (HSD_JObjSetupMatrix on the links' jobj). Returns the item's new
/// state when the reel-in ends (it_2725_Logic70_PickedUp).
pub fn rope_accessory(
    f: &mut Fighter,
    _assets: &FighterAssets,
    _map: &mut melee_mp::CollMap,
) -> Option<u16> {
    if !belay(f).rope_out {
        return None;
    }
    let anchor = belay(f).anchor;
    let hand = hand_position(f);
    let r = rope(f);
    match r.phase {
        rope::phase::HELD => r.hold(),
        rope::phase::PAYING_OUT => {
            r.release_tail();
            r.pay_out(anchor, hand);
        }
        rope::phase::HANGING => {
            r.release_tail();
            r.hang(anchor, hand);
        }
        rope::phase::REELING => {
            if r.reel(hand) {
                r.phase = rope::phase::HELD;
                return Some(rope::phase::HELD);
            }
        }
        phase => unreachable!("rope phase {phase}"),
    }
    None
}

/// ftPp_SpecialHi_8012280C (8012280C): Popo leaves the ground (every jump
/// spent) or spends his jumps, heads for Nana (ftPp_SpecialS_80120E68)
/// and climbs; x21F8 = ftCommon_8007F76C.
fn climb(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if is_air(f) {
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    } else {
        f.leave_ground_with_spent_jumps();
    }
    aim_at_partner(f);
    f.change_motion_state(AIR_CLIMB, assets)?;
    f.set_cape_turn_end(melee_ft::fighter::cape_turn::CapeTurnEnd::SpeedForward);
    Ok(())
}

/// ftPp_SpecialS_80120E68 (80120E68): toward a point behind and above
/// Nana, normalised (lbVector_Normalize), at x94 plus one per x98 of their
/// distance; Popo faces his way.
fn aim_at_partner(f: &mut Fighter) {
    let Some(nana) = climber::payload(f).partner else {
        return;
    };
    let (base, per_distance) = {
        let a = attrs(f);
        (a.climb_base_speed, a.climb_distance_per_speed)
    };
    let own = f.physics.position;
    let mut v = Vec3::new(nana.position.x - own.x, nana.position.y - own.y, 0.0);
    // retail 80120ED8: fnmsubs.
    v.x = fnmsubs(CLIMB_AIM_BEHIND, nana.facing, v.x);
    v.y += CLIMB_AIM_ABOVE;
    let mut v = melee_lb::vector::normalize(v);
    // 80120F04..80120F18: separate fsubs, fmuls and fadds; the inline
    // sqrtf; fdivs.
    let dx = own.x - nana.position.x;
    let dy = own.y - nana.position.y;
    let distance = sqrtf(dx * dx + dy * dy) / per_distance;
    v.x *= base + distance;
    v.y *= base + distance;
    f.physics.self_velocity = v;
    f.physics.facing = if v.x > 0.0 { 1.0 } else { -1.0 };
}

/// ft_80084F3C for the grounded rows; the start, throw and climb also
/// note where Nana's hand is.
fn ground_physics<const ROPE: bool>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    if ROPE {
        note_anchor(f);
    }
}

/// Nana's accessory4 (fn_80123218) reached Popo: her left hand, where the
/// rope's far end hangs (u.pp.x2240).
pub fn note_rope_anchor(f: &mut Fighter, hand: Vec3) {
    belay(f).anchor = hand;
}

/// The start's and throw's rope anchor: Nana's left hand while she is in
/// her Belay (361..366), else zero.
fn note_anchor(f: &mut Fighter) {
    let anchor = climber::payload(f)
        .partner
        .filter(|nana| partner::BELAY.contains(&nana.action.0))
        .and_then(|nana| nana.part_position)
        .unwrap_or(Vec3::ZERO);
    belay(f).anchor = anchor;
}

/// ftCommon_Fall (8007D494): gravity, then the terminal-speed clamp.
fn fall(f: &mut Fighter, gravity: f32, terminal: f32) {
    f.physics.self_velocity.y =
        melee_ft::physics::airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
}

/// ftCommon_8007CEF4 (8007CEF4): the fighter's aerial friction.
fn aerial_friction(f: &mut Fighter) {
    let friction = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, friction);
}

/// ftCommon_8007D344 (8007D344) with no threshold: drift toward the
/// stick through ftCommon_8007D140.
fn drift(f: &mut Fighter, acceleration: f32, maximum: f32) {
    let x = f.input.current.stick.x;
    let air = &f.core.attributes.air;
    f.core.physics.animation_velocity.x = melee_ft::physics::airborne::drift_acceleration(
        f.core.physics.self_velocity.x,
        x * acceleration,
        x * maximum,
        air,
    );
}

/// ftPp_SpecialAirHiStart_0_Phys / ftPp_SpecialAirHiThrow_0_Phys: gravity
/// x8C (terminal x90), aerial friction, the anchor.
fn throw_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let a = attrs(f);
        (a.throw_gravity, a.throw_terminal_velocity)
    };
    fall(f, gravity, terminal);
    aerial_friction(f);
    note_anchor(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPp_SpecialAirHiThrow2_Phys (80122538): gravity x9C (terminal xA0);
/// past the stick threshold the climb drifts (the common drift scaled by
/// xB0 and xB4, fmuls), else falling it slows; the anchor.
fn climb_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = *attrs(f);
    fall(f, a.climb_gravity, a.climb_terminal_velocity);
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    if magnitude > a.stick_threshold {
        let air = &f.attributes.air;
        let (acceleration, maximum) = (
            air.air_drift_stick_mul * a.climb_drift_scale,
            air.air_drift_max * a.climb_drift_max_scale,
        );
        drift(f, acceleration, maximum);
    } else if f.physics.self_velocity.y < 0.0 {
        aerial_friction(f);
    }
    note_anchor(f);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPp_SpecialAirHiStart_1_Phys / ftPp_SpecialAirHiThrow_1_Phys: gravity
/// xA8 (terminal xAC); falling, aerial friction.
fn solo_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal) = {
        let a = attrs(f);
        (a.solo_gravity, a.solo_terminal_velocity)
    };
    fall(f, gravity, terminal);
    if f.physics.self_velocity.y < 0.0 {
        aerial_friction(f);
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge; false off the floor.
fn stays_on_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    ground::map_escape(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported
}

/// ft_CheckGroundAndLedge (800822A4) toward the fighter's facing.
fn lands_facing(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    )
}

/// Off the floor: every jump spent (ftCommon_8007D60C) and the aerial
/// counterpart at the current frame (801217EC, 80121CE0, 801227AC,
/// 80121FD8, 801222E8).
fn to_air(f: &mut Fighter, p: &mut CollisionPhase<'_>, state: ActionId) -> Result<()> {
    if stays_on_edge(f, p) {
        return Ok(());
    }
    let assets = p.assets.expect("Belay collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)
}

/// Landing: the grounded counterpart at the current frame
/// (ftCommon_AirToGroundStateChange: 8012184C, 80121D40, 80122038);
/// otherwise a ledge (ftCliffCommon_80081298).
fn to_ground(f: &mut Fighter, p: &mut CollisionPhase<'_>, state: ActionId) -> Result<()> {
    let assets = p.assets.expect("Belay collision assets");
    if lands_facing(f, p) {
        f.land();
        let frame = f.animation.frame;
        f.change_motion_state_with_flags(state, assets, GROUND_AIR_FLAGS, frame, 1.0)
    } else {
        f.try_grab_ledge(assets, p.map).map(|_| ())
    }
}

/// Landing: LandingFallSpecial with x78's lag
/// (ftCo_LandingFallSpecial_Enter(gobj, false, x78)); otherwise a ledge.
/// True when neither happened.
fn land_special(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> Result<bool> {
    let assets = p.assets.expect("Belay collision assets");
    if lands_facing(f, p) {
        let lag = attrs(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
        return Ok(false);
    }
    Ok(!f.try_grab_ledge(assets, p.map)?)
}

/// ftPp_SpecialHiStart_0_Coll (80121740).
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_air(f, &mut p, AIR_START)
}
/// ftPp_SpecialAirHiStart_0_Coll (8012177C).
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_ground(f, &mut p, START)
}
/// ftPp_SpecialHiThrow_0_Coll (80121C34).
fn throw_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_air(f, &mut p, AIR_THROW)
}
/// ftPp_SpecialAirHiThrow_0_Coll (80121C70).
fn throw_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_ground(f, &mut p, THROW)
}
/// ftPp_SpecialHiThrow2_Coll (80122664).
fn climb_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_air(f, &mut p, AIR_CLIMB)
}
/// ftPp_SpecialAirHiThrow2_Coll (801226A0): landing or a ledge; else a
/// wall on the side Popo moves toward stops him, and a ceiling stops his
/// rise and drops him into the special fall.
fn climb_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !land_special(f, &mut p)? {
        return Ok(());
    }
    let env = f.collision.data.env_flags as u32;
    let vx = f.physics.self_velocity.x;
    if (env & collide::LEFT_WALL_MASK != 0 && vx > 0.0)
        || (env & collide::RIGHT_WALL_MASK != 0 && vx < 0.0)
    {
        f.physics.self_velocity.x = 0.0;
    } else if env & collide::CEILING_MASK != 0 {
        f.physics.self_velocity.y = 0.0;
        let assets = p.assets.expect("Belay collision assets");
        finish::<true>(f, assets)?;
    }
    Ok(())
}
/// ftPp_SpecialHiStart_1_Coll (80121F2C).
fn solo_start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_air(f, &mut p, AIR_SOLO_START)
}
/// ftPp_SpecialAirHiStart_1_Coll (80121F68).
fn solo_start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_ground(f, &mut p, SOLO_START)
}
/// ftPp_SpecialHiThrow_1_Coll (80122228).
fn solo_throw_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    to_air(f, &mut p, AIR_SOLO_THROW)
}
/// ftPp_SpecialAirHiThrow_1_Coll (80122264).
fn solo_throw_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_special(f, &mut p).map(|_| ())
}
