//! ftfoxspecialn.c, SpecialNStart/Loop/End and aerial counterparts.
use crate::{FamilyState, FoxFamily, SpecialNeutral};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        commands::{FootstepSound, SoundChannel},
        state::{callbacks, AnimationPhase, InputPhase},
        ActionId, Fighter, FighterCore, MotionRow, SpecialSlot,
    },
    input::pad::Buttons,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart};

/// ftFx_Init_MotionStateTable (800E5534), first six rows, animations 295..300.
pub const fn rows<C: FoxFamily>() -> [MotionRow; 6] {
    [
        row(
            FamilyState::SpecialNStart,
            295,
            start::<C, false>,
            loop_input::<C>,
            false,
        ),
        row(
            FamilyState::SpecialNLoop,
            296,
            firing::<C, false>,
            loop_input::<C>,
            false,
        ),
        row(
            FamilyState::SpecialNEnd,
            297,
            end::<C, false>,
            no_input,
            false,
        ),
        row(
            FamilyState::SpecialAirNStart,
            298,
            start::<C, true>,
            loop_input::<C>,
            true,
        ),
        row(
            FamilyState::SpecialAirNLoop,
            299,
            firing::<C, true>,
            loop_input::<C>,
            true,
        ),
        row(
            FamilyState::SpecialAirNEnd,
            300,
            end::<C, true>,
            no_input,
            true,
        ),
    ]
}

const fn row(
    state: FamilyState,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    airborne: bool,
) -> MotionRow {
    MotionRow {
        action: ActionId(state as u16),
        id: CommonMotionState::None,
        animation,
        anim,
        iasa,
        physics: if airborne {
            callbacks::physics::pass
        } else {
            callbacks::physics::guard_on
        },
        collision: if airborne {
            callbacks::collision::air_catch_hit
        } else {
            callbacks::collision::ground_action
        },
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftFx_SpecialN_Enter (800E608C), ftFx_SpecialAirN_Enter (800E61A8).
pub fn enter_special<C: FoxFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    airborne: bool,
    assets: &FighterAssets,
) {
    assert_eq!(slot, SpecialSlot::Neutral, "unported Fox family special");
    let state = if airborne {
        FamilyState::SpecialAirNStart
    } else {
        FamilyState::SpecialNStart
    };
    f.change_motion_state(state.into(), assets)
        .expect("SpecialN motion assets");
    f.commands.variables.fill(0);
    f.step_animation(assets);
    if !airborne {
        f.physics.ground_velocity = 0.0;
        f.physics.self_velocity = Vec3::ZERO;
    }
    *f.character.get_mut::<C>().special_neutral() = SpecialNeutral {
        blaster_present: true,
        ..SpecialNeutral::default()
    };
    let spawn = SpawnItem::held(
        C::BLASTER,
        f.player.id,
        f.physics.position,
        f.physics.facing,
    );
    f.core.item_requests.push(ItemRequest::Spawn(spawn));
}

fn control<C: FoxFamily>(f: &mut Fighter, control: ItemControl) {
    if f.character.get_mut::<C>().special_neutral().blaster_present {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: C::BLASTER,
            control,
        });
    }
}

/// ftFox_SpecialN_UpdateBlaster, inlined into Start/Loop animation callbacks.
fn update_blaster<C: FoxFamily>(f: &mut Fighter) {
    control::<C>(f, ItemControl::Visibility(1));
    if f.commands.variables[3] == 1 && f.character.get_mut::<C>().special_neutral().blaster_present
    {
        f.commands.variables[3] = 0;
        control::<C>(f, ItemControl::Open);
    }
}

fn start<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    update_blaster::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            (if AIR {
                FamilyState::SpecialAirNLoop
            } else {
                FamilyState::SpecialNLoop
            })
            .into(),
            phase.assets,
        )?;
        f.character.get_mut::<C>().special_neutral().accessory_shot = true;
        control::<C>(f, ItemControl::Visibility(1));
    }
    Ok(None)
}

fn firing<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    update_blaster::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.character.get_mut::<C>().special_neutral().repeat {
            f.change_motion_state(
                (if AIR {
                    FamilyState::SpecialAirNLoop
                } else {
                    FamilyState::SpecialNLoop
                })
                .into(),
                phase.assets,
            )?;
            f.character.get_mut::<C>().special_neutral().repeat = false;
            f.character.get_mut::<C>().special_neutral().accessory_shot = true;
        } else {
            f.change_motion_state(
                (if AIR {
                    FamilyState::SpecialAirNEnd
                } else {
                    FamilyState::SpecialNEnd
                })
                .into(),
                phase.assets,
            )?;
            f.character.get_mut::<C>().special_neutral().accessory_shot = false;
            f.commands.variables[1] = 1;
        }
        control::<C>(f, ItemControl::Visibility(1));
    }
    fire::<C>(f, phase.assets);
    Ok(None)
}

fn end<C: FoxFamily, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    control::<C>(f, ItemControl::Visibility(f.commands.variables[1] as i32));
    if f.commands.variables[3] == 2 && f.character.get_mut::<C>().special_neutral().blaster_present
    {
        f.commands.variables[3] = 0;
        control::<C>(f, ItemControl::Close);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.character.get_mut::<C>().special_neutral().blaster_present = false;
        f.character.get_mut::<C>().special_neutral().accessory_shot = false;
        if AIR && f.character.get::<C>().attributes().blaster.landing_lag != 0.0 {
            let lag = f.character.get::<C>().attributes().blaster.landing_lag;
            f.enter_special_fall(phase.assets, true, false, true, 1.0, lag)?;
        } else {
            f.change_motion_state(
                (if AIR {
                    CommonMotionState::Fall
                } else {
                    CommonMotionState::Wait
                })
                .into(),
                phase.assets,
            )?;
        }
    }
    Ok(None)
}

/// ftFox_SpecialN_CheckLoopInput: only a fresh B press during cmd_var0's window.
fn loop_input<C: FoxFamily>(f: &mut Fighter, _phase: InputPhase<'_>) {
    if f.commands.variables[0] != 0 && f.input.pressed.intersects(Buttons::B) {
        f.character.get_mut::<C>().special_neutral().repeat = true;
    }
}
fn no_input(_f: &mut Fighter, _phase: InputPhase<'_>) {}

/// ftFx_SpecialN_FtGetHoldJoint / ItGetHoldJoint (800E5CF8 / 800E5D44).
pub fn hold_position(core: &mut FighterCore, assets: &FighterAssets, muzzle: bool) -> Vec3 {
    let bone = assets
        .parts
        .joint(FtPart::RThumbNb)
        .expect("blaster hold bone");
    let offset = Vec3::new(
        0.0,
        1.232_500_1,
        if muzzle { 4.2636 } else { 0.013_600_001 },
    );
    // lb_8000B1CC delegates to the audited PSMTXMultVec kernel.
    melee_ft::fighter::caches::bone_position(
        &mut core.skeleton,
        core.animation.root,
        usize::from(bone),
        offset,
    )
}

/// ftFx_SpecialN_CreateBlasterShot (800E5F28): reached in animation and accessory4.
fn fire<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[2] == 0 {
        return;
    }
    f.commands.variables[2] = 0;
    let mut position = hold_position(&mut f.core, assets, true);
    position.z = 0.0;
    let attrs = &f.character.get::<C>().attributes().blaster;
    // Retail 800E5FCC fsub (double), rounded on the it_8029C6A4 call boundary.
    let angle = if f.physics.facing == 1.0 {
        attrs.angle
    } else {
        (std::f64::consts::PI - f64::from(attrs.angle)) as f32
    };
    let speed = attrs.velocity;
    let mut spawn = SpawnItem::ray(C::LASER, f.player.id, position, f.physics.facing);
    // Item_InitRaySpawnPosition -> ftLib_80086990 starts the spawn sweep at
    // the fighter ECB midpoint. Retail 800869AC..BC: fadds, fmuls, fadds.
    let midpoint = 0.5 * (f.collision.data.ecb.top.y + f.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        f.physics.position.x + 0.0,
        f.physics.position.y + midpoint,
        f.physics.position.z + 0.0,
    );
    f.core.item_requests.push(ItemRequest::SpawnLaser {
        spawn,
        angle,
        speed,
        motion: 0,
    });
    control::<C>(f, ItemControl::Fire);
    let sound = C::SOUNDS.fire[usize::from(f.physics.facing == -1.0)];
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id: sound,
        volume: 127,
        pan: 64,
    });
}

pub fn accessory<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) {
    if f.character.get_mut::<C>().special_neutral().accessory_shot {
        fire::<C>(f, assets);
    }
}

/// ftFx_SpecialN_GetBlasterAction / CheckRemoveBlaster.
pub fn item_owner<C: FoxFamily>(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
    let action = f.motion_state.action.0;
    melee_it::ItemOwner {
        illusion: None,
        position: f.physics.position,
        facing: f.physics.facing,
        hold_position: f.physics.position,
        blaster_action: if (341..347).contains(&action) {
            action - 341
        } else {
            9
        },
        remove_blaster: !f.character.get_mut::<C>().special_neutral().blaster_present,
    }
}

/// it_802ADF10: muzzle coordinates are sampled after fighter pose, in item link 9.
pub fn item_muzzle(f: &mut FighterCore, assets: &FighterAssets) -> (Vec3, f32) {
    let muzzle = hold_position(f, assets, true);
    let hold = hold_position(f, assets, false);
    // it_802ADF10 (802ADFE4/802ADFE8): separate fsubs before atan2f.
    let angle = melee_lb::trigf::atan2f(muzzle.y - hold.y, muzzle.x - hold.x);
    (muzzle, angle)
}
