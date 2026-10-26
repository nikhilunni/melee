//! Great Bay's Tingle (It_Kind_Tincle), ittincle.c (802EB5C8..802ECCA4).
//!
//! grGreatBay_801F5460 makes him over the stage (it_802ECA70), hidden. He
//! waits (state 0), floats down on his balloon (1), and bobs there (2),
//! drifting left and right now and then. Those standing on him sink him
//! (3, 4; the stage feeds their weight, it_802ECC98) and he rises back (5).
//! A hit pops the balloon (6) and, his own hit dealt, knocks him off it
//! (7); he falls (8), lands (9), lies there a while (10), gets up (11) and
//! floats back up to his height on a new balloon (12).
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, MonsterBehavior, MonsterState, TincleState,
};
use melee_types::ItemKind;

pub struct Tincle;

/// it_803F9000's anim_id column.
pub const ARTICLE_STATES: [i32; 13] = [-1, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 6];
/// itTincleAttributes x0..x55: twenty-two words.
pub const SPECIAL_ATTRIBUTES: u32 = 22;
/// it_802ECA70's spawn point.
pub const SPAWN: Vec3 = Vec3::new(0.0, 200.0, 0.0);
/// The bone at the top of his balloon (xBBC_dynamicBoneTable->bones[11]).
const BALLOON_BONE: usize = 11;
/// it_802EB6DC: he floats on the stage plane's near side (@202).
const DEPTH: f32 = 2.0;
/// it_802EC35C: knocked off, he falls behind the stage (@380).
const FALL_DEPTH: f32 = -10.0;
/// it_802EC69C: a landing this close below his height rises 20 more (@440).
const RISE_EXTRA: f32 = 20.0;
/// it_802EBD14: a sink's first frames and its easing (@176).
const SINK_FRAMES: i32 = 0x13;
const SINK_EASE: f32 = 10.0;
/// it_802EC18C: the pop's frames before it may knock him off.
const POP_FRAMES: i32 = 3;
/// Item_80268E5C's ITEM_UNK_0x1, alone: the joint animation plays on.
const UNK_0X1: u32 = 1;

pub mod motion {
    pub const WAIT: u16 = 0;
    pub const DESCEND: u16 = 1;
    pub const BOB: u16 = 2;
    pub const SINK: u16 = 3;
    pub const HOLD: u16 = 4;
    pub const RISE: u16 = 5;
    pub const POPPED: u16 = 6;
    pub const KNOCKED_OFF: u16 = 7;
    pub const FALL: u16 = 8;
    pub const LAND: u16 = 9;
    pub const LIE: u16 = 10;
    pub const GET_UP: u16 = 11;
    pub const FLOAT_UP: u16 = 12;
}

/// itTincleAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn int(&self, index: usize) -> i32 {
        self.0[index].to_bits() as i32
    }
    /// x4 / x8: the first wait, `rand(|x4 - x8|)`.
    fn wait_range(&self) -> i32 {
        let range = self.int(1).wrapping_sub(self.int(2));
        if range < 0 {
            -range
        } else {
            range
        }
    }
    /// xC / x10: his horizontal range.
    fn left(&self) -> f32 {
        self.0[3]
    }
    fn right(&self) -> f32 {
        self.0[4]
    }
    /// x14: where he waits, high above.
    fn start_y(&self) -> f32 {
        self.0[5]
    }
    /// x18: his speed coming down.
    fn descent(&self) -> f32 {
        self.0[6]
    }
    /// x1C / x20: the height he floats at, `x20 + rand(|x1C - x20|)`.
    fn float_low(&self) -> f32 {
        self.0[7]
    }
    fn float_high(&self) -> f32 {
        self.0[8]
    }
    /// x24: the bob's height.
    fn bob_height(&self) -> f32 {
        self.0[9]
    }
    /// x28: the bob's half period.
    fn bob_frames(&self) -> i32 {
        self.int(10)
    }
    /// x2C: frames between drift draws.
    fn drift_wait(&self) -> i32 {
        self.int(11)
    }
    /// x30: one drift in this many draws.
    fn drift_odds(&self) -> i32 {
        self.int(12)
    }
    /// x34 / x38: the drift's slowest and fastest speeds.
    fn drift_low(&self) -> f32 {
        self.0[13]
    }
    fn drift_high(&self) -> f32 {
        self.0[14]
    }
    /// x3C: a drift's length.
    fn drift_frames(&self) -> i32 {
        self.int(15)
    }
    /// x40: frames he holds sunk.
    fn hold_frames(&self) -> i32 {
        self.int(16)
    }
    /// x44: how far he may sink.
    fn sink_limit(&self) -> f32 {
        self.0[17]
    }
    /// x48: his sinking speed.
    fn sink_speed(&self) -> f32 {
        self.0[18]
    }
    /// x4C: his rising speed.
    fn rise_speed(&self) -> f32 {
        self.0[19]
    }
    /// x50: his speed floating up after a fall.
    fn float_up_speed(&self) -> f32 {
        self.0[20]
    }
    /// x54 / x55 (s8): the lying down's repeats, `x54 + rand(|x55 - x54|)`.
    fn lie_repeats(&self) -> [i32; 2] {
        let word = self.0[21].to_bits();
        [i32::from((word >> 24) as u8 as i8), i32::from((word >> 16) as u8 as i8)]
    }
}

const fn row(
    index: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[index],
        animation,
        physics,
        collision,
    }
}

static STATES: [ItemStateRow; 13] = [
    row(0, no_animation, wait_physics, wait_collision),
    row(1, no_animation, descend_physics, descend_collision),
    row(2, bob_animation, bob_physics, bob_collision),
    row(3, no_animation, sink_physics, sink_collision),
    row(4, no_animation, hold_physics, sink_collision),
    row(5, no_animation, rise_physics, contact_collision),
    row(6, popped_animation, no_physics, popped_collision),
    row(7, popped_animation, no_physics, popped_collision),
    row(8, no_animation, fall_physics, fall_collision),
    row(9, land_animation, no_physics, grounded_collision),
    row(10, lie_animation, no_physics, grounded_collision),
    row(11, get_up_animation, no_physics, grounded_collision),
    row(12, no_animation, rise_physics, contact_collision),
];

impl ItemLogic for Tincle {
    const KIND: ItemKind = ItemKind::Tincle;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8027B5B0 creates with SpawnItem x44 b0: the creation pass.
    const CONSTRUCTOR_READS_MAP: bool = true;
    /// No pickup callback in his logic row.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802EB5C8 (802EB5C8): the zako scratch (it_8027B730), no slide
    /// (xD5C), the blast zones off (xDCC b3), not grabbable (xDC8 x15),
    /// sparing kindred items (it_8027542C), his balloon's height above him
    /// (bone 11's world y, retail 0x802EB630: fsubs, then its magnitude),
    /// his collision box kept (x68), then it_802EB6DC.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.scratch = ItemScratch::Monster(MonsterState {
            joint: None,
            joint_index: -1,
            floor_normal: Vec3::new(0.0, 1.0, 0.0),
            turn_frames: 0,
            bounces: 0,
            last_voice: 0,
            behavior: MonsterBehavior::Tincle(TincleState::default()),
        });
        item.pushed_by_fighters = false;
        item.pushed_by_open_palm = false;
        // xD5C = 0: he never slides on a slope.
        item.slide_mode = 0;
        item.blast_zone_checked = false;
        item.grabbable = false;
        item.strikes_kindred_items = false;
        let pose = assets.pose.as_ref().expect("Tingle's pose");
        let balloon = pose.rest_bone_matrix(BALLOON_BONE, item.root_srt(), None).0[1][3];
        let height = balloon - item.position.y;
        let collision_box = item.collision_box;
        let s = state_mut(item);
        s.balloon_height = if height < 0.0 { -height } else { height };
        s.collision_box = collision_box;
        // it_802EB6DC draws; the scene runs it with the stream
        // ([`reset`]) once the spawn returns.
    }
    /// itTincle_Logic13_DmgReceived (802EB684): any hit pops the balloon.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        pop(item, context.assets);
        false
    }
    /// itTincle_Logic13_DmgDealt (802EB6A8): his hit knocks him off the
    /// balloon, unless it already has.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion != motion::KNOCKED_OFF {
            knock_off(item, context.assets);
        }
        false
    }
}

fn state(item: &ItemCore) -> &TincleState {
    match &item.scratch {
        ItemScratch::Monster(MonsterState {
            behavior: MonsterBehavior::Tincle(state),
            ..
        }) => state,
        _ => panic!("Tingle scratch"),
    }
}
fn state_mut(item: &mut ItemCore) -> &mut TincleState {
    match &mut item.scratch {
        ItemScratch::Monster(MonsterState {
            behavior: MonsterBehavior::Tincle(state),
            ..
        }) => state,
        _ => panic!("Tingle scratch"),
    }
}

/// `(f32) HSD_Randi(fctiwz(range))`, through the 0x43300000 double.
fn draw(range: f32, rng: &mut gekko_math::HsdRng) -> f32 {
    rng.randi(gekko_math::msl::fctiwz(range)) as f32
}

fn magnitude(x: f32) -> f32 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

/// it_802EB6DC (802EB6DC): a place in his range (retail 0x802EB714:
/// fsubs, magnitude, fctiwz; 0x802EB758: fadds), high above, airborne and
/// still, his first wait drawn, the bob and drift cleared, taking no hits,
/// hidden, state 0 with his box (it_802EC9E8).
pub fn reset(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
    let (x, wait) = reset_draws(assets, rng);
    reset_with(item, assets, x, wait);
}

/// it_802EB6DC's two draws, in order: his place in his range, his wait.
pub fn reset_draws(assets: &ItemAssets, rng: &mut gekko_math::HsdRng) -> (i32, i32) {
    let a = Attributes(&assets.special_attributes);
    let x = rng.randi(gekko_math::msl::fctiwz(magnitude(a.right() - a.left())));
    let wait = rng.randi(a.wait_range());
    (x, wait)
}

/// it_802EB6DC with its draws taken.
pub fn reset_with(item: &mut ItemCore, assets: &ItemAssets, x_draw: i32, wait: i32) {
    let a = Attributes(&assets.special_attributes);
    let x = a.left() + x_draw as f32;
    item.position = Vec3::new(x, a.start_y(), DEPTH);
    item.enter_air();
    item.velocity = Vec3::ZERO;
    let s = state_mut(item);
    s.timer = wait;
    s.drift_frames = 0;
    s.drift_count = 0;
    s.half_cycle = 0;
    s.drift_acceleration = 0.0;
    s.acceleration = 0.0;
    s.sink_start = 0.0;
    item.hurt_intangible = true;
    item.hidden = true;
    item.change_motion_with(motion::WAIT, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    apply_box(item);
}

/// it_802ECA70 (802ECA70) after it_8027B5B0: the map's scale on him and
/// his model (Ground_801C0498), no facing, his platform's joint skipped by
/// his collision (x3C0); it_802EB6DC follows ([`reset`]).
pub fn start(item: &mut ItemCore, map_scale: f32) {
    item.scale = map_scale;
    item.model_scale = Vec3::new(map_scale, map_scale, map_scale);
    item.joints_dirty = true;
    item.facing = 0.0;
    item.collision.as_mut().expect("Tingle's collision").joint_id_skip =
        TINGLE_PLATFORM_JOINT;
}

/// grGb_803E3E60's joint 5: his platform, which his own collision skips.
const TINGLE_PLATFORM_JOINT: i32 = 5;

/// it_802EC9E8 (802EC9E8): his box (x68), its top and sides halved while
/// he is off the balloon (states 6..11; 0x802ECA38..50 fmuls 0.5), in use
/// (it_80275D5C).
fn apply_box(item: &mut ItemCore) {
    let mut b = state(item).collision_box;
    if (motion::POPPED..=motion::GET_UP).contains(&item.motion) {
        b.top *= 0.5;
        b.right *= 0.5;
        b.left *= 0.5;
    }
    item.set_collision_box(b);
}

fn no_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}

/// it_80272C6C != 0: the article animation still runs.
fn running(item: &ItemCore, assets: &ItemAssets) -> bool {
    !item.article_animation_ended(assets)
}

/// itTincle_UnkMotion0_Phys (802EB810): the wait, then down.
fn wait_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if state(item).timer <= 0 {
        let mut rng = ctx.rng.get();
        descend(item, ctx.assets, &mut rng);
        ctx.rng.set(rng);
        return;
    }
    state_mut(item).timer -= 1;
}

/// itTincle_UnkMotion0_Coll -> it_8026DA08.
fn wait_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_pass(ctx.map);
    false
}

/// it_802EB870 (802EB870): the middle of his range (retail 0x802EB8A4:
/// fsubs, magnitude, fmuls 0.5, fctiwz; 0x802EB8E4 fadds) and his height
/// drawn (0x802EB8F8: fsubs, magnitude, fctiwz; 0x802EB93C fadds), coming
/// down at x18, shown, state 1.
fn descend(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
    let a = Attributes(&assets.special_attributes);
    let half = gekko_math::msl::fctiwz(magnitude(a.right() - a.left()) * 0.5);
    let center = a.left() + half as f32;
    let height = a.float_high() + draw(magnitude(a.float_low() - a.float_high()), rng);
    let s = state_mut(item);
    s.center_x = center;
    s.float_y = height;
    item.velocity.y = -a.descent();
    item.velocity.x = 0.0;
    item.hidden = false;
    item.change_motion_with(motion::DESCEND, ARTICLE_STATES[1], ANIM_UPDATE, assets);
    apply_box(item);
}

/// itTincle_UnkMotion1_Phys (802EB99C): at his height he starts to bob.
fn descend_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.position.y <= state(item).float_y {
        bob(item, ctx.assets);
    }
}

/// itTincle_UnkMotion1_Coll -> it_8026E414(gobj, it_802EBA00).
fn descend_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        bob(item, ctx.assets);
    }
    false
}

/// it_802EBA00 (802EBA00): still, the drift's wait (x2C), the bob's
/// acceleration `2 * x24 / x28^2 * 0.5` (retail 0x802EBA6C..0x802EBA84:
/// fmuls, fmuls, fdivs, fmuls), a full cycle to go, taking hits, pushable
/// (xDC8 x1A), shown, state 2 with the animation playing on.
fn bob(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.velocity = Vec3::ZERO;
    let frames = a.bob_frames() as f32;
    let acceleration = (2.0 * a.bob_height()) / (frames * frames);
    let s = state_mut(item);
    s.timer = a.drift_wait();
    s.acceleration = acceleration * 0.5;
    s.half_cycle = a.bob_frames() * 2;
    s.bob_phase = 2.0;
    s.drift_frames = 0;
    item.hurt_intangible = false;
    item.pushable = true;
    item.hidden = false;
    item.change_motion_keeping_animation(motion::BOB, UNK_0X1, assets);
    apply_box(item);
}

/// itTincle_UnkMotion2_Anim (802EBAF0): the bob's animation loops.
fn bob_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        item.change_motion_with(motion::BOB, ARTICLE_STATES[2], ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itTincle_UnkMotion2_Phys (802EBB38): the bob (retail 0x802EBBC8 /
/// 0x802EBBDC fadds / fsubs), then the drift's wait, a drift drawn (one in
/// x30) or eased (0x802EBC90 fmuls -1, 0x802EBCA0 fadds).
fn bob_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let s = state(item);
    if s.bob_phase == 0.0 {
        let s = state_mut(item);
        s.bob_phase = 2.0;
        s.half_cycle = a.bob_frames() * 2;
        s.acceleration *= -1.0;
    } else if s.half_cycle == 0 {
        state_mut(item).bob_phase = s.bob_phase - 1.0;
    } else {
        if s.half_cycle < a.bob_frames() {
            item.velocity.y += s.acceleration;
        } else {
            item.velocity.y -= s.acceleration;
        }
        state_mut(item).half_cycle -= 1;
    }
    let s = state_mut(item);
    if s.timer != 0 {
        s.timer -= 1;
        return;
    }
    if s.drift_frames == 0 {
        let mut rng = ctx.rng.get();
        let start = rng.randi(a.drift_odds()) == 0;
        if start {
            start_drift(item, &a, &mut rng);
            ctx.rng.set(rng);
            return;
        }
        ctx.rng.set(rng);
        item.velocity.x = 0.0;
        state_mut(item).timer = a.drift_wait();
        return;
    }
    if s.drift_count >= s.drift_frames {
        item.velocity.x = 0.0;
        let s = state_mut(item);
        s.drift_acceleration = 0.0;
        s.drift_count = 0;
        s.drift_frames = 0;
        s.timer = a.drift_wait();
        return;
    }
    if s.drift_count == s.drift_frames / 2 {
        s.drift_acceleration *= -1.0;
    }
    let acceleration = s.drift_acceleration;
    item.velocity.x += acceleration;
    state_mut(item).drift_count += 1;
}

/// it_802EC870(gobj, 0) (802EC870): a drift toward the middle (or away one
/// time in three, never past his range), at a random fraction of the
/// speed span (retail 0x802EC944: fsubs, magnitude; 0x802EC958 fmuls),
/// easing to the fastest speed over half its length (0x802EC98C fsubs,
/// 0x802EC9A0 fdivs by the halved length), both along the facing.
fn start_drift(item: &mut ItemCore, a: &Attributes<'_>, rng: &mut gekko_math::HsdRng) {
    let center = state(item).center_x;
    item.facing = if item.position.x < center { 1.0 } else { -1.0 };
    if rng.randi(3) == 0 {
        item.facing *= -1.0;
    }
    if item.position.x < a.left() {
        item.facing = 1.0;
    }
    if item.position.x > a.right() {
        item.facing = -1.0;
    }
    let span = magnitude(a.drift_high() - a.drift_low());
    item.velocity.x = span * rng.randf();
    let frames = a.drift_frames();
    let acceleration = (a.drift_high() - item.velocity.x) / (frames / 2) as f32;
    let s = state_mut(item);
    s.drift_frames = frames;
    s.drift_acceleration = acceleration;
    item.velocity.x *= item.facing;
    let facing = item.facing;
    let s = state_mut(item);
    s.drift_acceleration *= facing;
    s.drift_count = 0;
}

/// itTincle_UnkMotion2_Coll (802EBCD0): any surface (it_8026DAA8) turns
/// the drift back (it_802EC870(gobj, 1): facing, speed and acceleration
/// all reversed).
fn bob_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) != 0 {
        item.facing = -item.facing;
        item.velocity.x *= -1.0;
        state_mut(item).drift_acceleration *= -1.0;
    }
    false
}

/// it_802EBD14 (802EBD14), from the stage when someone stands on him: the
/// bob kept (x30, x44, x48, x40), still but sinking at their weight (x54),
/// easing by a tenth of it (@176) for nineteen frames from where he was.
pub fn sink(item: &mut ItemCore, assets: &ItemAssets) {
    let motion = item.motion;
    let velocity_y = item.velocity.y;
    let s = state_mut(item);
    s.saved_motion = f32::from(motion);
    s.saved_velocity_y = velocity_y;
    s.saved_acceleration = s.acceleration;
    s.saved_half_cycle = s.half_cycle as f32;
    let weight = s.weight;
    s.acceleration = weight / SINK_EASE;
    s.timer = SINK_FRAMES;
    item.velocity.z = 0.0;
    item.velocity.x = 0.0;
    item.velocity.y = -weight;
    let y = item.position.y;
    state_mut(item).sink_start = y;
    item.change_motion_with(motion::SINK, ARTICLE_STATES[3], ANIM_UPDATE, assets);
}

/// itTincle_UnkMotion3_Phys (802EBDD0): the sink eases (retail 0x802EBE0C
/// fadds) for its frames, then he holds at where it started.
fn sink_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let s = state_mut(item);
    if s.timer == 0 {
        item.position.y = s.sink_start;
        item.velocity.y = 0.0;
        hold(item, ctx.assets);
        return;
    }
    let acceleration = s.acceleration;
    s.timer -= 1;
    item.velocity.y += acceleration;
}

/// itTincle_UnkMotion3_Coll / 4_Coll -> it_8026E414(gobj, it_802EC18C): a
/// floor pops the balloon.
fn sink_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        pop(item, ctx.assets);
    }
    false
}

/// it_802EBE5C (802EBE5C): the state kept as a sink keeps it, still, the
/// hold's frames (x40) from here, state 4.
fn hold(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    let motion = item.motion;
    let velocity_y = item.velocity.y;
    let s = state_mut(item);
    s.saved_motion = f32::from(motion);
    s.saved_velocity_y = velocity_y;
    s.saved_acceleration = s.acceleration;
    s.saved_half_cycle = s.half_cycle as f32;
    item.velocity = Vec3::ZERO;
    let y = item.position.y;
    let s = state_mut(item);
    s.timer = a.hold_frames();
    s.sink_start = y;
    item.change_motion_with(motion::HOLD, ARTICLE_STATES[4], ANIM_UPDATE, assets);
}

/// itTincle_UnkMotion4_Phys (802EBF08): after the hold he sinks at x48; a
/// sink past x44 from where it started pops the balloon (retail
/// 0x802EBF44: fsubs, magnitude).
fn hold_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let s = state_mut(item);
    if s.timer != 0 {
        s.timer -= 1;
        return;
    }
    item.velocity.y = -a.sink_speed();
    let distance = magnitude(state(item).sink_start - item.position.y);
    if distance > a.sink_limit() {
        pop(item, ctx.assets);
    }
}

/// it_802EBFAC (802EBFAC), from the stage when no one stands on him any
/// more: far below his height he rises at x4C, decelerating to stop there
/// (retail 0x802EC020..0x802EC03C: fmuls, fneg, fadds, fdivs); otherwise
/// the bob he had resumes (a bob drawing its drift wait afresh), or the
/// rise he had.
pub fn rise(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    let distance = state(item).float_y - item.position.y;
    let speed = a.rise_speed();
    if distance > speed {
        item.velocity = Vec3::new(0.0, speed, 0.0);
        let half_cycle = gekko_math::msl::fctiwz(distance);
        rise_toward(item, half_cycle);
        item.change_motion_keeping_animation(motion::RISE, UNK_0X1, assets);
        return;
    }
    let s = *state(item);
    item.velocity = Vec3::new(0.0, s.saved_velocity_y, 0.0);
    let resume = s.saved_motion == 2.0;
    let st = state_mut(item);
    st.acceleration = s.saved_acceleration;
    st.half_cycle = gekko_math::msl::fctiwz(s.saved_half_cycle);
    if resume {
        st.timer = a.drift_wait();
        st.drift_frames = 0;
        item.change_motion_keeping_animation(motion::BOB, UNK_0X1, assets);
    } else {
        item.change_motion_keeping_animation(motion::RISE, UNK_0X1, assets);
    }
}

/// The rise's deceleration: `-(v * v) / (v + 2 * distance)` with the
/// distance truncated (x2C) and doubled as an int.
fn rise_toward(item: &mut ItemCore, half_cycle: i32) {
    let v = item.velocity.y;
    let s = state_mut(item);
    s.half_cycle = half_cycle;
    s.acceleration = -(v * v) / (v + (half_cycle * 2) as f32);
    s.timer = 0;
}

/// itTincle_UnkMotion5_Phys / 12_Phys (802EC104 / 802EC7A8): the rise
/// decelerates (retail 0x802EC150: fadds) until it stops, then he bobs.
fn rise_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.velocity.y <= 0.0 {
        item.velocity = Vec3::ZERO;
        let s = state_mut(item);
        s.acceleration = 0.0;
        s.half_cycle = 0;
        s.timer = 0;
        bob(item, ctx.assets);
        return;
    }
    let acceleration = state(item).acceleration;
    item.velocity.y += acceleration;
}

/// itTincle_UnkMotion5_Coll / 12_Coll -> it_8026DAA8, its result unused.
fn contact_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map);
    false
}

/// it_802EC18C (802EC18C): still, no hits, three frames, state 6 (the
/// hitboxes kept), his box.
fn pop(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.hurt_intangible = true;
    state_mut(item).timer = POP_FRAMES;
    item.change_motion_with(motion::POPPED, ARTICLE_STATES[6], HIT_PRESERVE | ANIM_UPDATE, assets);
    apply_box(item);
}

/// it_802EC1F4 (802EC1F4): as the pop, straight to state 7.
fn knock_off(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.hurt_intangible = true;
    state_mut(item).timer = 0;
    item.change_motion_with(
        motion::KNOCKED_OFF,
        ARTICLE_STATES[7],
        HIT_PRESERVE | ANIM_UPDATE,
        assets,
    );
    apply_box(item);
}

/// itTincle_UnkMotion7_Anim (802EC25C), states 6 and 7: the pop's frames
/// run out into state 7 (the animation playing on); once it ends he falls.
fn popped_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let timer = state(item).timer;
    if timer == 0 {
        state_mut(item).timer = -1;
        if item.motion == motion::POPPED {
            item.change_motion_keeping_animation(motion::KNOCKED_OFF, UNK_0X1, ctx.assets);
        }
    } else if timer > 0 {
        state_mut(item).timer = timer - 1;
    }
    if !running(item, ctx.assets) {
        fall(item, ctx.assets);
    }
    false
}

/// itTincle_UnkMotion7_Coll (802EC2E8): airborne, it_8026E414 lands him
/// (it_802EC3F4); grounded, the pop over lands him too, then a grounded
/// pass (it_8026D6F4) whose loss of ground drops him (it_802EC35C).
fn popped_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_or_air == melee_types::GroundOrAir::Air {
        if item.airborne_collision(ctx.map, ctx.assets).floor {
            land(item, ctx.assets);
        }
        return false;
    }
    if state(item).timer < 0 {
        land(item, ctx.assets);
    }
    if !item.ground_pass(ctx.map) {
        fall(item, ctx.assets);
    }
    false
}

/// it_802EC35C (802EC35C): he falls behind the stage (@380), state 8.
fn fall(item: &mut ItemCore, assets: &ItemAssets) {
    item.position.z = FALL_DEPTH;
    item.change_motion_with(motion::FALL, ARTICLE_STATES[8], ANIM_UPDATE, assets);
}

/// itTincle_UnkMotion8_Phys -> it_80272860 with ItemAttr x10 / x14.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itTincle_UnkMotion8_Coll -> it_8026E414(gobj, it_802EC3F4).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        land(item, ctx.assets);
    }
    false
}

/// it_802EC3F4 (802EC3F4): still, not pushable, no nudge, state 9, his box.
fn land(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.pushable = false;
    item.nudge = Vec3::ZERO;
    item.change_motion_with(motion::LAND, ARTICLE_STATES[9], ANIM_UPDATE, assets);
    apply_box(item);
}

/// itTincle_UnkMotion9_Anim (802EC460): the landing over, he lies down
/// (it_802EC4D0: `x54 + rand(|x55 - x54|)` repeats, state 10).
fn land_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        let a = Attributes(&ctx.assets.special_attributes);
        let [low, high] = a.lie_repeats();
        let cell = ctx.rng.expect("it_802EC4D0 draws");
        let mut rng = cell.get();
        let span = high - low;
        let repeats = rng.randi(if span < 0 { -span } else { span });
        cell.set(rng);
        state_mut(item).timer = repeats + low;
        item.change_motion_with(motion::LIE, ARTICLE_STATES[10], ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itTincle_UnkMotion10_Anim (802EC55C): the lying animation repeats its
/// count, then he gets up (it_802EC604, state 11).
fn lie_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        let s = state_mut(item);
        s.timer -= 1;
        if s.timer != 0 {
            item.change_motion_with(motion::LIE, ARTICLE_STATES[10], ANIM_UPDATE, ctx.assets);
        } else {
            item.change_motion_with(motion::GET_UP, ARTICLE_STATES[11], ANIM_UPDATE, ctx.assets);
        }
    }
    false
}

/// itTincle_UnkMotion11_Anim (802EC62C): up, he floats back up.
fn get_up_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !running(item, ctx.assets) {
        float_up(item, ctx.assets);
    }
    false
}

/// itTincle_UnkMotion9/10/11_Coll -> it_8026D62C(gobj, it_802EC35C): a
/// grounded pass; off the ground he falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        fall(item, ctx.assets);
    }
    false
}

/// it_802EC69C (802EC69C): back on the stage plane; close below his height
/// he aims 20 higher (@440; retail 0x802EC6F4 / 0x802EC6F8 fadds); up at
/// x50, decelerating to stop there (as it_802EBFAC), airborne, taking
/// hits, state 12 with the animation playing on.
fn float_up(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.position.z = 0.0;
    let y = item.position.y;
    let s = state_mut(item);
    let mut distance = magnitude(s.float_y - y);
    if distance < a.sink_limit() {
        distance += RISE_EXTRA;
        s.float_y += RISE_EXTRA;
    }
    item.velocity = Vec3::new(0.0, a.float_up_speed(), 0.0);
    rise_toward(item, gekko_math::msl::fctiwz(distance));
    item.enter_air();
    item.hurt_intangible = false;
    item.change_motion_keeping_animation(motion::FLOAT_UP, UNK_0X1, assets);
    apply_box(item);
}

/// it_802ECC8C: the state's frame count (the stage reads whether the pop
/// has run).
pub fn timer(item: &ItemCore) -> i32 {
    state(item).timer
}

/// it_802ECC98: the weight of those standing on him this frame.
pub fn set_weight(item: &mut ItemCore, weight: f32) {
    state_mut(item).weight = weight;
}

/// it_802ECCA4: his motion state and the platform's place (his position
/// raised by his balloon's height).
pub fn platform(item: &ItemCore) -> (u16, Vec3) {
    let mut position = item.position;
    position.y += state(item).balloon_height;
    (item.motion, position)
}
