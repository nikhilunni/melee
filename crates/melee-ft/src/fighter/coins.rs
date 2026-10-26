//! A Coin match's fighter side (gm_8016B0B4): coins spilled by every hit
//! (ftCommon_8007EA90), the fall's payout request (ftCo_800D4E50) and the
//! two pickup spheres coins are taken with (ft_8007C630 / ft_8007C6DC).
//! A hit's spill draws here, where retail draws it; the scene makes the
//! coins (it_802E609C), pays a fall out and keeps the purses.
use super::{assets::FighterAssets, FighterCore};
use hsd_types::Vec3;
use melee_it::coin::{CoinAttributes, PlannedCoin, Spill, MAX_SPILL};
use melee_types::fixed::FixedVec;

/// What a Coin match gives each fighter: ItCo's coin attributes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoinRules {
    pub attributes: CoinAttributes,
}

/// PlCo's spill parameters and ftData x38's pickup spheres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoinAssets {
    /// +5C0: the launch speed of a hit without knockback.
    pub unlaunched_speed: f32,
    /// +5C8: the most coins a fall spills.
    pub fall_limit: i32,
    /// +5CC: a fall's launch speed.
    pub fall_speed: f32,
    pub spheres: [CoinSphereData; 2],
}

impl CoinAssets {
    pub(super) fn read(
        data: &hsd_archive::Archive,
        fighter_data: u32,
        common: &hsd_archive::Archive,
        common_data: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let c = common.reader();
        let mut spheres = [CoinSphereData::default(); 2];
        if let Some(table) = data.link(fighter_data + 0x38)? {
            let r = data.reader();
            for (index, sphere) in spheres.iter_mut().enumerate() {
                // struct ftData_x38 (ft/types.h:646): part, offset, radius.
                let at = table + index as u32 * 0x14;
                *sphere = CoinSphereData {
                    part: r.s32(at)? as usize,
                    offset: Vec3::new(r.f32(at + 4)?, r.f32(at + 8)?, r.f32(at + 0xC)?),
                    radius: r.f32(at + 0x10)?,
                };
            }
        }
        Ok(Self {
            unlaunched_speed: c.f32(common_data + 0x5C0)?,
            fall_limit: c.s32(common_data + 0x5C8)?,
            fall_speed: c.f32(common_data + 0x5CC)?,
            spheres,
        })
    }
}

/// ftData x38: one pickup sphere's part, offset and radius.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoinSphereData {
    pub part: usize,
    pub offset: Vec3,
    pub radius: f32,
}

/// Fighter.x1614: a pickup sphere, swept from last frame's centre.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoinSphere {
    /// x0: ftData's radius.
    pub radius: f32,
    /// x4 (the part's JObj) and ftData's offset from it.
    pub part: usize,
    pub offset: Vec3,
    /// x8 / x14: the centre now and last frame.
    pub center: Vec3,
    pub previous_center: Vec3,
}

/// One it_802E5F8C a hit made: where, how and which coins, and the player
/// its worth is credited to (Player_SetUnk98).
#[derive(Clone, Debug, PartialEq)]
pub struct CoinSpill {
    pub position: Vec3,
    pub spill: Spill,
    pub striker: Option<u8>,
    /// it_802E609C's return: the spill's worth.
    pub total: i32,
    pub coins: FixedVec<PlannedCoin, MAX_SPILL>,
}

/// ftCo_800D4E50's arguments: the scene halves the purse and spills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallPayout {
    pub position: Vec3,
    /// arg2: fighters can take the coins (not a star or screen KO's).
    pub collectable: bool,
    pub angle: f32,
}

/// A launched hit's part of ftCommon_8007EA90: ftCo_Damage_CalcAngle of
/// its knockback, dmg.facing_dir_1 and dmg.x1854_collpos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoinLaunch {
    pub angle: f32,
    pub knockback: f32,
    pub facing: f32,
    pub position: Vec3,
}

/// "@961": a hit without knockback spills straight up.
const UPWARD: f32 = 1.570_796_4;

impl FighterCore {
    /// ft_8007C630 (8007C630, Fighter_UnkProcessDeath at every spawn): each
    /// sphere at its part and offset (lb_8000B1CC), flat in z, with no
    /// sweep yet. Retail places them in every match; only a Coin match
    /// reads them.
    pub fn place_coin_spheres(&mut self, assets: &FighterAssets) {
        if self.coin_rules.is_none() {
            return;
        }
        for (sphere, data) in self.coin_spheres.iter_mut().zip(&assets.coins.spheres) {
            sphere.part = data.part;
            sphere.offset = data.offset;
            sphere.radius = data.radius;
            let mut center = super::caches::part_position(
                &mut self.skeleton,
                &self.animation,
                data.part,
                data.offset,
            );
            center.z = 0.0;
            sphere.center = center;
            sphere.previous_center = center;
        }
    }

    /// ft_8007C6DC (8007C6DC), Fighter_8006C80C in a Coin match: last
    /// frame's centre kept, the new one at the part.
    pub(crate) fn follow_coin_spheres(&mut self) {
        if self.coin_rules.is_none() {
            return;
        }
        for sphere in &mut self.coin_spheres {
            sphere.previous_center = sphere.center;
            let mut center = super::caches::part_position(
                &mut self.skeleton,
                &self.animation,
                sphere.part,
                sphere.offset,
            );
            center.z = 0.0;
            sphere.center = center;
        }
    }

    /// ftCommon_8007EA90 (8007EA90), after Fighter_TakeDamage_8006CC7C in a
    /// Coin match (Fighter_UnkTakeDamage_8006CC30 converts the damage with
    /// fctiwz): that many coins' worth spill from where the hit landed,
    /// along its launch (mirrored for a hit from the left), or, without
    /// knockback, upward from the camera bone at PlCo +5C0.
    pub(crate) fn spill_coins_for_hit(
        &mut self,
        damage: f32,
        launch: Option<CoinLaunch>,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        let Some(rules) = self.coin_rules else {
            return;
        };
        let count = gekko_math::msl::fctiwz(damage);
        let (angle, speed, position) = match launch {
            Some(launch) => {
                let mut angle = launch.angle;
                // 8007EAE8..8007EB10: x1844 > 0 mirrors the angle, in
                // double ("@959" pi, "@960" 3pi), then frsp.
                if launch.facing > 0.0 {
                    angle = if f64::from(angle) > core::f64::consts::PI {
                        (3.0 * core::f64::consts::PI - f64::from(angle)) as f32
                    } else {
                        (core::f64::consts::PI - f64::from(angle)) as f32
                    };
                }
                let at = Vec3::new(launch.position.x, launch.position.y, 0.0);
                (angle, launch.knockback, at)
            }
            None => (UPWARD, assets.coins.unlaunched_speed, self.camera_bone_position()),
        };
        let spill = melee_it::coin::spill(angle, speed, true);
        let mut coins = FixedVec::default();
        let total = melee_it::coin::plan_spill(
            &rules.attributes,
            count,
            usize::from(self.standing_rank),
            spill,
            rng,
            &mut coins,
        );
        // fp->dmg.x18c4_source_ply (6: nobody).
        let striker = self.combat.ko_source.player;
        self.coin_spills.push(CoinSpill {
            position,
            spill,
            striker,
            total,
            coins,
        });
    }

    /// ftCommon_8007EA90's launch for the hit ProcessHit applies:
    /// ftCo_Damage_CalcAngle of dmg.kb_applied as the fighter stands now,
    /// dmg.facing_dir_1 and dmg.x1854_collpos.
    pub(crate) fn coin_launch(
        &self,
        hit: &melee_coll::damage::ReceivedHit,
        assets: &FighterAssets,
    ) -> CoinLaunch {
        let angle = assets.damage.launch_angle(
            self.combat
                .launch_angle_override
                .unwrap_or(i32::from(hit.descriptor.angle)),
            hit.knockback,
            self.physics.ground_or_air,
        );
        CoinLaunch {
            angle,
            knockback: hit.knockback,
            facing: hit.facing,
            position: self.combat.hit_position,
        }
    }

    /// A Fighter_UnkTakeDamage_8006CC30 whose ftCommon_8007EA90 has no
    /// witness yet (`site` names its C line): a Coin match stops there.
    pub(crate) fn refuse_unwitnessed_coin_spill(&self, site: &str) {
        if self.coin_rules.is_some() {
            unimplemented!("ftCommon_8007EA90: a Coin match's spill from {site}");
        }
    }

    /// ftCo_800D4E50 (800D4E50), from a death's entry: the scene halves the
    /// purse of a player's own fighter (not x221F_b4) and spills from
    /// `position` at `angle`.
    pub(crate) fn request_fall_payout(&mut self, position: Vec3, collectable: bool, angle: f32) {
        if self.coin_rules.is_none() || self.player.secondary {
            return;
        }
        assert!(self.fall_payout.is_none(), "ftCo_800D4E50 twice in one proc");
        self.fall_payout = Some(FallPayout {
            position,
            collectable,
            angle,
        });
    }
}
