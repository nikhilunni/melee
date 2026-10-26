//! Birdo's egg (It_Kind_Kyasarin_Egg), itkyasarinegg.c (802EFA44..802F0320).
//!
//! Birdo spits it level along her facing (state 0) until it meets the map,
//! which bounces it back up (state 1) to fall away; anything it meets then
//! ends it. A fighter may pick it up (state 2) and throw or drop it
//! (state 3): a thrown egg that lands or connects breaks, hatching a burst
//! one time in x10 (state 4), otherwise letting out x0C items from the
//! random item table (it_8026F3D4).
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

pub struct KyasarinEgg;

/// it_803F9328's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [0, -1, -1, 1, 2];
/// itKyasarinEggAttributes x0..x10: five words.
pub const SPECIAL_ATTRIBUTES: u32 = 5;

mod motion {
    /// it_802EFB0C: spat out, flying level.
    pub const FLY: u16 = 0;
    /// it_802EFCC0: bounced back, falling.
    pub const BOUNCE: u16 = 1;
    /// itKyasarinEgg_Logic28_PickedUp: in a fighter's hand.
    pub const HELD: u16 = 2;
    /// itKyasarinEgg_Logic28_Thrown: thrown or dropped.
    pub const THROWN: u16 = 3;
    /// it_802EFD84: the burst.
    pub const BURST: u16 = 4;
}

/// efSync_Spawn(0x4D0): the shell burst draws a model and two angles for
/// each of its twelve fragments as it makes them (efsync.c:241-262).
const SHELL_CREATION_DRAWS: usize = 3 * 12;
/// Item_8026AE84(ip, 0xF4, 0x7F, 0x40).
const BREAK_SOUND: u32 = 0xF4;
/// it_80272C08's efSync_Spawn id.
const BURST_EFFECT: u16 = 0x410;

/// itKyasarinEggAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x4: the flying speed.
    fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: the bounce's upward speed.
    fn bounce(&self) -> f32 {
        self.0[2]
    }
    /// xC: items a breaking egg lets out.
    fn contents(&self) -> i32 {
        self.0[3].to_bits() as i32
    }
    /// x10: the odds against a breaking egg bursting.
    fn burst_odds(&self) -> i32 {
        self.0[4].to_bits() as i32
    }
}

static STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: no_physics,
        collision: fly_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: falling_physics,
        collision: bounce_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: no_animation,
        physics: falling_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: burst_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for KyasarinEgg {
    const KIND: ItemKind = ItemKind::KyasarinEgg;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8027B5B0 creates with SpawnItem x44 b0: the creation pass.
    const CONSTRUCTOR_READS_MAP: bool = true;
    /// Its logic row has no spawn callback; the scene starts it ([`start`]).
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::KyasarinEgg { bursting: false };
    }
    /// itKyasarinEgg_Logic28_PickedUp (802EFBC0).
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itKyasarinEgg_Logic28_Dropped (802EFBE8) -> Thrown.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itKyasarinEgg_Logic28_Thrown (802EFC08).
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// it_2725_Logic28_DmgDealt (802EFE28).
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context)
    }
    /// it_802F022C (802F022C).
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context)
    }
    /// it_2725_Logic28_Reflected (802F0104): no recording yet reflects an
    /// egg (a thrown egg's hitbox would need a reflector in its path).
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("it_2725_Logic28_Reflected (itkyasarinegg.c:184): a reflected egg is not witnessed")
    }
    /// it_2725_Logic28_Clanked (802EFF1C).
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context)
    }
    /// it_2725_Logic28_HitShield (802F0010).
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        struck(item, context)
    }
    /// itKyasarinEgg_Logic28_ShieldBounced (802F01F8): only a thrown egg
    /// glances off (itColl_BounceOffShield).
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::THROWN {
            item.bounce_off_shield(context.shield_normal);
        }
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn bursting(item: &ItemCore) -> bool {
    match item.scratch {
        ItemScratch::KyasarinEgg { bursting } => bursting,
        _ => panic!("Birdo egg scratch"),
    }
}

/// it_802EFA44 (802EFA44) after it_8027B5B0: Birdo's facing, its
/// collision's too (mpCollSetFacingDir), no creation intangibility (xDD0
/// b6, xD40), not bursting, hits taken (it_802756E0, xDD0 b7 clear), then
/// it_802EFB0C: grabbable (it_8026B390), flying at x4 along the facing
/// (retail 0x802EFB4C: fmuls), state 0.
pub fn start(item: &mut ItemCore, facing: f32, assets: &ItemAssets) {
    item.facing = facing;
    item.set_collision_facing(if item.facing == -1.0 { -1 } else { 1 });
    item.spawn_intangible_frames = None;
    item.scratch = ItemScratch::KyasarinEgg { bursting: false };
    if !item.keeps_intangible {
        item.hurt_intangible = false;
    }
    item.grabbable = true;
    let speed = Attributes(&assets.special_attributes).speed();
    item.velocity = Vec3::new(speed * item.facing, 0.0, 0.0);
    change(item, motion::FLY, ANIM_UPDATE, assets);
}

fn no_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}
fn no_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_802EFCC0 (802EFCC0): state 1, back the way it came (fneg), up at x8.
fn bounce(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::BOUNCE, ANIM_UPDATE, assets);
    item.velocity.x = -item.velocity.x;
    item.velocity.y = Attributes(&assets.special_attributes).bounce();
}

/// itKyasarinegg_UnkMotion0_Coll (802EFB80): any surface (it_8026DAA8)
/// bounces it.
fn fly_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) & 0xF != 0 {
        bounce(item, ctx.assets);
    }
    false
}

/// itKyasarinegg_UnkMotion1_Phys / UnkMotion3_Phys -> Item_ApplyFallingPhysics
/// (it_80272860 with ItemAttr x10 / x14, then it_80274658 with
/// it_804D6D28 +68).
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itKyasarinegg_UnkMotion1_Coll -> it_8026DFB0: any surface ends it.
fn bounce_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}

/// itKyasarinegg_UnkMotion3_Coll (802EFC7C): a floor (it_8026DA08) breaks
/// a thrown egg as a hit would.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.air_pass(ctx.map) {
        return false;
    }
    let cell = ctx.rng.expect("it_2725_Logic28_DmgDealt draws");
    let mut rng = cell.get();
    let ended = break_open(item, ctx.assets, &mut rng);
    cell.set(rng);
    ended
}

/// it_802EFD84 -> itKyasarinegg_UnkMotion4_Anim -> it_802751D8: the burst
/// runs out its lifetime.
fn burst_animation(item: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// it_damage_inline, shared by the hit, clank, shield, reflect and
/// damage-received callbacks: nothing once bursting; an egg not thrown
/// bounces (it_802EFCC0's body, inlined); a thrown one breaks.
fn struck(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
    if bursting(item) {
        return false;
    }
    if item.motion != motion::THROWN {
        bounce(item, context.assets);
        return false;
    }
    let cell = context.rng.expect("it_2725_Logic28_DmgDealt draws");
    let mut rng = cell.get();
    let ended = break_open(item, context.assets, &mut rng);
    cell.set(rng);
    ended
}

/// The thrown egg breaks: efSync_Spawn(0x4D0) at it, its sound; one time
/// in x10 it bursts (it_802EFD84), otherwise it lets out xC items
/// (it_8026F3D4(gobj, 0, xC, 0)) and is gone.
fn break_open(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) -> bool {
    item.events.push(ItemEvent::EggShell {
        position: item.position,
        seed: rng.seed,
    });
    for _ in 0..SHELL_CREATION_DRAWS {
        rng.rand();
    }
    item.sound_requests.push(BREAK_SOUND);
    let a = Attributes(&assets.special_attributes);
    if rng.randi(a.burst_odds()) == 0 {
        burst(item, assets);
        return false;
    }
    let count = a.contents();
    let ecb = item.collision.as_ref().expect("the egg's collision").ecb;
    // it_8026BB88: 0.5 * (top + bottom), then added to the position.
    let rise = 0.5 * (ecb.top.y + ecb.bottom.y);
    item.events.push(ItemEvent::Release {
        count: count as u8,
        same_kind: false,
        position: Vec3::new(item.position.x, item.position.y + rise, item.position.z),
        previous_position: Vec3::new(item.position.x, item.position.y, 0.0),
    });
    true
}

/// it_802EFD84 (802EFD84): the model hides (it_8026BB44), efSync 0x410
/// (it_80272C08, which also ends hitlag on contact), no hits taken
/// (it_802756D0), not grabbable (it_8026B3A8, it_8026BD24), the common
/// explosion lifetime (it_8027518C), still, bursting, state 4.
fn burst(item: &mut ItemCore, assets: &ItemAssets) {
    item.hidden = true;
    item.events.push(ItemEvent::Effect {
        id: BURST_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    item.hurt_intangible = true;
    item.grabbable = false;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    item.scratch = ItemScratch::KyasarinEgg { bursting: true };
    change(item, motion::BURST, ANIM_UPDATE, assets);
}
