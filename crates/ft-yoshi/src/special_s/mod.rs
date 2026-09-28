//! Egg Roll, ftyoshispecials.c (8012EB48..8013295C).
//!
//! Both entries hop into the aerial start (360). The roll proper is a ground
//! loop (357) with a turn-around (358) and an aerial loop (361 from the hop,
//! 362 after rolling off the ground); 359 and 363 break the egg. The motion
//! scratch is `mv.ys.specials`; the per-motion callbacks it installs are
//! tracked in [`Hooks`].
mod air;
mod ground;
mod model;

use crate::init::Yoshi;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags, MotionRow,
    },
};
use melee_types::{mp::FtCollisionBox, CommonMotionState};

/// ftYs_MS_SpecialAirSStart_0 (356): the grounded start after the hop lands.
pub const START_GROUND: ActionId = ActionId(356);
/// ftYs_MS_SpecialAirSLoop_0 (357): rolling on the ground.
pub const LOOP_GROUND: ActionId = ActionId(357);
/// ftYs_MS_SpecialAirSLoop_1 (358): turning round on the ground.
pub const TURN_GROUND: ActionId = ActionId(358);
/// ftYs_MS_SpecialAirSEnd (359): the egg breaks on the ground.
pub const END_GROUND: ActionId = ActionId(359);
/// ftYs_MS_SpecialAirSStart_1 (360): the entry hop, both entries.
pub const START_AIR: ActionId = ActionId(360);
/// ftYs_MS_SpecialAirSLoop_2 (361): rolling in the air from the hop.
pub const LOOP_AIR: ActionId = ActionId(361);
/// ftYs_MS_SpecialAirSLoop_3 (362): rolling in the air after the ground.
pub const LOOP_AIR_BOUNCE: ActionId = ActionId(362);
/// ftYs_MS_SpecialAirSLanding (363): the egg breaks in the air.
pub const END_AIR: ActionId = ActionId(363);

/// Flags for the loops' in-place state changes: KeepGfx | SkipModel |
/// SkipItemVis | SkipModelPartVis.
const LOOP_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0044_0012);
/// The end states entered from a collision: LOOP_FLAGS | SkipMatAnim |
/// UpdateCmd | Unk19.
const BREAK_FLAGS: MotionEntryFlags = MotionEntryFlags(0x004C_4092);
/// ftYs_MF_SpecialS_Coll: ftCommon_GroundAirColl_MF | KeepGfx | SkipModel.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5092);
/// ftYs_Unk3_803CEDA4: the egg's fixed ECB.
const EGG_BOX: FtCollisionBox = FtCollisionBox {
    top: 12.0,
    bottom: 0.0,
    left: hsd_types::Vec2 { x: -6.0, y: 6.0 },
    right: hsd_types::Vec2 { x: 6.0, y: 6.0 },
};
/// fp->parts indices the roll poses directly.
mod part {
    /// Faces the model along the facing; the turn spins it.
    pub const FACING: usize = 0;
    /// Tilts into the turn.
    pub const TILT: usize = 2;
    /// Rolls the egg.
    pub const ROLL: usize = 3;
    /// The shell's burst origin.
    pub const SHELL: usize = 4;
}

/// fp->mv.ys.specials (Fighter +2340..+2370).
#[derive(Clone, Debug, Default)]
pub struct EggRoll {
    /// x0: frames of rolling left.
    pub remaining: i32,
    /// x4: step through the shell-closing model sequence, -1 when idle.
    pub shell_step: i32,
    /// x8: step through the landing squash, -1 when idle.
    pub squash_step: i32,
    /// xC: frames since the hitbox group last toggled.
    pub group_timer: i32,
    /// x10: ground speed when the turn began.
    pub turn_speed: f32,
    /// x14: the roll angle about part 3, kept in [0, 2pi].
    pub roll_angle: f32,
    /// x18: the turn's tilt about part 2.
    pub tilt: f32,
    /// x1C: rolling speed.
    pub speed: f32,
    /// x20: the turn's per-frame acceleration.
    pub turn_acceleration: f32,
    /// x24: the facing the turn will leave, 0 when none.
    pub pending_facing: f32,
    /// x28: frames of turning, for the dust interval.
    pub dust_timer: i32,
    /// x30: grounded collision ticks; the turn needs one.
    pub ground_ticks: i32,
}

/// The callbacks the roll installs, until the next motion change clears them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hooks {
    /// ftYs_SpecialS_8012F35C: deal_dmg_cb and x21F8 only.
    Entry,
    /// ftYoshi_SpecialS_SetCall: death2, take_dmg, deal_dmg and x21F8.
    Full,
}

pub const fn rows() -> [MotionRow; 8] {
    [
        row(
            START_GROUND,
            301,
            ground::start_anim,
            no_input,
            ground::start_physics,
            ground::start_collision,
        ),
        row(
            LOOP_GROUND,
            302,
            ground::loop_anim,
            ground::loop_input,
            ground::loop_physics,
            ground::loop_collision,
        ),
        row(
            TURN_GROUND,
            303,
            ground::turn_anim,
            ground::turn_input,
            ground::turn_physics,
            ground::turn_collision,
        ),
        row(
            END_GROUND,
            304,
            ground::end_anim,
            no_input,
            ground::end_physics,
            ground::end_collision,
        ),
        row(
            START_AIR,
            305,
            air::start_anim,
            air::start_input,
            air::start_physics,
            air::start_collision,
        ),
        row(
            LOOP_AIR,
            306,
            air::loop_anim,
            air::loop_input,
            air::loop_physics,
            air::loop_collision,
        ),
        row(
            LOOP_AIR_BOUNCE,
            307,
            air::bounce_anim,
            air::loop_input,
            air::loop_physics,
            air::bounce_collision,
        ),
        row(
            END_AIR,
            308,
            air::end_anim,
            air::end_input,
            air::end_physics,
            air::end_collision,
        ),
    ]
}

const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

fn yoshi(f: &Fighter) -> &Yoshi {
    f.character.get::<Yoshi>()
}
fn roll(f: &mut Fighter) -> &mut EggRoll {
    &mut f.character.get_mut::<Yoshi>().egg_roll
}
fn attributes(f: &Fighter) -> &crate::attributes::EggRollAttributes {
    &yoshi(f).attributes.egg_roll
}

/// ftYs_SpecialS_Enter (8012F4B4) / ftYs_SpecialAirS_Enter (8012F588): face
/// the stick, hop into the aerial start.
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.physics.facing = if f.input.current.stick.x > 0.0 {
        1.0
    } else {
        -1.0
    };
    f.change_motion_state(START_AIR, assets)
        .expect("Egg Roll assets");
    f.commands.variables = [0; 4];
    if !airborne {
        f.leave_ground();
    }
    f.step_animation(assets);
    begin(f);
    // Retail 8012F54C fmuls.
    f.physics.self_velocity.x = roll(f).speed * f.physics.facing;
    f.physics.animation_velocity.x = 0.0;
    let a = attributes(f);
    let (hop, gravity) = (a.hop_speed, a.start_gravity);
    f.physics.self_velocity.y = hop;
    f.physics.animation_velocity.y = gravity;
}

/// ftYs_SpecialS_8012F35C (8012F35C): remember the model scale, choose the
/// start speed (after the grounded entry left the ground, so always the
/// aerial one there), reset the scratch.
fn begin(f: &mut Fighter) {
    let root = f.animation.root;
    let scale = f.skeleton.get(root).scale;
    f.character.get_mut::<Yoshi>().egg_roll_scale = scale;
    let a = attributes(f);
    let mut speed = if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        a.ground_start_speed
    } else {
        a.air_start_speed
    };
    let (smash, duration) = (a.smash_speed_multiplier, a.duration);
    if smash_input(f) {
        // Retail 8012F43C fmuls.
        speed *= smash;
    }
    *roll(f) = EggRoll {
        remaining: duration,
        shell_step: -1,
        squash_step: -1,
        speed,
        ..Default::default()
    };
    set_hooks(f, Hooks::Entry);
}

/// ftYs_SpecialS_CheckButtonPressure: the stick crossed the smash threshold
/// recently (x673, as a float, below the window); the timer is consumed.
fn smash_input(f: &mut Fighter) -> bool {
    let window = attributes(f).smash_window;
    // Retail 8012F414: the u8 converts exactly, fsubs, fcmpo.
    let smash = f32::from(f.input.horizontal.held) < window;
    f.input.horizontal.held = 0;
    smash
}

fn set_hooks(f: &mut Fighter, hooks: Hooks) {
    let action = f.motion_state.action;
    f.character.get_mut::<Yoshi>().egg_roll_hooks = Some((action, hooks));
}

/// The callbacks installed in the current motion, if any: Fighter_ChangeMotionState
/// clears them (fighter.c:1377-1389), so they belong to the motion that set them.
pub fn hooks(f: &Fighter) -> Option<Hooks> {
    yoshi(f)
        .egg_roll_hooks
        .filter(|(action, _)| *action == f.motion_state.action)
        .map(|(_, hooks)| hooks)
}

/// A roll state change at `start` with `rate`, installing every callback
/// (ftYoshi_SpecialS_SetCall).
fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    start: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, flags, start, rate)?;
    set_hooks(f, Hooks::Full);
    Ok(())
}

/// ftYs_SpecialS_8012F0DC (8012F0DC): break the egg on the ground (359) or
/// in the air (363), damping the roll's speed.
fn end(
    f: &mut Fighter,
    air: bool,
    flags: MotionEntryFlags,
    start: f32,
    assets: &FighterAssets,
) -> Result<()> {
    // ftColl_8007AFF8: every hitbox off.
    f.commands.hitboxes.fill(None);
    let a = attributes(f);
    let (horizontal, vertical) = (a.end_horizontal_multiplier, a.end_vertical_multiplier);
    if air {
        change(f, END_AIR, flags, start, 1.0, assets)?;
        // Retail fmuls, no fusion.
        f.physics.self_velocity.x *= horizontal;
        f.physics.self_velocity.y *= vertical;
        f.physics.ground_acceleration = 0.0;
        f.physics.ground_velocity = 0.0;
    } else {
        change(f, END_GROUND, flags, start, 1.0, assets)?;
        f.physics.ground_velocity *= horizontal;
        f.physics.animation_velocity.y = 0.0;
        f.physics.self_velocity.y = 0.0;
    }
    model::restore(f);
    roll(f).pending_facing = 0.0;
    model::face(f);
    f.core.set_part_rotation(part::TILT, Axis::Z, 0.0);
    Ok(())
}

/// The shared tail of every loop anim: count the roll down, then break.
fn count_down(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<bool> {
    let scratch = roll(f);
    scratch.remaining -= 1;
    if scratch.remaining > 0 {
        return Ok(false);
    }
    scratch.remaining = 0;
    end(f, air, LOOP_FLAGS, 0.0, assets)?;
    Ok(true)
}

/// The loops' IASA: after the minimum roll, B breaks the egg.
fn b_breaks(f: &Fighter) -> bool {
    let a = attributes(f);
    yoshi(f).egg_roll.remaining < a.duration - a.minimum_duration
        && f.input.pressed.intersects(melee_ft::input::Buttons::B)
}

/// fn_8012EFF4 (8012EFF4), deal_dmg_cb: a landed roll hit costs roll time
/// and slows the roll toward its target speed.
pub fn hit_dealt(f: &mut Fighter, _: &FighterAssets) {
    if hooks(f).is_none() {
        return;
    }
    roll(f).group_timer = 0;
    let action = f.motion_state.action;
    if action != LOOP_AIR_BOUNCE && action != LOOP_GROUND {
        return;
    }
    let a = attributes(f);
    let (cost, deceleration) = (a.collision_frame_cost, a.hit_deceleration);
    let target = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        a.air_target_speed
    } else {
        a.ground_target_speed
    };
    let facing = f.physics.facing;
    let scratch = roll(f);
    scratch.remaining -= cost;
    if gekko_math::msl::fabsf(scratch.speed) > target {
        if scratch.speed > 0.0 {
            scratch.speed -= deceleration;
            if scratch.speed < target {
                scratch.speed = target;
            }
        } else {
            scratch.speed += deceleration;
            if scratch.speed > -target {
                scratch.speed = -target;
            }
        }
    } else if scratch.speed == 0.0 {
        scratch.speed = facing * target;
    }
}

/// fn_8012EDE8 (8012EDE8), take_dmg_cb: outside the end states the shell
/// bursts; then the model is restored.
pub fn damage_taken(f: &mut Fighter) {
    if hooks(f) != Some(Hooks::Full) {
        return;
    }
    let action = f.motion_state.action;
    if action != END_AIR && action != END_GROUND {
        model::burst_shell(f);
        f.commands
            .footstep_sounds
            .push(melee_ft::fighter::commands::FootstepSound {
                channel: melee_ft::fighter::commands::SoundChannel::Ordinary,
                id: 0x44618,
                volume: 0x7F,
                pan: 0x40,
            });
    }
    model::restore(f);
}

/// fn_8012EC7C (8012EC7C), death2_cb: restore the model.
pub fn death(f: &mut Fighter) {
    if hooks(f) == Some(Hooks::Full) {
        model::restore(f);
    }
}

/// ftCommon_8007EBAC: controller rumble, an output request only.
fn rumble(f: &mut Fighter, id: u16, duration: u16) {
    f.commands
        .rumble_requests
        .push(melee_ft::fighter::commands::RumbleRequest {
            all_players: false,
            id,
            duration,
        });
}

/// ft_PlaySFX(fp, 0x44621, 0x7F, 0x40): the roll touching down.
fn touchdown_sound(f: &mut Fighter) {
    f.commands
        .footstep_sounds
        .push(melee_ft::fighter::commands::FootstepSound {
            channel: melee_ft::fighter::commands::SoundChannel::Ordinary,
            id: 0x44621,
            volume: 0x7F,
            pan: 0x40,
        });
}

/// ftYs_SpecialS_SpawnWallBounceEffect: the bounce spark on the touched
/// wall at the ECB's side and half height, a medium quake and rumble.
fn wall_bounce_effect(f: &mut Fighter, facing_right: bool) {
    let cd = &f.collision.data;
    let mut position = f.physics.position;
    let (wall, side) = if facing_right {
        (cd.left_facing_wall.normal, cd.ecb.right.x)
    } else {
        (cd.right_facing_wall.normal, cd.ecb.left.x)
    };
    let angle = melee_lb::trigf::atan2f(-wall.x, wall.y);
    // Retail 80132020 fadds / 801320E0 fsubs.
    if facing_right {
        position.x += gekko_math::msl::fabsf(side);
    } else {
        position.x -= gekko_math::msl::fabsf(side);
    }
    // Retail 80132030 fadds, 80132054 fmadds.
    let height = gekko_math::msl::fabsf(cd.ecb.top.y + cd.ecb.bottom.y);
    position.y = gekko_math::fma::fmadds(0.5, height, position.y);
    f.effects
        .push(melee_ef::request::EffectRequest::SurfaceRebound { position, angle });
    f.core.quake_request = Some(melee_cm::QuakeKind::Medium);
    rumble(f, 0xC, 0xA);
}

/// The wall test after a box collision: the side the roll moves toward.
/// Returns whether a wall was touched, after its effect.
fn touched_wall(f: &mut Fighter, toward_right: bool) -> bool {
    use melee_types::mp::collide::{LEFT_WALL_MASK, RIGHT_WALL_MASK};
    let env = f.collision.data.env_flags as u32;
    let wall = if toward_right {
        env & LEFT_WALL_MASK != 0
    } else {
        env & RIGHT_WALL_MASK != 0
    };
    if wall {
        wall_bounce_effect(f, toward_right);
    }
    wall
}

/// A wall bounce breaks the egg in the air, reversing the horizontal speed.
fn bounce_off_wall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    end(f, true, BREAK_FLAGS, 0.0, assets)?;
    let a = attributes(f);
    let (kept, vertical) = (a.wall_bounce_multiplier, a.wall_bounce_vertical_speed);
    // Retail 80132170 fmuls by the negated attribute.
    f.physics.self_velocity.x *= -kept;
    f.physics.self_velocity.y = vertical;
    Ok(())
}

fn assets<'a>(p: &CollisionPhase<'a>) -> &'a FighterAssets {
    p.assets.expect("Egg Roll collision assets")
}

pub(crate) fn step(f: &mut Fighter, p: &AnimationPhase<'_>) {
    f.step_animation(p.assets);
}

pub(crate) fn finish_air(f: &mut Fighter, p: &PhysicsPhase<'_>) {
    f.core.finish_air_update(p.assets, p.wind);
}

pub(crate) type AnimResult = Result<Option<WaitChoice>>;
