//! The copied Fire Breath, ftkirbyspecialkoopa.c (800FA588..800FB3C8):
//! Bowser's states (`ft_koopa_family::fire_breath`) over Kirby's rows
//! 443..448 (Bowser's hat), with his attributes
//! (+118..+13C), fuels (u.kb.x84 / x88) and It_Kind_Kirby_KoopaFlame from
//! PlKbCpKp.dat. No damage callbacks and no put-away arm (800F19AC /
//! 800F1A8C have no case for Bowser).
use crate::init::Kirby;
use ft_koopa_family::fire_breath::{
    self, Breath, BreathRows, FireBreathAttributes, FireBreathUser, Fuels,
};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_types::{FighterKind, ItemKind};

/// Kirby with Bowser's ability.
pub struct KirbyKoopa;

/// ftKb_MS_KpSpecialNStart (443), ftKb_SM_KpSpecialNStart (378).
const KOOPA_ROWS: BreathRows = BreathRows::from(443);
const KOOPA_ANIMATION: i32 = 378;
impl FireBreathUser for KirbyKoopa {
    fn rows(_: &Fighter) -> BreathRows {
        KOOPA_ROWS
    }
    /// ftKb_KpSpecialAirNStart_Coll: `li r4, 0x1bc` (444) at 800FB218.
    const AIR_START_LANDS_IN_LOOP: bool = true;
    /// ftKb_SpecialNKp_800FA8B4: mv.kb.specialn_kp.x0[2] = -1.
    const FIRST_LAST_EFFECT: i32 = -1;
    const ITEM: ItemKind = ItemKind::KirbyKoopaFlame;
    /// ftKb_SpecialNKp_800FA588: lb_8000B1CC(fp->parts[12].joint,
    /// &(0, 0.5, 3), &pos) (@240..@242).
    const MOUTH_PART: usize = 12;
    const MOUTH_OFFSET: Vec3 = Vec3::new(0.0, 0.5, 3.0);
    /// ftKb_DatAttrs +118..+13C (`specialn_kp_*`).
    fn attributes(f: &Fighter) -> &FireBreathAttributes {
        &f.character.get::<Kirby>().attributes.fire_breath
    }
    fn breath(f: &mut Fighter) -> &mut Breath {
        &mut f.character.get_mut::<Kirby>().fire_breath
    }
    /// u.kb.x84 / x88.
    fn fuels(f: &mut Fighter) -> &mut Fuels {
        &mut f.character.get_mut::<Kirby>().breath_fuels
    }
}

/// ftKb_SpecialNKp_800FA8B4 / 800FA958 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    fire_breath::enter::<KirbyKoopa>(f, airborne, assets);
}

/// ftKb_Init_UnkMotionStates3 -> ftKb_SpecialNKp_800FA7D4 (800FA7D4): with
/// Bowser's or Giga Bowser's hat, outside rows 443..448 (Giga Bowser's
/// rows are not excluded), the fuels refill.
pub fn refill(f: &mut Fighter) {
    if !matches!(
        f.character.get::<Kirby>().copied,
        FighterKind::Koopa | FighterKind::GKoops
    ) {
        return;
    }
    let action = f.motion_state.action.0;
    if (443..449).contains(&action) {
        return;
    }
    fire_breath::refill::<KirbyKoopa>(f);
}

/// The witnessed Bowser copy rows 443..448. Giga Bowser's rows
/// 538..543 stay unimplemented in the motion table until retail-witnessed.
pub const fn rows() -> [MotionRow; 6] {
    fire_breath::rows::<KirbyKoopa>(KOOPA_ROWS, KOOPA_ANIMATION)
}
