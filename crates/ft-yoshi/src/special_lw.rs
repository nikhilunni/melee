//! Yoshi Bomb, ftyoshispeciallw.c (8012E644..8012EB48).
use crate::init::Yoshi;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags, MotionRow,
    },
};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// ftYs_MS_SpecialLw (366): the grounded start, entered airborne.
pub const SPECIAL_LW: ActionId = ActionId(366);
/// ftYs_MS_SpecialLwLanding (367).
pub const SPECIAL_LW_LANDING: ActionId = ActionId(367);
/// ftYs_MS_SpecialAirLw (368): the aerial start and the descent.
pub const SPECIAL_AIR_LW: ActionId = ActionId(368);

/// ftYs_SpecialS_8012EA04: the grounded start's hand-off to the descent,
/// SkipHit | SkipMatAnim | SkipColAnim | UpdateCmd | SkipItemVis | Unk19 |
/// SkipModelPartVis | SkipModelFlags | Unk27.
const DESCENT_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5088);
/// ftYs_SpecialS_8012EA04's start frame for the descent animation.
const DESCENT_START_FRAME: f32 = 20.0;

/// ftYs_Init_MotionStateTable[25..28]: ftYs_SM_SpecialLw (311),
/// SpecialLwLanding (312) and SpecialAirLw (313).
pub const fn rows() -> [MotionRow; 3] {
    [
        row(SPECIAL_LW, 311, start_anim, start_physics, collision),
        row(
            SPECIAL_LW_LANDING,
            312,
            landing_anim,
            landing_physics,
            landing_collision,
        ),
        row(SPECIAL_AIR_LW, 313, air_anim, air_physics, collision),
    ]
}

const fn row(
    action: ActionId,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim,
        iasa: no_input,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftYs_SpecialLw_Enter (8012E6E0) / ftYs_SpecialAirLw_Enter (8012E774).
/// The grounded entry leaves the ground first (ftCommon_8007D5D4).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    let state = if airborne {
        SPECIAL_AIR_LW
    } else {
        f.leave_ground();
        SPECIAL_LW
    };
    f.change_motion_state(state, assets)
        .expect("Yoshi Bomb assets");
    f.step_animation(assets);
    // ftYoshi_SpecialLw_SetVars, after the first animation step: the
    // frame-zero commands' variables and TransN's vertical delta are dropped.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.commands.throw_accessory = false;
    f.physics.self_velocity.x = 0.0;
    f.physics.self_velocity.y = 0.0;
    if let Some(root) = &mut f.animation.root_motion {
        root.primary_history.offset.y = 0.0;
    }
    // x2223_b4 disables ft_80081A00's item landing, which is not modelled.
}

/// ftYs_SpecialLw_Anim (8012E7F8): the start becomes the descent at frame 20.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state_with_flags(
            SPECIAL_AIR_LW,
            p.assets,
            DESCENT_FLAGS,
            DESCENT_START_FRAME,
            1.0,
        )?;
    }
    Ok(None)
}

/// ftYs_SpecialAirLw_Anim (8012E838): the finished animation forces the
/// terminal fall speed (cmd_vars[1]).
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.commands.variables[1] = 1;
    }
    Ok(None)
}

/// ft_80085134 (80085134): TransN's delta drives both velocity axes.
fn root_motion_velocity(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Yoshi Bomb TransN")
        .primary_history
        .offset;
    // Retail 80085140 fmuls.
    f.physics.self_velocity.x = offset.z * f.physics.facing;
    f.physics.self_velocity.y = offset.y;
}

/// ftYs_SpecialLw_Phys (8012E870): the hop never descends.
fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_velocity(f);
    if f.physics.self_velocity.y < 0.0 {
        f.physics.self_velocity.y = 0.0;
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftYs_SpecialAirLw_Phys (8012E8C0): at least the attribute fall speed,
/// exactly it once the animation has ended.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    root_motion_velocity(f);
    let fall_speed = f
        .character
        .get::<Yoshi>()
        .attributes
        .ground_pound
        .fall_speed;
    if f.physics.self_velocity.y < fall_speed || f.commands.variables[1] != 0 {
        f.physics.self_velocity.y = fall_speed;
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftYs_SpecialAirLw_Coll (8012E91C), also ftYs_SpecialLw_Coll: once the
/// script arms the landing (cmd_vars[0]) a falling touch lands and a ledge
/// can be caught; before that, a floor touch keeps Yoshi airborne.
fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    // ft_CheckGroundAndLedge with the facing direction.
    let touched = air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    );
    let armed = f.commands.variables[0] != 0;
    let assets = p.assets.expect("Yoshi Bomb collision assets");
    if touched {
        if armed && f.physics.self_velocity.y <= 0.0 {
            land(f, assets)?;
        } else {
            f.leave_ground();
        }
    } else if armed {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// ftYs_SpecialS_8012EAD8 (8012EAD8): land, and spawn the stars from
/// accessory4 (fn_8012E644).
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.physics.ground_velocity = 0.0;
    f.land();
    f.change_motion_state(SPECIAL_LW_LANDING, assets)?;
    f.character.get_mut::<Yoshi>().stars_pending = true;
    f.arm_accessory4();
    Ok(())
}

/// ftYs_SpecialLwLanding_Anim (8012EA4C): Wait at the end (ft_8008A2BC).
fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}

/// ftYs_SpecialLwLanding_Phys (8012EA88): ft_80084F3C.
fn landing_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftYs_SpecialLwLanding_Coll (8012EAA8): Fall when the floor is lost.
fn landing_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        let assets = p.assets.expect("Yoshi Bomb landing assets");
        f.change_motion_state(CommonMotionState::Fall.into(), assets)?;
    }
    Ok(())
}

/// fn_8012E644 (8012E644): two stars from TransN, one each way.
pub fn spawn_stars(f: &mut Fighter, assets: &FighterAssets) {
    let pending = std::mem::take(&mut f.character.get_mut::<Yoshi>().stars_pending);
    if !f.run_accessory4(pending) {
        return;
    }
    let offset = f
        .character
        .get::<Yoshi>()
        .attributes
        .ground_pound
        .star_offset;
    let part = usize::from(assets.parts.joint(FtPart::TransN).expect("Yoshi TransN"));
    let c = &mut f.core;
    let transn = melee_ft::fighter::caches::bone_position(
        &mut c.skeleton,
        c.animation.root,
        part,
        hsd_types::Vec3::ZERO,
    );
    // Retail 8012E68C fmadds (-1.0 * x + transn.x), 8012E6A4 fadds.
    let left = hsd_types::Vec3::new(
        gekko_math::fma::fmadds(-1.0, offset.x, transn.x),
        transn.y + offset.y,
        transn.z,
    );
    let right = hsd_types::Vec3::new(transn.x + offset.x, left.y, left.z);
    for (position, facing) in [(left, -1.0), (right, 1.0)] {
        request_star(f, position, facing);
    }
}

/// it_802B2FC8 (802B2FC8): prev_pos is the star position; the spawn sweep
/// starts at the fighter's ECB centre (it_8026BB68 -> ftLib_80086990).
fn request_star(f: &mut Fighter, position: hsd_types::Vec3, facing: f32) {
    let mut spawn = melee_it::SpawnItem::ray(ItemKind::YoshiStar, f.player.id, position, facing);
    spawn.previous_position = position;
    // Retail 800869AC..BC: fadds, fmuls, fadds.
    let ecb = &f.collision.data.ecb;
    let midpoint = 0.5 * (ecb.top.y + ecb.bottom.y);
    spawn.position = hsd_types::Vec3::new(
        f.physics.position.x + 0.0,
        f.physics.position.y + midpoint,
        f.physics.position.z + 0.0,
    );
    f.core
        .item_requests
        .push(melee_it::ItemRequest::Spawn(spawn));
}
