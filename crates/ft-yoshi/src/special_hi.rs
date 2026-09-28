//! Egg Throw, ftyoshispecialhi.c (8012DF8C..8012E6C8). Yoshi winds up
//! (SpecialHi 364 / SpecialAirHi 365); a subaction flag creates the egg in
//! his hand, and command variable 0 sends it out along the stick-bent aim,
//! faster and spinning harder the longer B was held.
use crate::init::Yoshi;
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation, MotionRow,
    },
    input::pad::Buttons,
    physics::airborne,
};
use melee_it::{ItemControl, ItemRequest, Launch, SpawnItem};
use melee_types::{CommonMotionState, ItemKind};

/// ftYs_MS_SpecialHi / SpecialAirHi.
pub const GROUND: ActionId = ActionId(364);
pub const AIR: ActionId = ActionId(365);
/// ftYs_SM_SpecialHi / SpecialAirHi (ftCo_SM_Count + 14 / + 15).
const GROUND_ANIMATION: i32 = 309;
const AIR_ANIMATION: i32 = 310;
/// fn_8012E110: the egg hangs from fp->parts[31].
const EGG_PART: u8 = 31;

/// fp->mv.ys.specialhi (Fighter +2340 / +2344).
#[derive(Clone, Debug, Default)]
pub struct SpecialHi {
    /// x0: the egg has been thrown; landing afterwards may grab a ledge.
    pub thrown: bool,
    /// x4: frames B was held, which speed up and spin the egg.
    pub charge: i32,
}

/// ftYs_Init_MotionStateTable[23..24].
pub const fn rows() -> [MotionRow; 2] {
    [
        MotionRow {
            action: GROUND,
            id: CommonMotionState::None,
            animation: GROUND_ANIMATION,
            anim: ground_animation,
            iasa: no_input,
            physics: callbacks::physics::guard_on,
            collision: ground_collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        },
        MotionRow {
            action: AIR,
            id: CommonMotionState::None,
            animation: AIR_ANIMATION,
            anim: air_animation,
            iasa: no_input,
            physics: air_physics,
            collision: air_collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        },
    ]
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftYs_SpecialHi_Enter (8012E2C0) / ftYs_SpecialAirHi_Enter (8012E338):
/// scratch, egg handle, throw flags and command variable 0 reset; the
/// motion enters at frame 0 and plays its first frame (ftAnim_8006EBA4).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let yoshi = f.character.get_mut::<Yoshi>();
    yoshi.special_hi = SpecialHi::default();
    yoshi.egg_active = false;
    // fp->throw_flags = 0.
    f.commands.throw_accessory = false;
    f.commands.throw_reverse = false;
    f.commands.grab_release = false;
    f.commands.rapid_jab_loop_end = false;
    f.commands.variables[0] = 0;
    f.change_motion_state(if air { AIR } else { GROUND }, assets)
        .expect("Egg Throw motion assets");
    f.arm_accessory4();
    f.step_animation(assets);
}

/// ftYs_SpecialHi_Anim (8012E4DC): count held B, then Wait (ft_8008A2BC).
fn ground_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    count_charge(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
/// ftYs_SpecialAirHi_Anim (8012E540): count held B, then Fall.
fn air_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    count_charge(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
    }
    Ok(None)
}
fn count_charge(f: &mut Fighter) {
    if f.input.current.held.intersects(Buttons::B) {
        f.character.get_mut::<Yoshi>().special_hi.charge += 1;
    }
}

/// ftYs_SpecialAirHi_Phys -> ft_80084EEC: gravity and air friction.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let core = &mut f.core;
    let air = &core.attributes.air;
    core.physics.self_velocity.y =
        airborne::gravity(core.physics.self_velocity.y, air.gravity, air.terminal_velocity);
    core.physics.animation_velocity.x =
        airborne::drift_acceleration(core.physics.self_velocity.x, 0.0, 0.0, air);
    core.finish_air_update(p.assets, p.wind);
}

/// ftYs_SpecialHi_Coll -> ft_8008403C: off the floor, fn_8012E3B4.
fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    let supported = matches!(
        map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        WaitGroundResult::Supported
    );
    if !supported {
        let assets = p.assets.expect("Egg Throw collision assets");
        // fn_8012E3B4: ftCommon_GroundToAirStateChange (0x0C4C5082).
        f.leave_ground();
        f.change_ground_air_motion(AIR, assets, keep_effects())?;
        f.arm_accessory4();
        // ftCommon_ClampAirDrift (8007D468): clamp only X.
        let maximum = f.attributes.air.air_drift_max;
        f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    }
    Ok(())
}

/// ftYs_SpecialAirHi_Coll (8012E5E4): before the throw an ordinary air
/// pass (ft_80082C74); after it, ft_80083A48, which can also catch a
/// ledge from either side. Landing keeps the motion (fn_8012E44C).
fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::{air, ecb::EcbPose};
    let assets = p.assets.expect("Egg Throw collision assets");
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let thrown = f.character.get::<Yoshi>().special_hi.thrown;
    let c = &mut f.core;
    let landed = if thrown {
        // ft_CheckGroundAndLedge(gobj, CLIFFCATCH_BOTH).
        let cd = &mut c.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = c.physics.position;
        let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
        let landed = if c.status.ledge_cooldown == 0 {
            melee_mp::set_facing_dir(cd, 0);
            p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
        } else {
            p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
        };
        c.physics.position = cd.cur_pos;
        c.skeleton
            .set_translate(c.animation.root, &c.physics.position);
        landed
    } else {
        air::collide_air_dodge(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
        )
    };
    if landed {
        // fn_8012E44C: ftCommon_AirToGroundStateChange (0x0C4C5082).
        f.land();
        f.change_ground_air_motion(GROUND, assets, keep_effects())?;
        f.arm_accessory4();
    } else if thrown {
        // ftWallJump_8008169C: Yoshi has no wall jump.
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// The shared ground/air change flags 0x0C4C5082: effects are kept.
fn keep_effects() -> MotionPreservation {
    MotionPreservation {
        effects: true,
        ..Default::default()
    }
}

/// fn_8012E110 (8012E110), accessory4 of both motions: the throw flag
/// creates the egg in Yoshi's hand; command variable 0 then throws it.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    if !f.accessory4_armed || (f.motion_state.action != GROUND && f.motion_state.action != AIR) {
        return;
    }
    if std::mem::take(&mut f.commands.throw_accessory) {
        create_egg(f, assets);
    }
    if f.commands.variables[0] != 0 && f.character.get::<Yoshi>().egg_active {
        f.commands.variables[0] = 0;
        throw_egg(f, assets);
    }
}

/// it_802B2A10 via fn_8012E110: the egg at the hand (lb_8000B1CC of
/// fp->parts[31]), swept from Yoshi's body centre; fp->x2238 holds it and
/// the take-damage and death callbacks will drop it.
fn create_egg(f: &mut Fighter, assets: &FighterAssets) {
    let mut holder = f.core.item_holder(EGG_PART, assets);
    let hand = holder.part_position();
    let center = holder.center;
    // it_802B2A10: prev_pos is the hand as sampled (z included), pos the
    // body centre (it_8026BB68), no velocity, x44_flag.b0 set.
    let spawn = SpawnItem {
        previous_position: hand,
        position: center,
        ..SpawnItem::ray(ItemKind::YoshiEggThrow, f.player.id, hand, f.physics.facing)
    };
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: EGG_PART,
        hold: false,
    });
    f.character.get_mut::<Yoshi>().egg_active = true;
}

/// fn_8012E110's throw: it_802B28C8 with the aim of ftYs_SpecialS_8012DF8C,
/// an offset of (x104 * facing, x108, 0) and x4 * x110 + x10C degrees of
/// spin per frame (retail 8012E23C: fmadds). The callbacks come off.
fn throw_egg(f: &mut Fighter, assets: &FighterAssets) {
    let scratch = &mut f.character.get_mut::<Yoshi>().special_hi;
    scratch.thrown = true;
    let charge = scratch.charge as f32;
    let a = &f.character.get::<Yoshi>().attributes.egg_throw;
    // retail 8012E1F0: fmuls.
    let offset = Vec3::new(a.spawn_offset_x * f.physics.facing, a.spawn_offset_y, 0.0);
    let spin_degrees = fmadds(charge, a.spin_per_charge_frame, a.base_spin);
    let velocity = aim(f);
    let holder = f.core.item_holder(EGG_PART, assets);
    // ftLib_80086630: fp->parts[xDC4].joint's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let launch = Launch {
        velocity,
        offset,
        spin_degrees,
        hand,
        center: holder.center,
        attack: holder.attack,
        attack_stale: holder.attack_stale,
        aim: None,
        angle: 0.0,
        long_lifetime: false,
        shot: None,
    };
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind: ItemKind::YoshiEggThrow,
        launch,
    });
    f.character.get_mut::<Yoshi>().egg_active = false;
}

/// ftYs_SpecialS_8012DF8C (8012DF8C): the throw velocity. The stick's
/// horizontal share (|x| / xEC, capped at 1) scaled by xF0 bends the base
/// angle xF8, ignored below xF4; speed grows with the charge.
fn aim(f: &Fighter) -> Vec3 {
    let a = &f.character.get::<Yoshi>().attributes.egg_throw;
    let stick = f.input.current.stick.x;
    // retail 8012DFC0..DC: the fcmpo/fneg absolute value, then fdivs.
    let magnitude = if stick < 0.0 { -stick } else { stick };
    let mut bend = magnitude / a.angle_stick_divisor;
    if bend > 1.0 {
        bend = 1.0;
    }
    bend *= a.angle_range;
    if bend < a.minimum_angle_adjustment {
        bend = 0.0;
    }
    bend *= if stick < 0.0 { -1.0 } else { 1.0 };
    let angle = if f.physics.facing == 1.0 {
        a.base_angle - bend
    } else {
        // retail 8012E064..6C: M_PI - base in double, then - bend, frsp.
        ((std::f64::consts::PI - f64::from(a.base_angle)) - f64::from(bend)) as f32
    };
    let charge = f.character.get::<Yoshi>().special_hi.charge as f32;
    // retail 8012E0A0 / 8012E0D8: fmadds, then fmuls by the cosine / sine.
    let speed_x = fmadds(charge, a.speed_per_charge_frame, a.base_speed);
    let x = speed_x * cosf(angle);
    let speed_y = fmadds(charge, a.speed_per_charge_frame, a.base_speed);
    let y = speed_y * sinf(angle);
    Vec3::new(x, y, 0.0)
}

/// ftYs_Init_8012BA8C (8012BA8C), Yoshi's take_dmg_cb and death2_cb while
/// the egg is in his hand (installed only then, and cleared by any other
/// motion change): ftYs_SpecialS_8012E270 destroys the unthrown egg
/// (it_802B2890); ftYs_SpecialS_8012DF18's swallowed item is not ported.
pub fn drop_egg(f: &mut Fighter) {
    let action = f.motion_state.action;
    let yoshi = f.character.get_mut::<Yoshi>();
    if !yoshi.egg_active || (action != GROUND && action != AIR) {
        return;
    }
    let thrown = yoshi.special_hi.thrown;
    yoshi.egg_active = false;
    if !thrown {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: ItemKind::YoshiEggThrow,
            control: ItemControl::Remove,
        });
    }
}
