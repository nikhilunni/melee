//! PK Flash, ftnessspecialn.c (80116E68..80117B70): the family's states
//! (`ft_ness_family::pk_flash`) over Ness's rows 348..355.
use crate::init::Ness;
use ft_ness_family::pk_flash::{self, PkFlashAttributes, PkFlashUser};
pub use ft_ness_family::pk_flash::PkFlash;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FtPart, ItemKind};

/// ftNs_MS_SpecialNStart..SpecialNEnd (348..351) and the aerial rows
/// (352..355).
pub const START: ActionId = ActionId(348);

impl PkFlashUser for Ness {
    const START: ActionId = START;
    /// ftNs_Init_MotionStateTable's anim_id column (0x803CC650).
    const START_ANIMATION: i32 = 0x12B;
    /// lb_8000B1CC(fp->parts[FtPart_L2ndNa].joint, ...).
    const JOINT: FtPart = FtPart::L2ndNa;
    const FLASH: ItemKind = ItemKind::NessPKFlush;
    const CLEARS_CALLBACKS: bool = true;
    fn attributes(f: &Fighter) -> PkFlashAttributes {
        f.character.get::<Ness>().attributes.pk_flash
    }
    fn scratch(f: &mut Fighter) -> &mut PkFlash {
        &mut f.character.get_mut::<Ness>().pk_flash
    }
    fn state(f: &Fighter) -> &PkFlash {
        &f.character.get::<Ness>().pk_flash
    }
    /// death2_cb and take_dmg_cb = ftNs_Init_OnDamage.
    fn set_damage_callbacks(f: &mut Fighter, installed: bool) {
        f.character.get_mut::<Ness>().damage_callbacks = installed;
    }
}

pub const fn rows() -> [MotionRow; 8] {
    pk_flash::rows::<Ness>()
}

/// ftNs_SpecialNStart_Enter (80116F94) / ftNs_SpecialAirNStart_Enter
/// (80117034).
pub fn enter(f: &mut Fighter, air_entry: bool, a: &FighterAssets) {
    pk_flash::enter::<Ness>(f, air_entry, a);
}

/// ftNs_SpecialN_ItemPKFlushSetNULL (80116F00), from ftNs_Init_OnDamage:
/// the flash flies on without Ness (it_802AAA50), and his callbacks go.
pub fn orphan_flash(f: &mut Fighter) {
    pk_flash::orphan_flash::<Ness>(f);
}

/// ftNs_SpecialN_SetNULL (80116EBC): the flash is no longer Ness's, and
/// take_dmg_cb / death2_cb clear.
pub fn flash_gone(f: &mut Fighter) {
    pk_flash::flash_gone::<Ness>(f);
}

/// ftNs_SpecialN_CheckSpecialNHold (80116E68), which the flash reads: Ness
/// still holds it in the hold row (349 / 353).
pub fn holding(f: &Fighter) -> bool {
    pk_flash::holding::<Ness>(f)
}
