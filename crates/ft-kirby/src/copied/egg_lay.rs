//! The copied Egg Lay, ftkirbyspecialyoshi.c (80109260..8010A8BC): Yoshi's
//! states (`ft_yoshi_family::egg_lay`) over Kirby's rows 413..422, with his
//! attributes (+3AC..+3E0, Yoshi's layout) and the victim's Kirby rows
//! (ftCo_MS_CaptureKirbyYoshi, ftCo_MS_KirbyYoshiEgg). Every row change
//! installs take_dmg_cb / death2_cb (ftKb_Init_800EE7B8 / 800EE74C); the
//! hat's animations (hat_dynamics[1..4]) only draw.
use crate::init::Kirby;
use ft_yoshi_family::egg_lay::{self, EggLayAttributes, EggLayUser};
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    capture_yoshi::EggLayer,
    Fighter, MotionRow,
};
use melee_types::FighterKind;

/// Kirby with Yoshi's ability.
pub struct KirbyYoshi;

impl EggLayUser for KirbyYoshi {
    /// ftKb_MS_YsSpecialN1 (413) .. ftKb_MS_YsSpecialAirN2_1 (422).
    const FIRST: u16 = 413;
    /// ftKb_SM_YsSpecialN1, YsSpecialNCapture1, YsSpecialNCapture2,
    /// YsSpecialAirNCapture2, YsSpecialAirCapture1, YsSpecialAirN2.
    const ANIMATIONS: [i32; 6] = [352, 353, 354, 355, 356, 357];
    const LAYER: EggLayer = EggLayer::Kirby;
    /// ftKb_DatAttrs +3AC..+3E0.
    fn attributes(f: &Fighter) -> EggLayAttributes {
        f.character.get::<Kirby>().attributes.yoshi_egg_lay
    }
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
}

/// ftKb_SpecialNYs_8010941C / 801094FC (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    egg_lay::enter::<KirbyYoshi>(f, airborne, assets);
}

/// Rows 413..422: ftKb_YsSpecialN* (the item rows fail closed).
pub const fn rows() -> [MotionRow; 10] {
    egg_lay::rows::<KirbyYoshi>()
}

/// Whether the tongue of Kirby's copied Egg Lay made the catch (the grab
/// callbacks fn_80109680 / fn_801097B8 that its rows install).
pub fn tongue_out(f: &Fighter) -> bool {
    f.character.get::<Kirby>().copied == FighterKind::Yoshi && egg_lay::in_rows::<KirbyYoshi>(f)
}

/// fn_80109680 (80109680) / fn_801097B8 (801097B8), then fn_8010A930.
pub fn grab(
    kirby: &mut Fighter,
    victim: &mut Fighter,
    kirby_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    egg_lay::grab::<KirbyYoshi>(kirby, victim, kirby_assets, victim_assets)
}
