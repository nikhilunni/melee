//! Sheik's needles: the bundle in her hand while she charges
//! (It_Kind_Seak_NeedleHeld, itseakneedleheld.c, 802B18B0..802B1C3C) and
//! each needle she throws or drops (It_Kind_Seak_NeedleThrow,
//! itseakneedlethrown.c, 802AFD8C..802B1890). A thrown needle flies
//! straight; at a floor it sticks or bounces, as it may off what it hits.
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::{cosf, sinf},
    HsdRng,
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, NeedleState, SpawnItem,
};
use melee_lb::trigf::atan2f;
use melee_types::ItemKind;

pub struct SeakNeedleHeld;
pub struct SeakNeedleThrow;

/// ftData.x48_items indices (ftSk_Init_OnLoad registers the thrown needle
/// first, the held bundle second).
pub const THROWN_ARTICLE_INDEX: u32 = 0;
pub const HELD_ARTICLE_INDEX: u32 = 1;
/// it_803F70A8's anim_id column: the bundle has no animation.
pub const HELD_ARTICLE_STATES: [i32; 1] = [-1];
/// it_803F6F50's anim_id column: flying, dropped, stuck, (unused) resting,
/// bouncing.
pub const THROWN_ARTICLE_STATES: [i32; 5] = [0, 1, 2, 3, 4];
/// itSeakNeedleThrownAttributes: three floats.
pub const THROWN_SPECIAL_ATTRIBUTES: u32 = 3;

const FLYING: u16 = 0;
const DROPPED: u16 = 1;
const STUCK: u16 = 2;
const BOUNCING: u16 = 4;

/// Item_8026AE84(ip, 0x41F39, 127, 64): a needle meeting the floor.
const STICK_SOUND: u32 = 0x4_1F39;
/// it_802AFF08: the throw's angle from straight ahead, per facing: 3pi/4
/// in the air, pi/2 on the ground (retail @233 / @234), plus pi/2.
const AIR_THROW_ANGLE: f32 = 2.3561945;
const GROUND_THROW_ANGLE: f32 = 1.5707964;
/// The trail start's distance behind the needle, in frames of flight.
const TRAIL_FRAMES: f32 = 3.0;

/// it_803F6FA0 / it_803F7020: fall terminal velocities.
const TERMINAL_VELOCITIES: [f32; 8] = [-2.0, -2.1, -2.2, -2.3, -2.4, -2.5, -2.6, -2.7];
/// it_803F6FC0 / it_803F7040: fall gravities.
const GRAVITIES: [f32; 8] = [-0.1, -0.12, -0.14, -0.18, -0.2, -0.22, -0.24, -0.26];
/// it_803F6FE0: a dropped needle's spin per frame.
const DROP_SPINS: [f32; 8] = [
    0.20943952, 0.2443461, 0.27925268, 0.31415927, 0.34906584, 0.38397244, 0.41887903, 0.4537856,
];
/// it_803F7000: a bounce's horizontal speeds.
const BOUNCE_DRIFTS: [f32; 8] = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4];
/// it_803F7060: a bounce's spin per frame (retail data words, not constants).
#[allow(clippy::approx_constant)]
const BOUNCE_SPINS: [f32; 8] = [
    0.5235988, 0.61086524, 0.6981317, 0.7853982, 0.87266463, 0.9599311, 1.0471976, 1.134464,
];

/// How the needle leaves Sheik (it_802AFEA8's third argument), carried in
/// the spawn's argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    /// 0: thrown ahead (it_802AFF08); `airborne` is her ground_or_air
    /// (ftLib_800865CC).
    Thrown { airborne: bool },
    /// 1: dropped when her charge breaks (it_802B00F4).
    Dropped,
}
impl Launch {
    pub const fn argument(self) -> i32 {
        match self {
            Self::Thrown { airborne: false } => 0,
            Self::Dropped => 1,
            Self::Thrown { airborne: true } => 2,
        }
    }
    /// The values the launch draws (`SpawnItem::launch_draws`): a drop's
    /// spin sign and size, terminal velocity and gravity (it_802B00F4,
    /// 0x802B0180..0x802B01D8); a throw draws nothing.
    pub const fn draws(self) -> u8 {
        match self {
            Self::Dropped => 4,
            Self::Thrown { .. } => 0,
        }
    }
    const fn from_argument(argument: i32) -> Self {
        match argument {
            0 => Self::Thrown { airborne: false },
            1 => Self::Dropped,
            _ => Self::Thrown { airborne: true },
        }
    }
}

/// itSeakNeedleThrownAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the flight's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the lifetime after it meets the floor or bounces.
    fn landed_lifetime(&self) -> f32 {
        self.0[1]
    }
    /// x8: the flight's speed.
    fn speed(&self) -> f32 {
        self.0[2]
    }
}

fn needle(item: &mut ItemCore) -> &mut NeedleState {
    match &mut item.scratch {
        ItemScratch::Needle(state) => state,
        _ => unreachable!("a needle without its state"),
    }
}

/// it_80275158: the lifetime and its half.
fn set_lifetime(item: &mut ItemCore, lifetime: f32, common: &ItemCommonData) {
    item.life_timer = lifetime;
    item.half_life = lifetime * common.half_life_scale;
}

static HELD_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: HELD_ARTICLE_STATES[0],
    animation: held_anim,
    physics: no_physics,
    collision: no_collision,
}];

impl ItemLogic for SeakNeedleHeld {
    const KIND: ItemKind = ItemKind::SeakNeedleHeld;
    const STATES: &'static [ItemStateRow] = &HELD_STATES;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itSeakNeedleHeld_Logic110_PickedUp: state 0 with its animation.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, HELD_ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    }
}

/// itSeakneedleheld_UnkMotion0_Anim (802B1A80): the bundle goes once Sheik
/// lets go of it (ftSk_SpecialS_80111F70); it_802B18B0 shows one needle
/// per charge and the model follows her scale.
fn held_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let _ = item;
    ctx.owner.is_none_or(|owner| !owner.holds_needles)
}

static THROWN_STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: THROWN_ARTICLE_STATES[0],
        animation: flying_anim,
        physics: no_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: THROWN_ARTICLE_STATES[1],
        animation: tumbling_anim,
        physics: tumbling_physics,
        collision: dropped_collision,
    },
    ItemStateRow {
        animation_id: THROWN_ARTICLE_STATES[2],
        animation: stuck_anim,
        physics: no_physics,
        collision: stuck_collision,
    },
    ItemStateRow {
        animation_id: THROWN_ARTICLE_STATES[3],
        animation: resting_anim,
        physics: stop,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: THROWN_ARTICLE_STATES[4],
        animation: bouncing_anim,
        physics: tumbling_physics,
        collision: bouncing_collision,
    },
];

impl ItemLogic for SeakNeedleThrow {
    const KIND: ItemKind = ItemKind::SeakNeedleThrow;
    const STATES: &'static [ItemStateRow] = &THROWN_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Needle(NeedleState::default());
    }
    /// it_802AFD8C once Item_80268B18 returns (command variables, the
    /// lifetime, the stuck line unset, the trail at the needle), then
    /// it_802AFEA8's throw or drop.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        rng: &mut HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        item.command_variables = [0; 4];
        set_lifetime(item, a.lifetime(), common);
        let position = item.position;
        *needle(item) = NeedleState {
            line: -1,
            previous_position: position,
            ..NeedleState::default()
        };
        // it_8026BD6C sets xDCD b3 (no floor-slope projection of a grounded
        // needle's velocity); it_8026B3A8 clears grabbing; it_80272940
        // shows the model.
        item.grabbable = false;
        match Launch::from_argument(spawn.spawn_argument) {
            Launch::Thrown { airborne } => throw(item, assets, common, airborne),
            Launch::Dropped => drop(item, assets, common, rng),
        }
    }
    /// it_2725_Logic109_DmgDealt: a third of the time the needle bounces
    /// off; otherwise it goes.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        glance(item, ctx)
    }
    /// it_2725_Logic109_Clanked.
    fn clanked(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        glance(item, ctx)
    }
    /// it_2725_Logic109_DmgReceived.
    fn damage_received(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        glance(item, ctx)
    }
    /// it_2725_Logic109_HitShield.
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        glance(item, ctx)
    }
    /// it_2725_Logic109_Reflected (802B156C): the needle turns round, its
    /// trail mirrored through it, and flies back along it at full speed
    /// with its half life.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.facing = -item.facing;
        item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        let position = item.position;
        let state = needle(item);
        let dx = position.x - state.previous_position.x;
        let dy = position.y - state.previous_position.y;
        state.previous_position = Vec3::new(position.x + dx, position.y + dy, 0.0);
        let trail = state.previous_position;
        let angle = atan2f(position.y - trail.y, position.x - trail.x);
        let facing = item.facing;
        item.child_rotation_x = Some(-facing * angle);
        let speed = Attributes(&ctx.assets.special_attributes).speed();
        item.velocity.x = speed * cosf(angle);
        item.velocity.y = speed * sinf(angle);
        item.life_timer = item.half_life;
        false
    }
    /// it_2725_Logic109_ShieldBounced (802B16E4): the velocity mirrors off
    /// the shield and the model points along it.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, ctx.shield_normal);
        let mut angle = atan2f(item.velocity.y, item.velocity.x);
        while angle < 0.0 {
            angle = (f64::from(angle) + std::f64::consts::TAU) as f32;
        }
        while f64::from(angle) > std::f64::consts::TAU {
            angle = (f64::from(angle) - std::f64::consts::TAU) as f32;
        }
        item.facing = if item.velocity.x >= 0.0 { 1.0 } else { -1.0 };
        let facing = item.facing;
        item.child_rotation_x = Some(angle * -facing);
        item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        false
    }
    /// itSeakNeedleThrown_Logic109_Destroyed: the needle forgets Sheik.
    fn destroyed(item: &mut ItemCore) {
        item.owner = None;
    }
}

/// it_802AFF08 (802AFF08): state 0, the lifetime again, and the flight
/// along the throw's angle (802AFFB4: fmadds of the facing and the angle
/// with pi/2); the trail starts three frames back (802AFFFC / 802B000C:
/// fnmsubs).
fn throw(item: &mut ItemCore, assets: &ItemAssets, common: &ItemCommonData, airborne: bool) {
    let a = Attributes(&assets.special_attributes);
    item.change_motion_with(FLYING, THROWN_ARTICLE_STATES[0], ANIM_UPDATE, assets);
    set_lifetime(item, a.lifetime(), common);
    let angle = if airborne {
        AIR_THROW_ANGLE
    } else {
        GROUND_THROW_ANGLE
    };
    let pitch = fmadds(item.facing, angle, GROUND_THROW_ANGLE);
    needle(item).pitch = pitch;
    item.velocity.x = -a.speed() * cosf(pitch);
    item.velocity.y = a.speed() * sinf(pitch);
    item.velocity.z = 0.0;
    let (position, velocity) = (item.position, item.velocity);
    let trail = Vec3::new(
        fnmsubs(TRAIL_FRAMES, velocity.x, position.x),
        fnmsubs(TRAIL_FRAMES, velocity.y, position.y),
        0.0,
    );
    let facing = item.facing;
    needle(item).previous_position = trail;
    item.child_rotation_x = Some(-facing * atan2f(position.y - trail.y, position.x - trail.x));
}

/// it_802B00F4 (802B00F4): state 1, the lifetime again, still, then
/// itSeakNeedleThrown_SetupDrop's draws: the spin's sign then its size,
/// the fall's terminal velocity, its gravity.
fn drop(item: &mut ItemCore, assets: &ItemAssets, common: &ItemCommonData, rng: &mut HsdRng) {
    let a = Attributes(&assets.special_attributes);
    item.change_motion_with(DROPPED, THROWN_ARTICLE_STATES[1], ANIM_UPDATE, assets);
    set_lifetime(item, a.lifetime(), common);
    item.velocity = Vec3::ZERO;
    let sign = random_sign(rng);
    let spin = DROP_SPINS[rng.randi(8) as usize] * sign;
    let terminal_velocity = TERMINAL_VELOCITIES[rng.randi(8) as usize];
    let gravity = GRAVITIES[rng.randi(8) as usize];
    let state = needle(item);
    state.spin = spin;
    state.drift = 0.0;
    state.terminal_velocity = terminal_velocity;
    state.gravity = gravity;
}

/// `(HSD_Randi(2) == 0) ? 1.0f : -1.0f`.
fn random_sign(rng: &mut HsdRng) -> f32 {
    if rng.randi(2) == 0 {
        1.0
    } else {
        -1.0
    }
}

/// `ABS(it_803F7020[HSD_Randi(8)])`: the macro draws once for its test and
/// again for its value.
fn bounce_rise(rng: &mut HsdRng) -> f32 {
    let tested = TERMINAL_VELOCITIES[rng.randi(8) as usize];
    let value = TERMINAL_VELOCITIES[rng.randi(8) as usize];
    if tested < 0.0 {
        -value
    } else {
        value
    }
}

/// itSeakNeedleThrown_SetupBounce: the spin's sign and size, the drift's
/// sign and size, the terminal velocity, the gravity.
fn set_up_bounce(item: &mut ItemCore, rng: &mut HsdRng) {
    let sign = random_sign(rng);
    let spin = BOUNCE_SPINS[rng.randi(8) as usize] * sign;
    let sign = random_sign(rng);
    let drift = BOUNCE_DRIFTS[rng.randi(8) as usize] * sign;
    let terminal_velocity = TERMINAL_VELOCITIES[rng.randi(8) as usize];
    let gravity = GRAVITIES[rng.randi(8) as usize];
    let state = needle(item);
    state.spin = spin;
    state.drift = drift;
    state.terminal_velocity = terminal_velocity;
    state.gravity = gravity;
}

/// The hit callbacks' shared body: HSD_Randi(3) == 0 bounces the needle
/// (hitbox 0 gone, it_80272560; the landed lifetime; a rise; the bounce's
/// draws; state 4); otherwise it is destroyed.
fn glance(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
    let cell = ctx.rng.expect("needle hit callbacks draw");
    let mut rng = cell.get();
    let bounces = rng.randi(3) == 0;
    if bounces {
        item.hitboxes[0] = None;
        let lifetime = Attributes(&ctx.assets.special_attributes).landed_lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * ctx.half_life_scale;
        item.velocity = Vec3::new(0.0, bounce_rise(&mut rng), 0.0);
        item.enter_air();
        set_up_bounce(item, &mut rng);
        item.change_motion_with(BOUNCING, THROWN_ARTICLE_STATES[4], ANIM_UPDATE, ctx.assets);
    }
    cell.set(rng);
    !bounces
}

/// HSD_JObjAddRotationX on the model's child joint.
fn turn_child(item: &mut ItemCore, spin: f32) {
    item.child_rotation_x = Some(item.child_rotation_x.unwrap_or(0.0) + spin);
}

/// it_80273130: the lifetime counts down; the needle goes when it is spent.
fn tick_lifetime(item: &mut ItemCore) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itSeakneedlethrown_UnkMotion0_Anim: the trail starts where the needle
/// is.
fn flying_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let position = item.position;
    needle(item).previous_position = position;
    tick_lifetime(item)
}

/// itSeakneedlethrown_UnkMotion1_Anim: the trail, the spin.
fn tumbling_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let position = item.position;
    let spin = {
        let state = needle(item);
        state.previous_position = position;
        state.spin
    };
    turn_child(item, spin);
    tick_lifetime(item)
}

/// itSeakneedlethrown_UnkMotion2_Anim: the trail, the model at the stuck
/// pitch.
fn stuck_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let position = item.position;
    let pitch = {
        let state = needle(item);
        state.previous_position = position;
        state.pitch
    };
    item.child_rotation_x = Some(pitch);
    tick_lifetime(item)
}

/// itSeakneedlethrown_UnkMotion3_Anim.
fn resting_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    tick_lifetime(item)
}

/// itSeakneedlethrown_UnkMotion4_Anim: the trail and the spin; a bounce
/// never times out.
fn bouncing_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let position = item.position;
    let spin = {
        let state = needle(item);
        state.previous_position = position;
        state.spin
    };
    turn_child(item, spin);
    false
}

/// itSeakneedlethrown_UnkMotion1_Phys / 4: the drift, then gravity down to
/// the terminal velocity.
fn tumbling_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let state = *needle(item);
    item.velocity.x = state.drift;
    item.velocity.y += state.gravity;
    if item.velocity.y < state.terminal_velocity {
        item.velocity.y = state.terminal_velocity;
    }
}

/// itResetVelocity.
fn stop(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itSeakNeedleThrown_CheckGroundHit: it_8026EA20 along the trail (the
/// needle moves to the contact); a live line fires its joint callback
/// (mpColl_80043558), carries the needle with it (mpGetSpeed) and gives
/// its normal's angle.
fn meets_floor(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let from = needle(item).previous_position;
    let Some(hit) =
        ctx.map
            .check_all_remap(-1, -1, from.x, from.y, item.position.x, item.position.y)
    else {
        return false;
    };
    item.position = hit.pos;
    needle(item).line = hit.line_id;
    if !ctx.map.line_is_active(hit.line_id) {
        return false;
    }
    touch_line(item, ctx, hit.line_id);
    let angle = atan2f(hit.normal.y, hit.normal.x);
    let state = needle(item);
    state.previous_line_angle = angle;
    state.line_angle = angle;
    true
}

/// mpColl_80043558 on the needle's collision, then mpGetSpeed into its
/// velocity.
fn touch_line(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>, line: i32) {
    if let Some(collision) = item.collision.as_mut() {
        ctx.map.notify_line_joint(collision, line);
    }
    if let Some(speed) = ctx.map.line_speed(line, &item.position) {
        item.velocity = speed;
    }
}

/// itSeakneedlethrown_UnkMotion0_Coll (802B05E4): the model points along
/// the flight; at a floor, HSD_Randi(5) sticks it (0..2: still, the landed
/// lifetime, state 2, the sound) or bounces it (3, 4).
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let trail = needle(item).previous_position;
    let pitch = -item.facing * atan2f(item.position.y - trail.y, item.position.x - trail.x);
    item.child_rotation_x = Some(pitch);
    if !meets_floor(item, ctx) {
        return false;
    }
    let lifetime = Attributes(&ctx.assets.special_attributes).landed_lifetime();
    let half_life_scale = ctx.half_life_scale;
    let rng = ctx.rng.expect("needle floor draws");
    let mut random = rng.get();
    match random.randi(5) {
        0..=2 => {
            item.enter_air();
            item.velocity = Vec3::ZERO;
            item.life_timer = lifetime;
            item.half_life = lifetime * half_life_scale;
            needle(item).pitch = pitch;
            item.change_motion_with(STUCK, THROWN_ARTICLE_STATES[2], ANIM_UPDATE, ctx.assets);
            item.sound_requests.push(STICK_SOUND);
        }
        3 | 4 => {
            item.life_timer = lifetime;
            item.half_life = lifetime * half_life_scale;
            item.velocity.y = bounce_rise(&mut random);
            item.enter_air();
            set_up_bounce(item, &mut random);
            item.change_motion_with(BOUNCING, THROWN_ARTICLE_STATES[4], ANIM_UPDATE, ctx.assets);
        }
        _ => {}
    }
    rng.set(random);
    false
}

/// itSeakneedlethrown_UnkMotion1_Coll: a dropped needle bounces off the
/// floor (its fall reversed) with the sound.
fn dropped_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !meets_floor(item, ctx) {
        return false;
    }
    let lifetime = Attributes(&ctx.assets.special_attributes).landed_lifetime();
    item.life_timer = lifetime;
    item.half_life = lifetime * ctx.half_life_scale;
    item.velocity.y = item.velocity.y.abs();
    item.enter_air();
    let rng = ctx.rng.expect("needle bounce draws");
    let mut random = rng.get();
    set_up_bounce(item, &mut random);
    rng.set(random);
    item.change_motion_with(BOUNCING, THROWN_ARTICLE_STATES[4], ANIM_UPDATE, ctx.assets);
    item.sound_requests.push(STICK_SOUND);
    false
}

/// itSeakneedlethrown_UnkMotion2_Coll: a stuck needle rides its line (or
/// stops without one) and turns with it.
fn stuck_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.enter_air();
    item.velocity = Vec3::ZERO;
    let line = {
        let state = needle(item);
        state.previous_line_angle = state.line_angle;
        state.line
    };
    if line != -1 {
        if ctx.map.line_is_active(line) {
            touch_line(item, ctx, line);
            let normal = ctx.map.line_get_normal(line);
            needle(item).line_angle = atan2f(normal.y, normal.x);
        }
    } else {
        item.velocity = Vec3::ZERO;
    }
    let state = needle(item);
    if state.line_angle != state.previous_line_angle {
        state.pitch += state.line_angle - state.previous_line_angle;
        let pitch = state.pitch;
        item.child_rotation_x = Some(pitch);
    }
    false
}

/// itSeakneedlethrown_UnkMotion4_Coll: a bouncing needle sticks where it
/// lands, keeping its model's pitch.
fn bouncing_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !meets_floor(item, ctx) {
        return false;
    }
    let lifetime = Attributes(&ctx.assets.special_attributes).landed_lifetime();
    item.life_timer = lifetime;
    item.half_life = lifetime * ctx.half_life_scale;
    // HSD_JObjGetRotationX(child).
    let turned = item.child_rotation_x.unwrap_or(0.0);
    needle(item).pitch = turned;
    item.change_motion_with(STUCK, THROWN_ARTICLE_STATES[2], ANIM_UPDATE, ctx.assets);
    item.enter_air();
    item.velocity = Vec3::ZERO;
    false
}
