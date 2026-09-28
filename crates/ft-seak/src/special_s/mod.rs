//! Chain, ftseakspecials.c (80110490..80111FA0), with the chain article
//! (itseakchain.c) whose links Sheik's code drives.
//!
//! The start (349 / 352) spawns the chain in her left hand at frame 22 and
//! throws its tip the next frame. The loop (350 / 353) swings it with the
//! stick until B is released after ten frames; the end (351 / 354) reels
//! it in and removes it. The chain's four hitboxes are Sheik's: the start's
//! script creates them on her hand and var 0 lets the chain place them on
//! its links (ftSk_SpecialS_UpdateHitboxes); they stay live only while the
//! links move fast (ftSk_SpecialS_80110BCC).
pub mod chain;
mod hitboxes;

use crate::{common, init::Sheik};
use chain::{ChainItemAttributes, ChainLink, Payout, SwingInput, SwingState};
use gekko_math::fma::fmadds;
use gekko_math::msl::{fctiwz, sqrtf};
use hsd_types::Vec3;
use it_seak::chain::motion;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags, RetailTrig,
    },
    input::Buttons,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_mp::CollMap;
use melee_types::{FtPart, ItemKind};

/// ftSk_MS_SpecialSStart (349) .. ftSk_MS_SpecialAirSEnd (354).
pub const GROUND_START: ActionId = ActionId(349);
pub const GROUND_LOOP: ActionId = ActionId(350);
pub const GROUND_END: ActionId = ActionId(351);
pub const AIR_START: ActionId = ActionId(352);
pub const AIR_LOOP: ActionId = ActionId(353);
pub const AIR_END: ActionId = ActionId(354);
/// Their submotions in row order (ftSk_SM_SpecialSStart 303, SpecialSEnd
/// 304, SpecialS 305, and the aerial 306..308).
pub const ANIMATIONS: [i32; 6] = [303, 305, 304, 306, 308, 307];
/// ftSk_SpecialS_80110610's pose motions: the loop's, attached at a frame
/// the stick's angle picks.
const ARM_MOTIONS: [i32; 2] = [305, 308];

/// ftSk_SpecialS transition_flags: SkipHit, SkipMatAnim, SkipColAnim,
/// UpdateCmd, SkipItemVis, Unk19, SkipModelPartVis, SkipModelFlags, Unk27.
const TRANSITION_FLAGS: MotionEntryFlags = MotionEntryFlags(
    1 << 3 | 1 << 7 | 1 << 12 | 1 << 14 | 1 << 18 | 1 << 19 | 1 << 22 | 1 << 26 | 1 << 27,
);
/// ftSk_SpecialS_80111830's and the ends' Fighter_ChangeMotionState flags:
/// the start's hitboxes carry on.
const KEEP_HITBOXES: MotionEntryFlags = MotionEntryFlags::SKIP_HIT;

/// ftSk_SpecialS_80110788: ft_PlaySFX ids (the flag words it passes).
const FORWARD_SOUND: u32 = 0x41F48;
const BACKWARD_SOUND: u32 = 0x41F4B;
/// @319 / @318: the pose frame is 4 + 0.0556 * the stick angle.
const POSE_FRAMES_PER_DEGREE: f32 = 0.0556;
const POSE_FIRST_FRAME: f32 = 4.0;
/// ftSk_SpecialS_80111830: ftNs_AttackHi4_YoyoCheckEnvColl's box half size.
const REACH_BOX: f32 = 0.5;
/// fp->parts[FtPart_L3rdNa]: the chain's hand, the FtPart value used as
/// the parts index directly (no ftParts_GetBoneIndex).
const HAND_PART: u8 = FtPart::L3rdNa as u8;
/// fn_802BB44C etc.: the chain hangs 0.1 along the hand joint's z.
const HAND_OFFSET: f32 = 0.1;

/// mv.sk.specials (ftSeak/types.h).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChainScratch {
    /// x0: frames in the current phase.
    pub frames: i32,
    /// x4: B let go during the loop.
    pub released: bool,
    /// x8 / xC: frames until the forward / backward flick sounds again.
    pub forward_cooldown: i32,
    pub backward_cooldown: i32,
    /// x14: the stick's smoothed tilt, weighting the arm pose.
    pub magnitude: f32,
    /// x18: the stick's smoothed angle in degrees (starts at 4).
    pub angle: f32,
    /// x1C: frames the hitboxes stay live.
    pub hit_frames: i32,
    /// x20: slow frames tolerated before the hitboxes go.
    pub grace: i32,
}

/// Sheik's side of the chain: fp->u.sk's chain words, the move scratch and
/// the article's links (whose motion state the item mirrors).
#[derive(Clone, Debug)]
pub struct SpecialSide {
    pub scratch: ChainScratch,
    /// u.sk.x8: the chain is out.
    pub out: bool,
    /// u.sk.xC: the hitbox points the chain placed last.
    pub points: [Vec3; 4],
    /// u.sk.x3C: those points a frame earlier (x/y).
    pub previous_points: [Vec3; 4],
    /// u.sk.lstick_delta: the stick's motion this frame.
    pub stick_delta: Vec3,
    pub chain: Box<ChainData>,
}

/// The chain article's resources and links.
#[derive(Clone, Debug)]
pub struct ChainData {
    pub attributes: ChainItemAttributes,
    /// ftData.x48_items[4] / [5] element 2: the neutral arm poses
    /// ftSk_SpecialS_80110610 blends toward, ground and air.
    pub arm_poses: [Vec<hsd_anim::jobj::JObj>; 2],
    pub links: Vec<ChainLink>,
    /// The item's motion state (it_seak::chain::motion).
    pub phase: u16,
    pub swing: SwingState,
    /// The item's facing, from its spawn.
    pub facing: f32,
    /// Hitboxes the chain disabled (lbColl_80008428 keeps a capsule's data
    /// for a later lbColl_80008434).
    pub parked: [Option<melee_coll::hitbox::HitCapsule>; 4],
}

impl SpecialSide {
    pub fn read(
        archive: &hsd_archive::Archive,
    ) -> std::result::Result<Self, melee_ft::desc::FighterDescError> {
        let root =
            archive
                .public("ftDataSeak")
                .ok_or(melee_ft::desc::FighterDescError::InvalidData {
                    field: "ftDataSeak",
                    reason: "missing",
                })?;
        let attributes = ChainItemAttributes::read(archive, root)?;
        let arm_poses = [
            read_arm_pose(archive, root, 4)?,
            read_arm_pose(archive, root, 5)?,
        ];
        let links = vec![ChainLink::default(); attributes.link_count as usize];
        Ok(Self {
            scratch: ChainScratch::default(),
            out: false,
            points: [Vec3::ZERO; 4],
            previous_points: [Vec3::ZERO; 4],
            stick_delta: Vec3::ZERO,
            chain: Box::new(ChainData {
                attributes,
                arm_poses,
                links,
                phase: motion::HELD,
                swing: SwingState::default(),
                facing: 1.0,
                parked: Default::default(),
            }),
        })
    }
}

/// ftData.x48_items[index][2]: a neutral pose from TransN down.
fn read_arm_pose(
    archive: &hsd_archive::Archive,
    root: u32,
    index: u32,
) -> std::result::Result<Vec<hsd_anim::jobj::JObj>, melee_ft::desc::FighterDescError> {
    use melee_ft::desc::FighterDescError as E;
    let r = archive.reader();
    let items = r.u32(root + 0x48)?;
    let item = r.u32(items + index * 4)?;
    let joint = r.u32(item + 8)?;
    let desc = hsd_archive::desc::JObjDesc::read(archive, joint).map_err(E::Archive)?;
    let (tree, top) =
        hsd_anim::load::load_joint_tree(archive, &desc).map_err(|_| E::InvalidData {
            field: "chain arm pose",
            reason: "joint tree does not load",
        })?;
    Ok(tree
        .depth_first(top)
        .map(|id| tree.get(id).clone())
        .collect())
}

fn side(f: &mut Fighter) -> &mut SpecialSide {
    &mut f.character.get_mut::<Sheik>().special_side
}
fn attributes(f: &Fighter) -> &crate::attributes::ChainAttributes {
    &f.character.get::<Sheik>().attributes.chain
}
fn is_air(f: &Fighter) -> bool {
    matches!(f.motion_state.action, AIR_START | AIR_LOOP | AIR_END)
}

fn chain_request(f: &mut Fighter, control: ItemControl) {
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::SeakChain,
        control,
    });
}

/// The callbacks every chain entry installs: take_dmg_cb / death2_cb
/// (ftSk_Init_80110198) while the chain is out, accessory4 (a no-op,
/// ftSk_SpecialS_8011097C) and the hitlag pair freezing the chain
/// (ftSk_SpecialS_80110EE8 / ChainSomething).
fn install_callbacks(f: &mut Fighter) {
    if side(f).out {
        f.character.get_mut::<Sheik>().damage_callbacks = true;
        f.effect_state.article_hitlag = Some(ItemKind::SeakChain);
    }
}

/// ftSk_SpecialS_Enter / ftSk_SpecialAirS_Enter: the start (in the air
/// with no vertical speed), its first frame, then the scratch
/// (ftSk_SpecialS_80110F70).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.y = 0.0;
    }
    f.change_motion_state(if air { AIR_START } else { GROUND_START }, a)
        .expect("Chain assets");
    f.step_animation(a);
    f.commands.variables[0] = 0;
    let s = side(f);
    s.scratch = ChainScratch {
        angle: 4.0,
        ..Default::default()
    };
    s.out = false;
    s.points = [Vec3::ZERO; 4];
    s.previous_points = [Vec3::ZERO; 4];
    s.stick_delta = Vec3::ZERO;
}

/// ftSk_SpecialSStart_Anim / ftSk_SpecialAirSStart_Anim, with
/// ftSk_SpecialS_CheckInitChain.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let (spawn_frame, start_frames, hit_frames) = {
        let a = attributes(f);
        (a.spawn_frame, a.start_frames, a.hit_frames)
    };
    side(f).scratch.frames += 1;
    let frames = side(f).scratch.frames;
    if frames as f32 == spawn_frame {
        spawn_chain(f, p.map);
        side(f).scratch.hit_frames = fctiwz(hit_frames);
    }
    if frames as f32 == spawn_frame + 1.0 {
        throw_chain(f);
    }
    if frames as f32 > start_frames {
        if is_air(f) {
            enter_air_loop(f, p.assets)?;
        } else {
            enter_ground_loop(f, p.assets, p.map)?;
        }
    }
    Ok(None)
}

/// itSeakChain_Spawn at the left hand (lb_8000B1CC on FtPart_L3rdNa):
/// the handle in the hand (Item_8026AB54) and its links, all in.
fn spawn_chain(f: &mut Fighter, map: &mut CollMap) {
    let part = HAND_PART;
    let hand = common::joint_position(f, usize::from(part));
    let facing = f.physics.facing;
    let spawn = SpawnItem::attached(ItemKind::SeakChain, f.player.id, hand, facing);
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
        catch_item: false,
    });
    let s = side(f);
    s.out = true;
    let chain = &mut *s.chain;
    chain.phase = motion::HELD;
    chain.facing = facing;
    chain.swing = SwingState::default();
    chain.parked = Default::default();
    for link in &mut chain.links {
        // it_802BAF2C: every link at the origin, in, its collision a
        // point (it_802A24D0(link, 1.0)).
        link.reset(map, 1.0);
    }
    install_callbacks(f);
}

/// ftSk_SpecialS_CheckInitChain's next frame, it_802BCFC4: the tip leaves
/// the hand at the throw speed; its collision starts from Sheik's centre
/// (it_8026BB68 -> ftLib_80086990).
fn throw_chain(f: &mut Fighter) {
    let hand = hand_point(f);
    let ecb = &f.collision.data.ecb;
    let position = f.physics.position;
    // retail 800869A4..C8: three fadds.
    let center = Vec3::new(
        position.x + 0.0,
        position.y + 0.5 * (ecb.top.y + ecb.bottom.y),
        position.z + 0.0,
    );
    let s = side(f);
    let chain = &mut *s.chain;
    let speed = chain.attributes.throw_speed * chain.facing;
    let tip = chain.links.last_mut().expect("chain links");
    tip.position = hand;
    tip.collision.cur_pos = center;
    tip.collision.last_pos = tip.collision.cur_pos;
    tip.active = true;
    tip.velocity = Vec3::new(speed, 0.0, 0.0);
    chain.phase = motion::EXTENDING;
    chain_request(f, ItemControl::Motion(motion::EXTENDING));
}

/// The hand joint's point the chain hangs from: PSMTXConcat of the
/// FtPart_L3rdNa world matrix with a 0.1 z translation (fn_802BB44C).
fn hand_point(f: &mut Fighter) -> Vec3 {
    let part = HAND_PART;
    let joint = f.core.animation.parts[usize::from(part)].joint;
    let world = *f.core.skeleton.get_mtx(joint);
    let mut offset = hsd_types::Mtx::IDENTITY;
    offset.0[2][3] = HAND_OFFSET;
    let mut point = hsd_types::Mtx::IDENTITY;
    hsd_anim::mtx::mtx_concat(&world, &offset, &mut point);
    Vec3::new(point.0[0][3], point.0[1][3], point.0[2][3])
}

/// ftSk_SpecialS_80111830: the ground loop at its first frame with the
/// arm posed, every hitbox live again; if the way from Sheik's centre to
/// her hand is blocked, straight to the end.
fn enter_ground_loop(f: &mut Fighter, assets: &FighterAssets, map: &mut CollMap) -> Result<()> {
    f.change_motion_state_with_flags(GROUND_LOOP, assets, KEEP_HITBOXES, 0.0, 1.0)?;
    pose_arm(f, assets, 0.0)?;
    side(f).scratch.frames = 0;
    hitboxes::enable_all(f);
    install_callbacks(f);
    let ecb = &f.collision.data.ecb;
    let position = f.physics.position;
    let center = Vec3::new(
        0.0 + position.x,
        0.5 * (ecb.top.y + ecb.bottom.y) + position.y,
        0.0 + position.z,
    );
    let part = HAND_PART;
    let hand = common::joint_position(f, usize::from(part));
    if reach_blocked(f, center, hand, map) {
        enter_ground_end(f, assets)?;
    }
    Ok(())
}

/// ftNs_AttackHi4_YoyoCheckEnvColl (800BA1A8 region) as Sheik uses it: a
/// small box swept from `from` to `to` meets the stage.
fn reach_blocked(f: &Fighter, from: Vec3, to: Vec3, map: &mut CollMap) -> bool {
    use melee_types::mp::{collide, FtCollisionBox};
    let half = REACH_BOX * f.player.scale;
    let bounds = FtCollisionBox {
        top: half,
        bottom: -half,
        left: hsd_types::Vec2::new(-half, 0.0),
        right: hsd_types::Vec2::new(half, 0.0),
    };
    let mut coll = melee_types::mp::CollData::default();
    map.coll_data_init(&mut coll);
    // push_ecb twice: last_pos = from, cur_pos = to.
    coll.last_pos = coll.cur_pos;
    coll.cur_pos = from;
    coll.last_pos = coll.cur_pos;
    coll.cur_pos = to;
    coll.x34_flags.b1234 = 5;
    map.air_collide_box(&mut coll, &bounds);
    let env = coll.env_flags as u32;
    env & (collide::FLOOR_MASK
        | collide::LEFT_WALL_MASK
        | collide::RIGHT_WALL_MASK
        | collide::CEILING_MASK)
        != 0
}

/// ftSk_SpecialS_80111988: the aerial loop, as the ground one without the
/// reach check.
fn enter_air_loop(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(AIR_LOOP, assets, KEEP_HITBOXES, 0.0, 1.0)?;
    pose_arm(f, assets, 0.0)?;
    side(f).scratch.frames = 0;
    hitboxes::enable_all(f);
    install_callbacks(f);
    Ok(())
}

/// ftSk_SpecialS_80111DF8 / 80111EB4: the end; live hitboxes are
/// refreshed.
fn enter_end(f: &mut Fighter, state: ActionId, assets: &FighterAssets) -> Result<()> {
    f.change_motion_state_with_flags(state, assets, KEEP_HITBOXES, 0.0, 1.0)?;
    side(f).scratch.frames = 0;
    if side(f).scratch.hit_frames != 0 {
        hitboxes::enable_all(f);
    }
    install_callbacks(f);
    Ok(())
}
fn enter_ground_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    enter_end(f, GROUND_END, assets)
}
fn enter_air_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    enter_end(f, AIR_END, assets)
}

/// ftSk_SpecialS_80110490 (80110490): the stick's angle (degrees, facing
/// forward) and tilt, each eased toward by PlCo +44C.
fn update_stick(f: &mut Fighter, assets: &FighterAssets) {
    let stick = f.input.current.stick;
    let smoothing = assets.shield.tilt_smoothing;
    // retail 801104B4: atan2f (not the stick-angle helper).
    let mut radians = melee_lb::trigf::atan2f(stick.y, stick.x * f.physics.facing);
    if radians < 0.0 {
        radians += 2.0 * std::f32::consts::PI;
    }
    // retail 801104D8 / 801104E8: two ordered compares; a NaN passes.
    let degrees = (radians * 57.29578_f32).clamp(0.0, 359.0);
    let s = &mut side(f).scratch;
    let mut delta = degrees - s.angle;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    // retail 80110538: fmadds.
    let mut angle = fmadds(delta, smoothing, s.angle);
    if angle > 360.0 {
        angle -= 360.0;
    } else if angle < 0.0 {
        angle += 360.0;
    }
    s.angle = angle;
    let mut tilt = sqrtf(stick.x * stick.x + stick.y * stick.y);
    if tilt > 1.0 {
        tilt = 1.0;
    }
    // retail 801105F4: fmadds.
    s.magnitude = fmadds(smoothing, tilt - s.magnitude, s.magnitude);
}

/// ftSk_SpecialS_80110610 (80110610): the arm aims the chain; the loop's
/// pose at the stick angle's frame, blended toward the neutral one by the
/// tilt, then over the body by `weight` (ftCo_80091E78's composition).
fn pose_arm(f: &mut Fighter, assets: &FighterAssets, weight: f32) -> Result<()> {
    let air = is_air(f);
    update_stick(f, assets);
    let (magnitude, angle) = {
        let s = &side(f).scratch;
        (s.magnitude, s.angle)
    };
    // retail 80110680: fmadds.
    let frame = fmadds(POSE_FRAMES_PER_DEGREE, angle, POSE_FIRST_FRAME);
    let motion = &assets.motions[&ARM_MOTIONS[usize::from(air)]];
    let sheik = f.character.get::<Sheik>();
    let pose = &sheik.special_side.chain.arm_poses[usize::from(air)];
    let core = &mut f.core;
    core.animation
        .apply_guard_pose::<RetailTrig>(&mut core.skeleton, motion, pose, magnitude, frame, weight)
        .map_err(|e| format!("chain arm pose: {e:?}").into())
}

/// ftSk_SpecialS_Anim / ftSk_SpecialAirS_Anim: the hitboxes follow the
/// chain's speed; ten frames in, a released B ends the loop; otherwise the
/// arm aims.
pub fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hitboxes::park(f);
    f.step_animation(p.assets);
    update_hitboxes(f);
    let minimum = attributes(f).minimum_loop_frames;
    let s = &mut side(f).scratch;
    s.frames += 1;
    if s.frames as f32 > minimum && s.released {
        s.released = false;
        if is_air(f) {
            enter_air_end(f, p.assets)?;
        } else {
            enter_ground_end(f, p.assets)?;
        }
        return Ok(None);
    }
    pose_arm(f, p.assets, 1.0)?;
    Ok(None)
}

/// ftSk_SpecialS_IASA / ftSk_SpecialAirS_IASA: B let go, then the stick's
/// motion (ftSk_SpecialS_80110788).
pub fn loop_input(f: &mut Fighter, _p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        side(f).scratch.released = true;
    }
    read_stick_motion(f);
}

/// ftSk_SpecialS_80110788 (80110788): the stick's motion this frame, the
/// flick sounds, and half the motion once the stick is back in the middle.
fn read_stick_motion(f: &mut Fighter) {
    let current = f.input.current.stick;
    let previous = f.input.previous.stick;
    let facing = f.physics.facing;
    let dx = current.x - previous.x;
    let dy = current.y - previous.y;
    let mut sounds = [None; 2];
    {
        let s = side(f);
        s.stick_delta.x = dx;
        s.stick_delta.y = dy;
        let scratch = &mut s.scratch;
        if scratch.forward_cooldown > 0 {
            scratch.forward_cooldown -= 1;
        } else if (facing == 1.0 && dx > 0.3) || (facing == -1.0 && dx < -0.3) {
            sounds[0] = Some(FORWARD_SOUND);
            scratch.forward_cooldown = 6;
        } else if dy > 0.5 {
            sounds[0] = Some(FORWARD_SOUND);
            scratch.forward_cooldown = 12;
        }
        if scratch.backward_cooldown > 0 {
            scratch.backward_cooldown -= 1;
        } else if (facing == 1.0 && dx < -0.3 && current.x < 0.0)
            || (facing == -1.0 && dx > 0.3 && current.x > 0.0)
        {
            sounds[1] = Some(BACKWARD_SOUND);
            scratch.backward_cooldown = 6;
        }
        if s.out {
            let deadzone = s.chain.attributes.stick_deadzone;
            if current.x.abs() < deadzone && current.y.abs() < deadzone {
                s.stick_delta.x *= 0.5;
                s.stick_delta.y *= 0.5;
            }
        }
    }
    for sound in sounds.into_iter().flatten() {
        common::play_sound(f, sound);
    }
}

/// ftSk_SpecialS_80110BCC (80110BCC): the hitboxes stay live while any of
/// their points moved further than the chain's hit speed this frame, for
/// the attribute's frames at a time; slow, they go once the grace frames
/// (set by a hitlag's end) run out.
fn update_hitboxes(f: &mut Fighter) {
    if !side(f).out {
        return;
    }
    let hit_frames = fctiwz(attributes(f).hit_frames);
    let s = side(f);
    let mut moved = [0.0_f32; 4];
    for (i, distance) in moved.iter_mut().enumerate() {
        let x = s.points[i].x - s.previous_points[i].x;
        let y = s.points[i].y - s.previous_points[i].y;
        // sumOfSquares: separately rounded squares.
        *distance = x * x + y * y;
        s.previous_points[i].x = s.points[i].x;
        s.previous_points[i].y = s.points[i].y;
    }
    let speed = s.chain.attributes.hit_speed;
    let limit = speed * speed;
    if s.scratch.hit_frames > 0 {
        s.scratch.hit_frames -= 1;
        if s.scratch.hit_frames == 0 {
            hitboxes::disable_all_forgetting(f);
        }
    }
    let s = side(f);
    if moved.iter().any(|&d| d > limit) {
        if s.scratch.hit_frames <= 0 {
            s.scratch.hit_frames = hit_frames;
            hitboxes::enable_all(f);
        }
    } else if s.scratch.grace > 0 {
        s.scratch.grace -= 1;
    } else {
        s.scratch.hit_frames = 0;
        hitboxes::disable_all_forgetting(f);
    }
}

/// ftSk_SpecialSEnd_Anim / ftSk_SpecialAirSEnd_Anim: reel in at the
/// retract frame, remove the chain at the remove frame; at the end, Wait
/// or Fall.
pub fn end_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hitboxes::park(f);
    f.step_animation(p.assets);
    let (retract_frame, remove_frame) = {
        let a = attributes(f);
        (a.retract_frame, a.remove_frame)
    };
    side(f).scratch.frames += 1;
    let frames = side(f).scratch.frames as f32;
    if frames < remove_frame {
        if frames == retract_frame {
            // it_802BCF84: reel in (fn_802BB784).
            side(f).chain.phase = motion::RETRACTING;
            chain_request(f, ItemControl::Motion(motion::RETRACTING));
        }
        update_hitboxes(f);
    } else if frames == remove_frame {
        destroy_chain(f);
    } else {
        update_hitboxes(f);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, is_air(f), p.assets)?;
    }
    Ok(None)
}

/// it_802BB20C on the chain (with inlineA0's ftSk_SpecialS_80110E4C):
/// Sheik lets go and the item goes.
pub fn destroy_chain(f: &mut Fighter) {
    if !side(f).out {
        return;
    }
    chain_request(f, ItemControl::Remove);
    chain_gone(f);
}

/// ftSk_SpecialS_80110E4C (80110E4C): ChainSomething's thaw and grace,
/// then Sheik's chain pointer and the damage callbacks go.
pub fn chain_gone(f: &mut Fighter) {
    let s = side(f);
    if s.out {
        s.scratch.grace = 2;
    }
    s.out = false;
    f.character.get_mut::<Sheik>().damage_callbacks = false;
}

/// ftSk_SpecialS_ChainSomething as the post-hitlag callback: after the
/// chain thaws, two frames of grace before slow links drop the hitboxes.
pub fn hitlag_end(f: &mut Fighter) {
    let s = side(f);
    if s.out {
        s.scratch.grace = 2;
    }
}

/// ftSk_SpecialSStart_Phys: ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
/// ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}
/// ftSk_SpecialAirSStart_Phys: gravity only once the script's var 0 is
/// set, and aerial friction.
pub fn start_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        let air = &f.attributes.air;
        let (gravity, terminal) = (air.gravity, air.terminal_velocity);
        common::fall(f, gravity, terminal);
    }
    common::aerial_friction(f);
    common::finish_air(f, &p);
}

/// ftSk_SpecialSStart_Coll: off the floor, the aerial start
/// (ftSk_SpecialS_80111440).
pub fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chain collision assets");
    common::ground_to_air(f, AIR_START, TRANSITION_FLAGS, assets)?;
    install_callbacks(f);
    Ok(())
}
/// ftSk_SpecialAirSStart_Coll: landing, the ground start
/// (ftSk_SpecialS_801114E4).
pub fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chain collision assets");
    common::air_to_ground(f, GROUND_START, TRANSITION_FLAGS, assets)?;
    install_callbacks(f);
    Ok(())
}
/// ftSk_SpecialS_Coll: off the floor, the (ground) end.
pub fn loop_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    enter_ground_end(f, p.assets.expect("Chain collision assets"))
}
/// ftSk_SpecialAirS_Coll: landing, the (aerial) end.
pub fn loop_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    enter_air_end(f, p.assets.expect("Chain collision assets"))
}
/// ftSk_SpecialSEnd_Coll: off the floor, the aerial end
/// (ftSk_SpecialS_80111CB0).
pub fn end_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chain collision assets");
    common::ground_to_air(f, AIR_END, TRANSITION_FLAGS, assets)?;
    install_callbacks(f);
    Ok(())
}
/// ftSk_SpecialAirSEnd_Coll: landing, the ground end
/// (ftSk_SpecialS_80111D54).
pub fn end_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Chain collision assets");
    common::air_to_ground(f, GROUND_END, TRANSITION_FLAGS, assets)?;
    install_callbacks(f);
    Ok(())
}

/// The chain's on_accessory (Item_80269A9C, item link 9) for its motion
/// state: fn_802BB428 (in hand: its model only), fn_802BB44C (paying
/// out), fn_802BB574 (falling off a wall), fn_802BB694 (swinging),
/// fn_802BB784 (reeling in); then it_802BCB88 puts the hitbox points on
/// the links. Returns the chain's new motion state, if any.
pub fn chain_accessory(f: &mut Fighter, _assets: &FighterAssets, map: &mut CollMap) -> Option<u16> {
    if !side(f).out || side(f).chain.phase == motion::HELD {
        return None;
    }
    let hand = hand_point(f);
    let input = SwingInput {
        stick_delta: (side(f).stick_delta.x, side(f).stick_delta.y),
        stick: (f.input.current.stick.x, f.input.current.stick.y),
        hit_frames: side(f).scratch.hit_frames,
        facing: side(f).chain.facing,
    };
    let chain = &mut *side(f).chain;
    let a = &chain.attributes;
    let tip = chain.links.len() - 1;
    let mut next = None;
    let mut disable = false;
    match chain.phase {
        motion::EXTENDING => match chain::extend(&mut chain.links, hand, a, map) {
            Payout::Moving => {}
            Payout::Struck => {
                chain.links[tip].velocity.x *= -a.rebound;
                // it_802BCF2C.
                disable = true;
                next = Some(motion::FALLING);
            }
            Payout::Taut => {
                // it_802BCED4.
                disable = true;
                next = Some(motion::SWINGING);
            }
        },
        motion::FALLING => match chain::fall(&mut chain.links, hand, a, map) {
            Payout::Moving => {}
            Payout::Struck => chain.links[tip].velocity.x *= -a.rebound,
            Payout::Taut => {
                disable = true;
                next = Some(motion::SWINGING);
            }
        },
        motion::SWINGING => {
            chain::swing(&mut chain.links, &mut chain.swing, hand, &input, a, map);
        }
        motion::RETRACTING => {
            if chain::retract(&mut chain.links, hand, a, map) {
                // it_2725_Logic54_PickedUp.
                next = Some(motion::HELD);
            }
        }
        _ => unreachable!("chain motion {}", chain.phase),
    }
    if let Some(phase) = next {
        chain.phase = phase;
    }
    let mut points: melee_types::fixed::FixedVec<(usize, Vec3), 8> = Default::default();
    chain::hitbox_points(&chain.links, chain.attributes.link_count, |i, point| {
        points.push((i, point));
    });
    if disable {
        // ftColl_8007AFF8: every hitbox off, its data kept.
        hitboxes::disable_all(f);
    }
    for (i, point) in points.iter().copied() {
        hitboxes::set_point(f, i, point);
    }
    next
}
