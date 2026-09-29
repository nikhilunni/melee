//! Nana's side of a linked Squall Hammer, ftnanaspecials.c: SpecialS_0
//! (359, grounded) and SpecialS_1 (360, aerial). She copies Popo's speeds
//! and frame; while Popo spins linked and his cmd_vars[1] is clear she
//! stands on his position with his collision, otherwise she trails him.
use super::{Join, AIR_LINKED, GROUND_AIR_FLAGS, GROUND_LINKED, SPIN_BOX};
use crate::{climber, partner as link};
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, PartnerMotionChanges,
    },
};
use melee_types::GroundOrAir;

/// ftNn_MS_SpecialS_0 (359) and ftNn_MS_SpecialS_1 (360).
pub const GROUND: ActionId = ActionId(359);
pub const AIR: ActionId = ActionId(360);

/// How far Nana trails Popo per frame of trailing, up to `MAX_TRAIL`
/// frames (ftPp_SpecialS_0_Coll: 1.5, 5).
const TRAIL_STEP: f32 = 1.5;
const MAX_TRAIL: i32 = 5;

pub const fn rows() -> [MotionRow; 2] {
    [
        climber::row(
            GROUND.0,
            anim::<false>,
            climber::no_input,
            physics,
            collision::<false>,
        ),
        climber::row(
            AIR.0,
            anim::<true>,
            climber::no_input,
            physics,
            collision::<true>,
        ),
    ]
}

/// ftNn_Init_80123B10 (80123B10) is false: Nana is still in her rows.
pub fn in_rows(action: ActionId) -> bool {
    action == GROUND || action == AIR
}

/// ftPp_SpecialS_8011F964 (8011F964) is false: Popo spins linked.
fn leader_linked(action: ActionId) -> bool {
    action == GROUND_LINKED || action == AIR_LINKED
}

/// inlines of ftnanaspecials.c: take_dmg_cb and death2_cb =
/// ftNn_Init_80122FAC, and the efLib hitlag callbacks.
fn install_callbacks(f: &mut Fighter) {
    climber::vars(f).ice_callbacks = true;
    f.effect_state.hitlag_callbacks = true;
}

/// ftPp_SpecialS_0_Anim (80123CA4) / ftPp_SpecialS_1_Anim (80123D68):
/// while Popo spins linked, Nana takes his frame; otherwise both leave
/// each other's hitlag (ftNn_Init_801238E4) and she ends in Wait, or in
/// the air in Fall or FallSpecial (attribute x12C's landing lag).
fn anim<const AERIAL: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let popo = link::view(f);
    if leader_linked(popo.action) {
        f.animation.frame = popo.frame;
        return Ok(None);
    }
    link::separate(f);
    climber::vars(f).ice_callbacks = false;
    f.effect_state.hitlag_callbacks = false;
    if AERIAL {
        let lag = climber::attributes(f).partner_squall_landing_lag;
        super::fall(f, lag, p.assets)?;
    } else {
        climber::finish(f, p.assets, false)?;
    }
    Ok(None)
}

/// ftPp_SpecialS_0_Phys (80123E60) / ftPp_SpecialS_1_Phys (80123EFC):
/// Popo's speeds, accelerations and facing, and the root turned to it
/// (ftPartSetRotY(M_PI_2 * facing): fmul in double, frsp).
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let popo = link::view(f);
    let physics = &mut f.core.physics;
    physics.self_velocity = popo.self_velocity;
    physics.animation_velocity = popo.animation_velocity;
    physics.ground_velocity = popo.ground_velocity;
    physics.ground_acceleration = popo.ground_acceleration;
    physics.facing = popo.facing;
    face(f);
    super::finish_update(f, &p);
}

/// ftPartSetRotY(fp, 0, M_PI_2 * facing).
fn face(f: &mut Fighter) {
    let angle = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
    f.core.set_part_rotation(0, Axis::Y, angle);
}

/// ftPp_SpecialS_0_Coll (80123F98) / ftPp_SpecialS_1_Coll (801241A0):
/// following a linked Popo happens after this proc (`follow`, which needs
/// his collision); otherwise Nana trails him by up to five steps
/// (fnmsubs) and leaves or reaches the floor on her own.
fn collision<const AERIAL: bool>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let popo = link::view(f);
    if leader_linked(popo.action) && popo.command_1 == 0 {
        link::work(f).follow = true;
        return Ok(());
    }
    let trail = {
        let payload = climber::payload(f);
        payload.trail = (payload.trail + 1).min(MAX_TRAIL);
        payload.trail
    };
    // 80124098..801240A0: the count to float, fmuls by the facing, then
    // fnmsubs against Popo's x.
    let steps = trail as f32 * f.physics.facing;
    f.physics.position.x = gekko_math::fma::fnmsubs(TRAIL_STEP, steps, popo.position.x);
    let assets = p.assets.expect("Squall Hammer partner assets");
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if AERIAL {
        let landed = air::collide_box(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            SPIN_BOX,
        );
        if landed {
            f.commands.variables = [0; 4];
            install_callbacks(f);
            f.land();
            f.physics.self_velocity.y = 0.0;
            switch_row(f, GROUND, assets)?;
            install_callbacks(f);
        }
    } else {
        let supported = ground::collide_box(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            SPIN_BOX,
            false,
        );
        if !supported {
            f.commands.variables = [0; 4];
            install_callbacks(f);
            f.core.leave_ground();
            switch_row(f, AIR, assets)?;
            install_callbacks(f);
        }
    }
    finish_collision(f, popo.root_rotation_x);
    Ok(())
}

/// Fighter_ChangeMotionState(gobj, row, 0x0C4C528A, cur_anim_frame, 1, 0).
fn switch_row(f: &mut Fighter, row: ActionId, assets: &FighterAssets) -> Result<()> {
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(row, assets, GROUND_AIR_FLAGS, frame, 1.0)
}

/// Both collisions' tail: Popo's root X angle (ftPartGetRotX) and the
/// callbacks.
fn finish_collision(f: &mut Fighter, root_rotation_x: f32) {
    f.core.set_part_rotation(0, Axis::X, root_rotation_x);
    install_callbacks(f);
}

/// ftPp_SpecialS_0_Coll's first branch for `nana`, after her proc: she
/// stands on Popo's position with his collision (ft_800849EC); a change of
/// ground state switches her row at her frame, then takes his.
pub fn follow(
    nana: &mut Fighter,
    popo: &Fighter,
    assets: &FighterAssets,
) -> Result<PartnerMotionChanges> {
    nana.physics.position = popo.physics.position;
    melee_mp::copy_coll_data(&popo.collision.data, &mut nana.core.collision.data, 2);
    let mut changes = PartnerMotionChanges::default();
    if nana.physics.ground_or_air != popo.physics.ground_or_air {
        let row = if nana.motion_state.action == GROUND {
            AIR
        } else {
            GROUND
        };
        switch_row(nana, row, assets)?;
        changes.fighter = true;
    }
    nana.physics.ground_or_air = popo.physics.ground_or_air;
    // Fighter_procMap's translate, which ran before this.
    let root = nana.animation.root;
    let position = nana.physics.position;
    nana.skeleton.set_translate(root, &position);
    finish_collision(nana, popo.core.part_rotation_x(0));
    Ok(changes)
}

/// ftNn_Init_80123954's success path for `nana` (after Popo's entry): she
/// leaves or takes the floor as Popo stood, enters her row
/// (ftNn_Init_80123B3C / 80123BF0), and takes his position, speeds and
/// facing; x1A5C links her to him.
pub fn join(nana: &mut Fighter, popo: &Fighter, join: Join, assets: &FighterAssets) -> Result<()> {
    let row = if join.ground_or_air == GroundOrAir::Air {
        if nana.physics.ground_or_air != GroundOrAir::Air {
            nana.core.leave_ground();
        }
        AIR
    } else {
        if nana.physics.ground_or_air != GroundOrAir::Ground {
            melee_mp::copy_coll_data(&popo.collision.data, &mut nana.core.collision.data, 2);
            nana.land();
        }
        GROUND
    };
    enter_row(nana, row, assets)?;
    climber::payload(nana).trail = 0;
    let physics = &mut nana.core.physics;
    physics.position = join.position;
    physics.self_velocity = join.self_velocity;
    physics.ground_velocity = join.ground_velocity;
    physics.facing = join.facing;
    face(nana);
    nana.core.combat.hitlag_link.partner = Some(join.leader);
    Ok(())
}

/// ftNn_Init_80123B3C (80123B3C) / ftNn_Init_80123BF0 (80123BF0).
fn enter_row(nana: &mut Fighter, row: ActionId, assets: &FighterAssets) -> Result<()> {
    nana.commands.variables = [0; 4];
    install_callbacks(nana);
    nana.change_motion_state(row, assets)?;
    install_callbacks(nana);
    // ftAnim_8006EBA4.
    nana.step_animation(assets);
    nana.physics.animation_velocity.y = 0.0;
    nana.physics.self_velocity.y = 0.0;
    Ok(())
}
