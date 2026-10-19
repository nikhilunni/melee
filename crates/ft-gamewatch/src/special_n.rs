//! Chef, ftgamewatchspecialn.c (8014E4F0..8014EBFC): the states are
//! `ft_gamewatch_family::chef`'s, shared with Kirby's copy, over Mr. Game &
//! Watch's rows 353 and 354.
use crate::{
    articles::{self, Accessory},
    common::flags,
    init::GameWatch,
};
use ft_gamewatch_family::chef::{self, ChefAttributes, ChefUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::ItemKind;

pub use ft_gamewatch_family::chef::Chef;

/// ftGw_MS_SpecialN (353) and ftGw_MS_SpecialAirN (354).
pub const GROUND: ActionId = ActionId(353);
pub const AIR: ActionId = ActionId(354);

impl ChefUser for GameWatch {
    const GROUND: ActionId = GROUND;
    const AIR: ActionId = AIR;
    /// ftGw_SM_SpecialN = ftCo_SM_Count.
    const GROUND_ANIMATION: i32 = 295;
    const FOOD: ItemKind = ItemKind::GameWatchChef;
    /// transition_flags (ftgamewatchspecialn.c:228): the ground/air set with
    /// KeepColAnimHitStatus, SkipHit and SkipModel.
    const TRANSITION_FLAGS: u32 =
        flags::GROUND_AIR | flags::KEEP_COL_ANIM_HIT_STATUS | flags::SKIP_HIT | flags::SKIP_MODEL;
    /// x18_GAMEWATCH_CHEF_LOOPFRAME, x1C_GAMEWATCH_CHEF_MAX.
    fn attributes(f: &Fighter) -> ChefAttributes {
        f.character.get::<GameWatch>().attributes.chef
    }
    fn scratch(f: &mut Fighter) -> &mut Chef {
        &mut f.character.get_mut::<GameWatch>().chef
    }
    fn foods(f: &mut Fighter) -> (&mut i32, &mut i32) {
        let gw = f.character.get_mut::<GameWatch>();
        (&mut gw.chef_last, &mut gw.chef_previous)
    }
    /// ftGameWatch_SpecialN_SetVars: ftGw_SpecialN_CreateSausage.
    fn install_entry_accessory(f: &mut Fighter) {
        articles::install(f, Accessory::CreateSausage);
    }
    fn install_thrower(f: &mut Fighter) {
        articles::install(f, Accessory::CreateSausage);
    }
    fn uninstall(f: &mut Fighter) {
        articles::uninstall(f);
    }
}

pub const fn rows() -> [MotionRow; 2] {
    chef::rows::<GameWatch>()
}

/// ftGw_SpecialN_Enter (8014E644) / ftGw_SpecialAirN_Enter (8014E6C0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    chef::enter::<GameWatch>(f, air, a);
}

/// ftGw_SpecialN_CreateSausage (8014E4F0).
pub fn create_sausage(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    chef::create_sausage::<GameWatch>(f, assets, rng);
}

/// The foods other than the last two thrown, in order; HSD_Randi picks one,
/// which becomes the last thrown (x2240_chefVar1, then x2244_chefVar2).
pub fn draw_food(gw: &mut GameWatch, rng: &mut gekko_math::HsdRng) -> i32 {
    chef::draw_food(&mut gw.chef_last, &mut gw.chef_previous, rng)
}
