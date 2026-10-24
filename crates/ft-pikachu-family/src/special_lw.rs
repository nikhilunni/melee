//! Thunder, ftpikachuspeciallw.c (8012757C..80127C54). The start (359 /
//! 363) runs into the loop (360 / 364), whose accessory calls the bolt
//! chain down at the script's throw flag; the lead bolt striking Pikachu
//! turns the loop into the struck loop (361 / 365), and the script, the
//! lead's end or its strike elsewhere ends the move (362 / 366).
use crate::{
    common::{self, change, no_input},
    flags::{GROUND_AIR, KEEP_COL_ANIM_HIT_STATUS, KEEP_GFX, SKIP_HIT},
    row, Accessory, FamilyState as S, PikachuFamily,
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        ActionId, Fighter, MotionRow,
    },
    physics::friction,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftPk_MF_SpecialLw_Coll and its Hit / HitRumble extensions.
const COUNTERPART_FLAGS: u32 = GROUND_AIR | KEEP_GFX | KEEP_COL_ANIM_HIT_STATUS;
const HIT_FLAGS: u32 = COUNTERPART_FLAGS | SKIP_HIT;
/// Ft_MF_SkipRumble.
const SKIP_RUMBLE: u32 = 1 << 11;
const HIT_RUMBLE_FLAGS: u32 = HIT_FLAGS | SKIP_RUMBLE;
/// efSync_Spawn(1219): the cloud, an AppSRT generator 0x24C at a point.
const CLOUD: u16 = 1219;
/// efAsync_Spawn(..., 0, 1216, parts[FtPart_TopN]): the strike's model.
const STRIKE: u16 = 1216;
/// mv.pk.speciallw.x4 values: bolts may strike, the lead is gone.
const ARMED: i32 = 1;
const LEAD_GONE: i32 = 3;

/// mv.pk.speciallw (ftPikachu/types.h), which aliases specialhi's x0/x4.
#[derive(Clone, Debug, Default)]
pub struct Thunder {
    /// x4: 1 while the lead bolt may strike Pikachu, 0 once Pikachu was
    /// hit (take_dmg_cb), 3 once the lead is gone.
    pub state: i32,
    /// x0 != NULL: the bolts were called down.
    pub called: bool,
}

pub const fn rows<C: PikachuFamily>() -> [MotionRow; 8] {
    [
        row(
            S::SpecialLwStart,
            start_anim::<C, false>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision::<C, { S::SpecialAirLwStart as u16 }, COUNTERPART_FLAGS, false>,
        ),
        row(
            S::SpecialLwLoop0,
            loop_anim::<C, false>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision::<C, { S::SpecialAirLwLoop0 as u16 }, HIT_RUMBLE_FLAGS, true>,
        ),
        row(
            S::SpecialLwLoop1,
            struck_anim::<false>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision::<C, { S::SpecialAirLwLoop1 as u16 }, HIT_FLAGS, false>,
        ),
        row(
            S::SpecialLwEnd,
            end_anim::<false>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision::<C, { S::SpecialAirLwEnd as u16 }, COUNTERPART_FLAGS, false>,
        ),
        row(
            S::SpecialAirLwStart,
            start_anim::<C, true>,
            no_input,
            callbacks::physics::air_friction,
            air_collision::<C, { S::SpecialLwStart as u16 }, COUNTERPART_FLAGS, false>,
        ),
        row(
            S::SpecialAirLwLoop0,
            loop_anim::<C, true>,
            no_input,
            callbacks::physics::air_friction,
            air_collision::<C, { S::SpecialLwLoop0 as u16 }, HIT_RUMBLE_FLAGS, true>,
        ),
        row(
            S::SpecialAirLwLoop1,
            struck_anim::<true>,
            no_input,
            struck_air_physics::<C>,
            air_collision::<C, { S::SpecialLwLoop1 as u16 }, HIT_FLAGS, false>,
        ),
        row(
            S::SpecialAirLwEnd,
            end_anim::<true>,
            no_input,
            callbacks::physics::air_friction,
            air_collision::<C, { S::SpecialLwEnd as u16 }, COUNTERPART_FLAGS, false>,
        ),
    ]
}

fn attributes<C: PikachuFamily>(f: &Fighter) -> &crate::attributes::ThunderAttributes {
    &f.character.get::<C>().attributes().thunder
}
fn thunder<C: PikachuFamily>(f: &mut Fighter) -> &mut Thunder {
    &mut f.character.get_mut::<C>().specials().thunder
}

/// ftPk_SpecialLw_Enter (8012786C) / ftPk_SpecialAirLw_Enter (801278D4).
pub fn enter<C: PikachuFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.variables[0] = 0;
    f.commands.clear_throw_flags();
    *thunder::<C>(f) = Thunder {
        state: ARMED,
        called: false,
    };
    let state = if air {
        S::SpecialAirLwStart
    } else {
        S::SpecialLwStart
    };
    change(f, state.action(), 0, 0.0, 1.0, assets).expect("Thunder assets");
    f.step_animation(assets);
}

/// The loop's take_dmg_cb (ftPk_SpecialLw_SetState_Unk1) and accessory4
/// (ftPk_SpecialLw_SpawnEffect), installed together.
fn arm_loop<C: PikachuFamily>(f: &mut Fighter) {
    f.character.get_mut::<C>().specials().accessory = Accessory::ThunderCloud;
    f.arm_accessory4();
}

/// Whether the loop's take_dmg_cb is installed during `action`.
pub fn takes_damage(action: ActionId) -> bool {
    action == S::SpecialLwLoop0.action() || action == S::SpecialAirLwLoop0.action()
}

/// ftPk_SpecialLw_SetState_Unk1 (80127688), take_dmg_cb: hit, the bolts
/// no longer strike Pikachu.
pub fn take_damage<C: PikachuFamily>(f: &mut Fighter) {
    if takes_damage(f.motion_state.action) {
        thunder::<C>(f).state = 0;
    }
}

/// ftPk_SpecialLwStart_Anim / ftPk_SpecialAirLwStart_Anim: at the end the
/// loop (Ft_MF_SkipRumble), the throw flags clear, and the loop's
/// callbacks install.
fn start_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if AIR {
            S::SpecialAirLwLoop0
        } else {
            S::SpecialLwLoop0
        };
        change(f, state.action(), SKIP_RUMBLE, 0.0, 1.0, p.assets)?;
        f.commands.clear_throw_flags();
        arm_loop::<C>(f);
    }
    Ok(None)
}

/// ftPk_SpecialLwLoop0_Anim / ftPk_SpecialAirLwLoop0_Anim (80127C5C /
/// 80127CE0): the lead gone or the script's cue ends the move; the lead
/// striking Pikachu turns to the struck loop (the aerial one rising) with
/// the strike's model on TopN.
fn loop_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if thunder::<C>(f).state == LEAD_GONE || f.commands.variables[0] != 0 {
        let state = if AIR {
            S::SpecialAirLwEnd
        } else {
            S::SpecialLwEnd
        };
        return change(f, state.action(), 0, 0.0, 1.0, p.assets).map(|()| None);
    }
    if struck_by_lead::<C>(f) {
        let state = if AIR {
            S::SpecialAirLwLoop1
        } else {
            S::SpecialLwLoop1
        };
        change(f, state.action(), 0, 0.0, 1.0, p.assets)?;
        f.commands.variables[0] = 0;
        if AIR {
            f.physics.self_velocity.y = attributes::<C>(f).air_hit_vertical_speed;
        }
        // efAsync_Spawn after the change: queued behind the new script's
        // frame-0 graphics.
        f.core
            .push_effect_after_issued_graphics(EffectRequest::Attached { id: STRIKE, bone: 0 });
    }
    Ok(None)
}

/// ftPk_SpecialLw_8012765C (8012765C): while armed, the lead bolt's tip
/// (it_802B1FE8) within the attribute's reach of Pikachu (fsubs and the
/// height fadds / fsubs, each compared by size), and the lead not struck
/// yet (it_802B1DEC), strikes it (it_802B1FC8).
fn struck_by_lead<C: PikachuFamily>(f: &mut Fighter) -> bool {
    if thunder::<C>(f).state == 0 {
        return false;
    }
    let Some(lead) = f.core.owned_article else {
        return false;
    };
    let a = attributes::<C>(f);
    let (width, height, range) = (a.hit_width, a.hit_height, a.hit_range);
    let position = f.physics.position;
    if (position.x - lead.point.x).abs() >= width.abs() {
        return false;
    }
    if ((position.y + height.abs()) - lead.point.y).abs() >= range || lead.struck {
        return false;
    }
    let kind = bolt_kind::<C>(f);
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind,
        control: melee_it::ItemControl::Strike,
    });
    true
}

/// ftPk_SpecialLwLoop1_Anim / ftPk_SpecialAirLwLoop1_Anim: the script's
/// cue ends the move.
fn struck_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 {
        let state = if AIR {
            S::SpecialAirLwEnd
        } else {
            S::SpecialLwEnd
        };
        change(f, state.action(), 0, 0.0, 1.0, p.assets)?;
    }
    Ok(None)
}

/// ftPk_SpecialLwEnd_Anim / ftPk_SpecialAirLwEnd_Anim: Wait or Fall.
fn end_anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftPk_SpecialAirLwLoop1_Phys (80127B34): the struck rise's gravity, then
/// ftCommon_8007CF58.
fn struck_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let gravity = attributes::<C>(f).air_hit_gravity;
    let terminal = f.attributes.air.terminal_velocity;
    common::fall(f, gravity, terminal);
    let air = &f.attributes.air;
    f.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
        f.physics.self_velocity.x,
        air.aerial_friction,
        air.air_drift_max,
        p.assets.common.over_drift_air_friction,
    );
    common::finish_air(f, &p);
}

/// ft_8008403C with ftPk_SpecialLw_ChangeMotion_Unk01/03/05/07: off the
/// floor, the aerial counterpart (the loop's callbacks again), then
/// ftCommon_ClampAirDrift.
fn ground_collision<C: PikachuFamily, const AIR_STATE: u16, const FLAGS: u32, const LOOP: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Thunder collision assets");
        common::ground_to_air(f, ActionId(AIR_STATE), FLAGS, assets)?;
        if LOOP {
            arm_loop::<C>(f);
        }
        let maximum = f.attributes.air.air_drift_max;
        common::clamp_self_velocity_x(f, maximum);
    }
    Ok(())
}

/// ft_80082C74 with ftPk_SpecialLw_ChangeMotion_Unk00/02/04/06: landing,
/// the grounded counterpart (the loop's callbacks again).
fn air_collision<C: PikachuFamily, const GROUND_STATE: u16, const FLAGS: u32, const LOOP: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Thunder landing assets");
        common::air_to_ground(f, ActionId(GROUND_STATE), FLAGS, assets)?;
        if LOOP {
            arm_loop::<C>(f);
        }
    }
    Ok(())
}

/// The bolt's item kind (attribute +DC, ftPk_Init_OnLoad's registration).
fn bolt_kind<C: PikachuFamily>(f: &Fighter) -> ItemKind {
    let kind = attributes::<C>(f).bolt_item;
    ItemKind::try_from(kind as i32).expect("Thunder bolt kind")
}

/// ftPk_SpecialLw_SpawnEffect (80127698), the loop's accessory4, run every
/// frame while installed: the script's throw flag (always consumed) calls
/// the bolts once, from the attribute's height above Pikachu, with the
/// cloud a little above that.
pub fn call_bolts<C: PikachuFamily>(f: &mut Fighter) {
    if !std::mem::take(&mut f.commands.throw_accessory) || thunder::<C>(f).called {
        return;
    }
    let a = attributes::<C>(f);
    let (spawn_height, cloud, velocity, [count, delay]) = (
        a.spawn_height,
        a.cloud_height,
        a.bolt_velocity,
        a.bolt_parameters,
    );
    let mut position = f.physics.position;
    position.y += spawn_height;
    let mut cloud_position = position;
    cloud_position.y += cloud;
    f.effects.push(EffectRequest::PositionalGenerator {
        id: CLOUD,
        position: cloud_position,
    });
    let kind = bolt_kind::<C>(f);
    // it_802B1DF8: Item_InitSpawn-like fields, facing -1, initial collision.
    let mut spawn = SpawnItem::attached(kind, f.player.id, position, -1.0);
    spawn.position.z = 0.0;
    spawn.previous_position.z = 0.0;
    f.core.item_requests.push(ItemRequest::SpawnChain {
        spawn,
        count,
        delay,
        velocity: Vec3::new(0.0, velocity, 0.0),
    });
    thunder::<C>(f).called = true;
}

/// ftPk_MF_Special's x2071 nibble (Ft_MF_UnkUpdatePhys | FreezeState),
/// shared by every family row.
const SPECIAL_PROPERTY: u8 = 3;

/// ftPk_SpecialLw_CheckProperty (8012757C): the motion's x2071 nibble
/// (MotionState.x4_flags bits 20-23) is 1, 2 or 4..=13.
pub fn busy(action: ActionId) -> bool {
    let property = melee_types::motion_property::COMMON_MOTION_PROPERTIES
        .get(usize::from(action.0))
        .copied()
        .unwrap_or(SPECIAL_PROPERTY);
    matches!(property, 1 | 2 | 4..=13)
}

/// it_2725_Logic39_Destroyed (802B2020) for the lead:
/// ftPk_SpecialLw_SetState_Unk0 unless Pikachu's motion is busy
/// (ftPk_SpecialLw_CheckProperty).
pub fn lead_gone<C: PikachuFamily>(f: &mut Fighter) {
    if busy(f.motion_state.action) {
        return;
    }
    let action = f.motion_state.action;
    let in_thunder = (S::SpecialLwStart as u16..=S::SpecialAirLwEnd as u16).contains(&action.0);
    let in_quick_attack =
        (S::SpecialHiStart0 as u16..=S::SpecialAirHiEnd as u16).contains(&action.0);
    if in_thunder {
        thunder::<C>(f).state = LEAD_GONE;
    } else if in_quick_attack {
        // mv.pk.specialhi.x4 is Quick Attack's own word.
        f.character.get_mut::<C>().specials().quick_attack.zip_frames = Some(LEAD_GONE);
    } else if action.0 < S::SpecialN as u16 {
        // mv.pk.specialhi.x4 = 3 lands in whatever the common state keeps
        // at mv+4 (an int, read back as the float word).
        f.overwrite_common_scratch_word(f32::from_bits(LEAD_GONE as u32));
    } else {
        unimplemented!(
            "ftPk_SpecialLw_SetState_Unk0: mv+4 written during special {:?}",
            action
        );
    }
}
