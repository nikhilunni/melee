//! Whispy's apple (It_Kind_WhispyApple), itwhispyapple.c (802EE200..
//! 802EEF88): Whispy Woods drops it behind the stage; it comes forward to
//! the stage's plane as it falls, bounces to rest and can be picked up and
//! thrown, or (one in ten) vanishes in a puff where it lands. It goes when
//! its lifetime runs out or when it hits something.
//!
//! The healing apple (items on) is not witnessed and fails closed.
use gekko_math::{fma::fmsubs, HsdRng};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, HIT_PRESERVE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SlidePass,
    SpawnItem,
};
use melee_types::{GroundOrAir, ItemKind};

pub struct WhispyApple;

/// it_802EEA70 ends with it_8027429C: the apple leaves the hand and its
/// hold ends a second time inside Item_8026ABD8 (the throw count counts
/// both).
pub const RELEASES_ITSELF_ON_DROP: bool = true;

/// it_803F91D0's anim_id column.
pub const ARTICLE_STATES: [i32; 8] = [1, 0, 0, 0, 1, 1, 0, 2];
/// itWhispyAppleAttributes: seven words.
pub const SPECIAL_ATTRIBUTES: u32 = 7;
/// Item_802674AC: the stage enemies' range holds as kind 4.
const HOLD_KIND: u8 = 4;

mod motion {
    /// Coming forward and falling in (it_802EE374).
    pub const ENTER: u16 = 0;
    pub const REST: u16 = 1;
    pub const FALL: u16 = 2;
    pub const HELD: u16 = 3;
    /// Dropped (it_802EEA70) and thrown (it_802EEB28).
    pub const DROPPED: u16 = 4;
    pub const THROWN: u16 = 5;
    pub const SLIDE: u16 = 6;
    /// The vanishing puff (it_802EED00).
    pub const VANISH: u16 = 7;
}

/// it_802EED00: the puff's lifetime and gust.
const VANISH_LIFETIME: f32 = 20.0;
const VANISH_EFFECT: u16 = 0x410;
/// The z it comes forward from, over 20 ticks.
const APPROACH_TICKS: f32 = 20.0;

/// itWhispyAppleAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn word(&self, index: usize) -> i32 {
        self.0[index].to_bits() as i32
    }
    /// x4: what eating it heals.
    fn heal(&self) -> i32 {
        self.word(1)
    }
    /// x8: its lifetime.
    fn lifetime(&self) -> f32 {
        self.word(2) as f32
    }
    /// x14: its upward speed.
    fn rise(&self) -> f32 {
        self.0[5]
    }
    /// x18: its sideways speed's range either way.
    fn spread(&self) -> f32 {
        self.0[6]
    }
}

static STATES: [ItemStateRow; 8] = [
    row(motion::ENTER, enter_animation, enter_physics, enter_collision),
    row(motion::REST, live_animation, no_physics, rest_collision),
    row(motion::FALL, live_animation, fall_physics, fall_collision),
    row(motion::HELD, held_animation, no_physics, no_collision),
    row(motion::DROPPED, live_animation, fall_physics, fall_collision),
    row(motion::THROWN, live_animation, fall_physics, fall_collision),
    row(motion::SLIDE, live_animation, no_physics, slide_collision),
    row(motion::VANISH, live_animation, no_physics, no_collision),
];

const fn row(
    motion: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion as usize],
        animation,
        physics,
        collision,
    }
}

/// it_802EE200's spawn: kind WhispyApple at `position`, facing `facing`
/// (both drawn by the caller before Item_80268B18), `vanishes` its xDD8
/// (carried in the spawn argument).
pub fn spawn(position: Vec3, facing: f32, vanishes: bool) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind: ItemKind::WhispyApple,
        hold_kind: HOLD_KIND,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: i32::from(vanishes),
        initial_collision: false,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

impl ItemLogic for WhispyApple {
    const KIND: ItemKind = ItemKind::WhispyApple;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802EE200's it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    /// itWhispyApple_Logic18_EnteredAir: the slide.
    const ENTERED_AIR: Option<EnteredAir> =
        Some(|item, assets| change(item, motion::SLIDE, ANIM_UPDATE, assets));
    /// it_802EE374 (802EE374) once Item_80268B18 returns: state 0, the
    /// lifetime (it_80275158 of x8), the heal, a sideways speed drawn in
    /// [-x18, x18) (retail 0x802EE3E8: fmuls then fmsubs), x14 up and the
    /// approach |z / 20|; then xDD8, a vanishing one not grabbable.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        rng: &mut HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        change(item, motion::ENTER, ANIM_UPDATE, assets);
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        let vanishes = spawn.spawn_argument != 0;
        item.scratch = ItemScratch::WhispyApple {
            heal: a.heal(),
            vanishes,
        };
        let random = rng.randf();
        item.velocity.x = fmsubs(2.0 * a.spread(), random, a.spread());
        item.velocity.y = a.rise();
        let approach = item.position.z / APPROACH_TICKS;
        item.velocity.z = if approach < 0.0 { -approach } else { approach };
        if vanishes {
            item.grabbable = false;
        }
    }
    /// it_802EEA08: shown, state 3.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.hidden = false;
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// it_802EEB28: grabbable, shown, state 5, on the stage's plane.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = true;
        item.hidden = false;
        change(item, motion::THROWN, ANIM_UPDATE, context.assets);
        item.velocity.z = 0.0;
        item.position.z = 0.0;
    }
    /// it_802EEA70 (802EEA70): grabbable, shown, state 4, still in y and
    /// z on the stage's plane; then it releases itself from the hand once
    /// more (it_8027429C with no speed), which the scene's drop does
    /// ([`RELEASES_ITSELF_ON_DROP`]).
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = true;
        item.hidden = false;
        change(item, motion::DROPPED, ANIM_UPDATE, context.assets);
        item.velocity.z = 0.0;
        item.velocity.y = 0.0;
        item.position.z = 0.0;
    }
    /// it_802EEED0: it goes.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_802EEF10 -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// it_802EEED8: its hitboxes go (it_802725D4), itColl_BounceOffVictim.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.clear_hitboxes();
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// it_802EEF30: as it_802EEED8.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.clear_hitboxes();
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// it_802EEF68 -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn vanishes(item: &ItemCore) -> bool {
    matches!(item.scratch, ItemScratch::WhispyApple { vanishes: true, .. })
}

/// itWhispyapple_UnkMotion0_Anim_inline: the lifetime counts down; the
/// model hides in the puff (the blink near the end is drawing only).
fn live_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.motion == motion::VANISH {
        item.hidden = true;
    }
    item.life_timer <= 0.0
}

/// itWhispyapple_UnkMotion0_Anim: a vanishing apple stays ungrabbable.
fn enter_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if vanishes(item) {
        item.grabbable = false;
    }
    live_animation(item, ctx)
}

fn held_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// it_80272860 with ItemAttr x10 / x14.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

fn enter_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if vanishes(item) {
        item.grabbable = false;
    }
    fall_physics(item, ctx);
}

/// it_802EE6A0: still, state 1.
fn stop(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

/// itWhispyapple_UnkMotion0_Coll: nothing until it reaches the stage's
/// plane; then a vanishing apple's landing (it_8026E15C) puffs, any other
/// bounces with one of three sounds (HSD_Randi(3)) and rests.
fn enter_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let vanish = vanishes(item);
    if vanish {
        item.grabbable = false;
    }
    if item.position.z < 0.0 {
        return false;
    }
    item.position.z = 0.0;
    item.velocity.z = 0.0;
    if !vanish {
        return fall_collision(item, ctx);
    }
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            vanish_now(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itWhispyapple_UnkMotion5_Coll: bounce with a sound drawn, rest once
/// settled.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("the landing sound draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing_after(
        ctx.map,
        ctx.assets,
        ctx.entered_air,
        &mut rng,
        // it_803F9250[HSD_Randi(3)]: the landing sound.
        |rng| {
            rng.randi(3);
        },
    );
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            stop(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itWhispyapple_UnkMotion1_Coll -> it_8026D62C: off the floor it falls.
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itWhispyapple_UnkMotion6_Coll -> it_8026E8C4.
fn slide_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    match item.ground_collision_with_slide(ctx.map, ctx.assets) {
        SlidePass::Airborne => change(item, motion::FALL, ANIM_UPDATE, ctx.assets),
        SlidePass::Stopped => stop(item, ctx.assets),
        SlidePass::Sliding => {}
    }
    false
}

/// it_802EED00 (802EED00): hidden and ungrabbable, a 20-tick lifetime,
/// unpushable and still, the puff (it_80272C08: efSync 0x410, no hitlag),
/// no hits taken, its hits reaching its owner and kin, a gust
/// (lb_800119DC(&pos, 0x78, 1.0, 0.02, pi/3)), state 7.
fn vanish_now(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    item.hidden = true;
    item.life_timer = VANISH_LIFETIME;
    item.half_life = VANISH_LIFETIME * assets.half_life_scale;
    item.pushable = false;
    item.velocity = Vec3::ZERO;
    item.events.push(ItemEvent::Effect {
        id: VANISH_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    item.hurt_intangible = true;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.events.push(ItemEvent::Gust {
        center: item.position,
        frames: 0x78,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
    change(item, motion::VANISH, ANIM_UPDATE | HIT_PRESERVE, assets);
}
