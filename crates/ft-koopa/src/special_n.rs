//! Fire Breath, ftkoopaspecialn.c (80134ACC..80135780): the states of
//! `ft_koopa_family::fire_breath` over Bowser's rows 341..346.
use crate::init::Koopa;
use ft_koopa_family::fire_breath::{
    self, Breath, BreathRows, FireBreathAttributes, FireBreathUser, Fuels,
};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_types::ItemKind;

/// ftKp_MS_SpecialNStart (341) .. ftKp_MS_SpecialAirNEnd (346): the rows
/// in which the breath's fuels drain instead of refilling.
pub const FIRST_ROW: u16 = 341;
pub const LAST_ROW: u16 = 346;
const ROWS: BreathRows = BreathRows::from(FIRST_ROW);

impl FireBreathUser for Koopa {
    fn rows(_: &Fighter) -> BreathRows {
        ROWS
    }
    const AIR_START_LANDS_IN_LOOP: bool = false;
    /// ftKp_SpecialN_Enter_inline: mv.kp.specials +8 = 0.
    const FIRST_LAST_EFFECT: i32 = 0;
    const ITEM: ItemKind = ItemKind::KoopaFlame;
    /// lb_8000B1CC(fp->parts[48].joint, NULL, &v): the mouth.
    const MOUTH_PART: usize = 48;
    const MOUTH_OFFSET: Vec3 = Vec3::ZERO;
    /// ftKoopaAttributes +0x04..+0x28.
    fn attributes(f: &Fighter) -> &FireBreathAttributes {
        &f.character.get::<Koopa>().attributes.fire_breath
    }
    fn breath(f: &mut Fighter) -> &mut Breath {
        &mut f.character.get_mut::<Koopa>().breath
    }
    /// u.kp.x222C / x2230.
    fn fuels(f: &mut Fighter) -> &mut Fuels {
        &mut f.character.get_mut::<Koopa>().fuels
    }
}

/// ftKp_Init_MotionStateTable[0..6]: submotions 295..300.
pub const fn rows() -> [MotionRow; 6] {
    fire_breath::rows::<Koopa>(ROWS, 295)
}

/// ftKp_SpecialN_Enter (80134E58) / ftKp_SpecialAirN_Enter (80134ED0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    fire_breath::enter::<Koopa>(f, air, a);
}
