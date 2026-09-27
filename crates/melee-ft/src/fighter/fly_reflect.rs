//! FlyReflectWall / FlyReflectCeil (ftCo_FlyReflect.c): a tumbling fighter
//! driven into a wall or ceiling faster than PlCo +1B0 bounces off it,
//! mirrored about the surface and slowed, once per surface in a row.
use super::{
    assets::{FighterAssets, Result},
    state::CollisionPhase,
    Fighter, MotionData,
};
use gekko_math::{fma::fmadds, msl::fctiwz};
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::{mp::collide, CommonMotionState as S};

/// ftCo_Surface: mv.damage.x19, the surface of the last bounce, so a
/// fighter pressed into one surface bounces off it only once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BounceSurface {
    LeftWall,
    RightWall,
    Ceiling,
}

/// PlCo +1B8/+1BC/+1C0: the bounce's intangibility, damping and the frames
/// before the next bounce or wall tech may start.
#[derive(Clone, Copy, Debug)]
pub struct BounceParameters {
    pub intangible_frames: i32,
    pub damping: f32,
    pub lock_frames: f32,
}
impl BounceParameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            intangible_frames: r.s32(base + 0x1B8)?,
            damping: r.f32(base + 0x1BC)?,
            lock_frames: r.f32(base + 0x1C0)?,
        })
    }
}

impl Fighter {
    fn damage_scratch(&mut self) -> &mut super::damage::DamageState {
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("damage scratch missing")
        };
        damage
    }

    /// ftCo_800C15F4 (800C15F4): knockback driving into a wall bounces off
    /// it, unless that wall was the last bounce.
    pub(super) fn try_wall_bounce(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<bool> {
        let cd = &self.core.collision.data;
        let env = cd.env_flags as u32;
        let speed = assets.damage.fly_reflect_speed;
        let knockback = self.core.physics.knockback_velocity.x;
        let last = self.damage_scratch().last_bounce;
        let cd = &self.core.collision.data;
        let (corner, normal, surface) = if knockback < -speed
            && env & collide::RIGHT_WALL_HUG != 0
            && last != Some(BounceSurface::LeftWall)
        {
            (
                cd.ecb.left,
                cd.right_facing_wall.normal,
                BounceSurface::LeftWall,
            )
        } else if knockback > speed
            && env & collide::LEFT_WALL_HUG != 0
            && last != Some(BounceSurface::RightWall)
        {
            (
                cd.ecb.right,
                cd.left_facing_wall.normal,
                BounceSurface::RightWall,
            )
        } else {
            return Ok(false);
        };
        let offset = Vec3::new(corner.x, corner.y, 0.0);
        self.enter_fly_reflect(normal, offset, assets, map)?;
        self.damage_scratch().last_bounce = Some(surface);
        Ok(true)
    }

    /// ftCo_800C1718 (800C1718): the ceiling bounce, which is not ported;
    /// reaching it fails closed.
    pub(super) fn check_ceiling_bounce(&mut self, assets: &FighterAssets) {
        let env = self.core.collision.data.env_flags as u32;
        if self.core.physics.knockback_velocity.y > assets.damage.fly_reflect_speed
            && env & collide::CEILING_HUG != 0
            && self.damage_scratch().last_bounce != Some(BounceSurface::Ceiling)
        {
            unimplemented!("ftCo_800C1718: FlyReflectCeil (ft_80082084's ceiling pass)");
        }
    }

    /// ftCo_800C18A8 (800C18A8) for the wall: spark and small quake at the
    /// contact, velocity plus knockback mirrored about the wall and damped
    /// into knockback, then FlyReflectWall facing away from it.
    fn enter_fly_reflect(
        &mut self,
        normal: Vec3,
        offset: Vec3,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<()> {
        let p = self.core.physics.position;
        let contact = Vec3::new(p.x + offset.x, p.y + offset.y, p.z + offset.z);
        let angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::SurfaceRebound {
                position: contact,
                angle,
            });
        self.core.quake_request = Some(melee_cm::QuakeKind::Small);
        // lbVector_Add_xy, then lbVector_Mirror (8000DC6C): fmuls, fmadds,
        // fmuls by -2, then two fmadds.
        let v = self.core.physics.self_velocity;
        let kb = self.core.physics.knockback_velocity;
        let (x, y) = (v.x + kb.x, v.y + kb.y);
        let reflect = -2.0 * fmadds(normal.x, x, normal.y * y);
        let bounce = &assets.fly_reflect;
        // retail 800C1980 / 800C198C: fmuls by PlCo +1BC.
        self.core.physics.knockback_velocity = Vec3::new(
            fmadds(normal.x, reflect, x) * bounce.damping,
            fmadds(normal.y, reflect, y) * bounce.damping,
            v.z,
        );
        self.core.physics.self_velocity = Vec3::ZERO;
        self.core.physics.facing = if self.core.physics.knockback_velocity.x < 0.0 {
            -1.0
        } else {
            1.0
        };
        self.change_fly_reflect_motion(S::FlyReflectWall, assets)?;
        let trans_z = self
            .core
            .animation
            .root_motion
            .as_ref()
            .expect("FlyReflectWall TransN")
            .primary_history
            .position
            .z;
        // retail 800C1A1C / 800C1A20: fadds, then fnmsubs with the negated facing.
        self.core.physics.position.x =
            gekko_math::fma::fnmsubs(trans_z, -self.core.physics.facing, p.x + offset.x);
        self.wall_contact_map(assets, map);
        self.damage_scratch().bounce_lock = fctiwz(bounce.lock_frames) as u8;
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 7,
                duration: 0,
            });
        self.core.status.ledge_intangibility = self
            .core
            .status
            .ledge_intangibility
            .max(bounce.intangible_frames);
        // ftCo_80097630's impact sound is not modelled.
        Ok(())
    }
}

/// ftCo_FlyReflect_Coll (800C1B7C) for the wall: a landing techs or bounces
/// on the floor (ftCo_80090184); the ceiling is fail-closed; once the lock
/// runs out another wall tech or bounce may follow.
pub fn collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let assets = phase.assets.expect("FlyReflect collision assets");
    assert_eq!(
        fighter.core.motion_state.id,
        S::FlyReflectWall,
        "ftCo_FlyReflect_Coll: FlyReflectCeil (ft_80082084)"
    );
    let core = &mut fighter.core;
    crate::collision::air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    if fighter.wall_contact_map(assets, phase.map) {
        return fighter.tumble_landing(assets);
    }
    let env = fighter.core.collision.data.env_flags as u32;
    if env & collide::CEILING_HUG != 0 && fighter.core.tech_window_open(assets) {
        unimplemented!("ftCo_800C23A0: a ceiling tech");
    }
    fighter.check_ceiling_bounce(assets);
    if fighter.damage_scratch().bounce_lock == 0 {
        if fighter.try_wall_tech(assets, phase.map)? {
            return Ok(());
        }
        if fighter.try_wall_bounce(assets, phase.map)? {
            return Ok(());
        }
        fighter.check_ceiling_bounce(assets);
    }
    Ok(())
}

/// ftCo_FlyReflect_Anim (800C1B20): the lock counts down, then DamageFly's
/// animation.
pub fn animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    let damage = fighter.damage_scratch();
    damage.bounce_lock = damage.bounce_lock.saturating_sub(1);
    super::state::callbacks::animation::damage(fighter, phase)
}
