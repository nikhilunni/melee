//! The copied Thunder Jolt, ftkirbyspecialpikachu.c: Pikachu's states
//! (`ft_pikachu_family::special_n`) over Kirby's rows 429..430
//! (Pikachu's ability) and 526..527 (Pichu's), with his attributes. No
//! scratch and no put-away arm (ftKb_SpecialN_800F19AC / 800F1A8C have no
//! case for either kind): the ball and its crawler are on their own.
use crate::init::Kirby;
use ft_pikachu_family::{
    attributes::ThunderJoltAttributes,
    special_n::{self, ThunderJolt},
};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::FighterKind;

/// Kirby with Pikachu's or Pichu's ability: one marker, since
/// ftkirbyspecialpikachu.c's callbacks switch on the hat's kind themselves.
pub struct KirbyJolt;

/// ftKb_MS_PkSpecialN / PkSpecialAirN (429, 430), ftKb_SM_PkSpecialN (364).
const PIKACHU_ROWS: [ActionId; 2] = [ActionId(429), ActionId(430)];
const PIKACHU_ANIMATION: i32 = 364;
/// ftKb_MS_PcSpecialN / PcSpecialAirN (526, 527), ftKb_SM_PcSpecialN (461).
const PICHU_ROWS: [ActionId; 2] = [ActionId(526), ActionId(527)];
const PICHU_ANIMATION: i32 = 461;

/// ft_PlaySFX ids of ftKb_PkSpecialN_Anim / ftKb_PkSpecialAirN_Anim.
const PIKACHU_SOUND: u32 = 240076;
const PICHU_SOUND: u32 = 230067;

fn is_pichu(f: &Fighter) -> bool {
    f.character.get::<Kirby>().copied == FighterKind::Pichu
}

impl ThunderJolt for KirbyJolt {
    /// doEnter and both Coll callbacks: Pichu's rows under Pichu's hat,
    /// Pikachu's for any other.
    fn rows(f: &Fighter) -> [ActionId; 2] {
        if is_pichu(f) {
            PICHU_ROWS
        } else {
            PIKACHU_ROWS
        }
    }
    /// ftKb_DatAttrs +1C0 (`specialn_pk_*`) or +1DC (`specialn_pc_*`).
    fn attributes(f: &Fighter) -> &ThunderJoltAttributes {
        let attributes = &f.character.get::<Kirby>().attributes;
        if is_pichu(f) {
            &attributes.pichu_thunder_jolt
        } else {
            &attributes.pikachu_thunder_jolt
        }
    }
    fn sound(f: &Fighter) -> u32 {
        if is_pichu(f) {
            PICHU_SOUND
        } else {
            PIKACHU_SOUND
        }
    }
}

/// ftKb_SpecialNPk_800F9FD4 / 800FA064 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    special_n::enter::<KirbyJolt>(f, airborne, assets);
}

/// Rows 429..430 and 526..527: ftKb_PkSpecialN_* / ftKb_PkSpecialAirN_*
/// for both kinds.
pub const fn rows() -> [MotionRow; 4] {
    use special_n::row;
    [
        row::<KirbyJolt>(PIKACHU_ROWS[0], PIKACHU_ANIMATION, false),
        row::<KirbyJolt>(PIKACHU_ROWS[1], PIKACHU_ANIMATION + 1, true),
        row::<KirbyJolt>(PICHU_ROWS[0], PICHU_ANIMATION, false),
        row::<KirbyJolt>(PICHU_ROWS[1], PICHU_ANIMATION + 1, true),
    ]
}
