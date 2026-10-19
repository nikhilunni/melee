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
    unported("ftKb_SpecialN_800F9110", "ftKb_SpecialNMr_800F93CC"),
    // Fox
    CopiedNeutral::Ported(blaster::enter::<blaster::KirbyFox>),
    // Captain Falcon
    CopiedNeutral::Ported(captain::enter),
    // Donkey Kong
    unported("ftKb_SpecialNDk_800FF8EC", "ftKb_SpecialNDk_800FFA10"),
    // Kirby
    CopiedNeutral::Inhale,
    // Bowser
    unported("ftKb_SpecialNKp_800FA8B4", "ftKb_SpecialNKp_800FA958"),
    // Link
    unported("ftKb_SpecialNLk800FB880", "ftKb_SpecialNLk800FBA00"),
    // Sheik
    unported("ftKb_SpecialNSk_8010612C", "ftKb_SpecialNSk_801061E4"),
    // Ness
    unported("ftKb_SpecialNNs_800FED38", "ftKb_SpecialNNs_800FEDD0"),
    // Peach
    unported("ftKb_SpecialNPe_8010C4D4", "ftKb_SpecialNPe_8010C560"),
    // Ice Climbers
    unported("ftKb_SpecialNIc_80108D64", "ftKb_SpecialNIc_80108E14"),
    // Nana: NULL (a swallowed Nana gives Popo's ability).
    CopiedNeutral::Inhale,
    // Pikachu
    unported("ftKb_SpecialNPk_800F9FD4", "ftKb_SpecialNPk_800FA064"),
    // Samus
    unported("ftKb_SpecialNSs_800FCF74", "ftKb_SpecialNSs_800FD020"),
    // Yoshi
    unported("ftKb_SpecialNYs_8010941C", "ftKb_SpecialNYs_801094FC"),
    // Jigglypuff
    unported("ftKb_SpecialNPr_80101560", "ftKb_SpecialNPr_80101618"),
    // Mewtwo
    unported("ftKb_SpecialNMt_80107568", "ftKb_SpecialNMt_80107638"),
    // Luigi
    unported("ftKb_SpecialNLg_800F9614", "ftKb_SpecialNLg_800F9684"),
    // Marth
    CopiedNeutral::Ported(shield_breaker::enter::<shield_breaker::KirbyMarth>),
    // Zelda
    unported("ftKb_SpecialNZd_80105B2C", "ftKb_SpecialNZd_80105BA8"),
    // Young Link: Link's entries.
    unported("ftKb_SpecialNLk800FB880", "ftKb_SpecialNLk800FBA00"),
    // Dr. Mario: Mario's entries.
    unported("ftKb_SpecialN_800F9110", "ftKb_SpecialNMr_800F93CC"),
    // Falco: Fox's entries.
    CopiedNeutral::Ported(blaster::enter::<blaster::KirbyFalco>),
    // Pichu: Pikachu's entries.
    unported("ftKb_SpecialNPk_800F9FD4", "ftKb_SpecialNPk_800FA064"),
    // Mr. Game & Watch
    unported("ftKb_SpecialNGw_8010D188", "ftKb_SpecialNGw_8010D204"),
    // Ganondorf: Captain Falcon's entries.
    CopiedNeutral::Ported(captain::enter),
    // Roy: Marth's entries.
    CopiedNeutral::Ported(shield_breaker::enter::<shield_breaker::KirbyRoy>),
    // Master Hand, Crazy Hand and the Wireframes: NULL.
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    CopiedNeutral::Inhale,
    // Giga Bowser: Bowser's entries.
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
/// (`captain::rows`).
pub const fn rows() -> [MotionRow; 12] {
    blaster::rows()
}

/// accessory4_cb of a copied special's row.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    blaster::accessory(f, assets);
    shield_breaker::accessory(f, assets);
}

/// What an article reads of Kirby (the copied Blaster's
/// ftKb_SpecialNFx_800FDD4C / 800FDD14).
pub fn item_owner(f: &mut Fighter) -> melee_it::ItemOwner {
    blaster::item_owner(f).unwrap_or_else(|| melee_ft::fighter::default_item_owner(f))
}

/// The copied Blaster's flash point (ftKb_SpecialNFx_800FDC00 / 800FDC70).
pub fn item_muzzle(f: &mut Fighter, assets: &FighterAssets) -> Option<(hsd_types::Vec3, f32)> {
    blaster::item_muzzle(f, assets)
}
