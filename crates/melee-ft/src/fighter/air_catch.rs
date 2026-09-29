//! The aerial tether meeting a wall (ftCo_AirCatch.c): the hanging state
//! the kinds' AirCatchHit rows share (ftCo_800C3CC0, ftCo_AirCatchHit_Phys)
//! and the ways out of it their articles take: the wider ledge probe
//! (ftCo_800C3A14), the ledge climb's hop (ftCo_8009B390) and letting go
//! (ftCo_80090780).
use super::{
    assets::{FighterAssets, Result},
    ActionId, Fighter, MotionData, MotionEntryFlags,
};
use melee_mp::CollMap;
use melee_types::{mp::collide, GroundOrAir};

/// mv.co.aircatchhit (fp+2340): x0 is written and never read; x4 is the
/// kinds' hang countdown once their article is fully out (Samus's
/// mv.ss.grapple.x4, Link's mv.lk.specialn.x0.y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirCatchHitState {
    pub hang_frames: f32,
}

/// ftCo_800C3A14: the tether's ledge probe reaches 5 further up and 5
/// taller than the fighter's own (double adds).
const LEDGE_PROBE_EXTRA: f64 = 5.0;

impl Fighter {
    /// ftCo_800C3CC0 (800C3CC0): the kind's AirCatchHit (`action`) from
    /// frame 0 keeping the fast fall, the drift clamped to air_drift_max
    /// (ftCommon_ClampSelfVelX), mv.x0 = 20 and mv.x4 = 0, then off the
    /// ground if on it (ftCommon_8007D5D4).
    pub fn enter_air_catch_hit(&mut self, action: ActionId, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state_with_flags(
            action,
            assets,
            MotionEntryFlags::KEEP_FAST_FALL,
            0.0,
            1.0,
        )?;
        let maximum = self.core.attributes.air.air_drift_max;
        let velocity = self.core.physics.self_velocity.x;
        if velocity < -maximum {
            self.core.physics.self_velocity.x = -maximum;
        } else if velocity > maximum {
            self.core.physics.self_velocity.x = maximum;
        }
        self.core.state_data = MotionData::AirCatchHit(AirCatchHitState { hang_frames: 0.0 });
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.leave_ground();
        }
        Ok(())
    }

    /// mv.co.aircatchhit.x4, the hang countdown the kinds' articles keep.
    pub fn air_catch_hang_frames(&mut self) -> &mut f32 {
        let MotionData::AirCatchHit(state) = &mut self.core.state_data else {
            panic!("the tether's hang countdown outside AirCatchHit")
        };
        &mut state.hang_frames
    }

    /// ftCo_800C3A14 (800C3A14): a ledge in front within a probe 5 higher
    /// and 5 taller than the fighter's (mpColl_80044164 / 800443C4 on a
    /// copy of its CollData). A hit records the ledge's id and grab flag
    /// in the fighter's own CollData and stops it.
    pub fn tether_reaches_ledge(&mut self, map: &mut CollMap) -> bool {
        let mut probe = self.core.collision.data;
        probe.ledge_snap_y = (f64::from(probe.ledge_snap_y) + LEDGE_PROBE_EXTRA) as f32;
        probe.ledge_snap_height = (f64::from(probe.ledge_snap_height) + LEDGE_PROBE_EXTRA) as f32;
        let facing_right = f64::from(self.core.physics.facing) > 0.0;
        let (ledge, flag) = if facing_right {
            (
                map.check_for_left_ledge(&mut probe),
                collide::LEFT_LEDGE_GRAB,
            )
        } else {
            (
                map.check_for_right_ledge(&mut probe),
                collide::RIGHT_LEDGE_GRAB,
            )
        };
        let Some(ledge) = ledge else {
            return false;
        };
        let data = &mut self.core.collision.data;
        if facing_right {
            data.ledge_id_left = ledge;
        } else {
            data.ledge_id_right = ledge;
        }
        data.env_flags |= flag as i32;
        self.core.physics.self_velocity.x = 0.0;
        self.core.physics.self_velocity.y = 0.0;
        true
    }

    /// The end of the tether's climb (it_802B8B54 / it_802A3828): at the
    /// ledge ftCo_800C3A14 found and nobody holds (ft_80082E3C), the ledge
    /// catch (ftCliffCommon_80081370); otherwise the drift stops and the
    /// ledge jump's hop at `hop` of its height (ftCo_8009B390).
    pub fn finish_tether_climb(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        hop: f32,
    ) -> Result<()> {
        if self.tether_reaches_ledge(map) && !self.core.ledge_occupied(map) {
            return self.enter_cliff_catch(assets, map);
        }
        self.core.physics.self_velocity.x = 0.0;
        self.enter_cliff_hop(assets, hop)
    }

    /// ftCo_80090780 (80090780) from a tether's article: the fighter lets
    /// go into DamageFall.
    pub fn let_go_of_tether(&mut self, assets: &FighterAssets) -> Result<()> {
        self.enter_damage_fall(assets)
    }
}
