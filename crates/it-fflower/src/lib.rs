//! The Fire Flower (It_Kind_F_Flower, itfflower.c 80292D48..80293660) and
//! its flames (It_Kind_L_Gun_Beam, itlgunbeam.c 802993E0..80299A90; the
//! decomp's names are swapped with It_Kind_F_Flower_Flame, which nothing in
//! a match spawns).
//!
//! The holder's FireFlowerShoot accessory (ftCo_800CE650) plays the
//! flower's opening (it_80292E64), fires a flame a few frames apart while A
//! is held (it_80292F14: one of xD4C's shots each) and closes it
//! (it_80292EAC). Flames of six shots share one hit group
//! (Item_8026AE60). A thrown or dropped flower without shots vanishes on
//! the first floor it touches.
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    AirLanding, EnteredAir, FlameState, ItemAnimationContext, ItemCollisionContext, ItemControl,
    ItemCore, ItemEvent, ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch,
    ItemStateRow, SlidePass, SpawnItem,
};
use melee_types::{
    mp::{collide, EcbSourceParams},
    ItemKind,
};

/// It_Kind_F_Flower.
pub struct FireFlower;
/// It_Kind_L_Gun_Beam: one flame.
pub struct FireFlowerFlame;

/// it_803F6090's anim_id column.
pub const ARTICLE_STATES: [i32; 8] = [0, 0, 0, 1, 2, 3, 4, 0];
/// The article states (anim ids 0..4) the flower's model plays.
pub const ANIMATED_STATES: usize = 5;
/// FFlowerAttr: x0..x14.
pub const SPECIAL_ATTRIBUTES: u32 = 6;
/// it_803F6630's anim_id column.
pub const FLAME_ARTICLE_STATES: [i32; 1] = [0];
/// ItLGunBeamAttr: x0..x10.
pub const FLAME_SPECIAL_ATTRIBUTES: u32 = 5;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const OPENING: u16 = 3;
    pub const FIRING: u16 = 4;
    pub const CLOSING: u16 = 5;
    pub const FLYING: u16 = 6;
    pub const SLIDING: u16 = 7;
}

/// it_80292EF8 (80292EF8): the flower's mouth, in the hand joint's frame.
pub const MOUTH: Vec3 = Vec3::new(0.0, 5.5, 2.5);
/// it_80292F14's efSync_Spawn(0x473, gobj, bones[4]): the flame at the
/// flower's mouth while it fires.
const MOUTH_FLAME_EFFECT: u16 = 0x473;
const MOUTH_BONE: usize = 4;
/// it_80292F14: a new hit group every six flames.
const FLAMES_PER_GROUP: i32 = 6;
/// it_802998A0's efSync_Spawn(0x44E + Randi(4), gobj, jobj, &facing, &1):
/// one of four flame generators.
const FIRST_FLAME_EFFECT: u16 = 0x44E;
/// itLgunbeam_UnkMotion0_Coll's fixed ECB.
const FLAME_ECB_REACH: f32 = 3.0;
/// M_PI, M_PI_2 and M_TAU as the C's doubles.
const PI: f64 = core::f64::consts::PI;
const HALF_PI: f64 = core::f64::consts::FRAC_PI_2;
const TAU: f64 = core::f64::consts::TAU;
/// it_80299528's turn rates, the first a float widened to double.
const GLANCING_TURN: f64 = 0.02_f32 as f64;
const BLUNT_TURN: f64 = 0.5;

/// The flower's scratch: x0 flames since the hit group changed, x4 the
/// mouth flame made.
fn flower(item: &mut ItemCore) -> (&mut i32, &mut bool) {
    match &mut item.scratch {
        ItemScratch::FireFlower { flames, mouth_lit } => (flames, mouth_lit),
        _ => unreachable!("a Fire Flower without its scratch"),
    }
}

static STATES: [ItemStateRow; 8] = [
    row(motion::REST, rest_animation, no_physics, rest_collision),
    row(motion::FALL, fall_animation, gravity_physics, flying_collision),
    row(motion::HELD, held_animation, no_physics, no_collision),
    row(motion::OPENING, held_animation, no_physics, no_collision),
    row(motion::FIRING, held_animation, no_physics, no_collision),
    row(motion::CLOSING, held_animation, no_physics, no_collision),
    row(motion::FLYING, flying_animation, gravity_physics, flying_collision),
    row(motion::SLIDING, sliding_animation, no_physics, sliding_collision),
];

const fn row(
    state: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state as usize],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for FireFlower {
    const KIND: ItemKind = ItemKind::FFlower;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The flower draws a hit group id at spawn (xAC4 = Item_8026AE60).
    const NEW_HIT_GROUP: bool = true;
    /// itFFlower_Logic25_Spawned (80292DAC): the shots (x0), the scratch
    /// cleared, then the fall (it_8029313C). Item_8026AE60's id is the
    /// scene's (`NEW_HIT_GROUP`).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = assets.special_attributes[0].to_bits() as i32;
        item.scratch = ItemScratch::FireFlower {
            flames: 0,
            mouth_lit: false,
        };
        change(item, motion::FALL, assets);
    }
    /// itFFlower_Logic25_PickedUp: state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, context.assets);
    }
    /// itFFlower_Logic25_Dropped / Thrown: state 6, with ITEM_ANIM_UPDATE
    /// alone (no thrown damage scale).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, context.assets);
    }
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, context.assets);
    }
    /// itFFlower_Logic25_EnteredAir: state 7.
    const ENTERED_AIR: Option<EnteredAir> = Some(|item, assets| change(item, motion::SLIDING, assets));
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // it_80292E64 (80292E64): held or closing, it opens.
            ItemControl::Open => {
                if matches!(item.motion, motion::HELD | motion::CLOSING) {
                    change(item, motion::OPENING, assets);
                }
            }
            // it_80292EAC (80292EAC): opening or firing, it closes.
            ItemControl::Close => {
                if matches!(item.motion, motion::OPENING | motion::FIRING) {
                    change(item, motion::CLOSING, assets);
                }
            }
            // it_80292F14 (80292F14)'s own half: the mouth flame once, a
            // shot fewer, and the hit group's count (the flame itself is
            // the holder's spawn request, and the new group id the
            // scene's).
            ItemControl::Fire => {
                if item.ammo > 0 {
                    let (_, mouth_lit) = flower(item);
                    if !*mouth_lit {
                        *mouth_lit = true;
                        item.events.push(ItemEvent::BoneGenerator {
                            id: MOUTH_FLAME_EFFECT,
                            bone: MOUTH_BONE,
                        });
                    }
                    item.ammo -= 1;
                }
            }
            other => unimplemented!("itfflower.c: control {other:?}"),
        }
    }
}

/// Whether a flame fired now (`ItemControl::Fire` with shots left) closes
/// the flower's hit group: it_80292F14's x0 = (x0 + 1) % 6, a new
/// Item_8026AE60 id at 0. Called by the scene after the flame's spawn.
pub fn count_flame(item: &mut ItemCore) -> bool {
    let (flames, _) = flower(item);
    *flames = (*flames + 1) % FLAMES_PER_GROUP;
    *flames == 0
}

fn change(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// The flower's msid_check (itfflower.c:38-56), first in every animation
/// callback: its effects go (efLib_DestroyAll) and the mouth flame is
/// unlit, except while it fires with shots left.
fn put_out(item: &mut ItemCore) {
    if item.motion == motion::FIRING && item.ammo > 0 {
        return;
    }
    item.events.push(ItemEvent::DestroyEffects);
    *flower(item).1 = false;
}

/// it_80272C6C == 0: the state's joint animation has no frames left.
fn animation_ended(item: &ItemCore, assets: &ItemAssets) -> bool {
    item.article_animation_ended(assets)
}

/// it_80292FF0 (80292FF0): stopped, grabbable, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::REST, assets);
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itFflower_UnkMotion0_Anim (80293040): the rest loops. xDC8 x2 (a flower
/// a stage event dropped, it_8026BE84) is never set for a random one.
fn rest_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    put_out(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::REST, ctx.assets);
    }
    false
}

/// itFflower_UnkMotion6_Anim (80293164), the fall: it loops.
fn fall_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    put_out(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itFflower_UnkMotion5_Anim (802932AC), states 2..5 in the hand: held and
/// closing go back to held, opening and firing go on firing.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    put_out(item);
    if animation_ended(item, ctx.assets) {
        let next = match item.motion {
            motion::OPENING | motion::FIRING => motion::FIRING,
            _ => motion::HELD,
        };
        change(item, next, ctx.assets);
    }
    false
}

/// itFflower_UnkMotion6_Anim for state 6 (thrown or dropped): at the
/// animation's end it falls on in state 1 (the callback both rows share).
fn flying_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    put_out(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itFflower_UnkMotion7_Anim (8029355C): the slide loops.
fn sliding_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    put_out(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::SLIDING, ctx.assets);
    }
    false
}

/// it_80272860 with ItemAttr x10 / x14 (states 1 and 6: no spin).
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itFflower_UnkMotion0_Coll: it_8026D62C, off the floor it falls
/// (it_8029313C).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ctx.assets);
    }
    false
}

/// itFflower_UnkMotion1_Coll / UnkMotion6_Coll: with shots left it lands
/// (it_8026E15C(it_80292FF0)); empty, the floor pass it_8026DF34 removes it.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo == 0 {
        return item.air_pass(ctx.map);
    }
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itFflower_UnkMotion7_Coll: it_8026E8C4(it_80292FF0, it_8029313C).
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Stopped => come_to_rest(item, ctx.assets),
        SlidePass::Airborne => change(item, motion::FALL, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}

/// ItLGunBeamAttr.
struct FlameAttributes<'a>(&'a [f32]);
impl FlameAttributes<'_> {
    /// x0: the lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the speed's range.
    fn speed(&self) -> (f32, f32) {
        (self.0[1], self.0[2])
    }
    /// xC / x10: the angle's range from straight up, facing right.
    fn angle(&self) -> (f32, f32) {
        (self.0[3], self.0[4])
    }
}

fn flame(item: &mut ItemCore) -> &mut FlameState {
    match &mut item.scratch {
        ItemScratch::Flame(state) => state,
        _ => unreachable!("a Fire Flower flame without its state"),
    }
}

static FLAME_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: FLAME_ARTICLE_STATES[0],
    animation: flame_animation,
    physics: flame_physics,
    collision: flame_collision,
}];

impl ItemLogic for FireFlowerFlame {
    const KIND: ItemKind = ItemKind::LGunBeam;
    const STATES: &'static [ItemStateRow] = &FLAME_STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Flame(FlameState::default());
    }
    /// it_802996D0 (802996D0) once Item_80268B18 returns: the command
    /// variables, the lifetime, the start (`spawn.previous_position`, the
    /// mouth), two HSD_Randf draws for the speed (802997C4: fmadds) and the
    /// angle (802997DC: fmadds), mirrored facing left and wrapped; then
    /// it_802998A0 with HSD_Randi(4)'s generator: not grabbable, shown,
    /// motion 0, one animation step (Item_802694CC), the first velocity
    /// (the physics callback) and the generator (efSync_Spawn(0x44E + r)).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let a = FlameAttributes(&assets.special_attributes);
        item.command_variables = [0; 4];
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        let (slow, fast) = a.speed();
        let speed = fmadds(fast - slow, rng.randf(), slow);
        let (low, high) = a.angle();
        let angle = fmadds(high - low, rng.randf(), low);
        let angle = if item.facing == 1.0 { angle } else { -angle };
        *flame(item) = FlameState {
            origin: spawn.previous_position,
            angle: wrap(angle),
            speed,
            lifetime,
            ..FlameState::default()
        };
        item.blast_zone_checked = false;
        let generator = rng.randi(4);
        item.grabbable = false;
        item.hidden = false;
        item.change_motion_with(0, FLAME_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.advance_animation(assets);
        fly(item);
        item.events.push(ItemEvent::FacingGenerator {
            id: FIRST_FLAME_EFFECT + generator as u16,
            facing: item.facing,
            scale: 1.0,
        });
    }
    /// it_802998A0's tail: a collision pass (it_8026D9A0) from one step
    /// behind the spawn point; the flame then returns to where it spawned.
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
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itLGunBeam_Logic39_Reflected (80299B6C): the angle half a turn on
    /// (a double add, rounded), wrapped; facing and velocity reversed.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        let state = flame(item);
        state.angle = wrap((f64::from(state.angle) + PI) as f32);
        item.facing = -item.facing;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        false
    }
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// Item_ClampAngle: into -pi..pi, each step a double add rounded.
fn wrap(mut angle: f32) -> f32 {
    while f64::from(angle) < -PI {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > PI {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// Item_ClampAngleReverse: the upper bound tested first.
fn wrap_reverse(mut angle: f32) -> f32 {
    while f64::from(angle) > PI {
        angle = (f64::from(angle) - TAU) as f32;
    }
    while f64::from(angle) < -PI {
        angle = (f64::from(angle) + TAU) as f32;
    }
    angle
}

/// itLgunbeam_UnkMotion0_Phys (802999E4): the start of the step saved,
/// the velocity at the speed along the angle (sine for x, cosine for y),
/// and its unit vector (lbVector_Normalize).
fn fly(item: &mut ItemCore) {
    let position = item.position;
    let state = flame(item);
    state.origin = position;
    let (speed, angle) = (state.speed, state.angle);
    item.velocity = Vec3::new(speed * sinf(angle), speed * cosf(angle), 0.0);
    flame(item).heading = melee_lb::vector::normalize(item.velocity);
}

fn flame_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    fly(item);
}

/// itLgunbeam_UnkMotion0_Anim (8029999C): at the lifetime's end
/// (it_80273130) the generator goes (efLib_DestroyAll) and so does it.
fn flame_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.events.push(ItemEvent::DestroyEffects);
        return true;
    }
    false
}

/// itLgunbeam_UnkMotion0_Coll (80299A68): the angle wrapped, the ECB three
/// units each way, an airborne pass that never lands (it_8026D9A0); any
/// surface touched bends the flight.
fn flame_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let state = flame(item);
    state.angle = wrap(state.angle);
    let data = item.collision.as_mut().expect("item map collision");
    match &mut data.ecb_source.params {
        EcbSourceParams::Fixed {
            up,
            down,
            front,
            back,
            ..
        } => {
            *up = FLAME_ECB_REACH;
            *down = FLAME_ECB_REACH;
            *front = FLAME_ECB_REACH;
            *back = FLAME_ECB_REACH;
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

/// it_802993E0 (802993E0): the angle wrapped; the touched surfaces'
/// normals summed from zero (x and y, fadds in the order floor, ceiling,
/// left wall, right wall) and normalised.
fn bend_direction(item: &mut ItemCore, normals: [Option<Vec3>; 4]) {
    let state = flame(item);
    state.angle = wrap(state.angle);
    let mut direction = Vec3::ZERO;
    for normal in normals.into_iter().flatten() {
        direction.x += normal.x;
        direction.y += normal.y;
    }
    state.normal = melee_lb::vector::normalize(direction);
}

/// it_80299528 (80299528): the signed turn from the surfaces' normal to
/// the heading (two atan2f(x, y), fsubs, wrapped); the angle between them
/// (lbVector_Angle) times a share of that turn over pi (a double divide
/// and multiply, rounded), signed like the turn, comes off the angle.
fn bend_angle(item: &mut ItemCore) {
    let state = flame(item);
    let turn = wrap_reverse(
        melee_lb::trigf::atan2f(state.heading.x, state.heading.y)
            - melee_lb::trigf::atan2f(state.normal.x, state.normal.y),
    );
    let correction = if turn == 0.0 {
        0.0
    } else {
        let between = melee_lb::vector::angle(state.normal, state.heading);
        let magnitude = f64::from(if turn < 0.0 { -turn } else { turn });
        let rate = if magnitude < HALF_PI {
            GLANCING_TURN
        } else {
            BLUNT_TURN
        };
        let share = (rate * (magnitude / PI)) as f32;
        between * if turn < 0.0 { -share } else { share }
    };
    state.angle = wrap(state.angle - correction);
}
