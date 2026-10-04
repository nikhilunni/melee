//! Chef, ftgamewatchspecialn.c (8014E4F0..8014EBFC).
//!
//! The script raises cmd_vars[0] on the frame the pan flips; accessory4
//! (ftGw_SpecialN_CreateSausage) then throws one of five foods, never one
//! of the last two thrown (HSD_Randi over the three that remain). While B
//! is held the script's cmd_vars[2] restarts the motion at the loop frame;
//! once released, a B press does while cmd_vars[1] is up. A use throws at
//! most x1C_GAMEWATCH_CHEF_MAX foods.
use crate::{
    articles::{self, Accessory},
    common::{self, flags, row},
    init::GameWatch,
};
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
use melee_types::{FtPart, ItemKind};

/// ftGw_MS_SpecialN (353) and ftGw_MS_SpecialAirN (354).
pub const GROUND: ActionId = ActionId(353);
pub const AIR: ActionId = ActionId(354);

pub const fn rows() -> [MotionRow; 2] {
    [
        row(GROUND, anim, input, ground_physics, ground_collision),
        row(AIR, anim, input, common::fall_without_drift, air_collision),
    ]
}

/// transition_flags (ftgamewatchspecialn.c:228): the ground/air set with
/// KeepColAnimHitStatus, SkipHit and SkipModel.
const TRANSITION_FLAGS: u32 =
    flags::GROUND_AIR | flags::KEEP_COL_ANIM_HIT_STATUS | flags::SKIP_HIT | flags::SKIP_MODEL;

/// The food's offset from the left thumb (ftgamewatchspecialn.c:47-49).
const PAN_OFFSET: Vec3 = Vec3::new(2.5, 6.5, 0.0);

/// The foods a throw chooses between (itGamewatchchefAttributes' entries).
const FOODS: usize = 5;

/// fp->mv.gw.SpecialN.
#[derive(Clone, Copy, Debug, Default)]
pub struct Chef {
    /// isChefLoopDisable: B was released since the motion (re)started.
    pub loop_disabled: bool,
    /// maxSausage: foods thrown this use.
    pub thrown: i32,
}

fn chef(f: &mut Fighter) -> &mut Chef {
    &mut f.character.get_mut::<GameWatch>().chef
}

fn is_air(f: &Fighter) -> bool {
    f.motion_state.action == AIR
}

/// `maxSausage < x1C_GAMEWATCH_CHEF_MAX` (the int converted to float).
fn may_throw(f: &Fighter) -> bool {
    let gw = f.character.get::<GameWatch>();
    (gw.chef.thrown as f32) < gw.attributes.chef.max_sausages
}

/// ftGw_SpecialN_Enter (8014E644) / ftGw_SpecialAirN_Enter (8014E6C0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Chef assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // ftGameWatch_SpecialN_SetVars.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.commands.variables[2] = 0;
    *chef(f) = Chef::default();
    articles::install(f, Accessory::CreateSausage);
}

/// ftGw_SpecialN_CreateSausage (8014E4F0): on the script's flag, one more
/// food while the use has throws left; the accessory then uninstalls.
pub fn create_sausage(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    if f.commands.variables[0] == 0 {
        return;
    }
    f.commands.variables[0] = 0;
    if may_throw(f) {
        chef(f).thrown += 1;
        let bone = usize::from(assets.parts.joint(FtPart::LThumbNb).expect("LThumbNb part"));
        let c = &mut f.core;
        let pan = melee_ft::fighter::caches::part_position(
            &mut c.skeleton,
            &c.animation,
            bone,
            PAN_OFFSET,
        );
        let food = draw_food(f.character.get_mut::<GameWatch>(), rng);
        throw_food(f, pan, food);
    }
    articles::uninstall(f);
}

/// The foods other than the last two thrown, in order; HSD_Randi picks one,
/// which becomes the last thrown (x2240_chefVar1, then x2244_chefVar2).
pub fn draw_food(gw: &mut GameWatch, rng: &mut gekko_math::HsdRng) -> i32 {
    let mut choices = [0; FOODS];
    let mut count = 0;
    for food in 0..FOODS as i32 {
        if food != gw.chef_last && food != gw.chef_previous {
            choices[count] = food;
            count += 1;
        }
    }
    let food = choices[rng.randi(count as i32) as usize];
    gw.chef_previous = gw.chef_last;
    gw.chef_last = food;
    food
}

/// it_802C837C (802C837C): prev_pos is the pan on the stage plane; pos is
/// it_8026BB68's ECB midpoint (ftLib_80086990, retail 800869AC..BC: fadds,
/// fmuls, fadds).
fn throw_food(f: &mut Fighter, pan: Vec3, food: i32) {
    let c = &mut f.core;
    let mut spawn = SpawnItem::ray(ItemKind::GameWatchChef, c.player.id, pan, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = food;
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftGw_SpecialN_Loop (8014EB1C) / ftGw_SpecialAirN_Loop (8014EB8C): the
/// motion again one frame before the loop frame, keeping the count.
fn restart(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let frame = f.character.get::<GameWatch>().attributes.chef.loop_frame - 1.0;
    let state = f.motion_state.action;
    common::change(f, state, TRANSITION_FLAGS, frame, assets)?;
    f.step_animation(assets);
    f.commands.variables[1] = 0;
    f.commands.variables[2] = 0;
    chef(f).loop_disabled = false;
    articles::install(f, Accessory::CreateSausage);
    Ok(())
}

/// ftGw_SpecialN_Anim (8014E73C) / ftGw_SpecialAirN_Anim (8014E7DC): the
/// script's cmd_vars[2] loops while B has stayed held and throws remain;
/// the animation's end is Wait or Fall.
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[2] != 0 {
        f.commands.variables[2] = 0;
        if may_throw(f) && !chef(f).loop_disabled {
            restart(f, p.assets)?;
        }
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = is_air(f);
        common::finish(f, air, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialN_IASA (8014E87C) / ftGw_SpecialAirN_IASA (8014E900):
/// releasing B ends the held loop; a B press while cmd_vars[1] is up
/// throws again.
fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        chef(f).loop_disabled = true;
    }
    if f.commands.variables[1] != 0 && f.input.pressed.intersects(Buttons::B) && may_throw(f) {
        restart(f, p.assets).expect("Chef loop assets");
    }
}

/// ftGw_SpecialN_Phys (8014E984): ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftGw_SpecialN_Coll (8014E9C4): off the floor (ft_800827A0),
/// ftGw_SpecialN_GroundToAir (8014EA3C).
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chef collision assets");
    common::ground_to_air(f, AIR, TRANSITION_FLAGS, assets)?;
    articles::install(f, Accessory::CreateSausage);
    Ok(())
}

/// ftGw_SpecialAirN_Coll (8014EA00): landing (ft_80081D0C),
/// ftGw_SpecialAirN_AirToGround (8014EAAC).
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chef landing assets");
    common::air_to_ground(f, GROUND, TRANSITION_FLAGS, assets)?;
    articles::install(f, Accessory::CreateSausage);
    Ok(())
}
