//! The yo-yo smashes' shared work, ftnessattackhi4.c (80114EB8..80115BB0):
//! the yo-yo article in Ness's hand, the point hitbox 0 follows, the charge
//! and the timed rehit.
//!
//! Both smashes start the same way: the article appears at frame 2, goes
//! out on its string at the attributes' out frame, and on frame 13 a held A
//! with room for the yo-yo (`charge_has_room`) enters the charge row, which
//! throws it ahead on its string ([`it_ness::yoyo::string`]) while
//! `smash_attrs.state` is 4. Releasing A, or the charge's length, enters the
//! release row at frame 13 of its animation, the hit's damage scaled by the
//! charge. The article ends at the return frame (or when Ness leaves the
//! rows), and hitbox 0 follows u.ns.yoyo_hitbox_pos through accessory4.
//!
//! A hit starts the rehit timer (deal_dmg_cb); while the script's first
//! variable is clear the timer runs, and at zero hitbox 0 forgets its
//! victims and is enabled again (lbColl_80008440, lbColl_80008434) even if
//! the script has disabled it since. A capsule enabled that way after the
//! script's clear-all (x2219_b3 clear) survives the motion changes that
//! follow: the stored yo-yo hitbox.
pub mod down;
pub mod up;

use crate::{common, init::Ness};
use gekko_math::fma::{fmadds, fmsubs};
use hsd_types::Vec3;
use it_ness::yoyo::{
    motion,
    string::{YoyoAttributes, YoyoString},
};
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    state::{callbacks, CollisionPhase, PhysicsPhase},
    ActionId, Fighter, MotionEntryFlags,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_mp::CollMap;
use melee_types::{FtPart, GroundOrAir, ItemKind};

/// ftNs_MS_AttackHi4 (342) .. ftNs_MS_AttackLw4Release (347).
pub const UP_START: ActionId = ActionId(342);
pub const UP_CHARGE: ActionId = ActionId(343);
pub const UP_RELEASE: ActionId = ActionId(344);
pub const DOWN_START: ActionId = ActionId(345);
pub const DOWN_CHARGE: ActionId = ActionId(346);
pub const DOWN_RELEASE: ActionId = ActionId(347);
/// Every yo-yo smash row.
pub const ROWS: core::ops::RangeInclusive<u16> = 342..=347;

/// The rows' anim_id column (ftNs_Init_MotionStateTable): ftCo_SM_AttackHi4
/// and ftCo_SM_AttackLw4, then Ness's own charge and release animations
/// (ftNs_SM_AttackHi4Charge = ftCo_SM_Count, 0x127..0x12A).
pub const ANIMATIONS: [i32; 6] = [0x42, 0x127, 0x128, 0x43, 0x129, 0x12A];

/// ftNs_AttackHi4Charge_Enter / ftNs_AttackHi4Release_Enter's motion flags:
/// Ft_MF_SkipItemVis alone.
const CHARGE_FLAGS: MotionEntryFlags = MotionEntryFlags::SKIP_ITEM_VIS;
/// Their animations start at frame 12 and 13.
const CHARGE_START_FRAME: f32 = 12.0;
const RELEASE_START_FRAME: f32 = 13.0;
/// The scratch frame the charge's check falls on, and the release resumes
/// counting from (ftNs_AttackHi4_YoyoSetChargeDamage).
const CHARGE_CHECK_FRAME: i32 = 13;
const RELEASE_FRAME: i32 = 14;
/// ftNs_AttackHi4_YoyoThink_IsRemove: the article spawns at frame 2 and
/// goes at frame 49 (up) or 60 (down).
const SPAWN_FRAME: i32 = 2;
const UP_END_FRAME: i32 = 49;
const DOWN_END_FRAME: i32 = 60;

/// fp->parts indices Ness's code reads directly (no ftParts_GetBoneIndex):
/// joint 61, the yo-yo's anchor in the smash animations; FtPart_XRotN, the
/// slope pivot; FtPart_R2ndNa, the hand.
const ANCHOR_JOINT: usize = 61;
const HAND: FtPart = FtPart::R2ndNa;

/// ft_80088510 ids (ftnessattackhi4.c): the throw, the release, the stop,
/// and the yo-yo going out in the up and down smashes.
const THROW_SOUND: u32 = 210087;
const RELEASE_SOUND: u32 = 210090;
const STOP_SOUND: u32 = 210096;
const UP_OUT_SOUND: u32 = 210099;
const DOWN_OUT_SOUND: u32 = 210093;
/// ftCo_800DEF38: the smash charge's sound.
const CHARGE_SOUND: u32 = 0x7B;
/// ftNs_AttackHi4_YoyoApplySmash: smash_attrs.x2128, the charge's colour
/// animation (ftCo_800BFFD0 unless 0x7B).
const CHARGE_COLOR: u8 = 119;
/// ftNs_AttackHi4_YoyoApplyDamage (@266): "likely 1/256", the damage
/// attribute's scale.
const DAMAGE_SCALE: f32 = 0.003906;
/// ftNs_AttackHi4_YoyoCheckNoObstruct: the ECB-centre sweep's box, and the
/// probe down through the yo-yo's point (@302, @304; @303 is -1).
const CENTRE_SWEEP_BOX: f32 = 0.5;
const FLOOR_PROBE_BOX: f32 = 1.5;
/// ftNs_AttackHi4_YoyoCheckEnvColl's result bits.
const REACH_FLOOR: u32 = 0x8000;
const REACH_LEFT_WALL: u32 = 0x1;
const REACH_RIGHT_WALL: u32 = 0x40;
const REACH_CEILING: u32 = 0x2000;

/// mv.ns.attackhi4 / attacklw4 (ftNess/types.h, fp+2340).
#[derive(Clone, Copy, Debug, Default)]
pub struct YoyoScratch {
    /// x2340 yoyoCurrentFrame: the smash's own frame count.
    pub frame: i32,
    /// x2344 yoyoRehitTimer: frames until hitbox 0 may hit again.
    pub rehit_frames: i32,
    /// x2348 isChargeDisable: A was let go before the charge frame.
    pub charge_disabled: bool,
    /// x234C isPosUpdateMod: the anchor's point is turned with the slope.
    pub slope_adjusted: bool,
}

/// Ness's side of the yo-yo.
#[derive(Clone, Debug, Default)]
pub struct Yoyo {
    pub scratch: YoyoScratch,
    /// u.ns.yoyo_gobj != NULL.
    pub out: bool,
    /// u.ns.yoyo_hitbox_pos: where hitbox 0 goes. It outlives the smash.
    pub point: Vec3,
    /// smash_attrs.x2118_frames while state 4 (the yo-yo charge) runs.
    pub charge_frames: f32,
    /// deal_dmg_cb == ftNs_AttackHi4_YoyoStartTimedRehit.
    pub rehit_callback: bool,
    /// The article's special attributes (ftData.x48_items[10]).
    pub attributes: YoyoAttributes,
    /// The article's string, which both sides step.
    pub string: YoyoString,
}

impl Yoyo {
    pub fn new(attributes: YoyoAttributes) -> Self {
        Self {
            string: YoyoString::with_capacity(&attributes),
            attributes,
            ..Default::default()
        }
    }
}

pub(crate) fn yoyo(f: &mut Fighter) -> &mut Yoyo {
    &mut f.character.get_mut::<Ness>().yoyo
}
fn is_up(f: &Fighter) -> bool {
    (UP_START.0..=UP_RELEASE.0).contains(&f.motion_state.action.0)
}

fn request(f: &mut Fighter, control: ItemControl) {
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::NessYoyo,
        control,
    });
}

/// ft_80088510(fp, id, 127, 64): the fighter's effect channel.
fn play_effect_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Effect,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ftNs_AttackHi4_YoyoSetVarAll (80115534): the scratch for a new smash.
/// The command variables are cleared first, and the hitbox point.
fn reset(f: &mut Fighter) {
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    let y = yoyo(f);
    y.scratch.frame = 1;
    y.scratch.rehit_frames = 0;
    y.scratch.slope_adjusted = true;
    y.point = Vec3::ZERO;
}

/// What every yo-yo entry installs after its motion change: x2222_b2 (the
/// CAPE_TURN_BLOCKED hook reads the rows), deal_dmg_cb =
/// ftNs_AttackHi4_YoyoStartTimedRehit and accessory4_cb =
/// ftNs_AttackHi4_YoyoUpdateHitPos.
fn install_callbacks(f: &mut Fighter) {
    let ness = f.character.get_mut::<Ness>();
    ness.yoyo.rehit_callback = true;
    ness.accessory = crate::init::Accessory::Yoyo;
    f.core.arm_accessory4();
}

/// ftNs_AttackHi4_Enter (80115BB0) / ftNs_AttackLw4_Enter (8011659C).
fn enter_start(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    f.commands.allow_interrupt = false;
    yoyo(f).scratch.charge_disabled = false;
    reset(f);
    f.change_motion_state(state, assets)?;
    f.step_animation(assets);
    install_callbacks(f);
    Ok(())
}

/// ftNs_AttackHi4_YoyoStartTimedRehit (80115C74), deal_dmg_cb: the rehit
/// timer from xB4 (fctiwz).
pub fn start_rehit(f: &mut Fighter) {
    let rate = f.character.get::<Ness>().attributes.yoyo.rehit_frames;
    yoyo(f).scratch.rehit_frames = gekko_math::msl::fctiwz(rate);
}

/// ftNs_AttackHi4_YoyoCheckTimedRehit (80114F0C): while the script's first
/// variable is clear the timer runs; at zero hitbox 0 forgets its victims
/// and is enabled again.
fn count_down_rehit(f: &mut Fighter) {
    if f.commands.variables[0] != 0 {
        return;
    }
    let y = yoyo(f);
    if y.scratch.rehit_frames > 0 {
        y.scratch.rehit_frames -= 1;
        if y.scratch.rehit_frames == 0 {
            f.commands.rehit_hitbox(0);
        }
    }
}

/// ftNs_AttackHi4_YoyoUpdateHitPos (80114EB8), accessory4: an enabled
/// hitbox 0 becomes a world point at the yo-yo's, once that is off the
/// origin (ftColl_8007B8A8).
pub fn place_hitbox(f: &mut Fighter) {
    let point = yoyo(f).point;
    if let Some(hit) = f.commands.hitboxes[0].as_mut() {
        if point.x != 0.0 || point.y != 0.0 {
            hit.world = true;
            hit.descriptor.offset = point;
        }
    }
}

/// lb_8000B1CC(fp->parts[index].joint, NULL, out).
fn joint_position(f: &mut Fighter, index: usize) -> Vec3 {
    let c = &mut f.core;
    melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, index, Vec3::ZERO)
}

/// The hand's point the string hangs from: HSD_JObjSetupMatrix and
/// PSMTXConcat of the FtPart_R2ndNa world matrix with an identity (the
/// article's phys callbacks and it_802C0010).
fn hand_point(f: &mut Fighter) -> Vec3 {
    let joint = f.core.animation.parts[common::part(HAND)].joint;
    let world = *f.core.skeleton.get_mtx(joint);
    let mut point = hsd_types::Mtx::IDENTITY;
    hsd_anim::mtx::mtx_concat(&world, &hsd_types::Mtx::IDENTITY, &mut point);
    Vec3::new(point.0[0][3], point.0[1][3], point.0[2][3])
}

/// ftNs_AttackHi4_YoyoSetUnkPos (80115114): joint 61's point, turned about
/// FtPart_XRotN by the floor's slope (-atan2f(normal.x, normal.y),
/// lbVector_Rotate about z) while x234C is set.
fn anchor_point(f: &mut Fighter) -> Vec3 {
    if !yoyo(f).scratch.slope_adjusted {
        return joint_position(f, ANCHOR_JOINT);
    }
    let anchor = joint_position(f, ANCHOR_JOINT);
    let pivot = joint_position(f, common::part(FtPart::XRotN));
    let normal = f.collision.data.floor.normal;
    // lbVector_Sub, lbVector_Add: fsubs, fadds.
    let offset = Vec3::new(anchor.x - pivot.x, anchor.y - pivot.y, anchor.z - pivot.z);
    let angle = -melee_lb::trigf::atan2f(normal.x, normal.y);
    let turned = melee_lb::vector::rotate_about(offset, melee_lb::vector::Axis::Z, angle);
    Vec3::new(turned.x + pivot.x, turned.y + pivot.y, turned.z + pivot.z)
}

/// The yo-yo's point eased toward the anchor by `t` (ftNs_AttackHi4_
/// YoyoSetHitPosUnk, 801152D0; ftNs_AttackHi4Release_Phys): per component
/// fmadds(anchor, t, point * (1 - t)), retail 801153B8.
fn ease_point(f: &mut Fighter, t: f32) {
    let anchor = anchor_point(f);
    let y = yoyo(f);
    let old = y.point;
    let rest = 1.0 - t;
    y.point = Vec3::new(
        fmadds(anchor.x, t, old.x * rest),
        fmadds(anchor.y, t, old.y * rest),
        fmadds(anchor.z, t, old.z * rest),
    );
}

/// The release's easing weight: `scale * (frame - 14)` in [0, 1]
/// (ftNs_AttackHi4Release_Phys 0.1, ftNs_AttackLw4Release_Phys 0.2).
// Not f32::clamp: retail's `<= 0` turns -0 into +0.
#[allow(clippy::manual_clamp)]
fn release_weight(frame: i32, scale: f32) -> f32 {
    let t = scale * (frame as f32 - RELEASE_FRAME as f32);
    if t >= 1.0 {
        1.0
    } else if t <= 0.0 {
        0.0
    } else {
        t
    }
}

/// ftNs_AttackHi4_YoyoCheckEnvColl (80114FF8): a box `half` times the
/// fighter's y scale on each side swept from `from` to `to`
/// (mpColl_8004730C on a fresh CollData), and the stage it met.
fn reach_flags(f: &Fighter, from: Vec3, to: Vec3, half: f32, map: &mut CollMap) -> u32 {
    use melee_types::mp::{collide, FtCollisionBox};
    let size = half * f.player.scale;
    let bounds = FtCollisionBox {
        top: size,
        bottom: -size,
        left: hsd_types::Vec2::new(-size, 0.0),
        right: hsd_types::Vec2::new(size, 0.0),
    };
    let mut coll = melee_types::mp::CollData::default();
    map.coll_data_init(&mut coll);
    // The inlined push twice: last_pos = from, cur_pos = to.
    coll.last_pos = coll.cur_pos;
    coll.cur_pos = from;
    coll.last_pos = coll.cur_pos;
    coll.cur_pos = to;
    coll.x34_flags.b1234 = 5;
    map.air_collide_box(&mut coll, &bounds);
    let env = coll.env_flags as u32;
    let mut flags = 0;
    if env & collide::FLOOR_MASK != 0 {
        flags |= REACH_FLOOR;
    }
    if env & collide::LEFT_WALL_MASK != 0 {
        flags |= REACH_LEFT_WALL;
    }
    if env & collide::RIGHT_WALL_MASK != 0 {
        flags |= REACH_RIGHT_WALL;
    }
    if env & collide::CEILING_MASK != 0 {
        flags |= REACH_CEILING;
    }
    flags
}

/// ftNs_AttackHi4_YoyoCheckNoObstruct (80115404, inlined in
/// ftNs_AttackHi4_Anim): nothing between the ECB's centre and the yo-yo's
/// point, and a floor under the point within the fighter's y scale.
fn charge_has_room(f: &mut Fighter, map: &mut CollMap) -> bool {
    let ecb = f.collision.data.ecb;
    let position = f.physics.position;
    // retail 8011543C..78: fadds, fmuls, then three fadds.
    let centre = Vec3::new(
        0.0 + position.x,
        0.5 * (ecb.top.y + ecb.bottom.y) + position.y,
        0.0 + position.z,
    );
    let point = yoyo(f).point;
    if reach_flags(f, centre, point, CENTRE_SWEEP_BOX, map) != 0 {
        return false;
    }
    let scale = f.player.scale;
    let mut above = point;
    above.y += scale;
    let mut below = point;
    // retail 801154FC: fmadds(-1.0, scale.y, y).
    below.y = fmadds(-1.0, scale, point.y);
    reach_flags(f, above, below, FLOOR_PROBE_BOX, map) & REACH_FLOOR != 0
}

/// ftNs_AttackHi4_YoyoCreateItem (80115A08): the article at the hand
/// (it_802BE9D8, Item_8026AB54 on FtPart_R2ndNa), its string laid out
/// there (it_802BE65C); the down smash's faces backward. take_dmg_cb and
/// death2_cb become ftNs_Init_OnDamage, and the hitlag pair freezes it.
fn spawn_yoyo(f: &mut Fighter, map: &CollMap) {
    let hand = joint_position(f, common::part(HAND));
    let motion = f.motion_state.action.0;
    // it_802BE9D8: ip->facing_dir *= -1 for ftNs_MS_AttackLw4 (0x159).
    let facing = if motion == DOWN_START.0 {
        -f.physics.facing
    } else {
        f.physics.facing
    };
    let scale = f.player.scale;
    yoyo(f)
        .string
        .lay_out(hand, motion, facing, scale, map);
    let spawn = SpawnItem::attached(ItemKind::NessYoyo, f.player.id, hand, facing);
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: common::part(HAND) as u8,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    yoyo(f).out = true;
    common::install_damage_callbacks(f);
    f.effect_state.article_hitlag = Some(ItemKind::NessYoyo);
}

/// it_802BE958 from Ness's side (ftNs_AttackHi4_YoyoItemDespawn,
/// 80115AF8): the article goes and Ness lets go of it.
pub fn despawn(f: &mut Fighter) {
    if std::mem::take(&mut yoyo(f).out) {
        request(f, ItemControl::Remove);
    }
}

/// it_802C0010 (802C0010): the yo-yo out at the hand, still; the article
/// to motion 1.
fn send_out(f: &mut Fighter) {
    let hand = hand_point(f);
    yoyo(f).string.send_out(hand, Vec3::ZERO);
    request(f, ItemControl::Motion(motion::TETHERED));
}

/// ftNs_AttackHi4_YoyoThink_IsRemove (80115784): the article's frames.
/// True once Ness went to Wait for want of an article (Item_80268B18
/// failing, which the port's item pool does not).
fn step_article(f: &mut Fighter, map: &CollMap) -> bool {
    if f.commands.variables[0] != 0 {
        yoyo(f).scratch.slope_adjusted = false;
    }
    let up = is_up(f);
    let end = if up { UP_END_FRAME } else { DOWN_END_FRAME };
    let frame = yoyo(f).scratch.frame;
    if frame > SPAWN_FRAME && frame <= end {
        if !yoyo(f).out {
            unimplemented!(
                "ftnessattackhi4.c:460: ftNs_AttackHi4_YoyoThink_IsRemove reads the article of a NULL yoyo_gobj"
            );
        }
        // cmd_vars[1] 1 / 2: the spin starts or stops (it_802BE5D8's
        // material animation and u.ns.x223C, drawn only); the stop sounds.
        if f.commands.variables[1] == 1 {
            f.commands.variables[1] = 0;
        }
        if f.commands.variables[1] == 2 {
            f.commands.variables[1] = 0;
            play_effect_sound(f, STOP_SOUND);
        }
        let a = &yoyo(f).attributes;
        let (out_frame, return_frame) = if up {
            (a.up_out_frame, a.up_return_frame)
        } else {
            (a.down_out_frame, a.down_return_frame)
        };
        if frame == out_frame {
            send_out(f);
            play_effect_sound(f, if up { UP_OUT_SOUND } else { DOWN_OUT_SOUND });
        } else if frame == return_frame {
            // it_802BFEC4: the reel-in distance and motion 3. Its phys
            // (it_802BF800) never runs while the return frame is the end
            // frame, which ends the article in this same call.
            yoyo(f).string.motion = motion::REELING;
            request(f, ItemControl::Motion(motion::REELING));
        }
        if frame == end {
            despawn(f);
        }
    } else if frame == SPAWN_FRAME {
        spawn_yoyo(f, map);
    }
    false
}

/// ftNs_AttackHi4_YoyoApplySmash (8011556C): the yo-yo thrown ahead for the
/// charge (it_802BFE5C) and smash_attrs.state 4 from zero frames, with the
/// charge's colour animation and sound.
fn begin_charge(f: &mut Fighter, assets: &FighterAssets) {
    yoyo(f).scratch.frame = 0;
    if yoyo(f).out {
        let facing = f.physics.facing;
        let a = &yoyo(f).attributes;
        // retail 801156C0: fmuls.
        let velocity = Vec3::new(a.release_velocity_x * facing, a.release_velocity_y, 0.0);
        yoyo(f).string.throw(velocity);
        request(f, ItemControl::Motion(motion::SWINGING));
    }
    yoyo(f).charge_frames = 0.0;
    f.core.install_color_overlay_now(CHARGE_COLOR, assets);
    // ftCommon_8007EBAC(fp, 36, 0): rumble.
    play_effect_sound(f, THROW_SOUND);
}

/// ftCo_800DEF38 (800DEF38) with smash_attrs.state 4, ahead of the charge
/// rows' Anim: the frames count without limit; the charge sound once.
fn advance_charge(f: &mut Fighter, assets: &FighterAssets) {
    let y = yoyo(f);
    y.charge_frames += 1.0;
    let frames = y.charge_frames;
    if !f.combat.charge_sound_played && frames >= assets.attacks.charge_sound_frame {
        use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
        f.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: CHARGE_SOUND,
            volume: 127,
            pan: 64,
        });
        f.combat.charge_sound_played = true;
    }
}

/// ftNs_AttackHi4_YoyoApplyDamage (80114F70) / the tail of
/// ftNs_AttackHi4_YoyoSetChargeDamage: a charged release scales hitbox 0
/// while it is newly enabled (state 1) by xB0 over the charge's length,
/// through ftColl_8007ABD0. smash_attrs.state is already cleared, so no
/// smash charge scales it again.
fn charge_damage(f: &mut Fighter) {
    let frames = yoyo(f).charge_frames;
    if frames == 0.0 {
        return;
    }
    let enabled = f.commands.hitboxes[0]
        .as_ref()
        .is_some_and(|hit| hit.phase == melee_coll::hitbox::CapsulePhase::Enabled);
    if !enabled {
        return;
    }
    let attributes = &f.character.get::<Ness>().attributes.yoyo;
    let (length, multiplier) = (attributes.charge_frames, attributes.damage_multiplier);
    let damage = f.commands.hitboxes[0].as_ref().unwrap().descriptor.damage;
    // retail 80114FB0..C8: fdivs, fmsubs(@266, xB0, 1), fmadds(.., t, 1),
    // fmuls, then __cvt_fp2unsigned.
    let t = frames / length;
    let gain = fmsubs(DAMAGE_SCALE, multiplier, 1.0);
    let scaled = damage * fmadds(gain, t, 1.0);
    set_hitbox_damage(f, scaled as u32);
}

/// ftColl_8007ABD0 (8007ABD0) on hitbox 0: the damage as a float, its
/// integer knockback copy, then staling.
fn set_hitbox_damage(f: &mut Fighter, damage: u32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8: smash_attrs.state was cleared by the motion change.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: a smash charge under the yo-yo"
    );
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[0].as_mut().expect("yo-yo hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}

/// The charge rows' entry (ftNs_AttackHi4Charge_Enter 80116178,
/// ftNs_AttackLw4Charge_Enter 801168C4): frame 12 held (rate 0), then
/// ftNs_AttackHi4_YoyoApplySmash.
fn enter_charge(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, CHARGE_FLAGS, CHARGE_START_FRAME, 1.0)?;
    f.step_animation(assets);
    f.core.animation.set_rate(&mut f.core.skeleton, 0.0, false);
    begin_charge(f, assets);
    install_callbacks(f);
    Ok(())
}

/// The release rows' entry (ftNs_AttackHi4Release_Enter 80116494,
/// ftNs_AttackLw4Release_Enter 80116AE8): frame 13, the frame count from
/// 14, the yo-yo back out at the hand (it_802C0010), the sound and the
/// charged damage.
fn enter_release(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, CHARGE_FLAGS, RELEASE_START_FRAME, 1.0)?;
    f.step_animation(assets);
    yoyo(f).scratch.frame = RELEASE_FRAME;
    if yoyo(f).out {
        send_out(f);
    }
    play_effect_sound(f, RELEASE_SOUND);
    charge_damage(f);
    install_callbacks(f);
    Ok(())
}

/// The start rows' Anim (ftNs_AttackHi4_Anim 80115C9C, ftNs_AttackLw4_Anim
/// 80116638): the article's frames, the rehit, the charge at frame 13 and
/// Wait at the end.
fn start_anim(
    f: &mut Fighter,
    charge: ActionId,
    assets: &FighterAssets,
    map: &mut CollMap,
) -> Result<()> {
    f.step_animation(assets);
    yoyo(f).scratch.frame += 1;
    if step_article(f, map) {
        return Ok(());
    }
    count_down_rehit(f);
    let scratch = yoyo(f).scratch;
    if scratch.frame == CHARGE_CHECK_FRAME && !scratch.charge_disabled && charge_has_room(f, map)
    {
        enter_charge(f, charge, assets)?;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, assets)?;
    }
    Ok(())
}

/// The charge rows' Anim (ftNs_AttackHi4Charge_Anim 80115F88,
/// ftNs_AttackLw4Charge_Anim 80116798): the frame count (the spin rate it
/// sets is drawn only), the rehit, and the release at xAC frames.
fn charge_anim(
    f: &mut Fighter,
    release: ActionId,
    assets: &FighterAssets,
) -> Result<()> {
    f.step_animation(assets);
    advance_charge(f, assets);
    yoyo(f).scratch.frame += 1;
    count_down_rehit(f);
    let length = f.character.get::<Ness>().attributes.yoyo.charge_frames;
    if yoyo(f).scratch.frame as f32 >= length {
        enter_release(f, release, assets)?;
    }
    Ok(())
}

/// The release rows' Anim (ftNs_AttackHi4Release_Anim 8011620C,
/// ftNs_AttackLw4Release_Anim 80116958).
fn release_anim(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) -> Result<()> {
    f.step_animation(assets);
    yoyo(f).scratch.frame += 1;
    if step_article(f, map) {
        return Ok(());
    }
    count_down_rehit(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, assets)?;
    }
    Ok(())
}

/// The start rows' IASA (ftNs_AttackHi4_IASA 80115E74): letting go of A
/// rules out the charge; Wait's interrupts once the script allows them.
fn start_input(f: &mut Fighter, p: melee_ft::fighter::state::InputPhase<'_>) {
    if !f.input.current.held.intersects(melee_ft::input::Buttons::A) {
        yoyo(f).scratch.charge_disabled = true;
    }
    callbacks::input::tilt(f, p);
}

/// The charge rows' IASA (ftNs_AttackHi4Charge_IASA 801160B4): A let go
/// releases.
fn charge_input(f: &mut Fighter, release: ActionId, assets: &FighterAssets) {
    if !f.input.current.held.intersects(melee_ft::input::Buttons::A) {
        enter_release(f, release, assets).expect("yo-yo release assets");
    }
}

/// The start rows' Phys and the release rows' tail: ft_80084F3C, then the
/// yo-yo at the anchor (ftNs_AttackHi4_YoyoSetHitPos).
fn anchored_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::ground_friction(f, p);
    let anchor = anchor_point(f);
    yoyo(f).point = anchor;
}

/// The release rows' Phys: ft_80084F3C, then the yo-yo eased from where the
/// charge left it to the anchor until `until`.
fn release_physics(f: &mut Fighter, p: PhysicsPhase<'_>, until: i32, rate: f32) {
    common::ground_friction(f, p);
    let frame = yoyo(f).scratch.frame;
    if frame < until {
        ease_point(f, release_weight(frame, rate));
    } else {
        let anchor = anchor_point(f);
        yoyo(f).point = anchor;
    }
}

/// Every row's Coll (ftNs_AttackHi4_Coll 80115F14 and the rest):
/// ft_80084104, and off the ground the article goes.
fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    callbacks::collision::escape(f, p)?;
    if f.physics.ground_or_air == GroundOrAir::Air {
        despawn(f);
    }
    Ok(())
}

/// itNessyoyo_UnkMotion0..3_Phys (802BEB38..802BED54), the article's phys
/// as Ness's ARTICLE_PHYSICS hook: motion 1 holds the yo-yo at the hitbox
/// point (it_802BF28C), motion 2 swings it (it_802BF4A0); either way the
/// yo-yo's link then gives the point (it_802BFAFC). Motion 0's
/// it_802BF900 only poses the first link's model.
pub fn article_physics(f: &mut Fighter, _assets: &FighterAssets, map: &mut CollMap) {
    let motion_now = yoyo(f).string.motion;
    match motion_now {
        motion::HELD => {}
        motion::TETHERED => {
            let hand = hand_point(f);
            let scale = f.player.scale;
            let y = yoyo(f);
            let point = y.point;
            y.string.tether(&y.attributes, point, hand, scale);
            y.point = y.string.yoyo().position;
        }
        motion::SWINGING => {
            let hand = hand_point(f);
            let scale = f.player.scale;
            let y = yoyo(f);
            y.string.swing(&y.attributes, hand, scale, map);
            y.point = y.string.yoyo().position;
        }
        _ => unimplemented!(
            "itnessyoyo.c:255: itNessyoyo_UnkMotion3_Phys (the yo-yo reeled in, it_802BF800)"
        ),
    }
}
