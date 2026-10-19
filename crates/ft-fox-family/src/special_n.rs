//! The Blaster: ftfoxspecialn.c (Fox and Falco, SpecialNStart/Loop/End and
//! the aerial rows) and ftkirbyspecialfox.c (Kirby's copied Blaster, the
//! same states over Kirby's rows). The two files differ only in what
//! [`Blaster`] names: the rows, the items, the hand and the callbacks.
use crate::{FamilyState, FoxFamily, SpecialNeutral};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    desc::fox_attributes::BlasterAttributes,
    fighter::{
        assets::{FighterAssets, Result},
        commands::{FootstepSound, SoundChannel},
        state::{callbacks, AnimationPhase, InputPhase},
        ActionId, Fighter, FighterCore, MotionRow, SpecialSlot,
    },
    input::pad::Buttons,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, FtPart, ItemKind};

/// The six Blaster rows of one user, in table order.
#[derive(Clone, Copy, Debug)]
pub struct BlasterRows {
    /// SpecialNStart; Loop, End, AirStart, AirLoop and AirEnd follow.
    pub first_action: u16,
    /// SpecialNStart's submotion; the others follow.
    pub first_animation: i32,
}
impl BlasterRows {
    pub const fn action(&self, row: BlasterRow) -> ActionId {
        ActionId(self.first_action + row as u16)
    }
    /// ftFx_SpecialN_GetBlasterAction / ftKb_SpecialNFx_800FDD4C: the row
    /// index the blaster's motion follows (it_803F6E68), if `action` is one
    /// of these rows.
    pub fn index(&self, action: u16) -> Option<u16> {
        (self.first_action..self.first_action + 6)
            .contains(&action)
            .then(|| action - self.first_action)
    }
}

/// ftFx_SpecialNIndex.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlasterRow {
    Start,
    Loop,
    End,
    AirStart,
    AirLoop,
    AirEnd,
}

/// The joint the blaster's two points hang from.
#[derive(Clone, Copy, Debug)]
pub enum HoldJoint {
    /// ftParts_GetBoneIndex(fp, part): through the kind's parts table.
    Part(FtPart),
    /// `fp->parts[index]`: a bone index the code names directly.
    Bone(u8),
}

/// ftFx_SpecialN_FtGetHoldJoint / ItGetHoldJoint (800E5CF8 / 800E5D44)
/// and ftKb_SpecialNFx_800FDC00 / 800FDC70: lb_8000B1CC of (0, `y`, `z`)
/// on the hand, the muzzle and the grip.
#[derive(Clone, Copy, Debug)]
pub struct HoldPoints {
    pub joint: HoldJoint,
    pub y: f32,
    pub muzzle_z: f32,
    pub grip_z: f32,
}

/// Who fires a Blaster: Fox and Falco (every [`FoxFamily`]) and Kirby with
/// either one's copied ability. Kirby's two kinds are marker types whose
/// methods reach into Kirby's own state.
pub trait Blaster: 'static {
    const ROWS: BlasterRows;
    /// The shot (it_8029C6A4) and the gun (it_802AE8A8).
    const LASER: ItemKind;
    const BLASTER: ItemKind;
    /// ft_PlaySFX of a shot: facing right, then left.
    const FIRE_SOUNDS: [u32; 2];
    const HOLD: HoldPoints;
    fn attributes(f: &Fighter) -> BlasterAttributes;
    /// mv.fx.SpecialN and the blaster's GObj (u.fx.x222C, u.kb.xB0).
    fn scratch(f: &mut Fighter) -> &mut SpecialNeutral;
    /// ftFox_SpecialN_SetCall / ftKb_SpecialN_set_cbs: take_dmg_cb and
    /// death2_cb, until the next motion change.
    fn arm_callbacks(f: &mut Fighter);
}

/// ftFx_Init_MotionStateTable's first six rows, animations 295..300.
pub const FOX_FAMILY_ROWS: BlasterRows = BlasterRows {
    first_action: FamilyState::SpecialNStart as u16,
    first_animation: 295,
};

/// Fox's and Falco's Blaster: the right thumb, ftFx_SpecialN_SpawnBlaster.
impl<C: FoxFamily> Blaster for C {
    const ROWS: BlasterRows = FOX_FAMILY_ROWS;
    const LASER: ItemKind = <C as FoxFamily>::LASER;
    const BLASTER: ItemKind = <C as FoxFamily>::BLASTER;
    const FIRE_SOUNDS: [u32; 2] = C::SOUNDS.fire;
    const HOLD: HoldPoints = HoldPoints {
        joint: HoldJoint::Part(FtPart::RThumbNb),
        y: 1.232_500_1,
        muzzle_z: 4.2636,
        grip_z: 0.013_600_001,
    };
    fn attributes(f: &Fighter) -> BlasterAttributes {
        f.character.get::<C>().attributes().blaster
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialNeutral {
        f.character.get_mut::<C>().special_neutral()
    }
    fn arm_callbacks(f: &mut Fighter) {
        Self::scratch(f).callbacks_armed = true;
    }
}

/// The six rows for `C`.
pub const fn rows<C: Blaster>() -> [MotionRow; 6] {
    [
        row::<C>(BlasterRow::Start, start::<C, false>, loop_input::<C>),
        row::<C>(BlasterRow::Loop, firing::<C, false>, loop_input::<C>),
        row::<C>(BlasterRow::End, end::<C, false>, no_input),
        row::<C>(BlasterRow::AirStart, start::<C, true>, loop_input::<C>),
        row::<C>(BlasterRow::AirLoop, firing::<C, true>, loop_input::<C>),
        row::<C>(BlasterRow::AirEnd, end::<C, true>, no_input),
    ]
}

const fn row<C: Blaster>(
    row: BlasterRow,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
) -> MotionRow {
    let airborne = row as u16 >= BlasterRow::AirStart as u16;
    MotionRow {
        action: C::ROWS.action(row),
        id: CommonMotionState::None,
        animation: C::ROWS.first_animation + row as i32,
        anim,
        iasa,
        // ft_80084DB0 / ft_80084F3C.
        physics: if airborne {
            callbacks::physics::pass
        } else {
            callbacks::physics::guard_on
        },
        // ftCo_AirCatchHit_Coll / ft_80083F88.
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
    enter::<C>(f, airborne, assets);
}

/// ftFx_SpecialN_Enter / ftFx_SpecialAirN_Enter and Kirby's
/// ftKb_SpecialNFx_800FE100 / 800FE240: the Start row, the blaster in hand
/// and the callbacks in.
pub fn enter<C: Blaster>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    let row = if airborne {
        BlasterRow::AirStart
    } else {
        BlasterRow::Start
    };
    f.change_motion_state(C::ROWS.action(row), assets)
        .expect("SpecialN motion assets");
    f.commands.variables.fill(0);
    f.step_animation(assets);
    if !airborne {
        f.physics.ground_velocity = 0.0;
        f.physics.self_velocity = Vec3::ZERO;
    }
    // it_802AE8A8: the blaster, then the callbacks.
    *C::scratch(f) = SpecialNeutral {
        blaster_present: true,
        ..SpecialNeutral::default()
    };
    C::arm_callbacks(f);
    let spawn = SpawnItem::held(
        C::BLASTER,
        f.player.id,
        f.physics.position,
        f.physics.facing,
    );
    f.core.item_requests.push(ItemRequest::SpawnHeld(spawn));
}

fn control<C: Blaster>(f: &mut Fighter, control: ItemControl) {
    if C::scratch(f).blaster_present {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: C::BLASTER,
            control,
        });
    }
}

/// ftFox_SpecialN_UpdateBlaster, inlined into Start/Loop animation callbacks.
fn update_blaster<C: Blaster>(f: &mut Fighter) {
    control::<C>(f, ItemControl::Visibility(1));
    if f.commands.variables[3] == 1 && C::scratch(f).blaster_present {
        f.commands.variables[3] = 0;
        control::<C>(f, ItemControl::Open);
    }
}

fn start<C: Blaster, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    update_blaster::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        let next = if AIR {
            BlasterRow::AirLoop
        } else {
            BlasterRow::Loop
        };
        f.change_motion_state(C::ROWS.action(next), phase.assets)?;
        // The Start row's script runs without Ft_MF_SkipAttackCount: each
        // firing cycle is a new instance.
        f.combat.stale.new_instance();
        let scratch = C::scratch(f);
        // accessory4_cb = the shot.
        scratch.accessory_shot = true;
        C::arm_callbacks(f);
        control::<C>(f, ItemControl::Visibility(1));
    }
    Ok(None)
}

fn firing<C: Blaster, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    update_blaster::<C>(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        if C::scratch(f).repeat {
            let next = if AIR {
                BlasterRow::AirLoop
            } else {
                BlasterRow::Loop
            };
            f.change_motion_state(C::ROWS.action(next), phase.assets)?;
            C::scratch(f).repeat = false;
            // x21EC = ftFx_SpecialN_OnChangeAction / ftKb_SpecialNFx_OnChangeAction
            // -> ft_800892A0: each firing cycle is a new instance.
            f.combat.stale.new_instance();
            C::scratch(f).accessory_shot = true;
        } else {
            let next = if AIR {
                BlasterRow::AirEnd
            } else {
                BlasterRow::End
            };
            f.change_motion_state(C::ROWS.action(next), phase.assets)?;
            C::scratch(f).accessory_shot = false;
            f.commands.variables[1] = 1;
        }
        // The callbacks after either transition.
        C::arm_callbacks(f);
        control::<C>(f, ItemControl::Visibility(1));
    }
    fire::<C>(f, phase.assets);
    Ok(None)
}

fn end<C: Blaster, const AIR: bool>(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    control::<C>(f, ItemControl::Visibility(f.commands.variables[1] as i32));
    if f.commands.variables[3] == 2 && C::scratch(f).blaster_present {
        f.commands.variables[3] = 0;
        control::<C>(f, ItemControl::Close);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        // ftFox_SpecialN_RemoveBlasterNULL / ftKb_SpecialNFx_800FDEB4: the
        // blaster lets go of itself once its owner's row is no Blaster row.
        let scratch = C::scratch(f);
        scratch.blaster_present = false;
        scratch.accessory_shot = false;
        let lag = C::attributes(f).landing_lag;
        if AIR && lag != 0.0 {
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
fn loop_input<C: Blaster>(f: &mut Fighter, _phase: InputPhase<'_>) {
    if f.commands.variables[0] != 0 && f.input.pressed.intersects(Buttons::B) {
        C::scratch(f).repeat = true;
    }
}
fn no_input(_f: &mut Fighter, _phase: InputPhase<'_>) {}

/// The muzzle (`muzzle`) or the grip of `C`'s blaster hand.
pub fn hold_position<C: Blaster>(
    core: &mut FighterCore,
    assets: &FighterAssets,
    muzzle: bool,
) -> Vec3 {
    let hold = C::HOLD;
    let bone = match hold.joint {
        HoldJoint::Part(part) => assets.parts.joint(part).expect("blaster hold bone"),
        HoldJoint::Bone(bone) => bone,
    };
    let offset = Vec3::new(
        0.0,
        hold.y,
        if muzzle { hold.muzzle_z } else { hold.grip_z },
    );
    // lb_8000B1CC delegates to the audited PSMTXMultVec kernel.
    melee_ft::fighter::caches::bone_position(
        &mut core.skeleton,
        core.animation.root,
        usize::from(bone),
        offset,
    )
}

/// ftFx_SpecialN_CreateBlasterShot (800E5F28) / ftKb_SpecialNFx_800FDF30:
/// reached in the Loop rows' animation and in accessory4.
pub fn fire<C: Blaster>(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[2] == 0 {
        return;
    }
    f.commands.variables[2] = 0;
    let mut position = hold_position::<C>(&mut f.core, assets, true);
    position.z = 0.0;
    let attrs = C::attributes(f);
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
    let sound = C::FIRE_SOUNDS[usize::from(f.physics.facing == -1.0)];
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id: sound,
        volume: 127,
        pan: 64,
    });
}

/// ftFx_Init_800E5588 -> ftFx_SpecialN_RemoveBlaster (800E5EBC): the
/// take-damage and death cleanup, while ftFox_SpecialN_SetCall's callbacks
/// are installed.
pub fn remove_blaster<C: FoxFamily>(f: &mut Fighter) {
    if !f.character.get_mut::<C>().special_neutral().callbacks_armed {
        return;
    }
    control::<C>(f, ItemControl::Remove);
    f.character.get_mut::<C>().special_neutral().blaster_present = false;
    f.character.get_mut::<C>().special_neutral().accessory_shot = false;
}

/// ftKb_SpecialNFx_800FDEE0: the blaster, if out, is destroyed
/// (it_802AEAB4) and let go of.
pub fn destroy_blaster<C: Blaster>(f: &mut Fighter) {
    control::<C>(f, ItemControl::Remove);
    let scratch = C::scratch(f);
    scratch.blaster_present = false;
    scratch.accessory_shot = false;
}

/// accessory4_cb from the firing loop. Retail installs it at loop entry and
/// every other motion change clears it, so leaving the loop any other way
/// (a hit, a grab, a landing) disarms it before another move's command
/// variable 2 could fire a shot.
pub fn accessory<C: Blaster>(f: &mut Fighter, assets: &FighterAssets) {
    let action = f.motion_state.action.0;
    let firing = action == C::ROWS.action(BlasterRow::Loop).0
        || action == C::ROWS.action(BlasterRow::AirLoop).0;
    let scratch = C::scratch(f);
    if !firing {
        scratch.accessory_shot = false;
    }
    if scratch.accessory_shot {
        fire::<C>(f, assets);
    }
}

/// ftFx_SpecialN_GetBlasterAction / ftKb_SpecialNFx_800FDD4C: the row the
/// blaster's motion follows, 9 outside the Blaster rows and the three
/// throws that draw it (ThrowB/Hi/Lw: `msid - 0xD6`).
pub fn blaster_action<C: Blaster>(action: u16) -> u16 {
    if let Some(index) = C::ROWS.index(action) {
        index
    } else if (CommonMotionState::ThrowB as u16..=CommonMotionState::ThrowLw as u16)
        .contains(&action)
    {
        action - CommonMotionState::CatchDash as u16
    } else {
        9
    }
}

/// What the blaster reads of its owner: ftFx_SpecialN_GetBlasterAction /
/// CheckRemoveBlaster, or Kirby's 800FDD4C / 800FDD14.
pub fn item_owner<C: Blaster>(f: &mut Fighter) -> melee_it::ItemOwner {
    let action = f.motion_state.action.0;
    melee_it::ItemOwner {
        blaster_action: blaster_action::<C>(action),
        remove_blaster: !C::scratch(f).blaster_present,
        ..melee_ft::fighter::default_item_owner(f)
    }
}

/// it_802ADF10: muzzle coordinates are sampled after fighter pose, in item link 9.
pub fn item_muzzle<C: Blaster>(f: &mut FighterCore, assets: &FighterAssets) -> (Vec3, f32) {
    let muzzle = hold_position::<C>(f, assets, true);
    let hold = hold_position::<C>(f, assets, false);
    // it_802ADF10 (802ADFE4/802ADFE8): separate fsubs before atan2f.
    let angle = melee_lb::trigf::atan2f(muzzle.y - hold.y, muzzle.x - hold.x);
    (muzzle, angle)
}

/// ftFx_Throw_Anim (800E6CDC): common throw callbacks own animation stepping.
pub fn throw_animation<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let action = f.motion_state.id;
    if !matches!(
        action,
        CommonMotionState::ThrowB | CommonMotionState::ThrowHi | CommonMotionState::ThrowLw
    ) {
        return;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.character.get_mut::<C>().special_neutral().blaster_present = false;
        return;
    }
    match f.commands.variables[1] {
        1 => {
            if !f.character.get_mut::<C>().special_neutral().blaster_present {
                let spawn = SpawnItem::held(
                    <C as Blaster>::BLASTER,
                    f.player.id,
                    f.physics.position,
                    f.physics.facing,
                );
                let scratch = f.character.get_mut::<C>().special_neutral();
                scratch.blaster_present = true;
                // ftFox_SpecialN_SetCall.
                scratch.callbacks_armed = true;
                f.item_requests.push(ItemRequest::SpawnHeld(spawn));
                // Retail returns immediately after successful or failed creation.
                return;
            }
            control::<C>(f, ItemControl::Visibility(1));
            match f.commands.variables[3] {
                1 => {
                    f.commands.variables[3] = 0;
                    control::<C>(f, ItemControl::Open);
                }
                2 => {
                    f.commands.variables[3] = 0;
                    control::<C>(f, ItemControl::Close);
                }
                _ => {}
            }
            if std::mem::take(&mut f.commands.throw_accessory) {
                let mut muzzle = hold_position::<C>(&mut f.core, assets, true);
                let mut hold = hold_position::<C>(&mut f.core, assets, false);
                muzzle.z = 0.0;
                hold.z = 0.0;
                // 800E6EFC/800E6F00: two fsubs before atan2f, no fusion.
                let angle = melee_lb::trigf::atan2f(muzzle.y - hold.y, muzzle.x - hold.x);
                let speed = f.character.get::<C>().attributes().blaster.velocity;
                let mut spawn =
                    SpawnItem::ray(<C as Blaster>::LASER, f.player.id, muzzle, f.physics.facing);
                // Item_InitRaySpawnPosition uses the fighter ECB midpoint.
                let midpoint = 0.5 * (f.collision.data.ecb.top.y + f.collision.data.ecb.bottom.y);
                spawn.position = Vec3::new(
                    f.physics.position.x + 0.0,
                    f.physics.position.y + midpoint,
                    f.physics.position.z + 0.0,
                );
                f.item_requests.push(ItemRequest::SpawnLaser {
                    spawn,
                    angle,
                    speed,
                    motion: 1,
                });
                control::<C>(f, ItemControl::Fire);
                let sound = if action == CommonMotionState::ThrowB {
                    C::SOUNDS.fire[usize::from(f.physics.facing == 1.0)]
                } else {
                    C::SOUNDS.throw_fire
                };
                f.commands.footstep_sounds.push(FootstepSound {
                    channel: SoundChannel::Ordinary,
                    id: sound,
                    volume: 127,
                    pan: 64,
                });
            }
        }
        2 => {
            let state = f.character.get_mut::<C>().special_neutral();
            if state.blaster_present {
                state.blaster_present = false;
                f.commands.footstep_sounds.push(FootstepSound {
                    channel: SoundChannel::Ordinary,
                    id: C::SOUNDS.holster,
                    volume: 127,
                    pan: 64,
                });
            }
        }
        0 => {
            f.character.get_mut::<C>().special_neutral().blaster_present = false;
            // ftpickupitem_80094818(gobj, 0) is inert in the item-free throw.
        }
        _ => {}
    }
}
