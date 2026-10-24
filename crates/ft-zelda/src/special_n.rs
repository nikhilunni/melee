//! Nayru's Love, ftzeldaspecialn.c (8013A830..8013ADB4): the states of
//! `ft_zelda_family::nayrus_love` over Zelda's rows 341/342.
use crate::init::{Accessory, Zelda};
use ft_zelda_family::nayrus_love::{self, NayrusLoveAttributes, NayrusLoveUser};
use melee_ft::{
    desc::fox_attributes::ReflectionAttributes,
    fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow},
};

pub use ft_zelda_family::nayrus_love::NayrusLove;

/// ftZd_MS_SpecialN (341) and ftZd_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);

impl NayrusLoveUser for Zelda {
    const GROUND: ActionId = GROUND;
    const AIR: ActionId = AIR;
    /// ftZd_SM_SpecialN / SpecialAirN (295, 296).
    const GROUND_ANIMATION: i32 = 295;
    /// efSync_Spawn(1268 / 1269, ...): efLib_Create_AttachChild 0x4268 /
    /// 0x4269.
    const CRYSTALS: [u16; 2] = [0x4F4, 0x4F5];
    fn attributes(f: &Fighter) -> &NayrusLoveAttributes {
        &f.character.get::<Zelda>().attributes.nayrus_love
    }
    /// ftZelda_DatAttrs +0x84.
    fn reflection(f: &Fighter) -> &ReflectionAttributes {
        &f.character.get::<Zelda>().attributes.nayrus_love_reflection
    }
    fn scratch(f: &mut Fighter) -> &mut NayrusLove {
        &mut f.character.get_mut::<Zelda>().nayrus_love
    }
    fn scratch_ref(f: &Fighter) -> &NayrusLove {
        &f.character.get::<Zelda>().nayrus_love
    }
    /// accessory4_cb = ftZd_SpecialN_8013A830 / 8013A8AC.
    fn install_accessory(f: &mut Fighter, air: bool) {
        f.character.get_mut::<Zelda>().accessory = if air {
            Accessory::NayrusLoveAirCrystal
        } else {
            Accessory::NayrusLoveCrystal
        };
        f.core.arm_accessory4();
    }
    fn uninstall_accessory(f: &mut Fighter) {
        f.character.get_mut::<Zelda>().accessory = Accessory::None;
    }
    /// ftZd_SpecialN_8013AC88 / 8013AD1C: the new row's crystal spawner.
    fn after_ground_air_change(f: &mut Fighter, air: bool) {
        Self::install_accessory(f, air);
    }
}

/// Rows 341 and 342.
pub const fn rows() -> [MotionRow; 2] {
    nayrus_love::rows::<Zelda>()
}

/// ftZd_SpecialN_Enter (8013A93C) / ftZd_SpecialAirN_Enter (8013A9B4).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    nayrus_love::enter::<Zelda>(f, air, a);
}

/// ftZd_SpecialN_8013A830 / 8013A8AC.
pub fn crystal(f: &mut Fighter, air: bool) {
    nayrus_love::crystal::<Zelda>(f, air);
}
