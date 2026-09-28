//! Link's and Young Link's articles (ftData.x48_items of PlLk.dat and
//! PlCl.dat): [0] the bomb, [1] the boomerang, [2] the hookshot, [3] the
//! arrow, [4] the bow (and Young Link's [5] milk). Ported: the boomerang
//! and the hookshot.
pub mod arrow;
pub mod boomerang;
pub mod bow;
pub mod hookshot;

pub type LinkBoomerang = boomerang::Boomerang<false>;
pub type YoungLinkBoomerang = boomerang::Boomerang<true>;
pub type LinkHookshot = hookshot::Hookshot<false>;
pub type YoungLinkHookshot = hookshot::Hookshot<true>;
pub type LinkBow = bow::Bow<false>;
pub type YoungLinkBow = bow::Bow<true>;
pub type LinkArrow = arrow::Arrow<false>;
pub type YoungLinkArrow = arrow::Arrow<true>;
