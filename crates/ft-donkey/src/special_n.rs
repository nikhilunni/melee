//! Giant Punch, ftdonkeyspecialn.c (8010E840..8010F81C).
//!
//! The wind-up loop counts arm swings into the fighter's stored charge
//! (u.dk.x222C), which survives leaving the move: a shield press cancels at
//! the loop's wrap, a full charge walks away glowing. B (or a second neutral
//! B with a full charge) punches with the stored swings, which add damage to
//! both hitboxes and speed to the grounded lunge.
use crate::{
    common::{self, change, row},
    init::DonkeyKong,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_types::FtPart;

/// ftDk_MS_SpecialNStart (369) .. ftDk_MS_SpecialAirNFull (378).
pub const START: ActionId = ActionId(369);
pub const LOOP: ActionId = ActionId(370);
pub const CANCEL: ActionId = ActionId(371);
pub const PUNCH: ActionId = ActionId(372);
pub const FULL: ActionId = ActionId(373);
pub const AIR_START: ActionId = ActionId(374);
pub const AIR_LOOP: ActionId = ActionId(375);
pub const AIR_CANCEL: ActionId = ActionId(376);
pub const AIR_PUNCH: ActionId = ActionId(377);
pub const AIR_FULL: ActionId = ActionId(378);

/// The aerial rows follow the grounded ones in the same order.
const AIR_OFFSET: u16 = AIR_START.0 - START.0;
/// ftDk_SM_SpecialNStart: row 369's anim_id (0x803CB838).
const FIRST_ANIMATION: i32 = 0x13F;

/// ftCo_800BFFD0(fp, 57, 0): the full charge's glow.
pub const FULL_CHARGE_COLOR: u8 = 57;
/// efSync_Spawn(1224 / 1225, gobj, TopN, &facing): the punch's models
/// 0x1F42 (grounded) and 0x1F43 (aerial).
const PUNCH_EFFECT: u16 = 0x4C8;
const AIR_PUNCH_EFFECT: u16 = 0x4C9;

/// ftDk_MF_SpecialN_CollCancel (ftDonkey/forward.h:31): the wind-up rows
/// cross the floor's edge with ftCommon_GroundAirColl_MF alone.
const WINDUP_GROUND_AIR: MotionEntryFlags = common::GROUND_AIR;
/// ftDk_MF_SpecialN_Coll (ftDonkey/forward.h:34): the punches also keep
/// their effects, colour-animation hit status and hitboxes (KeepGfx |
/// KeepColAnimHitStatus | KeepColAnimPartHitStatus | SkipHit).
/// KeepColAnimPartHitStatus (bit 16) is left out: it only spares x2221_b1's
/// invincibility (ftCo_Damage.c:452, fighter.c:1007), which the move's own
/// entry has already ended.
const PUNCH_GROUND_AIR: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR.0 | common::KEEP_GFX | (1 << 2) | common::SKIP_HIT);

/// mv.dk.specialn (ftDonkey/types.h).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiantPunch {
    /// x0: a shield press asked for the cancel at the loop's next wrap.
    pub cancel: bool,
    /// x4: the grounded lunge, 0 before the hitbox, 1 for the frame it
    /// appears, 2 once the speed is set.
    pub lunge: i32,
    /// x8: the punch model, 0 before the script's cue, 1 for the frame it
    /// is due, 2 once spawned.
    pub effect: i32,
    /// xC: the swings this punch spends.
    pub swings: i32,
    /// x10, x14: the two hitboxes' script damage, -1 until first seen.
    pub base_damage: [i32; 2],
}
impl Default for GiantPunch {
    fn default() -> Self {
        Self {
            cancel: false,
            lunge: 0,
            effect: 0,
            swings: 0,
            base_damage: [-1; 2],
        }
    }
}

pub const fn rows() -> [MotionRow; 10] {
    const fn animation(action: ActionId) -> i32 {
        FIRST_ANIMATION + (action.0 - START.0) as i32
    }
    [
        row(
            START,
            animation(START),
            start_anim::<false>,
            common::no_input,
            common::ground_friction,
            windup_collision::<false>,
        ),
        row(
            LOOP,
            animation(LOOP),
            loop_anim::<false>,
            loop_input::<false>,
            common::ground_friction,
            windup_collision::<false>,
        ),
        row(
            CANCEL,
            animation(CANCEL),
            cancel_anim::<false>,
            common::no_input,
            common::ground_friction,
            windup_collision::<false>,
        ),
        row(
            PUNCH,
            animation(PUNCH),
            punch_anim::<false, true>,
            common::no_input,
            common::ground_friction,
            punch_collision::<false>,
        ),
        row(
            FULL,
            animation(FULL),
            punch_anim::<false, false>,
            common::no_input,
            common::ground_friction,
            punch_collision::<false>,
        ),
        row(
            AIR_START,
            animation(AIR_START),
            start_anim::<true>,
            common::no_input,
            common::air_friction_fall,
            windup_collision::<true>,
        ),
        row(
            AIR_LOOP,
            animation(AIR_LOOP),
            loop_anim::<true>,
            loop_input::<true>,
            common::air_friction_fall,
            windup_collision::<true>,
        ),
        row(
            AIR_CANCEL,
            animation(AIR_CANCEL),
            cancel_anim::<true>,
            common::no_input,
            common::air_friction_fall,
            windup_collision::<true>,
        ),
        row(
            AIR_PUNCH,
            animation(AIR_PUNCH),
            punch_anim::<true, true>,
            common::no_input,
            common::air_friction_fall,
            punch_collision::<true>,
        ),
        row(
            AIR_FULL,
            animation(AIR_FULL),
            punch_anim::<true, false>,
            common::no_input,
            common::air_friction_fall,
            punch_collision::<true>,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::GiantPunchAttributes {
    &f.character.get::<DonkeyKong>().attributes.giant_punch
}

fn scratch(f: &mut Fighter) -> &mut GiantPunch {
    &mut f.character.get_mut::<DonkeyKong>().giant_punch
}

/// The stored swings (u.dk.x222C), and ftDk_Init_UnkMotionStates4's glow
/// (colour 57 whenever the secondary colour slot empties) while they are
/// the full charge.
pub fn set_swings(f: &mut Fighter, swings: i32) {
    f.character.get_mut::<DonkeyKong>().punch_swings = swings;
    let full = swings == attributes(f).max_swings;
    f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
}

fn swings(f: &Fighter) -> i32 {
    f.character.get::<DonkeyKong>().punch_swings
}

/// setCallbacks: take_dmg_cb and death2_cb = ftDk_Init_8010D774,
/// take_dmg_2_cb = ftDk_SpecialN_DestroyAllEffects, and the efLib hitlag
/// pair.
fn set_callbacks(f: &mut Fighter) {
    common::install_damage_callbacks(f);
    f.character.get_mut::<DonkeyKong>().hit_destroys_effects = true;
}

/// clearCallbacks: take_dmg_2_cb and the hitlag pair go; take_dmg_cb and
/// death2_cb stay until the motion change every caller makes next.
fn clear_callbacks(f: &mut Fighter) {
    f.character.get_mut::<DonkeyKong>().hit_destroys_effects = false;
    f.effect_state.hitlag_callbacks = false;
}

/// ftDk_SpecialN_Enter (8010E840) / ftDk_SpecialAirN_Enter (8010E930): a
/// full charge punches at once and is spent; anything less winds up.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let offset = if air { AIR_OFFSET } else { 0 };
    let stored = swings(f);
    let full = stored == attributes(f).max_swings;
    let state = if full { FULL } else { START };
    change(
        f,
        ActionId(state.0 + offset),
        MotionEntryFlags(0),
        0.0,
        1.0,
        a,
    )
    .expect("Giant Punch assets");
    *scratch(f) = GiantPunch {
        effect: i32::from(full),
        swings: if full { stored } else { 0 },
        ..GiantPunch::default()
    };
    if full {
        set_swings(f, 0);
    }
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    set_callbacks(f);
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftDk_SpecialNStart_Anim (8010EA28) / ftDk_SpecialAirNStart_Anim
/// (8010EDD4): the loop at the animation's end.
fn start_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if AIR { AIR_LOOP } else { LOOP };
        common::seal_graphics(f, p.assets, p.rng);
        change(f, state, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
        set_callbacks(f);
    }
    Ok(None)
}

/// ftDk_SpecialNLoop_Anim (8010EAA0) / ftDk_SpecialAirNLoop_Anim
/// (8010EE48): each wrap of the loop stores a swing; the last one glows
/// (ftCo_800BFFD0(57)) and ends the wind-up.
fn loop_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.animation.frame == 0.0 {
        let stored = swings(f) + 1;
        let maximum = attributes(f).max_swings;
        if stored >= maximum {
            f.core
                .install_color_overlay_now(FULL_CHARGE_COLOR, p.assets);
            set_swings(f, maximum);
            clear_callbacks(f);
            common::seal_graphics(f, p.assets, p.rng);
            finish(f, AIR, p.assets)?;
        } else {
            set_swings(f, stored);
        }
    }
    Ok(None)
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    if air {
        common::fall(f, assets)
    } else {
        common::wait(f, assets)
    }
}

/// ftDk_SpecialNCancel_Anim (8010EB44) / ftDk_SpecialAirNCancel_Anim
/// (8010EEEC).
fn cancel_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        clear_callbacks(f);
        common::seal_graphics(f, p.assets, p.rng);
        finish(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007ABD0 (8007ABD0): hitbox `index`'s damage from `damage`,
/// staled.
fn set_hitbox_damage(f: &mut Fighter, index: usize, damage: i32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: charged Giant Punch"
    );
    // 8007ABE8: the u32 argument converted to float.
    let damage = damage as u32 as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[index]
        .as_mut()
        .expect("Giant Punch hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}

/// The script's hitboxes take the stored swings' damage: each one's own
/// damage as first seen (the float truncated into x10 / x14), plus
/// x30_DAMAGE_PER_SWING per swing.
fn charge_hitboxes(f: &mut Fighter) {
    let bonus = scratch(f).swings * attributes(f).damage_per_swing;
    for index in 0..2 {
        if scratch(f).base_damage[index] == -1 {
            // x914[index] is read whatever its state: a capsule the script
            // has not made keeps its last damage.
            let damage = f.commands.hitboxes[index]
                .as_ref()
                .map(|hit| hit.descriptor.damage)
                .unwrap_or_else(|| {
                    unimplemented!(
                        "ftdonkeyspecialn.c:165: Giant Punch hitbox {index} not created with hitbox 0"
                    )
                });
            scratch(f).base_damage[index] = gekko_math::msl::fctiwz(damage);
        }
        let base = scratch(f).base_damage[index];
        set_hitbox_damage(f, index, base + bonus);
    }
}

/// ftDk_SpecialN_Anim (8010EB98), ftDk_SpecialNFull_Anim (8010ED04),
/// ftDk_SpecialAirN_Anim (8010EF40) and ftDk_SpecialAirNFull_Anim
/// (8010F0D0). `CHARGED`: the partial punch scales its hitboxes (the full
/// punch's script carries its own damage). Only the grounded rows lunge.
fn punch_anim<const AIR: bool, const CHARGED: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && scratch(f).effect == 0 {
        scratch(f).effect = 1;
    }
    if scratch(f).effect == 1 {
        scratch(f).effect = 2;
        // The row decides nothing: a punch that crossed the floor's edge
        // spawns the model of where it is now.
        let id = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
            AIR_PUNCH_EFFECT
        } else {
            PUNCH_EFFECT
        };
        f.effects.push(EffectRequest::SyncAttached {
            id,
            bone: common::part(FtPart::TopN),
        });
    }
    if f.commands.hitboxes[0].is_some() {
        if !AIR && scratch(f).lunge == 0 {
            scratch(f).lunge = 1;
        }
        if CHARGED {
            charge_hitboxes(f);
        }
    }
    if scratch(f).lunge == 1 {
        scratch(f).lunge = 2;
        // updateVelocity (8010E7CC..): the int swings to float, fmuls by
        // x34, fmuls by the facing.
        let speed = attributes(f).punch_speed * scratch(f).swings as f32;
        f.physics.ground_velocity = f.physics.facing * speed;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        set_swings(f, 0);
        clear_callbacks(f);
        common::seal_graphics(f, p.assets, p.rng);
        if !AIR {
            return common::wait(f, p.assets).map(|()| None);
        }
        let lag = attributes(f).landing_lag;
        if lag == 0.0 {
            common::fall(f, p.assets)?;
        } else {
            // ftCo_80096900(gobj, 1, 0, 1, 1.0, lag).
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftDk_SpecialNLoop_IASA (8010F280) / ftDk_SpecialAirNLoop_IASA
/// (8010F3B8): the grounded loop first offers the roll (ftCo_8009917C); B
/// punches with the stored swings; a press of the shield bit asks for the
/// cancel, taken on the loop's wrap frame.
fn loop_input<const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !AIR {
        if let Some(roll) = f.core.roll_input(p.assets) {
            f.enter_escape(p.assets, roll)
                .expect("roll out of the wind-up");
            return;
        }
    }
    let pressed = f.input.pressed;
    if pressed.intersects(Buttons::B) {
        let state = if AIR { AIR_PUNCH } else { PUNCH };
        change(f, state, MotionEntryFlags(0), 0.0, 1.0, p.assets).expect("Giant Punch assets");
        scratch(f).swings = swings(f);
        set_swings(f, 0);
        set_callbacks(f);
        // ftAnim_8006EBA4.
        f.step_animation(p.assets);
    }
    // retail 0x8010F140 / 0x8010F284: clrrwi. r0, r0, 31, the input proc's
    // shield bit, which a digital shoulder, an analog trigger past the
    // deadzone or Z all set (the decomp's HSD_PAD_LR is wrong here).
    if pressed.intersects(Buttons::SHIELD) {
        scratch(f).cancel = true;
    }
    if f.animation.frame == 0.0 && scratch(f).cancel {
        let state = if AIR { AIR_CANCEL } else { CANCEL };
        change(f, state, MotionEntryFlags(0), 0.0, 1.0, p.assets).expect("Giant Punch assets");
        set_callbacks(f);
    }
}

/// ftDk_SpecialNStart/Loop/Cancel_Coll and their aerial counterparts:
/// ft_80082708 walks off the floor into the aerial row; ft_80081D0C lands
/// into the grounded one.
fn windup_collision<const AIR: bool>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    cross_floor::<AIR>(f, &mut p, false, WINDUP_GROUND_AIR)
}

/// ftDk_SpecialN_Coll / ftDk_SpecialNFull_Coll and their aerial
/// counterparts: ft_800827A0 stops at the floor's edge.
fn punch_collision<const AIR: bool>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    cross_floor::<AIR>(f, &mut p, true, PUNCH_GROUND_AIR)
}

fn cross_floor<const AIR: bool>(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    stop_at_edge: bool,
    flags: MotionEntryFlags,
) -> Result<()> {
    let action = f.motion_state.action;
    let assets = p.assets.expect("Giant Punch collision assets");
    if AIR {
        if common::lands(f, p) {
            common::air_to_ground(f, ActionId(action.0 - AIR_OFFSET), flags, assets)?;
            set_callbacks(f);
        }
        return Ok(());
    }
    let supported = if stop_at_edge {
        common::stays_on_edge(f, p)
    } else {
        common::stays_grounded(f, p)
    };
    if !supported {
        common::ground_to_air(f, ActionId(action.0 + AIR_OFFSET), flags, assets)?;
        set_callbacks(f);
    }
    Ok(())
}
