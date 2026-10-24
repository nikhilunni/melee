//! The copied Giant Punch, ftkirbyspecialdonkey.c: Donkey Kong's states
//! (`ft_donkey_family::giant_punch`) over Kirby's rows 455..464, colour 58,
//! efAlt 0x4A9 / 0x4AA (EfKbDk.dat) and the swings kept in u.kb.xBC. No
//! article; the states arm ftKb_SpecialN_set_cbs and the efLib hitlag
//! pair, with no take_dmg_2_cb (Donkey Kong's own destroys his effects on
//! any hit in the move).
use crate::init::Kirby;
use ft_donkey_family::giant_punch::{self, GiantPunch, GiantPunchAttributes, GiantPunchUser};
use melee_ef::request::EffectRequest;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};

/// Kirby with Donkey Kong's ability.
pub struct KirbyDonkey;

impl GiantPunchUser for KirbyDonkey {
    /// ftKb_MS_DkSpecialNStart (455) .. DkSpecialAirNFull (464).
    const START: ActionId = ActionId(455);
    /// ftKb_SM_DkSpecialNStart (the table's anim id, 0x803C8368 row 455).
    const FIRST_ANIMATION: i32 = 390;
    /// ftKb_DkSpecialNLoop_Anim: ftCo_800BFFD0(fp, 58, 0).
    const FULL_CHARGE_COLOR: u8 = 58;
    /// efSync_Spawn(1193 / 1194, gobj, parts[TopN], &facing_dir): efAlt
    /// 0x4A9 / 0x4AA, EfKbDk.dat's models 0x9858 / 0x9859.
    const PUNCH_EFFECTS: [u16; 2] = [0x4A9, 0x4AA];
    /// ftKb_DatAttrs +190 (`specialn_dk_*`).
    fn attributes(f: &Fighter) -> GiantPunchAttributes {
        f.character.get::<Kirby>().attributes.giant_punch
    }
    fn scratch(f: &mut Fighter) -> &mut GiantPunch {
        &mut f.character.get_mut::<Kirby>().giant_punch
    }
    fn swings(f: &Fighter) -> i32 {
        f.character.get::<Kirby>().giant_punch_swings
    }
    /// u.kb.xBC, and ftKb_Init_UnkMotionStates4's Donkey Kong arm.
    fn store_swings(f: &mut Fighter, swings: i32) {
        f.character.get_mut::<Kirby>().giant_punch_swings = swings;
        crate::charge_flash::refresh(f);
    }
    /// ftKb_DkSpecialNStart_Coll_inline: ftKb_SpecialN_set_cbs and
    /// Fighter_SetEffectHitlagCallbacks.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
        f.effect_state.hitlag_callbacks = true;
    }
    /// pre_hitlag_cb = post_hitlag_cb = NULL.
    fn clear_callbacks(f: &mut Fighter) {
        f.effect_state.hitlag_callbacks = false;
    }
}

/// ftKb_SpecialNDk_800FF8EC / 800FFA10 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    giant_punch::enter::<KirbyDonkey>(f, airborne, assets);
}

/// Rows 455..464 (ftKb_MS_DkSpecialN*), ftKb_DkSpecialN*.
pub const fn rows() -> [MotionRow; 10] {
    giant_punch::rows::<KirbyDonkey>()
}

/// ftKb_SpecialNPr_80100DE0 (80100DE0), Donkey Kong's arm of 800F19AC and
/// 800F1A8C: the stored swings go (xBC = 0), then every effect of Kirby's.
pub fn put_away(f: &mut Fighter) {
    KirbyDonkey::store_swings(f, 0);
    f.effects.push(EffectRequest::DestroyOwned);
}
