//! Squall Hammer, ftpopospecials.c (8011F68C..80120E68) for Popo and
//! ftnanaspecials.c (801238E4..801241A0) for Nana.
//!
//! Popo spins alone (SpecialS1 343 / SpecialAirS1 345) unless Nana stands
//! within attribute xD0 and is free (ftNn_Init_80123954); then both spin
//! linked (SpecialS2 344 / SpecialAirS2 346, Nana SpecialS_0 359 /
//! SpecialS_1 360) and share hitlag through x1A5C. Nana's rows copy Popo's
//! speeds and follow his position. B presses once the script opens
//! cmd_vars[2] lift the spin (every attribute x68 frames); the stick steers
//! it (the IASA's x1C); cmd_vars[3] tilts the model to the floor.
pub mod air;
pub mod ground;
pub mod partner;

use crate::{
    attributes::SquallAttributes,
    climber::{self, vars},
    partner as link,
};
use hsd_types::Vec3;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    part_rotation::Axis,
    state::MotionRow,
    ActionId, Fighter, MotionEntryFlags,
};
use melee_types::{mp::FtCollisionBox, GroundOrAir};

/// ftPp_MS_SpecialS1..ftPp_MS_SpecialAirS2 (343..346).
pub const GROUND_SOLO: ActionId = ActionId(343);
pub const GROUND_LINKED: ActionId = ActionId(344);
pub const AIR_SOLO: ActionId = ActionId(345);
pub const AIR_LINKED: ActionId = ActionId(346);

/// ftNn_Init_803CD820 / ftNn_Unk2_803CDD60: top 12, bottom 0, left (-6, 6),
/// right (6, 6).
const SPIN_BOX: FtCollisionBox = FtCollisionBox {
    top: 12.0,
    bottom: 0.0,
    left: hsd_types::Vec2 { x: -6.0, y: 6.0 },
    right: hsd_types::Vec2 { x: 6.0, y: 6.0 },
};

/// The ground/air switches' Fighter_ChangeMotionState flags (retail
/// 0x0C4C528A): the ground-air set with KeepGfx, SkipHit and KeepSfx.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_528A);

/// Rows 343..346 and Nana's 359/360.
pub const fn rows() -> [MotionRow; 4] {
    [
        climber::row(
            GROUND_SOLO.0,
            ground::anim::<false>,
            ground::input,
            ground::physics::<false>,
            ground::collision::<false>,
        ),
        climber::row(
            GROUND_LINKED.0,
            ground::anim::<true>,
            ground::input,
            ground::physics::<true>,
            ground::collision::<true>,
        ),
        climber::row(
            AIR_SOLO.0,
            air::anim::<false>,
            air::input,
            air::physics::<false>,
            air::collision::<false>,
        ),
        climber::row(
            AIR_LINKED.0,
            air::anim::<true>,
            air::input,
            air::physics::<true>,
            air::collision::<true>,
        ),
    ]
}

/// fp->mv.pp.specials. The entry's x0 (0.0) and x4 (10) are never read,
/// and x8 (an effect some other code would attach) stays NULL, so
/// ftPp_SpecialS_8011F720 never acts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SquallHammer {
    /// +234C xC: the last collision found the floor.
    pub on_floor: bool,
    /// +2350 x10: B presses waiting to lift.
    pub presses: i32,
    /// +2354 x14: frames since the last lift (attribute x68 at entry).
    pub since_rise: i32,
    /// +2358 x18: frames in the move, for the aerial gravity.
    pub frames: i32,
    /// +235C x1C: the stick's acceleration (the IASA's).
    pub steer: f32,
}

fn scratch(f: &mut Fighter) -> &mut SquallHammer {
    &mut climber::payload(f).squall
}

fn attributes(f: &Fighter) -> &SquallAttributes {
    &climber::attributes(f).squall
}

/// Which of the pair a row serves: attributes indexed [solo, linked].
const fn side(linked: bool) -> usize {
    linked as usize
}

/// What ftNn_Init_80123954 copies to Nana from Popo as it stood when it
/// ran (before his motion change set his speeds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Join {
    /// pp_ga: Popo's ground state picks Nana's row.
    pub ground_or_air: GroundOrAir,
    pub position: Vec3,
    pub self_velocity: Vec3,
    pub ground_velocity: f32,
    pub facing: f32,
    /// Popo's spawn number, Nana's x1A5C.
    pub leader: u32,
}

/// ftPp_SpecialS_Enter (8011F99C) / ftPp_SpecialAirS_Enter (8011FB08).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.variables = [0; 4];
    let interval = attributes(f).press_interval;
    *scratch(f) = SquallHammer {
        since_rise: interval,
        ..SquallHammer::default()
    };
    let linked = invite_partner(f);
    let state = match (airborne, linked) {
        (false, false) => GROUND_SOLO,
        (false, true) => GROUND_LINKED,
        (true, false) => AIR_SOLO,
        (true, true) => AIR_LINKED,
    };
    f.change_motion_state(state, assets)
        .expect("Squall Hammer assets");
    f.core.combat.hitlag_link.partner = linked.then(|| link::view(f).spawn_number);
    let a = attributes(f);
    let (ground_speed, air_speed, rise) = (
        a.ground_entry_speed,
        a.air_entry_speed,
        a.air_entry_rise[side(linked)],
    );
    let facing = f.physics.facing;
    if airborne {
        f.physics.self_velocity.y = rise;
        // retail 8011FBFC: fmuls.
        f.physics.self_velocity.x = air_speed * facing;
    } else {
        f.physics.self_velocity.y = 0.0;
        // retail 8011FA98: fmuls.
        let speed = ground_speed * facing;
        f.physics.self_velocity.x = speed;
        f.physics.ground_velocity = speed;
    }
    install_callbacks(f);
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    // inlineA1: x8 = NULL, x2219_b0.
    f.effect_state.destroy_on_state_change = true;
    install_callbacks(f);
}

/// ftNn_Init_80123954 (80123954) as Popo's entry sees it: Nana joins unless
/// her motion's class is 1..13 or she stands at least attribute xD0 away
/// (squared, against Popo's scale: fmuls, fctiwz). A joining Nana is moved
/// into her row after this proc (`partner::join`).
fn invite_partner(f: &mut Fighter) -> bool {
    let nana = link::view(f);
    let class = nana.class.unwrap_or_else(|| {
        unimplemented!("ftNn_Init_80123954: Nana's motion {:?} class", nana.action)
    });
    if (1..=13).contains(&class) {
        return false;
    }
    let reach = climber::attributes(f).squall_join_distance;
    // 801239C0/D0: fmuls, fmuls; fctiwz to s32 and back.
    let limit = gekko_math::msl::fctiwz(f.player.scale * (reach * reach)) as f32;
    let dx = nana.position.x - f.physics.position.x;
    let dy = nana.position.y - f.physics.position.y;
    // 801239E8..801239FC: fmuls, fmuls, fadds; fcmpo, bge (unordered too).
    let near = (dx * dx + dy * dy).partial_cmp(&limit) == Some(std::cmp::Ordering::Less);
    if !near {
        return false;
    }
    link::work(f).join = Some(Join {
        ground_or_air: f.physics.ground_or_air,
        position: f.physics.position,
        self_velocity: f.physics.self_velocity,
        ground_velocity: f.physics.ground_velocity,
        facing: f.physics.facing,
        leader: f.spawn_number,
    });
    true
}

/// inlineA0: take_dmg_cb and death2_cb = ftPp_Init_8011F060, and the
/// efLib pause/resume hitlag callbacks.
fn install_callbacks(f: &mut Fighter) {
    vars(f).ice_callbacks = true;
    f.effect_state.hitlag_callbacks = true;
}

/// resetAnim / inline0: the callbacks go and x1A5C is cleared.
fn remove_callbacks(f: &mut Fighter) {
    vars(f).ice_callbacks = false;
    f.effect_state.hitlag_callbacks = false;
    link::unlink(f);
}

/// ftCommon_ClampSelfVelX (8007D440).
fn clamp_self_velocity_x(f: &mut Fighter, maximum: f32) {
    let v = &mut f.physics.self_velocity.x;
    if *v < -maximum {
        *v = -maximum;
    } else if *v > maximum {
        *v = maximum;
    }
}

/// ftCommon_ClampGrVel (8007CC78).
fn clamp_ground_velocity(f: &mut Fighter, maximum: f32) {
    let v = &mut f.physics.ground_velocity;
    if *v < -maximum {
        *v = -maximum;
    } else if *v > maximum {
        *v = maximum;
    }
}

/// C's ABS (keeps -0.0's sign irrelevant to comparisons).
fn abs(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

/// ftPp_SpecialS1_IASA (8011FF40) / ftPp_SpecialAirS1_IASA (8011FFE0): the
/// stick past attribute x40 steers by x30 (ground) or x34 (air).
fn read_steer(f: &mut Fighter, multiplier: f32) {
    let stick = f.input.current.stick.x;
    let threshold = attributes(f).steer_threshold;
    // retail: fmuls.
    scratch(f).steer = if abs(stick) >= threshold {
        stick * multiplier
    } else {
        0.0
    };
}

/// The B-press count both physics callbacks keep: x14 counts frames, and
/// a press while cmd_vars[2] is open queues a lift. True when a queued
/// lift is due (more than attribute x68 frames since the last); the
/// counters restart.
fn press_due(f: &mut Fighter) -> bool {
    let open = f.commands.variables[2] != 0;
    let pressed = f.input.pressed.intersects(melee_ft::input::Buttons::B);
    let interval = attributes(f).press_interval;
    let s = scratch(f);
    s.since_rise += 1;
    if open && pressed {
        s.presses += 1;
    }
    if s.presses != 0 && s.since_rise > interval {
        s.presses = 0;
        s.since_rise = 0;
        return true;
    }
    false
}

/// inline2: with cmd_vars[3] set on the floor, the root tilts to it
/// (facing * atan2f(n.x, n.y)); otherwise it stands upright.
fn tilt(f: &mut Fighter) {
    let angle = if f.commands.variables[3] != 0 && scratch(f).on_floor {
        let normal = f.collision.data.floor.normal;
        f.physics.facing * melee_lb::trigf::atan2f(normal.x, normal.y)
    } else {
        0.0
    };
    f.core.set_part_rotation(0, Axis::X, angle);
}

/// The wall rebound both collisions share: moving into a wall on the
/// moving side, the speed reverses scaled by attribute x44 (fneg, fmuls),
/// or at attribute x48 when slower.
fn rebound(f: &Fighter, speed: f32) -> Option<f32> {
    use melee_types::mp::collide::{LEFT_WALL_MASK, RIGHT_WALL_MASK};
    if speed == 0.0 {
        return None;
    }
    let env = f.collision.data.env_flags as u32;
    let mask = if speed > 0.0 {
        LEFT_WALL_MASK
    } else {
        RIGHT_WALL_MASK
    };
    if env & mask == 0 {
        return None;
    }
    let a = attributes(f);
    Some(if abs(speed) < a.wall_rebound_min {
        if speed > 0.0 {
            -a.wall_rebound_min
        } else {
            a.wall_rebound_min
        }
    } else {
        speed * -a.wall_rebound
    })
}

/// Fighter_procUpdate's tail after the state's physics callback, by the
/// fighter's ground state then.
fn finish_update(f: &mut Fighter, p: &melee_ft::fighter::state::PhysicsPhase<'_>) {
    use melee_ft::physics::grounded;
    if f.physics.ground_or_air == GroundOrAir::Ground {
        let core = &mut f.core;
        grounded::finish_ground_update(
            &mut core.physics,
            &core.collision.data,
            &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
            p.map,
            p.wind,
        );
    } else {
        f.core.finish_air_update(p.assets, p.wind);
    }
}

/// ftCo_80096900(gobj, 1, 0, 1, 1.0, lag) after ftCommon_8007D60C, or
/// Fall with no lag: the aerial ends.
fn fall(f: &mut Fighter, lag: f32, assets: &FighterAssets) -> Result<()> {
    if lag == 0.0 {
        return climber::finish(f, assets, true);
    }
    f.core.leave_ground_with_spent_jumps();
    f.enter_special_fall(assets, true, false, true, 1.0, lag)
}
