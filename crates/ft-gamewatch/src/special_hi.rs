//! Fire, ftgamewatchspecialhi.c (8014DEF0..8014E4EC).
//!
//! Both entries leave the ground with every jump spent; accessory4 sets the
//! firemen's trampoline under him (a free article at TopN, 2.5 model units
//! down) and he rises on the animation's TransN, turned once by the stick
//! (ft_80085154 with fp->lstick_angle). The animation's end is the special
//! fall (the parachute is the fall's own model selection).
use crate::{
    articles::{self, Accessory},
    common::{self, row},
    init::GameWatch,
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        commands::{FootstepSound, SoundChannel},
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, ItemKind};

/// ftGw_MS_SpecialHi (373) and ftGw_MS_SpecialAirHi (374).
pub const GROUND: ActionId = ActionId(373);
pub const AIR: ActionId = ActionId(374);

pub const fn rows() -> [MotionRow; 2] {
    [
        row(GROUND, anim, input, physics, collision),
        row(AIR, anim, input, physics, collision),
    ]
}

/// ft_80088510(fp, 290066, 127, 64): the trampoline's sound.
const RESCUE_SOUND: u32 = 290_066;
/// The trampoline's drop below TopN, in model units (retail @249).
const RESCUE_DROP: f32 = 2.5;
/// The collision callback does nothing through this frame.
const COLLISION_START_FRAME: f32 = 4.0;

fn gw(f: &mut Fighter) -> &mut GameWatch {
    f.character.get_mut::<GameWatch>()
}

/// ftGw_SpecialHi_Enter (8014E0AC) / ftGw_SpecialAirHi_Enter (8014E158).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if !air {
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
    }
    // ftCommon_8007D60C.
    f.leave_ground_with_spent_jumps();
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Fire assets");
    // ftGameWatch_SpecialHi_SetVars.
    f.commands.variables = [0; 4];
    articles::install(f, Accessory::RescueSetup);
    // ftAnim_8006EBA4.
    f.step_animation(a);
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Effect,
        id: RESCUE_SOUND,
        volume: 127,
        pan: 64,
    });
}

/// ftGw_SpecialHi_ItemRescueSetup (8014DEF0): without a trampoline out,
/// one at TopN lowered by 2.5 model scales (8014DF4C: fnmsubs), in the
/// motion of the grounded or aerial row (it_802C8038); the accessory then
/// uninstalls. With one out it stays installed.
pub fn rescue_setup(f: &mut Fighter) {
    if gw(f).articles.rescue {
        return;
    }
    let c = &mut f.core;
    let mut position =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, 0, Vec3::ZERO);
    let scale = c.player.scale * c.attributes.size.model_scaling;
    // retail 0x8014DF4C: fnmsubs.
    position.y = gekko_math::fma::fnmsubs(RESCUE_DROP, scale, position.y);
    // Item_InitSpawnOnPlaneNoInitialCollision.
    let mut spawn = SpawnItem::held(
        ItemKind::GameWatchRescue,
        c.player.id,
        position,
        c.physics.facing,
    );
    spawn.spawn_argument = i32::from(c.motion_state.action.0 - GROUND.0);
    c.item_requests.push(ItemRequest::Spawn(spawn));
    gw(f).articles.rescue = true;
    articles::install_callbacks(f, ItemKind::GameWatchRescue);
    articles::uninstall(f);
}

/// ftGw_SpecialHi_ItemRescueRemove (8014DFFC): it_802C8158 destroys the
/// trampoline, which lets go of its owner.
pub fn remove_rescue(f: &mut Fighter) {
    if gw(f).articles.rescue {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::GameWatchRescue,
            control: ItemControl::Remove,
        });
        articles::destroyed(f, ItemKind::GameWatchRescue);
    }
}

/// ftGw_SpecialHi_Anim (8014E1F8) / ftGw_SpecialAirHi_Anim (8014E218): at
/// the animation's end, Fall when the landing-lag attribute is zero, else
/// ftCo_80096900(gobj, 1, 0, 1, 1.0, lag).
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let lag = gw(f).attributes.rescue.landing_lag;
        if lag == 0.0 {
            f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
        } else {
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftGw_SpecialHi_IASA (8014E290) / ftGw_SpecialAirHi_IASA (8014E2B0):
/// once, a stick past the range turns him to face it and leans the rise
/// by the excess times the angle attribute (8014E304 fsubs, 8014E31C and
/// 8014E328 fmuls; the part rotation is M_PI_2 * facing in double, frsp).
fn input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[0] != 0 {
        return;
    }
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    let (range, angle) = {
        let a = &gw(f).attributes.rescue;
        (a.stick_range, a.angle)
    };
    if magnitude > range {
        let side = if x > 0.0 { 1.0 } else { -1.0 };
        let lean = ((magnitude - range) * side) * angle;
        // ftCommon_UpdateFacing.
        f.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
        let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
        let root = f.animation.root;
        f.skeleton.set_rotation_y(root, rotation);
        gw(f).rescue_angle = -lean;
        f.commands.variables[0] = 1;
    }
}

/// ftGw_SpecialHi_Phys (8014E374) / ftGw_SpecialAirHi_Phys (8014E394):
/// ft_80085154, TransN turned by fp->lstick_angle (80085198: fmsubs,
/// 8008519C: fmadds).
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Fire TransN")
        .primary_history
        .offset;
    let angle = gw(f).rescue_angle;
    let cosine = gekko_math::msl::cosf(angle);
    let sine = gekko_math::msl::sinf(angle);
    let horizontal = offset.z * f.physics.facing;
    f.physics.self_velocity.x = gekko_math::fma::fmsubs(horizontal, cosine, offset.y * sine);
    f.physics.self_velocity.y = gekko_math::fma::fmadds(horizontal, sine, offset.y * cosine);
    common::finish_update(f, &p);
}

/// ftGw_SpecialHi_Coll (8014E3B4) / ftGw_SpecialAirHi_Coll (8014E3D4):
/// nothing through frame 4. Then, while rising, a landing (ft_80081D0C)
/// removes the trampoline and is the special landing; while falling,
/// ft_CheckGroundAndLedge toward the facing lands the same way (the
/// trampoline stays for its own check) or a ledge is caught.
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let started = f.animation.frame > COLLISION_START_FRAME;
    if !started {
        return Ok(());
    }
    let assets = p.assets.expect("Fire collision assets");
    let lag = gw(f).attributes.rescue.landing_lag;
    if f.physics.self_velocity.y >= 0.0 {
        if common::lands(f, &mut p) {
            remove_rescue(f);
            // ftCommon_8007D7FC, then ftCo_LandingFallSpecial_Enter.
            f.land();
            f.enter_special_landing(assets, false, lag)?;
            gw(f).articles.damage_callbacks = false;
        }
    } else if common::lands_facing(f, &mut p) {
        f.enter_special_landing(assets, false, lag)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}
