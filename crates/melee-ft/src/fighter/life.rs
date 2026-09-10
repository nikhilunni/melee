//! Stock loss and revival, ft_0D31.c / ft_0D4D.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub camera_top: f32,
    pub revival_positions: [Vec3; 4],
    /// grLast stage initialization (801DFF18): stage_info.unk8C.b4.
    pub player_revival_markers: bool,
}
#[derive(Clone, Debug)]
pub enum LifeState {
    Dead { remaining: i32 },
    AwaitingRespawn,
    Revival { remaining: i32, target: Vec3 },
    PlatformWait { remaining: i32, target: Vec3 },
}
/// Fighter accessory JObj, loaded from PlCo ftLoadCommonData[8].
#[derive(Clone, Debug)]
pub struct RevivalPlatform {
    pub tree: hsd_anim::jobj::JObjTree,
    pub root: hsd_anim::jobj::JObjId,
}
pub struct LifeParameters {
    pub death_delay: i32,
    pub revival_duration: i32,
    pub platform_duration: i32,
    pub invincibility_duration: i32,
    pub death_effect_scale: f32,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// gm_8016719C -> Player_80032070 -> Fighter_UnkProcessDeath (80068354).
    #[allow(clippy::too_many_arguments)]
    pub fn reset_for_revival(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        archive: &hsd_archive::Archive,
        skeleton: hsd_anim::jobj::JObjTree,
        root: hsd_anim::jobj::JObjId,
        context: super::SpawnContext<'_>,
    ) -> Result<()> {
        let mut player = self.core.player.clone();
        if !arena.player_revival_markers {
            unimplemented!("gm_80167638: shared revival marker offset allocation");
        }
        let target = arena.revival_positions[usize::from(player.id)];
        player.position = Vec3::new(target.x, arena.camera_top, 0.0);
        player.facing = if player.position.x >= 0.0 { -1.0 } else { 1.0 };
        player.damage = 0.0;
        let character = C::from_archive(archive)?;
        let mut next = Self::spawn(player, character, assets, skeleton, root, context)?;
        next.enter_revival(assets, target)?;
        *self = next;
        Ok(())
    }
    /// ftCo_800D3158 / ftCo_800D3BC8 (800D3158 / 800D3BC8), after Update.
    pub fn check_blast_zone(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        if matches!(self.core.state_data, MotionData::Life(_))
            || self.core.status.disabled
            || self.core.status.ledge_grab_disabled
        {
            return Ok(());
        }
        let p = self.core.physics.position;
        if p.x < arena.left || p.x > arena.right || p.y > arena.top {
            unimplemented!("ftCo_800D3158: side/up death");
        }
        if p.y >= arena.bottom {
            return Ok(());
        }
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800D331C: release linked fighter on death");
        }
        // ftCommon_8007E2FC (8007E2FC): death clears every velocity owner.
        self.core.physics.self_velocity = Vec3::ZERO;
        self.core.physics.animation_velocity = Vec3::ZERO;
        self.core.physics.knockback_velocity = Vec3::ZERO;
        self.core.physics.shield_knockback_velocity = Vec3::ZERO;
        self.core.physics.ground_velocity = 0.0;
        self.core.physics.ground_knockback_velocity = 0.0;
        self.core.physics.ground_shield_knockback_velocity = 0.0;
        self.change_motion_state(S::DeadDown, assets)?;
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.death_delay,
        });
        self.core.player.stocks = self.core.player.stocks.saturating_sub(1);
        self.core.effect_state.invisible = true;
        self.core
            .effects
            .push(super::effects::EffectRequest::Death {
                position: p,
                scale: assets.life.death_effect_scale,
            });
        Ok(())
    }
    /// ftCo_800D4FF4 (800D4FF4), after Fighter_UnkProcessDeath reset.
    pub fn enter_revival(&mut self, assets: &FighterAssets, target: Vec3) -> Result<()> {
        self.leave_ground();
        self.change_motion_state(S::Rebirth, assets)?;
        self.core.state_data = MotionData::Life(LifeState::Revival {
            remaining: assets.life.revival_duration,
            target,
        });
        self.core.status.input_frozen = false;
        self.core.status.ignore_fighter_nudge = true;
        self.core.commands.hurt_status = super::escape::HurtStatus::Intangible;
        let mut platform = assets.revival_platform.clone();
        // ftCoD4FF4 (800D51C0): separate model-scale product, no FMA.
        let scale = self.core.player.scale
            * self.core.attributes.size.model_scaling
            * self.core.attributes.size.respawn_platform_scale;
        platform
            .tree
            .set_scale(platform.root, &Vec3::new(scale, scale, scale));
        platform
            .tree
            .set_translate(platform.root, &self.core.physics.position);
        self.core.revival_platform = Some(platform);
        Ok(())
    }
    /// Rebirth_Anim (800D52F8), RebirthWait_Anim (800D56EC).
    pub(super) fn revival_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        match &mut self.core.state_data {
            MotionData::Life(LifeState::Revival { remaining, target }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    let target = *target;
                    melee_mp::set_position(
                        &mut self.core.collision.data,
                        &self.core.physics.position,
                    );
                    self.core.physics.self_velocity.y = 0.0;
                    self.change_motion_state(S::RebirthWait, assets)?;
                    self.core.state_data = MotionData::Life(LifeState::PlatformWait {
                        remaining: assets.life.platform_duration,
                        target,
                    });
                    self.core.status.ignore_fighter_nudge = true;
                    self.core.commands.hurt_status = super::escape::HurtStatus::Intangible;
                }
            }
            MotionData::Life(LifeState::PlatformWait { remaining, .. }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    unimplemented!(
                        "ftCo_RebirthWait_Anim: platform timeout -> Fall with invincibility"
                    );
                }
            }
            _ => panic!("revival scratch"),
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_DeadDown_Anim (800D3E00): GM respawn is performed at this callback boundary.
    pub(super) fn death_animation(&mut self) {
        let MotionData::Life(LifeState::Dead { remaining }) = &mut self.state_data else {
            panic!("death scratch");
        };
        *remaining -= 1;
        if *remaining == 0 {
            if self.player.stocks == 0 {
                unimplemented!("gm_80167320: final stock / elimination");
            }
            self.state_data = MotionData::Life(LifeState::AwaitingRespawn);
        }
    }
    /// fn_800D54A4 and Fighter_8006C80C: place then animate the accessory.
    pub fn update_revival_platform(&mut self) {
        if let Some(platform) = &mut self.revival_platform {
            platform
                .tree
                .set_translate(platform.root, &self.physics.position);
            platform.tree.anim_all::<super::RetailTrig>(platform.root);
            assert!(
                platform.tree.events.is_empty(),
                "revival accessory event routing"
            );
        }
    }
    /// Rebirth_Phys (800D535C) / RebirthWait_Phys (800D58F4).
    pub(super) fn revival_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        let (remaining, target) = match self.state_data {
            MotionData::Life(
                LifeState::Revival { remaining, target }
                | LifeState::PlatformWait { remaining, target },
            ) => (remaining, target),
            _ => panic!("revival physics scratch"),
        };
        // This scene's static marker has zero player offset. The retail FMA
        // at 800D53BC / 800D5954 is the moving-marker offset path.
        let inv = 1.0 / remaining as f32;
        self.physics.self_velocity.x = (target.x - self.physics.position.x) * inv;
        self.physics.self_velocity.y = (target.y - self.physics.position.y) * inv;
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
}
