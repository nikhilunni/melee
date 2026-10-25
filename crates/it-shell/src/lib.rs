//! Koopa shells, random items a fighter throws to send them sliding along
//! the floor: the Green Shell (itgshell.c) and the Red Shell (itrshell.c).
use melee_it::ItemCore;

mod green;
pub use green::GreenShell;
mod red;
pub use red::RedShell;
/// The Red Shell's article tables, for the scene's asset loading.
pub mod red_assets {
    pub use crate::red::{ARTICLE_STATES, SPECIAL_ATTRIBUTES};
}
/// The Green Shell's article tables, for the scene's asset loading.
pub mod green_assets {
    pub use crate::green::{ARTICLE_STATES, SPECIAL_ATTRIBUTES};
}

/// it_8027236C (8027236C): the fighter that struck it (xCEC), or the
/// fighter owning the article that did (xCF0), owns it now; its attack
/// identity resets (it_8027B1F4).
pub(crate) fn take_striker_as_owner(item: &mut ItemCore) {
    if item.struck_by_item && item.hit_by.is_none() {
        unimplemented!("it_8027236C: an unowned article strikes a shell (itcoll.c:1373)");
    }
    item.owner = item.hit_by;
    item.owner_secondary = item.hit_by_secondary;
    item.stale_source = None;
}
