use crate::{
    desc::{ItemAssets, ItemCommonData},
    *,
};
use hsd_types::Vec3;
use melee_cmd::{Command, ScriptState};
use melee_coll::hitbox::HitCapsule;
use melee_types::{GroundOrAir, ItemKind};

/// Checked port storage budget. Retail Item_8026784C does NOT cap hold-kind 8;
/// unlike the admission limits this is deliberately not called a retail bound.
pub const ITEM_CAPACITY: usize = 128;
pub const ITEM_GOBJ_LINK: u8 = 9;
pub const ITEM_GOBJ_PRIORITY: u8 = 0;
pub const ITEM_PROCESS_LINKS: [u8; 10] = [0, 1, 4, 5, 9, 11, 12, 13, 14, 16];
/// Stage offsets consumed by Item_802696CC. Items use a fixed 10,000-unit ceiling.
#[derive(Clone, Copy, Debug)]
pub struct ItemBounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    /// Stage_GetBlastZoneTopOffset: read by kinds that watch the blast zone
    /// themselves (it_802D9714); Item_802696CC's ceiling is fixed.
    pub top: f32,
    /// Stage_UnkSetVec3TCam_Offset: the camera's centre (cam_x_offset,
    /// cam_y_offset).
    pub camera_offset: hsd_types::Vec2,
}
#[derive(Clone, Debug, Default)]
pub struct RayState {
    pub angle: f32,
    pub speed: f32,
    pub scale: f32,
    pub previous_position: Vec3,
}
#[derive(Clone, Debug, Default)]
pub struct HeldState {
    pub opening_pose_frame: usize,
    pub recoil_pose_frame: usize,
    pub visibility: i32,
    pub recoil_frame: usize,
    pub opening_frame: i32,
    pub opening_direction: i32,
    pub shot_pending: bool,
    pub opening_sound_played: bool,
}
#[derive(Clone, Debug, Default)]
pub struct AfterimageState {
    pub secondary_visible: bool,
    pub secondary_position: Vec3,
    pub secondary_rotation: Vec3,
}
/// Requests an item makes of the scene during a proc, drained after it.
#[derive(Clone, Debug)]
pub enum ItemEvent {
    /// efSync_Spawn(id, gobj, &pos): a world-space effect.
    Effect { id: u16, position: Vec3 },
    /// efSync_Spawn(id, gobj, jobj): generators that follow the item's own
    /// root JObj (Toad's spores, 0x4D3).
    OwnEffect { id: u16 },
    /// efSync_Spawn(id, gobj, jobj, &offset) through
    /// efLib_CreateGenerator_AppSRT_SetPos: a model that each update sits
    /// at the item's root JObj plus `offset` (the fire arrow's flame, 0x448).
    FollowingEffect { id: u16, offset: Vec3 },
    /// efAsync kinds 0 (EF_SPAWN_ATTACH) and 3 (EF_SPAWN_ATTACH_PARAM,
    /// the item's facing) on the model root's child (HSD_JObjGetChild):
    /// efSync's generators that follow that joint (the Ice Climbers' ice).
    ChildEffect { id: u16, facing: Option<f32> },
    /// efLib_DestroyAll(gobj): the generators on the item's JObj go, from
    /// the kind's own code (the Thunder Jolt ball's trail) or from
    /// Item_8026A8EC before ItemSwitch's destroy effect (item.c:1991).
    DestroyEffects,
    /// ItemSwitch -> it_8027327C -> it_802787B4 (802787B4): the kind's
    /// destroy effect at the item's root, through it_80278800's zero-range
    /// offset spread (three HSD_Randf draws). `root` is the JObj translation
    /// when it trails the item's position (a collision destroy); otherwise
    /// the item's position when the event is handled.
    DestroyEffect { id: u16, root: Option<Vec3> },
    /// efSync_Spawn(0x3E8, gobj, &pos, &damage): the spark of a hit landing
    /// on the item (it_80270E30).
    HitSpark { position: Vec3, damage: f32 },
    /// efSync_Spawn(0x3EC, item, &pos, item): a slashing hit's spark on a
    /// stage enemy or Pokemon (it_80270E30).
    SlashSpark { position: Vec3 },
    /// Script opcode 10 (it_80278F2C): an effect at a joint whose offset gets
    /// a random spread (it_80278800) when the scene resolves it.
    ScriptEffect(melee_types::combat::GraphicsCommand),
    /// EF_SPAWN_CAMERA_SHAKE: Camera_RequestQuake(kind) at a joint offset.
    Quake {
        kind: u16,
        joint: usize,
        offset: Vec3,
    },
    /// efAsync kind 2: an effect at a joint offset, dispatched from the
    /// item's queue (it_80278800's default path).
    JointEffect { id: u16, joint: usize, offset: Vec3 },
    /// efAsync kinds 1 (EF_SPAWN_POS) and 4 (EF_SPAWN_POS_PARAM) on the
    /// model root: effect `id` at the root's world translation when the
    /// request is processed, with kind 4's float parameter.
    RootEffect { id: u16, parameter: Option<f32> },
    /// efAsync kind 0 (EF_SPAWN_ATTACH) on one of the model's bones
    /// (xBBC_dynamicBoneTable->bones[bone]): efAlt's attached generator.
    BoneGenerator { id: u16, bone: usize },
    /// efLib_Cb_DPtcl from the item's joint animation: generator `id` of
    /// `bank` attached to the model root, whose transform at the AnimAll
    /// step is given.
    JointParticle {
        bank: u8,
        id: u32,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
    },
    /// lb_800119DC: a radial gust.
    Gust {
        center: Vec3,
        frames: i32,
        strength: f32,
        decay: f32,
        phase_step: f32,
    },
    /// lb_800119DC centred on one of the model's bones (lb_8000B1CC on it
    /// when the event is handled): Mr. Game & Watch's Manhole.
    BoneGust {
        bone: usize,
        frames: i32,
        strength: f32,
        decay: f32,
        phase_step: f32,
    },
}

/// The model root's first child in depth-first order, the joint
/// `ItemCore::child_rotation_x` turns.
pub const CHILD_JOINT: usize = 1;
/// Item_StateChangeFlags (it/forward.h) that Item_80268E5C consults.
pub mod state_change {
    pub const ANIM_UPDATE: u32 = 1 << 1;
    /// ITEM_DROP_UPDATE: hitboxes take the throw speed as a damage scale.
    pub const DROP_UPDATE: u32 = 1 << 2;
    /// ITEM_MODEL_UPDATE (it_80274740): the spin stops and its angle resets.
    pub const MODEL_UPDATE: u32 = 1 << 3;
    pub const HIT_PRESERVE: u32 = 1 << 4;
    pub const CMD_UPDATE: u32 = 1 << 8;
}

#[derive(Clone, Debug)]
pub enum ItemScratch {
    Needle(NeedleState),
    Afterimage(AfterimageState),
    Ray(RayState),
    Held(HeldState),
    Bomb(BombState),
    Dosei(DoseiState),
    Heiho(HeihoState),
    Turnip(TurnipState),
    Jolt(JoltState),
    Thunder(ThunderState),
    ClimbersIce(ClimbersIceState),
    Missile(MissileState),
    ChargeShot(ChargeShotState),
    SamusBomb(SamusBombState),
    DinFire(DinFireState),
    Boomerang(BoomerangState),
    Bow(BowState),
    Arrow(ArrowState),
    LinkBomb(LinkBombState),
    Milk(MilkState),
    KoopaFlame(KoopaFlameState),
    Chef(ChefState),
    Rescue(RescueState),
    Judge(JudgeState),
    None,
}
/// Item.xDD4_itemVar.koopaflame (itkoopaflame.c): one flame of Bowser's
/// Fire Breath.
#[derive(Clone, Copy, Debug, Default)]
pub struct KoopaFlameState {
    /// xC: the direction the flame is turned toward, bent by each surface
    /// it touches (itKoopaFlame_Update_Direction).
    pub direction: hsd_types::Vec3,
    /// x18: the velocity's unit vector.
    pub heading: hsd_types::Vec3,
    /// x24: the flight's angle from straight up, clockwise.
    pub angle: f32,
    /// x28: the flight's speed.
    pub speed: f32,
    /// x34: the hitbox's authored size, once read.
    pub hitbox_size: f32,
    /// x3C: the owner's breath left at spawn, as a share of full: the
    /// model's and the hitbox's scale.
    pub scale: f32,
    /// x40: frames flown.
    pub frames: i32,
    /// x44: the flame's generator was made.
    pub effect_spawned: bool,
    /// x48: which of the four flame generators it carries.
    pub effect: i32,
}
/// Mr. Game & Watch's Judgment sign (itgamewatchjudge.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct JudgeState {
    /// it_802C7774's arg4: the face drawn, 0..8 (the sign shows it plus 1).
    pub face: i32,
}
/// Item.xDD4_itemVar.gamewatchchef (itgamewatchchef.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChefState {
    /// x4: which of the five foods it is (its flight's attribute entry).
    pub food: usize,
    /// it_804D6D28 +4C as the launch read it: it_80275158's half-life
    /// scale, for the lifetime the animation callback sets.
    pub half_life_scale: f32,
}
/// Item.xDD4_itemVar.gamewatchrescue (itgamewatchrescue.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct RescueState {
    /// xDD8: the fighter the trampoline was made for; the item is its
    /// `owner` only while this names the same fighter.
    pub fighter: Option<u8>,
}
/// Item.xDD4_itemVar.samusbomb (itsamusbomb.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct SamusBombState {
    /// x4: the fighter the blast can launch; a reflection or the owner's
    /// removal clears it.
    pub owner: Option<u8>,
    /// x0: the blast has yet to test its owner (it_802B5478).
    pub blast_pending: bool,
}
/// Item.xDD4_itemVar.samuschargeshot (itsamuschargeshot.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChargeShotState {
    /// xE00: the fighter that formed the shot.
    pub original_owner: Option<u8>,
    /// xDD8: the flight's angle, in radians.
    pub angle: f32,
    /// xDDC: the flight's speed.
    pub speed: f32,
    /// xDE4 (flight) / the hand's scale: the model grandchild's scale.
    pub scale: f32,
    /// xDE8: fired.
    pub launched: bool,
    /// xDEC / xDF0: the charge level and the full level.
    pub level: i32,
    pub full: i32,
    /// xDF4: the full shot's sparkle counter, 0..2.
    pub sparkle: i32,
    /// xDF8: the damage the charge gives (nothing ported reads it).
    pub damage: u32,
    /// xDFC: the hand glow plays.
    pub glowing: bool,
}
/// Item.xDD4_itemVar.samusmissile (itsamusmissile.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct MissileState {
    /// is_smash_missile: a super missile.
    pub smash: bool,
    /// x4: the owner's missile count (u.ss.x2238) at launch.
    pub launch_count: i32,
    /// x8: the homing turn, the model child's X rotation.
    pub turn: f32,
}
/// Item.xDD4_itemVar.seakneedlethrown (itseakneedlethrown.c): a thrown or
/// dropped needle of Sheik's.
#[derive(Clone, Copy, Debug, Default)]
pub struct NeedleState {
    /// xDD4: the spin added to the needle's model each frame after a bounce
    /// or a drop.
    pub spin: f32,
    /// xDD8: the bounce's or drop's horizontal velocity.
    pub drift: f32,
    /// xDDC: its fall's terminal velocity.
    pub terminal_velocity: f32,
    /// xDE0: its gravity.
    pub gravity: f32,
    /// xDE4: the position before this frame's move, the ray's start.
    pub previous_position: Vec3,
    /// xDF0: the pitch a stuck needle keeps.
    pub pitch: f32,
    /// xDF4: the line it stuck in (-1 before one).
    pub line: i32,
    /// xDF8 / xDFC: that line's angle now and the frame before.
    pub line_angle: f32,
    pub previous_line_angle: f32,
}
/// Item.xDD4_itemVar.zeldadinfire (itzeldadinfire.c) and
/// .zeldadinfireexplode: Zelda's Din's Fire and its explosion.
#[derive(Clone, Copy, Debug, Default)]
pub struct DinFireState {
    /// xDD8 (the fire) / xDD4 (the explosion): the charge, one per frame
    /// of flight up to the attribute.
    pub charge: f32,
    /// xDDC: a reflection took the fire out of its creator's hands.
    pub reflected: bool,
    /// xDE0: the creator, who steers and detonates it while it owns it
    /// (it_802C3D44 clears it).
    pub creator: Option<u8>,
    /// xDE8: the steering's angle offset.
    pub angle_offset: f32,
    /// xDEC: the flight's base angle (0 facing right, pi facing left).
    pub base_angle: f32,
    /// xDF0: the flight's speed.
    pub speed: f32,
    /// xDF4 (the fire) / xDE0 (the explosion): its generators are live.
    pub effects: bool,
    /// The explosion's xDD8: the hitbox's authored size, once read.
    pub hitbox_size: f32,
}
/// Item.xDD4_itemVar.clinkmilk (itclinkmilk.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct MilkState {
    /// x0: the fighter holding it.
    pub parent: Option<u8>,
}
/// What an item stuck in a fighter's shield reads of that fighter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShieldView {
    /// ftLib_80086A18: the fighter is in GuardOn, Guard or GuardSetOff.
    pub guarding: bool,
    /// ftCo_80094098: the shield joint's world position and the bubble's
    /// size now (inlineB0).
    pub center: Vec3,
    pub size: f32,
    /// ftLib_800869D4: the fighter's model scale.
    pub scale: f32,
}
/// Item.xDD4_itemVar.linkbomb (itlinkbomb.c). x8 and xC, the fuse joint's
/// per-frame drop and turn, move bone 3 of the model only.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinkBombState {
    /// x0 b0: the fuse is lit (the model plays article state 3; state
    /// changes keep that animation and only restart the script).
    pub lit: bool,
    /// x0 b1: it rolls along the floor (it_8029F18C).
    pub rolling: bool,
    /// x0 b2: a hit already knocked it about (Logic16_DmgReceived).
    pub knocked: bool,
    /// x4: the roll's direction, the sign of its speed on landing.
    pub roll_direction: f32,
    /// x10: the fighter that pulled it.
    pub puller: Option<u8>,
}
/// Item.xDD4_itemVar.linkbow (itlinkbow.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct BowState {
    /// x0: the model scale (ftLib_800869D4 of the owner at creation).
    pub scale: f32,
    /// x4: the fighter that drew it.
    pub archer: Option<u8>,
}
/// Item.xDD4_itemVar.linkarrow (itlinkarrow.c). The trail models' pose
/// history (x24..x90, xB0, xB4) is drawing only.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArrowState {
    /// x18: the tail (while nocked, the bow hand; in flight, the last
    /// frame's position).
    pub tail: Vec3,
    /// x94: the flight angle (the model's root rotation Z).
    pub angle: f32,
    /// x9C: stuck, the model's remaining wobbles.
    pub wobbles: i32,
    /// xA0: shot.
    pub shot: bool,
    /// xA4 / xA8: the charged damage and speed.
    pub damage: u32,
    pub speed: f32,
    /// xAC: the charge it was shot with.
    pub charge: f32,
    /// xC0: the model scale (ftLib_800869D4 of the archer at creation).
    pub scale: f32,
    /// xE0: the fighter that nocked it.
    pub archer: Option<u8>,
    /// xE4: the line it is stuck in, or -1.
    pub line: i32,
    /// xE8 / xEC: that line's normal angle now and a frame ago.
    pub normal_angle: f32,
    pub previous_normal_angle: f32,
    /// xF0: frames since it faded out (after its stuck lifetime).
    pub faded_frames: i32,
    /// Stuck in a shield: xC8..xD0, the shield's centre (ftCo_80094098),
    /// xD4, its radius at the fighter's scale, and xD8, the arrow's angle
    /// about it.
    pub shield_center: Vec3,
    pub shield_radius: f32,
    pub shield_angle: f32,
}
/// Item.xDD4_itemVar.linkboomerang (itlinkboomerang.c). The trail models'
/// pose history (xDD8..xDDC, xDF0, xEB0, xF90) is drawing only.
#[derive(Clone, Copy, Debug, Default)]
pub struct BoomerangState {
    /// xF98: the fighter that threw it; cleared when that owner goes.
    pub thrower: Option<u8>,
    /// xDEC == FTKIND_LINK: the flight sound's kind.
    pub thrown_by_link: bool,
    /// xDE8: a reflector sent it back; it no longer steers.
    pub reflected: bool,
    /// xF70: frames before the next trail model starts.
    pub trail_timer: f32,
    /// xF74: the flight angle (radians).
    pub angle: f32,
    /// xF78: the angle turned for the facing (the model's root rotation Z).
    pub facing_angle: f32,
    /// xF7C: -sin of the attribute angle; a surface met straighter than
    /// this sends it back instead of glancing off.
    pub glance_limit: f32,
    /// xF80: frames the return still homes on the thrower.
    pub homing_frames: f32,
    /// xF84: the most the return turns per frame.
    pub turn_limit: f32,
    /// xF88 / xDE4: the return's model spin and its frames (drawing).
    pub spin_step: f32,
    pub spin_frames: i32,
    /// xF8C: frames to the next flight sound.
    pub sound_timer: f32,
}
/// Item.xDD4_itemVar.pikachuthunder (itpikachuthunder.c): one bolt of
/// Pikachu's Thunder chain; the partner is the next bolt (x34).
#[derive(Clone, Copy, Debug, Default)]
pub struct ThunderState {
    /// x0: the bolt's place in the chain; the lead is 0.
    pub index: i32,
    /// x4: the bolt struck (its lead reached it, or it met the floor).
    pub struck: bool,
    /// x8: frames before it starts falling.
    pub delay: i32,
    /// xC / x10: the bolt's length before and after this frame's shrink.
    pub length: f32,
    pub next_length: f32,
    /// x14: the struck bolt's lifetime.
    pub strike_frames: f32,
    /// x18: its model's y scale.
    pub scale: f32,
    /// x1C: the fall's velocity.
    pub velocity: Vec3,
    /// x28: where the bolt before it stopped.
    pub reached: Vec3,
}
/// Item.xDD4_itemVar.pikachujoltground and .pikachujoltair
/// Item.xDD4_itemVar.climbersice (itclimbersice.c): an ice block.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClimbersIceState {
    /// x4: the model child's scale, which shrinks once the block melts.
    pub scale: f32,
    /// x8_b0: launched (it_802C16F8). Before that the block is its owner's
    /// fp->u.pp.x222C.
    pub launched: bool,
    /// ItCo's half-life scale, which it_80275158 reads at the launch.
    pub half_life_scale: f32,
}

/// (itpikachutjoltground.c / itpikachutjoltair.c): the Thunder Jolt ball
/// and the crawler it rides along a surface.
#[derive(Clone, Copy, Debug, Default)]
pub struct JoltState {
    /// Ball xDD4: the flight's angle.
    pub angle: f32,
    /// Ball xDE8: the position before this frame's move, the ray's start.
    pub previous_position: Vec3,
    /// Ball xDF4, crawler xDE8: the surface's normal (the crawler's
    /// rotation follows it).
    pub normal: Vec3,
    /// Ball xE00: the normal before the last contact.
    pub previous_normal: Vec3,
    /// Ball xDE4: frames since the last contact; crawler xDF4: frames on
    /// the current surface.
    pub frames: i32,
    /// Ball xDE0: its trail generator (efSync 0x4BD) still plays.
    pub trail: bool,
}
/// Item.xDD4_itemVar.heiho (itheiho.c): a Yoshi's Story Shy Guy.
#[derive(Clone, Debug)]
pub struct HeihoState {
    /// x20: its place in the spawned group.
    pub group_index: i8,
    /// x21: which walking speed (special attributes x4..xC) it uses.
    pub speed_variant: i8,
    /// x24: frames left of the spawn delay, the stun, or the turn.
    pub countdown: i32,
    /// x22: it has been inside the blast zones once (it_802D9714).
    pub entered_screen: bool,
    /// x3C: the gait joint's last reading (itUpdateVelocityFromBone).
    pub bone_previous: Vec3,
    /// Where the gait joint's animation stands: its rate schedule and the
    /// steps taken since the state began.
    pub bone_rate: crate::bone_motion::BoneRate,
    pub bone_step: usize,
}
impl Default for HeihoState {
    fn default() -> Self {
        Self {
            group_index: 0,
            speed_variant: 0,
            countdown: 0,
            entered_screen: false,
            bone_previous: Vec3::ZERO,
            bone_rate: crate::bone_motion::BoneRate::Steady,
            bone_step: 0,
        }
    }
}
/// Item.xDD4_itemVar.dosei (itdosei.c): Mr. Saturn.
#[derive(Clone, Copy, Debug, Default)]
pub struct DoseiState {
    /// xDD4: frames it sits after landing from a spawn before looking round.
    pub idle_countdown: i32,
    /// xDD8: the look-round's phase (1 turning, 2 walking off).
    pub turn_phase: i32,
    /// xDDC: the look-round's angle, in radians.
    pub turn_angle: f32,
    /// xDE0: the scale a throw restores (it_80274484); Peach's pull stores
    /// the item's scale here (it_802BD4AC).
    pub throw_scale: f32,
    /// xDE4: the position at the start of the frame, which a turn returns to.
    pub last_position: Vec3,
    /// xDF0: frames it lies after being hit before walking again.
    pub recover_timer: i32,
    /// xDF8: the floor normal when it slid off the ground.
    pub floor_normal: Vec3,
}
/// Item.xDD4_itemVar.peachturnip (itpeachturnip.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct TurnipState {
    /// xDD8: the face (article state 0's animation frame).
    pub face: i32,
    /// xDDC: the face's damage, which the thrown hitbox takes.
    pub damage: i32,
    /// xDD4 b0: it has been in a hand before (a later pickup plays state 4).
    pub picked_up: bool,
    /// xDE0: the scale when pulled, which each throw restores.
    pub scale: f32,
}
/// Item.xDD4_itemVar.bombhei (itbombhei.c).
#[derive(Clone, Debug, Default)]
pub struct BombState {
    /// xDD4: the current phase's frame countdown.
    pub countdown: i32,
    /// xDD8: the blink's scale direction, +1 or -1.
    pub blink_direction: i32,
    /// xDDC: it_80280B60 has run.
    pub exploded: bool,
    /// xDE0: the fuse is lit.
    pub lit: bool,
    /// xDE4: walked before (it_8027E978).
    pub walked: bool,
    /// xDE8: the thrown scale factor.
    pub throw_scale: f32,
    /// xDEC: frames left in the whole life, counting every phase.
    pub life_frames: f32,
    /// xDF0: frames left on the lit fuse.
    pub fuse: f32,
    /// xDF4: the facing to restore after turning.
    pub turn_facing: f32,
    /// xDF8 / xDFC: per-frame squash and tilt while waking.
    pub squash_step: f32,
    pub tilt_step: f32,
    /// xE04: the per-frame turn.
    pub turn_step: f32,
    /// xE0C: the velocity before this frame's landing.
    pub landing_velocity: Vec3,
}
#[derive(Clone, Debug)]
pub struct ItemCore {
    pub ground_or_air: melee_types::GroundOrAir,
    pub id: u32,
    pub kind: ItemKind,
    pub owner: Option<u8>,
    /// The owner is its player's second fighter (x221F_b4: the Ice
    /// Climbers' Nana). Retail names the owner by its fighter GObj; the
    /// player slot and this flag together do the same.
    pub owner_secondary: bool,
    pub stale_source: Option<melee_types::combat::AttackInstance>,
    /// Current owner factor used only when authoring/re-authoring a hitbox.
    pub stale_multiplier: f32,
    pub pending_reflection: Option<PendingReflection>,
    /// xC90_absorbGObj: the fighter whose absorbing bubble took a hitbox
    /// this frame (ftColl_8007925C), for the kind's absorbed callback.
    pub pending_absorb: Option<u8>,
    pub reflection_direction: f32,
    pub reflection_history: [melee_types::fixed::FixedVec<RehitVictim, 12>; 4],
    /// xAC4_ignoreItemID: members of one hit group share victim histories
    /// (see `hit_group`); 0 for none.
    pub hit_group: u32,
    /// Hitboxes (bits) a script started in a hit group, awaiting
    /// it_8026FCF8's copy of the group's history.
    pub(crate) group_history_pending: u8,
    pub hold_kind: u8,
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    /// x70: this frame's push (see [`crate::push`]).
    pub nudge: Vec3,
    /// xDC8 x1A (ItemAttr x1_4): grounded, it can be pushed.
    pub pushable: bool,
    /// xDC8 x1C / x1D / x1E: fighters and other items push it, and a hold
    /// kind 3 item may push it; all set at creation.
    pub pushed_by_fighters: bool,
    pub pushed_by_items: bool,
    pub pushed_by_open_palm: bool,
    /// xDC8 x1B: its animation proc has run this frame (it_80272298).
    pub push_settled: bool,
    /// xBEC (ItemAttr x20, scaled by it_80274E44): its push box.
    pub push_box: melee_types::mp::ItEcb,
    /// xDD1 b0: it_8027518C started its explosion lifetime; items no
    /// longer push off it.
    pub exploding: bool,
    pub environmental_velocity: Vec3,
    pub platform_velocity: Vec3,
    pub facing: f32,
    pub rotation: Vec3,
    pub model_scale: Vec3,
    pub motion: u16,
    pub life_timer: f32,
    /// xD48: the half-life it_80275158 sets beside the lifetime; a
    /// reflection (it_80273030) restarts the lifetime from it.
    pub half_life: f32,
    pub scale: f32,
    pub frozen: bool,
    pub destroyed: bool,
    pub pending_damage_dealt: i32,
    pub pending_damage_without_hitlag: i32,
    /// ftColl_80077688's xC50; shield contact has priority at item link 14.
    pub pending_shield_damage: i32,
    pub pending_shield_deflection: Option<melee_lb::shield::ShieldDeflection>,
    /// xCC0: a counter-style shield's own hitlag for this item (the
    /// defender's shield_unk1), which overrides the damage formula.
    pub pending_shield_hitlag: f32,
    /// xC48: the strongest clank against a fighter hitbox this frame
    /// (ftColl_80077970).
    pub pending_clank_damage: i32,
    /// ItemAttr x1_5 (xDC8 xC): contacts put this kind into hitlag.
    pub hitlag_enabled: bool,
    /// xCA8: this frame's contact damage that sets hitlag. Event callbacks
    /// may clear it (itFoxIllusion_Logic14_DmgDealt).
    pub hitlag_damage: i32,
    /// xCBC: hitlag frames left.
    pub hitlag_frames: f32,
    /// xDC8 x9: animation, physics, movement and accessory are paused.
    pub in_hitlag: bool,
    pub sound_requests: melee_types::fixed::FixedVec<u32, 8>,
    pub animation_frame: f32,
    /// x5D0_animFrameSpeed: the joint animation's per-frame advance.
    pub animation_rate: f32,
    /// xDCF b2 (it_8027518C): the item ends without its destroy effect.
    pub destroy_effect_suppressed: bool,
    /// Item_8026A8EC's efLib_DestroyAll already ran in the proc that ended
    /// the item.
    pub effects_destroyed: bool,
    pub script: ScriptState,
    pub command_variables: [u32; 4],
    pub hitboxes: [Option<HitCapsule>; 4],
    pub scratch: ItemScratch,
    pub hit_flags: [desc::ItemHitFlags; 4],
    /// x378_itemColl, created by it_80275E98 at spawn.
    pub collision: Option<melee_types::mp::CollData>,
    /// xDC8 x17: the model rotation axis the fixed ECB follows (0 Z, 1 X, else Y).
    pub rotation_axis: u8,
    /// xC30: the floor (or wall/ceiling) line of the last map contact.
    pub floor_line: i32,
    /// xD5C: it_80277544's fall-through-platform state.
    pub platform_drop: u32,
    /// xCF4 (ftColl_80077688): the fighter whose shield took this frame's
    /// strongest hit.
    pub pending_shield_owner: Option<u8>,
    /// A fighter whose shield the item reads each frame (the Link arrow's
    /// xC4), and the scene's view of it (or of `pending_shield_owner` for
    /// the hit-shield callback) refreshed before the item's procs.
    pub shield_anchor: Option<u8>,
    pub shield_view: Option<ShieldView>,
    /// xD50: landings since the last throw or bounce-free touchdown
    /// (it_8026DDFC, it_8026DD5C).
    pub land_count: u32,
    /// xD54: throws since creation (it_80273F34).
    pub throw_count: u32,
    /// JOBJ_HIDDEN on the model (it_80280B60).
    pub hidden: bool,
    /// spin_spd: ItemAttr xC, degrees-per-frame scale of the spin.
    pub spin_rate: f32,
    /// xD3C_spinSpeed: radians added to the model rotation per airborne frame.
    pub spin_speed: f32,
    /// xDC8 x19: the spin keeps its sign regardless of facing.
    pub spin_ignores_facing: bool,
    /// xDC8 x15 (it_8026B390 / it_8026B3A8): Item_IsGrabbable requires it,
    /// and it gates the common lifetime countdown (Item_80269528).
    pub grabbable: bool,
    /// xBCC / xBD4 (ItemAttr x30 / x38): the pickup box's offset from the
    /// item's position and its half extents (ftpickupitem_800942A0).
    pub grab_offset: hsd_types::Vec2,
    pub grab_range: hsd_types::Vec2,
    /// The model root JObj's translation, which trails `position`: the
    /// collision proc (Item_80269978) and a release from the hand
    /// (it_80273748 / it_80273B50) copy `position` into it.
    pub root_translation: Vec3,
    /// Item.x3C: the size of the last hitbox a script command made, before
    /// it_80275594 divides the capsule by the scale.
    pub hitbox_size: f32,
    /// HSD_JObjSetRotationX / AddRotationX on the model root's first child
    /// (depth-first joint 1), for kinds whose code turns that joint (Sheik's
    /// needles); None leaves the joint to its animation.
    pub child_rotation_x: Option<f32>,
    /// xDC8 x13 (it_802742F4): held by its owner. A held item neither moves
    /// nor leaves the blast zones.
    pub held: bool,
    /// xDC4: the holder's part it hangs from, a fp->parts index (the
    /// character's own joint numbering, as ftData x8 stores it).
    pub holder_part: u8,
    /// xDCD b5 (it_80275444 / it_80275474): hitboxes may hit the owner.
    pub hits_owner: bool,
    /// xC44: the speed the item was thrown or dropped at (Item_8026AD20).
    pub throw_speed: f32,
    /// xC40: the damage scale new hitboxes take, the throw speed after a
    /// DROP_UPDATE state change and 1 otherwise (Item_80268E5C).
    pub hitbox_damage_scale: f32,
    /// HSD_GObj_804D7838->s_link > 11: the running proc comes after item
    /// link 11's capsule refresh (it_802790C0).
    pub past_hitbox_refresh: bool,
    /// xD0C == 2 (it_802756D0 / it_802756E0): hurtboxes take no hits.
    pub hurt_intangible: bool,
    /// xD40 with xDD0 b6 (item.c foobar): creation's intangible frames
    /// still to run, counted down in Item_80269528.
    pub spawn_intangible_frames: Option<f32>,
    /// xDD0 b7: the item was already intangible at creation, so the
    /// countdown's end leaves it so.
    pub keeps_intangible: bool,
    /// xDC8 x14: set when a throw ends the hold (it_80273F34), cleared by
    /// the next state change or a pickup; it_8026B1D4 then adds speed to
    /// the item's contact damage.
    pub speed_damage: bool,
    /// xDCE b0 (it_802754D4): the owner's hits land too, once dropped or thrown.
    pub hurt_by_owner: bool,
    /// xDCD b7 (it_80275444 / it_80275474): hitboxes reach items whose owner
    /// matches (both unowned counts), as a blast does.
    pub strikes_kindred_items: bool,
    /// Damage taken: xC9C (its percent, capped at 999), and this frame's
    /// total xCA0 and largest hit xCA4.
    pub damage_percent: i32,
    pub pending_damage_taken: i32,
    pub largest_damage_taken: i32,
    /// it_80270E30's result for this frame: xCC8, xCAC, xCCC and xCB0.
    pub pending_knockback: f32,
    pub knockback_angle: u16,
    pub hit_direction: f32,
    pub hit_by: Option<u8>,
    /// efAsync requests queued below s_link 9, flushed at link 9 (Item_80269A9C).
    pub queued_events: melee_types::fixed::FixedVec<ItemEvent, 4>,
    /// A fighter proc at s_link 9 or later drives this item (an owner's
    /// accessory4): efAsync_Spawn dispatches at once (efasync.c:1458).
    pub efasync_immediate: bool,
    pub events: melee_types::fixed::FixedVec<ItemEvent, 8>,
    /// xDCC b3 (Item_80268B18 sets it): Item_802696CC removes the item past
    /// the blast zones. A Shy Guy clears it until it has been on screen.
    pub blast_zone_checked: bool,
    /// The other item this one points at (see [`crate::LinkRequest`]).
    pub partner: Option<u32>,
    /// x520_cameraBox: the camera's subject for an item it frames (item.c
    /// foobar3), placed at the item each accessory proc (Item_80269A9C) and
    /// unlinked when the item goes (Item_80267454).
    pub camera: Option<melee_cm::Subject>,
    /// Requests for linked items, delivered once the proc returns.
    pub link_requests: melee_types::fixed::FixedVec<crate::LinkRequest, 4>,
    /// A request for a fighter (the owner, or the thrower a reflector took
    /// the article from), delivered once the proc returns.
    pub owner_request: Option<(u8, crate::OwnerRequest)>,
    /// HSD_JObjAnimAll steps since the article state's animation began
    /// (Item_80268D34's HSD_JObjReqAnimAll), for [`crate::pose::ItemPose`].
    pub pose_steps: u32,
    /// The article state the motion plays (its ItemStateTable anim_id).
    pub article_state: usize,
    /// Each hitbox slot's last placed positions (HitCapsule x58 and x4C:
    /// previous, current), which a cleared slot keeps in retail until a
    /// new capsule there is first placed.
    pub hitbox_trace: [(Vec3, Vec3); 4],
}
/// lb_8000B804: the model root's authored rotation.
pub(crate) fn rest_rotation(assets: &ItemAssets) -> Vec3 {
    let r = assets.visual.model.rotation;
    Vec3::new(r.x, r.y, r.z)
}
/// The tag that keeps item victim ids apart from fighter spawn numbers.
const ITEM_VICTIM: u32 = 1 << 31;

impl ItemCore {
    /// checkHitLag / EnterHitlagThink (Item_8026A294): hitlag frames from
    /// it_8026B424 (retail 8026B450: fmadds, then fctiwz), never lowered.
    fn enter_hitlag(&mut self, common: &desc::ItemCommonData, damage: i32) {
        let frames = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(
            damage as f32,
            common.hitlag_scale,
            common.hitlag_base,
        )) as f32;
        self.enter_hitlag_frames(frames);
    }
    /// checkHitLag (Item_8026A294): raise the hitlag to `frames`.
    fn enter_hitlag_frames(&mut self, frames: f32) {
        if self.hitlag_frames < frames {
            self.hitlag_frames = frames;
        }
        // No supported kind has an entered_hitlag callback.
        self.in_hitlag = true;
    }
    /// efAsync_Spawn on the item's queue (xBC0): below s_link 9 the request
    /// waits for the item's link 9 flush (Item_80269A9C); the event
    /// callbacks run past it (`past_hitbox_refresh`) and dispatch at once.
    /// efAsync_Spawn links a queued request at the head, so the flush
    /// (efAsync_QueueFlush) processes the latest request first.
    pub fn spawn_async(&mut self, event: ItemEvent) {
        if self.past_hitbox_refresh || self.efasync_immediate {
            self.events.push(event);
        } else {
            self.queued_events.insert(0, event);
        }
    }
    /// it_80272860 (80272860): accelerate while below `limit` or while the
    /// velocity still points against gravity. There is no clamp at the limit.
    pub fn fall(&mut self, acceleration: f32, limit: f32) {
        let gravity_sign = if acceleration < 0.0 { -1 } else { 1 };
        let speed = self.velocity.y;
        let velocity_sign = if speed < 0.0 { -1 } else { 1 };
        if velocity_sign == gravity_sign || gekko_math::msl::fabsf(speed) < limit {
            self.velocity.y -= acceleration;
        }
    }
    /// How fighter hitboxes list this item among their victims. Retail
    /// stores gobj pointers; fighters are keyed by spawn number, items by
    /// their id with the top bit set.
    pub fn hitbox_victim(&self) -> u32 {
        ITEM_VICTIM | self.id
    }
    /// ftColl_80077C60 keeps the largest dealt damage until item link 14.
    pub fn record_damage_dealt(&mut self, damage: f32) {
        if damage > self.pending_damage_dealt as f32 {
            self.pending_damage_dealt = gekko_math::msl::fctiwz(damage);
        }
    }
    /// it_802742F4 (802742F4), Item_8026AB54's attachment before the kind's
    /// pickup callback: the model returns to its rest pose at the item's
    /// scale, `owner` holds it at `part`, pickup and the lifetime countdown
    /// stop, and a common item's lifetime restarts at `lifetime`.
    /// it_80273168's pickup sound and the hand constraint are presentation.
    pub fn attach_to_holder(
        &mut self,
        owner: u8,
        part: u8,
        assets: &ItemAssets,
        lifetime: f32,
        half_life_scale: f32,
    ) {
        // lb_8000B804, then Item_8026849C.
        self.rotation = rest_rotation(assets);
        self.model_scale = Vec3::new(self.scale, self.scale, self.scale);
        // ftLib_80086960: fighters only; xDC8 x0 (a secondary owner) is
        // never set on a pickup-capable kind here.
        self.owner = Some(owner);
        self.held = true;
        self.holder_part = part;
        // it_802756D0: a held item takes no hits.
        self.hurt_intangible = true;
        // it_8026B3A8.
        self.grabbable = false;
        // The pickup (it_26B1.c:597) clears xDC8 x14.
        self.speed_damage = false;
        if (self.kind as u32) < ItemKind::LGunRay as u32 {
            // it_80275158: both timers.
            self.life_timer = lifetime;
            self.half_life = lifetime * half_life_scale;
        }
    }
    pub fn change_motion(&mut self, motion: u16, assets: &ItemAssets) {
        self.motion = motion;
        self.article_state = usize::from(motion);
        self.animation_frame = 0.0;
        self.script = ScriptState::default();
        if assets
            .scripts
            .get(motion as usize)
            .is_some_and(|s| !s.is_empty())
        {
            self.script.restart(0);
            self.advance_script(assets);
        }
    }
    /// it_80274740 (80274740): the spin joint's angle about xDC8 x17's axis
    /// and the spin speed return to zero.
    pub fn reset_spin(&mut self) {
        self.spin_speed = 0.0;
        match self.rotation_axis {
            0 => self.rotation.z = 0.0,
            1 => self.rotation.x = 0.0,
            _ => self.rotation.y = 0.0,
        }
    }
    /// Item_80268D34 then HSD_JObjAnimAll (itlinkbomb.c's lit fuse): the
    /// model plays `article_state`'s joint animation from its first frame
    /// while the motion state and its script stay.
    pub fn play_article_animation(&mut self, article_state: usize, assets: &ItemAssets) {
        self.article_state = article_state;
        self.animation_frame = 0.0;
        self.pose_steps = 1;
        self.emit_particle_keys(assets, self.root_translation);
    }
    /// it_80272C6C == 0: the joint animation the model plays has no frames
    /// left.
    pub fn article_animation_ended(&self, assets: &ItemAssets) -> bool {
        assets
            .animation_ends
            .get(self.article_state)
            .copied()
            .flatten()
            .is_some_and(|end| self.animation_frame >= end)
    }
    /// it_80274658 (80274658): the spin speed for `degrees` per frame at
    /// spin_spd, reversed when the item moves against its facing and, unless
    /// xDC8 x19 is set, signed by the facing. retail 80274674..7C: fmuls
    /// 0.01*spin_spd and (pi/180)*degrees separately, then their product.
    pub fn update_spin(&mut self, degrees: f32) {
        let mut speed = 0.0;
        if self.spin_rate != 0.0 {
            speed = (0.01 * self.spin_rate) * (0.017_453_292 * degrees);
        }
        self.spin_speed = speed;
        let facing = if self.facing < 0.0 { -1 } else { 1 };
        let moving = if self.velocity.x < 0.0 { -1 } else { 1 };
        if moving != facing {
            self.spin_speed = -self.spin_speed;
        }
        if !self.spin_ignores_facing {
            self.spin_speed *= -self.facing;
        }
    }

    /// it_80274A64 (80274A64): add the spin about xDC8 x17's axis (retail
    /// 80274AE4 and siblings: fadds into the JObj rotation).
    pub fn spin(&mut self) {
        match self.rotation_axis {
            0 => self.rotation.z += self.spin_speed,
            1 => self.rotation.x += self.spin_speed,
            _ => self.rotation.y += self.spin_speed,
        }
    }

    /// Item_80268E5C (80268E5C) with its flags: the frame restarts; without
    /// HIT_PRESERVE any live hitbox is cleared (it_802725D4); ANIM_UPDATE or
    /// CMD_UPDATE restart the script, which then runs its first step; a state
    /// without an article row has no script. Other flags only touch the model.
    pub fn change_motion_with(
        &mut self,
        motion: u16,
        article_state: i32,
        flags: u32,
        assets: &ItemAssets,
    ) {
        self.motion = motion;
        // Item_80268E5C: HSD_JObjSetTranslate(jobj, &pos) first.
        self.root_translation = self.position;
        self.animation_frame = 0.0;
        self.speed_damage = false;
        if flags & state_change::MODEL_UPDATE != 0 {
            self.reset_spin();
        }
        // item.c:1205 HSD_JObjSetFacingDirItem, after the model reset.
        self.face_spin_axis();
        self.hitbox_damage_scale = if flags & state_change::DROP_UPDATE != 0 {
            self.throw_speed
        } else {
            1.0
        };
        if flags & state_change::HIT_PRESERVE == 0 && self.hitboxes.iter().any(Option::is_some) {
            self.hitboxes.fill(None);
            for history in &mut self.reflection_history {
                history.clear();
            }
        }
        if article_state < 0 {
            self.script = ScriptState::default();
            return;
        }
        self.article_state = article_state as usize;
        // Item_80268D34's HSD_JObjReqAnimAll(0) with ANIM_UPDATE, then the
        // HSD_JObjAnimAll every state change takes.
        self.pose_steps = if flags & state_change::ANIM_UPDATE != 0 {
            1
        } else {
            self.pose_steps + 1
        };
        self.emit_particle_keys(assets, self.position);
        if flags & (state_change::ANIM_UPDATE | state_change::CMD_UPDATE) != 0 {
            self.script = ScriptState::default();
            if assets
                .scripts
                .get(motion as usize)
                .is_some_and(|s| !s.is_empty())
            {
                self.script.restart(0);
            }
        }
        self.advance_script(assets);
    }
    /// item.c foobar (Item_80268B18): common and hold-kind 6 items start
    /// intangible for ItCo +2C frames. The it_80279B88 colour flash is visual.
    fn begin_spawn_intangibility(&mut self, frames: f32) {
        if matches!(self.hold_kind, 0 | 6) {
            self.keeps_intangible = self.hurt_intangible;
            self.spawn_intangible_frames = Some(frames);
            self.hurt_intangible = true;
        }
    }
    /// Item_80269528: the creation countdown runs even while frozen; at its
    /// end it_802756E0 restores hits unless the item began intangible.
    fn count_down_spawn_intangibility(&mut self) {
        let Some(frames) = &mut self.spawn_intangible_frames else {
            return;
        };
        *frames -= 1.0;
        if *frames <= 0.0 {
            self.spawn_intangible_frames = None;
            if !self.keeps_intangible {
                self.hurt_intangible = false;
            }
        }
    }
    /// it_8027137C / it_8027129C: refresh world capsules at item s-link 11.
    pub fn update_hitboxes(&mut self, assets: &ItemAssets) {
        for id in 0..self.hitboxes.len() {
            self.update_hitbox(id, assets);
        }
    }
    /// The model root's transform as the kind code set it.
    pub fn root_srt(&self) -> crate::pose::RootSrt {
        crate::pose::RootSrt {
            translate: self.position,
            rotate: self.rotation,
            scale: self.model_scale,
        }
    }
    /// it_8027129C (8027129C): one capsule's position from its bone. The
    /// root is the model's JObj, which follows `position` at a state change
    /// (Item_80268E5C) and after the collision proc (Item_80269978).
    fn update_hitbox(&mut self, id: usize, assets: &ItemAssets) {
        let root = crate::pose::RootSrt {
            translate: self.root_translation,
            ..self.root_srt()
        };
        let (state, steps) = (self.article_state, self.pose_steps);
        let Some(hit) = &mut self.hitboxes[id] else {
            return;
        };
        let matrix = if hit.descriptor.bone == 0 {
            let mut matrix = hsd_types::Mtx::default();
            hsd_anim::mtx::hsd_mtx_srt(
                &mut matrix,
                &self.model_scale,
                &self.rotation,
                &self.root_translation,
                None,
            );
            matrix
        } else {
            let pose = assets
                .pose
                .as_ref()
                .expect("item non-root hitbox bone without a sampled pose");
            pose.bone_matrix_turned(
                state,
                steps,
                hit.descriptor.bone,
                root,
                self.child_rotation_x.map(|x| (CHILD_JOINT, x)),
            )
        };
        let mut position = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(&matrix, &hit.descriptor.offset, &mut position);
        hit.update_position(position);
        self.hitbox_trace[id] = (hit.previous_position, hit.position);
    }
    /// Item_8026A8EC (item.c:1991-1995): efLib_DestroyAll, then ItemSwitch's
    /// destroy effect unless suppressed or the item is still in its owner's
    /// hand.
    fn queue_destroy_effect(&mut self, effect: Option<u16>, root: Option<Vec3>) {
        // item.c:1991: efLib_DestroyAll first.
        self.events.push(ItemEvent::DestroyEffects);
        self.effects_destroyed = true;
        if self.destroy_effect_suppressed || (self.held && self.owner.is_some()) {
            return;
        }
        if let Some(id) = effect {
            self.events.push(ItemEvent::DestroyEffect { id, root });
        }
    }
    /// it_80273670(item, 0, frame) (80273670): the model posed at article
    /// state 0's `frame`; its joint animation and script are removed, so no
    /// further script command runs until the next state change.
    pub fn pose_article_frame(&mut self, frame: f32) {
        self.animation_frame = frame;
        self.script = ScriptState::default();
    }
    /// it_80274484 (80274484): the model at `scale`; every live hitbox
    /// takes x3C, the command size, as its radius (it_80275534), the grab
    /// offset and range grow by the scale (it_80274DFC), and so does the
    /// push box (it_80274E44).
    pub fn rescale(&mut self, scale: f32) {
        self.scale = scale;
        self.model_scale = Vec3::new(scale, scale, scale);
        let size = self.hitbox_size;
        for hit in self.hitboxes.iter_mut().flatten() {
            hit.descriptor.radius = size;
        }
        self.grab_offset.x *= scale;
        self.grab_offset.y *= scale;
        self.grab_range.x *= scale;
        self.grab_range.y *= scale;
        // it_80274E44: the push box (xBEC); xBDC has no port consumer.
        self.push_box.top *= scale;
        self.push_box.bottom *= scale;
        self.push_box.right *= scale;
        self.push_box.left *= scale;
    }
    /// it_80272460 (80272460) for an existing hitbox `id`: its count and
    /// the damage restaled for the item's attack (ft_80089228).
    pub fn set_hitbox_damage(&mut self, id: usize, damage: u32) {
        if let Some(hit) = &mut self.hitboxes[id] {
            hit.knockback_damage = damage;
            hit.descriptor.damage = damage as f32 * self.stale_multiplier;
        }
    }
    /// Item_802694CC (802694CC): the joint animation advances by the item's
    /// rate (HSD_JObjAnimAll, x5CC), then its script runs (it_802799E4).
    pub fn advance_animation(&mut self, assets: &ItemAssets) {
        // lbGetJObjCurrFrame: an article without a model (Item_80267978's
        // bare JObj) has no AObj, so x5CC stays zero.
        if assets.model != 0 {
            self.animation_frame += self.animation_rate;
        }
        // Item_802694CC's HSD_JObjAnimAll, on the JObj the collision proc
        // last placed.
        self.pose_steps += 1;
        self.emit_particle_keys(assets, self.root_translation);
        self.advance_script(assets);
    }
    /// The DPtcl keys the AnimAll step just taken fires (efLib_Cb_DPtcl);
    /// `pose_steps` counts it, so its index is one less.
    fn emit_particle_keys(&mut self, assets: &ItemAssets, position: Vec3) {
        let Some(tracks) = &assets.particle_tracks else {
            return;
        };
        let step = u16::try_from(self.pose_steps - 1).expect("particle track step");
        for key in tracks.keys(self.article_state, step) {
            self.events.push(ItemEvent::JointParticle {
                bank: key.bank as u8,
                id: key.id as u32,
                position,
                rotation: self.rotation,
                scale: self.model_scale,
            });
        }
    }
    /// Item_802799E4: shared command timing, item-owned application.
    fn advance_script(&mut self, assets: &ItemAssets) {
        let Some(script) = assets.scripts.get(self.motion as usize) else {
            return;
        };
        self.script.begin_frame(self.animation_frame, 1.0);
        while let Some(command) = self.script.next(script, 1.0) {
            match command {
                Command::SpawnHitbox { id, descriptor } => {
                    let starts = self.hitboxes[*id]
                        .as_ref()
                        .is_none_or(|h| h.descriptor.group != descriptor.group);
                    // it_802790C0 -> it_8026FCF8: a group member's new hitbox
                    // takes the group's history once the pool can see it.
                    if starts && self.hit_group != 0 {
                        self.group_history_pending |= 1 << *id;
                    }
                    if starts {
                        self.reflection_history[*id] = self
                            .hitboxes
                            .iter()
                            .position(|h| {
                                h.as_ref()
                                    .is_some_and(|h| h.descriptor.group == descriptor.group)
                            })
                            .map(|i| self.reflection_history[i].clone())
                            .unwrap_or_default();
                    }
                    // it_802790C0 -> it_80272460: the command damage scaled by
                    // xC40 (xC3C is 1) and truncated to the hit's count.
                    let mut descriptor = descriptor.clone();
                    if self.hitbox_damage_scale != 1.0 {
                        descriptor.damage =
                            gekko_math::msl::fctiwz(descriptor.damage * self.hitbox_damage_scale)
                                as f32;
                    }
                    melee_coll::hitbox::spawn(&mut self.hitboxes, *id, &descriptor);
                    let hit = self.hitboxes[*id].as_mut().unwrap();
                    hit.descriptor.damage *= self.stale_multiplier;
                    // it_802790C0: x3C keeps the command's size, then
                    // it_80275594(1 / scl): the capsule radius is stored
                    // unscaled; contacts multiply the item scale back in.
                    self.hitbox_size = hit.descriptor.radius;
                    hit.descriptor.radius *= 1.0 / self.scale;
                    let index = self
                        .script
                        .instruction
                        .expect("emitted command has continuation")
                        - 1;
                    self.hit_flags[*id] =
                        assets.hit_flags[self.motion as usize][index].expect("item hit flags");
                    // it_802790C0: created after link 11's refresh, the
                    // capsule is placed at once.
                    if self.past_hitbox_refresh {
                        self.update_hitbox(*id, assets);
                    }
                }
                Command::SetHitboxDamage { id, damage } => {
                    if let Some(hit) = &mut self.hitboxes[*id] {
                        hit.knockback_damage = gekko_math::msl::fctiwz(*damage) as u32;
                        hit.descriptor.damage = hit.knockback_damage as f32 * self.stale_multiplier;
                    }
                }
                Command::SetHitboxRadius { id, radius } => {
                    // it_802795EC: x3C keeps the size, then it_80275594(1 /
                    // scl) on a live capsule.
                    self.hitbox_size = *radius;
                    if let Some(hit) = &mut self.hitboxes[*id] {
                        hit.descriptor.radius = *radius * (1.0 / self.scale);
                    }
                }
                Command::ClearHitbox(id) => {
                    self.hitboxes[*id] = None;
                    self.reflection_history[*id].clear();
                    // it_80272560 updates every surviving capsule immediately.
                    self.update_hitboxes(assets);
                }
                Command::ClearHitboxes => {
                    self.hitboxes.fill(None);
                    for history in &mut self.reflection_history {
                        history.clear();
                    }
                }
                Command::SetVariable { index, value } => self.command_variables[*index] = *value,
                Command::Graphics(graphics) => {
                    self.events.push(ItemEvent::ScriptEffect(graphics.clone()))
                }
                // it_8027990C: controller rumble only.
                Command::Rumble { .. } => {}
                // it_8027978C: sub-operations 0..2 play the sound
                // (Item_8026AE84 / AF0C / AFA0); 10 and 11 stop the item's
                // sounds (Item_8026B034 / B074), which nothing simulated hears.
                Command::ItemSound { sub, id } => {
                    if *sub <= 2 {
                        self.sound_requests.push(*id);
                    }
                }
                _ => unimplemented!("item command {command:?}"),
            }
        }
    }
}
#[derive(Debug)]
pub struct ItemPool {
    // Reserve the complete pool on the heap at initialization. Inline storage
    // would copy hundreds of KiB through constructors on small test stacks.
    // spawn checks ITEM_CAPACITY before push, so this buffer never grows.
    pub(crate) items: Vec<ItemCore>,
    common: ItemCommonData,
    next_id: u32,
    /// it_804D6D14: Item_8026AE60's hit group counter.
    pub(crate) next_hit_group: u32,
    /// The id Item_8026AE60 last gave each fighter that asked for one
    /// (ItemRequest::NewHitGroup), keyed by owner slot and fighter; the
    /// fighter keeps it in its motion scratch (Bowser's mv.kp.specials.x4).
    pub(crate) owner_hit_groups: [u32; crate::hit_group::OWNER_GROUPS],
    /// Item_804A0CCC: the fighters it_802722B0 last sampled.
    pub(crate) fighter_push: crate::push::FighterPushSnapshot,
}
impl ItemPool {
    pub fn new(common: ItemCommonData) -> Self {
        Self {
            items: Vec::with_capacity(ITEM_CAPACITY),
            common,
            next_id: 0,
            next_hit_group: 1,
            owner_hit_groups: [0; crate::hit_group::OWNER_GROUPS],
            fighter_push: Default::default(),
        }
    }
    /// it_804D6D28: ItCo's common item data.
    pub fn common(&self) -> &ItemCommonData {
        &self.common
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &ItemCore> {
        self.items.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut ItemCore> {
        self.items.iter_mut()
    }
    /// The common data beside mutable items, for passes that read both.
    pub fn common_and_iter_mut(
        &mut self,
    ) -> (
        &ItemCommonData,
        impl DoubleEndedIterator<Item = &mut ItemCore>,
    ) {
        (&self.common, self.items.iter_mut())
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// The common data beside the item at `index` in list order.
    pub fn common_and_item_mut(&mut self, index: usize) -> (&ItemCommonData, &mut ItemCore) {
        (&self.common, &mut self.items[index])
    }
    /// The common data beside every item, mutably.
    pub fn common_and_items_mut(
        &mut self,
    ) -> (&ItemCommonData, impl DoubleEndedIterator<Item = &mut ItemCore>) {
        (&self.common, self.items.iter_mut())
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// item.c foobar3 (Camera_80029044): kind 1 an Active subject, any
    /// other nonzero kind an Auto one, aiming at the common extents.
    fn camera_subject(&self, kind: u8) -> Option<melee_cm::Subject> {
        if kind == 0 {
            return None;
        }
        let mut subject = melee_cm::Subject::new(if kind == 1 {
            melee_cm::SubjectState::Active
        } else {
            melee_cm::SubjectState::Auto
        });
        let [left, right, top, bottom] = self.common.camera_extents;
        let target = &mut subject.target_extents;
        target.left = left;
        target.right = right;
        target.top = top;
        target.bottom = bottom;
        Some(subject)
    }
    /// Item_80268B18 -> Item_8026862C. Equal p-link priority appends in spawn order.
    pub fn spawn<D: ItemDispatch>(&mut self, spawn: SpawnItem, assets: &ItemAssets) -> Option<u32> {
        self.spawn_with_stale::<D>(spawn, assets, 1.0)
    }
    pub fn spawn_with_stale<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        stale_multiplier: f32,
    ) -> Option<u32> {
        if let Some(limit) = self.common.hold_limits[usize::from(spawn.hold_kind)] {
            if self
                .items
                .iter()
                .filter(|i| i.hold_kind == spawn.hold_kind)
                .count()
                >= limit
            {
                return None;
            }
        }
        assert!(
            self.len() < ITEM_CAPACITY,
            "explicit item storage budget exhausted"
        );
        let id = self.next_id;
        self.next_id += 1;
        let mut item = ItemCore {
            ground_or_air: spawn.ground_or_air,
            id,
            kind: spawn.kind,
            owner: spawn.owner,
            owner_secondary: false,
            stale_source: spawn.stale_source,
            stale_multiplier,
            pending_reflection: None,
            pending_absorb: None,
            reflection_direction: 0.0,
            reflection_history: Default::default(),
            hit_group: 0,
            group_history_pending: 0,
            hold_kind: spawn.hold_kind,
            position: if spawn.initial_collision {
                spawn.previous_position
            } else {
                spawn.position
            },
            previous_position: spawn.previous_position,
            velocity: spawn.velocity,
            nudge: Vec3::ZERO,
            pushable: assets.pushable,
            pushed_by_fighters: true,
            pushed_by_items: true,
            pushed_by_open_palm: true,
            push_settled: false,
            push_box: assets.push_box,
            exploding: false,
            environmental_velocity: Vec3::ZERO,
            platform_velocity: Vec3::ZERO,
            facing: spawn.facing,
            rotation: Vec3::new(
                0.0,
                if assets.rotate_to_facing {
                    (std::f64::consts::FRAC_PI_2 * f64::from(spawn.facing)) as f32
                } else {
                    0.0
                },
                0.0,
            ),
            model_scale: [assets.scale; 3].into(),
            motion: 0,
            // Item_80268B18's it_80275158: both timers.
            life_timer: self.common.lifetime,
            half_life: self.common.lifetime * self.common.half_life_scale,
            scale: assets.scale,
            frozen: false,
            destroyed: false,
            pending_damage_dealt: 0,
            pending_damage_without_hitlag: 0,
            pending_shield_damage: 0,
            pending_shield_deflection: None,
            pending_shield_hitlag: 0.0,
            pending_clank_damage: 0,
            hitlag_enabled: assets.hitlag,
            hitlag_damage: 0,
            hitlag_frames: 0.0,
            in_hitlag: false,
            sound_requests: Default::default(),
            animation_frame: 0.0,
            animation_rate: 1.0,
            destroy_effect_suppressed: false,
            effects_destroyed: false,
            script: ScriptState::default(),
            command_variables: [0; 4],
            hitboxes: std::array::from_fn(|_| None),
            scratch: ItemScratch::None,
            hit_flags: [desc::ItemHitFlags::default(); 4],
            collision: None,
            rotation_axis: assets.rotation_axis,
            floor_line: -1,
            platform_drop: 0,
            land_count: 0,
            throw_count: 0,
            pending_shield_owner: None,
            shield_anchor: None,
            shield_view: None,
            hidden: false,
            spin_rate: assets.spin_rate,
            spin_speed: 0.0,
            spin_ignores_facing: assets.rotate_to_facing,
            grabbable: false,
            grab_offset: assets.grab_offset,
            grab_range: assets.grab_range,
            hitbox_size: 0.0,
            child_rotation_x: None,
            root_translation: if spawn.initial_collision {
                spawn.previous_position
            } else {
                spawn.position
            },
            held: false,
            holder_part: 0,
            hits_owner: false,
            throw_speed: 1.0,
            hitbox_damage_scale: 1.0,
            past_hitbox_refresh: false,
            hurt_intangible: false,
            spawn_intangible_frames: None,
            keeps_intangible: false,
            speed_damage: false,
            hurt_by_owner: false,
            strikes_kindred_items: false,
            damage_percent: 0,
            pending_damage_taken: 0,
            largest_damage_taken: 0,
            pending_knockback: 0.0,
            knockback_angle: 0,
            hit_direction: 0.0,
            hit_by: None,
            queued_events: Default::default(),
            efasync_immediate: false,
            events: Default::default(),
            blast_zone_checked: true,
            partner: None,
            camera: self.camera_subject(assets.camera_kind),
            link_requests: Default::default(),
            owner_request: None,
            pose_steps: 0,
            article_state: 0,
            hitbox_trace: Default::default(),
        };
        // Item_80267130 -> it_80274658(x6C) before the kind's spawn callback.
        item.update_spin(self.common.spawn_spin_degrees);
        (D::logic(item.kind).spawned)(&mut item, assets);
        item.begin_spawn_intangibility(self.common.spawn_intangible_frames);
        self.items.push(item);
        Some(id)
    }
    /// ftColl_80077C60 records xC34_damageDealt; item callbacks run at s-link14.
    pub fn record_damage_dealt(&mut self, id: u32, damage: f32) {
        if let Some(item) = self.get_mut(id) {
            item.record_damage_dealt(damage);
        }
    }
    /// Item_8026A294 -> OnGiveDamageThink, after all fighter/item detection.
    /// Multiple hits accumulate a maximum; one callback runs in this slot.
    pub fn process_events<D: ItemDispatch>(&mut self, id: u32, assets: &ItemAssets) {
        let rng = core::cell::Cell::new(gekko_math::HsdRng::default());
        self.process_events_with_stale::<D>(id, 1.0, assets, &rng);
    }
    pub fn process_events_with_stale<D: ItemDispatch>(
        &mut self,
        id: u32,
        reflected_stale: f32,
        assets: &ItemAssets,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let cap = self.common.maximum_reflected_damage;
        // retail 80269E18/20: add then multiply, no FMA or double promotion.
        let bounce_limit =
            (std::f32::consts::PI / 180.0) * (90.0 + self.common.shield_bounce_degrees);
        let common = self.common.clone();
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let alive = !item.destroyed;
        item.past_hitbox_refresh = true;
        if item.pending_knockback != 0.0 || item.pending_damage_taken != 0 {
            // OnTakeDamageThink: the percent grows (capped at 999) and the
            // frame's damage becomes the hitlag damage (xCA8), unconditionally.
            item.damage_percent = (item.damage_percent + item.pending_damage_taken).min(999);
            item.hitlag_damage = item.pending_damage_taken;
            let context = ItemEventContext {
                rng: Some(rng),
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).damage_received)(item, &context);
        } else if item.pending_shield_damage != 0 {
            if let Some(deflection) = item.pending_shield_deflection.filter(|d| {
                item.ground_or_air == melee_types::GroundOrAir::Air && d.angle < bounce_limit
            }) {
                let context = ItemEventContext {
                    shield_normal: deflection.normal,
                    ..ItemEventContext::new(assets, &common)
                };
                item.destroyed |= (D::logic(item.kind).shield_bounced)(item, &context);
            } else {
                if item.hitlag_enabled {
                    item.hitlag_damage = item.pending_shield_damage;
                }
                let context = ItemEventContext {
                    rng: Some(rng),
                    ..ItemEventContext::new(assets, &common)
                };
                item.destroyed |= (D::logic(item.kind).hit_shield)(item, &context);
            }
        } else if item.pending_clank_damage != 0 {
            // OnClankThink: the clank damage becomes the hitlag damage.
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_clank_damage;
            }
            let context = ItemEventContext {
                rng: Some(rng),
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).clanked)(item, &context);
        } else if item.pending_damage_dealt != 0 || item.pending_damage_without_hitlag != 0 {
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_damage_dealt;
            }
            let context = ItemEventContext {
                rng: Some(rng),
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).damage_dealt)(item, &context);
        } else if let Some(reflection) = item.pending_reflection {
            item.reflect::<D>(reflection, reflected_stale, cap, assets, &common);
        } else if item.pending_absorb.is_some() {
            // Item_8026A294: the kind's absorbed callback.
            let context = ItemEventContext::new(assets, &common);
            item.destroyed |= (D::logic(item.kind).absorbed)(item, &context);
        }
        // processCallback (item.c:1739): destroy_type 2.
        if alive && item.destroyed {
            item.queue_destroy_effect(assets.event_destroy_effect, None);
        }
        // Item_8026A294: a surviving item enters hitlag from xCA8. (The xCC0
        // path is a counter-style shield's own hitlag; no supported fighter
        // sets Fighter.shield_unk1 while an item can reach it.)
        if !item.destroyed {
            // xDC8 xD is set for every item, so a counter's own hitlag wins.
            if item.pending_shield_hitlag > 0.0 {
                let frames = item.pending_shield_hitlag;
                item.enter_hitlag_frames(frames);
            } else if item.hitlag_damage != 0 {
                let damage = item.hitlag_damage;
                item.enter_hitlag(&common, damage);
            }
        }
        item.past_hitbox_refresh = false;
        // Item_80269CC4 resets per-frame contact accumulators.
        item.pending_damage_taken = 0;
        item.largest_damage_taken = 0;
        item.pending_knockback = 0.0;
        item.hit_direction = 0.0;
        item.pending_reflection = None;
        item.pending_absorb = None;
        item.reflection_direction = 0.0;
        item.pending_damage_dealt = 0;
        item.pending_damage_without_hitlag = 0;
        item.pending_shield_damage = 0;
        item.pending_shield_deflection = None;
        item.pending_shield_owner = None;
        item.pending_shield_hitlag = 0.0;
        item.pending_clank_damage = 0;
        item.hitlag_damage = 0;
    }
    /// Item_802693E4 (802693E4), item link 0: count hitlag down and resume.
    pub fn advance_hitlag(&mut self, id: u32) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if item.hitlag_frames > 0.0 {
            item.hitlag_frames -= 1.0;
            if item.hitlag_frames <= 0.0 {
                item.hitlag_frames = 0.0;
                // Item_8026A1E8: no supported kind has an exited_hitlag callback.
                item.in_hitlag = false;
            }
        }
    }
    pub fn retire(&mut self, id: u32) {
        if let Some(item) = self.get_mut(id) {
            item.destroyed = true;
        }
    }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut ItemCore> {
        self.items.iter_mut().find(|i| i.id == id)
    }
    pub fn control<D: ItemDispatch>(
        &mut self,
        owner: u8,
        kind: ItemKind,
        control: ItemControl,
        assets: &ItemAssets,
    ) {
        self.control_owned::<D>(owner, false, kind, control, assets, false);
    }
    /// [`Self::control`] for the articles of one fighter of the player
    /// (`secondary`: its Nana). `immediate`: the requesting proc runs at
    /// s_link 9 or later, where efAsync_Spawn dispatches at once.
    pub fn control_owned<D: ItemDispatch>(
        &mut self,
        owner: u8,
        secondary: bool,
        kind: ItemKind,
        control: ItemControl,
        assets: &ItemAssets,
        immediate: bool,
    ) {
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.owner == Some(owner) && i.owner_secondary == secondary && i.kind == kind)
        {
            item.efasync_immediate = immediate;
            (D::logic(kind).control)(item, control, assets);
            item.efasync_immediate = false;
        }
    }
    #[allow(clippy::too_many_arguments)] // Owner, holder, map, partner and RNG stay separate.
    pub fn animate<D: ItemDispatch>(
        &mut self,
        id: u32,
        assets: &ItemAssets,
        owner: Option<&ItemOwner>,
        holder: Option<crate::ItemHolder<'_>>,
        map: &mut melee_mp::CollMap,
        partner: Option<crate::PartnerView>,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        // Item_80269528: hitlag pauses the animation and its callback.
        if !(item.frozen || item.in_hitlag) {
            item.advance_animation(assets);
            let row = D::logic(item.kind).states[item.motion as usize];
            let ended = (row.animation)(
                item,
                &mut ItemAnimationContext {
                    owner,
                    holder,
                    map,
                    assets,
                    partner,
                    rng: Some(rng),
                },
            );
            if ended && !item.destroyed {
                // Item_80269528: destroy_type 0.
                item.queue_destroy_effect(assets.destroy_effect, None);
            }
            item.destroyed |= ended;
            if item.destroyed {
                return;
            }
        }
        item.count_down_spawn_intangibility();
        // Item_80269528's common-item lifetime: xDC8 x15 on a kind below
        // It_Kind_L_Gun_Ray (xDD0 b3, set on explosion, clears x15 here).
        // The x34 warning blink (it_802728C8) is visual.
        if item.grabbable && (item.kind as u32) < ItemKind::LGunRay as u32 {
            item.life_timer -= 1.0;
            if item.life_timer <= 0.0 {
                item.destroyed = true;
                return;
            }
        }
        // Item_80269528's tail: it_802721B8 and it_80272298.
        self.push_apart(id, rng);
    }
    #[allow(clippy::too_many_arguments)] // Owner, targets and bounds stay separate.
    pub fn physics<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        targets: &crate::LockOnTargets,
        bounds: &ItemBounds,
        assets: &ItemAssets,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if !item.frozen && !item.in_hitlag {
            (D::logic(item.kind).states[item.motion as usize].physics)(
                item,
                &ItemPhysicsContext {
                    owner,
                    targets,
                    assets,
                    bounds,
                    rng,
                },
            );
        }
        integrate(item, bounds);
    }
    /// it_8026E9A4 -> mpCheckAllRemap, used by ray state collision callbacks.
    pub fn stage_contact(&self, id: u32, map: &mut melee_mp::CollMap) -> bool {
        let Some(item) = self.items.iter().find(|i| i.id == id) else {
            return false;
        };
        let ItemScratch::Ray(ray) = &item.scratch else {
            return false;
        };
        map.check_all_remap(
            -1,
            -1,
            ray.previous_position.x,
            ray.previous_position.y,
            item.position.x,
            item.position.y,
        )
        .is_some()
    }
    #[allow(clippy::too_many_arguments)] // Owner, map and bounds stay separate.
    pub fn collide<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        stage_contact: bool,
        map: &mut melee_mp::CollMap,
        bounds: &ItemBounds,
        assets: &ItemAssets,
        rng: Option<&core::cell::Cell<gekko_math::HsdRng>>,
    ) {
        let half_life_scale = self.common.half_life_scale;
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let alive = !item.destroyed;
        item.destroyed |= (D::logic(item.kind).states[item.motion as usize].collision)(
            item,
            &mut ItemCollisionContext {
                owner,
                stage_contact,
                map,
                assets,
                bounds,
                rng,
                half_life_scale,
            },
        );
        if alive && item.destroyed {
            // Item_80269978: destroy_type 1, ItemSwitch's x68 effect at the
            // root JObj, which this proc has not yet moved to `position`.
            let root = item.root_translation;
            item.queue_destroy_effect(assets.event_destroy_effect, Some(root));
        } else {
            // Item_80269978: HSD_JObjSetTranslate(jobj, &pos).
            item.root_translation = item.position;
        }
    }
    /// Item_8026A8EC (8026A8EC) outside the item procs: remove `id` now and
    /// run its kind's destroyed callback.
    pub fn destroy<D: ItemDispatch>(&mut self, id: u32) {
        let index = self
            .items
            .iter()
            .position(|item| item.id == id)
            .expect("Item_8026A8EC: Not Found Item_Struct");
        let mut item = self.items.remove(index);
        (D::logic(item.kind).destroyed)(&mut item);
    }
    pub fn remove_destroyed<D: ItemDispatch>(&mut self) {
        let mut index = 0;
        while index < self.items.len() {
            if self.items[index].destroyed {
                let mut item = self.items.remove(index);
                (D::logic(item.kind).destroyed)(&mut item);
                if D::logic(item.kind).unlinks_partner_on_destroy {
                    if let Some(partner) = item.partner.and_then(|id| self.get_mut(id)) {
                        partner.partner = None;
                    }
                }
            } else {
                index += 1;
            }
        }
    }
}
/// Item_802697D4: audited retail uses four PSVECAdd calls; never reassociate
/// velocity+nudge, position+delta, environmental movement, platform movement.
fn integrate(item: &mut ItemCore, bounds: &ItemBounds) {
    if !item.held && !item.frozen && !item.in_hitlag {
        let delta = add(item.velocity, item.nudge);
        item.position = add(item.position, delta);
    }
    // Item_802697D4 -> Item_802696CC, before environmental/platform movement.
    // Item_802680CC enables all four bounds; attachment skips this whole block.
    if !item.held
        && item.blast_zone_checked
        && (item.position.x > bounds.right
            || item.position.x < bounds.left
            || item.position.y > 10000.0
            || item.position.y < bounds.bottom)
    {
        item.destroyed = true;
        return;
    }
    // Item_802697D4 -> it_80274A64: an airborne item spins about its axis.
    if !item.held && item.spin_speed != 0.0 && item.ground_or_air == GroundOrAir::Air {
        item.spin();
    }
    // Each movement is spent once added: it_80273484 (retail 0x802698F0)
    // clears the environmental one, it_8027346C (0x80269954) the platform
    // one, which a bounce off a moving line set (it_8027781C).
    item.position = add(item.position, item.environmental_velocity);
    item.environmental_velocity = Vec3::ZERO;
    item.position = add(item.position, item.platform_velocity);
    item.platform_velocity = Vec3::ZERO;
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

impl Clone for ItemPool {
    fn clone(&self) -> Self {
        Self {
            items: hsd_types::storage::clone_vec(&self.items),
            common: self.common.clone(),
            next_id: self.next_id,
            next_hit_group: self.next_hit_group,
            owner_hit_groups: self.owner_hit_groups,
            fighter_push: self.fighter_push,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct InertTestItem;
    impl ItemLogic for InertTestItem {
        const KIND: ItemKind = ItemKind::FoxBlaster;
        const STATES: &'static [ItemStateRow] = &[];
    }
    item_kinds! { enum TestKinds { Inert:InertTestItem } }
    fn assets() -> ItemAssets {
        ItemAssets {
            visual: Default::default(),
            scripts: Vec::new(),
            hit_flags: Vec::new(),
            special_attributes: Vec::new(),
            scale: 1.0,
            model: 0,
            rotate_to_facing: false,
            collision_box: Default::default(),
            push_box: Default::default(),
            pushable: false,
            collision_damage_multiplier: 1.0,
            hitlag: false,
            camera_kind: 0,
            fall_acceleration: 0.0,
            fall_speed_limit: 0.0,
            launch_vertical_velocity: 0.0,
            rotation_axis: 0,
            explosion_lifetime: 0.0,
            spin_rate: 0.0,
            fall_spin_degrees: 0.0,
            animation_ends: Vec::new(),
            hurtboxes: Vec::new(),
            release_box_scale: 0.0,
            throw_break: 0,
            heavy: false,
            use_kind: 0,
            hand_hold_kind: 0,
            throw_speed_multiplier: 1.0,
            bounce_scale: 1.0,
            bounce_sound: 0,
            rest_speed: 0.0,
            slide_speed: 0.0,
            landing_spin_degrees: 0.0,
            destroy_effect: None,
            event_destroy_effect: None,
            grab_offset: hsd_types::Vec2::ZERO,
            grab_range: hsd_types::Vec2::ZERO,
            attachment_translation: Vec3::ZERO,
            bone_motion: None,
            pose: None,
            particle_tracks: None,
            special_pointees: Vec::new(),
        }
    }
    #[test]
    fn admission_limit_releases_capacity_without_reordering_survivors() {
        let _kind = TestKinds::Inert;
        let mut hold_limits = [None; 13];
        hold_limits[8] = Some(2);
        let mut pool = ItemPool::new(ItemCommonData {
            hold_limits,
            lifetime: 1.0,
            spawn_intangible_frames: 0.0,
            half_life_scale: 0.5,
            speed_damage_scale: 0.0,
            speed_damage_base: 0.0,
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            clank_priority_gap: 9,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
            launch: Default::default(),
            victim_bounce: Default::default(),
            camera_extents: [0.0; 4],
            push_speed: 0.0,
        });
        let spawn = SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0);
        let first = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        let survivor = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        assert!(pool.spawn::<TestKinds>(spawn, &assets()).is_none());
        pool.retire(first);
        pool.remove_destroyed::<TestKinds>();
        let last = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        assert_eq!(
            pool.iter().map(|item| item.id).collect::<Vec<_>>(),
            [survivor, last]
        );
    }
    #[test]
    fn physics_keeps_retail_rounding_boundary_between_nudge_and_position() {
        let mut pool = ItemPool::new(ItemCommonData {
            hold_limits: [None; 13],
            lifetime: 1.0,
            spawn_intangible_frames: 0.0,
            half_life_scale: 0.5,
            speed_damage_scale: 0.0,
            speed_damage_base: 0.0,
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            clank_priority_gap: 9,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
            launch: Default::default(),
            victim_bounce: Default::default(),
            camera_extents: [0.0; 4],
            push_speed: 0.0,
        });
        let spawn = SpawnItem::held(
            ItemKind::FoxBlaster,
            0,
            Vec3::new(16777216.0, 0.0, 0.0),
            1.0,
        );
        let id = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        let item = pool.get_mut(id).unwrap();
        item.velocity.x = -16777216.0;
        item.nudge.x = -1.0;
        integrate(
            item,
            &ItemBounds {
                left: f32::NEG_INFINITY,
                right: f32::INFINITY,
                bottom: f32::NEG_INFINITY,
                top: f32::INFINITY,
                camera_offset: hsd_types::Vec2::ZERO,
            },
        );
        assert_eq!(item.position.x.to_bits(), 0.0f32.to_bits());
    }
}
