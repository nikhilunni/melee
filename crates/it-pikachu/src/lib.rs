//! Pikachu's and Pichu's articles (ftData.x48_items of PlPk.dat and
//! PlPc.dat): [0] Thunder's bolt, [1] the Thunder Jolt ball and [2] the
//! crawler it rides. The two kinds of each run the same item code
//! (it_3F2F.c's logic table repeats the Pikachu rows for Pichu); their
//! sizes and timings are the owner file's article data. Kirby's copied
//! Thunder Jolt spawns the same ball and crawler under kinds of its own
//! (It_Kind_Kirby_PikachuTJolt_*, It_Kind_Kirby_PichuTJolt_*), whose
//! articles are his copy archives' (PlKbCpPk.dat, PlKbCpPc.dat).
use melee_types::ItemKind;

pub mod jolt;
pub mod thunder;
pub use jolt::{ThunderJoltBall, ThunderJoltCrawler};
pub use thunder::ThunderBolt;

/// ftPk_Init_OnLoad's registrations: ftData.x48_items indices.
pub const THUNDER_ARTICLE_INDEX: u32 = 0;
pub const BALL_ARTICLE_INDEX: u32 = 1;
pub const CRAWLER_ARTICLE_INDEX: u32 = 2;

/// The item kinds one owner's Thunder Jolt takes.
pub trait Owner: 'static {
    const BALL: ItemKind;
    const CRAWLER: ItemKind;
    /// it_802B3554's efSync on the ball: 0x4BD for the family's own
    /// kinds, 0x4AD (efalt.c:450, EfKbPk.dat) for Kirby's.
    const TRAIL: u16;
}
/// The family members' Thunder, which Kirby's copy does not have.
pub trait ThunderOwner: Owner {
    const THUNDER: ItemKind;
}
/// It_Kind_Pikachu_Thunder, _TJolt_Ground and _TJolt_Air.
pub struct Pikachu;
impl Owner for Pikachu {
    const BALL: ItemKind = ItemKind::PikachuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::PikachuTJoltAir;
    const TRAIL: u16 = FAMILY_TRAIL;
}
impl ThunderOwner for Pikachu {
    const THUNDER: ItemKind = ItemKind::PikachuThunder;
}
/// It_Kind_Pichu_Thunder, _TJolt_Ground and _TJolt_Air.
pub struct Pichu;
impl Owner for Pichu {
    const BALL: ItemKind = ItemKind::PichuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::PichuTJoltAir;
    const TRAIL: u16 = FAMILY_TRAIL;
}
impl ThunderOwner for Pichu {
    const THUNDER: ItemKind = ItemKind::PichuThunder;
}
/// It_Kind_Kirby_PikachuTJolt_Ground and _Air.
pub struct KirbyPikachu;
impl Owner for KirbyPikachu {
    const BALL: ItemKind = ItemKind::KirbyPikachuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::KirbyPikachuTJoltAir;
    const TRAIL: u16 = KIRBY_TRAIL;
}
/// It_Kind_Kirby_PichuTJolt_Ground and _Air.
pub struct KirbyPichu;
impl Owner for KirbyPichu {
    const BALL: ItemKind = ItemKind::KirbyPichuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::KirbyPichuTJoltAir;
    const TRAIL: u16 = KIRBY_TRAIL;
}
/// it_802B3554 (itpikachutjoltground.c:144-158): the ball's trail.
const FAMILY_TRAIL: u16 = 0x4BD;
const KIRBY_TRAIL: u16 = 0x4AD;

/// Kirby's copy archives' hat_dynamics slots (ftKb_SpecialN_800F16D0,
/// ftkirby.c:3850-3862): [0] the ball, [1] the crawler.
pub const KIRBY_BALL_SLOT: u32 = 0;
pub const KIRBY_CRAWLER_SLOT: u32 = 1;
