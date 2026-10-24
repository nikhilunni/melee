//! The copied Charge Shot, ftkirbyspecialsamus.c: Samus's states
//! (`ft_samus_family::charge_shot`) over Kirby's rows 407..412, his right
//! thumb, colour 54, efAlt 0x4A4 and the article It_Kind_Kirby_SamusCharge.
//! Two things differ from Samus's file beyond the names: the charge's
//! efLib_DestroyAll runs whether or not u.kb.xAC is set
//! (ftKb_SpecialNSs_DestroyChargeShot), and the put-away arm
//! (ftKb_SpecialNSs_800FCD60) resets the kept level.
use crate::init::Kirby;
use ft_samus_family::charge_shot::{self, ChargeShotAttributes, ChargeShotUser, ChargeState};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, ItemKind};

/// Kirby with Samus's ability.
pub struct KirbySamus;

impl ChargeShotUser for KirbySamus {
    /// ftKb_MS_SsSpecialNStart (407) .. SsSpecialAirN (412).
    const START: ActionId = ActionId(407);
    /// ftKb_SM_SsSpecialNStart (the table's anim id, 0x803C8368 row 407).
    const START_ANIMATION: i32 = 346;
    const SHOT: ItemKind = ItemKind::KirbySamusCharge;
    /// `fp->parts[FtPart_R3rdNa]` (retail 0x800FD124: lwz 0x2C0 of parts,
    /// 44 * 0x10), for the shot's forming and its launch alike.
    const HAND_PART: usize = 44;
    const MUZZLE_PART: usize = 44;
    /// ftKb_SsSpecialNHold_Anim: ftCo_800BFFD0(fp, 0x36, 0).
    const FULL_CHARGE_COLOR: u8 = 0x36;
    /// efSync_Spawn(0x4A4, gobj, &pos, &facing_dir): model 0x84D0
    /// (EfKbSs.dat) turned to the facing.
    const MUZZLE_FLASH: u16 = 0x4A4;
    /// ftKb_DatAttrs +168 (`specialn_ss_*`).
    fn attributes(f: &Fighter) -> ChargeShotAttributes {
        f.character.get::<Kirby>().attributes.charge_shot
    }
    fn state(f: &Fighter) -> &ChargeState {
        &f.character.get::<Kirby>().charge_shot
    }
    fn state_mut(f: &mut Fighter) -> &mut ChargeState {
        &mut f.character.get_mut::<Kirby>().charge_shot
    }
    /// ftKb_SpecialN_set_cbs.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
    /// ftKb_Init_UnkMotionStates4's Samus arm.
    fn level_changed(f: &mut Fighter) {
        crate::charge_flash::refresh(f);
    }
    /// ftKb_SpecialNSs_DestroyChargeShot: efLib_DestroyAll, then xAC = 0,
    /// with no test.
    fn take_charge_effects(f: &mut Fighter) -> bool {
        f.character.get_mut::<Kirby>().charge_shot.effects = false;
        true
    }
}

/// ftKb_SpecialNSs_800FCF74 / 800FD020 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    charge_shot::enter::<KirbySamus>(f, airborne, assets);
}

/// Rows 407..412 (ftKb_MS_SsSpecialN*), ftKb_SsSpecialN*.
pub const fn rows() -> [MotionRow; 6] {
    charge_shot::rows::<KirbySamus>()
}

/// ftKb_SpecialNSs_800FCD60 (800FCD60), Samus's arm of 800F19AC and
/// 800F1A8C: the shot in hand goes (ftKb_ChargeShot_inline, which destroys
/// Kirby's effects too) and the kept level resets.
pub fn put_away(f: &mut Fighter) {
    charge_shot::drop_shot::<KirbySamus>(f);
    charge_shot::set_charge_level::<KirbySamus>(f, 0);
}

/// it_2725_Logic108_Destroyed's Kirby arm, ftKb_SpecialNSs_800FCD04: a
/// shot still in hand lets Kirby go of it.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbySamusCharge {
        charge_shot::let_go::<KirbySamus>(f);
    }
}

/// ftKb_SpecialNSs_800FCC14 (800FCC14) for the shot in hand: the kept
/// level and `(long) specialn_ss_charge_time`, while Kirby holds one.
pub fn item_owner(f: &mut Fighter) -> Option<melee_it::ItemOwner> {
    if f.character.get::<Kirby>().copied != FighterKind::Samus {
        return None;
    }
    let state = f.character.get::<Kirby>().charge_shot;
    Some(melee_it::ItemOwner {
        charge: state
            .article
            .then(|| (state.level, charge_shot::full_charge::<KirbySamus>(f))),
        ..melee_ft::fighter::default_item_owner(f)
    })
}
