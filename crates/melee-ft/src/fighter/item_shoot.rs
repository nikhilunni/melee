//! Firing a held shooter (it_8026B30C == 3): ft_0CDD.c's dispatch
//! (ftCo_Attack_800CDD14 on the ground, ftCo_800CDDA0 in the air) and the
//! Ray Gun's states (ft_0CDF.c, LGunShoot and its air and empty rows). The
//! Fire Flower and the Super Scope fail closed at entry.
use super::{
    assets::{FighterAssets, Result},
    Fighter,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, GroundOrAir, ItemKind};

/// ftCo_800CE1D4's efSync_Spawn ids: the Ray Gun's muzzle flash, and the
/// empty gun's puff.
const RAY_GUN_FLASH: u16 = 1073;
const EMPTY_SHOT_PUFF: u16 = 1029;
/// ftCo_800CE1D4's sounds: a shot facing right, facing left, and empty.
const RAY_GUN_SHOT_RIGHT: u32 = 228;
const RAY_GUN_SHOT_LEFT: u32 = 229;
const RAY_GUN_EMPTY: u32 = 230;

/// Whether `kind` is a shooter whose shots are ported (it_8026B30C == 3
/// for the Ray Gun, Fire Flower and Super Scope).
pub fn is_shooter(kind: ItemKind) -> bool {
    matches!(kind, ItemKind::LGun | ItemKind::FFlower | ItemKind::SScope)
}

/// The Ray Gun's four rows (148..=151).
fn is_ray_gun_state(action: u16) -> bool {
    (S::LGunShoot as u16..=S::LGunShootAirEmpty as u16).contains(&action)
}

impl Fighter {
    /// ftCo_Attack_800CDD14 (800CDD14): a grounded shot. An item in reach
    /// is picked up instead (ftpickupitem_80094790); otherwise the held
    /// shooter's state begins.
    pub(super) fn enter_item_shoot(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.try_item_pickup(assets)? {
            return Ok(());
        }
        let held = self.core.held_item.expect("ft_0CDD.c:17: fp->item_gobj");
        match held.kind {
            ItemKind::LGun => self.enter_ray_gun_shoot(GroundOrAir::Ground, assets),
            ItemKind::FFlower => self.enter_flower_shoot(GroundOrAir::Ground, assets),
            ItemKind::SScope => self.enter_scope_start(GroundOrAir::Ground, assets),
            // ft_0CDD.c:27: any other kind does nothing.
            _ => Ok(()),
        }
    }

    /// ftCo_AttackAir_CheckItemThrowInput (8008CD68) once an aerial is
    /// requested: a neutral or forward aerial with a shooter in hand shoots
    /// (ftCo_800CDDA0); anything else is the kind's aerial (decideFighter).
    pub fn enter_aerial_attack(&mut self, assets: &FighterAssets) -> Result<()> {
        let aerial = super::attack::aerial::select(
            &self.core.input,
            &assets.input,
            self.core.physics.facing,
        );
        if matches!(aerial, S::AttackAirN | S::AttackAirF)
            && self.core.held_item.is_some_and(|held| held.use_kind == 3)
        {
            return self.enter_air_item_shoot(assets);
        }
        (self.character.table().enter_aerial)(self, assets)
    }

    /// ftCo_800CDDA0 (800CDDA0): an aerial shot (a neutral or forward
    /// aerial input with a shooter in hand).
    pub(super) fn enter_air_item_shoot(&mut self, assets: &FighterAssets) -> Result<()> {
        let held = self.core.held_item.expect("ft_0CDD.c:40: fp->item_gobj");
        match held.kind {
            ItemKind::LGun => self.enter_ray_gun_shoot(GroundOrAir::Air, assets),
            ItemKind::FFlower => self.enter_flower_shoot(GroundOrAir::Air, assets),
            ItemKind::SScope => self.enter_scope_start(GroundOrAir::Air, assets),
            _ => Ok(()),
        }
    }

    /// ftCo_800CDF5C / ftCo_800CE010 (800CDF5C, 800CE010): throw flags
    /// cleared, LGunShoot (or its empty row) from frame 0, the first step,
    /// the air drift clamped in the air, the shot accessory installed, then
    /// the gun's recoil (it_8028E938).
    fn enter_ray_gun_shoot(&mut self, ground_or_air: GroundOrAir, assets: &FighterAssets) -> Result<()> {
        let held = self.core.held_item.expect("ft_0CDF.c:19: fp->item_gobj");
        self.core.commands.clear_throw_flags();
        // it_8026B594: empty once xD4C reaches zero.
        let empty = held.ammo <= 0;
        let state = match (ground_or_air, empty) {
            (GroundOrAir::Ground, false) => S::LGunShoot,
            (GroundOrAir::Ground, true) => S::LGunShootEmpty,
            (GroundOrAir::Air, false) => S::LGunShootAir,
            (GroundOrAir::Air, true) => S::LGunShootAirEmpty,
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        if ground_or_air == GroundOrAir::Air {
            self.clamp_shot_air_drift();
        }
        self.arm_accessory4();
        self.core.item_requests.push(melee_it::ItemRequest::ControlInstance {
            item: held.item,
            control: melee_it::ItemControl::Recoil,
        });
        Ok(())
    }

    /// ftCommon_ClampAirDrift (8007D468): only X.
    pub(super) fn clamp_shot_air_drift(&mut self) {
        let maximum = self.core.attributes.air.air_drift_max;
        let velocity = &mut self.core.physics.self_velocity;
        velocity.x = velocity.x.clamp(-maximum, maximum);
    }

    /// ftCo_LGunShoot_Anim / ftCo_800CDE18 (800CDE18): at the animation's
    /// end, Wait on the ground or Fall in the air (ftCommon_8007D92C).
    pub(super) fn item_shoot_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            let next = if self.core.physics.ground_or_air == GroundOrAir::Air {
                S::Fall
            } else {
                S::Wait
            };
            self.change_motion_state(next.into(), assets)?;
        }
        Ok(())
    }

    /// ft_800841B8 for a shot on the ground: true when the fighter left the
    /// floor (ftCommon_8007D5D4 done) and the shot goes on in the air.
    pub(super) fn ground_shot_collision(&mut self, map: &mut melee_mp::CollMap) -> Option<bool> {
        use crate::collision::ground::{map_escape, WaitGroundResult};
        if map_escape(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            self.leave_ground();
            return Some(true);
        }
        None
    }

    /// ft_80082C74 for a shot in the air: false when it landed
    /// (ftCommon_8007D7FC done) and the shot goes on on the ground.
    pub(super) fn air_shot_collision(&mut self, map: &mut melee_mp::CollMap) -> Option<bool> {
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        if crate::collision::air::collide_air_dodge(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
        ) {
            self.land();
            return Some(false);
        }
        None
    }

    /// ftCo_LGunShoot_Coll / ftCo_LGunShootAir_Coll (ft_800841B8 with
    /// ftCo_800CE14C, ft_80082C74 with ftCo_800CE0CC): the other row, full
    /// or empty alike, at the same frame and rate (flags 0x0C4C5080), the
    /// drift clamped in the air, the accessory installed again.
    pub(super) fn ray_gun_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        let switched = if self.core.physics.ground_or_air == GroundOrAir::Air {
            self.air_shot_collision(map)
        } else {
            self.ground_shot_collision(map)
        };
        let Some(to_air) = switched else {
            return Ok(());
        };
        let next = match (to_air, self.core.motion_state.id) {
            (true, S::LGunShoot) => S::LGunShootAir,
            (true, _) => S::LGunShootAirEmpty,
            (false, S::LGunShootAir) => S::LGunShoot,
            (false, _) => S::LGunShootEmpty,
        };
        let rate = self.core.animation.speed;
        self.change_ground_air_motion_at_rate(next.into(), assets, rate)?;
        if to_air {
            self.clamp_shot_air_drift();
        }
        self.arm_accessory4();
        Ok(())
    }

    /// ftCo_800CE1D4 (800CE1D4), the Ray Gun's accessory4: on throw flag
    /// b0, a ray from the muzzle (it_8028E79C: one shot fewer, then
    /// it_80298168) with its flash and sound, or the empty gun's puff.
    /// Returns whether it owns accessory4 this tick.
    pub fn shoot_accessory(&mut self) -> bool {
        use melee_it::{ItemControl, ItemRequest, SpawnItem};
        if !is_ray_gun_state(self.core.motion_state.action.0) || !self.core.accessory4_armed {
            return false;
        }
        let Some(mut held) = self.core.held_item else {
            return true;
        };
        // ftCheckThrowB0.
        if !std::mem::take(&mut self.core.commands.throw_accessory) {
            return true;
        }
        assert_eq!(held.kind, ItemKind::LGun, "ft_0CDF.c: It_Kind_L_Gun");
        let muzzle = self.shot_origin(held.shot_offset);
        let facing = self.core.physics.facing;
        let empty = held.ammo <= 0;
        let (sound, effect) = if empty {
            (
                RAY_GUN_EMPTY,
                melee_ef::request::EffectRequest::PositionalGenerator {
                    id: EMPTY_SHOT_PUFF,
                    position: muzzle,
                },
            )
        } else {
            // it_8028E79C: one shot fewer, then it_80298168's ray from the
            // muzzle (z 0) toward the holder's ECB centre (it_8026BB68).
            self.core.item_requests.push(ItemRequest::ControlInstance {
                item: held.item,
                control: ItemControl::Fire,
            });
            held.ammo -= 1;
            self.core.held_item = Some(held);
            let c = &self.core;
            let mut spawn = SpawnItem::ray(
                ItemKind::LGunRay,
                c.player.id,
                Vec3::new(muzzle.x, muzzle.y, 0.0),
                facing,
            );
            // ftLib_80086990: the ECB's midpoint (fadds, fmuls, fadds).
            let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
            spawn.position = Vec3::new(
                c.physics.position.x + 0.0,
                c.physics.position.y + midpoint,
                c.physics.position.z + 0.0,
            );
            self.core.item_requests.push(ItemRequest::Spawn(spawn));
            (
                if facing == 1.0 {
                    RAY_GUN_SHOT_RIGHT
                } else {
                    RAY_GUN_SHOT_LEFT
                },
                melee_ef::request::EffectRequest::PositionalModel {
                    id: RAY_GUN_FLASH,
                    position: Vec3::new(muzzle.x, muzzle.y, 0.0),
                },
            )
        };
        self.core.effects.push(effect);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: sound,
                volume: 127,
                pan: 64,
            });
        true
    }

    /// ftCo_800CDE94 (800CDE94): a point in the hand joint's frame (ftData
    /// x8 +0x10) in the world, the offset first divided by x34_scale.y when
    /// it is not 1.
    pub(super) fn shot_origin(&mut self, offset: Vec3) -> Vec3 {
        let mut offset = offset;
        if let Some(scale) = self.core.commands.model_scale.filter(|&s| s != 1.0) {
            let inverse = 1.0 / scale;
            offset = Vec3::new(offset.x * inverse, offset.y * inverse, offset.z * inverse);
        }
        let part = usize::from(self.core.bones.model.animation_translation);
        super::caches::part_position(
            &mut self.core.skeleton,
            &self.core.animation,
            part,
            offset,
        )
    }
}
