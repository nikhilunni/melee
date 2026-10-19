//! Chef: ftgamewatchspecialn.c (8014E4F0..8014EBFC) and Kirby's copy,
//! ftkirbyspecialgamewatch.c (8010CE5C..8010D740). The two files differ
//! only in what [`ChefUser`] names: the rows, the food's item kind, the
//! attributes and scratch, the motion-change flags and the accessory the
//! entry installs.
//!
//! The script raises cmd_vars[0] on the frame the pan flips; accessory4
//! (ftGw_SpecialN_CreateSausage / fn_8010CE5C) then throws one of five
//! foods, never one of the last two thrown (HSD_Randi over the three that
//! remain). While B is held the script's cmd_vars[2] restarts the motion at
//! the loop frame; once released, a B press does while cmd_vars[1] is up. A
//! use throws at most the attribute's count of foods.
use crate::motion;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// The food's offset from the left thumb (ftgamewatchspecialn.c:47-49,
/// ftkirbyspecialgamewatch.c:59-61).
const PAN_OFFSET: Vec3 = Vec3::new(2.5, 6.5, 0.0);

/// The foods a throw chooses between (itGamewatchchefAttributes' entries).
pub const FOODS: usize = 5;

/// Chef's two attribute words (G&W +18/+1C, Kirby +3E4/+3E8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChefAttributes {
    /// x18_GAMEWATCH_CHEF_LOOPFRAME / specialn_gw_frame_on_repeat: the
    /// frame the loop restarts at.
    pub loop_frame: f32,
    /// x1C_GAMEWATCH_CHEF_MAX / specialn_gw_max_sausages_per_use: foods per
    /// use, as a float.
    pub max_sausages: f32,
}

/// fp->mv.gw.SpecialN / fp->mv.kb.specialn_gw.
#[derive(Clone, Copy, Debug, Default)]
pub struct Chef {
    /// isChefLoopDisable: B was released since the motion (re)started.
    pub loop_disabled: bool,
    /// maxSausage: foods thrown this use.
    pub thrown: i32,
}

/// Who cooks: Mr. Game & Watch, and Kirby with his copied ability. Kirby's
/// implementation is a marker type whose methods reach into Kirby's state.
pub trait ChefUser: 'static {
    /// SpecialN; SpecialAirN is the next row.
    const GROUND: ActionId;
    const AIR: ActionId;
    /// SpecialN's submotion; SpecialAirN's is the next.
    const GROUND_ANIMATION: i32;
    /// It_Kind_GameWatch_Chef / It_Kind_Kirby_GameWatchChef.
    const FOOD: ItemKind;
    /// The flags of the ground/air changes and of the loop's restart.
    const TRANSITION_FLAGS: u32;
    fn attributes(f: &Fighter) -> ChefAttributes;
    fn scratch(f: &mut Fighter) -> &mut Chef;
    /// The last food thrown and the one before it (u.gw.x2240 / x2244,
    /// u.kb.xD4 / xD8).
    fn foods(f: &mut Fighter) -> (&mut i32, &mut i32);
    /// accessory4_cb as the entry leaves it (SetVars).
    fn install_entry_accessory(f: &mut Fighter);
    /// accessory4_cb = the food's thrower.
    fn install_thrower(f: &mut Fighter);
    /// accessory4_cb = NULL.
    fn uninstall(f: &mut Fighter);
}

/// The two rows for `C`.
pub const fn rows<C: ChefUser>() -> [MotionRow; 2] {
    [
        row(
            C::GROUND,
            C::GROUND_ANIMATION,
            anim::<C>,
            ground_physics,
            ground_collision::<C>,
            input::<C>,
        ),
        row(
            C::AIR,
            C::GROUND_ANIMATION + 1,
            anim::<C>,
            motion::fall_without_drift,
            air_collision::<C>,
            input::<C>,
        ),
    ]
}

const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
    iasa: melee_ft::fighter::state::InputFn,
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

fn is_air<C: ChefUser>(f: &Fighter) -> bool {
    f.motion_state.action == C::AIR
}

/// `maxSausage < max` (the int converted to float).
fn may_throw<C: ChefUser>(f: &mut Fighter) -> bool {
    let max = C::attributes(f).max_sausages;
    (C::scratch(f).thrown as f32) < max
}

/// ftGw_SpecialN_Enter (8014E644) / ftGw_SpecialAirN_Enter (8014E6C0) and
/// ftKb_SpecialNGw_8010D188 / 8010D204.
pub fn enter<C: ChefUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(if air { C::AIR } else { C::GROUND }, a)
        .expect("Chef assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // ftGameWatch_SpecialN_SetVars / setGwVars.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.commands.variables[2] = 0;
    *C::scratch(f) = Chef::default();
    C::install_entry_accessory(f);
}

/// ftGw_SpecialN_CreateSausage (8014E4F0) / fn_8010CE5C (8010CE5C): on the
/// script's flag, one more food while the use has throws left; the
/// accessory then uninstalls.
pub fn create_sausage<C: ChefUser>(
    f: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) {
    if f.commands.variables[0] == 0 {
        return;
    }
    f.commands.variables[0] = 0;
    if may_throw::<C>(f) {
        C::scratch(f).thrown += 1;
        let bone = usize::from(assets.parts.joint(FtPart::LThumbNb).expect("LThumbNb part"));
        let c = &mut f.core;
        let pan = melee_ft::fighter::caches::part_position(
            &mut c.skeleton,
            &c.animation,
            bone,
            PAN_OFFSET,
        );
        let (last, previous) = C::foods(f);
        let food = draw_food(last, previous, rng);
        throw_food(f, C::FOOD, pan, food);
    }
    C::uninstall(f);
}

/// The foods other than the last two thrown, in order; HSD_Randi picks one,
/// which becomes the last thrown (and the last one the one before it).
pub fn draw_food(last: &mut i32, previous: &mut i32, rng: &mut gekko_math::HsdRng) -> i32 {
    let mut choices = [0; FOODS];
    let mut count = 0;
    for food in 0..FOODS as i32 {
        if food != *last && food != *previous {
            choices[count] = food;
            count += 1;
        }
    }
    let food = choices[rng.randi(count as i32) as usize];
    *previous = *last;
    *last = food;
    food
}

/// it_802C837C (802C837C): prev_pos is the pan on the stage plane; pos is
/// it_8026BB68's ECB midpoint (ftLib_80086990, retail 800869AC..BC: fadds,
/// fmuls, fadds).
fn throw_food(f: &mut Fighter, kind: ItemKind, pan: Vec3, food: i32) {
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(kind, c.player.id, pan, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = food;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftGw_SpecialN_Loop (8014EB1C) / ftGw_SpecialAirN_Loop (8014EB8C) and
/// ftKb_SpecialNGw_8010D660 / 8010D6D0: the motion again one frame before
/// the loop frame, keeping the count.
fn restart<C: ChefUser>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let frame = C::attributes(f).loop_frame - 1.0;
    let state = f.motion_state.action;
    motion::change(f, state, C::TRANSITION_FLAGS, frame, assets)?;
    f.step_animation(assets);
    f.commands.variables[1] = 0;
    f.commands.variables[2] = 0;
    C::scratch(f).loop_disabled = false;
    C::install_thrower(f);
    Ok(())
}

/// ftGw_SpecialN_Anim (8014E73C) / ftGw_SpecialAirN_Anim (8014E7DC) and
/// ftKb_GwSpecialN_Anim / ftKb_GwSpecialAirN_Anim: the script's cmd_vars[2]
/// loops while B has stayed held and throws remain; the animation's end is
/// Wait (ft_8008A2BC) or Fall.
fn anim<C: ChefUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[2] != 0 {
        f.commands.variables[2] = 0;
        if may_throw::<C>(f) && !C::scratch(f).loop_disabled {
            restart::<C>(f, p.assets)?;
        }
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = is_air::<C>(f);
        motion::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialN_IASA (8014E87C) / ftGw_SpecialAirN_IASA (8014E900) and
/// ftKb_GwSpecialN_IASA / ftKb_GwSpecialAirN_IASA: releasing B ends the
/// held loop; a B press while cmd_vars[1] is up throws again.
fn input<C: ChefUser>(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        C::scratch(f).loop_disabled = true;
    }
    if f.commands.variables[1] != 0 && f.input.pressed.intersects(Buttons::B) && may_throw::<C>(f) {
        restart::<C>(f, p.assets).expect("Chef loop assets");
    }
}

/// ftGw_SpecialN_Phys (8014E984) / ftKb_GwSpecialN_Phys: ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftGw_SpecialN_Coll (8014E9C4) / ftKb_GwSpecialN_Coll: off the floor
/// (ft_800827A0), the aerial row (ftGw_SpecialN_GroundToAir 8014EA3C,
/// ftKb_SpecialNGw_8010D580) with the thrower installed.
fn ground_collision<C: ChefUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if motion::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chef collision assets");
    motion::ground_to_air(f, C::AIR, C::TRANSITION_FLAGS, assets)?;
    C::install_thrower(f);
    Ok(())
}

/// ftGw_SpecialAirN_Coll (8014EA00) / ftKb_GwSpecialAirN_Coll: landing
/// (ft_80081D0C), the grounded row (ftGw_SpecialAirN_AirToGround 8014EAAC,
/// ftKb_SpecialNGw_8010D5F0) with the thrower installed.
fn air_collision<C: ChefUser>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !motion::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chef landing assets");
    motion::air_to_ground(f, C::GROUND, C::TRANSITION_FLAGS, assets)?;
    C::install_thrower(f);
    Ok(())
}
