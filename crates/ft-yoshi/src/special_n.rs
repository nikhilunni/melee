//! Egg Lay, ftyoshispecialn.c (8012CC94..8012DF18): the family's states
//! (`ft_yoshi_family::egg_lay`) over Yoshi's rows 346..355. The captured
//! fighter's CaptureYoshi and YoshiEgg states are common code
//! (ftCo_CaptureYoshi.c, ftCo_YoshiEgg.c) in melee-ft.
use crate::init::Yoshi;
use ft_yoshi_family::egg_lay::{self, EggLayAttributes, EggLayUser};
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    capture_yoshi::EggLayer,
    Fighter, MotionRow,
};

/// First Yoshi-owned row (ftYs_MS_SpecialN1 - ftCo_MS_Count).
pub const FIRST_ROW: usize = 5;

impl EggLayUser for Yoshi {
    /// ftYs_MS_SpecialN1.
    const FIRST: u16 = 346;
    /// ftYs_Init_MotionStateTable's anim_id column.
    const ANIMATIONS: [i32; 6] = [295, 296, 297, 298, 299, 300];
    const LAYER: EggLayer = EggLayer::Yoshi;
    fn attributes(f: &Fighter) -> EggLayAttributes {
        f.character.get::<Yoshi>().attributes.egg_lay
    }
    /// Yoshi's rows install no damage callbacks.
    fn arm_callbacks(_: &mut Fighter) {}
}

/// Rows 346..355.
pub const fn rows() -> [MotionRow; 10] {
    egg_lay::rows::<Yoshi>()
}

/// ftYs_SpecialN_Enter (8012CDE8) / ftYs_SpecialAirN_Enter (8012CE64).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    egg_lay::enter::<Yoshi>(f, airborne, assets);
}

/// fn_8012CF7C / fn_8012D0A0, the tongue's grab_cb on a fighter.
pub fn grab(
    yoshi: &mut Fighter,
    victim: &mut Fighter,
    yoshi_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    egg_lay::grab::<Yoshi>(yoshi, victim, yoshi_assets, victim_assets)
}
