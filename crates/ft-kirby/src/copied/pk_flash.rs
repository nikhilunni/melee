//! The copied PK Flash, ftkirbyspecialness.c (800FEC78..800FF8EC): Ness's
//! states (`ft_ness_family::pk_flash`) over Kirby's rows 435..442, with
//! his attributes (+1A0), his waist and It_Kind_Kirby_NessPKFlush.
//!
//! Unlike Ness's, Kirby's code never clears take_dmg_cb / death2_cb with
//! the flash pointer (800FECBC, the IASA at 800FF444, the end rows' reset
//! at 800FF0CC): a motion change does.
use crate::init::Kirby;
use ft_ness_family::pk_flash::{self, PkFlash, PkFlashAttributes, PkFlashUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, FtPart, ItemKind};

/// Kirby with Ness's ability.
pub struct KirbyNess;

impl PkFlashUser for KirbyNess {
    /// ftKb_MS_NsSpecialNStart (435): Start, Hold0, Hold1, End, then the
    /// aerial four (439..442).
    const START: ActionId = ActionId(435);
    /// ftKb_SM_NsSpecialNStart.
    const START_ANIMATION: i32 = 370;
    /// lb_8000B1CC(fp->parts[FtPart_WaistN].joint, ...) (800FEECC..D8).
    const JOINT: FtPart = FtPart::WaistN;
    const FLASH: ItemKind = ItemKind::KirbyNessPKFlush;
    const CLEARS_CALLBACKS: bool = false;
    /// ftKb_DatAttrs +1A0..+1BC.
    fn attributes(f: &Fighter) -> PkFlashAttributes {
        f.character.get::<Kirby>().attributes.ness_pk_flash
    }
    fn scratch(f: &mut Fighter) -> &mut PkFlash {
        &mut f.character.get_mut::<Kirby>().pk_flash
    }
    fn state(f: &Fighter) -> &PkFlash {
        &f.character.get::<Kirby>().pk_flash
    }
    /// death2_cb = ftKb_Init_800EE74C, take_dmg_cb = ftKb_Init_800EE7B8.
    fn set_damage_callbacks(f: &mut Fighter, installed: bool) {
        f.character.get_mut::<Kirby>().damage_callbacks = installed;
    }
}

/// ftKb_SpecialNNs_800FED38 / 800FEDD0 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    pk_flash::enter::<KirbyNess>(f, airborne, assets);
}

/// Rows 435..442: ftKb_NsSpecialN* and ftKb_NsSpecialAirN*.
pub const fn rows() -> [MotionRow; 8] {
    pk_flash::rows::<KirbyNess>()
}

/// ftKb_SpecialNNs_800FEC78 (800FEC78), which the flash reads for the
/// steering and the burst: Kirby holds it in row 436 or 440.
pub fn steering(f: &Fighter) -> bool {
    f.character.get::<Kirby>().copied == FighterKind::Ness && pk_flash::holding::<KirbyNess>(f)
}

/// ftKb_SpecialNNs_800FECE8 (800FECE8), the Ness arm of 800F19AC and
/// 800F1A8C: a flash still Kirby's flies on without him (it_802AAA50).
pub fn put_away(f: &mut Fighter) {
    pk_flash::orphan_flash::<KirbyNess>(f);
}

/// it_2725_Logic102_Destroyed -> ftKb_SpecialNNs_800FECBC (800FECBC): the
/// flash is no longer Kirby's.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyNessPKFlush {
        pk_flash::flash_gone::<KirbyNess>(f);
    }
}
