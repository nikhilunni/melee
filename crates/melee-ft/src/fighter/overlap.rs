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
            // x2C4, scaled by ftCo_800D105C (ftchangeparam.c:193-195).
            center: assets.overlap.center * self.attribute_scale,
            half_width: assets.overlap.half_width * self.attribute_scale,
            facing: self.physics.facing,
            floor: self.collision.data.floor.index,
            eligible: !self.status.disabled && self.physics.ground_or_air == GroundOrAir::Ground,
            linked: self.combat.grab.is_some(),
            ignore_others: self.status.ignore_fighter_nudge,
            hitlag: self.in_hitlag(),
            out_of_play: self.out_of_play(),
            player: self.player.id,
            secondary: self.player.secondary,
        }
    }
}
