//! Bowser's Fire Breath flames (It_Kind_Koopa_Flame), itkoopaflame.c
//! (802AC8A0..802AD478). Bowser's mouth spawns one a frame while he
//! breathes (itKoopaFlame_Spawn). Each flies at a random speed and angle,
//! scaled by how much breath he had left; its hitbox lasts attribute x4
//! frames and the flame itself attribute x0. A surface it touches bends
//! its flight along it.
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, KoopaFlameState, SpawnItem,
};
use melee_types::{
    mp::{collide, EcbSourceParams},
    ItemKind,
};

pub struct KoopaFlame;

/// ftData.x48_items index (ftKp_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;
/// ItemStateTable_KoopaFlame's anim_id column: one motion state.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itKoopaFlame_Attributes x0..x14.
pub const SPECIAL_ATTRIBUTES: u32 = 6;

/// efSync_Spawn(1243 + x48, gobj, jobj): the four flame generators
/// (efLib_CreateGenerator_Attach_Scale 0x2EE5..0x2EE8 on the flame's JObj).
const FIRST_FLAME_EFFECT: u16 = 0x4DB;
/// The fixed ECB the collision callback writes each frame (@310).
const ECB_REACH: f32 = 3.0;
/// M_PI, M_PI_2 and M_TAU as the C's doubles (@187, @221, @185).
const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const TAU: f64 = std::f64::consts::TAU;
/// itKoopaFlame_Update_Angle's turn rates, floats widened to double
/// (@222, @223): a glancing touch barely turns the flame, a blunt one
/// turns it half way.
const GLANCING_TURN: f64 = 0.02_f32 as f64;
const BLUNT_TURN: f64 = 0.5;

/// SpawnItem.spawn_argument: the flame generator in the low two bits, then
/// the owner's reach and life fuels as itKoopaFlame_Spawn's two s32
/// arguments, ten bits each (both stay under 1024).
pub fn spawn_argument(effect: i32, reach: i32, life: i32) -> i32 {
    assert!((0..4).contains(&effect), "flame effect {effect}");
    assert!(
        (0..1024).contains(&reach) && (0..1024).contains(&life),
        "itKoopaFlame_Spawn: fuels {reach} / {life}"
    );
    effect | reach << 2 | life << 12
}

/// The owner's full fuels (ftKp_SpecialLw_80134DE0 / 80134E1C), which the
/// flame divides its arguments by: the item scene appends them to the
/// special attributes at load ([`owner_fuels`]).
const FULL_REACH: usize = 6;
const FULL_LIFE: usize = 7;

/// ftKp_SpecialLw_80134DE0 / ftKp_SpecialLw_80134E1C return the owner's
/// attributes x10 and x18 as s32 (fctiwz); the flame converts them back to
/// float before dividing (802ACCE4, 802ACD20).
pub fn owner_fuels(reach_max: f32, life_max: f32) -> [f32; 2] {
    [
        gekko_math::msl::fctiwz(reach_max) as f32,
        gekko_math::msl::fctiwz(life_max) as f32,
    ]
}

/// The flame's special attributes (itKoopaFlame_Attributes).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the flame's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the frames its hitbox lasts.
    fn hitbox_frames(&self) -> f32 {
        self.0[1]
    }
    /// x8 / xC: the speed's range at full breath.
    fn speed(&self) -> (f32, f32) {
        (self.0[2], self.0[3])
    }
    /// x10 / x14: the angle's range from straight up, facing right.
    fn angle(&self) -> (f32, f32) {
        (self.0[4], self.0[5])
    }
    fn full_reach(&self) -> f32 {
        self.0[FULL_REACH]
    }
    fn full_life(&self) -> f32 {
        self.0[FULL_LIFE]
    }
}

fn state(item: &mut ItemCore) -> &mut KoopaFlameState {
    match &mut item.scratch {
        ItemScratch::KoopaFlame(state) => state,
        _ => unreachable!("a Fire Breath flame without its state"),
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

/// it_3F2F.c's Logic111 row.
impl ItemLogic for KoopaFlame {
    const KIND: ItemKind = ItemKind::KoopaFlame;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itKoopaFlame_Setup clears xDC8 x15 (it_8026B3A8).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::KoopaFlame(KoopaFlameState::default());
    }
    /// itKoopaFlame_Spawn (802ACBA0) once Item_80268B18 returns: the
    /// command variables, the lifetime, the breath's share of full for the
    /// speed and the scale (integers converted and divided, 802ACCEC /
    /// 802ACD28: fdivs), two HSD_Randf draws for the speed (802ACDC4:
    /// fmadds, then fmuls) and the angle (802ACDE0: fmadds), mirrored for
    /// a left-facing flame and wrapped; then itKoopaFlame_Setup's motion,
    /// animation step and first physics.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        item.command_variables = [0; 4];
        // it_80275158: both timers.
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        let argument = spawn.spawn_argument;
        let (effect, reach, life) = (argument & 3, (argument >> 2) & 1023, argument >> 12);
        let reach_share = reach as f32 / a.full_reach();
        let scale = life as f32 / a.full_life();
        let (slow, fast) = a.speed();
        let speed = reach_share * fmadds(fast - slow, rng.randf(), slow);
        let (low, high) = a.angle();
        let angle = fmadds(high - low, rng.randf(), low);
        let angle = if item.facing == 1.0 { angle } else { -angle };
        let facing = item.facing;
        *state(item) = KoopaFlameState {
            direction: Vec3::new(facing, 0.0, 0.0),
            heading: Vec3::ZERO,
            angle: wrap(angle),
            speed,
            hitbox_size: 0.0,
            scale,
            frames: 0,
            effect_spawned: false,
            effect,
        };
        // itKoopaFlame_Setup (802ACEBC): not grabbable, not held, shown,
        // motion 0, one animation step (Item_802694CC), then the physics
        // callback for the first velocity.
        item.blast_zone_checked = false;
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.advance_animation(assets);
        fly(item);
    }
    /// itKoopaFlame_Setup's tail: a collision pass (it_8026D9A0) from one
    /// step behind the spawn point, which primes the collision's previous
    /// position; the flame then returns to where it spawned.
    fn spawned_with_map(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &ItemCommonData,
        _spawn: &SpawnItem,
        map: &mut melee_mp::CollMap,
    ) {
        let position = item.position;
        item.position.x -= item.velocity.x;
        item.position.y -= item.velocity.y;
        item.airborne_pass(map);
        item.position = position;
    }
    /// itKoopaFlame_Logic111_DmgDealt: the flame burns on.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itKoopaFlame_Logic111_Reflected (802AD380): the angle half a turn
    /// on (a double add, rounded), wrapped; facing and velocity reversed.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        let flame = state(item);
        flame.angle = wrap((f64::from(flame.angle) + PI) as f32);
        item.facing = -item.facing;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        false
    }
    /// itKoopaFlame_Logic111_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itKoopaFlame_Logic111_Absorbed: efLib_DestroyAll, and the flame ends.
    fn absorbed(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.events.push(ItemEvent::DestroyEffects);
        true
    }
    /// itKoopaFlame_Logic111_ShieldBounced (802AD3E0): the velocity
    /// mirrored off the shield (lbVector_Mirror), and the angle taken from
    /// it as atan2f(y, x).
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, ctx.shield_normal);
        item.velocity.z = 0.0;
        let angle = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        state(item).angle = wrap(angle);
        false
    }
    /// itKoopaFlame_Logic111_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
}

/// Item_ClampAngle (it/kinds/inlines.h): into -pi..pi, each step a double
/// add rounded to the stored float (802ACE18..802ACE58).
fn wrap(mut angle: f32) -> f32 {
    while f64::from(angle) < -PI {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > PI {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// Item_ClampAngleReverse: the same range, the upper bound tested first
/// (802ACA44..802ACA6C).
fn wrap_reverse(mut angle: f32) -> f32 {
    while f64::from(angle) > PI {
        angle = (f64::from(angle) - TAU) as f32;
    }
    while f64::from(angle) < -PI {
        angle = (f64::from(angle) + TAU) as f32;
    }
    angle
}

/// itKoopaFlame_UnkMotion0_Phys (802AD160): the velocity at the flight's
/// speed along its angle (sine for x, cosine for y; fmuls each), and its
/// unit vector (lbVector_Normalize).
fn fly(item: &mut ItemCore) {
    let flame = *state(item);
    item.velocity.x = flame.speed * sinf(flame.angle);
    item.velocity.y = flame.speed * cosf(flame.angle);
    item.velocity.z = 0.0;
    state(item).heading = melee_lb::vector::normalize(item.velocity);
}

fn physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    fly(item);
}

/// itKoopaFlame_UnkMotion0_Anim (802ACF9C): a live hitbox takes its
/// authored size, read once, times the scale (802ACFF8: fmuls); the model
/// takes the scale; past attribute x4 frames the hitbox goes
/// (it_802725D4); the first call makes the flame's generator; then the
/// lifetime (it_80273130).
fn animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let scale = state(item).scale;
    if let Some(radius) = item.hitboxes[0].as_ref().map(|hit| hit.descriptor.radius) {
        let size = &mut state(item).hitbox_size;
        if *size == 0.0 {
            *size = radius;
        }
        let size = *size;
        item.hitboxes[0].as_mut().unwrap().descriptor.radius = size * scale;
    }
    item.model_scale = Vec3::new(scale, scale, scale);
    let flame = state(item);
    flame.frames += 1;
    let frames = flame.frames;
    if frames as f32 > Attributes(&ctx.assets.special_attributes).hitbox_frames() {
        item.clear_hitboxes();
    }
    let flame = state(item);
    if !flame.effect_spawned {
        flame.effect_spawned = true;
        let id = FIRST_FLAME_EFFECT + flame.effect as u16;
        item.events.push(ItemEvent::OwnEffect { id });
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itKoopaFlame_UnkMotion0_Coll (802AD1E8): the angle wrapped, the ECB
/// three units each way, an airborne pass that never lands (it_8026D9A0);
/// any surface touched bends the flight.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let flame = state(item);
    flame.angle = wrap(flame.angle);
    let data = item.collision.as_mut().expect("item map collision");
    match &mut data.ecb_source.params {
        EcbSourceParams::Fixed {
            up,
            down,
            front,
            back,
            ..
        } => {
            *up = ECB_REACH;
            *down = ECB_REACH;
            *front = ECB_REACH;
            *back = ECB_REACH;
        }
        EcbSourceParams::JObj { .. } => unreachable!("a flame's ECB follows no joint"),
    }
    item.airborne_pass(ctx.map);
    let data = item.collision.as_ref().expect("item map collision");
    let env = data.env_flags as u32;
    let mut normals = [None; 4];
    if env & collide::FLOOR_MASK != 0 {
        normals[0] = Some(data.floor.normal);
    }
    if env & collide::CEILING_MASK != 0 {
        normals[1] = Some(data.ceiling.normal);
    }
    if env & collide::LEFT_WALL_MASK != 0 {
        normals[2] = Some(data.left_facing_wall.normal);
    }
    if env & collide::RIGHT_WALL_MASK != 0 {
        normals[3] = Some(data.right_facing_wall.normal);
    }
    if normals.iter().any(Option::is_some) {
        bend_direction(item, normals);
        bend_angle(item);
    }
    false
}

/// itKoopaFlame_Update_Direction (802AC8A0): the angle wrapped; each
/// touched surface's normal joins the direction's x and y (fadds, in the
/// order floor, ceiling, left wall, right wall), which is normalised again.
fn bend_direction(item: &mut ItemCore, normals: [Option<Vec3>; 4]) {
    let flame = state(item);
    flame.angle = wrap(flame.angle);
    let mut direction = flame.direction;
    for normal in normals.into_iter().flatten() {
        direction.x += normal.x;
        direction.y += normal.y;
    }
    flame.direction = melee_lb::vector::normalize(direction);
}

/// itKoopaFlame_Update_Angle (802AC9F0): the signed turn from the bent
/// direction to the heading (two atan2f(x, y), fsubs, wrapped); the angle
/// between them (lbVector_Angle) times a share of that turn over pi (a
/// double divide and multiply, rounded: 802ACAD4..DC / 802ACB04..0C),
/// signed like the turn (802ACB28: fmuls), comes off the flight's angle.
fn bend_angle(item: &mut ItemCore) {
    let flame = state(item);
    let turn = wrap_reverse(
        melee_lb::trigf::atan2f(flame.heading.x, flame.heading.y)
            - melee_lb::trigf::atan2f(flame.direction.x, flame.direction.y),
    );
    let correction = if turn == 0.0 {
        0.0
    } else {
        let between = melee_lb::vector::angle(flame.direction, flame.heading);
        let magnitude = f64::from(if turn < 0.0 { -turn } else { turn });
        let rate = if magnitude < HALF_PI {
            GLANCING_TURN
        } else {
            BLUNT_TURN
        };
        let share = (rate * (magnitude / PI)) as f32;
        between * if turn < 0.0 { -share } else { share }
    };
    flame.angle = wrap(flame.angle - correction);
}
