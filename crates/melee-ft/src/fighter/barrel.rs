//! Kongo Jungle's barrel cannons, fighter side: ftCo_BarrelWait.c
//! (8009EB18..8009EC70) and the catch device's test (ftColl_8007BA0C,
//! ftCo_800C0A28).
//!
//! The stage's catch callback decides whether its barrel takes the fighter;
//! the fighter then waits inside, invisible and intangible, placed on the
//! cannon by the scene each tick (accessory1 ftCo_8009EC44), until the
//! barrel fires it (ftCo_8009EC70) or A/B asks to fire early
//! (Ground_801C4DD0). A fired fighter cannot be caught again for PlCo +5E0
//! frames.
use super::{
    assets::{FighterAssets, Result},
    ledge::GrabExclusions,
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter, MotionData,
};
use crate::{anim::WaitChoice, input::Buttons};
use hsd_types::{Vec2, Vec3};
use melee_types::{combat::HurtStatus, CommonMotionState as S, HitElement};

/// PlCo +4DC, +4E4, +5E0.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +4DC (Vec2): the camera box's horizontal reach while inside.
    pub camera_horizontal: Vec2,
    /// +4E4 (Vec3): its top, bottom and radius.
    pub camera_vertical: Vec3,
    /// +5E0 (bury_timer_unk2): frames after a shot before another catch.
    pub catch_cooldown: u32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            camera_horizontal: Vec2::new(r.f32(base + 0x4DC)?, r.f32(base + 0x4E0)?),
            camera_vertical: Vec3::new(
                r.f32(base + 0x4E4)?,
                r.f32(base + 0x4E8)?,
                r.f32(base + 0x4EC)?,
            ),
            catch_cooldown: r.u32(base + 0x5E0)?,
        })
    }
}

/// The hit a barrel fires with (lbColl_80008D30's argument record).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarrelShot {
    pub damage: i32,
    pub set_knockback: i32,
    pub growth: i32,
    pub base_knockback: i32,
    pub element: i32,
    /// The aim, in degrees.
    pub angle: f32,
}

/// x1A6A's category 5 (the ledge options') keeps a fighter out of barrels.
const LEDGE_OPTION: u16 = GrabExclusions::LEDGE_OPTION.0;

impl Fighter {
    /// ftColl_8007BA0C's x2219_b1 guard and ftCo_800C0A28 (BuryType_Unk2):
    /// whether a barrel may test this fighter.
    pub fn barrel_catchable(&self) -> bool {
        !self.core.out_of_play()
            && self.core.status.barrel_cooldown == 0
            && self.core.motion_state.id != S::BarrelWait
            && self.core.status.grab_exclusions.0 & LEDGE_OPTION == 0
    }

    /// ftCo_8009EB18 (8009EB18): into the barrel.
    pub fn enter_barrel(&mut self, assets: &FighterAssets) -> Result<()> {
        // ftCommon_8007DB58.
        self.interrupt_actions(assets);
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C (ftCo_BarrelWait.c:37): caught holding a heavy item");
        }
        // ftCo_800DD168: the catch device skips a grabbed or grabbing fighter
        // only through its states' grab exclusions.
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800DD168 (ftCo_BarrelWait.c:38): caught in a grab pair");
        }
        self.change_motion_state(S::BarrelWait.into(), assets)?;
        self.step_animation(assets);
        // ftCommon_8007E2FC.
        self.core.clear_movement();
        // x1988 = 2 written directly: no colour animation.
        self.core.commands.hurt_status = HurtStatus::Intangible;
        self.core.effect_state.invisible = true;
        self.core.status.grab_exclusions = GrabExclusions::ALL;
        let p = &assets.barrel;
        let extents = &mut self.core.camera.target_extents;
        extents.left = p.camera_horizontal.x;
        extents.right = p.camera_horizontal.y;
        extents.top = p.camera_vertical.x;
        extents.bottom = p.camera_vertical.y;
        extents.radius = p.camera_vertical.z;
        // ftCommon_8007EFC0(fp, 1).
        self.core.status.name_tag_timer = 1;
        self.core.state_data = MotionData::Barrel;
        Ok(())
    }

    /// ftCo_8009EC44 (8009EC44), BarrelWait's accessory1: the fighter sits on
    /// the cannon (Ground_801C4DA0).
    pub fn ride_barrel(&mut self, cannon: Vec3) {
        if self.core.motion_state.id == S::BarrelWait {
            self.core.physics.position = cannon;
        }
    }

    /// ftCo_8009EC70 (8009EC70): fired from `position` along `shot.angle`.
    pub fn fire_from_barrel(
        &mut self,
        position: Vec3,
        shot: BarrelShot,
        assets: &FighterAssets,
        stage_camera: &melee_cm::StageCamera,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        self.core.status.barrel_cooldown = assets.barrel.catch_cooldown;
        self.core.physics.position = position;
        // mpColl_80043680: the collision positions start at the cannon.
        melee_mp::set_position(&mut self.core.collision.data, &position);
        // ftCamera_80076064.
        self.core.reset_camera_subject(assets, stage_camera);
        // ftColl_800788D4 (0x800788D4): a sourceless hit.
        self.core
            .combat
            .ko_source
            .record(melee_coll::damage_log::HitCredit::Nobody);
        // ftCo_Barrel_ApplyKnockback (ftCommon/inlines.h:40): the aim turns
        // to the fighter's facing, then truncates into x1848_kb_angle.
        let mut angle = shot.angle;
        if angle < 0.0 {
            angle += 360.0;
        }
        let angle = if self.core.physics.facing < 0.0 {
            angle
        } else {
            180.0 - angle
        };
        let element = HitElement::try_from(shot.element).expect("barrel shot element");
        let descriptor = melee_types::combat::HitboxDescriptor {
            group: 0,
            bone: 0,
            common_bone: false,
            requires_throw_owner: false,
            damage: shot.damage as f32,
            shield_damage: 0,
            sound_severity: 0,
            radius: 0.0,
            offset: Vec3::ZERO,
            // The launch reads x1848_kb_angle (launch_angle_override) instead.
            angle: 0,
            growth: shot.growth as u16,
            weight_knockback: shot.set_knockback as u16,
            base_knockback: shot.base_knockback as u16,
            element,
            hit_ground: true,
            hit_air: true,
            ignore_scale: false,
            clank: false,
            rebound: false,
        };
        // ftColl_80079EA8 (80079EA8): ftColl_80079AB0's formula on the
        // fighter's own percent, this frame's logged damage and the
        // record's integer damage (lbColl_80008D30's unk_count).
        let knockback = assets.damage.knockback_for_frame(
            &descriptor,
            self.core.knockback_percent(&assets.damage),
            self.core.combat.frame_damage,
            self.core.attributes.size.weight,
            shot.damage as u32,
            melee_coll::damage::KnockbackRatios::NONE,
        );
        let hit = melee_coll::damage::ReceivedHit {
            facing: self.core.physics.facing,
            facing_override: None,
            percent_damage: shot.damage as f32,
            descriptor,
            height: melee_coll::hurtbox::HurtHeight::Middle,
            knockback,
        };
        // x1848_kb_angle: the truncated aim, possibly negative (0x8009ED24).
        self.core.combat.launch_angle_override = Some(gekko_math::msl::fctiwz(angle));
        // Fighter_UnkTakeDamage_8006CC30, then ftCo_8008DCE0(gobj, 91, 0.0).
        self.core
            .refuse_unwitnessed_coin_spill("ftCo_BarrelWait.c:85 (the barrel's shot)");
        self.begin_damage_reaction(hit, Some(S::DamageFlyRoll), None, None, false, assets, rng)?;
        Ok(())
    }
}

/// ftCo_BarrelWait_Anim, _Phys, _Coll: empty.
pub fn wait_anim(_: &mut Fighter, _: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    Ok(None)
}
pub fn wait_physics(_: &mut Fighter, _: PhysicsPhase<'_>) {}
pub fn wait_collision(_: &mut Fighter, _: CollisionPhase<'_>) -> Result<()> {
    Ok(())
}

/// ftCo_BarrelWait_IASA (8009EBD4): A or B asks the barrel to fire
/// (Ground_801C4DD0); the scene relays it to the stage.
pub fn wait_input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.core.input.pressed.intersects(Buttons::A | Buttons::B) {
        f.core.barrel_fire_request = true;
    }
}
