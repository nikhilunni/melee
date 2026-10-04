//! Ness's articles (ftData.x48_items of PlNs.dat, registered by
//! ftNs_Init_OnLoad): PK Fire's bolt [0] and pillar [1], PK Flash [2] and
//! its explosion [8], PK Thunder's head [3] and four trail kinds [4..=7],
//! the bat [9] and the yo-yo [10].
use melee_it::ItemCore;

pub mod bat;

pub use bat::NessBat;

fn no_physics(_item: &mut ItemCore, _ctx: &melee_it::ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut melee_it::ItemCollisionContext<'_>) -> bool {
    false
}
