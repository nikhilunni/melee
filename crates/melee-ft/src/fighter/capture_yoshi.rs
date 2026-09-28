//! The captured side of Yoshi's Egg Lay: ftCo_CaptureYoshi.c (on the
//! tongue) and ftCo_YoshiEgg.c (laid as an egg). Yoshi's own states live in
//! ft-yoshi; it hands the egg its attributes through `EggParameters`.
use super::{
    assets::{FighterAssets, Result},
    grab::GrabLink,
    ledge::GrabExclusions,
    state::{AnimationPhase, CollisionPhase, PhysicsPhase},
    Fighter, FighterCore, MotionData,
};
use crate::anim::WaitChoice;
use hsd_types::{Vec2, Vec3};
use melee_types::{combat::HurtStatus, CommonMotionState as S, FtPart, GroundOrAir};

/// Yoshi's submotion for the egg (ftCo_SM_YoshiEgg): the victim borrows the
/// captor's animation and script (Fighter_ChangeMotionState's `arg3`).
pub const EGG_MOTION: i32 = 277;
/// ftCo_YoshiEgg_Anim: ft_PlaySFX(fp, 280088, 127, 64) as the shell breaks.
const EGG_BREAK_SOUND: u32 = 280088;
/// ftCo_800BBC88 / ftCo_800DDDE4: ftColl_8007B62C's colour animations for
/// body states 2 (intangible) and 0 (normal).
const INTANGIBLE_FLASH: u8 = 2;
const NORMAL_BODY_FLASH: u8 = 1;
const INVINCIBLE_BODY_FLASH: u8 = 3;
/// ftColl_8007B760: the intangibility flash.
const INTANGIBILITY_FLASH: u8 = 9;

/// Egg Lay's attributes as the egg reads them (ftYs_SpecialN_Get*): the
/// captor's dat_attrs at the swallow, and ftData[Yoshi].ext_attr (the same
/// block for every Yoshi) each frame after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EggParameters {
    /// ftYs_SpecialN_SetupItemVel (8012CC94): -facing * +10, +14, 0.
    pub launch_velocity: Vec3,
    /// ftYs_SpecialN_GetFacingDir: the captor's facing.
    pub facing: f32,
    /// +18, dmg.x182c_behavior while in the egg.
    pub damage_behavior: f32,
    /// +1C, the share of the egg's scale that grows in.
    pub growth: f32,
    /// +20, frames over which it grows.
    pub growth_frames: f32,
    /// +24, the escape timer (ftCommon_InitGrab).
    pub duration: f32,
    /// +28, timer decrement per frame.
    pub decrement: f32,
    /// +2C, ftCommon_GrabMash's decrement per input.
    pub mash_decrement: f32,
    /// +30 / +34, the mashing animation's hold frames and rate.
    pub fast_frames: f32,
    pub fast_rate: f32,
    /// +38, ftColl_8007B760's intangibility after breaking out.
    pub exit_intangibility: i32,
    /// +3C, the velocity out of the shell (ftYs_SpecialN_8012CD88).
    pub release_velocity: Vec2,
    /// ftYs_SpecialN_8012CDB4: +44 / +18, fdivs.
    pub damage_ratio: f32,
}

/// Fighter mv.co.yoshiegg (+2340) with the fighter's grab timer and mash state.
#[derive(Clone, Debug)]
pub struct YoshiEggState {
    pub parameters: EggParameters,
    /// x0: the Yoshi that laid the egg (spawn number).
    pub layer: u32,
    /// x4: this frame's ftCommon_GrabMash result.
    pub mashed: bool,
    /// x8: frames left at the mashing animation rate.
    pub fast_remaining: f32,
    /// x14: growth frames left; accessory4 (ftCo_800BBCC0) runs until 0.
    pub growth_remaining: f32,
    /// x18: the root joint's scale at the swallow.
    pub root_scale: Vec3,
    /// accessory4_cb == ftCo_800BBCC0.
    pub growing: bool,
    /// grab_timer (+1A4C).
    pub timer: f32,
    /// x1A50 / x1A51, reset by ftCommon_InitGrab.
    stick_directions: [i8; 2],
}

/// Requests Yoshi's swallow makes of its captured fighter from its own
/// animation callback; the scene applies them to the pair, in this order,
/// right after that callback (ftYs_SpecialN2_0_Anim's inlineA1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureRequests {
    /// cmd_vars[1]: ftCo_800BBC88 on the victim.
    pub hide: bool,
    /// cmd_vars[0]: ftCo_800DE2CC, then ftCo_800BBED4.
    pub lay_egg: Option<EggParameters>,
}

/// ftCommon_GrabMash (8007DC08): a button press and a new stick direction
/// each take `decrement` from the timer. Neutral keeps each axis's last
/// non-neutral direction.
pub(super) fn grab_mash(
    timer: &mut f32,
    directions: &mut [i8; 2],
    input: &crate::input::FighterInput,
    threshold: f32,
    decrement: f32,
) -> bool {
    use crate::input::Buttons;
    let buttons = input
        .pressed
        .intersects(Buttons::A | Buttons::B | Buttons::XY | Buttons::SHIELD);
    if buttons {
        *timer -= decrement;
    }
    let previous = *directions;
    for (direction, value) in directions
        .iter_mut()
        .zip([input.current.stick.x, input.current.stick.y])
    {
        if value < -threshold {
            *direction = -1;
        }
        if value > threshold {
            *direction = 1;
        }
    }
    let stick = previous != *directions;
    if stick {
        *timer -= decrement;
    }
    buttons || stick
}

impl Fighter {
    /// ftCo_800BBB8C (800BBB8C): the tongue's grabbed_cb. The captured
    /// fighter hangs from Yoshi's TransN2 like a thrown fighter (the XRotN
    /// constraint of ftCo_800DB368, accessory1 ftCo_800DB464).
    pub fn enter_capture_yoshi(
        &mut self,
        captor: &mut FighterCore,
        assets: &FighterAssets,
        captor_assets: &FighterAssets,
    ) -> Result<()> {
        self.interrupt_actions();
        // ftCo_8009750C drops a heavy item; ftCo_800DD168 releases this
        // fighter's own victim (a grabbed fighter is never a candidate).
        if self.held_item.is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_Lift.c:201-211: a heavy item dropped by Egg Lay's catch");
        }
        assert!(self.combat.grab.is_none(), "ftCo_800DD168: a captor caught");
        self.combat.grab = Some(GrabLink::Captured {
            captor: captor.spawn_number,
        });
        self.physics.facing = -captor.physics.facing;
        super::grab_throw::constrain_to_captor(&mut self.core, captor, assets, captor_assets);
        self.change_motion_state(S::CaptureYoshi.into(), assets)?;
        self.leave_ground();
        self.status.grab_exclusions = GrabExclusions::ALL;
        self.step_animation(assets);
        self.clear_movement();
        Ok(())
    }
}

/// ftCo_800BBC88 (800BBC88): the swallowed fighter vanishes and its body
/// turns intangible (ftColl_8007B62C(gobj, 2)).
pub fn hide_captured(victim: &mut FighterCore) {
    victim.effect_state.invisible = true;
    set_body_status(victim, HurtStatus::Intangible, INTANGIBLE_FLASH);
}

impl FighterCore {
    /// ftColl_8007B62C (8007B62C): the whole body's hurt status (x1988) and
    /// the colour animation each status installs (1, 3 and 2).
    pub fn set_body_hurt_status(&mut self, status: HurtStatus) {
        set_body_status(self, status, body_status_color(status));
    }
}

/// ftColl_8007B62C's colour animation for each whole-body status.
pub(crate) fn body_status_color(status: HurtStatus) -> u8 {
    match status {
        HurtStatus::Normal => NORMAL_BODY_FLASH,
        HurtStatus::Invincible => INVINCIBLE_BODY_FLASH,
        HurtStatus::Intangible => INTANGIBLE_FLASH,
    }
}

/// ftColl_8007B62C: x1988 and its colour animation.
fn set_body_status(fighter: &mut FighterCore, status: HurtStatus, flash: u8) {
    fighter.commands.hurt_status = status;
    fighter
        .commands
        .color_animations
        .push(melee_cmd::ColorAnimationRequest {
            id: flash,
            duration: 0,
        });
}

/// ftCo_800DE2CC (ftCo_800DDDE4 without the release offset), then
/// ftCo_800BBED4 (800BBED4): Yoshi lets go of the swallowed fighter at its
/// TransN2 and lays it as an egg.
pub fn lay_egg(
    victim: &mut Fighter,
    yoshi: &mut Fighter,
    victim_assets: &FighterAssets,
    yoshi_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    parameters: EggParameters,
) -> Result<()> {
    // ftCo_800DDDE4: a swallowed body is normal again before the damage
    // test; ftColl_8007B868 then zeroes the damage of an intangible or
    // invincible victim.
    if victim.commands.hurt_status != HurtStatus::Normal {
        set_body_status(&mut victim.core, HurtStatus::Normal, NORMAL_BODY_FLASH);
    }
    if victim.status.ledge_intangibility != 0 || victim.status.revival_invincibility != 0 {
        unimplemented!("ftCo_800DDDE4: Egg Lay on an intangible or invincible fighter");
    }
    let hit = super::grab_throw::release_captured(
        &mut victim.core,
        &mut yoshi.core,
        victim_assets,
        yoshi_assets,
        map,
        false,
    );
    // ftColl_80076640: the damage waits for the victim's ProcessHit. The
    // egg's motion entry clears the knockback, so it lands there as damage
    // without a reaction or hitlag.
    victim.combat.frame_damage += hit.percent_damage;
    victim.combat.frame_max_damage = victim
        .combat
        .frame_max_damage
        .max(super::hit_log::damage_count(hit.percent_damage));
    enter_egg(victim, yoshi, victim_assets, yoshi_assets, parameters)
}

/// ftCo_800BBED4 (800BBED4).
fn enter_egg(
    victim: &mut Fighter,
    yoshi: &Fighter,
    assets: &FighterAssets,
    yoshi_assets: &FighterAssets,
    parameters: EggParameters,
) -> Result<()> {
    if victim.physics.ground_or_air == GroundOrAir::Ground {
        victim.leave_ground();
    }
    victim.enter_borrowed_egg_motion(yoshi.spawn_number, assets, yoshi_assets)?;
    victim.status.grab_exclusions = GrabExclusions::ALL;
    victim.effect_state.invisible = true;
    egg_hurt_capsule(&mut victim.core, assets);
    victim.physics.self_velocity = parameters.launch_velocity;
    victim.physics.facing = parameters.facing;
    // ftCommon_8007EFC0(fp, 1).
    victim.status.name_tag_timer = 1;
    let root = victim.animation.root;
    victim.state_data = MotionData::YoshiEgg(YoshiEggState {
        parameters,
        layer: yoshi.spawn_number,
        mashed: false,
        fast_remaining: 0.0,
        growth_remaining: parameters.growth_frames,
        root_scale: victim.skeleton.scale(root),
        growing: true,
        timer: parameters.duration,
        stick_directions: [0; 2],
    });
    Ok(())
}

/// ftCo_800BBED4's inlineA0: every capsule intangible, then capsule 0 is the
/// egg on TransN from the victim's own co_attrs.xBC.
fn egg_hurt_capsule(victim: &mut FighterCore, assets: &FighterAssets) {
    victim.set_hurt_capsules(HurtStatus::Intangible);
    let egg = &victim.attributes.yoshi_egg;
    let capsule = melee_coll::hurtbox::HurtCapsule {
        height: melee_coll::hurtbox::HurtHeight::Middle,
        grabbable: false,
        bone: usize::from(assets.parts.joint(FtPart::TransN).expect("TransN")),
        offsets: [egg.hurtbox_start, egg.hurtbox_end],
        radius: egg.hurtbox_scale,
        positions: [Vec3::ZERO; 2],
        cached: false,
    };
    victim.replace_hurt_capsule(0, capsule);
}

fn egg(fighter: &mut FighterCore) -> &mut YoshiEggState {
    let MotionData::YoshiEgg(egg) = &mut fighter.state_data else {
        panic!("Yoshi egg scratch missing")
    };
    egg
}

/// ftCo_YoshiEgg_Anim (800BC1B0).
pub fn egg_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let threshold = phase.assets.grab_escape.stick_threshold;
    let input = f.input.clone();
    let state = egg(&mut f.core);
    let p = state.parameters;
    state.timer -= p.decrement;
    state.mashed = grab_mash(
        &mut state.timer,
        &mut state.stick_directions,
        &input,
        threshold,
        p.mash_decrement,
    );
    if state.timer <= 0.0 {
        break_out(f, phase.assets, p)?;
        return Ok(None);
    }
    let mut rate = None;
    if state.fast_remaining != 0.0 {
        state.fast_remaining -= 1.0;
        if state.fast_remaining <= 0.0 && !state.mashed {
            rate = Some(1.0);
            state.fast_remaining = 0.0;
        }
    }
    if state.fast_remaining <= 0.0 && state.mashed {
        state.fast_remaining = p.fast_frames;
        rate = Some(p.fast_rate);
    }
    if let Some(rate) = rate {
        f.core.animation.set_rate(&mut f.core.skeleton, rate, false);
    }
    Ok(None)
}

/// ftCo_YoshiEgg_Anim's escape: the shell bursts and the fighter falls out.
fn break_out(f: &mut Fighter, assets: &FighterAssets, p: EggParameters) -> Result<()> {
    f.core
        .commands
        .footstep_sounds
        .push(super::commands::FootstepSound {
            channel: super::commands::SoundChannel::Ordinary,
            id: EGG_BREAK_SOUND,
            volume: 127,
            pan: 64,
        });
    // efAsync_Spawn(gobj, &x60C, 4, 1231, parts[FtPart_TopN], &co_attrs.xBC):
    // the part index is used unmapped.
    f.core
        .effects
        .push(melee_ef::request::EffectRequest::EggShell {
            bone: FtPart::TopN as usize,
            scale: f.core.attributes.yoshi_egg.size,
        });
    f.physics.self_velocity = Vec3::new(p.release_velocity.x, p.release_velocity.y, 0.0);
    f.leave_ground();
    f.core.update_model_scale();
    // ftColl_8007B760: x1990 = max(x1990, frames), then the flash.
    f.status.ledge_intangibility = f.status.ledge_intangibility.max(p.exit_intangibility);
    f.core
        .commands
        .color_animations
        .push(melee_cmd::ColorAnimationRequest {
            id: INTANGIBILITY_FLASH,
            duration: 0,
        });
    f.enter_fall_from_egg(assets)
}

/// ftCo_YoshiEgg_Phys (800BC308): ft_80084F3C on the ground, ft_80084DB0 in the air.
pub fn egg_physics(f: &mut Fighter, phase: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        super::state::callbacks::physics::guard_on(f, phase);
    } else {
        super::state::callbacks::physics::pass(f, phase);
    }
}

/// ftCo_YoshiEgg_Coll (800BC340): ft_8008403C / ft_80082C74 whose callbacks
/// only change ground state (ftCo_800BC3AC, ftCo_800BC388): the egg rolls
/// on in the same motion.
pub fn egg_collision(f: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    use crate::collision::{air, ground};
    let c = &mut f.core;
    if c.physics.ground_or_air == GroundOrAir::Ground {
        if ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            phase.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ) != ground::WaitGroundResult::Supported
        {
            f.leave_ground();
        }
    } else {
        air::begin_map(
            &c.physics,
            &mut c.collision,
            &mut c.skeleton,
            c.animation.root,
        );
        if air::collide_air_dodge(
            &mut c.physics,
            &mut c.collision,
            phase.map,
            &mut c.skeleton,
            c.animation.root,
        ) {
            f.land();
        }
    }
    Ok(())
}

/// Fighter_ProcessHit's knockback branch for the egg: the frame's damage
/// reaches percent (Fighter_UnkTakeDamage_8006CC30), then take_dmg_2_cb
/// ftCo_800BC3D0 (800BC3D0) takes it, scaled, off the escape timer.
pub(super) fn egg_hit(fighter: &mut FighterCore, hit: &melee_coll::damage::ReceivedHit) {
    fighter.physics.percent += hit.percent_damage;
    let state = egg(fighter);
    // retail 800BC3F0: fnmsubs, timer = -(percentTemp * ratio - timer).
    state.timer = gekko_math::fma::fnmsubs(
        hit.percent_damage,
        state.parameters.damage_ratio,
        state.timer,
    );
    // 800BC3F8: dmg.x18CC == 3 (a stage hazard's hit) with a burying
    // x18D0 empties the timer; fighter and item hits log 1 and 2.
}

impl FighterCore {
    /// dmg.x182c_behavior (Fighter +182C): Fighter_ChangeMotionState resets
    /// it to 1; ftCo_800BBED4 sets the egg's +18. Hit detection multiplies
    /// every received damage by it (ftColl_800765F0, ftcoll.c:580 / 1158).
    pub(super) fn received_damage_scale(&self) -> f32 {
        match &self.state_data {
            MotionData::YoshiEgg(egg) => egg.parameters.damage_behavior,
            _ => 1.0,
        }
    }

    /// ftCo_800BBCC0 (800BBCC0), the egg's accessory4: grow the body into the
    /// egg, then restore the model scale and uninstall. Returns whether the
    /// accessory4 slot is the egg's. Separate fsubs/fdivs/fmuls/fadds.
    pub fn egg_accessory(&mut self) -> bool {
        let MotionData::YoshiEgg(egg) = &mut self.state_data else {
            return false;
        };
        if !egg.growing {
            return false;
        }
        if egg.growth_remaining <= 0.0 {
            egg.growing = false;
            self.update_model_scale();
            return true;
        }
        egg.growth_remaining -= 1.0;
        let total = egg.parameters.growth_frames;
        let growth = egg.parameters.growth;
        let grown = (total - egg.growth_remaining) / total * growth;
        let s = (1.0 - growth) + grown;
        let scale = Vec3::new(
            s * egg.root_scale.x,
            s * egg.root_scale.y,
            s * egg.root_scale.z,
        );
        // The accessory's own scale (s * mv.yoshiegg.scale) only draws.
        let root = self.animation.root;
        self.skeleton.set_scale(root, &scale);
        true
    }

    /// Fighter_UpdateModelScale (80067BB4), fighter.c:213-230: the root
    /// takes Player_GetModelScale * co_attrs.model_scaling (separate fmuls).
    pub(super) fn update_model_scale(&mut self) {
        let scale = self.player.scale * self.attributes.size.model_scaling;
        let root = self.animation.root;
        self.skeleton
            .set_scale(root, &Vec3::new(scale, scale, scale));
    }
}
