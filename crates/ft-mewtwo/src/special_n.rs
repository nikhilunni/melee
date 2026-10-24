//! Shadow Ball, ftmewtwospecialn.c (80146CCC..80148C64): the states are
//! `ft_mewtwo_family::shadow_ball`'s, shared with Kirby's copy, over
//! Mewtwo's rows 341..350, his right shoulder, colour 92 and the flash
//! seven units above the root; and the forward throw's ball.
use crate::init::Mewtwo;
use ft_mewtwo_family::shadow_ball::{self, ShadowBallAttributes, ShadowBallUser};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

pub use ft_mewtwo_family::shadow_ball::ShadowBall;

/// ftMt_MS_SpecialNStart (341) .. ftMt_MS_SpecialAirNEnd (350).
pub const START: ActionId = ActionId(341);

/// `fp->parts[FtPart_RShoulderN]` / `[FtPart_LHandNb]`: the parts array
/// indexed by the part enum.
const SHOULDER_JOINT: usize = 35;
const LEFT_HAND_JOINT: usize = 32;
/// ftCo_800BFFD0(fp, 92, 0): the full charge's colour animation, and
/// ftMt_Init_UnkMotionStates4's.
const FULL_CHARGE_COLOR: u8 = 92;

impl ShadowBallUser for Mewtwo {
    const START: ActionId = START;
    /// ftMt_SM_SpecialNStart.
    const START_ANIMATION: i32 = 295;
    const BALL: ItemKind = ItemKind::MewtwoShadowBall;
    const BALL_JOINT: usize = SHOULDER_JOINT;
    /// ftMt_SpecialNLoop_Anim's shadowBallPos.
    const FLASH_OFFSET: Vec3 = Vec3::new(0.0, 7.0, 0.0);
    const FULL_CHARGE_COLOR: u8 = FULL_CHARGE_COLOR;
    /// ftMewtwo_SpecialN_RemoveShadowBall2 hangs the effects' removal on
    /// x2238, which is never set.
    const DESTROYS_EFFECTS: bool = false;
    fn attributes(f: &Fighter) -> ShadowBallAttributes {
        f.character.get::<Mewtwo>().attributes.shadow_ball
    }
    fn scratch(f: &mut Fighter) -> &mut ShadowBall {
        &mut f.character.get_mut::<Mewtwo>().shadow_ball
    }
    fn charge(f: &Fighter) -> i32 {
        f.character.get::<Mewtwo>().shadow_ball_charge
    }
    fn store_charge(f: &mut Fighter, charge: i32) {
        set_charge(f, charge);
    }
    fn held(f: &Fighter) -> bool {
        f.character.get::<Mewtwo>().held_ball
    }
    fn set_held(f: &mut Fighter, held: bool) {
        f.character.get_mut::<Mewtwo>().held_ball = held;
    }
    /// ftMewtwo_SpecialN_SetCall: death2_cb and death3_cb are
    /// ftMt_Init_OnDeath2 and take_dmg_cb is ftMt_Init_OnTakeDamage until
    /// the next motion change.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Mewtwo>().damage_callbacks = true;
    }
}

pub const fn rows() -> [MotionRow; 10] {
    shadow_ball::rows::<Mewtwo>()
}

/// ftMt_SpecialN_Enter (80147320) / ftMt_SpecialAirN_Enter (801473F4).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    shadow_ball::enter::<Mewtwo>(f, air, a);
}

/// Mewtwo's stored charge (u.mt.x2234), and ftMt_Init_UnkMotionStates4's
/// glow (colour 92 whenever the secondary colour slot empties) while it is
/// the full charge x0.
pub fn set_charge(f: &mut Fighter, level: i32) {
    f.character.get_mut::<Mewtwo>().shadow_ball_charge = level;
    let full = shadow_ball::is_full::<Mewtwo>(f);
    f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
}

/// The full charge as the ball reads it (ftMt_SpecialN_GetChargeLevel: the
/// float x0 stored to an s32).
pub fn full_charge(f: &Fighter) -> i32 {
    shadow_ball::full_charge::<Mewtwo>(f)
}

/// ftMewtwo_SpecialN_RemoveShadowBall2: a held ball is destroyed
/// (it_802C573C).
pub fn remove_ball(f: &mut Fighter) {
    shadow_ball::remove_ball::<Mewtwo>(f);
}

/// ftMt_SpecialN_Shoot (80148B78), from ftCo_ThrowF_Anim: on the forward
/// throw's cmd_vars[3] a ball leaves the right shoulder joint
/// (it_802C519C) along the line from the left hand to it (atan2f of
/// separate fsubs), both taken on the stage plane.
pub fn throw_shot(f: &mut Fighter, _assets: &FighterAssets) {
    if f.commands.variables[3] != 1 {
        return;
    }
    f.commands.variables[3] = 0;
    let (mut shoulder, mut hand) = {
        let c = &mut f.core;
        let mut at = |joint| {
            melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                joint,
                Vec3::ZERO,
            )
        };
        (at(SHOULDER_JOINT), at(LEFT_HAND_JOINT))
    };
    hand.z = 0.0;
    shoulder.z = 0.0;
    let angle = melee_lb::trigf::atan2f(shoulder.y - hand.y, shoulder.x - hand.x);
    let full = full_charge(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(
        ItemKind::MewtwoShadowBall,
        c.player.id,
        shoulder,
        c.physics.facing,
    );
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_variant = it_mewtwo::shadow_ball::THROWN_VARIANT;
    spawn.spawn_argument = angle.to_bits() as i32;
    spawn.auxiliary_damage = full as i16;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}
