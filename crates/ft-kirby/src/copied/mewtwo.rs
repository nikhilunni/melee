//! The copied Shadow Ball, ftkirbyspecialmewtwo.c: Mewtwo's states
//! (`ft_mewtwo_family::shadow_ball`) over Kirby's rows 500..509, his right
//! hand, colour 93, the flash four units above TopN, and the article
//! It_Kind_Kirby_MewtwoShadowBall. Unlike Mewtwo's file, every removal of
//! the ball and its release also destroy Kirby's effects
//! (ftKb_SpecialNMt_80107040 and its inlined twins).
use crate::init::Kirby;
use ft_mewtwo_family::shadow_ball::{self, ShadowBall, ShadowBallAttributes, ShadowBallUser};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, ItemKind};

/// Kirby with Mewtwo's ability.
pub struct KirbyMewtwo;

impl ShadowBallUser for KirbyMewtwo {
    /// ftKb_MS_MtSpecialNStart (500) .. MtSpecialAirNEnd (509).
    const START: ActionId = ActionId(500);
    /// ftKb_SM_MtSpecialNStart (the table's anim id, 0x803C8368 row 500).
    const START_ANIMATION: i32 = 435;
    const BALL: ItemKind = ItemKind::KirbyMewtwoShadowBall;
    /// `fp->parts[FtPart_RHandN]` (retail 0x8010775C: lwz 0x270 of
    /// parts, 39 * 0x10).
    const BALL_JOINT: usize = 39;
    /// ftKb_Init_803B7570 / 803B757C.
    const FLASH_OFFSET: Vec3 = Vec3::new(0.0, 4.0, 0.0);
    /// ftKb_MtSpecialNLoop_Anim: ftCo_800BFFD0(fp, 0x5D, 0).
    const FULL_CHARGE_COLOR: u8 = 0x5D;
    const DESTROYS_EFFECTS: bool = true;
    /// ftKb_DatAttrs +384 (`specialn_mt_*`).
    fn attributes(f: &Fighter) -> ShadowBallAttributes {
        f.character.get::<Kirby>().attributes.shadow_ball
    }
    fn scratch(f: &mut Fighter) -> &mut ShadowBall {
        &mut f.character.get_mut::<Kirby>().shadow_ball
    }
    fn charge(f: &Fighter) -> i32 {
        f.character.get::<Kirby>().shadow_ball_charge
    }
    /// u.kb.x9C, and ftKb_Init_UnkMotionStates4's Mewtwo arm.
    fn store_charge(f: &mut Fighter, charge: i32) {
        f.character.get_mut::<Kirby>().shadow_ball_charge = charge;
        crate::charge_flash::refresh(f);
    }
    fn held(f: &Fighter) -> bool {
        f.character.get::<Kirby>().shadow_ball_held
    }
    fn set_held(f: &mut Fighter, held: bool) {
        f.character.get_mut::<Kirby>().shadow_ball_held = held;
    }
    /// ftKb_SpecialN_set_cbs.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
}

/// ftKb_SpecialNMt_80107568 / 80107638 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    shadow_ball::enter::<KirbyMewtwo>(f, airborne, assets);
}

/// Rows 500..509 (ftKb_MS_MtSpecialN*), ftKb_MtSpecialN*.
pub const fn rows() -> [MotionRow; 10] {
    shadow_ball::rows::<KirbyMewtwo>()
}

/// ftKb_SpecialNMt_8010709C (8010709C), Mewtwo's arm of 800F19AC: the ball
/// goes with Kirby's effects, then the charge (x9C = 0), the secondary
/// colour slot (ftCo_800BFFAC) and Kirby's effects again.
pub fn put_away(f: &mut Fighter) {
    shadow_ball::remove_ball::<KirbyMewtwo>(f);
    drop_charge(f);
}

/// ftKb_SpecialNMt_80107130 (80107130), the arm of 800F1A8C: the same,
/// except that a full charge (`(float) x9C == specialn_mt_charge_time`)
/// is kept.
pub fn put_away_on_hit(f: &mut Fighter) {
    shadow_ball::remove_ball::<KirbyMewtwo>(f);
    if !shadow_ball::is_full::<KirbyMewtwo>(f) {
        drop_charge(f);
    }
}

fn drop_charge(f: &mut Fighter) {
    KirbyMewtwo::store_charge(f, 0);
    f.core.clear_secondary_color_slot();
    f.effects.push(EffectRequest::DestroyOwned);
}

/// it_2725_Logic101_Destroyed's Kirby arm, ftKb_SpecialNMt_80107040: a
/// ball still in hand lets Kirby go of it, and his effects go.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyMewtwoShadowBall {
        KirbyMewtwo::set_held(f, false);
        f.effects.push(EffectRequest::DestroyOwned);
    }
}

/// ftKb_SpecialNMt_80106F44 (80106F44) for the ball in hand: the stored
/// charge and `(s32) specialn_mt_charge_time`, while Kirby holds one.
pub fn item_owner(f: &mut Fighter) -> Option<melee_it::ItemOwner> {
    if f.character.get::<Kirby>().copied != FighterKind::Mewtwo {
        return None;
    }
    let held = KirbyMewtwo::held(f);
    let charge = KirbyMewtwo::charge(f);
    Some(melee_it::ItemOwner {
        charge: held.then(|| (charge, shadow_ball::full_charge::<KirbyMewtwo>(f))),
        ..melee_ft::fighter::default_item_owner(f)
    })
}
