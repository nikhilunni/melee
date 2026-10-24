//! Charge Shot: ftsamusspecialn.c (801291A8..8012A068) and Kirby's copy,
//! ftkirbyspecialsamus.c (800FCC14..800FDB78). The two files differ only in
//! what [`ChargeShotUser`] names: the rows, the shot's item kind, the hand
//! and muzzle parts, the full-charge colour, the muzzle flash, the
//! attributes and the words that keep the charge, and whether the charge's
//! effects are destroyed only when flagged.
//!
//! The start creates the shot in the right hand on the script's
//! cmd_vars[0]. On the ground an unfinished charge continues in the hold,
//! which gains a level every `frames_per_level` frames up to `max_charge`
//! (the level kept between charges); B fires, a shield press cancels and a
//! roll leaves. In the air, or at full charge, the start fires at once: on
//! the script's cmd_vars[1] the shot leaves the muzzle with the kept charge
//! and the level resets.
use crate::motion::{self, change, GROUND_AIR};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{Aim, ItemControl, ItemRequest, Launch, SpawnItem};
use melee_types::{CommonMotionState, ItemKind};

/// Charge Shot's attributes: Samus +18..+28, Kirby +168..+178
/// (`specialn_ss_*`), the same four words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeShotAttributes {
    /// x18 / specialn_ss_charge_time: full charge, in levels (a float
    /// compared with the integer level).
    pub max_charge: f32,
    /// x1C / specialn_ss_aerial_shot_recoil: aerial recoil per level.
    pub recoil_per_level: f32,
    /// x20 / specialn_ss_frames_per_charge_level.
    pub frames_per_level: i32,
    /// x24 / specialn_ss_freefall_toggle: landing lag after an aerial shot
    /// (0: plain fall).
    pub landing_lag: f32,
}

/// mv.ss.unk3 / mv.kb.specialhi (as ftkirbyspecialsamus.c uses it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeShot {
    /// x0: the charge began in the air (or went airborne): fire at once.
    pub airborne: bool,
    /// x4: frames toward the next level.
    pub frames: i32,
}

/// The words that outlive the motion: u.ss.x222C..x2234 for Samus,
/// u.kb.xA4..xAC for Kirby.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeState {
    /// x2230 / xA8: the charge level kept between shots.
    pub level: i32,
    /// x222C / xA4: the shot in the hand.
    pub article: bool,
    /// x2234 / xAC: the charge's effects are flagged alive.
    pub effects: bool,
    /// The motion's scratch.
    pub motion: ChargeShot,
}

/// Who charges: Samus, and Kirby with her copied ability.
pub trait ChargeShotUser: 'static {
    /// SpecialNStart; Hold, Cancel, SpecialN, SpecialAirNStart and
    /// SpecialAirN are the next five rows.
    const START: ActionId;
    /// SpecialNStart's submotion; the other rows' follow.
    const START_ANIMATION: i32;
    /// It_Kind_Samus_Charge / It_Kind_Kirby_SamusCharge.
    const SHOT: ItemKind;
    /// `fp->parts[...]` the shot forms at (4 along its Z) and leaves from.
    const HAND_PART: usize;
    const MUZZLE_PART: usize;
    /// ftCo_800BFFD0's colour as the hold reaches the full charge.
    const FULL_CHARGE_COLOR: u8;
    /// efSync_Spawn's muzzle flash (efAlt 0x486 / 0x4A4).
    const MUZZLE_FLASH: u16;
    fn attributes(f: &Fighter) -> ChargeShotAttributes;
    fn state(f: &Fighter) -> &ChargeState;
    fn state_mut(f: &mut Fighter) -> &mut ChargeState;
    /// The take-damage and death callbacks (ftSamus_updateDamageDeathCBs,
    /// ftKb_SpecialN_set_cbs).
    fn arm_callbacks(f: &mut Fighter);
    /// The kept level changed: the kind's ftData_UnkMotionStates4 colour
    /// (`CombatState::secondary_color_fallback`) follows it.
    fn level_changed(f: &mut Fighter);
    /// Whether the charge's efLib_DestroyAll runs, clearing the flag:
    /// Samus's ftSamus_destroyAllEF tests x2234, Kirby's copy always runs
    /// it (ftkirbyspecialsamus.c:138-145).
    fn take_charge_effects(f: &mut Fighter) -> bool;
}

const START: usize = 0;
const HOLD: usize = 1;
const CANCEL: usize = 2;
const FIRE: usize = 3;
const AIR_START: usize = 4;
const AIR_FIRE: usize = 5;

/// 801292E4 / ftKb_SsSpecialNStart_Anim: the shot forms 4 units along the
/// hand joint's Z.
const HAND_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 4.0);

fn action<C: ChargeShotUser>(row: usize) -> ActionId {
    ActionId(C::START.0 + row as u16)
}

const fn row<C: ChargeShotUser>(
    index: usize,
    anim: state::AnimFn,
    iasa: state::InputFn,
    physics: state::PhysicsFn,
    collision: state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: ActionId(C::START.0 + index as u16),
        id: CommonMotionState::None,
        animation: C::START_ANIMATION + index as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The six rows for `C`.
pub const fn rows<C: ChargeShotUser>() -> [MotionRow; 6] {
    [
        row::<C>(
            START,
            start_anim::<C>,
            motion::no_input,
            motion::ground_friction,
            start_collision::<C>,
        ),
        row::<C>(
            HOLD,
            hold_anim::<C>,
            hold_input::<C>,
            motion::ground_friction,
            hold_collision::<C>,
        ),
        row::<C>(
            CANCEL,
            cancel_anim::<C>,
            motion::no_input,
            motion::ground_friction,
            cancel_collision::<C>,
        ),
        row::<C>(
            FIRE,
            fire_anim::<C>,
            motion::no_input,
            motion::ground_friction,
            fire_collision::<C>,
        ),
        row::<C>(
            AIR_START,
            air_start_anim::<C>,
            motion::no_input,
            motion::air_physics,
            air_start_collision::<C>,
        ),
        row::<C>(
            AIR_FIRE,
            air_fire_anim::<C>,
            motion::no_input,
            motion::air_physics,
            air_fire_collision::<C>,
        ),
    ]
}

fn scratch<C: ChargeShotUser>(f: &mut Fighter) -> &mut ChargeShot {
    &mut C::state_mut(f).motion
}

/// ftSs_SpecialN_Enter (8012954C) / ftSs_SpecialAirN_Enter (801295F0);
/// ftKb_SpecialNSs_800FCF74 / 800FD020.
pub fn enter<C: ChargeShotUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let state = action::<C>(if air { AIR_START } else { START });
    change(f, state, MotionEntryFlags(0), 0.0, 1.0, a).expect("Charge Shot assets");
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    C::arm_callbacks(f);
    *scratch::<C>(f) = ChargeShot {
        airborne: air,
        frames: 0,
    };
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// The kept charge level, and the kind's colour that follows it.
pub fn set_charge_level<C: ChargeShotUser>(f: &mut Fighter, level: i32) {
    C::state_mut(f).level = level;
    C::level_changed(f);
}

/// The kept level is the full level: the float x18 against the integer
/// level converted.
pub fn is_full<C: ChargeShotUser>(f: &Fighter) -> bool {
    C::state(f).level as f32 == C::attributes(f).max_charge
}

/// The shot's full level, fctiwz of the float x18.
pub fn full_charge<C: ChargeShotUser>(f: &Fighter) -> i32 {
    gekko_math::msl::fctiwz(C::attributes(f).max_charge)
}

/// ftSs_SpecialN_801292E4 (801292E4) / ftKb_SsSpecialNStart_Anim's head:
/// on the script's cmd_vars[0], with no shot yet, the shot forms at the
/// right hand (it_802B55C8, then Item_8026AB54 into the hand) and the
/// callbacks go in.
fn form_shot<C: ChargeShotUser>(f: &mut Fighter) {
    if f.commands.variables[0] != 1 || C::state(f).article {
        return;
    }
    f.commands.variables[0] = 0;
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::HAND_PART,
        HAND_OFFSET,
    );
    position.z = 0.0;
    // it_802B55C8: prev_pos on the stage plane, pos it_8026BB68's ECB
    // midpoint (ftLib_80086990: fadds, fmuls, fadds), x44_flag.b0.
    let mut spawn = SpawnItem::ray(C::SHOT, c.player.id, position, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: C::HAND_PART as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    C::state_mut(f).article = true;
    C::arm_callbacks(f);
}

/// ftSs_SpecialN_801293BC (801293BC) / ftKb_SpecialNSs_800FCDE0: on the
/// script's cmd_vars[1], with a shot, it leaves the muzzle (it_802B56E4)
/// facing right (0) or left (M_PI) with the kept charge; an aerial shot
/// pushes back by the recoil per level; the charge resets, the shot is let
/// go and the muzzle flashes.
fn fire_shot<C: ChargeShotUser>(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[1] != 1 || !C::state(f).article {
        return;
    }
    f.commands.variables[1] = 2;
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::MUZZLE_PART,
        Vec3::ZERO,
    );
    position.z = 0.0;
    let angle = if c.physics.facing == 1.0 {
        0.0
    } else {
        std::f64::consts::PI as f32
    };
    let level = C::state(f).level as u32;
    let full = C::attributes(f).max_charge;
    let holder = f.core.item_holder(C::HAND_PART as u8, assets);
    // ftLib_80086630: the hand joint's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let launch = Launch {
        velocity: Vec3::ZERO,
        offset: Vec3::ZERO,
        spin_degrees: 0.0,
        hand,
        center: holder.center,
        attack: holder.attack,
        attack_stale: holder.attack_stale,
        aim: Some(Aim {
            position,
            angle,
            charge: level as f32,
            full_charge: full,
            facing: f.physics.facing,
        }),
        angle: 0.0,
        long_lifetime: false,
        shot: None,
    };
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind: C::SHOT,
        launch,
    });
    if f.motion_state.action == action::<C>(AIR_FIRE)
        || f.physics.ground_or_air == melee_types::GroundOrAir::Air
    {
        // Retail 0x8012949C / 0x800FCEEC: separate fmuls, level first.
        let recoil = C::attributes(f).recoil_per_level;
        let level = C::state(f).level as f32;
        f.physics.self_velocity.x = f.physics.facing * (recoil * level);
    }
    set_charge_level::<C>(f, 0);
    // The shot's own efLib_DestroyAll (it_802B5CBC, from it_802B56E4)
    // removes the hand glow's generators before the owner's (Samus:
    // ftSs_SpecialN_801291F0 inlined, efLib_DestroyAll at 0x80129504;
    // Kirby: 0x800FCF2C) and the flash (efSync_Spawn at 0x80129524 /
    // 0x800FCF4C): the flash's generators go in behind the head the glow
    // left, so both wait for the launch request.
    C::state_mut(f).article = false;
    if C::take_charge_effects(f) {
        f.core.effects_after_items.push(EffectRequest::DestroyOwned);
    }
    f.core
        .effects_after_items
        .push(EffectRequest::PositionalModel {
            id: C::MUZZLE_FLASH,
            position,
        });
}

/// ftSs_SpecialN_801291F0 (801291F0) / ftKb_SpecialNSs_800FCD04
/// (800FCD04): the shot is no longer the owner's, and the charge's
/// effects go.
pub fn let_go<C: ChargeShotUser>(f: &mut Fighter) {
    C::state_mut(f).article = false;
    destroy_charge_effects::<C>(f);
}

/// ftSamus_destroyAllEF / ftKb_SpecialNSs_DestroyChargeShot.
fn destroy_charge_effects<C: ChargeShotUser>(f: &mut Fighter) {
    if C::take_charge_effects(f) {
        f.core.effects.push(EffectRequest::DestroyOwned);
    }
}

/// ftSamus_UnkAndDestroyAllEF / ftKb_ChargeShot_inline: the shot in hand
/// is destroyed (it_802B5974), then the charge's effects.
pub fn drop_shot<C: ChargeShotUser>(f: &mut Fighter) {
    if std::mem::take(&mut C::state_mut(f).article) {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: C::SHOT,
            control: ItemControl::Remove,
        });
    }
    destroy_charge_effects::<C>(f);
}

/// ftSs_SpecialNStart_Anim (80129684) / ftKb_SsSpecialNStart_Anim
/// (800FD0BC): the shot forms; at the end an aerial or full charge fires,
/// otherwise the hold (x2234 / xAC cleared).
fn start_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_shot::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        if scratch::<C>(f).airborne || is_full::<C>(f) {
            change(
                f,
                action::<C>(FIRE),
                MotionEntryFlags(0),
                0.0,
                1.0,
                p.assets,
            )?;
        } else {
            change(
                f,
                action::<C>(HOLD),
                MotionEntryFlags(0),
                0.0,
                1.0,
                p.assets,
            )?;
            C::state_mut(f).effects = false;
        }
        C::arm_callbacks(f);
    }
    Ok(None)
}

/// ftSs_SpecialNHold_Anim (80129774) / ftKb_SsSpecialNHold_Anim: a level
/// every `frames_per_level` frames; the full level flashes and ends the
/// hold in the cancel, the shot kept as a charge.
fn hold_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // cmd_vars[2]'s charge sound is presentation.
    f.commands.variables[2] = 0;
    let attributes = C::attributes(f);
    let s = scratch::<C>(f);
    s.frames += 1;
    if s.frames > attributes.frames_per_level {
        s.frames = 0;
        let level = C::state(f).level + 1;
        set_charge_level::<C>(f, level);
        if level as f32 >= attributes.max_charge {
            f.core
                .install_color_overlay_now(C::FULL_CHARGE_COLOR, p.assets);
            set_charge_level::<C>(f, gekko_math::msl::fctiwz(attributes.max_charge));
            motion::seal_graphics(f, p.assets, p.rng);
            change(
                f,
                action::<C>(CANCEL),
                MotionEntryFlags(0),
                0.0,
                1.0,
                p.assets,
            )?;
            drop_shot::<C>(f);
            C::arm_callbacks(f);
        }
    }
    Ok(None)
}

/// ftSs_SpecialNCancel_Anim (80129940) / ftKb_SsSpecialNCancel_Anim: the
/// shot goes (every frame); Wait at the end.
fn cancel_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    drop_shot::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        motion::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialN_Anim (801299D0) / ftKb_SsSpecialN_Anim: the shot fires;
/// Wait at the end.
fn fire_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    fire_shot::<C>(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        motion::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirNStart_Anim (80129A14) / ftKb_SsSpecialAirNStart_Anim:
/// the shot forms; the aerial fire follows at the end.
fn air_start_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    form_shot::<C>(f);
    scratch::<C>(f).airborne = true;
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        change(
            f,
            action::<C>(AIR_FIRE),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )?;
        C::arm_callbacks(f);
    }
    Ok(None)
}

/// ftSs_SpecialAirN_Anim (80129A98) / ftKb_SsSpecialAirN_Anim: the shot
/// fires; Fall at the end, or special fall with the landing lag
/// (ftCo_80096900(1, 0, 1, 1, lag)).
fn air_fire_anim<C: ChargeShotUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    fire_shot::<C>(f, p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = C::attributes(f).landing_lag;
        motion::seal_graphics(f, p.assets, p.rng);
        if lag == 0.0 {
            motion::fall(f, p.assets)?;
        } else {
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftSs_SpecialNHold_IASA (80129B1C) / ftKb_SsSpecialNHold_IASA: a roll
/// (ftCo_8009917C) drops the shot; otherwise B fires and a shield press
/// cancels. The cancel tests the input proc's shield bit (retail
/// 0x80129BD8: clrrwi. r0, r3, 31), which a digital shoulder, an analog
/// trigger past the deadzone or Z all set.
fn hold_input<C: ChargeShotUser>(f: &mut Fighter, p: InputPhase<'_>) {
    if let Some(roll) = f.core.roll_input(p.assets) {
        f.enter_escape(p.assets, roll)
            .expect("roll out of the charge");
        drop_shot::<C>(f);
        return;
    }
    let pressed = f.input.pressed;
    if pressed.intersects(Buttons::B) {
        change(
            f,
            action::<C>(FIRE),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )
        .expect("Charge Shot fire");
        C::arm_callbacks(f);
        return;
    }
    if pressed.intersects(Buttons::SHIELD) {
        change(
            f,
            action::<C>(CANCEL),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )
        .expect("Charge Shot cancel");
        drop_shot::<C>(f);
        C::arm_callbacks(f);
    }
}

/// The grounded rows' ft_80082708: off the floor, the aerial counterpart
/// (ftCommon_GroundToAirStateChange) and the callbacks.
fn leave_floor<C: ChargeShotUser>(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    row: usize,
) -> Result<bool> {
    if motion::stays_grounded(f, p) {
        return Ok(false);
    }
    let assets = p.assets.expect("Charge Shot collision assets");
    motion::ground_to_air(f, action::<C>(row), GROUND_AIR, assets)?;
    C::arm_callbacks(f);
    Ok(true)
}

/// ftSs_SpecialNStart_Coll (80129D48).
fn start_collision<C: ChargeShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor::<C>(f, &mut p, AIR_START).map(|_| ())
}

/// ftSs_SpecialNHold_Coll (80129DC8): off the floor the shot fires at once
/// in the air (cmd_vars[1] = 1).
fn hold_collision<C: ChargeShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if leave_floor::<C>(f, &mut p, AIR_FIRE)? {
        f.commands.variables[1] = 1;
    }
    Ok(())
}

/// ftSs_SpecialNCancel_Coll (80129E68): off the floor, the aerial fire.
fn cancel_collision<C: ChargeShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor::<C>(f, &mut p, AIR_FIRE).map(|_| ())
}

/// ftSs_SpecialN_Coll (80129EE8).
fn fire_collision<C: ChargeShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    leave_floor::<C>(f, &mut p, AIR_FIRE).map(|_| ())
}

/// The aerial rows' ft_80081D0C: landing continues on the ground
/// (ftCommon_AirToGroundStateChange).
fn land_as<C: ChargeShotUser>(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    row: usize,
) -> Result<()> {
    if !motion::lands(f, p) {
        return Ok(());
    }
    let assets = p.assets.expect("Charge Shot landing assets");
    motion::air_to_ground(f, action::<C>(row), GROUND_AIR, assets)?;
    C::arm_callbacks(f);
    Ok(())
}

/// ftSs_SpecialAirNStart_Coll (80129F68).
fn air_start_collision<C: ChargeShotUser>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    land_as::<C>(f, &mut p, START)
}

/// ftSs_SpecialAirN_Coll (80129FE8).
fn air_fire_collision<C: ChargeShotUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    land_as::<C>(f, &mut p, FIRE)
}
