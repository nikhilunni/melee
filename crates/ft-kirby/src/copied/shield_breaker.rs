//! The copied Shield Breaker and Flare Blade, ftkirbyspecialmars.c:
//! Marth's and Roy's states (`ft_mars_family::special_n`) over Kirby's rows
//! 492..499 and 530..537, with Kirby's attributes and effects and without
//! the hips' gusts.
//!
//! The entries also hold the copy's sword (ftCommon_SetAccessory with the
//! copy archive's hat_dynamics[0] on the right thumb, kept across the
//! special's motion changes by Ft_MF_KeepAccessory): a model only, drawn
//! and animated by nothing the simulation reads.
use crate::init::Kirby;
use ft_mars_family::{
    attributes::ShieldBreakerAttributes,
    special_n::{self, ShieldBreaker, ShieldBreakerRows, SpecialN},
};
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_types::FighterKind;

/// Kirby with Marth's ability.
pub struct KirbyMarth;
/// Kirby with Roy's ability.
pub struct KirbyRoy;

impl ShieldBreaker for KirbyMarth {
    /// ftKb_MS_MsSpecialNStart (492), ftKb_SM_MsSpecialNStart (427).
    const ROWS: ShieldBreakerRows = ShieldBreakerRows {
        first_action: 492,
        first_animation: 427,
    };
    /// fn_8010B1F4's FTKIND_MARS arm: efAlt 0x4B4 / 0x4B5 (models
    /// 0x4E20 / 0x4E21, EfKbMs.dat).
    const RELEASE_EFFECTS: [u16; 2] = [0x4B4, 0x4B5];
    /// ftKb_MsSpecialNStart_Anim: `hat.kind == FTKIND_MARS ? 99 : 100`.
    const CHARGE_COLOR_ANIMATION: u8 = 99;
    const GUSTS: bool = false;
    /// ftKb_DatAttrs +35C (`ms`).
    fn attributes(f: &Fighter) -> ShieldBreakerAttributes {
        f.character.get::<Kirby>().attributes.marth_shield_breaker
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialN {
        &mut f.character.get_mut::<Kirby>().shield_breaker
    }
    fn scratch_ref(f: &Fighter) -> &SpecialN {
        &f.character.get::<Kirby>().shield_breaker
    }
}

impl ShieldBreaker for KirbyRoy {
    /// ftKb_MS_FeSpecialNStart (530), ftKb_SM_FeSpecialNStart (465).
    const ROWS: ShieldBreakerRows = ShieldBreakerRows {
        first_action: 530,
        first_animation: 465,
    };
    /// fn_8010B1F4's other arm: efAlt 0x4B8 / 0x4B9 (models 0xBB80 /
    /// 0xBB81, EfKbFe.dat).
    const RELEASE_EFFECTS: [u16; 2] = [0x4B8, 0x4B9];
    const CHARGE_COLOR_ANIMATION: u8 = 100;
    const GUSTS: bool = false;
    /// ftKb_DatAttrs +370 (`fe`).
    fn attributes(f: &Fighter) -> ShieldBreakerAttributes {
        f.character.get::<Kirby>().attributes.roy_flare_blade
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialN {
        &mut f.character.get_mut::<Kirby>().shield_breaker
    }
    fn scratch_ref(f: &Fighter) -> &SpecialN {
        &f.character.get::<Kirby>().shield_breaker
    }
}

/// ftKb_SpecialNMs_8010B2FC / 8010B4A0 (the grounded and aerial entries,
/// for either kind).
pub fn enter<C: ShieldBreaker>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    special_n::enter::<C>(f, airborne, assets);
}

/// Rows 492..499 (ftKb_MS_MsSpecialN*) and 530..537 (ftKb_MS_FeSpecialN*):
/// the same callbacks, ftKb_MsSpecialN*.
pub const fn rows() -> [MotionRow; 16] {
    let marth = special_n::rows::<KirbyMarth>();
    let roy = special_n::rows::<KirbyRoy>();
    let mut rows = [marth[0]; 16];
    let mut i = 0;
    while i < 8 {
        rows[i] = marth[i];
        rows[8 + i] = roy[i];
        i += 1;
    }
    rows
}

/// accessory4_cb = fn_8010B1F4, from the end rows.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    match f.character.get::<Kirby>().copied {
        FighterKind::Mars => special_n::accessory::<KirbyMarth>(f, assets),
        FighterKind::Emblem => special_n::accessory::<KirbyRoy>(f, assets),
        _ => {}
    }
}
