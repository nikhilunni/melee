//! Judgment, ftgamewatchspecials.c (8014C46C..8014CBF4).
//!
//! The entry draws a face (HSD_Randi over the enabled faces other than the
//! last two drawn) and enters that face's row: each of the nine has its own
//! script, and so its own hitboxes and effects (1's recoil, 6's flame, 8's
//! freeze, 9's launch are all the scripts'). On the script's cmd_vars[1]
//! the accessory puts the numbered sign in the right hand; face 7
//! (x222C == 6) also asks for food (it_8028FAF4), which needs items on.
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
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftGw_MS_SpecialS1 (355) and ftGw_MS_SpecialAirS1 (364); a face's row is
/// the first plus x222C_judgeVar1.
pub const GROUND_FIRST: u16 = 355;
pub const AIR_FIRST: u16 = 364;
/// The faces, 1 to 9.
pub const FACES: usize = crate::attributes::JUDGE_FACES;
/// x222C of face 7, whose sign brings food.
const FOOD_FACE: i32 = 6;

pub const fn rows() -> [MotionRow; 2 * FACES] {
    let mut rows = [row(
        ActionId(GROUND_FIRST),
        ground_anim,
        common::no_input,
        ground_physics,
        ground_collision,
    ); 2 * FACES];
    let mut face = 0;
    while face < FACES {
        rows[face] = row(
            ActionId(GROUND_FIRST + face as u16),
            ground_anim,
            common::no_input,
            ground_physics,
            ground_collision,
        );
        rows[FACES + face] = row(
            ActionId(AIR_FIRST + face as u16),
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        );
        face += 1;
    }
    rows
}

/// transition_flags (ftgamewatchspecials.c:287): as Chef's.
const TRANSITION_FLAGS: u32 =
    flags::GROUND_AIR | flags::KEEP_COL_ANIM_HIT_STATUS | flags::SKIP_HIT | flags::SKIP_MODEL;

fn gw(f: &mut Fighter) -> &mut GameWatch {
    f.character.get_mut::<GameWatch>()
}

/// ftData_SpecialS / ftData_SpecialAirS: the entry draws from the RNG to
/// choose its row, so it runs once the entering IASA returns
/// (`Fighter::finish_input`), where nothing in between draws.
pub fn request(f: &mut Fighter, air: bool) {
    gw(f).pending_judgement = Some(air);
}

/// ftGw_SpecialS_Enter (8014C7A0) / ftGw_SpecialAirS_Enter (8014C828,
/// 8014C858: fdivs).
pub fn enter_pending(
    f: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let Some(air) = gw(f).pending_judgement.take() else {
        return Ok(());
    };
    if air {
        f.physics.self_velocity.x /= gw(f).attributes.judge.momentum_preserve;
    } else {
        f.physics.self_velocity.y = 0.0;
    }
    let face = draw_face(gw(f), rng);
    let first = if air { AIR_FIRST } else { GROUND_FIRST };
    f.change_motion_state(ActionId(first + face as u16), assets)?;
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    // ftGameWatch_SpecialS_SetVars.
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    articles::install(f, Accessory::JudgementSetup);
    Ok(())
}

/// ftGw_SpecialS_GetRandomInt (8014C6B4): the faces other than the last two
/// drawn, each weighted by its attribute switch; HSD_Randi over the total
/// picks one, which becomes the last drawn (x222C, then x2230).
pub fn draw_face(gw: &mut GameWatch, rng: &mut gekko_math::HsdRng) -> i32 {
    let mut faces = [0; FACES];
    let mut ceilings = [0; FACES];
    let mut count = 0;
    let mut total = 0;
    for face in 0..FACES as i32 {
        if face != gw.judge_last && face != gw.judge_previous {
            faces[count] = face;
            total += i32::from(gw.attributes.judge.enabled[face as usize]);
            ceilings[count] = total;
            count += 1;
        }
    }
    let draw = rng.randi(total);
    let Some(index) = (0..count).find(|&i| draw < ceilings[i]) else {
        unimplemented!(
            "ftGw_SpecialS_GetRandomInt (ftgamewatchspecials.c:135-143): no face enabled, result uninitialised"
        );
    };
    gw.judge_previous = gw.judge_last;
    gw.judge_last = faces[index];
    faces[index]
}

/// ftGw_SpecialS_ItemJudgementSetup (8014C46C): on the script's flag, the
/// sign in the right hand showing the face (it_802C7774); the accessory
/// then uninstalls.
pub fn judgement_setup(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[1] == 0 {
        return;
    }
    f.commands.variables[1] = 0;
    let part = assets.parts.joint(FtPart::RThumbNb).expect("RThumbNb part");
    let c = &mut f.core;
    let hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(part),
        Vec3::ZERO,
    );
    let face = f.character.get::<GameWatch>().judge_last;
    let c = &mut f.core;
    let mut spawn = SpawnItem::attached(
        ItemKind::GameWatchJudge,
        c.player.id,
        hand,
        c.physics.facing,
    );
    spawn.spawn_argument = face;
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    if face == FOOD_FACE {
        // it_8028FAF4 returns NULL unless food is switched on
        // (it_8026D324: the item frequency is not "none"). The port's
        // matches run with items off.
    }
    gw(f).articles.judgement = true;
    articles::install_callbacks(f, ItemKind::GameWatchJudge);
    articles::uninstall(f);
}

/// ftGw_SpecialS_ItemJudgementRemove (8014C5CC): it_802C7A84 destroys the
/// sign, which lets go of its owner.
pub fn remove_judgement(f: &mut Fighter) {
    if gw(f).articles.judgement {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::GameWatchJudge,
            control: ItemControl::Remove,
        });
        articles::destroyed(f, ItemKind::GameWatchJudge);
    }
}

/// ftGw_SpecialS_Anim (8014C8BC).
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialAirS_Anim (8014C8F8).
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, true, p.assets)?;
    }
    Ok(None)
}

/// ftGw_SpecialS_Phys (8014C93C): the script's swing flag advances
/// (cmd_vars[0] 1 becomes 2), then ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] == 1 {
        f.commands.variables[0] = 2;
    }
    callbacks::physics::guard_on(f, p);
}

/// ftGw_SpecialAirS_Phys (8014C974): before the swing ftCommon_FallBasic;
/// from it the move's own gravity and terminal speed, with one hop per
/// airtime (x2234) on the swing's frame; then the move's air friction.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = gw(f).attributes.judge.clone();
    let swing = f.commands.variables[0];
    if swing >= 1 {
        if swing == 1 {
            f.commands.variables[0] = 2;
            if gw(f).x2234 == 0 {
                gw(f).x2234 = 1;
                f.physics.self_velocity.y = a.hop;
            } else {
                f.physics.self_velocity.y = 0.0;
            }
        }
        common::fall(f, a.gravity, a.terminal_velocity);
    } else {
        common::fall_basic(f);
    }
    common::air_friction(f, a.air_friction);
    common::finish_update(f, &p);
}

/// ftGw_SpecialS_Coll (8014CA10): off the floor (ft_800827A0),
/// ftGw_SpecialS_GroundToAir (8014CAA0).
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Judgment collision assets");
    let face = gw(f).judge_last as u16;
    common::ground_to_air(f, ActionId(AIR_FIRST + face), TRANSITION_FLAGS, assets)?;
    if f.commands.variables[0] == 1 {
        f.commands.variables[0] = 2;
    }
    set_callbacks(f);
    Ok(())
}

/// ftGw_SpecialAirS_Coll (8014CA4C): landing (ft_80081D0C),
/// ftGw_SpecialAirS_AirToGround (8014CB44): the hop is available again.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Judgment landing assets");
    gw(f).x2234 = 0;
    let face = gw(f).judge_last as u16;
    common::air_to_ground(f, ActionId(GROUND_FIRST + face), TRANSITION_FLAGS, assets)?;
    set_callbacks(f);
    Ok(())
}

/// ftGameWatch_SpecialS_SetCall: the callbacks the motion change removed.
fn set_callbacks(f: &mut Fighter) {
    if gw(f).articles.judgement {
        gw(f).articles.damage_callbacks = true;
    }
    f.effect_state.article_hitlag = Some(ItemKind::GameWatchJudge);
    articles::install(f, Accessory::JudgementSetup);
}
