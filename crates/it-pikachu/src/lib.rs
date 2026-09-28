//! Pikachu's articles (ftData.x48_items of PlPk.dat): [0] Thunder's
//! bolt, [1] the Thunder Jolt ball and [2] the crawler it rides.
pub mod jolt;
pub mod thunder;
pub use jolt::{ThunderJoltBall, ThunderJoltCrawler};
pub use thunder::ThunderBolt;

/// ftPk_Init_OnLoad's registrations: ftData.x48_items indices.
pub const THUNDER_ARTICLE_INDEX: u32 = 0;
pub const BALL_ARTICLE_INDEX: u32 = 1;
pub const CRAWLER_ARTICLE_INDEX: u32 = 2;
