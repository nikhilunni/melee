//! Kirby's copied neutral specials: ftKb_Init_803C9DD0 (grounded) and
//! ftKb_Init_803C9E54 (aerial), indexed by the copied kind (ftkirby.c:
//! 2349-2419), and the per-kind arms of the code around them.
//!
//! Each copied special lives in a module of its own that shares the
//! original's states through its family crate, generic over the family's
//! trait (`ft_fox_family::special_n::Blaster` for Fox's and Falco's).
//! The kinds not yet ported fail closed with their entry's name.
use crate::init::Kirby;
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_types::FighterKind;

pub mod blaster;
pub mod captain;
pub mod shield_breaker;
pub mod rollout;
pub mod chef;
pub mod fireball;
pub mod donkey;
pub mod mewtwo;
pub mod samus;
pub mod thunder_jolt;
pub mod toad;
pub mod pk_flash;
pub mod egg_lay;
pub mod bow;
pub mod nayrus_love;
pub mod fire_breath;
pub mod ice_shot;
pub mod needle_storm;

/// One kind's entry in ftKb_Init_803C9DD0 / 803C9E54.
#[derive(Clone, Copy)]
pub enum CopiedNeutral {
    /// NULL in both tables, and Kirby's own row: the Inhale
    /// (ftKb_SpecialN_800F5F68 / 800F6070).
    Inhale,
    /// A ported entry, grounded or aerial by its second argument.
    Ported(fn(&mut Fighter, bool, &FighterAssets)),
    /// An entry not yet ported: the grounded and aerial functions.
    Unported {
        ground: &'static str,
        air: &'static str,
    },
}

const fn unported(ground: &'static str, air: &'static str) -> CopiedNeutral {
    CopiedNeutral::Unported { ground, air }
}

/// ftKb_Init_803C9DD0 and ftKb_Init_803C9E54, by `FighterKind`.
pub static NEUTRAL: [CopiedNeutral; FighterKind::MAX as usize] = [
    // Mario
    CopiedNeutral::Ported(fireball::enter_mario),
    // Fox
    CopiedNeutral::Ported(blaster::enter::<blaster::KirbyFox>),
    // Captain Falcon
    CopiedNeutral::Ported(captain::enter),
    // Donkey Kong
    CopiedNeutral::Ported(donkey::enter),
    // Kirby
    CopiedNeutral::Inhale,
    // Bowser
    CopiedNeutral::Ported(fire_breath::enter),
    // Link
    CopiedNeutral::Ported(bow::enter),
    // Sheik
    CopiedNeutral::Ported(needle_storm::enter),
    // Ness
    CopiedNeutral::Ported(pk_flash::enter),
    // Peach
    CopiedNeutral::Ported(toad::enter),
    // Ice Climbers
    CopiedNeutral::Ported(ice_shot::enter),
    // Nana: NULL (a swallowed Nana gives Popo's ability).
    CopiedNeutral::Inhale,
    // Pikachu
    CopiedNeutral::Ported(thunder_jolt::enter),
    // Samus
    CopiedNeutral::Ported(samus::enter),
    // Yoshi
    CopiedNeutral::Ported(egg_lay::enter),
    // Jigglypuff
    CopiedNeutral::Ported(rollout::enter),
    // Mewtwo
    CopiedNeutral::Ported(mewtwo::enter),
    // Luigi
    CopiedNeutral::Ported(fireball::enter_luigi),
    // Marth
    CopiedNeutral::Ported(shield_breaker::enter::<shield_breaker::KirbyMarth>),
    // Zelda
    CopiedNeutral::Ported(nayrus_love::enter),
    // Young Link: Link's entries.
    CopiedNeutral::Ported(bow::enter),
    // Dr. Mario: Mario's entries.
    CopiedNeutral::Ported(fireball::enter_mario),
    // Falco: Fox's entries.
    CopiedNeutral::Ported(blaster::enter::<blaster::KirbyFalco>),
    // Pichu: Pikachu's entries.
    CopiedNeutral::Ported(thunder_jolt::enter),
    // Mr. Game & Watch
    CopiedNeutral::Ported(chef::enter),
    // Ganondorf: Captain Falcon's entries.
    CopiedNeutral::Ported(captain::enter),
    // Roy: Marth's entries.
    CopiedNeutral::Ported(shield_breaker::enter::<shield_breaker::KirbyRoy>),
    // Master Hand, Crazy Hand and the Wireframes: NULL.
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    // Giga Bowser: ftKb_Init_803C9DD0/803C9E54; no retail witness yet.
    unported("ftKb_SpecialNKp_800FA8B4", "ftKb_SpecialNKp_800FA958"),
    // Sandbag: NULL.
    CopiedNeutral::Inhale,
];

/// ftKb_SpecialN_Enter (800F1638) / ftKb_SpecialAirN_Enter: the copied
/// kind's entry, or the Inhale.
pub fn enter_neutral(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    let copied = f.character.get::<Kirby>().copied;
    match NEUTRAL[i32::from(copied) as usize] {
        CopiedNeutral::Inhale => crate::special_n::enter(f, airborne, assets),
        CopiedNeutral::Ported(enter) => enter(f, airborne, assets),
        CopiedNeutral::Unported { ground, air } => unimplemented!(
            "{} (ftkirby.c:3777-3790, ftKb_Init_803C9DD0/803C9E54): Kirby's copied {copied:?} neutral special",
            if airborne { air } else { ground }
        ),
    }
}

/// The copied specials' motion rows (ftKb_Init_MotionStateTable 399..543)
/// that are ported: the Blaster's here, each other kind's in its module
/// (`captain::rows`, `rollout::rows`, `chef::rows`, `samus::rows`,
/// `donkey::rows`, `mewtwo::rows`).
pub const fn rows() -> [MotionRow; 12] {
    blaster::rows()
}

/// deal_dmg_cb of a copied special's row: the copied Rollout's
/// fn_80100E0C, which does nothing unless one of its rows installed it.
pub fn deal_damage(f: &mut Fighter, assets: &FighterAssets) {
    rollout::deal_damage(f, assets);
}

/// x21F8 after a cape turnaround (ftCo_800C37A0): the copied Rollout's
/// fn_80105978 is the only one a Kirby row installs.
/// Jigglypuff's generic (`ft_purin_family::rollout::cape_turn_end`) is the
/// same code, but the witness needs three fighters (Mario, Kirby,
/// Jigglypuff); it stays fail-closed until such a match gates.
pub fn cape_turn_end(_: &mut Fighter) {
    unimplemented!(
        "fn_80105978 (ftkirbyspecialpurin.c:1368): the copied Rollout after a cape turnaround"
    )
}

/// accessory4_cb of a copied special's row.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    blaster::accessory(f, assets);
    shield_breaker::accessory(f, assets);
}

/// What an article reads of Kirby (the copied Blaster's
/// ftKb_SpecialNFx_800FDD4C / 800FDD14).
pub fn item_owner(f: &mut Fighter) -> melee_it::ItemOwner {
    blaster::item_owner(f)
        .or_else(|| samus::item_owner(f))
        .or_else(|| mewtwo::item_owner(f))
        .unwrap_or_else(|| melee_it::ItemOwner {
            // The copied PK Flash's ftKb_SpecialNNs_800FEC78.
            steering_article: pk_flash::steering(f),
            // The copied bow's ftKb_SpecialNLk800FB394.
            article_stage: bow::stage(f),
            // ftKb_SpecialNSk_80105FF0, for the copied needle bundle.
            holds_needles: needle_storm::holds_bundle(f),
            ..melee_ft::fighter::default_item_owner(f)
        })
}

/// The copied Blaster's flash point (ftKb_SpecialNFx_800FDC00 / 800FDC70).
pub fn item_muzzle(f: &mut Fighter, assets: &FighterAssets) -> Option<(hsd_types::Vec3, f32)> {
    blaster::item_muzzle(f, assets)
}

/// The articles' Destroyed callbacks reaching Kirby: the hammer's
/// (ftKb_SpecialAirLw_800F539C) and the copied specials'.
pub fn article_destroyed(f: &mut Fighter, kind: melee_types::ItemKind) {
    crate::special_s::article_destroyed(f, kind);
    chef::article_destroyed(f, kind);
    ice_shot::article_destroyed(f, kind);
    samus::article_destroyed(f, kind);
    mewtwo::article_destroyed(f, kind);
    toad::article_destroyed(f, kind);
    pk_flash::article_destroyed(f, kind);
    bow::article_destroyed(f, kind);
}

/// shield_hit_cb's volume of a copied special against a fighter's hit
/// (the copied Toad's).
pub fn defense_contact(
    f: &mut Fighter,
    attacker: &mut Fighter,
    assets: &FighterAssets,
    id: usize,
) -> bool {
    toad::contact(f, attacker, assets, id)
}

/// The same against an item's hit.
pub fn item_defense_contact(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    toad::item_contact(f, item, id, assets)
}

/// shield_hit_cb itself (the copied Toad's fn_8010CD88).
pub fn process_defense_hit(f: &mut Fighter, assets: &FighterAssets) {
    toad::process_hit(f, assets);
}
