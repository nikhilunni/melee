//! Fireball, ftluigispecialn.c (8014267C..80142A24): the states of
//! `ft_luigi_family::fireball` over Luigi's rows 341 and 342.
use crate::init::{Accessory, Luigi};
use ft_luigi_family::fireball::{self, GreenFireball};
use melee_ft::fighter::{assets::FighterAssets, state::MotionRow, ActionId, Fighter};
use melee_types::{FtPart, ItemKind};

/// ftLg_MS_SpecialN (341) and ftLg_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);

impl GreenFireball for Luigi {
    const GROUND: ActionId = GROUND;
    const AIR: ActionId = AIR;
    /// ftLg_SM_SpecialN = ftCo_SM_Count.
    const GROUND_ANIMATION: i32 = 295;
    const ITEM: ItemKind = ItemKind::LuigiFire;
    /// efSync_Spawn(1287, gobj, hand, &facing_dir): the hand flash (model
    /// 0x4650 turned to the facing, then generator 0x4650).
    const FLASH: u16 = 0x507;
    /// lb_8000B1CC(parts[L1stNb].joint).
    const HAND: FtPart = FtPart::L1stNb;
    /// accessory4_cb = ftLg_SpecialN_FireSpawn.
    fn install_accessory(f: &mut Fighter) {
        f.character.get_mut::<Luigi>().accessory = Accessory::Fireball;
        f.core.arm_accessory4();
    }
}

pub const fn rows() -> [MotionRow; 2] {
    fireball::rows::<Luigi>()
}

/// ftLg_SpecialN_Enter (8014267C) / ftLg_SpecialAirN_Enter (801426EC).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    fireball::enter::<Luigi>(f, air, a);
}

/// ftLg_SpecialN_FireSpawn (8014295C).
pub fn spawn_fireball(f: &mut Fighter, assets: &FighterAssets) {
    fireball::spawn::<Luigi>(f, assets);
}
