use crate::{
    desc::{ItemAssets, ItemCommonData},
    *,
};
use hsd_types::Vec3;
use melee_cmd::{Command, ScriptState};
use melee_coll::hitbox::HitCapsule;
use melee_types::{GroundOrAir, ItemKind};

/// Item.toucher: who an inert hitbox met.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toucher {
    Fighter(u8),
    Item(u32),
}

/// xCB8 (ftcoll.c:1252-1268, itcoll.c:497-508): moving, the way the item
/// moves; all but still (under ItCo +D4), away from the victim at
/// `victim_x`.
pub fn out_damage_direction(velocity_x: f32, position_x: f32, still_speed: f32, victim_x: f32) -> f32 {
    let speed = if velocity_x < 0.0 { -velocity_x } else { velocity_x };
    if speed < still_speed {
        if position_x > victim_x {
            -1.0
        } else {
            1.0
        }
    } else if velocity_x < 0.0 {
        -1.0
    } else {
        1.0
    }
}

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
/// `xDD4_itemVar.arwinglaser` (itarwinglaser.c): a stage ship's laser.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArwingLaserState {
    /// xDFC: the map scale (Ground_801C0498); xE34: the shooter's scale.
    pub map_scale: f32,
    pub scale: f32,
    /// xE0C: the unit direction toward the target (it_802E7654).
    pub direction: Vec3,
    /// xE18: the tip (dynamic bone 4) as the collision proc last found it;
    /// xE24: the tip a tick before.
    pub tip: Vec3,
    pub previous_tip: Vec3,
    /// xE30: the shot has left the gun.
    pub launched: bool,
    /// xE38: the laser's type (1 an Arwing's, 3 a Wolfen's).
    pub kind: i32,
    /// xE3C: reflected or bounced off a shield.
    pub turned: bool,
    /// xE48: the collision joint the line tests keep to (it_8026EA9C).
    pub joint: i32,
    /// xE4C: the ticks the laser has spent near the stage's plane.
    pub near_ticks: i32,
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
    /// The effect model of the shot's flash (it_802ADF10's efSync_Spawn).
    pub muzzle_model: u32,
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
    /// the root's translation (or a held item's display-pass root) when the
    /// event is handled.
    DestroyEffect { id: u16, root: Option<Vec3> },
    /// efSync_Spawn(0x3E8, gobj, &pos, &damage): the spark of a hit landing
    /// on the item (it_80270E30).
    HitSpark { position: Vec3, damage: f32 },
    /// efSync_Spawn(0x3EC, item, &pos, item): a slashing hit's spark on a
    /// stage enemy or Pokemon (it_80270E30).
    SlashSpark { position: Vec3 },
    /// efSync_Spawn(0x3ED, item, &pos, &item->facing_dir): an Ice hit's
    /// spark on a stage enemy or Pokemon (it_80270E30), turned by the
    /// facing the item had when the hit resolved.
    IceSpark { position: Vec3, facing: f32 },
    /// efSync_Spawn(0x479, item, &pos, item): a Coin hit's positional
    /// generator on a stage enemy or Pokemon (it_80270E30); the scene
    /// picks efAlt's 0x479 or efsync.c:67-69's 0x506 by Mario's bank.
    CoinSpark { position: Vec3 },
    /// Script opcode 10 (it_80278F2C): an effect at a joint whose offset gets
    /// a random spread (it_80278800) when the scene resolves it.
    ScriptEffect(melee_types::combat::GraphicsCommand),
    /// [`Self::ScriptEffect`] from a script run past the item's link 9
    /// flush (a motion change in a hit's callback, Item_80268E5C ->
    /// it_802799E4): efAsync_Spawn dispatches it at once.
    ScriptEffectNow(melee_types::combat::GraphicsCommand),
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
    /// efSync_Spawn(id, gobj, jobj, &facing, &scale) through
    /// efLib_CreateGenerator_AppSRT_SetFacingDirScale: a generator on the
    /// item's root whose AppSRT turns to the facing (+-M_PI_2) and takes a
    /// uniform scale (a Fire Flower flame, 0x44E..0x451).
    FacingGenerator { id: u16, facing: f32, scale: f32 },
    /// efAsync kind 0 on a joint of the model for an efAlt row that makes
    /// a model with the item as its parent (efLib_Create_AttachChild: the
    /// hammer of Kirby's side special, 0x498 / 0x499). `bone` is the
    /// joint's depth-first index.
    BoneModel { id: u16, bone: usize },
    /// efLib_Cb_DPtcl from the item's joint animation: generator `id` of
    /// `bank` attached to the joint whose track fired (`joint`, its
    /// depth-first index; 0 is the model root), under the root transform
    /// at the AnimAll step, which is given.
    JointParticle {
        bank: u8,
        id: u32,
        joint: usize,
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
    /// it_8026F3D4 (itdrop.c): a container lets out up to `count` items
    /// drawn from the random item spawner's contents table (it_804A0E50),
    /// each at `position` (it_8026BB88: the container's ECB centre) with
    /// `previous_position` its position on the stage plane, and a random
    /// sideways launch (it_8026F53C).
    Release {
        count: u8,
        /// it_8026F3D4's arg3 bit 0: every item is the first's kind.
        same_kind: bool,
        position: Vec3,
        previous_position: Vec3,
    },
    /// A crate's or barrel's contents (it_80286248, it_802886C4). A coin
    /// match's coins aside, a monster may come out instead (it_8026F6BC:
    /// ItCo +124 odds, under its hold-kind 6 limit); otherwise the Box's
    /// roll picks how many ([`CrateRoll`]), a barrel lets out one, as a
    /// [`Self::Release`].
    CrateContents {
        roll: Option<CrateRoll>,
        position: Vec3,
        previous_position: Vec3,
    },
    /// it_80289BE8 (itkusudama.c): an opened Party Ball's shower. A coin
    /// match's coins aside, a monster may come out (it_8026F6BC at
    /// `position` lowered by 10, after a small random drift); otherwise
    /// HSD_Randi over the weights picks `count` of `kind`, ten to fourteen
    /// Foods, or three or four draws from the contents table, each
    /// created at `center` (the ECB centre) facing from `position`
    /// lowered by 5, with its own random drift.
    PartyBallContents {
        /// x4, x0, x8: the weights of the attribute kind, of Foods and of
        /// random items.
        weights: [i32; 3],
        /// x10 and x14: the attribute kind and how many of it.
        kind: i32,
        count: i32,
        center: Vec3,
        position: Vec3,
    },
    /// it_8027B5B0 from the item's callback: a stage enemy of `kind` made
    /// at `position` (Birdo's egg, it_802EFA44), which the scene finishes
    /// for its kind with `facing`.
    SpawnZako {
        kind: melee_types::ItemKind,
        position: Vec3,
        facing: f32,
    },
    /// efSync_Spawn(0x4D0, gobj, &pos) (efsync.c:229-292): Birdo's egg's
    /// shell burst at `position`, whose twelve fragments' creation draws the
    /// callback took from the stream at `seed`, where retail makes them.
    EggShell { position: Vec3, seed: u32 },
    /// A call from the item's callback into the stage that made it (Birdo's
    /// knockout drops her platform: grInishie2_801FD4CC).
    StageNotice(u8),
    /// it_80276100(gobj, &pos) from a callback without the map (Birdo's
    /// knockout): the item's map collision starts over at its position.
    ResetCollision,
    /// The rest of the item's callback, which the scene runs through
    /// `ItemLogic::resumed` once the effects before it are made.
    Resume {
        step: u8,
        /// it_802790C0 (8027951C): the originating proc's s_link is past 11.
        past_hitbox_refresh: bool,
        /// Preserve an animation callback's immediate efAsync dispatch too.
        efasync_immediate: bool,
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

// PK Thunder's head keeps its position history inline: an item's scratch
// never allocates on the tick path, so the large variant is not boxed.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum ItemScratch {
    Needle(NeedleState),
    Afterimage(AfterimageState),
    Ray(RayState),
    ArwingLaser(ArwingLaserState),
    Held(HeldState),
    Bomb(BombState),
    Dosei(DoseiState),
    Heiho(HeihoState),
    /// Item.xDD4_itemVar.tools.x0 (ittools.c): which of the five tools.
    Tools(i32),
    Turnip(TurnipState),
    Jolt(JoltState),
    Thunder(ThunderState),
    ClimbersIce(ClimbersIceState),
    Missile(MissileState),
    ChargeShot(ChargeShotState),
    SamusBomb(SamusBombState),
    DinFire(DinFireState),
    PkFlash(PkFlashState),
    PkThunder(PkThunderState),
    PkThunderTrail(PkThunderTrailState),
    Boomerang(BoomerangState),
    Bow(BowState),
    Arrow(ArrowState),
    LinkBomb(LinkBombState),
    Milk(MilkState),
    KoopaFlame(KoopaFlameState),
    Chef(ChefState),
    Rescue(RescueState),
    Judge(JudgeState),
    GameWatchFire(GameWatchFireState),
    MewtwoDisable(DisableState),
    ShadowBall(ShadowBallState),
    CutterBeam(CutterBeamState),
    KirbyHammer(KirbyHammerState),
    Container(ContainerState),
    Barrel(BarrelState),
    Shell(ShellState),
    RedShell(RedShellState),
    Freeze(FreezeState),
    WarpStar(WarpStarState),
    /// itMSBomb_ItemVars xDD4 b0: it has armed before.
    MotionSensor { armed_before: bool },
    /// A healing item's xDD4 scratch: what eating it heals (it_8026B47C)
    /// and, for Food, which dish it is (xDD4_itemVar.foods.x0).
    Heal { amount: i32, dish: u32 },
    /// The Fire Flower's xDD4 (itfflower.c): x0 flames since its hit group
    /// changed, x4 its mouth flame (efSync 0x473) made.
    FireFlower { flames: i32, mouth_lit: bool },
    /// itStar_ItemVars: the direction it bounces along (dir) and the
    /// invincibility it gives (it_80272818, its attributes' x0).
    Starman { direction: f32, frames: i32 },
    /// A Fire Flower flame's xDD4 (itlgunbeam.c).
    Flame(FlameState),
    /// Whispy's apple (itWhispyApple_ItemVars): xDD4 its heal, xDD8 set
    /// when it vanishes where it lands.
    WhispyApple { heal: i32, vanishes: bool },
    Monster(MonsterState),
    /// Item.xDD4_itemVar.it_2E5A: a Coin match's coin.
    Coin(crate::coin::CoinState),
    /// itKyasarinEgg_ItemVars x20: the egg is bursting (it_802EFD84).
    KyasarinEgg { bursting: bool },
    Yaku(YakuAnchor),
    /// One float of a projectile's xDD4 scratch: the Star Rod star's spin
    /// per frame (HSD_JObjAddRotationZ), the spore's slowdown (xDD8).
    Float(f32),
    None,
}
/// it_8027B730: the scratch every stage enemy shares (itZako_ItemVars).
#[derive(Clone, Debug)]
pub struct MonsterState {
    pub joint: Option<usize>,
    pub joint_index: i32,
    /// x8: the floor normal the model last turned toward (it_8027C0F0).
    pub floor_normal: Vec3,
    /// x14.x: frames left to finish that turn.
    pub turn_frames: i32,
    /// x14.y: wall and floor bounces of a launched enemy (it_8027C79C).
    pub bounces: i32,
    /// x14.z: the last hurt voice (it_8027C9D8), never repeated.
    pub last_voice: i32,
    pub behavior: MonsterBehavior,
}
#[derive(Clone, Debug)]
pub enum MonsterBehavior {
    Klaptrap {
        parent: Option<crate::StageParent>,
    },
    /// itOldkuri: xDF4 the walk speed, xDF8 the stand's countdown, xDFC
    /// whether the walk has run a whole cycle (and may turn).
    Kuriboh {
        walking_speed: f32,
        spawn_timer: i32,
        cycled: bool,
    },
    ReDead(ReDeadState),
    Kyasarin(KyasarinState),
    Tincle(TincleState),
}
/// Tingle on his balloon (itTincle_ItemVars x20..x68, it/itCommonItems.h).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TincleState {
    /// x20: the state's frame count (a wait, a sink, the landed bounces).
    pub timer: i32,
    /// x24 / x28: a drift's length and the frames it has run.
    pub drift_frames: i32,
    pub drift_count: i32,
    /// x2C: frames left of the bob's half cycle (or the rise's height).
    pub half_cycle: i32,
    /// x30: the motion state a sink interrupted (as a float).
    pub saved_motion: f32,
    /// x34: the drift's acceleration.
    pub drift_acceleration: f32,
    /// x38: the bob's (or a rise's) vertical acceleration.
    pub acceleration: f32,
    /// x3C: half cycles left before the bob turns.
    pub bob_phase: f32,
    /// x40 / x44 / x48: the bob a sink interrupted: x2C, vel.y and x38.
    pub saved_half_cycle: f32,
    pub saved_velocity_y: f32,
    pub saved_acceleration: f32,
    /// x4C: the height a sink started from.
    pub sink_start: f32,
    /// x50: the balloon's top above him (bone 11).
    pub balloon_height: f32,
    /// x54: the weight of those standing on him (grGreatBay_801F5600).
    pub weight: f32,
    /// x58: the middle of his range.
    pub center_x: f32,
    /// x5C: the height he floats at.
    pub float_y: f32,
    /// x68: his authored collision box (xC1C at creation).
    pub collision_box: melee_types::mp::ItEcb,
}
/// Birdo (itKyasarin_ItemVars x20..x3C, it/itCommonItems.h), past the
/// shared zako scratch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KyasarinState {
    /// x20 != NULL: the stage platform (map 15) she rides on.
    pub platform: bool,
    /// x24: frames left of a wait (states 1 and 3).
    pub wait: f32,
    /// x28: frames before the next egg (state 6).
    pub egg_timer: f32,
    /// x2C: frames before she turns to leave (state 6).
    pub turn_timer: f32,
    /// x30: the facing she came in with.
    pub entry_facing: f32,
    /// x34: eggs left in the volley.
    pub eggs: i32,
    /// x38: the damage taken so far.
    pub damage: i32,
    /// x3C: the motion state a light hit interrupted.
    pub resume_motion: u16,
}
/// A fighter a ReDead holds (itLeadead x38, xE14): its player slot and
/// whether it is the player's second fighter, and the part it clings to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReDeadCaptive {
    pub player: u8,
    pub secondary: bool,
    pub part: u8,
}
/// What a ReDead reads of the fighter it holds when it lets go
/// (it_802EAC8C, it_802759DC); the scene refreshes it before each of the
/// item's procs that may let go.
#[derive(Clone, Copy, Debug)]
pub struct CaptiveView {
    /// The world matrix of the part it clings to (ftLib_80086630).
    pub part: hsd_types::Mtx,
    /// The fighter's floor line (it_8026C100's CollData).
    pub floor_line: i32,
    /// The fighter ECB's middle above its position: 0.5 * (top + bottom).
    pub ecb_middle: f32,
    /// fp->cur_pos (it_8026BC90).
    pub position: Vec3,
}
/// itLeadead_ItemVars (itCharItems.h:204).
#[derive(Clone, Debug)]
pub struct ReDeadState {
    /// x38 / xE14: the fighter it holds.
    pub captive: Option<ReDeadCaptive>,
    /// The held fighter as the scene last saw it.
    pub captive_view: Option<CaptiveView>,
    /// x30 (init_facing_dir): its facing before the last hit turned it.
    pub facing_before_hit: f32,
    /// x3C: how far in front of the fighter it clings (joint 1's Z).
    pub cling_depth: f32,
    /// x44: frames in the current state.
    pub frames: i32,
    /// x48: the state's countdown (or count up, in the lunge).
    pub timer: i32,
    /// x4C: frames before it may lunge.
    pub cooldown: i32,
    /// x50: hits that only staggered it, since its last fall.
    pub staggers: i32,
    /// xE30..xE4C: hurt capsule 0 at birth (it_80271590).
    pub hurt_offsets: [Vec3; 2],
    pub hurt_radius: f32,
    /// xE50: the ECB box at birth.
    pub collision_box: melee_types::mp::ItEcb,
}
/// itTaru_ItemVars (ittaru.c): a barrel.
#[derive(Clone, Copy, Debug, Default)]
pub struct BarrelState {
    /// xDD4: broken.
    pub opened: bool,
    /// xDD8: ticks left once broken open.
    pub despawn_timer: i32,
    /// xDDC: the staves' tilt about X, drawn at the spawn.
    pub tilt: f32,
    /// xDE0: the roll's angle (the model root's rotation Z).
    pub roll_angle: f32,
    /// xDE4: the roll's signed speed.
    pub roll_speed: f32,
    /// xDE8: the normal of the floor it rolls on.
    pub floor_normal: Vec3,
}
/// itGShell_ItemVars / itRShell_ItemVars: a Green or Red Shell.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShellState {
    /// xDD4: frames left while it slides empty-handed (it ends at zero).
    pub lifetime: f32,
    /// xDD8: frames before its hits reach its owner again (it_80275444).
    pub owner_grace: f32,
    /// xDDC: frames to the next roll spark (efAsync 0x405).
    pub spark_timer: f32,
    /// xDE0: frames its first hitbox stays unable to clank.
    pub clank_timer: f32,
    /// xDE4: frames before a bounced shell may be stomped again.
    pub stomp_timer: f32,
    /// xDE8: the bounces a hit may still send it on before it lies still.
    pub bounces: i32,
    /// xDEC b0: it has been bounced (by a hit or a stomp) before.
    pub bounced_before: bool,
    /// xDEC b1: the clank timer is running.
    pub clank_pending: bool,
    /// xDEC b2: when the clank timer ends its hurtboxes take hits again.
    pub restore_hurtbox: bool,
    /// xDEC b3: the stomp timer is running.
    pub stomp_pending: bool,
}
/// itRShell_ItemVars: a Red Shell.
#[derive(Clone, Copy, Debug, Default)]
pub struct RedShellState {
    /// xDD4: frames left while it drives (it ends at zero).
    pub lifetime: f32,
    /// xDD8: frames before its hits reach its owner again.
    pub owner_grace: f32,
    /// xDDC: frames to the next spark.
    pub spark_timer: f32,
    /// xDE0: the homing's push toward its target, per frame.
    pub acceleration: f32,
    /// xDE4: frames to the next choice of target.
    pub retarget_timer: f32,
    /// xDE8: bounces left (a fall with none ends it on landing).
    pub bounces: i32,
    /// xDEC: hits it may still take.
    pub hits_left: i32,
    /// xDF0: out of hits, it breaks.
    pub broken: bool,
    /// xDF4 b0: it has rebounded before.
    pub bounced_before: bool,
}
/// itWstar_ItemVars (xDD4..xDDC): the ride's model and hitbox scales and
/// the flight path it drew.
#[derive(Clone, Copy, Debug)]
pub struct WarpStarState {
    /// xDD4: the model scale the rider set (it_80294430), 1 until then.
    pub scale: f32,
    /// xDD8: the hitbox scale (the rider's warp_star_hitbox_scale).
    pub hitbox_scale: f32,
    /// xDDC: the flight path it_80294364 drew.
    pub path: i32,
    /// it_80294430 scaled the attach joint to `scale` (it_80272F7C).
    pub joint_scaled: bool,
}
/// itFreeze_ItemVars: a Freezie's slide.
#[derive(Clone, Copy, Debug, Default)]
pub struct FreezeState {
    /// x0: the slide's signed speed along the floor.
    pub speed: f32,
    /// x4: the normal of the floor it slides on.
    pub floor_normal: Vec3,
}
/// it_80286248's roll: one in `same_kind_odds` lets out one kind only,
/// and the weights pick one, two or three items.
#[derive(Clone, Copy, Debug)]
pub struct CrateRoll {
    pub weights: [i32; 3],
    pub same_kind_odds: i32,
}
/// A random item container's flags (itCapsule_ItemVars, itBox_ItemVars,
/// ...).
#[derive(Clone, Copy, Debug, Default)]
pub struct ContainerState {
    /// x0: it let out its contents, so it goes.
    pub opened: bool,
    /// x4: it exploded instead.
    pub exploded: bool,
    /// The Box's despawn_timer: ticks left once opened.
    pub despawn_timer: i32,
}
/// Item.xDD4_itemVar.yaku (ityaku.c) with x2 == 1: a stage material item
/// that follows a stage JObj. The stage writes the JObj's world translation
/// and local scale and rotation here after its proc moves the JObj; the
/// item's physics copies them (it_2E6A_UnkMotion19_Phys).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct YakuAnchor {
    /// lb_8000B1CC(x4): the JObj's world translation.
    pub position: Vec3,
    /// HSD_JObjGetScale(x4).
    pub scale: Vec3,
    /// HSD_JObjGetRotation(x4), Euler.
    pub rotation: Vec3,
}
/// Item.xDD4_itemVar.kirbyhammer (itkirbyhammer.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct KirbyHammerState {
    /// x0: it_802ADC54's `vars`, the owner's ground_or_air at the swing.
    pub airborne: bool,
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
/// itLGunBeam_ItemVars (itlgunbeam.c): one Fire Flower flame.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlameState {
    /// x0: where this step began (the physics callback's position0).
    pub origin: hsd_types::Vec3,
    /// xC: the touched surfaces' normal (it_802993E0).
    pub normal: hsd_types::Vec3,
    /// x18: the velocity's unit vector.
    pub heading: hsd_types::Vec3,
    /// x24: the flight's angle from straight up, clockwise.
    pub angle: f32,
    /// x28: the flight's speed.
    pub speed: f32,
    /// x2C: the lifetime it was given.
    pub lifetime: f32,
}
/// Mr. Game & Watch's Judgment sign (itgamewatchjudge.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct JudgeState {
    /// it_802C7774's arg4: the face drawn, 0..8 (the sign shows it plus 1).
    pub face: i32,
}
/// Mr. Game & Watch's Fire torch (itgamewatchfire.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct GameWatchFireState {
    /// fp->u.gw.x2254_fireGObj points at a newer torch: a second forward
    /// smash's ftGw_ItemTorchSetup (8014A848) overwrites the pointer
    /// without removing this one, so its owner's removal and hitlag calls
    /// no longer reach it. It still reads its owner's motion and goes by
    /// itself (itGamewatchFire_Motion0_Anim).
    pub superseded: bool,
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
/// Item.xDD4_itemVar.mewtwoshadowball (itmewtwoshadowball.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct ShadowBallState {
    /// x4.x / x4.y: the flight's angle (radians) and speed.
    pub angle: f32,
    pub speed: f32,
    /// x10: the model grandchild's scale in flight.
    pub scale: f32,
    /// x14: it left the hand (or was thrown).
    pub launched: bool,
    /// x18 / x1C: the charge and the full charge.
    pub level: i32,
    pub full: i32,
    /// x20: a counter 0..2 fn_802C5E18 runs in flight (nothing reads it).
    pub cycle: i32,
    /// x24: the damage the charge gives (nothing ported reads it).
    pub damage: u32,
    /// x2C: the fighter that formed it.
    pub creator: Option<u8>,
    /// x30: the model grandchild's translation, the waver.
    pub waver: Vec3,
    /// x3C / x40 / x44: the waver's direction (radians), speed and the
    /// speed's change per frame.
    pub waver_angle: f32,
    pub waver_speed: f32,
    pub waver_acceleration: f32,
    /// x48: frames since the waver last turned.
    pub waver_frames: i32,
    /// x4C: physics frames in flight.
    pub flight_frames: i32,
    /// x50: the charge's share of the speed and the waver, 0.5..1.
    pub strength: f32,
    /// x54: the creator's model scale (ftLib_800869D4).
    pub owner_scale: f32,
    /// x64: the burst's hitbox radius.
    pub burst_radius: f32,
}
/// Item.xDD4_itemVar.kirbycutterbeam (itkirbycutterbeam.c): Final Cutter's
/// beam.
#[derive(Clone, Copy, Debug, Default)]
pub struct CutterBeamState {
    /// x0: the slope it runs along, the floor's angle signed by the facing.
    pub angle: f32,
    /// x4: its speed, which the attribute's deceleration wears down.
    pub speed: f32,
    /// x8: the floor's tangent at the launch.
    pub direction: hsd_types::Vec3,
    /// x14: the position before this frame's move.
    pub previous_position: hsd_types::Vec3,
}
/// Item.xDD4_itemVar.mdisable (itmewtwodisable.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct DisableState {
    /// owner: the fighter that created it, while it still keeps it.
    pub owner: Option<u8>,
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
/// Item.xDD4_itemVar.pkflush (itnesspkflash.c) and .pkflushexplode: Ness's
/// PK Flash and its explosion.
#[derive(Clone, Copy, Debug, Default)]
pub struct PkFlashState {
    /// xDD8 (the flash) / xDD4 (the explosion): the charge, one per frame
    /// of flight up to the attribute.
    pub charge: f32,
    /// xDDC: a reflection took the flash out of its creator's hands.
    pub reflected: bool,
    /// xDE0: the creator, who steers it and whose release bursts it while
    /// it owns it (it_802AAA50 clears it).
    pub creator: Option<u8>,
    /// The explosion's xDD8: the hitbox's authored size, once read.
    pub hitbox_size: f32,
}
/// Item.xDD4_itemVar.pkthunder (itnesspkthunderball.c): PK Thunder's head.
#[derive(Clone, Copy, Debug, Default)]
pub struct PkThunderState {
    /// xDEC positions[16]: the head's position at its last sixteen physics
    /// callbacks, newest first.
    pub positions: [Vec3; 16],
    /// xEAC angles[16]: its heading at each, in radians.
    pub angles: [f32; 16],
    /// xDD4[6]: its tail segments, in spawn order.
    pub trails: [Option<u32>; 6],
    /// xEEC: its speed.
    pub speed: f32,
    /// xEF8: animation callbacks since the launch or the last reflection.
    pub frames: i32,
    /// xEFC: segments spawned since then.
    pub spawned: i32,
    /// xF00: a reflection took it out of its creator's hands.
    pub reflected: bool,
    /// xF04: the creator, who steers it while it owns it.
    pub creator: Option<u8>,
    /// it_802AB90C ran from one of the head's own callbacks, which told
    /// the creator there (retail clears xF04); otherwise it runs as the
    /// `destroyed` callback inside Item_8026A8EC.
    pub ended_by_callback: bool,
}
/// Item.xDD4_itemVar.nesspkthundertrail (itnesspkthundertrail.c): one tail
/// segment. Its head (x0) is the item's partner.
#[derive(Clone, Copy, Debug, Default)]
pub struct PkThunderTrailState {
    /// x4: which of the head's kept positions it follows.
    pub index: u8,
    /// x8: animation callbacks so far; the model shows on even ones.
    pub blink: u32,
    /// What it last read of the head: its own kept position, the next
    /// one, and the head's speed (it_802AB3F0, it_802AB468).
    pub point: Vec3,
    pub next: Vec3,
    pub length: f32,
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
    /// x38: the fighter that called the chain (its slot, and whether it is
    /// the player's second fighter). A reflection changes the bolt's owner
    /// and not this; only the fighter's removal clears it
    /// (itPikachuThunder_Logic39_EvtUnk).
    pub creator: Option<(u8, bool)>,
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
    /// The item's GObj (Item_80268B18's GObj_Create, item.c:957): what a
    /// hitbox's victim list holds of it.
    pub gobj: hsd_gobj::GObjAddress,
    /// The GObjs of a chain's or tether's links, in creation order
    /// (`SpawnItem::link_gobjs`).
    pub link_gobjs: melee_types::fixed::FixedVec<hsd_gobj::GObjAddress, MAX_LINK_GOBJS>,
    pub kind: ItemKind,
    pub owner: Option<u8>,
    /// The owner is its player's second fighter (x221F_b4: the Ice
    /// Climbers' Nana). Retail names the owner by its fighter GObj; the
    /// player slot and this flag together do the same.
    pub owner_secondary: bool,
    pub stale_source: Option<melee_types::combat::AttackInstance>,
    /// Current owner factor used only when authoring/re-authoring a hitbox.
    pub stale_multiplier: f32,
    /// The match's fighters' model scale and PlCo's damage curve
    /// (Fighter_804D6524 x4) when the scale is not 1 (Giant and Tiny
    /// Melee): it_80272460 scales a fighter-owned item's hitbox count.
    pub fighter_scale: Option<(f32, f32)>,
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
    /// itIsHeavy: a crate or barrel (ItemAttr x0 b?), which it_80271F78 pushes
    /// off other heavy items.
    pub heavy: bool,
    /// xDC8 x1C / x1D / x1E: fighters and other items push it, and a hold
    /// kind 3 item may push it; all set at creation.
    pub pushed_by_fighters: bool,
    pub pushed_by_items: bool,
    pub pushed_by_open_palm: bool,
    /// xDC8 x1B: its animation proc has run this frame (it_80272298).
    pub push_settled: bool,
    /// xBEC (ItemAttr x20, scaled by it_80274E44): its push box.
    pub push_box: melee_types::mp::ItEcb,
    /// xBDC: the kind's push box at the item's scale, which the accessory
    /// proc turns with the model into `push_box` (it_80271A58).
    pub push_box_base: melee_types::mp::ItEcb,
    /// xDD1 b0: it_8027518C started its explosion lifetime; items no
    /// longer push off it.
    pub exploding: bool,
    pub environmental_velocity: Vec3,
    pub platform_velocity: Vec3,
    pub facing: f32,
    pub rotation: Vec3,
    /// ItemModelDesc xC bits 6-7: how many first children down that joint is.
    pub spin_joint_depth: u8,
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
    /// The state's joint animation has no AObj (lbGetJObjCurrFrame finds
    /// none): x5CC, which item scripts time their frames by, reads zero.
    pub frame_without_aobj: bool,
    /// x5D0_animFrameSpeed: the joint animation's per-frame advance.
    pub animation_rate: f32,
    /// xDCF b2 (it_8027518C): the item ends without its destroy effect.
    pub destroy_effect_suppressed: bool,
    /// The kind's own code set ip->owner = NULL before Item_8026A8EC
    /// (it_802BE958_inline, Ness's yo-yo): a held item's destroy effect is
    /// not withheld (item.c:1993).
    pub owner_released: bool,
    /// Item_8026A8EC's efLib_DestroyAll already ran in the proc that ended
    /// the item.
    pub effects_destroyed: bool,
    /// The owner already heard of this item's end (`ItemLogic::NOTICE_AT_ONCE`).
    pub owner_notified: bool,
    pub script: ScriptState,
    /// The motion row whose decoded program owns the script's indices.
    /// Item_80268E5C preserves x524_cmd.u without ANIM_UPDATE/CMD_UPDATE,
    /// even when the current motion names a different command program.
    pub script_source_motion: Option<u16>,
    pub command_variables: [u32; 4],
    pub hitboxes: [Option<HitCapsule>; 4],
    /// The capsules a full clear (it_802725D4, a state change) disabled:
    /// retail only marks a slot disabled, so it_80272674 can enable the
    /// same capsule again.
    pub disabled_hitboxes: [Option<HitCapsule>; 4],
    /// The disabled capsules' victim timers (retail keeps them inside the
    /// capsule, where a disabled one does not count down).
    pub disabled_history: [melee_types::fixed::FixedVec<RehitVictim, 12>; 4],
    /// xCB8: the side this frame's hit went toward (ftcoll.c:1268,
    /// itcoll.c:508): by the item's speed, or its place against the victim
    /// when all but still.
    pub out_damage_direction: f32,
    /// it_804D6D28 +D4, for `aim_out_damage`.
    pub direction_still_speed: f32,
    pub scratch: ItemScratch,
    /// Item.jumped_on: what a fighter's stomp does (Item_80269B60, item
    /// link 11), for kinds that set it.
    pub jumped_on: Option<fn(&mut ItemCore, &ItemEventContext<'_>) -> bool>,
    /// xDD0 b0 (it_80274CAC / it_80274C88): fighters landing on it stomp
    /// it (ft_80081A00).
    pub stompable: bool,
    /// xCFC: the fighter that stomped it this frame, cleared after the
    /// link 11 proc reads it.
    pub stomped_by: Option<u8>,
    /// The stomper is its player's second fighter.
    pub stomped_by_secondary: bool,
    /// xCD0: the side the stomp came from (-1 when the item is right of
    /// the stomper's position, else +1).
    pub stomp_direction: f32,
    /// xDCE b3: the item sticks to the line xC30 (the Motion-Sensor Bomb):
    /// the physics proc takes that line's speed.
    pub stuck_to_line: bool,
    /// xDCE b6 with Item.toucher: an inert hitbox met this frame (a
    /// fighter's player, or an item's id), for `touched`.
    pub touched_by: Option<Toucher>,
    /// Item.touched: what an inert touch does (Item_8026A294's last case).
    pub touched: Option<fn(&mut ItemCore, &ItemEventContext<'_>) -> bool>,
    /// xB54: the box a stomp lands on (it_8027574C).
    pub stomp_box: crate::stomp::StompBox,
    pub hit_flags: [desc::ItemHitFlags; 4],
    /// x378_itemColl, created by it_80275E98 at spawn.
    pub collision: Option<melee_types::mp::CollData>,
    /// xC1C: the ECB box in use (Item_80267454 starts all three boxes from
    /// ItemAttr x40; xBFC stays the kind's).
    pub collision_box: melee_types::mp::ItEcb,
    /// xC0C: the box a landing restores (it_80275DFC), which is whatever
    /// was in use at the last release (it_80275BC8).
    pub saved_collision_box: melee_types::mp::ItEcb,
    /// xDC8 x17: the model rotation axis the fixed ECB follows (0 Z, 1 X, else Y).
    pub rotation_axis: u8,
    /// xC30: the floor (or wall/ceiling) line of the last map contact.
    pub floor_line: i32,
    /// xD5C: whether a floor's slope may start the item sliding
    /// (it_80277544). 0: never; 1: once it moves; 2: at once (every new
    /// item, item.c:636, and a slide that met a floor turning by ItCo +C4).
    pub slide_mode: u32,
    /// xDC8 x1F: the slide moves the item (it_8027737C).
    pub sliding: bool,
    /// x7C: the slide's velocity, beside the item's own.
    pub slide_velocity: Vec3,
    /// x88 / x94: the slope's pull this frame and the frame before
    /// (it_80277040).
    pub slide_acceleration: Vec3,
    pub previous_slide_acceleration: Vec3,
    /// xAC / xC2C: the normal and line of the surface the item last came
    /// to rest on (it_80276CEC). Creation stands the normal up but leaves
    /// the line as the struct's last user left it (`ItemPool`'s free list).
    pub rest_normal: Vec3,
    pub rest_line: i32,
    /// xD58: frames the slope's pull has turned against the frame before.
    pub slide_reversals: u32,
    /// xCF4 (ftColl_80077688): the fighter whose shield took this frame's
    /// strongest hit.
    pub pending_shield_owner: Option<u8>,
    /// A fighter whose shield the item reads each frame (the Link arrow's
    /// xC4), and the scene's view of it (or of `pending_shield_owner` for
    /// the hit-shield callback) refreshed before the item's procs.
    pub shield_anchor: Option<u8>,
    pub shield_view: Option<ShieldView>,
    /// xDC0: what pushed it this frame (`push::PUSHED_BY_FIGHTER`,
    /// `push::PUSHED_BY_ITEM`; it_802750E8 reads it).
    pub push_contacts: u8,
    /// xD50: landings since the last throw or bounce-free touchdown
    /// (it_8026DDFC, it_8026DD5C).
    pub land_count: u32,
    /// xD4C: shots left (the Star Rod and Lip's Stick set it at spawn;
    /// it_8026B594 reads it). The Fan reads it unwritten: a stale heap word
    /// (`ItemPool::set_heap_ammo_words`). Zero for every other kind, which
    /// never reads it.
    pub ammo: i32,
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
    /// xDCE b7, set at creation (item.c:672): each map pass re-aims the
    /// fixed ECB at the model's rotation (it_80276214 -> it_80276278). A
    /// kind that clears it keeps the ECB upright while its model spins.
    pub ecb_follows_rotation: bool,
    /// The rotation of the joint ItemModelDesc xC bits 6-7 name when it is
    /// not the root (it_802746F8): the spin turns it, and the push box
    /// (it_80271A58) and the ECB angle (it_80274990) read it. `None`: the
    /// root's (`rotation`).
    pub spin_joint_rotation: Option<Vec3>,
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
    /// HSD_JObjSetTranslate on a joint below the root, for kinds whose code
    /// moves that joint after its animation step (the Shadow Ball's
    /// wavering grandchild): `(joint, translation)`.
    pub joint_translation: Option<(usize, Vec3)>,
    /// HSD_JObjSetScale on a joint below the root, likewise.
    pub joint_scale: Option<(usize, Vec3)>,
    /// HSD_JObjSetTranslateZ on a joint below the root (a clinging ReDead's
    /// joint 1): only Z replaces the animated translation.
    pub joint_translation_z: Option<(usize, f32)>,
    /// lb_8000C1C0's position constraint on the model root: the world
    /// position of the fighter part it hangs from, which replaces the
    /// root's translation in every bone matrix. The scene refreshes it
    /// before the item's bones are read.
    pub constrained_root: Option<Vec3>,
    /// The world translation of the holder part a held item's attach joint
    /// (ItemModelDesc x8) is constrained to (it_80274F48), sampled by the
    /// scene before the item's hitboxes are placed: a capsule on that joint
    /// with no offset sits there (the ridden Warp Star's).
    pub holder_attach: Option<Vec3>,
    /// xDC8 x13 (it_802742F4): held by its owner. A held item neither moves
    /// nor leaves the blast zones.
    pub held: bool,
    /// xDC4: the holder's part it hangs from, a fp->parts index (the
    /// character's own joint numbering, as ftData x8 stores it).
    pub holder_part: u8,
    /// A held item's root JObj world translation as the last display pass
    /// left it, at its holder part's matrix (the hand constraint), while no
    /// HSD_JObjSetTranslate has dirtied the root since (Item_80269978's, in
    /// the collision proc, or Item_80268E5C's). it_80278800's lb_8000B1CC
    /// reads that cached matrix for the destroy effect.
    pub held_display_root: Option<Vec3>,
    /// Whether HSD_JObjSetMtxDirty reached the model's joints since the
    /// particle pass last set up the bones its generators follow
    /// (hsd_8039D214's HSD_JObjSetupMatrix): an HSD_JObjSetTranslate on the
    /// root (Item_80269978, Item_80268E5C, a release), an animation step
    /// (Item_802694CC's HSD_JObjAnimAll) or a joint's scale. A generator
    /// kept on a bone of an item torn down while clean reads the bone's
    /// last world matrix; a dirty one is rebuilt from its local SRT alone.
    pub joints_dirty: bool,
    /// xDCD b5 (it_80275444 / it_80275474): hitboxes may hit the owner.
    pub hits_owner: bool,
    /// xDCF b0: a bounce makes no spark (it_80277C40); a coin sets it at
    /// its third landing in a row (it_802E5AC4).
    pub bounce_spark_off: bool,
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
    /// xACC (the first hurt capsule) as a stage rewrote it
    /// (grMaterial_801C8DE0): its two offsets and radius.
    pub hurt_override: Option<([Vec3; 2], f32)>,
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
    /// xCC4: the strongest hit's element (it_80270E30), which it_8027CBFC reads.
    pub hit_element: melee_types::HitElement,
    /// xDCF b6: the strongest hit's x43_b0 (it_80270E30, itcoll.c:862).
    pub hit_interacts: bool,
    /// xDD0 b4 (it_80275270 / it_80275258): a fighter's grab may catch it
    /// (ftColl_8007BC90).
    pub catchable: bool,
    /// xD09 (it_80274ECC): ftColl_8007BC90 passes it over.
    pub catch_ignored: bool,
    /// The strongest hit names its player's second fighter, including an article owner.
    pub hit_by_secondary: bool,
    /// The strongest hit this frame came from an item (xCF0, not xCEC).
    pub struck_by_item: bool,
    /// ftLib_800866DC of xCEC, the fighter whose hit was the last
    /// strongest: the scene fills it before Item_8026A294 for the kinds
    /// whose damage callback reads it (it_802EF548).
    pub attacker_position: Option<Vec3>,
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
    pub link_requests: melee_types::fixed::FixedVec<crate::LinkRequest, 8>,
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
impl ItemCore {
    /// Continue this callback after its preceding synchronous effects run.
    /// The scene drains later, but retail still observes this proc's phase.
    pub fn defer_callback(&mut self, step: u8) {
        self.events.push(ItemEvent::Resume {
            step,
            past_hitbox_refresh: self.past_hitbox_refresh,
            efasync_immediate: self.efasync_immediate,
        });
    }

    /// lb_8000B804 on the model: back to its authored rotation.
    pub fn reset_model_rotation(&mut self, assets: &ItemAssets) {
        self.rotation = rest_rotation(assets);
        self.spin_joint_rotation = assets.spin_joint_rest();
    }
    /// The spin joint's (it_802746F8) rotation about xDC8 x17's axis: Z, X,
    /// then Y.
    pub(crate) fn spin_axis_angle_mut(&mut self) -> &mut f32 {
        let joint = self.spin_joint_rotation.as_mut().unwrap_or(&mut self.rotation);
        match self.rotation_axis {
            0 => &mut joint.z,
            1 => &mut joint.x,
            _ => &mut joint.y,
        }
    }
}
/// lb_8000B804: the model root's authored rotation.
pub(crate) fn rest_rotation(assets: &ItemAssets) -> Vec3 {
    let r = assets.visual.model.rotation;
    Vec3::new(r.x, r.y, r.z)
}
/// The most ItemLink GObjs one article has: the Grapple Beam's doubled
/// rope (it_802B75FC) has sixty links.
pub const MAX_LINK_GOBJS: usize = 64;
/// The tag that keeps item GObj addresses apart from fighter spawn numbers.
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
    /// efAsync_QueueFlush on the item's own queue, as a motion change does
    /// (Item_80268E5C, retail 0x80268F44): what the proc queued so far is
    /// dispatched once it returns, ahead of every later link 9 flush.
    fn flush_queued_effects(&mut self) {
        // A script or bounce effect this proc already requested
        // (it_80278800) sits in the queue the flush empties; the scene only
        // resolves it after the proc, so it is marked as dispatched now.
        for event in self.events.iter_mut() {
            if let ItemEvent::ScriptEffect(graphics) = event {
                *event = ItemEvent::ScriptEffectNow(graphics.clone());
            }
        }
        while !self.queued_events.is_empty() {
            let event = self.queued_events.remove(0);
            self.events.push(event);
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
    /// Item_8026A8EC: the kind's destroyed callback frees its links from
    /// the newest (the list walk of it_802BB20C and its like), then the
    /// item's own GObj goes.
    fn destroy_gobjs(&mut self, gobjs: &mut hsd_gobj::GObjAddresses) {
        while let Some(link) = self.link_gobjs.pop() {
            gobjs.destroy(hsd_gobj::GObjClass::ItemLink, link);
        }
        gobjs.destroy(hsd_gobj::GObjClass::Item, self.gobj);
    }
    /// How hitboxes list this item among their victims. Retail stores GObj
    /// pointers (lbColl_80008688) and never clears a destroyed victim's, so
    /// an item created where a struck one was is taken for it: the key is
    /// the item's GObj address, top bit set; fighters, whose GObjs last
    /// the match, are keyed by spawn number.
    pub fn hitbox_victim(&self) -> u32 {
        ITEM_VICTIM | self.gobj.index()
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
        self.reset_model_rotation(assets);
        self.model_scale = Vec3::new(self.scale, self.scale, self.scale);
        // ftLib_80086960: fighters only; xDC8 x0 (a secondary owner) is
        // never set on a pickup-capable kind here.
        self.owner = Some(owner);
        self.held = true;
        self.holder_part = part;
        // it_802756D0: a held item takes no hits.
        self.hurt_intangible = true;
        // it_802742F4 (it_2725.c:1001): the pickup ends the creation
        // countdown (xDD0 b6, xD40), so its end cannot restore hits
        // (it_802756E0) while the item is in a hand.
        self.spawn_intangible_frames = None;
        // it_8026B3A8.
        self.grabbable = false;
        // it_80274C88 (it_2725.c:1012): no longer stompable.
        self.stompable = false;
        // The pickup (it_26B1.c:597) clears xDC8 x14.
        self.speed_damage = false;
        if (self.kind as u32) < ItemKind::LGunRay as u32 {
            // it_80275158: both timers.
            self.life_timer = lifetime;
            self.half_life = lifetime * half_life_scale;
        }
    }
    /// Item_80268E5C (80268E5C) for the kinds without an article row.
    pub fn change_motion(&mut self, motion: u16, assets: &ItemAssets) {
        self.motion = motion;
        // Item_80268E5C: HSD_JObjSetTranslate(jobj, &pos) first, so a
        // hitbox the script makes at once (a laser's, after its initial
        // collision moved it) is placed from the resolved position.
        self.set_root_translation(self.position);
        self.held_display_root = None;
        self.flush_queued_effects();
        self.article_state = usize::from(motion);
        self.animation_frame = 0.0;
        self.frame_without_aobj = !assets.has_joint_animation(self.article_state);
        self.restart_script(motion, assets);
        self.advance_script(assets);
    }
    fn clear_script(&mut self) {
        self.script = ScriptState::default();
        self.script_source_motion = None;
    }
    /// Retail 0x80269314..28 / 0x80269340..54 replace the command pointer
    /// and reset its control state together; the motion alone does neither.
    fn restart_script(&mut self, motion: u16, assets: &ItemAssets) {
        self.clear_script();
        if assets
            .scripts
            .get(usize::from(motion))
            .is_some_and(|s| !s.is_empty())
        {
            self.script_source_motion = Some(motion);
            self.script.restart(0);
        }
    }
    /// it_80274740 (80274740): the spin joint's angle about xDC8 x17's axis
    /// and the spin speed return to zero.
    pub fn reset_spin(&mut self) {
        self.spin_speed = 0.0;
        *self.spin_axis_angle_mut() = 0.0;
    }
    /// Item_80268D34 then HSD_JObjAnimAll (itlinkbomb.c's lit fuse): the
    /// model plays `article_state`'s joint animation from its first frame
    /// while the motion state and its script stay.
    pub fn play_article_animation(&mut self, article_state: usize, assets: &ItemAssets) {
        self.article_state = article_state;
        self.animation_frame = 0.0;
        self.frame_without_aobj = !assets.has_joint_animation(article_state);
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
        let speed = self.spin_speed;
        *self.spin_axis_angle_mut() += speed;
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
        self.set_root_translation(self.position);
        self.held_display_root = None;
        // retail 0x80268F44: efAsync_QueueFlush(gobj, &ip->xBC0).
        self.flush_queued_effects();
        self.animation_frame = 0.0;
        self.speed_damage = false;
        // item.c:1278: xD09 (it_80274ECC) lasts until the next state change.
        self.catch_ignored = false;
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
            self.park_hitboxes();
            for history in &mut self.reflection_history {
                history.clear();
            }
        }
        if article_state < 0 {
            self.frame_without_aobj = false;
            self.clear_script();
            return;
        }
        self.article_state = article_state as usize;
        self.frame_without_aobj = !assets.has_joint_animation(self.article_state);
        // Item_80268D34's HSD_JObjReqAnimAll(0) with ANIM_UPDATE, then the
        // HSD_JObjAnimAll every state change takes.
        if flags & state_change::ANIM_UPDATE != 0 {
            // Item_80268D34 returns the joints to their rest pose.
            self.joint_translation_z = None;
        }
        self.pose_steps = if flags & state_change::ANIM_UPDATE != 0 {
            1
        } else {
            self.pose_steps + 1
        };
        self.emit_particle_keys(assets, self.position);
        if flags & (state_change::ANIM_UPDATE | state_change::CMD_UPDATE) != 0 {
            self.restart_script(motion, assets);
        }
        self.advance_script(assets);
    }
    /// Item_80268E5C (80268E5C) with neither ITEM_ANIM_UPDATE nor
    /// ITEM_CMD_UPDATE (Tingle's ITEM_UNK_0x1 changes): the model's joint
    /// animation is not requested again, so it plays on through the
    /// change's HSD_JObjAnimAll, and the pose, its particle keys and its end
    /// (lb_8000B09C) follow the animation the model already plays; x5CC
    /// reads that animation's frame again from the next step. The script
    /// carries on, its step at the change seeing x5CC at zero.
    pub fn change_motion_keeping_animation(&mut self, motion: u16, flags: u32, assets: &ItemAssets) {
        assert_eq!(
            flags & (state_change::ANIM_UPDATE | state_change::CMD_UPDATE),
            0,
            "a change that requests the animation"
        );
        let (article, frame) = (self.article_state, self.animation_frame);
        self.change_motion_with(motion, article as i32, flags, assets);
        if assets.model != 0 {
            self.animation_frame = frame + self.animation_rate;
        }
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
    /// xCB8 for a hit on something at `victim_x` ([`out_damage_direction`]).
    pub fn aim_out_damage(&mut self, victim_x: f32) {
        self.out_damage_direction = out_damage_direction(
            self.velocity.x,
            self.position.x,
            self.direction_still_speed,
            victim_x,
        );
    }
    /// Every live hitbox disabled, its capsule kept for it_80272674.
    pub(crate) fn park_hitboxes(&mut self) {
        for id in 0..self.hitboxes.len() {
            self.park_hitbox(id);
        }
    }
    fn park_hitbox(&mut self, id: usize) {
        if let Some(hit) = self.hitboxes[id].take() {
            self.disabled_hitboxes[id] = Some(hit);
            self.disabled_history[id] = core::mem::take(&mut self.reflection_history[id]);
        }
    }
    /// it_80272674 (80272674): hitbox `id`, disabled, is enabled again
    /// (lbColl_80008434) and placed at once (prev = current, state 2).
    pub fn restore_hitbox(&mut self, id: usize, assets: &ItemAssets) {
        let mut hit = self.disabled_hitboxes[id]
            .take()
            .unwrap_or_else(|| unimplemented!("it_80272674: a slot no hitbox ever used"));
        hit.phase = melee_coll::hitbox::CapsulePhase::Enabled;
        self.hitboxes[id] = Some(hit);
        self.reflection_history[id] = core::mem::take(&mut self.disabled_history[id]);
        self.update_hitbox(id, assets);
    }
    /// it_8027570C / it_8027572C: hitbox `id`'s clank flag (HitCapsule x40
    /// b0). A cleared slot keeps nothing: the next script hitbox there
    /// writes its own flag.
    pub fn set_hitbox_clank(&mut self, id: usize, clank: bool) {
        if let Some(hit) = &mut self.hitboxes[id] {
            hit.descriptor.clank = clank;
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
        let translate = self.constrained_root.unwrap_or(self.root_translation);
        let root = crate::pose::RootSrt {
            translate,
            ..self.root_srt()
        };
        let (state, steps) = (self.article_state, self.pose_steps);
        let Some(hit) = &mut self.hitboxes[id] else {
            return;
        };
        if self.held && hit.descriptor.bone == assets.visual.attachment_bone {
            // it_80274F48's position constraint: the joint's world
            // translation is the holder part's.
            assert!(
                hit.descriptor.offset == Vec3::ZERO,
                "it_8027129C: an offset capsule on a held item's attach joint"
            );
            let position = self
                .holder_attach
                .expect("a held item's attach joint before the scene sampled its holder");
            hit.update_position(position);
            self.hitbox_trace[id] = (hit.previous_position, hit.position);
            return;
        }
        let matrix = if hit.descriptor.bone == 0 {
            let mut matrix = hsd_types::Mtx::default();
            hsd_anim::mtx::hsd_mtx_srt(
                &mut matrix,
                &self.model_scale,
                &self.rotation,
                &translate,
                None,
            );
            matrix
        } else {
            let pose = assets
                .pose
                .as_ref()
                .expect("item non-root hitbox bone without a sampled pose");
            pose.bone_matrix_posed(
                state,
                steps,
                hit.descriptor.bone,
                root,
                self.child_rotation_x.map(|x| (CHILD_JOINT, x)),
                self.joint_translation,
                self.joint_translation_z,
                self.joint_scale,
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
        if self.destroy_effect_suppressed
            || (self.held && self.owner.is_some() && !self.owner_released)
        {
            return;
        }
        if let Some(id) = effect {
            self.events.push(ItemEvent::DestroyEffect { id, root });
        }
    }
    /// Item_80268DD4 (80268DD4): the joint animation requested at `frame`
    /// and stepped once (HSD_JObjReqAnimAll, HSD_JObjAnimAll); the script
    /// is left as it is.
    pub fn jump_animation_to(&mut self, frame: f32) {
        self.animation_frame = frame;
        self.pose_steps += 1;
    }
    /// it_80273670(item, 0, frame) (80273670): the model posed at article
    /// state 0's `frame`; its joint animation and script are removed, so no
    /// further script command runs until the next state change.
    pub fn pose_article_frame(&mut self, frame: f32) {
        self.animation_frame = frame;
        self.clear_script();
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
        // it_80274E44: the push box (xBEC) and its unturned source (xBDC).
        self.push_box.top *= scale;
        self.push_box.bottom *= scale;
        self.push_box.right *= scale;
        self.push_box.left *= scale;
        self.push_box_base.top *= scale;
        self.push_box_base.bottom *= scale;
        self.push_box_base.right *= scale;
        self.push_box_base.left *= scale;
    }
    /// ftLib_80086A0C of the owner (x34_scale.y): a fighter owner's model
    /// scale, 1 for an item no fighter owns.
    pub fn owner_model_scale(&self) -> f32 {
        match (self.owner, self.fighter_scale) {
            (Some(_), Some((scale, _))) => scale,
            _ => 1.0,
        }
    }
    /// it_80272460's first step (itcoll.c:1408-1412): a fighter-owned
    /// item's count along the owner's model scale, rounded up from 0.999.
    pub fn owner_scaled_count(&self, count: u32) -> u32 {
        match (self.owner, self.fighter_scale) {
            (Some(_), Some((scale, curve))) => {
                (0.999 + melee_coll::damage::y_scaled(count as f32, scale, curve)) as u32
            }
            _ => count,
        }
    }
    /// it_80272460 (80272460) for an existing hitbox `id`: its count and
    /// the damage restaled for the item's attack (ft_80089228).
    pub fn set_hitbox_damage(&mut self, id: usize, damage: u32) {
        let damage = self.owner_scaled_count(damage);
        if let Some(hit) = &mut self.hitboxes[id] {
            hit.knockback_damage = damage;
            hit.descriptor.damage = damage as f32 * self.stale_multiplier;
        }
    }
    /// Item_802694CC (802694CC): the joint animation advances by the item's
    /// rate (HSD_JObjAnimAll, x5CC), then its script runs (it_802799E4).
    /// HSD_JObjSetTranslate on the model root: the joints are dirty.
    pub fn set_root_translation(&mut self, translation: Vec3) {
        self.root_translation = translation;
        self.joints_dirty = true;
    }
    pub fn advance_animation(&mut self, assets: &ItemAssets) {
        // lbGetJObjCurrFrame: an article without a model (Item_80267978's
        // bare JObj) has no AObj, so x5CC stays zero.
        // A state whose joint animation has no AObj reads zero likewise
        // (the Party Ball's hang times its opening by it).
        if assets.model != 0 {
            if self.frame_without_aobj {
                self.animation_frame = 0.0;
            } else {
                self.animation_frame += self.animation_rate;
            }
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
                joint: key.joint,
                position,
                rotation: self.rotation,
                scale: self.model_scale,
            });
        }
    }
    /// Item_802799E4: shared command timing, item-owned application.
    fn advance_script(&mut self, assets: &ItemAssets) {
        let Some(source) = self.script_source_motion.map(usize::from) else {
            return;
        };
        let script = &assets.scripts[source];
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
                    if self.owner.is_some() && self.fighter_scale.is_some() {
                        descriptor.damage =
                            self.owner_scaled_count(descriptor.damage as u32) as f32;
                    }
                    melee_coll::hitbox::spawn(&mut self.hitboxes, *id, &descriptor);
                    let hit = self.hitboxes[*id].as_mut().unwrap();
                    // it_8026FCF8 (8026FCF8): outside a hit group
                    // (xAC4_ignoreItemID 0) a capsule that starts takes no
                    // sibling's history: lbColl_80008440 empties it (the PK
                    // Fire pillar's second capsule strikes at once).
                    if starts && self.hit_group == 0 {
                        hit.clear_victims();
                        self.reflection_history[*id].clear();
                    }
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
                    self.hit_flags[*id] = assets.hit_flags[source][index].expect("item hit flags");
                    // it_802790C0: created after link 11's refresh, the
                    // capsule is placed at once.
                    if self.past_hitbox_refresh {
                        self.update_hitbox(*id, assets);
                    }
                }
                Command::SetHitboxDamage { id, damage } => {
                    let count = self.owner_scaled_count(gekko_math::msl::fctiwz(*damage) as u32);
                    if let Some(hit) = &mut self.hitboxes[*id] {
                        hit.knockback_damage = count;
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
                    self.park_hitbox(*id);
                    self.reflection_history[*id].clear();
                    // it_80272560 updates every surviving capsule immediately.
                    self.update_hitboxes(assets);
                }
                Command::ClearHitboxes => {
                    self.park_hitboxes();
                    for history in &mut self.reflection_history {
                        history.clear();
                    }
                }
                Command::SetVariable { index, value } => self.command_variables[*index] = *value,
                Command::Graphics(graphics) => {
                    let graphics = graphics.clone();
                    self.events
                        .push(if self.past_hitbox_refresh || self.efasync_immediate {
                            ItemEvent::ScriptEffectNow(graphics)
                        } else {
                            ItemEvent::ScriptEffect(graphics)
                        })
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
                // it_80279B88 / it_80279BBC: the colour overlay, drawing
                // only (the Party Ball's flash).
                Command::ItemColorOverlay(_) => {}
                // `desc::read_block` decodes only the opcodes above; the
                // shared flow commands (Command_00..09) stay in the script
                // state machine.
                _ => unreachable!("item command {command:?} was never decoded"),
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
    /// it_804D6D00: the Warp Star flight path drawn last (it_80294364), -1
    /// from Item_80266FA8 on.
    pub warp_star_path: i8,
    /// Every fighter's model scale and PlCo's damage curve when the match
    /// scales fighters (Giant and Tiny Melee), for new items
    /// (`ItemCore::fighter_scale`).
    pub fighter_scale: Option<(f32, f32)>,
    /// The id Item_8026AE60 last gave each fighter that asked for one
    /// (ItemRequest::NewHitGroup), keyed by owner slot and fighter; the
    /// fighter keeps it in its motion scratch (Bowser's mv.kp.specials.x4).
    pub(crate) owner_hit_groups: [u32; crate::hit_group::OWNER_GROUPS],
    /// Item_804A0CCC: the fighters it_802722B0 last sampled.
    pub(crate) fighter_push: crate::push::FighterPushSnapshot,
    /// item_alloc_data's free list, newest first at the end, as the one
    /// word of a freed Item a new one reads before writing: xC2C
    /// (`ItemCore::rest_line`). HSD_ObjFree pushes and HSD_ObjAlloc pops the
    /// head (objalloc.c), and Item_80268B18 does not clear the struct.
    /// Never longer than the pool's peak, so it does not grow.
    freed_rest_lines: Vec<i32>,
    /// The items marked destroyed and not yet removed, in the order they
    /// were marked (`note_destroyed`): Item_8026A8EC frees each one's GObj
    /// where it is called, so that order is the allocator's.
    destroy_order: melee_types::fixed::FixedVec<u32, ITEM_CAPACITY>,
    /// xD4C of each new item whose kind reads it unwritten (the Fan,
    /// `ItemLogic::READS_HEAP_AMMO`), in creation order: whatever the heap
    /// held where HSD_ObjAlloc placed the Item (Item_80268B5C leaves it).
    /// Retail recordings show heap garbage (0x3B65664E, 0xF1F1F1F1, 0, ...)
    /// that no game state decides, so a scenario names the words its
    /// recording read (`Setup::heap_ammo_words`); past the list, or with
    /// none, [`DEFAULT_HEAP_AMMO`].
    heap_ammo_words: Vec<i32>,
    heap_ammo_next: usize,
}

/// xD4C for a new Fan when no word is named: nonzero, as about three Fans
/// in four read in the retail recordings, so a dropped Fan lands as a
/// thrown one (it_8026E15C) instead of vanishing (it_8026DF34).
pub const DEFAULT_HEAP_AMMO: i32 = 1;

/// xC2C of an Item struct no item has used: whatever the heap held (retail
/// traces show 0xC0C0C0C0, 0x8A97CE97 and 0x95428D21), which is no line.
const UNUSED_REST_LINE: i32 = i32::MIN;

impl ItemPool {
    pub fn new(common: ItemCommonData) -> Self {
        Self {
            items: Vec::with_capacity(ITEM_CAPACITY),
            common,
            next_id: 0,
            next_hit_group: 1,
            warp_star_path: -1,
            owner_hit_groups: [0; crate::hit_group::OWNER_GROUPS],
            fighter_push: Default::default(),
            freed_rest_lines: Vec::with_capacity(ITEM_CAPACITY),
            destroy_order: Default::default(),
            fighter_scale: None,
            heap_ammo_words: Vec::new(),
            heap_ammo_next: 0,
        }
    }
    /// The stale xD4C words new Fans read, in creation order (see
    /// `heap_ammo_words`). Called once, before the match steps.
    pub fn set_heap_ammo_words(&mut self, words: &[i32]) {
        self.heap_ammo_words = words.to_vec();
        self.heap_ammo_next = 0;
    }
    /// xD4C of a new item: the next named heap word for a kind that reads
    /// it unwritten, else 0 (the ammo kinds write theirs in `spawned`).
    fn heap_ammo(&mut self, reads_heap_ammo: bool) -> i32 {
        if !reads_heap_ammo {
            return 0;
        }
        let word = self
            .heap_ammo_words
            .get(self.heap_ammo_next)
            .copied()
            .unwrap_or(DEFAULT_HEAP_AMMO);
        self.heap_ammo_next += 1;
        word
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
    ) -> (&ItemCommonData, impl DoubleEndedIterator<Item = &mut ItemCore>) {
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
    pub fn spawn<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        gobjs: &mut hsd_gobj::GObjAddresses,
    ) -> Option<u32> {
        self.spawn_with_stale::<D>(spawn, assets, 1.0, gobjs)
    }
    pub fn spawn_with_stale<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        stale_multiplier: f32,
        gobjs: &mut hsd_gobj::GObjAddresses,
    ) -> Option<u32> {
        assert!(
            D::logic(spawn.kind).facing_constructor.is_none()
                && !D::logic(spawn.kind).constructor_reads_map,
            "Item_8026A810: this constructor requires live facing context"
        );
        let mut item = self.prepare_spawn(spawn, assets, stale_multiplier, gobjs)?;
        item.ammo = self.heap_ammo(D::logic(item.kind).reads_heap_ammo);
        item.update_spin(self.common.spawn_spin_degrees);
        (D::logic(item.kind).spawned)(&mut item, assets);
        Some(self.finish_spawn(item))
    }

    /// Item_80267130 -> initial collision, root translation and spin, then
    /// Item_8026A810. Only table-owned map-reading constructors use this path.
    pub fn spawn_with_initial_collision<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        gobjs: &mut hsd_gobj::GObjAddresses,
        map: &mut melee_mp::CollMap,
    ) -> Option<u32> {
        assert!(D::logic(spawn.kind).constructor_reads_map);
        let mut item = self.prepare_spawn(spawn, assets, 1.0, gobjs)?;
        item.ammo = self.heap_ammo(D::logic(item.kind).reads_heap_ammo);
        item.initialize_collision(spawn, assets, map);
        item.set_root_translation(item.position);
        item.update_spin(self.common.spawn_spin_degrees);
        (D::logic(item.kind).spawned)(&mut item, assets);
        Some(self.finish_spawn(item))
    }

    /// Item_8026A810: initial collision precedes the constructor's live
    /// facing query (it_802D73F0 / it_802E8BCC). Ordinary kinds keep their
    /// original spawn path and never evaluate this query.
    pub fn spawn_with_facing_constructor<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        gobjs: &mut hsd_gobj::GObjAddresses,
        map: &mut melee_mp::CollMap,
        query: impl FnOnce(Vec3) -> f32,
    ) -> Option<u32> {
        let constructor = D::logic(spawn.kind)
            .facing_constructor
            .expect("a table-owned constructor requiring live facing context");
        let mut item = self.prepare_spawn(spawn, assets, 1.0, gobjs)?;
        item.ammo = self.heap_ammo(D::logic(item.kind).reads_heap_ammo);
        item.initialize_collision(spawn, assets, map);
        item.set_root_translation(item.position);
        item.update_spin(self.common.spawn_spin_degrees);
        (constructor.before_query)(&mut item, assets);
        item.facing = query(item.position);
        (constructor.after_query)(&mut item, assets);
        Some(self.finish_spawn(item))
    }

    // Concrete shared initialization, compiled once rather than per query closure.
    fn prepare_spawn(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        stale_multiplier: f32,
        gobjs: &mut hsd_gobj::GObjAddresses,
    ) -> Option<ItemCore> {
        if let Some(limit) = self.common.hold_limits[usize::from(spawn.hold_kind)] {
            if self
                .items
                .iter()
                .filter(|i| i.hold_kind == spawn.hold_kind && !i.destroyed)
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
        let item = ItemCore {
            ground_or_air: spawn.ground_or_air,
            id,
            gobj: gobjs.create(hsd_gobj::GObjClass::Item),
            link_gobjs: (0..spawn.link_gobjs)
                .map(|_| gobjs.create(hsd_gobj::GObjClass::ItemLink))
                .collect(),
            kind: spawn.kind,
            owner: spawn.owner,
            owner_secondary: false,
            stale_source: spawn.stale_source,
            stale_multiplier,
            fighter_scale: self.fighter_scale,
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
            heavy: assets.heavy,
            pushed_by_fighters: true,
            pushed_by_items: true,
            pushed_by_open_palm: true,
            push_settled: false,
            push_box: assets.push_box,
            push_box_base: assets.push_box,
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
            spin_joint_depth: assets.visual.spin_joint_depth,
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
            frame_without_aobj: false,
            animation_rate: 1.0,
            destroy_effect_suppressed: false,
            owner_released: false,
            effects_destroyed: false,
            owner_notified: false,
            script: ScriptState::default(),
            script_source_motion: None,
            command_variables: [0; 4],
            hitboxes: std::array::from_fn(|_| None),
            disabled_hitboxes: std::array::from_fn(|_| None),
            disabled_history: Default::default(),
            out_damage_direction: 0.0,
            direction_still_speed: self.common.direction_still_speed,
            scratch: ItemScratch::None,
            jumped_on: None,
            stompable: false,
            stomped_by: None,
            stomped_by_secondary: false,
            stomp_direction: 0.0,
            stomp_box: Default::default(),
            stuck_to_line: false,
            touched_by: None,
            touched: None,
            hit_flags: [desc::ItemHitFlags::default(); 4],
            collision: None,
            collision_box: assets.collision_box,
            saved_collision_box: assets.collision_box,
            rotation_axis: assets.rotation_axis,
            floor_line: -1,
            // Item_80267454 (item.c:635): xD58 = 0, xD5C = 2; it_80273500
            // -> it_802734B4 zeroes the slide and stands xAC up.
            slide_mode: 2,
            sliding: false,
            slide_velocity: Vec3::ZERO,
            slide_acceleration: Vec3::ZERO,
            previous_slide_acceleration: Vec3::ZERO,
            rest_normal: Vec3::new(0.0, 1.0, 0.0),
            rest_line: self.freed_rest_lines.pop().unwrap_or(UNUSED_REST_LINE),
            slide_reversals: 0,
            land_count: 0,
            push_contacts: 0,
            ammo: 0,
            throw_count: 0,
            pending_shield_owner: None,
            shield_anchor: None,
            shield_view: None,
            hidden: false,
            spin_rate: assets.spin_rate,
            spin_speed: 0.0,
            spin_ignores_facing: assets.rotate_to_facing,
            ecb_follows_rotation: true,
            spin_joint_rotation: assets.spin_joint_rest(),
            // Item_80267130 (item.c:233): it_8026B390.
            grabbable: true,
            grab_offset: assets.grab_offset,
            grab_range: assets.grab_range,
            hitbox_size: 0.0,
            child_rotation_x: None,
            joint_translation: None,
            joint_scale: None,
            joint_translation_z: None,
            constrained_root: None,
            holder_attach: None,
            root_translation: if spawn.initial_collision {
                spawn.previous_position
            } else {
                spawn.position
            },
            held: false,
            holder_part: 0,
            held_display_root: None,
            // Item_80268B18 builds a new JObj tree, dirty until set up.
            joints_dirty: true,
            hits_owner: false,
            bounce_spark_off: false,
            throw_speed: 1.0,
            hitbox_damage_scale: 1.0,
            past_hitbox_refresh: false,
            hurt_intangible: false,
            hurt_override: None,
            spawn_intangible_frames: None,
            keeps_intangible: false,
            speed_damage: false,
            hurt_by_owner: false,
            hit_element: melee_types::HitElement::Normal,
            hit_interacts: false,
            catchable: false,
            catch_ignored: false,
            strikes_kindred_items: false,
            damage_percent: 0,
            pending_damage_taken: 0,
            largest_damage_taken: 0,
            pending_knockback: 0.0,
            knockback_angle: 0,
            hit_direction: 0.0,
            hit_by: None,
            hit_by_secondary: false,
            struck_by_item: false,
            attacker_position: None,
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
        Some(item)
    }

    fn finish_spawn(&mut self, mut item: ItemCore) -> u32 {
        item.begin_spawn_intangibility(self.common.spawn_intangible_frames);
        let id = item.id;
        self.items.push(item);
        id
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
        self.process_events_with_stale::<D>(id, 1.0, assets, &rng, None, None);
    }
    pub fn process_events_with_stale<D: ItemDispatch>(
        &mut self,
        id: u32,
        reflected_stale: f32,
        assets: &ItemAssets,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
        map: Option<&melee_mp::CollMap>,
        camera_position: Option<Vec3>,
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
                map,
                camera_position,
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).damage_received)(item, &context);
        } else if item.pending_shield_damage != 0 {
            if let Some(deflection) = item.pending_shield_deflection.filter(|d| {
                item.ground_or_air == melee_types::GroundOrAir::Air && d.angle < bounce_limit
            }) {
                let context = ItemEventContext {
                    shield_normal: deflection.normal,
                    rng: Some(rng),
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
            item.reflect::<D>(reflection, reflected_stale, cap, assets, &common, rng);
        } else if item.pending_absorb.is_some() {
            // Item_8026A294: the kind's absorbed callback.
            let context = ItemEventContext {
                rng: Some(rng),
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).absorbed)(item, &context);
        } else if item.touched_by.is_some() {
            // item.c:1836: xDCE b6 runs Item.touched.
            if let Some(touched) = item.touched {
                let context = ItemEventContext {
                    rng: Some(rng),
                    ..ItemEventContext::new(assets, &common)
                };
                item.destroyed |= touched(item, &context);
            }
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
        item.hit_element = melee_types::HitElement::Normal;
        item.hit_interacts = false;
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
        item.out_damage_direction = 0.0;
        // Item_80269CC4 (item.c:1548, 1570): toucher and xDCE b6.
        item.touched_by = None;
        // Item_80269CC4: xCFC and xCD0.
        item.stomped_by = None;
        item.stomped_by_secondary = false;
        item.stomp_direction = 0.0;
    }
    /// Item_80269B60 (item link 11)'s head: an item a fighter stomped this
    /// frame (xCFC) runs its jumped_on callback, which may end it.
    pub fn run_stomp(
        &mut self,
        id: u32,
        assets: &ItemAssets,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let common = self.common.clone();
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let (Some(_), Some(jumped_on)) = (item.stomped_by, item.jumped_on) else {
            return;
        };
        let context = ItemEventContext {
            rng: Some(rng),
            ..ItemEventContext::new(assets, &common)
        };
        if jumped_on(item, &context) && !item.destroyed {
            // destroy_type 2, Item_8026A8EC: the rest of the item's procs
            // do not run.
            item.destroyed = true;
            item.queue_destroy_effect(assets.event_destroy_effect, None);
        }
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
    /// How many items the pool has created: the next item's id.
    pub fn issued_ids(&self) -> u32 {
        self.next_id
    }
    /// The live items split at the first created at or after id `id`:
    /// (older, newer), each in creation order.
    pub fn split_at_id_mut(&mut self, id: u32) -> (&mut [ItemCore], &mut [ItemCore]) {
        let at = self.items.iter().position(|item| item.id >= id).unwrap_or(self.items.len());
        self.items.split_at_mut(at)
    }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut ItemCore> {
        self.items.iter_mut().find(|i| i.id == id)
    }
    /// A call naming one concrete Item GObj (Ness's `yoyo_gobj`), by its
    /// pool identity, whatever other articles its owner has of the kind.
    pub fn control_instance<D: ItemDispatch>(
        &mut self,
        id: u32,
        control: ItemControl,
        assets: &ItemAssets,
        immediate: bool,
    ) {
        let item = self.get_mut(id).expect("control of a live article instance");
        item.efasync_immediate = immediate;
        (D::logic(item.kind).control)(item, control, assets);
        item.efasync_immediate = false;
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
        // The fighter calls its article by the pointer it keeps, whoever
        // owns the article now (`ItemLogic::tracker`).
        let tracker = D::logic(kind).tracker;
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.kind == kind && tracker(i) == Some((owner, secondary)))
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
            // Item_802694CC's HSD_JObjAnimAll dirties the joints it poses
            // (a state without an animation of its own poses none).
            item.joints_dirty |= row.animation_id >= 0;
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
                // Item_80269528 (item.c:1345, retail 0x80269644..0x8026965C):
                // destroy_type 0, then Item_8026A8EC, whose ItemSwitch plays
                // the kind's destroy effect as an ended animation does.
                item.queue_destroy_effect(assets.destroy_effect, None);
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
        map: &melee_mp::CollMap,
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
        integrate(item, bounds, assets, map);
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
        self.collide_with_stage_parent::<D>(
            id,
            owner,
            stage_contact,
            map,
            bounds,
            assets,
            rng,
            None,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub fn collide_with_stage_parent<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        stage_contact: bool,
        map: &mut melee_mp::CollMap,
        bounds: &ItemBounds,
        assets: &ItemAssets,
        rng: Option<&core::cell::Cell<gekko_math::HsdRng>>,
        stage_parent: Option<crate::StageParentView>,
    ) {
        let half_life_scale = self.common.half_life_scale;
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let alive = !item.destroyed;
        let logic = D::logic(item.kind);
        item.destroyed |= (logic.states[item.motion as usize].collision)(
            item,
            &mut ItemCollisionContext {
                stage_parent,
                owner,
                stage_contact,
                map,
                assets,
                bounds,
                rng,
                half_life_scale,
                entered_air: logic.entered_air,
            },
        );
        if alive && item.destroyed {
            // Item_80269978: destroy_type 1, ItemSwitch's x68 effect at the
            // root JObj, which this proc has not yet moved to `position`.
            let root = item.root_translation;
            item.queue_destroy_effect(assets.event_destroy_effect, Some(root));
        } else {
            // Item_80269978: HSD_JObjSetTranslate(jobj, &pos). The root is
            // dirty: its next setup builds it from `pos`, no longer at the
            // hand the display left it at.
            item.set_root_translation(item.position);
            item.held_display_root = None;
            item.update_stomp_box();
        }
    }
    /// Item_8026A8EC (8026A8EC) outside the item procs: remove `id` now and
    /// run its kind's destroyed callback.
    pub fn destroy<D: ItemDispatch>(&mut self, id: u32, gobjs: &mut hsd_gobj::GObjAddresses) {
        let index = self
            .items
            .iter()
            .position(|item| item.id == id)
            .expect("Item_8026A8EC: Not Found Item_Struct");
        let mut item = self.items.remove(index);
        (D::logic(item.kind).destroyed)(&mut item);
        item.destroy_gobjs(gobjs);
        self.freed_rest_lines.push(item.rest_line);
    }
    /// The items a replay created at another address
    /// (`hsd_gobj::GObjAddresses::replay`).
    pub fn rename_gobjs(&mut self, renamed: &hsd_gobj::Renamed) {
        for item in self.items.iter_mut() {
            item.gobj = renamed.apply(item.gobj);
            for link in item.link_gobjs.iter_mut() {
                *link = renamed.apply(*link);
            }
        }
    }
    /// Notes the items marked destroyed since the last call, in pool
    /// order. Called after each request or proc that can mark one, so the
    /// notes follow retail's Item_8026A8EC calls: Link's take-damage
    /// callback removes the arrow, then the bow (ftLk_800EAF58), whichever
    /// was created first.
    pub fn note_destroyed(&mut self) {
        for item in self.items.iter().filter(|item| item.destroyed) {
            if !self.destroy_order.iter().any(|&id| id == item.id) {
                self.destroy_order.push(item.id);
            }
        }
    }
    /// The next item to remove, in the order the items were marked.
    pub fn next_destroyed(&mut self) -> Option<u32> {
        self.note_destroyed();
        (!self.destroy_order.is_empty()).then(|| self.destroy_order.remove(0))
    }
    /// Every item marked destroyed goes, in the order they were marked.
    pub fn remove_all_destroyed<D: ItemDispatch>(&mut self, gobjs: &mut hsd_gobj::GObjAddresses) {
        while let Some(id) = self.next_destroyed() {
            self.remove_destroyed::<D>(id, gobjs);
        }
    }
    /// Item_8026A8EC's end for an item marked destroyed: its kind's
    /// destroyed callback, then its GObjs are freed.
    pub fn remove_destroyed<D: ItemDispatch>(
        &mut self,
        id: u32,
        gobjs: &mut hsd_gobj::GObjAddresses,
    ) {
        let index = self
            .items
            .iter()
            .position(|item| item.id == id)
            .expect("Item_8026A8EC: Not Found Item_Struct");
        assert!(self.items[index].destroyed, "removing a live item");
        let mut item = self.items.remove(index);
        (D::logic(item.kind).destroyed)(&mut item);
        item.destroy_gobjs(gobjs);
        self.freed_rest_lines.push(item.rest_line);
        if D::logic(item.kind).unlinks_partner_on_destroy {
            if let Some(partner) = item.partner.and_then(|id| self.get_mut(id)) {
                partner.partner = None;
            }
        }
    }
}
/// Item_802697D4: audited retail uses four PSVECAdd calls; never reassociate
/// velocity+nudge, position+delta, environmental movement, platform movement.
fn integrate(
    item: &mut ItemCore,
    bounds: &ItemBounds,
    assets: &ItemAssets,
    map: &melee_mp::CollMap,
) {
    if !item.held && !item.frozen && !item.in_hitlag {
        let mut delta = add(item.velocity, item.nudge);
        // retail 0x8026985C: a grounded item's slide (xDCD b3 is never set
        // for the ported kinds).
        if item.ground_or_air == GroundOrAir::Ground {
            item.slide_step(assets, &mut delta);
        }
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
    // retail 0x8026993C: a grounded item rides its floor line (mpGetSpeed
    // of x378's floor at the item), or with xDCE b3 and that line still
    // live (it_8026D604) the line it sticks to (xC30). An inactive line
    // leaves x64 as it was.
    if item.ground_or_air == GroundOrAir::Ground {
        if let Some(collision) = &item.collision {
            let line = if item.stuck_to_line && map.line_is_active(item.floor_line) {
                item.floor_line
            } else {
                collision.floor.index
            };
            if let Some(speed) = map.line_speed(line, &item.position) {
                item.platform_velocity = speed;
            }
        }
    }
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
            warp_star_path: self.warp_star_path,
            owner_hit_groups: self.owner_hit_groups,
            fighter_push: self.fighter_push,
            freed_rest_lines: hsd_types::storage::clone_vec(&self.freed_rest_lines),
            destroy_order: self.destroy_order.clone(),
            fighter_scale: self.fighter_scale,
            heap_ammo_words: hsd_types::storage::clone_vec(&self.heap_ammo_words),
            heap_ammo_next: self.heap_ammo_next,
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
    pub(super) fn assets() -> ItemAssets {
        ItemAssets {
            visual: Default::default(),
            scripts: Vec::new(),
            hit_flags: Vec::new(),
            special_attributes: Vec::new(),
            redead: None,
            kuriboh: None,
            monster_common: None,
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
            common_lifetime: 0.0,
            half_life_scale: 0.0,
            spin_rate: 0.0,
            fall_spin_degrees: 0.0,
            animation_ends: Vec::new(),
            joint_animated: Vec::new(),
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
            slide_friction: 0.0,
            slide_rule: crate::desc::SlideRule::NEVER,
            landing_spin_degrees: 0.0,
            destroy_effect: None,
            event_destroy_effect: None,
            grab_offset: hsd_types::Vec2::ZERO,
            grab_range: hsd_types::Vec2::ZERO,
            attachment_translation: Some(Vec3::ZERO),
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
        let mut gobjs = hsd_gobj::GObjAddresses::default();
        let mut pool = ItemPool::new(ItemCommonData {
            random_items: Default::default(),
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
            release_floor_depth: 0.0,
            direction_still_speed: 0.0,
            slide_rule: crate::desc::SlideRule::NEVER,
        });
        let spawn = SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0);
        let first = pool
            .spawn::<TestKinds>(spawn, &assets(), &mut gobjs)
            .unwrap();
        let survivor = pool
            .spawn::<TestKinds>(spawn, &assets(), &mut gobjs)
            .unwrap();
        assert!(pool
            .spawn::<TestKinds>(spawn, &assets(), &mut gobjs)
            .is_none());
        pool.retire(first);
        assert_eq!(pool.next_destroyed(), Some(first));
        pool.remove_destroyed::<TestKinds>(first, &mut gobjs);
        assert_eq!(pool.next_destroyed(), None);
        let last = pool
            .spawn::<TestKinds>(spawn, &assets(), &mut gobjs)
            .unwrap();
        assert_eq!(
            pool.iter().map(|item| item.id).collect::<Vec<_>>(),
            [survivor, last]
        );
    }
    #[test]
    fn physics_keeps_retail_rounding_boundary_between_nudge_and_position() {
        let mut gobjs = hsd_gobj::GObjAddresses::default();
        let mut pool = ItemPool::new(ItemCommonData {
            random_items: Default::default(),
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
            release_floor_depth: 0.0,
            direction_still_speed: 0.0,
            slide_rule: crate::desc::SlideRule::NEVER,
        });
        let spawn = SpawnItem::held(
            ItemKind::FoxBlaster,
            0,
            Vec3::new(16777216.0, 0.0, 0.0),
            1.0,
        );
        let id = pool
            .spawn::<TestKinds>(spawn, &assets(), &mut gobjs)
            .unwrap();
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
            &assets(),
            &one_floor(),
        );
        assert_eq!(item.position.x.to_bits(), 0.0f32.to_bits());
    }

    /// A map for callers that need one; an airborne item does not read it.
    fn one_floor() -> melee_mp::CollMap {
        let mut b = melee_mp::CollMapBuilder::new();
        let left = b.vertex(0, -10.0, 0.0);
        let right = b.vertex(0, 10.0, 0.0);
        b.floor(0, left, right, 0);
        melee_mp::CollMap::load(b.build(), 1.0, melee_types::GrKind::Last)
    }
}

#[cfg(test)]
#[path = "item_script_tests.rs"]
mod item_script_tests;

#[cfg(test)]
#[path = "constructor_tests.rs"]
mod constructor_tests;
