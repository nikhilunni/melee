//! Hammer, ftkirbyspecials.c (800F539C..800F57FC).
//!
//! One row on the ground (383) and one in the air (384), with no
//! counterpart change between them: off the floor's end the grounded swing
//! ends in Fall, and an aerial swing that reaches the floor ends in
//! LandingFallSpecial with the attribute's lag. The first aerial Hammer of
//! a flight lifts Kirby (`u.kb.x64`, cleared by any landing).
//!
//! The hammer is an article (`It_Kind_Kirby_Hammer`, `it-kirby`): the
//! script's cmd_vars[0] takes it out into the right hand (1) with the
//! swing's trail (efAlt 0x496 on the ground, 0x497 in the air) and puts it
//! away (2). While it is out a hit or a death removes it
//! (ftKb_Init_800EE7B8 / ftKb_Init_800EE74C).
use crate::{
    common,
    init::{Accessory, Kirby},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks::physics, AnimationPhase, CollisionPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftKb_MS_SpecialS (383) and ftKb_MS_SpecialAirS (384).
pub const GROUND: ActionId = ActionId(383);
pub const AIR: ActionId = ActionId(384);

/// `fp->parts[FtPart_R3rdNa]`: the hand the hammer is held in, as an index
/// into the parts array.
const HAMMER_PART: u8 = 44;
/// `fp->parts[FtPart_HipN]`: the joint the swing's trail follows.
const TRAIL_JOINT: usize = 4;
/// efAlt 0x496 / 0x497: efLib_Create_Attach(0x138C / 0x138D), the aerial
/// one turned to the facing on every update.
const GROUND_TRAIL: u16 = 0x496;
const AIR_TRAIL: u16 = 0x497;
/// What the script writes to cmd_vars[0].
const TAKE_OUT: u32 = 1;
const PUT_AWAY: u32 = 2;

pub const fn rows() -> [MotionRow; 2] {
    [
        // ftKb_SpecialS_Phys (800F56D8): ft_80084F3C.
        common::row(
            GROUND,
            322,
            ground_anim,
            common::no_input,
            physics::guard_on,
            ground_collision,
        ),
        // ftKb_SpecialAirS_Phys (800F56F8): ft_80084EEC.
        common::row(
            AIR,
            323,
            air_anim,
            common::no_input,
            physics::air_friction,
            air_collision,
        ),
    ]
}

/// ftKb_SpecialS_Enter (800F5550) / ftKb_SpecialAirS_Enter (800F55C0): in
/// the air the first Hammer since the last landing sets the upward speed.
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.variables[0] = 0;
    if air {
        let kirby = f.character.get_mut::<Kirby>();
        let lift = kirby.attributes.hammer.aerial_lift;
        if !std::mem::replace(&mut kirby.hammer_lift_used, true) {
            f.physics.self_velocity.y = lift;
        }
    }
    f.change_motion_state(if air { AIR } else { GROUND }, assets)
        .expect("Hammer assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    // accessory4_cb = fn_800F53AC.
    f.core.arm_accessory4();
    f.character.get_mut::<Kirby>().accessory = Accessory::Hammer;
}

/// fn_800F53AC (800F53AC), accessory4 for the whole move. On the script's
/// 1 the hammer is created at the hand's position (it_802ADC54: its ray
/// starts at Kirby's ECB centre, it_8026BB68) and taken into the hand;
/// with it the damage and death callbacks, the trail, and the effects'
/// hitlag callbacks. On the script's 2 it goes.
pub fn hammer_accessory(f: &mut Fighter) {
    if f.commands.variables[0] == TAKE_OUT {
        let hand = common::joint_position(f, usize::from(HAMMER_PART));
        let origin = common::ecb_centre(f);
        let airborne = f.physics.ground_or_air == GroundOrAir::Air;
        let c = &mut f.core;
        let mut spawn = SpawnItem::ray(ItemKind::KirbyHammer, c.player.id, hand, c.physics.facing);
        spawn.previous_position = hand;
        spawn.position = origin;
        // it_802ADC54's `vars`: the owner's ground_or_air picks the
        // article's own effect.
        spawn.spawn_argument = i32::from(airborne);
        c.item_requests.push(ItemRequest::SpawnInHand {
            spawn,
            part: HAMMER_PART,
            hold: false,
            catch_item: false,
            scale_by_owner: false,
        });
        f.commands.variables[0] = 0;
        let kirby = f.character.get_mut::<Kirby>();
        kirby.hammer_out = true;
        kirby.damage_callbacks = true;
        f.effects.push(EffectRequest::SyncAttached {
            id: if airborne { AIR_TRAIL } else { GROUND_TRAIL },
            bone: TRAIL_JOINT,
        });
        f.core.effect_state.destroy_on_state_change = true;
        f.core.effect_state.hitlag_callbacks = true;
    }
    if f.commands.variables[0] == PUT_AWAY {
        remove_hammer(f);
        f.commands.variables[0] = 0;
    }
}

/// ftKb_SpecialAirLw_800F5524 (800F5524): it_802ADC34 destroys the hammer
/// (Item_8026A8EC) and Kirby lets go of it.
pub fn remove_hammer(f: &mut Fighter) {
    if !std::mem::take(&mut f.character.get_mut::<Kirby>().hammer_out) {
        return;
    }
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::KirbyHammer,
        control: ItemControl::Remove,
    });
}

/// ftKb_SpecialAirLw_800F539C (800F539C), from the article's Destroyed
/// callback.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyHammer {
        f.character.get_mut::<Kirby>().hammer_out = false;
    }
}

/// ftKb_SpecialS_Anim (800F5660): Wait at the end (ft_8008A2BC).
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialAirS_Anim (800F569C): Fall at the end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialS_Coll (800F5718): off the floor (ft_80082708) the hammer
/// goes and Kirby falls.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    remove_hammer(f);
    common::enter_fall(f, p.assets.expect("Hammer collision assets"))
}

/// ftKb_SpecialAirS_Coll (800F5778): on a floor (ft_80081D0C) the hammer
/// goes, the lift is ready again and Kirby lands in LandingFallSpecial
/// with the attribute's lag.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    remove_hammer(f);
    let kirby = f.character.get_mut::<Kirby>();
    kirby.hammer_lift_used = false;
    let lag = kirby.attributes.hammer.landing_lag;
    f.enter_special_landing(p.assets.expect("Hammer collision assets"), false, lag)
}
