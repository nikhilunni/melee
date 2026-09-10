//! Guard state, collision volumes and model pose (ftCo_Guard.c).
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData, RetailTrig,
};
use crate::input::{Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use gekko_math::{
    fma::fmadds,
    msl::{fctiwz, sqrtf},
};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

/// PlCo shield parameters; archive offsets are confined to the reader.
#[derive(Clone, Debug)]
pub struct ShieldParameters {
    pub minimum_size: f32,
    pub minimum_hold: f32,
    pub drain: f32,
    pub regeneration: f32,
    pub frame_damage: f32,
    pub damage_multiplier: f32,
    pub damage_lightshield: [f32; 2],
    pub stun_multiplier: f32,
    pub stun_base: f32,
    pub stun_lightshield: [f32; 2],
    pub pushback_multiplier: f32,
    pub pushback_maximum: f32,
    pub ordinary_pushback_multiplier: f32,
    pub attacker_pushback_multiplier: f32,
    pub attacker_pushback_base: f32,
    pub reflect_frames: f32,
    pub reflect_radius: f32,
    pub reflect_damage: f32,
    pub reflect_speed: f32,
    pub powershield_frames: f32,
    pub size_range: [f32; 2],
    pub drain_range: [f32; 2],
    pub alpha: f32,
    pub tilt_smoothing: f32,
    pub roll_threshold: f32,
    pub roll_window: i32,
    pub roll_interrupt_frames: i32,
}
impl ShieldParameters {
    pub fn read(a: &hsd_archive::Archive, p: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            minimum_size: r.f32(p + 0x264)?,
            minimum_hold: r.f32(p + 0x268)?,
            drain: r.f32(p + 0x278)?,
            regeneration: r.f32(p + 0x27C)?,
            frame_damage: r.f32(p + 0x288)?,
            damage_multiplier: r.f32(p + 0x284)?,
            damage_lightshield: [r.f32(p + 0x2dc)?, r.f32(p + 0x2e0)?],
            stun_multiplier: r.f32(p + 0x28c)?,
            stun_base: r.f32(p + 0x290)?,
            stun_lightshield: [r.f32(p + 0x2e4)?, r.f32(p + 0x2e8)?],
            pushback_multiplier: r.f32(p + 0x294)?,
            pushback_maximum: r.f32(p + 0x298)?,
            ordinary_pushback_multiplier: r.f32(p + 0x2bc)?,
            attacker_pushback_multiplier: r.f32(p + 0x3e0)?,
            attacker_pushback_base: r.f32(p + 0x3e4)?,
            reflect_frames: r.f32(p + 0x2A4)?,
            reflect_radius: r.f32(p + 0x2A8)?,
            reflect_damage: r.f32(p + 0x2AC)?,
            reflect_speed: r.f32(p + 0x2B0)?,
            powershield_frames: r.f32(p + 0x2B4)?,
            size_range: [r.f32(p + 0x2D4)?, r.f32(p + 0x2D8)?],
            drain_range: [r.f32(p + 0x2EC)?, r.f32(p + 0x2F0)?],
            alpha: r.f32(p + 0x2F4)?,
            tilt_smoothing: r.f32(p + 0x44C)?,
            roll_threshold: r.f32(p + 0x31C)?,
            roll_window: r.s32(p + 0x320)?,
            roll_interrupt_frames: r.s32(p + 0x324)?,
        })
    }
}
/// Fighter.mv.co.guard, +2340..236C. Survives Guard-family transitions.
#[derive(Clone, Debug, Default)]
pub struct GuardState {
    pub elapsed: f32,
    pub tilt_magnitude: f32,
    pub tilt_frame: f32,
    pub released: bool,
    pub minimum_hold: f32,
    pub reflect_frames: f32,
    pub powershield_frames: f32,
    pub interrupt_frames: i32,
    pub jump_delay: i32,
    pub grab_delay: i32,
    pub previous_lightshield: f32,
}
/// ShieldDesc/HitResult, ftColl_8007B1B8 (0x8007B1B8).
#[derive(Clone, Debug, Default)]
pub struct ShieldVolume {
    pub bone: usize,
    pub radius: f32,
    pub offset: Vec3,
    pub position: Vec3,
    pub position_cached: bool,
}
/// ReflectDesc, ftCo_8009370C (0x8009370C); M5 owns hit dispatch.
#[derive(Clone, Debug, Default)]
pub struct ReflectVolume {
    pub volume: ShieldVolume,
    pub maximum_damage: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub reflect_behavior: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldHitCallback {
    SetOff,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReflectHitCallback {
    /// ftYs_Shield_8012CACC: no response callback.
    Egg,
    Powershield,
}
/// ftColl_80076CBC's strongest contact; health damage accumulates separately.
#[derive(Clone, Debug)]
pub struct ShieldImpact {
    pub damage: i32,
    pub facing: f32,
    pub element: melee_types::HitElement,
}
/// Persistent Fighter shield fields and typed collision callbacks.
#[derive(Clone, Debug, Default)]
pub struct ShieldState {
    pub enabled: bool,
    pub active: bool,
    pub reflecting: bool,
    pub fresh_powershield: bool,
    pub reflect_window: bool,
    pub powershield_window: bool,
    pub lightshield: f32,
    pub size: f32,
    pub alpha: u8,
    pub damage_taken: i32,
    pub impact: Option<ShieldImpact>,
    /// Fighter.allow_sdi: GuardSetOff collision clamps to stage during SDI.
    pub allow_sdi: bool,
    pub hit: ShieldVolume,
    pub reflect: ReflectVolume,
    pub on_hit: Option<ShieldHitCallback>,
    pub on_reflect: Option<ReflectHitCallback>,
}
impl ShieldState {
    /// Fighter_ChangeMotionState (0x800693AC), fighter.c:1048-1056.
    pub fn clear_collision(&mut self) {
        self.fresh_powershield = false;
        self.enabled = false;
        self.active = false;
        self.reflecting = false;
        self.on_hit = None;
        self.on_reflect = None;
    }
    /// ftCo_80093790 (0x80093790), ftCo_Guard.c:875-882.
    pub fn reflect_hit(&mut self) {
        unimplemented!("ftCo_Guard.c:875-882: powershield reflection response");
    }
}
impl Fighter {
    /// ftCo_80092E50 -> ftCo_80092F2C (80092F2C): shield stun and defender pushback.
    fn take_shield_hit(&mut self, impact: ShieldImpact, assets: &FighterAssets) -> Result<()> {
        if self.core.shield.powershield_window {
            unimplemented!("ftCo_80092F2C: powershield impact");
        }
        self.character.guard_variant(&mut self.core.commands);
        self.change_motion_state(S::GuardSetOff.into(), assets)?;
        self.core.apply_shield_impact(impact, assets)
    }
    /// ftCo_80091A4C / ftCo_800924C0 / ftCo_80093A50,
    /// retail 80091A4C / 800924C0 / 80093A50.
    pub(super) fn enter_shield(&mut self, assets: &FighterAssets) -> Result<()> {
        let reflect = self
            .core
            .input
            .pressed
            .intersects(Buttons::DIGITAL_SHOULDERS)
            && i32::from(self.core.input.shoulder.tilt) < assets.input.powershield_window;
        if let Some(result) = (self.character.table().enter_shield)(self, assets, reflect) {
            return result;
        }
        self.change_motion_state(
            (if reflect { S::GuardReflect } else { S::GuardOn }).into(),
            assets,
        )?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Guard(GuardState {
            minimum_hold: assets.shield.minimum_hold,
            tilt_frame: 10.0,
            reflect_frames: if reflect {
                assets.shield.reflect_frames
            } else {
                0.0
            },
            powershield_frames: if reflect {
                assets.shield.powershield_frames
            } else {
                0.0
            },
            ..Default::default()
        });
        self.core.shield.fresh_powershield = reflect;
        self.core.shield.reflect_window = reflect;
        self.core.shield.powershield_window = reflect;
        self.install_shield();
        if reflect {
            self.core.input.shoulder.tilt = 0xFE;
            self.core.shield.reflecting = true;
            self.core.shield.reflect = ReflectVolume {
                volume: ShieldVolume {
                    bone: usize::from(self.core.bones.model.shield),
                    radius: assets.shield.reflect_radius,
                    ..Default::default()
                },
                maximum_damage: self.core.status.shield_health,
                damage_multiplier: assets.shield.reflect_damage,
                speed_multiplier: assets.shield.reflect_speed,
                reflect_behavior: true,
            };
            self.core.shield.on_reflect = Some(ReflectHitCallback::Powershield);
        }
        // ftCo_800921DC (800921DC): no fused arithmetic in retail.
        self.core.shield.lightshield = self.lightshield_input(assets).max(0.0);
        let joint = self.core.animation.parts[usize::from(self.core.bones.model.shield)].joint;
        self.core.skeleton.set_translate(joint, &Vec3::ZERO);
        self.queue_shield_effect(0x417);
        self.update_guard_pose(assets, 0.0)?;
        self.character.guard_variant(&mut self.core.commands);
        Ok(())
    }
    /// ftCo_80092908 (0x80092908): preserve scratch, replace the shield effect.
    pub fn enter_guard_hold(&mut self, assets: &FighterAssets) -> Result<()> {
        if let Some(result) = (self.character.table().enter_guard_hold)(self, assets) {
            return result;
        }
        self.change_motion_state(S::Guard.into(), assets)?;
        self.install_shield();
        self.queue_shield_effect(0x418);
        self.update_guard_pose(assets, 1.0)
    }
    pub fn enter_guard_off(&mut self, assets: &FighterAssets) -> Result<()> {
        if let Some(result) = (self.character.table().enter_guard_off)(self, assets) {
            return result;
        }
        self.change_motion_state(S::GuardOff.into(), assets)
    }
    /// GuardOn/Guard/GuardOff/GuardSetOff/GuardReflect Anim, ftCo_Guard.c.
    pub(super) fn shield_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if let Some(result) = (self.character.table().animate_shield)(self, assets) {
            return result;
        }
        let state = self.core.motion_state.id;
        if state == S::GuardSetOff {
            self.update_reflect_windows();
            if !self.core.animation.frames_remaining(&self.core.skeleton) {
                if self.guard().released {
                    return self.enter_guard_off(assets);
                }
                return self.enter_guard_hold(assets);
            }
            self.update_shield_size(assets);
            return Ok(());
        }
        if state == S::GuardReflect {
            self.update_reflect_windows();
        }
        self.guard().elapsed += 1.0;
        if state == S::GuardOff {
            if !self.core.animation.frames_remaining(&self.core.skeleton) {
                self.change_motion_state(S::Wait.into(), assets)?;
            }
            return Ok(());
        }
        self.drain_shield(assets);
        if state != S::Guard && self.guard().elapsed >= assets.motions[&37].animation.frames {
            self.enter_guard_hold(assets)
        } else {
            let blend = if state == S::Guard {
                1.0
            } else {
                self.guard().elapsed / assets.motions[&37].animation.frames
            };
            self.update_guard_pose(assets, blend)
        }
    }
    /// Guard IASAs, ftCo_Guard.c:468-478, 539-547, 602-617, 1052-1062.
    pub(super) fn shield_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if let Some(result) = (self.character.table().input_shield)(self, assets) {
            return result;
        }
        let state = self.core.motion_state.id;
        if state == S::GuardSetOff {
            return Ok(());
        }
        if state != S::GuardOff {
            if !self.core.input.current.held.intersects(Buttons::SHIELD) {
                self.guard().released = true;
            }
            if (self.guard().released && self.guard().minimum_hold == 0.0)
                || (!self.core.shield.active && !self.core.shield.reflecting)
            {
                return self.enter_guard_off(assets);
            }
            if self.guard().interrupt_frames != 0 {
                self.guard().interrupt_frames -= 1;
            }
            if state == S::GuardOn
                && self.guard().elapsed < assets.input.powershield_window as f32
                && self
                    .core
                    .input
                    .pressed
                    .intersects(Buttons::DIGITAL_SHOULDERS)
                && i32::from(self.core.input.shoulder.tilt) < assets.input.powershield_window
            {
                unimplemented!("ftCo_Guard.c:885-915: delayed powershield activation");
            }
        } else if self.guard().interrupt_frames != 0 {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[
                    P::SpecialSide,
                    P::SpecialUp,
                    P::SpecialNeutral,
                    P::SpecialDown,
                    P::Grab,
                    P::SmashSide,
                    P::SmashUp,
                    P::SmashDown,
                    P::TiltSide,
                    P::TiltUp,
                    P::TiltDown,
                    P::Jab,
                ],
            );
            if transition != T::None {
                return self.apply_ground_transition(assets, transition);
            }
        }
        // ftCo_8009515C: item-throw predicate is false without a held item.
        if self.spot_dodge_input(assets) {
            return self.enter_escape(assets, S::EscapeN);
        }
        if state != S::GuardOff {
            if let Some(roll) = self.roll_input(assets) {
                return self.enter_escape(assets, roll);
            }
            if matches!(state, S::GuardOn | S::GuardReflect) && self.guard().grab_delay != 0 {
                if self.core.input.pressed.intersects(Buttons::A) {
                    unimplemented!("ftCo_Catch.c:87-90: dash grab out of shield");
                }
                self.guard().grab_delay -= 1;
            }
            if self.core.input.pressed.intersects(Buttons::A)
                && self.core.input.current.held.intersects(Buttons::SHIELD)
            {
                unimplemented!("ftCo_Catch.c:30-34: grab out of shield");
            }
        }
        let jump = self.first_ground_transition(assets, context, &[P::Jump]);
        if jump != T::None {
            return self.apply_ground_transition(assets, jump);
        }
        if self.core.input.current.cstick.y >= assets.common.input.tap_jump_threshold {
            unimplemented!("ftCo_Jump.c:94-98: C-stick shield jump");
        }
        if state == S::GuardOff {
            return Ok(());
        }
        // ftCo_Pass.c:18-37: platform check is false on FD's solid floor.
        if self.core.collision.data.floor.flags & 0x100 != 0
            && self.core.input.current.stick.y <= -assets.movement.platform_drop_threshold
            && i32::from(self.core.input.vertical.tilt) < assets.movement.platform_drop_window
        {
            unimplemented!("ftCo_Pass.c:56-60: shield platform drop");
        }
        Ok(())
    }
    /// Fighter_ProcessHit_8006D1EC (0x8006D1EC), fighter.c:2816-2843.
    pub(super) fn shield_proc(&mut self, assets: &FighterAssets) -> Result<()> {
        self.core.update_shield_health(assets);
        if let Some(impact) = self.core.shield.impact.take() {
            self.take_shield_hit(impact, assets)?;
        }
        self.core.shield.damage_taken = 0;
        Ok(())
    }
}
impl FighterCore {
    /// lbColl_80007BCC (80007BCC): a shield is a point capsule in its bone's scale.
    pub(super) fn shield_contact(
        &mut self,
        hit: &melee_coll::hitbox::HitCapsule,
        attacker_scale: f32,
    ) -> Option<melee_coll::geometry::Contact> {
        use melee_coll::geometry::{capsule_contact, Capsule};
        let volume = &mut self.shield.hit;
        if !volume.position_cached {
            volume.position = super::caches::bone_position(
                &mut self.skeleton,
                self.animation.root,
                volume.bone,
                volume.offset,
            );
            volume.position_cached = true;
        }
        let joint = self.animation.parts[volume.bone].joint;
        let matrix = *self.skeleton.get_mtx(joint);
        // lbColl_80007BCC --fused: none; broadphase differs from hurtboxes.
        capsule_contact(
            Capsule {
                start: hit.previous_position,
                end: hit.position,
                radius: hit.descriptor.radius
                    * if hit.descriptor.ignore_scale {
                        1.0
                    } else {
                        attacker_scale
                    },
            },
            Capsule {
                start: volume.position,
                end: volume.position,
                radius: volume.radius,
            },
            &matrix,
            20.0 * self.player.scale,
        )
    }
    pub fn guard(&mut self) -> &mut GuardState {
        let MotionData::Guard(guard) = &mut self.state_data else {
            panic!("guard scratch missing")
        };
        guard
    }
    /// ftCo_80092450 (0x80092450), ftcoll.c:3175-3188.
    pub fn install_shield(&mut self) {
        self.shield.active = true;
        self.shield.enabled = true;
        self.shield.hit.bone = usize::from(self.bones.model.shield);
        self.shield.hit.radius = 1.0;
        self.shield.hit.offset = Vec3::ZERO;
        self.shield.on_hit = Some(ShieldHitCallback::SetOff);
    }
    pub(super) fn queue_shield_effect(&mut self, id: u16) {
        self.effect_state.destroy_on_state_change = true;
        self.effects.push(melee_ef::request::EffectRequest::Shield {
            id,
            bone: usize::from(self.bones.model.shield),
        });
    }
    pub(super) fn lightshield_input(&self, assets: &FighterAssets) -> f32 {
        let deadzone = assets.common.input.analog_shoulder_deadzone;
        (self.input.current.trigger - deadzone) / (1.0 - deadzone)
    }
    /// ftCo_80093BC0 (0x80093BC0): startup windows expire without a hit.
    pub fn update_reflect_windows(&mut self) {
        self.shield.fresh_powershield = false;
        if self.shield.reflect_window {
            self.guard().reflect_frames -= 1.0;
            if self.guard().reflect_frames < 0.0 {
                self.shield.reflect_window = false;
                self.shield.reflecting = false;
                self.install_shield();
            }
        }
        if self.shield.powershield_window {
            self.guard().powershield_frames -= 1.0;
            if self.guard().powershield_frames < 0.0 {
                self.shield.powershield_window = false;
            }
        }
    }
    /// ftCo_800925A4 (0x800925A4): continuous shield drain and hold timer.
    pub fn drain_shield(&mut self, assets: &FighterAssets) {
        if !self.shield.active {
            return;
        }
        self.guard().previous_lightshield = self.shield.lightshield;
        let input = self.lightshield_input(assets);
        if input >= 0.0 {
            self.shield.lightshield = input;
        }
        let p = &assets.shield;
        // retail 80092624 fmadds; health subtraction follows a rounded fmuls.
        self.status.shield_health -= p.drain
            * fmadds(
                self.shield.lightshield,
                p.drain_range[1] - p.drain_range[0],
                p.drain_range[0],
            );
        if self.status.shield_health < 0.0 {
            unimplemented!("ftCo_Guard.c:428-436: shield exhaustion -> ShieldBreakFly");
        }
        if self.guard().minimum_hold > 0.0 {
            self.guard().minimum_hold = (self.guard().minimum_hold - 1.0).max(0.0);
        }
    }
    /// ftCo_80091BC4 (0x80091BC4), stick-angle wrap and magnitude smoothing.
    pub(super) fn update_shield_tilt(&mut self, assets: &FighterAssets) {
        let stick = self.input.current.stick;
        let mut radians = melee_lb::trigf::stick_angle(stick.y, stick.x * self.physics.facing);
        if radians < 0.0 {
            radians += 2.0 * std::f32::consts::PI;
        }
        let degrees = (radians * (57.29578_f32)).clamp(0.0, 359.0);
        let g = self.guard();
        let angle = g.tilt_frame - 10.0;
        let mut delta = degrees - angle;
        if delta > 180.0 {
            delta -= 360.0;
        } else if delta < -180.0 {
            delta += 360.0;
        }
        // retail 80091C78 fmadds.
        let mut angle = fmadds(delta, assets.shield.tilt_smoothing, angle);
        if angle > 360.0 {
            angle -= 360.0;
        } else if angle < 0.0 {
            angle += 360.0;
        }
        g.tilt_frame = 10.0 + angle;
        // retail 80091CB4..CC0: separately rounded squares and sum.
        let magnitude = sqrtf(stick.x * stick.x + stick.y * stick.y).min(1.0);
        // retail 80091D3C fmadds.
        g.tilt_magnitude = fmadds(
            assets.shield.tilt_smoothing,
            magnitude - g.tilt_magnitude,
            g.tilt_magnitude,
        );
    }
    /// ftCo_80091D58 (0x80091D58); same inlined size math at 80092014/1C.
    pub(super) fn update_shield_size(&mut self, assets: &FighterAssets) {
        let p = &assets.shield;
        // retail 80091DAC / 80091DB4 fmadds; health fraction product rounds first.
        let light = fmadds(
            self.shield.lightshield,
            p.size_range[1] - p.size_range[0],
            p.size_range[0],
        );
        let health = (self.status.shield_health / assets.shield_health) * light;
        let size = fmadds(1.0 - p.minimum_size, health, p.minimum_size)
            * self.attributes.shield.initial_shield_size;
        self.shield.size = size;
        let bone = self.animation.parts[usize::from(self.bones.model.shield)].joint;
        self.skeleton.set_scale(bone, &Vec3::new(size, size, size));
        // inlineD0: integer truncation before the alpha addition, no FMA.
        self.shield.alpha =
            fctiwz(p.alpha + fctiwz(self.shield.lightshield * (255.0 - p.alpha)) as f32) as u8;
    }
    /// ftCo_80091E78 (0x80091E78): shield pose is collision/model state.
    pub(super) fn update_guard_pose(&mut self, assets: &FighterAssets, blend: f32) -> Result<()> {
        if !self.shield.active && !self.shield.reflecting {
            return Ok(());
        }
        self.update_shield_tilt(assets);
        let (magnitude, frame) = {
            let g = self.guard();
            (g.tilt_magnitude, g.tilt_frame)
        };
        self.animation.apply_guard_pose::<RetailTrig>(
            &mut self.skeleton,
            &assets.motions[&38],
            &assets.guard_pose,
            magnitude,
            frame,
            blend,
        )?;
        self.update_shield_size(assets);
        Ok(())
    }
}

impl FighterCore {
    /// Fighter_ProcessHit (fighter.c:2816-2843), before the shield-response entry.
    fn update_shield_health(&mut self, assets: &FighterAssets) {
        if self.shield.enabled {
            let p = &assets.shield;
            // Fighter_ProcessHit, retail 8006D2AC / 8006D2CC: fmadds.
            let light = fmadds(
                self.shield.lightshield,
                p.damage_lightshield[1] - p.damage_lightshield[0],
                p.damage_lightshield[0],
            );
            self.status.shield_health -= fmadds(
                p.damage_multiplier,
                self.shield.damage_taken as f32 * (1.0 - light),
                p.frame_damage,
            );
            if self.status.shield_health < 0.0 {
                unimplemented!("fighter.c:2837-2843: shield break");
            }
        } else if self.status.shield_health < assets.shield_health {
            self.status.shield_health =
                (self.status.shield_health + assets.shield.regeneration).min(assets.shield_health);
        }
    }
    /// ftCo_80092F2C (80092F2C): shield stun and push after motion entry.
    fn apply_shield_impact(&mut self, impact: ShieldImpact, assets: &FighterAssets) -> Result<()> {
        self.input.horizontal.tilt = 254;
        self.queue_shield_effect(0x419);
        let p = &assets.shield;
        // retail 80093038 and 8009305C: fmadds with a rounded damage product.
        let light = fmadds(
            self.shield.lightshield,
            p.stun_lightshield[1] - p.stun_lightshield[0],
            p.stun_lightshield[0],
        );
        let frames = fmadds(
            p.stun_multiplier,
            impact.damage as f32 * (1.0 - light),
            p.stun_base,
        );
        self.animation.set_rate(
            &mut self.skeleton,
            (0.1 + assets.motions[&40].animation.frames) / frames,
            false,
        );
        if impact.element == melee_types::HitElement::Cape {
            unimplemented!("ftCo_80092E50: cape shield response");
        }
        let push = ((frames * p.pushback_multiplier) * p.ordinary_pushback_multiplier)
            .min(p.pushback_maximum);
        self.physics.ground_velocity = if impact.facing < 0.0 { push } else { -push };
        self.install_shield();
        self.update_shield_size(assets);
        self.combat.hitlag_remaining = assets.damage.hitlag(impact.damage);
        self.shield.allow_sdi = self.combat.hitlag_remaining > 0.0;
        self.status.interaction = if self.shield.allow_sdi {
            super::Interaction::Hitlag
        } else {
            super::Interaction::Shield
        };
        Ok(())
    }
}
