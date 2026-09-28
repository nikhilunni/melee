//! Pikachu's and Pichu's articles (ftData.x48_items of PlPk.dat and
//! PlPc.dat): [0] Thunder's bolt, [1] the Thunder Jolt ball and [2] the
//! crawler it rides. The two kinds of each run the same item code
//! (it_3F2F.c's logic table repeats the Pikachu rows for Pichu); their
//! sizes and timings are the owner file's article data.
use melee_types::ItemKind;

pub mod jolt;
pub mod thunder;
pub use jolt::{ThunderJoltBall, ThunderJoltCrawler};
pub use thunder::ThunderBolt;

/// ftPk_Init_OnLoad's registrations: ftData.x48_items indices.
pub const THUNDER_ARTICLE_INDEX: u32 = 0;
pub const BALL_ARTICLE_INDEX: u32 = 1;
pub const CRAWLER_ARTICLE_INDEX: u32 = 2;

/// The item kinds one family member's articles take.
pub trait Owner: 'static {
    const THUNDER: ItemKind;
    const BALL: ItemKind;
    const CRAWLER: ItemKind;
}
/// It_Kind_Pikachu_Thunder, _TJolt_Ground and _TJolt_Air.
pub struct Pikachu;
impl Owner for Pikachu {
    const THUNDER: ItemKind = ItemKind::PikachuThunder;
    const BALL: ItemKind = ItemKind::PikachuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::PikachuTJoltAir;
}
/// It_Kind_Pichu_Thunder, _TJolt_Ground and _TJolt_Air.
pub struct Pichu;
impl Owner for Pichu {
    const THUNDER: ItemKind = ItemKind::PichuThunder;
    const BALL: ItemKind = ItemKind::PichuTJoltGround;
    const CRAWLER: ItemKind = ItemKind::PichuTJoltAir;
}
