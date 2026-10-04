//! Hand Slap, ftdonkeyspeciallw.c (8010DCD8..8010E0C8) and its quake
//! (ftDk_Init_8010DB3C).
//!
//! A grounded move only: the start, then a slap loop that repeats while B
//! was pressed during the previous slap, then the end. Each slap is a new
//! attack instance. On the script's cue the loop's four hitboxes are laid
//! along the floor either side of Donkey Kong; leaving the floor at any
//! point falls.
use crate::{
    common::{self, change, row},
    init::{Accessory, DonkeyKong},
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

/// ftDk_MS_SpecialLwStart (383), SpecialLwLoop (384), SpecialLwEnd0 (385).
/// ftDk_MS_SpecialLwEnd1 (386, an aerial ending) has no entry in retail's
/// code and stays unported.
pub const START: ActionId = ActionId(383);
pub const LOOP: ActionId = ActionId(384);
pub const END: ActionId = ActionId(385);

/// efAsync_Spawn(gobj, &fp->x60C, 1, 1228, TopN): model 0x1F46 at the
/// fighter's root, turned with the fighter.
const EFFECT: u16 = 0x4CC;
/// The quake's four hitboxes.
const QUAKE_HITBOXES: usize = 4;
/// `1.5f` (retail @295): the points are centred on Donkey Kong.
const CENTRE: f32 = 1.5;
/// `2.0f` (retail @296): the points sit this far above the floor.
const LIFT: f32 = 2.0;

pub const fn rows() -> [MotionRow; 3] {
    [
        row(START, 0x14D, start_anim, common::no_input, common::ground_friction, collision),
        row(LOOP, 0x14E, loop_anim, loop_input, common::ground_friction, collision),
        row(END, 0x14F, end_anim, common::no_input, common::ground_friction, collision),
    ]
}

/// ftDk_SpecialLw_Enter (8010DCD8).
pub fn enter(f: &mut Fighter, a: &FighterAssets) {
    f.character.get_mut::<DonkeyKong>().slap_again = false;
    change(f, START, MotionEntryFlags(0), 0.0, 1.0, a).expect("Hand Slap assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
}

/// ftDk_SpecialLwStart_Anim (8010DD3C).
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        slap(f, p.assets)?;
    }
    Ok(None)
}

/// doAnim (8010DE88's caller): x21EC = `callback` runs inside this motion
/// change, before the loop's frame-zero script: ft_800892A0's new attack
/// instance (ft_80089824 only touches statistics). Then the slap's model
/// unless effects are already kept (x2219_b0), the efLib hitlag pair, and
/// the quake as accessory4.
fn slap(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.core.combat.stale.new_instance();
    f.commands.clear_throw_flags();
    change(f, LOOP, MotionEntryFlags(0), 0.0, 1.0, assets)?;
    if !f.effect_state.destroy_on_state_change {
        f.core.push_effect_after_issued_graphics(EffectRequest::BoneModel {
            id: EFFECT,
            bone: common::part(FtPart::TopN),
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    f.character.get_mut::<DonkeyKong>().accessory = Accessory::Quake;
    f.core.arm_accessory4();
    Ok(())
}

/// ftDk_SpecialLwLoop_Anim (8010DDDC): at the slap's end, another one if B
/// was pressed during it, else the ending.
fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        if std::mem::take(&mut f.character.get_mut::<DonkeyKong>().slap_again) {
            slap(f, p.assets)?;
        } else {
            change(f, END, MotionEntryFlags(0), 0.0, 1.0, p.assets)?;
        }
    }
    Ok(None)
}

/// ftDk_SpecialLwLoop_IASA (8010DE44).
fn loop_input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.input.pressed.intersects(Buttons::B) {
        f.character.get_mut::<DonkeyKong>().slap_again = true;
    }
}

/// ftDk_SpecialLwEnd0_Anim (8010DF94).
fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftDk_SpecialLwStart_Coll / Loop_Coll / End0_Coll: off the floor
/// (ft_80082708), ftCo_Fall_Enter.
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::stays_grounded(f, &mut p) {
        common::fall(f, p.assets.expect("Hand Slap collision assets"))?;
    }
    Ok(())
}

/// ftDk_Init_8010DB3C (8010DB3C): on the script's throw_flags_b3, standing
/// on a floor, the four hitboxes become world points along the floor:
/// spaced x68 apart about a centre x6C ahead of Donkey Kong (mpLib_80056C54
/// from his position, wall runs up to x70), or his own position where the
/// floor ends first, each 2 units up (ftColl_8007B8A8). The accessory stays
/// installed for the whole slap.
pub fn quake(f: &mut Fighter, map: &mut melee_mp::CollMap) {
    if !f.commands.take_throw_flag_b3() {
        return;
    }
    let data = &f.core.collision.data;
    if data.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK == 0 {
        return;
    }
    let (spacing, forward, reach) = {
        let a = &f.character.get::<DonkeyKong>().attributes.hand_slap;
        (a.spacing, a.forward_offset, a.reach)
    };
    let floor = f.core.collision.data.floor.index;
    let origin = f.physics.position;
    // retail 0x8010DBD8: fmuls (the centre), 0x8010DBEC: fmuls (the lead).
    let centre = CENTRE * spacing;
    let lead = forward * f.physics.facing;
    for index in 0..QUAKE_HITBOXES {
        // retail 0x8010DC00: fmsubs, 0x8010DC0C: fadds.
        let distance = gekko_math::fma::fmsubs(spacing, index as f32, centre) + lead;
        let mut point = match map.walk_along_floor(floor, &origin, distance, reach) {
            Some(walk) if walk.reached => walk.pos,
            _ => origin,
        };
        // retail 0x8010DC40: fadds.
        point.y += LIFT;
        if let Some(hit) = f.core.commands.hitboxes[index].as_mut() {
            hit.world = true;
            hit.descriptor.offset = point;
        } else {
            unimplemented!(
                "ftcoll.c:3385: ftColl_8007B8A8 on Hand Slap hitbox {index} the script has not made"
            );
        }
    }
}
