//! Mushroom Kingdom II's Birdo (It_Kind_Kyasarin), itkyasarin.c
//! (802ECCDC..802EE1E0), and her egg ([`egg`]).
//!
//! grInishie2_801FD4F0 makes her past a blast zone at a fixed height
//! (it_802ECD3C). She walks in (state 0) to her stop, waits (1), steps back
//! (2), waits (3) and walks up again (4); there she stands (6), now and
//! then laying a volley of eggs (7), until her timer sends her away (5),
//! back the way she came. A light hit flinches her (8); a heavier one sends
//! her away at once, faster; past her endurance she is knocked out, flying
//! at the screen (9) or tumbling off (10).
//!
//! She has no map collision while she walks: her own coordinates bound her
//! walk, and the blast zones remove her once she has turned to leave.
pub mod egg;

use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, MonsterCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, KyasarinState, MonsterBehavior, MonsterState,
};
use melee_types::ItemKind;

pub struct Kyasarin;

/// it_803F90D0's anim_id column.
pub const ARTICLE_STATES: [i32; 11] = [0, 1, 0, 1, 0, 0, 1, 2, 3, 3, 3];
/// itKyasarinAttributes x0..x48: nineteen words.
pub const SPECIAL_ATTRIBUTES: u32 = 19;
/// grInishie2_801FD4CC's [`ItemEvent::StageNotice`]: her platform's joint
/// goes (mpLib_80057BC0(0xE)).
pub const DROP_PLATFORM: u8 = 0;

pub(crate) mod motion {
    /// it_802ECEB0: walking in.
    pub const WALK_IN: u16 = 0;
    /// it_802ECFE0: the first wait.
    pub const FIRST_WAIT: u16 = 1;
    /// it_802ED0D0: stepping back.
    pub const STEP_BACK: u16 = 2;
    /// The second wait (itKyasarin_UnkMotion2_Coll).
    pub const SECOND_WAIT: u16 = 3;
    /// it_802ED25C: walking up again.
    pub const WALK_UP: u16 = 4;
    /// it_802ED8BC: turned round, leaving.
    pub const LEAVE: u16 = 5;
    /// it_802ED4F8: standing, between volleys.
    pub const STAND: u16 = 6;
    /// it_802ED774: laying an egg.
    pub const LAY: u16 = 7;
    /// A light hit's flinch.
    pub const FLINCH: u16 = 8;
    /// Knocked out toward the screen.
    pub const SCREEN: u16 = 9;
    /// Knocked out, tumbling.
    pub const TUMBLE: u16 = 10;
}

/// it_804D6D40-style literals: HSD_JObjSetRotationY's half and whole turns
/// (M_PI and 2 * M_PI as doubles, rounded once), and the knockout's
/// HSD_JObjSetRotationX (@572, pi as a float).
const HALF_TURN: f64 = core::f64::consts::PI;
const WHOLE_TURN: f64 = 2.0 * core::f64::consts::PI;
const UPSIDE_DOWN: f32 = core::f32::consts::PI;

/// itKyasarinAttributes.
pub(crate) struct Attributes<'a>(pub(crate) &'a [f32]);
impl Attributes<'_> {
    fn int(&self, index: usize) -> i32 {
        self.0[index].to_bits() as i32
    }
    /// x4: the walking speed.
    pub(crate) fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8 / xC: the first and second waits.
    fn first_wait(&self) -> f32 {
        self.0[2]
    }
    fn second_wait(&self) -> f32 {
        self.0[3]
    }
    /// x10 / x14: the stand before she leaves, `x10 + rand(x14)`.
    fn leave_base(&self) -> f32 {
        self.0[4]
    }
    fn leave_range(&self) -> f32 {
        self.0[5]
    }
    /// x18 / x1C: the wait before a volley, `x18 + rand(x1C)`.
    fn volley_base(&self) -> f32 {
        self.0[6]
    }
    fn volley_range(&self) -> f32 {
        self.0[7]
    }
    /// x24: the animation rate once a heavy hit sends her away.
    fn hurried_rate(&self) -> f32 {
        self.0[9]
    }
    /// x28 / x2C: where she stops walking in, from the right and the left.
    fn stop_from_right(&self) -> f32 {
        self.0[10]
    }
    fn stop_from_left(&self) -> f32 {
        self.0[11]
    }
    /// x30 / x34: where her step back ends, from the right and the left.
    fn back_from_right(&self) -> f32 {
        self.0[12]
    }
    fn back_from_left(&self) -> f32 {
        self.0[13]
    }
    /// x38 / x3C: the egg's offset ahead of and above her.
    fn egg_ahead(&self) -> f32 {
        self.0[14]
    }
    fn egg_above(&self) -> f32 {
        self.0[15]
    }
    /// x40 / x44: the damage below which a hit flinches her, and below
    /// which it sends her away.
    fn flinch_limit(&self) -> i32 {
        self.int(16)
    }
    fn endurance(&self) -> i32 {
        self.int(17)
    }
    /// x48: eggs in a full volley.
    fn volley(&self) -> i32 {
        self.int(18)
    }
}

static STATES: [ItemStateRow; 11] = [
    row(0, walk_in_animation, no_physics, walk_in_collision),
    row(1, first_wait_animation, no_physics, no_collision),
    row(2, step_back_animation, no_physics, step_back_collision),
    row(3, second_wait_animation, no_physics, no_collision),
    row(4, walk_up_animation, no_physics, walk_up_collision),
    row(5, leave_animation, no_physics, no_collision),
    row(6, stand_animation, no_physics, no_collision),
    row(7, lay_animation, no_physics, no_collision),
    row(8, flinch_animation, no_physics, no_collision),
    row(9, screen_animation, screen_physics, knocked_out_collision),
    row(10, tumble_animation, no_physics, tumble_collision),
];

const fn row(
    motion: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for Kyasarin {
    const KIND: ItemKind = ItemKind::Kyasarin;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8027B5B0 creates with SpawnItem x44 b0: the creation pass.
    const CONSTRUCTOR_READS_MAP: bool = true;
    /// No pickup callback in her logic row.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802ECE90 -> it_8027B730 (8027B730): the shared zako scratch, not
    /// pushed by fighters or an open palm (it_80275228, it_80275240).
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Monster(MonsterState {
            joint: None,
            joint_index: -1,
            floor_normal: Vec3::new(0.0, 1.0, 0.0),
            turn_frames: 0,
            bounces: 0,
            last_voice: 0,
            behavior: MonsterBehavior::Kyasarin(KyasarinState::default()),
        });
        item.pushed_by_fighters = false;
        item.pushed_by_open_palm = false;
    }
    /// itKyasarin_Logic25_Destroyed (802ECCDC): the stage hears of it
    /// (grInishie2_801FD448) from the scene, which owns her platform.
    fn destroyed(item: &mut ItemCore) {
        state_mut(item).platform = false;
    }
    /// it_802EDDC0 (802EDDC0).
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        take_damage(item, context);
        false
    }
}

pub(crate) fn state(item: &ItemCore) -> &KyasarinState {
    match &item.scratch {
        ItemScratch::Monster(MonsterState {
            behavior: MonsterBehavior::Kyasarin(state),
            ..
        }) => state,
        _ => panic!("Birdo scratch"),
    }
}
pub(crate) fn state_mut(item: &mut ItemCore) -> &mut KyasarinState {
    match &mut item.scratch {
        ItemScratch::Monster(MonsterState {
            behavior: MonsterBehavior::Kyasarin(state),
            ..
        }) => state,
        _ => panic!("Birdo scratch"),
    }
}
fn monster_mut(item: &mut ItemCore) -> &mut MonsterState {
    match &mut item.scratch {
        ItemScratch::Monster(state) => state,
        _ => panic!("Birdo scratch"),
    }
}

/// Whether Birdo still rides the stage's platform (x20).
pub fn has_platform(item: &ItemCore) -> bool {
    state(item).platform
}

/// it_802ECD3C (802ECD3C) after it_8027B5B0: her platform, no damage yet,
/// her facing (turned half round facing right, retail 0x802ECDB4: fmul by
/// the double M_PI, frsp), the blast zones not yet in force (xDCC b3), not
/// grabbable (it_8026B3A8), then walking in (it_802ECEB0).
pub fn start(item: &mut ItemCore, facing: f32, assets: &ItemAssets) {
    let s = state_mut(item);
    s.platform = true;
    s.damage = 0;
    s.entry_facing = facing;
    item.facing = facing;
    if item.facing == 1.0 {
        item.rotation.y = (HALF_TURN * f64::from(item.facing)) as f32;
    }
    item.blast_zone_checked = false;
    item.grabbable = false;
    walk(item, motion::WALK_IN, 1.0, assets);
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// it_802ECEB0 / it_802ED0D0 / it_802ED25C: walk at x4 along `sign` times
/// her facing (retail 0x802ECED8: fmuls; 0x802ED0F8 fneg first).
fn walk(item: &mut ItemCore, motion: u16, sign: f32, assets: &ItemAssets) {
    let speed = Attributes(&assets.special_attributes).speed();
    let speed = if sign < 0.0 { -speed } else { speed };
    item.velocity = Vec3::new(speed * item.facing, 0.0, 0.0);
    change(item, motion, assets);
}

/// it_80272C6C != 0: the article animation still runs.
fn running(item: &ItemCore, assets: &ItemAssets) -> bool {
    !item.article_animation_ended(assets)
}

fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}
fn no_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itKyasarin_UnkMotion0_Anim (802ECF00): the walk loops.
fn walk_in_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        walk(item, motion::WALK_IN, 1.0, ctx.assets);
    }
    false
}
/// itKyasarin_UnkMotion2_Anim (802ED124).
fn step_back_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        walk(item, motion::STEP_BACK, -1.0, ctx.assets);
    }
    false
}
/// itKyasarin_UnkMotion4_Anim (802ED2AC).
fn walk_up_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        walk(item, motion::WALK_UP, 1.0, ctx.assets);
    }
    false
}

/// Past her stop going her way (fcmpo; cror: `>=` facing right, `<=`
/// facing left).
fn reached_stop(item: &ItemCore, assets: &ItemAssets) -> bool {
    let a = Attributes(&assets.special_attributes);
    if item.facing == 1.0 {
        item.position.x >= a.stop_from_left()
    } else {
        item.position.x <= a.stop_from_right()
    }
}

/// itKyasarin_UnkMotion0_Coll (802ECF70): at her stop she waits
/// (it_802ECFE0: state 1, still, the first wait).
fn walk_in_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if reached_stop(item, ctx.assets) {
        change(item, motion::FIRST_WAIT, ctx.assets);
        item.velocity = Vec3::ZERO;
        state_mut(item).wait = Attributes(&ctx.assets.special_attributes).first_wait();
    }
    false
}

/// itKyasarin_UnkMotion1_Anim (802ED040): the wait counts down (retail
/// 0x802ED058: fsubs 1.0), then she steps back.
fn first_wait_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let s = state_mut(item);
    s.wait -= 1.0;
    if s.wait < 0.0 {
        walk(item, motion::STEP_BACK, -1.0, ctx.assets);
    }
    false
}

/// itKyasarin_UnkMotion2_Coll (802ED198): her step back ends at x34
/// (facing right, `<=`) or x30 (facing left, `>=`): the second wait.
fn step_back_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let a = Attributes(&ctx.assets.special_attributes);
    let done = if item.facing == 1.0 {
        item.position.x <= a.back_from_left()
    } else {
        item.position.x >= a.back_from_right()
    };
    if done {
        second_wait(item, ctx.assets);
    }
    false
}

/// Item_80268E5C(3), it_80273454, x24 = xC.
fn second_wait(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::SECOND_WAIT, assets);
    item.velocity = Vec3::ZERO;
    state_mut(item).wait = Attributes(&assets.special_attributes).second_wait();
}

/// itKyasarin_UnkMotion3_Anim (802ED088): the second wait, then she walks
/// up again.
fn second_wait_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let s = state_mut(item);
    s.wait -= 1.0;
    if s.wait < 0.0 {
        walk(item, motion::WALK_UP, 1.0, ctx.assets);
    }
    false
}

/// `base + (float) HSD_Randi((int) range)` (retail 0x802ED36C..0x802ED3A0:
/// fctiwz, the int back to float through the 0x43300000 double, fadds).
fn timer(base: f32, range: f32, rng: &mut HsdRng) -> f32 {
    let drawn = rng.randi(gekko_math::msl::fctiwz(range));
    base + drawn as f32
}

/// itKyasarin_UnkMotion4_Coll (802ED31C): back at her stop she draws her
/// stay (x2C) and her first volley's wait (x28), stops, draws whether the
/// volley is full (x48 eggs) or empty, and stands (it_802ED4F8).
fn walk_up_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !reached_stop(item, ctx.assets) {
        return false;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    let cell = ctx.rng.expect("itKyasarin_UnkMotion4_Coll draws");
    let mut rng = cell.get();
    let turn = timer(a.leave_base(), a.leave_range(), &mut rng);
    let egg = timer(a.volley_base(), a.volley_range(), &mut rng);
    let full = rng.randi(2) != 0;
    cell.set(rng);
    let s = state_mut(item);
    s.turn_timer = turn;
    s.egg_timer = egg;
    item.velocity = Vec3::ZERO;
    state_mut(item).eggs = if full { a.volley() } else { 0 };
    change(item, motion::STAND, ctx.assets);
    false
}

/// itKyasarin_TurnAround: state 5, the model turned (retail 0x802ED5A0 /
/// 0x802ED640: the facing she came with times M_PI, or 2 * M_PI, in
/// double, frsp), facing back, the blast zones in force, walking at the
/// animation's rate (0x802ED6FC / 0x802ED700: fmuls, fmuls).
fn turn_around(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::LEAVE, assets);
    let entry = state(item).entry_facing;
    item.rotation.y = if entry == -1.0 {
        (HALF_TURN * f64::from(entry)) as f32
    } else {
        (WHOLE_TURN * f64::from(entry)) as f32
    };
    item.joints_dirty = true;
    item.facing = -entry;
    item.blast_zone_checked = true;
    let speed = Attributes(&assets.special_attributes).speed();
    item.velocity = Vec3::new(item.animation_rate * (speed * item.facing), 0.0, 0.0);
}

/// itKyasarin_UnkMotion6_Anim (802ED520): both timers count down (retail
/// 0x802ED54C, 0x802ED558: fsubs 1.0); her stay's end turns her round,
/// her volley's starts it, otherwise the stand loops.
fn stand_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let s = state_mut(item);
    s.turn_timer -= 1.0;
    s.egg_timer -= 1.0;
    if s.turn_timer < 0.0 {
        turn_around(item, ctx.assets);
    } else if s.egg_timer < 0.0 {
        change(item, motion::LAY, ctx.assets);
    } else if !running(item, ctx.assets) {
        change(item, motion::STAND, ctx.assets);
    }
    false
}

/// itKyasarin_UnkMotion7_Anim (802ED79C): once the animation ends, an egg
/// ahead of her mouth (retail 0x802ED800: fmadds x38 * facing + x; 0x802ED810:
/// fadds x3C) on the stage plane (it_802EFA44); a spent volley draws the
/// next wait and stands, otherwise she lays again.
fn lay_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if running(item, ctx.assets) {
        return false;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    let position = Vec3::new(
        fmadds(a.egg_ahead(), item.facing, item.position.x),
        item.position.y + a.egg_above(),
        0.0,
    );
    item.events.push(ItemEvent::SpawnZako {
        kind: ItemKind::KyasarinEgg,
        position,
        facing: item.facing,
    });
    let s = state_mut(item);
    s.eggs -= 1;
    if s.eggs <= 0 {
        let cell = ctx.rng.expect("itKyasarin_UnkMotion7_Anim draws");
        let mut rng = cell.get();
        let wait = timer(a.volley_base(), a.volley_range(), &mut rng);
        cell.set(rng);
        state_mut(item).egg_timer = wait;
        change(item, motion::STAND, ctx.assets);
    } else {
        change(item, motion::LAY, ctx.assets);
    }
    false
}

/// itKyasarin_UnkMotion5_Anim (802ED8E4): the walk away loops.
fn leave_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        change(item, motion::LEAVE, ctx.assets);
    }
    false
}

/// itKyasarin_UnkMotion8_Anim (802ED92C): the flinch over, she resumes
/// the state it interrupted afresh, and takes hits again (it_802756E0).
fn flinch_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if running(item, ctx.assets) {
        return false;
    }
    let assets = ctx.assets;
    match state(item).resume_motion {
        motion::WALK_IN => walk(item, motion::WALK_IN, 1.0, assets),
        motion::FIRST_WAIT => {
            change(item, motion::FIRST_WAIT, assets);
            item.velocity = Vec3::ZERO;
            state_mut(item).wait = Attributes(&assets.special_attributes).first_wait();
        }
        motion::STEP_BACK => walk(item, motion::STEP_BACK, -1.0, assets),
        motion::SECOND_WAIT => second_wait(item, assets),
        motion::WALK_UP => walk(item, motion::WALK_UP, 1.0, assets),
        motion::LEAVE => turn_around(item, assets),
        motion::STAND => change(item, motion::STAND, assets),
        motion::LAY => change(item, motion::LAY, assets),
        motion::FLINCH => flinch(item, assets),
        // itkyasarin.c:390: the switch covers states 0..8 only.
        _ => {}
    }
    item.hurt_intangible = false;
    false
}

/// The flinch (itKyasarin_SaveStateAndStop): the state it interrupts kept
/// (x3C), state 8, still (it_80273454), no hits taken (it_802756D0).
fn flinch(item: &mut ItemCore, assets: &ItemAssets) {
    state_mut(item).resume_motion = item.motion;
    change(item, motion::FLINCH, assets);
    item.velocity = Vec3::ZERO;
    item.hurt_intangible = true;
}

/// itKyasarin_UnkMotion9_Anim / 10_Anim (802EDCD0 / 802EDD54): the
/// knockout's animation loops.
fn screen_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        change(item, motion::SCREEN, ctx.assets);
    }
    false
}
fn tumble_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        change(item, motion::TUMBLE, ctx.assets);
    }
    false
}

/// itKyasarin_UnkMotion9_Phys (802EDD18): ItemAttr x10's gravity,
/// unlimited (retail 0x802EDD28: fsubs).
fn screen_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y -= ctx.assets.fall_acceleration;
}

/// itKyasarin_UnkMotion9_Coll -> it_8027C794: nothing.
fn knocked_out_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itKyasarin_UnkMotion10_Coll -> it_8027C79C (8027C79C): every surface
/// she meets (it_8026DAA8) would cry out (it_8027C9D8) and bounce her on
/// (it_8027B964(gobj, 1)), as the ReDead's tumble does. No recording yet
/// has a tumbling Birdo meet a surface: she leaves the stage up and out.
fn tumble_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) & 0xF == 0 {
        return false;
    }
    unimplemented!("it_8027C79C (itzako.c:462): a tumbling Birdo meeting the map is not witnessed")
}

fn monster_common(assets: &ItemAssets) -> MonsterCommonData {
    assets
        .monster_common
        .expect("it_804D6D40 for Birdo")
}

/// it_802EDDC0 (802EDDC0): the hit's damage (xCA0) adds to hers (x38).
/// Below x40 she flinches; below x44 she hurries away (her animation at
/// x24, then itKyasarin_TurnAround); otherwise she is knocked out.
fn take_damage(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    let a = Attributes(&context.assets.special_attributes);
    let damage = state(item).damage.wrapping_add(item.pending_damage_taken);
    state_mut(item).damage = damage;
    if damage < a.flinch_limit() {
        flinch(item, context.assets);
    } else if damage < a.endurance() {
        item.animation_rate = a.hurried_rate();
        turn_around(item, context.assets);
    } else {
        knock_out(item, context);
    }
}

/// it_802EDDC0's knockout (itkyasarin.c:496-520): her platform drops
/// (grInishie2_801FD4CC), the blast zones are in force; one time in
/// it_804D6D40 +8 she flies at the screen (it_8027BA54, state 9),
/// otherwise she tumbles (it_8027B964(gobj, 0), state 10). Either way she
/// turns upside down (@572), leaves the floor (it_802762BC), her map
/// collision restarts where she is (it_80276100) and she takes no more
/// hits (it_802756D0).
fn knock_out(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    if state(item).platform {
        item.events.push(ItemEvent::StageNotice(DROP_PLATFORM));
    }
    item.blast_zone_checked = true;
    let common = monster_common(context.assets);
    let cell = context.rng.expect("it_802EDDC0 draws");
    let mut rng = cell.get();
    let screen = rng.randf() < common.screen_chance;
    cell.set(rng);
    flip_and_fall(item);
    if screen {
        item.velocity = screen_velocity(item, context);
        change(item, motion::SCREEN, context.assets);
    } else {
        launch(item, &common, &context.launch);
        monster_mut(item).bounces = 0;
        change(item, motion::TUMBLE, context.assets);
    }
}

/// itKyasarin_FlipAndFall.
fn flip_and_fall(item: &mut ItemCore) {
    item.rotation.x = UPSIDE_DOWN;
    item.joints_dirty = true;
    item.enter_air();
    item.events.push(ItemEvent::ResetCollision);
    item.hurt_intangible = true;
}

/// it_8027BA54 (8027BA54): toward the camera's eye, jittered by up to 5
/// sideways (HSD_Randf in double: retail 0x8027BA84..90 fsub, fmadd,
/// frsp) and up to 30 up (fmadds), at 10.
fn screen_velocity(item: &ItemCore, context: &ItemEventContext<'_>) -> Vec3 {
    let mut target = context
        .camera_position
        .expect("it_8027BA54 reads the camera");
    let cell = context.rng.expect("it_8027BA54 draws");
    let mut rng = cell.get();
    target.x = gekko_math::fma::fmadd(10.0, f64::from(rng.randf()) - 0.5, f64::from(target.x)) as f32;
    target.y = fmadds(30.0, rng.randf(), target.y);
    cell.set(rng);
    let toward = melee_lb::vector::normalize(melee_lb::vector::difference(target, item.position));
    Vec3::new(toward.x * 10.0, toward.y * 10.0, toward.z * 10.0)
}

/// it_8027B964(gobj, 0) (8027B964): along the hit's knockback
/// (it_8027B798); a still one goes up and away from the hitter at 0.1.
fn launch(item: &mut ItemCore, common: &MonsterCommonData, launch: &melee_it::hurt::ItemLaunch) {
    let (velocity, _) = item.knockback_launch(launch);
    item.velocity = velocity;
    if item.velocity.x == 0.0 && item.velocity.y == 0.0 {
        item.velocity.x = 0.1 * -item.hit_direction;
        item.velocity.y = 0.1;
    }
    if item.velocity.y == 0.0 {
        item.velocity.y = 0.1;
    }
    scale_launch(item, common);
}

/// it_8027B964's tail: lbVector_Normalize, then fmuls by it_804D6D40 +4.
fn scale_launch(item: &mut ItemCore, common: &MonsterCommonData) {
    let v = melee_lb::vector::normalize(item.velocity);
    let speed = common.launch_speed;
    item.velocity = Vec3::new(v.x * speed, v.y * speed, v.z * speed);
}
