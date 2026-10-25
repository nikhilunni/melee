//! Giant Punch: ftdonkeyspecialn.c (8010E840..8010F81C) and Kirby's copy,
//! ftkirbyspecialdonkey.c (800FF8EC..80100DE0). The two files differ only
//! in what [`GiantPunchUser`] names: the rows, the full charge's colour, the
//! punch's models, the attributes, the word that stores the swings, and
//! the callbacks the states arm (Kirby's copy has no take_dmg_2_cb).
//!
//! The wind-up loop counts arm swings into the fighter's stored charge
//! (u.dk.x222C / u.kb.xBC), which survives leaving the move: a shield press
//! cancels at the loop's wrap, a full charge walks away glowing. B (or a
//! second neutral B with a full charge) punches with the stored swings,
//! which add damage to both hitboxes and speed to the grounded lunge.
use crate::motion::{self, change, row};
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

/// Giant Punch's attributes: Donkey Kong +2C..+3C, Kirby +190..+1A0
/// (`specialn_dk_*`), the same four words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GiantPunchAttributes {
    /// x2C_MAX_ARM_SWINGS / specialn_dk_swings_to_full_charge.
    pub max_swings: i32,
    /// x30_DAMAGE_PER_SWING / specialn_dk_damage_increase_per_swing.
    pub damage_per_swing: i32,
    /// x34_PUNCH_HORIZONTAL_VEL / specialn_dk_grounded_punch_horizontal_velocity,
    /// per swing.
    pub punch_speed: f32,
    /// x38_LANDING_LAG / specialn_dk_freefall_toggle after an aerial punch
    /// (0: plain fall).
    pub landing_lag: f32,
}

/// mv.dk.specialn (ftDonkey/types.h), which Kirby's copy uses too.
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

/// Who punches: Donkey Kong, and Kirby with his copied ability.
pub trait GiantPunchUser: 'static {
    /// SpecialNStart; Loop, Cancel, SpecialN, Full and the five aerial rows
    /// follow in that order.
    const START: ActionId;
    /// SpecialNStart's anim_id; the other rows' follow.
    const FIRST_ANIMATION: i32;
    /// ftCo_800BFFD0's colour as the loop reaches the full charge.
    const FULL_CHARGE_COLOR: u8;
    /// efSync_Spawn's punch models at TopN, grounded and aerial.
    const PUNCH_EFFECTS: [u16; 2];
    fn attributes(f: &Fighter) -> GiantPunchAttributes;
    fn scratch(f: &mut Fighter) -> &mut GiantPunch;
    /// The stored swings (u.dk.x222C / u.kb.xBC).
    fn swings(f: &Fighter) -> i32;
    /// Stores the swings; the kind's ftData_UnkMotionStates4 colour
    /// (`CombatState::secondary_color_fallback`) follows them.
    fn store_swings(f: &mut Fighter, swings: i32);
    /// setCallbacks / ftKb_DkSpecialNStart_Coll_inline: take_dmg_cb and
    /// death2_cb, Fighter_SetEffectHitlagCallbacks, and Donkey Kong's
    /// take_dmg_2_cb.
    fn arm_callbacks(f: &mut Fighter);
    /// clearCallbacks: the hitlag pair (and Donkey Kong's take_dmg_2_cb)
    /// go; take_dmg_cb and death2_cb stay until the motion change every
    /// caller makes next.
    fn clear_callbacks(f: &mut Fighter);
}

const START: u16 = 0;
const LOOP: u16 = 1;
const CANCEL: u16 = 2;
const PUNCH: u16 = 3;
const FULL: u16 = 4;
/// The aerial rows follow the grounded ones in the same order.
const AIR_OFFSET: u16 = 5;

fn action<C: GiantPunchUser>(row: u16, air: bool) -> ActionId {
    ActionId(C::START.0 + row + if air { AIR_OFFSET } else { 0 })
}

/// ftDk_MF_SpecialN_CollCancel (ftDonkey/forward.h:31): the wind-up rows
/// cross the floor's edge with ftCommon_GroundAirColl_MF alone.
const WINDUP_GROUND_AIR: MotionEntryFlags = motion::GROUND_AIR;
/// ftDk_MF_SpecialN_Coll (ftDonkey/forward.h:34): the punches also keep
/// their effects, colour-animation hit status and hitboxes (KeepGfx |
/// KeepColAnimHitStatus | KeepColAnimPartHitStatus | SkipHit).
/// KeepColAnimPartHitStatus (bit 16) is left out: it only spares x2221_b1's
/// invincibility (ftCo_Damage.c:452, fighter.c:1007), which the move's own
/// entry has already ended.
const PUNCH_GROUND_AIR: MotionEntryFlags =
    MotionEntryFlags(motion::GROUND_AIR.0 | motion::KEEP_GFX | (1 << 2) | motion::SKIP_HIT);

const fn windup_row<C: GiantPunchUser, const AIR: bool>(
    index: u16,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
) -> MotionRow {
    let index = index + if AIR { AIR_OFFSET } else { 0 };
    row(
        ActionId(C::START.0 + index),
        C::FIRST_ANIMATION + index as i32,
        anim,
        iasa,
        if AIR {
            motion::air_friction_fall
        } else {
            motion::ground_friction
        },
        windup_collision::<C, AIR>,
    )
}

const fn punch_row<C: GiantPunchUser, const AIR: bool, const CHARGED: bool>() -> MotionRow {
    let index = if CHARGED { PUNCH } else { FULL } + if AIR { AIR_OFFSET } else { 0 };
    row(
        ActionId(C::START.0 + index),
        C::FIRST_ANIMATION + index as i32,
        punch_anim::<C, AIR, CHARGED>,
        motion::no_input,
        if AIR {
            motion::air_friction_fall
        } else {
            motion::ground_friction
        },
        punch_collision::<C, AIR>,
    )
}

/// The ten rows for `C`.
pub const fn rows<C: GiantPunchUser>() -> [MotionRow; 10] {
    [
        windup_row::<C, false>(START, start_anim::<C, false>, motion::no_input),
        windup_row::<C, false>(LOOP, loop_anim::<C, false>, loop_input::<C, false>),
        windup_row::<C, false>(CANCEL, cancel_anim::<C, false>, motion::no_input),
        punch_row::<C, false, true>(),
        punch_row::<C, false, false>(),
        windup_row::<C, true>(START, start_anim::<C, true>, motion::no_input),
        windup_row::<C, true>(LOOP, loop_anim::<C, true>, loop_input::<C, true>),
        windup_row::<C, true>(CANCEL, cancel_anim::<C, true>, motion::no_input),
        punch_row::<C, true, true>(),
        punch_row::<C, true, false>(),
    ]
}

/// ftDk_SpecialN_Enter (8010E840) / ftDk_SpecialAirN_Enter (8010E930);
/// ftKb_SpecialNDk_800FF8EC / 800FFA10: a full charge punches at once and
/// is spent; anything less winds up.
pub fn enter<C: GiantPunchUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let stored = C::swings(f);
    let full = stored == C::attributes(f).max_swings;
    let state = action::<C>(if full { FULL } else { START }, air);
    change(f, state, MotionEntryFlags(0), 0.0, 1.0, a).expect("Giant Punch assets");
    *C::scratch(f) = GiantPunch {
        effect: i32::from(full),
        swings: if full { stored } else { 0 },
        ..GiantPunch::default()
    };
    if full {
        C::store_swings(f, 0);
    }
    f.commands.variables = [0; 4];
    if !air {
        // ftCommon_8007D7FC.
        f.land();
        f.physics.self_velocity.y = 0.0;
    }
    C::arm_callbacks(f);
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftDk_SpecialNStart_Anim (8010EA28) / ftDk_SpecialAirNStart_Anim
/// (8010EDD4): the loop at the animation's end.
fn start_anim<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        motion::seal_graphics(f, p.assets, p.rng);
        change(
            f,
            action::<C>(LOOP, AIR),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )?;
        C::arm_callbacks(f);
    }
    Ok(None)
}

/// ftDk_SpecialNLoop_Anim (8010EAA0) / ftDk_SpecialAirNLoop_Anim
/// (8010EE48): each wrap of the loop stores a swing; the last one glows
/// and ends the wind-up.
fn loop_anim<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.animation.frame == 0.0 {
        let stored = C::swings(f) + 1;
        let maximum = C::attributes(f).max_swings;
        if stored >= maximum {
            f.core
                .install_color_overlay_now(C::FULL_CHARGE_COLOR, p.assets);
            C::store_swings(f, maximum);
            C::clear_callbacks(f);
            motion::seal_graphics(f, p.assets, p.rng);
            finish(f, AIR, p.assets)?;
        } else {
            C::store_swings(f, stored);
        }
    }
    Ok(None)
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
fn finish(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    if air {
        motion::fall(f, assets)
    } else {
        motion::wait(f, assets)
    }
}

/// ftDk_SpecialNCancel_Anim (8010EB44) / ftDk_SpecialAirNCancel_Anim
/// (8010EEEC).
fn cancel_anim<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        C::clear_callbacks(f);
        motion::seal_graphics(f, p.assets, p.rng);
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
/// damage as first seen (the float truncated into x10 / x14), plus the
/// damage per swing for each swing.
fn charge_hitboxes<C: GiantPunchUser>(f: &mut Fighter) {
    let bonus = C::scratch(f).swings * C::attributes(f).damage_per_swing;
    for index in 0..2 {
        if C::scratch(f).base_damage[index] == -1 {
            // Retail reads x914[1] after testing only x914[0]. Original
            // PlDk.dat and PlKb.dat punch scripts create/clear the pair in
            // one command batch; their SkipHit transitions retain both.
            let damage = f.commands.hitboxes[index]
                .as_ref()
                .map(|hit| hit.descriptor.damage)
                .unwrap_or_else(|| {
                    unreachable!(
                        "ftdonkeyspecialn.c:165: Giant Punch paired-hitbox invariant: hitbox {index} missing with hitbox 0 active"
                    )
                });
            C::scratch(f).base_damage[index] = gekko_math::msl::fctiwz(damage);
        }
        let base = C::scratch(f).base_damage[index];
        set_hitbox_damage(f, index, base + bonus);
    }
}

/// ftDk_SpecialN_Anim (8010E930), ftDk_SpecialNFull_Anim (8010EB0C),
/// ftDk_SpecialAirN_Anim (8010EDD8) and ftDk_SpecialAirNFull_Anim
/// (8010EF7C), and Kirby's ftKb_Dk* twins. `CHARGED`: the partial punch
/// scales its hitboxes (the full punch's script carries its own damage).
/// Only the grounded rows lunge.
fn punch_anim<C: GiantPunchUser, const AIR: bool, const CHARGED: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && C::scratch(f).effect == 0 {
        C::scratch(f).effect = 1;
    }
    if C::scratch(f).effect == 1 {
        C::scratch(f).effect = 2;
        // The row decides nothing: a punch that crossed the floor's edge
        // spawns the model of where it is now.
        let [ground, air] = C::PUNCH_EFFECTS;
        let id = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
            air
        } else {
            ground
        };
        f.effects.push(EffectRequest::SyncAttached {
            id,
            bone: motion::part(FtPart::TopN),
        });
    }
    if f.commands.hitboxes[0].is_some() {
        if !AIR && C::scratch(f).lunge == 0 {
            C::scratch(f).lunge = 1;
        }
        if CHARGED {
            charge_hitboxes::<C>(f);
        }
    }
    if C::scratch(f).lunge == 1 {
        C::scratch(f).lunge = 2;
        // updateVelocity (8010E7CC..): the int swings to float, fmuls by
        // the speed, fmuls by the facing.
        let speed = C::attributes(f).punch_speed * C::scratch(f).swings as f32;
        f.physics.ground_velocity = f.physics.facing * speed;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        C::store_swings(f, 0);
        C::clear_callbacks(f);
        motion::seal_graphics(f, p.assets, p.rng);
        if !AIR {
            return motion::wait(f, p.assets).map(|()| None);
        }
        let lag = C::attributes(f).landing_lag;
        if lag == 0.0 {
            motion::fall(f, p.assets)?;
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
fn loop_input<C: GiantPunchUser, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !AIR {
        if let Some(roll) = f.core.roll_input(p.assets) {
            f.enter_escape(p.assets, roll)
                .expect("roll out of the wind-up");
            return;
        }
    }
    let pressed = f.input.pressed;
    if pressed.intersects(Buttons::B) {
        change(
            f,
            action::<C>(PUNCH, AIR),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )
        .expect("Giant Punch assets");
        C::scratch(f).swings = C::swings(f);
        C::store_swings(f, 0);
        C::arm_callbacks(f);
        // ftAnim_8006EBA4.
        f.step_animation(p.assets);
    }
    // retail 0x8010F140 / 0x8010F284 (Kirby's 0x80100484): clrrwi. r0, r0,
    // 31, the input proc's shield bit, which a digital shoulder, an analog
    // trigger past the deadzone or Z all set (the decomp's HSD_PAD_LR is
    // wrong here).
    if pressed.intersects(Buttons::SHIELD) {
        C::scratch(f).cancel = true;
    }
    if f.animation.frame == 0.0 && C::scratch(f).cancel {
        change(
            f,
            action::<C>(CANCEL, AIR),
            MotionEntryFlags(0),
            0.0,
            1.0,
            p.assets,
        )
        .expect("Giant Punch assets");
        C::arm_callbacks(f);
    }
}

/// The wind-up rows' Coll callbacks: ft_80082708 walks off the floor into
/// the aerial row; ft_80081D0C lands into the grounded one.
fn windup_collision<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    cross_floor::<C, AIR>(f, &mut p, false, WINDUP_GROUND_AIR)
}

/// The punches' Coll callbacks: ft_800827A0 stops at the floor's edge.
fn punch_collision<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    cross_floor::<C, AIR>(f, &mut p, true, PUNCH_GROUND_AIR)
}

fn cross_floor<C: GiantPunchUser, const AIR: bool>(
    f: &mut Fighter,
    p: &mut CollisionPhase<'_>,
    stop_at_edge: bool,
    flags: MotionEntryFlags,
) -> Result<()> {
    let action = f.motion_state.action;
    let assets = p.assets.expect("Giant Punch collision assets");
    if AIR {
        if motion::lands(f, p) {
            motion::air_to_ground(f, ActionId(action.0 - AIR_OFFSET), flags, assets)?;
            C::arm_callbacks(f);
        }
        return Ok(());
    }
    let supported = if stop_at_edge {
        motion::stays_on_edge(f, p)
    } else {
        motion::stays_grounded(f, p)
    };
    if !supported {
        motion::ground_to_air(f, ActionId(action.0 + AIR_OFFSET), flags, assets)?;
        C::arm_callbacks(f);
    }
    Ok(())
}
