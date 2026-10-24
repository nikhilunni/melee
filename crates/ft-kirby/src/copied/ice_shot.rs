//! The copied Ice Shot, ftkirbyspecialiceclimber.c (80108CE8..80109238):
//! the Ice Climbers' states (`ft_iceclimbers_family::ice_shot`) over
//! Kirby's rows 453/454, with his attributes (+39C..+3A8), the block he
//! holds (u.kb.xC0), the lift (xC4) and drop (xC8), and
//! It_Kind_Kirby_IceClimberIce from PlKbCpPp.dat. A swallowed Nana gives
//! Popo's ability (her entry in ftKb_Init_803C9DD0 is NULL).
use crate::init::{Accessory, Kirby};
use ft_iceclimbers_family::ice_shot::{self, IceShotAttributes, IceShotUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, ItemKind};

/// Kirby with the Ice Climbers' ability.
pub struct KirbyPopo;

/// ft_800881D8(fp, 0x2233B, 127, 64): Kirby's voice at the launch
/// (fn_801090D4).
const LAUNCH_VOICE: u32 = 0x2233B;

fn kirby(f: &mut Fighter) -> &mut Kirby {
    f.character.get_mut::<Kirby>()
}

impl IceShotUser for KirbyPopo {
    /// ftKb_MS_PpSpecialN / PpSpecialAirN (453, 454).
    const GROUND: ActionId = ActionId(453);
    const AIR: ActionId = ActionId(454);
    /// ftKb_SM_PpSpecialN (388).
    const GROUND_ANIMATION: i32 = 388;
    const ITEM: ItemKind = ItemKind::KirbyIceClimberIce;
    /// ftKb_DatAttrs +39C..+3A8 (`specialn_pp_*`).
    fn attributes(f: &Fighter) -> &IceShotAttributes {
        &f.character.get::<Kirby>().attributes.ice_shot
    }
    fn held(f: &mut Fighter) -> &mut bool {
        &mut kirby(f).ice_held
    }
    fn lift_used(f: &mut Fighter) -> &mut bool {
        &mut kirby(f).ice_shot_lift_used
    }
    fn drop(f: &mut Fighter) -> &mut f32 {
        &mut kirby(f).ice_drop
    }
    /// accessory4_cb = fn_801090D4.
    fn install_accessory(f: &mut Fighter) {
        kirby(f).accessory = Accessory::IceShot;
        f.core.arm_accessory4();
    }
    /// fn_801090D4: with the block made, death2_cb = ftKb_Init_800EE74C and
    /// take_dmg_cb = ftKb_Init_800EE7B8.
    fn hold(f: &mut Fighter) {
        let state = kirby(f);
        state.ice_held = true;
        state.damage_callbacks = true;
    }
    /// u.kb.xC0 = NULL; the callbacks stay.
    fn let_go(f: &mut Fighter) {
        kirby(f).ice_held = false;
    }
    fn launch_voice(_: &Fighter) -> u32 {
        LAUNCH_VOICE
    }
}

/// ftKb_SpecialNIc_80108D64 / 80108E14 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    ice_shot::enter::<KirbyPopo>(f, airborne, assets);
}

/// fn_801090D4 (801090D4), from accessory4.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    ice_shot::accessory::<KirbyPopo>(f, assets);
}

/// ftKb_SpecialNIc_80108D04 (80108D04): the put-away arm of 800F19AC /
/// 800F1A8C, the held block breaks.
pub fn put_away(f: &mut Fighter, kind: FighterKind) {
    if kind == FighterKind::Popo {
        ice_shot::break_held_ice::<KirbyPopo>(f);
    }
}

/// ftKb_SpecialNIc_80108CE8 (80108CE8), from it_2725_Logic90_Destroyed:
/// the block Kirby held is gone.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyIceClimberIce {
        kirby(f).ice_held = false;
    }
}

/// Rows 453, 454 (ftKb_PpSpecial*N_*).
pub const fn rows() -> [MotionRow; 2] {
    ice_shot::rows::<KirbyPopo>()
}
