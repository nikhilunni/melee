//! Flat Zone's falling tools (It_Kind_Tools), it/kinds/ittools.c
//! (802EEFA8..802EFA24).
//!
//! grFlatzone_80217EF0 drops one of five tools from the top of the screen
//! (it_802EEFA8). It falls spinning (states 0..4, one per tool) and hits
//! what it touches; a floor, a hit it deals, a clank or a shield stops it
//! (states 5..9: it lies blinking until its lifetime runs out). A hit it
//! takes knocks it back up with damage that grows with the hit
//! (it_802EF548).
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct Tools;

/// it_803F9260's anim_id column: each motion state plays its own article
/// state.
pub const ARTICLE_STATES: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// itToolsAttributes: x0..xC, then five 0x1C-byte motion rows.
pub const SPECIAL_ATTRIBUTES: u32 = 4 + 5 * MOTION_WORDS;
const MOTION_WORDS: u32 = 7;
/// The state a landed tool takes: its falling state plus five.
const LANDED: i32 = 5;
/// it_802EEFA8: Item_802674AC gives the stage's items hold kind 4.
const STAGE_HOLD_KIND: u8 = 4;
/// it_802EF548's `10.0f` and `1.0f` (@269, @194).
const DAMAGE_BASE: f32 = 10.0;

/// itToolsAttributes, read from the special attribute words.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the falling lifetime (it_80275158).
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the landed lifetime, read through `(s32) x4 & 0xFFFE`.
    fn landed_lifetime(&self) -> f32 {
        (gekko_math::msl::fctiwz(self.0[1]) & 0xFFFE) as f32
    }
    /// x8: a hit's damage multiplier.
    fn damage_scale(&self) -> f32 {
        self.0[2]
    }
    /// xC (s32, converted unsigned): the damage cap.
    fn damage_cap(&self) -> f32 {
        self.0[3].to_bits() as f32
    }
    fn motion(&self, tool: i32) -> Motion {
        let base = 4 + MOTION_WORDS as usize * tool as usize;
        let w = |i: usize| self.0[base + i];
        Motion {
            fall_speed: w(0),
            acceleration: w(1),
            speed_limit: w(2),
            spin: w(3),
            knock_up: w(4),
            hittable_speed: w(5),
            knock_scale: w(6),
        }
    }
}

/// `itToolsMotionAttrs`.
struct Motion {
    /// x0: the fall's starting speed.
    fall_speed: f32,
    /// x4 / x8: it_80272860's gravity and limit.
    acceleration: f32,
    speed_limit: f32,
    /// xC: the spin per frame, by the facing.
    spin: f32,
    /// x10: the upward speed a hit gives.
    knock_up: f32,
    /// x14: a hit only lands while |vel.x| is at most this.
    hittable_speed: f32,
    /// x18: the sideways speed per point of damage.
    knock_scale: f32,
}

const FALLING: ItemStateRow = ItemStateRow {
    animation_id: 0,
    animation: falling_animation,
    physics: falling_physics,
    collision: floor_collision,
};
const LYING: ItemStateRow = ItemStateRow {
    animation_id: 0,
    animation: landed_animation,
    physics: landed_physics,
    collision: floor_collision,
};
static STATES: [ItemStateRow; 10] = {
    let mut rows = [FALLING; 10];
    let mut i = 0;
    while i < 10 {
        rows[i] = if i < 5 { FALLING } else { LYING };
        rows[i].animation_id = ARTICLE_STATES[i];
        i += 1;
    }
    rows
};

impl ItemLogic for Tools {
    const KIND: ItemKind = ItemKind::Tools;
    const STATES: &'static [ItemStateRow] = &STATES;

    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itTools_Logic22_DmgDealt (802EF070) -> it_802EF320.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        land(item, context.assets);
        false
    }
    /// itTools_Logic22_DmgReceived -> it_802EF548.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        knock(item, context.assets);
        false
    }
    /// it_2725_Logic22_Clanked (ittools.c:268) lands it like a hit it
    /// deals; 600 explorer matches never clank a tool.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("ittools.c:268: it_2725_Logic22_Clanked, a tool's hitbox clanks")
    }
    /// it_2725_Logic22_HitShield.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        land(item, context.assets);
        false
    }
    /// it_2725_Logic22_Absorbed (ittools.c:298): it lands with no lifetime
    /// left. Unreachable: GrFz.dat's tool hitboxes are not absorbable
    /// (tests/real_tools.rs).
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("ittools.c:298: it_2725_Logic22_Absorbed, an absorber takes a tool")
    }
    /// itTools_Logic22_ShieldBounced (ittools.c:314) ->
    /// itColl_BounceOffShield. Unreachable: the tool hitboxes lack the
    /// shield-bounce flag (tests/real_tools.rs).
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("ittools.c:314: itTools_Logic22_ShieldBounced, a tool glances off a shield")
    }
    /// itTools_Logic22_Reflected (ittools.c:319) ->
    /// itReflectItemAndUpdateRotation. Unreachable: the tool hitboxes are
    /// not reflectable (tests/real_tools.rs).
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("ittools.c:319: itTools_Logic22_Reflected, a reflector turns a tool")
    }
}

/// it_802EEFA8 (802EEFA8): a tool at `position`, unowned, motionless.
pub fn spawn(position: Vec3, facing: f32) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind: ItemKind::Tools,
        hold_kind: STAGE_HOLD_KIND,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: true,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// it_802EF098 (802EF098), after Item_80268B18: tool `tool` starts
/// falling at its speed.
pub fn start(item: &mut ItemCore, tool: i32, assets: &ItemAssets, half_life_scale: f32) {
    let a = Attributes(&assets.special_attributes);
    item.scratch = ItemScratch::Tools(tool);
    // it_80275158.
    item.life_timer = a.lifetime();
    item.half_life = a.lifetime() * half_life_scale;
    item.slide_mode = 0;
    item.velocity.x = 0.0;
    item.velocity.y = -a.motion(tool).fall_speed;
    // it_8026B3A8.
    item.grabbable = false;
    change(item, tool, assets);
}

fn tool(item: &ItemCore) -> i32 {
    match item.scratch {
        ItemScratch::Tools(tool) => tool,
        _ => panic!("tool scratch"),
    }
}

fn change(item: &mut ItemCore, state: i32, assets: &ItemAssets) {
    item.change_motion_with(state as u16, ARTICLE_STATES[state as usize], ANIM_UPDATE, assets);
}

/// it_802EF320 (802EF320) and its inlined copies: no more hits taken
/// (it_802756D0), not grabbable, stopped, and the landed state with its
/// lifetime.
fn land(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.hurt_intangible = true;
    item.grabbable = false;
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.life_timer = a.landed_lifetime();
    let state = tool(item) + LANDED;
    change(item, state, assets);
}

/// itTools_UnkMotion4_Anim (802EF138): spin by the facing; the lifetime
/// out, it lands where it is.
fn falling_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let a = Attributes(&ctx.assets.special_attributes);
    // retail 802EF16C: fmadds spin * facing + rotation.
    item.rotation.z = gekko_math::fma::fmadds(a.motion(tool(item)).spin, item.facing, item.rotation.z);
    item.joints_dirty = true;
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        land(item, ctx.assets);
    }
    false
}

/// itTools_UnkMotion4_Phys (802EF1F0) -> it_80272860.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let motion = Attributes(&ctx.assets.special_attributes).motion(tool(item));
    item.fall(motion.acceleration, motion.speed_limit);
}

/// itTools_UnkMotion4_Coll / itTools_UnkMotion9_Coll: while it moves, a
/// floor under it_8026DAA8 lands it.
fn floor_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.velocity.x != 0.0 || item.velocity.y != 0.0 {
        let bits = item.air_contact_bits(ctx.map);
        if bits & 3 != 0 {
            land(item, ctx.assets);
        }
    }
    false
}

/// itTools_UnkMotion9_Anim (802EF3A8): blink every other two frames
/// (shown frames re-pose the landed state at frame 0, it_80273670) and go
/// when the lifetime runs out.
fn landed_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        return true;
    }
    if gekko_math::msl::fctiwz(item.life_timer) & 2 != 0 {
        // it_8026BB20.
        item.hidden = false;
        item.pose_article_frame(0.0);
    } else {
        // it_8026BB44.
        item.hidden = true;
    }
    false
}

/// itTools_UnkMotion9_Phys: nothing.
fn landed_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

/// it_802EF548 (802EF548): unless it moves faster than the motion allows,
/// the lifetime restarts from the half-life, the hit's damage (times x8)
/// adds to the hitbox's (capped at xC), and it flies up and away from the
/// attacker (or against the hit's direction when the hit was an item's).
fn knock(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    let motion = a.motion(tool(item));
    if gekko_math::msl::fabsf(item.velocity.x) > motion.hittable_speed {
        unimplemented!("ittools.c:225: it_802EF548, a tool hit while flying faster than x14");
    }
    item.life_timer = item.half_life;
    // retail 802EF5EC: fmuls -1.0 * xCCC, which only flips the sign.
    let mut direction = -item.hit_direction;
    if !item.struck_by_item && item.hit_by.is_some() {
        let attacker = item
            .attacker_position
            .expect("it_802EF548: the attacker's ftLib_800866DC position");
        direction = if item.position.x > attacker.x { 1.0 } else { -1.0 };
    }
    let hitbox = |item: &ItemCore| {
        item.hitboxes[0]
            .as_ref()
            .unwrap_or_else(|| unimplemented!("it_802EF548: a tool without hitbox 0"))
            .descriptor
            .damage
    };
    // retail 802EF5F0..65C: the frame's damage as a float, fmuls x8, fadds
    // the hitbox's; capped at xC; __cvt_fp2unsigned.
    let mut damage = item.pending_damage_taken as f32 * a.damage_scale() + hitbox(item);
    if damage > a.damage_cap() {
        damage = a.damage_cap();
    }
    item.set_hitbox_damage(0, fp2unsigned(damage));
    // retail 802EF6A8..BC: fsubs 10, fadds 1, fmuls x18, fmuls direction.
    let knock = (1.0 + (hitbox(item) - DAMAGE_BASE)) * motion.knock_scale;
    item.velocity.x = direction * knock;
    item.velocity.y = motion.knock_up;
}

/// `__cvt_fp2unsigned` for a damage in `[0, 2^31)`.
fn fp2unsigned(value: f32) -> u32 {
    assert!(value >= 0.0, "__cvt_fp2unsigned: negative tool damage {value}");
    gekko_math::msl::fctiwz(value) as u32
}
