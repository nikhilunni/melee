//! Pikachu's articles (ftData.x48_items of PlPk.dat): [1] the Thunder
//! Jolt ball and [2] the crawler it rides.
pub mod jolt;
pub use jolt::{ThunderJoltBall, ThunderJoltCrawler};

/// ftPk_Init_OnLoad's registrations: ftData.x48_items indices.
pub const BALL_ARTICLE_INDEX: u32 = 1;
pub const CRAWLER_ARTICLE_INDEX: u32 = 2;
