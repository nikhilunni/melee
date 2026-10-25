//! Freezie (It_Kind_Freeze), itfreeze.c (8028EB88..8028F9B8): it falls in,
//! then slides along the floor, faster down slopes; thrown, it breaks on
//! anything it touches, and any hit it deals or takes breaks it.
//!
//! States 4 and 5 (it_8028F434, it_8028F7C8) are Icicle Mountain's: the
//! polar bear's Freezie (it_8028EB88, itwhitebea.c) enters them; nothing on
//! another stage does, so they fail closed.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    FreezeState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::{mp::collide, GroundOrAir, ItemKind};

pub struct Freezie;

/// it_803F5D28's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [0, 0, -1, 1, 0, 0];
/// itUnkAttributes: x0..x10.
pub const SPECIAL_ATTRIBUTES: u32 = 5;

mod motion {
    /// Sliding along the floor.
    pub const SLIDE: u16 = 0;
    /// Falling, from the spawn, a ledge or a drop.
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
}

/// it_8028EDBC's slope test: a floor normal leaning more than this in X.
const SLOPE_LEAN: f32 = 0.1;

/// itUnkAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x4: the speed it slides at on level ground.
    fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: the fastest slide.
    fn top_speed(&self) -> f32 {
        self.0[2]
    }
    /// xC: a slope's pull per unit of its normal's X.
    fn pull(&self) -> f32 {
        self.0[3]
    }
    /// x10: the braking back toward the level speed.
    fn braking(&self) -> f32 {
        self.0[4]
    }
}

static STATES: [ItemStateRow; 6] = [
    row(0, no_animation, slide_physics, slide_collision),
    row(1, no_animation, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, no_animation, gravity, thrown_collision),
    row(4, no_animation, bear_physics, bear_collision),
    row(5, no_animation, no_physics, bear_collision),
];

const fn row(
    state: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for Freezie {
    const KIND: ItemKind = ItemKind::Freeze;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_3F14_Logic17_Spawned (8028ED20): facing the stage's middle, never
    /// sliding down slopes (xD5C 0), still, the floor level, then the fall
    /// (it_8028F1D8).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.facing = if item.position.x > 0.0 { -1.0 } else { 1.0 };
        item.slide_mode = 0;
        item.scratch = ItemScratch::Freeze(FreezeState {
            speed: 0.0,
            floor_normal: Vec3::new(0.0, 1.0, 0.0),
        });
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itFreeze_Logic17_PickedUp: no bear to let go (unk_1C), state 2.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itFreeze_Logic17_Dropped: the fall, keeping the drop's damage.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(
            item,
            motion::FALL,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
    }
    /// itFreeze_Logic17_Thrown: state 3.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(
            item,
            motion::THROWN,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
    }
    /// itFreeze_Logic17_DmgDealt: it breaks.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itFreeze_Logic17_DmgReceived: it breaks.
    fn damage_received(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itFreeze_Logic17_Reflected -> it_80273030 (guarded).
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!(
            "itFreeze_Logic17_Reflected (itfreeze.c:326): a reflected Freezie is not witnessed"
        )
    }
    /// itFreeze_Logic17_Clanked: it breaks.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itFreeze_Logic17_Absorbed: it breaks.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itFreeze_Logic17_ShieldBounced -> itColl_BounceOffShield (guarded).
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itFreeze_Logic17_ShieldBounced (itfreeze.c:331): a Freezie glancing off a shield is not witnessed")
    }
    /// itFreeze_Logic17_HitShield: it breaks.
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
}

fn freeze(item: &ItemCore) -> &FreezeState {
    match &item.scratch {
        ItemScratch::Freeze(state) => state,
        _ => unreachable!("a Freezie's scratch"),
    }
}
fn freeze_mut(item: &mut ItemCore) -> &mut FreezeState {
    match &mut item.scratch {
        ItemScratch::Freeze(state) => state,
        _ => unreachable!("a Freezie's scratch"),
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_80272860 with ItemAttr x10 / x14.
fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// it_8028EF34 (8028EF34): on the floor at its slide speed, grabbable,
/// state 0.
fn start_sliding(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::new(freeze(item).speed, 0.0, 0.0);
    item.grabbable = true;
    change(item, motion::SLIDE, ANIM_UPDATE, assets);
}

/// itFreeze_UnkMotion0_Phys -> it_8028EDBC (8028EDBC): on the floor a
/// slope (normal X past 0.1) pulls the slide (from rest, at x4 down it;
/// retail 8028EE40: fmadds), level ground brings it back toward x4 by x10;
/// held to x8, faced along, and its X speed is the slide along the floor.
fn slide_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    use gekko_math::msl::fabsf;
    if item.ground_or_air == GroundOrAir::Air {
        return;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    let facing = item.facing;
    let state = freeze_mut(item);
    let lean = state.floor_normal.x;
    let mut facing = facing;
    if fabsf(lean) > SLOPE_LEAN {
        if state.speed == 0.0 {
            facing = if lean > 0.0 { 1.0 } else { -1.0 };
            state.speed = a.speed() * facing;
        } else {
            state.speed = gekko_math::fma::fmadds(lean, a.pull(), state.speed);
        }
    } else if fabsf(state.speed) < a.speed() {
        state.speed = a.speed() * facing;
    } else if state.speed > 0.0 {
        state.speed -= a.braking();
    } else {
        state.speed += a.braking();
    }
    if state.speed > a.top_speed() {
        state.speed = a.top_speed();
    } else if state.speed < -a.top_speed() {
        state.speed = -a.top_speed();
    }
    if (state.speed > 0.0 && facing < 0.0) || (state.speed < 0.0 && facing > 0.0) {
        facing *= -1.0;
    }
    let velocity_x = state.speed * state.floor_normal.y;
    item.facing = facing;
    item.velocity.x = velocity_x;
}

/// itFreeze_UnkMotion0_Coll (8028F05C): off the floor it falls
/// (it_8026D62C, it_8028F1D8); on it the floor's normal is kept (none:
/// level), a wall would turn it back (guarded), and the model leans with
/// the floor.
fn slide_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    if item.ground_or_air == GroundOrAir::Air {
        return false;
    }
    match item.floor_contact_normal() {
        Some(normal) => freeze_mut(item).floor_normal = normal,
        None => freeze_mut(item).floor_normal.x = 0.0,
    }
    if item.touches_wall() {
        unimplemented!("itFreeze_UnkMotion0_Coll (itfreeze.c:213): a Freezie turned back by a wall is not witnessed");
    }
    lean(item);
    false
}

/// HSD_JObjSetRotationZ by the floor's lean (it_8027649C), negated when
/// facing right.
fn lean(item: &mut ItemCore) {
    let lean = item.floor_lean();
    item.rotation.z = if item.facing == -1.0 { lean } else { -lean };
}

/// itFreeze_UnkMotion1_Coll (8028F208): it_8026E414 with it_8028EF34; on
/// the floor the floor's normal is kept.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let contact = item.airborne_collision(ctx.map, ctx.assets);
    if contact.floor {
        start_sliding(item, ctx.assets);
    }
    if item.ground_or_air != GroundOrAir::Air {
        let floor = item
            .collision
            .as_ref()
            .is_some_and(|c| c.env_flags as u32 & collide::FLOOR_MASK != 0);
        if floor {
            let normal = item.floor_contact_normal().expect("a floor contact");
            freeze_mut(item).floor_normal = normal;
        }
    }
    false
}

/// itFreeze_UnkMotion3_Coll: thrown, it breaks on any surface
/// (it_8026DAA8).
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) != 0
}

fn bear_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    unimplemented!("itFreeze_UnkMotion4_Phys: the polar bear's Freezie (itfreeze.c:471)")
}
fn bear_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itFreeze_UnkMotion4/5_Coll: the polar bear's Freezie (itfreeze.c:519)")
}
