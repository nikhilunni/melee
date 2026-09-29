//! A player's two fighters (Popo and Nana): the leader (x2222_b5) and the
//! second fighter (x221F_b4) share one player's stocks, spawn point and
//! stale table, and die and revive together (ftCo_800BFD9C,
//! Player_80032070). The scene holds both; a fighter sees its partner
//! through [`PartnerView`], which the scene refreshes before each proc.
use super::{
    assets::{FighterAssets, Result},
    life::LifeState,
    Fighter, FighterCore, MotionData,
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;

/// What a fighter's revival states read of the player's other fighter
/// (Player_GetEntityAtIndex(player, 0 or 1)).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PartnerView {
    /// self_vel, which the second fighter copies while reviving
    /// (ftCo_Rebirth_Phys, ftCo_RebirthWait_Phys).
    pub self_velocity: Vec3,
    /// cur_pos.y, which the revival accessories match (fn_800D54A4,
    /// fn_800D55B4).
    pub height: f32,
    /// x221F_b3: asleep (Sleep, out of play).
    pub asleep: bool,
    /// ftLib_800873CC: in Rebirth or RebirthWait.
    pub reviving: bool,
}
impl PartnerView {
    pub fn of(partner: &FighterCore) -> Self {
        Self {
            self_velocity: partner.physics.self_velocity,
            height: partner.physics.position.y,
            asleep: partner.status.disabled,
            reviving: partner.reviving(),
        }
    }
}

impl FighterCore {
    /// ftLib_800873CC (800873CC): Rebirth or RebirthWait.
    pub fn reviving(&self) -> bool {
        matches!(
            self.state_data,
            MotionData::Life(LifeState::Revival { .. } | LifeState::PlatformWait { .. })
        )
    }

    /// fn_800D54A4 / fn_800D55B4 (800D54A4 / 800D55B4), the revival
    /// accessory's first half: the leader rises to an awake partner above
    /// it; the second fighter rises to its leader.
    pub fn match_partner_height(&mut self) {
        let Some(partner) = self.partner else {
            return;
        };
        if self.player.secondary || !partner.asleep {
            // 800D54E4: fcmpo, ble.
            if partner.height > self.physics.position.y {
                self.physics.position.y = partner.height;
            }
        }
    }
}

impl Fighter {
    /// ftCo_800D4F24(gobj, 1) (800D4F24) for the second fighter when its
    /// leader's death ends (ftCo_800BFD9C): the vanish puff (efSync 0x43F at
    /// cur_pos, attribute +168 times x34_scale.y), the death releases
    /// (ftCo_800D331C) and Sleep (ftCo_800BFD04). The caller has checked
    /// x221F_b3.
    pub fn vanish_with_leader(&mut self, assets: &FighterAssets) -> Result<()> {
        // 800D4F68: fmuls.
        let scale = self.core.attributes.size.unknown_168 * self.core.player.scale;
        self.core.effects.push(EffectRequest::PartnerVanish {
            position: self.core.physics.position,
            scale,
        });
        self.release_for_death(assets);
        self.enter_sleep(assets)
    }

    /// Player_80032070's second fighter: ftCo_800D4FF4 at the platform the
    /// player's spawn (fn_8016719C) placed, with the player's facing; the
    /// fighter takes Player_GetDamage, the leader's HP.
    pub fn revive_beside_leader(
        &mut self,
        assets: &FighterAssets,
        platform: Vec3,
        leader: &super::PlayerSlot,
        context: super::SpawnContext<'_>,
    ) -> Result<()> {
        self.core.player.position = leader.position;
        self.core.player.facing = leader.facing;
        self.core.player.damage = leader.damage;
        self.revive_at(assets, platform, context)
    }
}
