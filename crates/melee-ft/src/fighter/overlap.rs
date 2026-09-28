//! Fighter-specific eligibility and stage context for body separation.
use super::{assets::FighterAssets, FighterCore};
use melee_coll::overlap::OverlapBody;
use melee_types::GroundOrAir;
impl FighterCore {
    pub fn overlap_body(&self, assets: &FighterAssets) -> OverlapBody {
        OverlapBody {
            // ftCommon_8007F8B4 (8007F8B4): deferred displacement is zero
            // throughout the supported ordinary, uncaptured states.
            position: self.physics.position,
            center: assets.overlap.center * self.player.scale,
            half_width: assets.overlap.half_width * self.player.scale,
            facing: self.physics.facing,
            floor: self.collision.data.floor.index,
            eligible: !self.status.disabled && self.physics.ground_or_air == GroundOrAir::Ground,
            linked: self.combat.grab.is_some(),
            ignore_others: self.status.ignore_fighter_nudge,
            hitlag: self.in_hitlag(),
        }
    }
}
